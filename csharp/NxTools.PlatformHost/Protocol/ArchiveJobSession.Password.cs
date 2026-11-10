using System.Runtime.InteropServices;
using System.Security.Cryptography;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Protocol;

// Original job pipe session only. This owner cannot issue entry/preview/OLE authorization.
internal sealed partial class ArchiveJobSession
{
    private readonly Dictionary<Guid, BatchSecret> batchSecrets = [];

    private sealed class BatchSecret : IDisposable
    {
        private byte[]? protectedBytes;
        private readonly int length;
        internal BatchSecret(ReadOnlySpan<byte> secret)
        {
            if (secret.Length == 0 || !ArchiveWorkerRunner.ValidPasswordBytes(secret)) throw new ArchiveOutputException("invalid_request");
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
            var bytes = (protectedBytes ?? throw new ObjectDisposedException(nameof(BatchSecret))).ToArray();
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
        public override string ToString() => "BatchSecret([PRIVATE])";
        [DllImport("crypt32.dll", ExactSpelling = true, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool CryptProtectMemory([In, Out] byte[] data, uint length, uint flags);
        [DllImport("crypt32.dll", ExactSpelling = true, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool CryptUnprotectMemory([In, Out] byte[] data, uint length, uint flags);
    }
    private static bool BatchPasswordEligible(Operation op) => op.Command.BatchId is not null &&
        op.Command.Generation == 1 && !op.BatchPasswordRetired && op.State != "cancelling" &&
        op.Command.Request is { Kind: "test" or "extract", Paths.Length: 1 } request && request.Extract?.Reclamation is null;

    private void RetireBatchSecret(Guid? batch)
    {
        if (batch is {} id && batchSecrets.Remove(id, out var secret)) secret.Dispose();
    }
    private void RetireBatchPassword(Guid? batch)
    {
        RetireBatchSecret(batch);
        if (batch is null) return;
        foreach (var op in active.Values)
            if (op.Command.BatchId == batch) op.BatchPasswordRetired = true;
    }
    private void InvalidateBatchPassword(Operation op)
    {
        if (op.Command.BatchId is not {} batch || !batchSecrets.ContainsKey(batch)) return;
        RetireBatchSecret(batch);
        Diagnostic("batch_password_invalidated", op.Command);
    }
    private void RetireBatchSecrets()
    {
        foreach (var secret in batchSecrets.Values) secret.Dispose();
        batchSecrets.Clear();
    }
    private void RememberBatchSecret(Operation op, ArchiveJobReply reply)
    {
        if (!reply.RememberBatchPassword || !BatchPasswordEligible(op) || reply.Password is not {} bytes) return;
        var batch = op.Command.BatchId!.Value;
        RetireBatchSecret(batch);
        // Same finite original queue budget: two running plus 32 queued. Never evict another batch.
        if (batchSecrets.Count >= 34) { Diagnostic("batch_password_capacity", op.Command); return; }
        try { batchSecrets.Add(batch, new BatchSecret(bytes)); Diagnostic("batch_password_remembered", op.Command); }
        catch (ArchiveOutputException failure) {
            RetireBatchSecret(batch);
            Diagnostic("batch_password_protection_failed", op.Command, new("failed", "secret_protection_failed", NativeErrorCode: failure.NativeCode));
        }
    }
    private bool TryReuseBatchSecret(Operation op, ArchiveJobPrompt prompt)
    {
        // The original worker's first missing-password notification follows read attempt 1.
        // Later password attempts always require a fresh explicit answer.
        if (!BatchPasswordEligible(op) || op.BatchPasswordTried ||
            prompt.Kind != "password" || prompt.ErrorCode != "PasswordRequired" || prompt.Attempt != 1 ||
            !batchSecrets.TryGetValue(op.Command.BatchId!.Value, out var secret)) return false;
        op.BatchPasswordTried = true;
        byte[]? plaintext = null;
        try {
            plaintext = secret.CopyPlaintext();
            if (!runner.RespondPasswordBytes(op.Command.OperationId, new(prompt.Ticket.OperationId, prompt.Ticket.PromptId, prompt.Ticket.Generation), plaintext)) return false;
            op.Prompt = null;
            runner.SetPromptWaiting(op.Command.OperationId, false);
            State(op, "running");
            Diagnostic("batch_password_reused", op.Command);
            return true;
        } catch (ArchiveOutputException failure) {
            RetireBatchSecret(op.Command.BatchId);
            Diagnostic("batch_password_protection_failed", op.Command, new("failed", "secret_protection_failed", NativeErrorCode: failure.NativeCode));
            return false;
        } finally { if (plaintext is not null) CryptographicOperations.ZeroMemory(plaintext); }
    }
}
