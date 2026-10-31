using System.Runtime.InteropServices;
using System.Security.Cryptography;
using NxTools.PlatformHost.Windows;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Protocol;

// Original pipe-session lifecycle only. Rust InteractionPort remains the sole retry owner.
internal sealed partial class ArchiveEntrySession
{
    private readonly Dictionary<Guid, Authorization> authorizations = [];
    private readonly Func<DateTimeOffset> utcNow;
    private readonly System.Threading.Timer authorizationTimer;
    private int expiryRunning;

    // Narrow SAME_PROCESS Windows protected-memory owner; never a record/string/debug payload.
    private sealed class ProtectedSecret : IDisposable
    {
        private byte[]? protectedBytes;
        private readonly int length;
        internal ProtectedSecret(ReadOnlySpan<byte> secret)
        {
            if (!ArchiveWorkerRunner.ValidPasswordBytes(secret)) throw new ArchiveOutputException("invalid_request");
            length = secret.Length;
            var bytes = new byte[Math.Max(16, (length + 15) / 16 * 16)];
            secret.CopyTo(bytes);
            try {
                if (!CryptProtectMemory(bytes, (uint)bytes.Length, 0)) throw new ArchiveOutputException("secret_protection_failed", Marshal.GetLastWin32Error());
                protectedBytes = bytes;
            } finally { if (protectedBytes is null) CryptographicOperations.ZeroMemory(bytes); }
        }
        internal byte[] CopyPlaintext()
        {
            var bytes = (protectedBytes ?? throw new ObjectDisposedException(nameof(ProtectedSecret))).ToArray();
            try {
                if (!CryptUnprotectMemory(bytes, (uint)bytes.Length, 0)) throw new ArchiveOutputException("secret_protection_failed", Marshal.GetLastWin32Error());
                return bytes.AsSpan(0, length).ToArray();
            } finally { CryptographicOperations.ZeroMemory(bytes); }
        }
        public void Dispose()
        {
            var bytes = Interlocked.Exchange(ref protectedBytes, null);
            if (bytes is not null) CryptographicOperations.ZeroMemory(bytes);
        }
        [DllImport("crypt32.dll", ExactSpelling = true, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool CryptProtectMemory([In, Out] byte[] data, uint length, uint flags);
        [DllImport("crypt32.dll", ExactSpelling = true, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool CryptUnprotectMemory([In, Out] byte[] data, uint length, uint flags);
    }
    private sealed class Authorization(ArchiveEntryCommand command, ProtectedSecret secret, DateTimeOffset expires) : IDisposable
    {
        internal readonly ArchiveEntryCommand Command = command;
        internal readonly ProtectedSecret Secret = secret;
        internal readonly DateTimeOffset Expires = expires;
        private readonly CancellationTokenSource stop = new();
        internal CancellationToken Token => stop.Token;
        public void Dispose()
        {
            Secret.Dispose();
            // Cancel state changes immediately, but callbacks must not run under entry's gate:
            // content cancellation may call this same owner from a different leased owner.
            _ = CancelAndDispose();
        }
        private async Task CancelAndDispose() { try { await stop.CancelAsync(); } finally { stop.Dispose(); } }
    }
    private static bool Matches(ArchiveEntryCommand original, ArchiveEntryCommand requested) =>
        original.SessionId == requested.SessionId && original.Generation == requested.Generation && original.Path == requested.Path &&
        original.Codepage == requested.Codepage && original.Depth == requested.Depth && original.CopyMotw == requested.CopyMotw &&
        original.Browser is { } a && requested.Browser is { } b && a.SnapshotId == b.SnapshotId && a.Format == b.Format && a.Comment == b.Comment &&
        a.Source.Native == b.Source.Native && a.Source.Size == b.Source.Size && a.Source.ModifiedUnixNs == b.Source.ModifiedUnixNs && a.Source.Digest.SequenceEqual(b.Source.Digest);
    private bool TryAuthorization(Guid id, ArchiveEntryCommand command, out Authorization grant)
    {
        if (authorizations.TryGetValue(id, out grant!) && grant.Expires > utcNow() && !grant.Token.IsCancellationRequested && Matches(grant.Command, command)) return true;
        grant = null!; return false;
    }
    internal bool TryCaptureVirtualAuthorization(Guid? id, ArchiveVirtualDragItem[] items, out CancellationToken token)
    {
        token = default;
        if (id is null) return true;
        lock (gate) {
            ExpireAuthorizationsLocked();
            if (stopped || items.Length == 0) return false;
            foreach (var item in items) if (!TryAuthorization(id.Value, item.Entry, out _)) return false;
            token = authorizations[id.Value].Token; return true;
        }
    }
    internal Task<WindowsCommandResult> DispatchVirtualMaterialize(ArchiveEntryCommand command, ulong outputLimit, Guid? authorizationId) =>
        Dispatch(ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data = command }, command.OperationId), outputLimit, authorizationId);
    private void CancelAuthorizedEntry(Pending pending)
    {
        lock (gate) {
            if (!active.TryGetValue(pending.Command.OperationId, out var current) || !ReferenceEquals(current, pending) || pending.Finishing) return;
            pending.Cancelled = true; pending.PasswordTicket = null; runner.Cancel(pending.Command.OperationId);
        }
    }
    private static void ClearSuppliedSecret(Pending pending) { pending.SuppliedSecret?.Dispose(); pending.SuppliedSecret = null; }
    internal bool IsAuthorizationPasswordOperation(Guid operation)
    {
        lock (gate) return active.TryGetValue(operation, out var pending) && pending.Command.Action == "authorize" || authorizations.ContainsKey(operation) || retired.Contains(operation);
    }
    internal bool RespondAuthorizationPassword(Guid operation, Guid session, ulong generation, ArchivePasswordTicket ticket, byte[]? password)
    {
        lock (gate) {
            if (!active.TryGetValue(operation, out var pending) || pending.Command.Action != "authorize" || stopped || pending.Cancelled || pending.Finishing ||
                pending.Command.SessionId != session || pending.Command.Generation != generation || pending.PasswordTicket != ticket) return false;
            if (!runner.RespondPasswordBytes(operation, ticket, password)) return false;
            ClearSuppliedSecret(pending); pending.PasswordTicket = null;
            if (password is not null) {
                try { pending.SuppliedSecret = new(password); }
                catch (Exception failure) { pending.Failure = "secret_protection_failed"; pending.FailureNative = (failure as ArchiveOutputException)?.NativeCode; runner.Cancel(operation); }
            } else {
                pending.Cancelled = true; runner.Cancel(operation);
            }
            return true;
        }
    }
    private WindowsCommandResult RevokeAuthorization(ArchiveEntryCommand command)
    {
        var matched = false;
        if (active.TryGetValue(command.OperationId, out var pending) && pending.Command.Action == "authorize" &&
            pending.Command.SessionId == command.SessionId && pending.Command.Generation == command.Generation) {
            pending.Cancelled = true; pending.PasswordTicket = null; ClearSuppliedSecret(pending); runner.Cancel(command.OperationId); matched = true;
        }
        if (authorizations.TryGetValue(command.OperationId, out var grant) && grant.Command.SessionId == command.SessionId && grant.Command.Generation == command.Generation) {
            authorizations.Remove(command.OperationId); grant.Dispose(); matched = true;
        }
        AuthorizationDiagnostic(matched ? "revoked" : "revoke_rejected", command);
        return ArchiveEntryWire.Result(new(1, command.OperationId, command.SessionId, command.Generation, matched, null, null, matched ? null : "authorization_invalid"));
    }
    private void ExpireAuthorizations()
    {
        if (Interlocked.Exchange(ref expiryRunning, 1) != 0) return;
        try { lock (gate) { if (!stopped) ExpireAuthorizationsLocked(); } }
        finally { Volatile.Write(ref expiryRunning, 0); }
    }
    private void ExpireAuthorizationsLocked()
    {
        var now = utcNow();
        foreach (var pair in authorizations.Where(pair => pair.Value.Expires <= now).ToArray()) {
            authorizations.Remove(pair.Key); pair.Value.Dispose(); AuthorizationDiagnostic("expired", pair.Value.Command);
        }
    }
    private void AuthorizationDiagnostic(string phase, ArchiveEntryCommand command)
    {
        try { log?.Write("info", "archive.entry.authorization." + phase, command.OperationId,
            new Dictionary<string, object?> { ["generation"] = command.Generation, ["live_count"] = authorizations.Count }); } catch { }
    }
    internal int AuthorizationCountForFixture { get { lock (gate) return authorizations.Count; } }
    internal int ActiveWorkerCountForFixture => runner.ActiveCount;
    internal void CleanupCheckpointForFixture(Action probe) => runner.CleanupCheckpointProbe = probe;
}
