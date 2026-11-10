using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text.Json;

namespace NxTools.ShellSelectionHelper;

/// <summary>
/// Planned helper identity. These CLSIDs are NOT registered in this round; registration needs a
/// separately authorized current-user plan (LocalServer32 + DelegateExecute) and live acceptance.
/// </summary>
internal static class HelperIdentity
{
    internal const string ExecutableName = "NxTools.ShellSelectionHelper.exe";
    internal const string DesktopExecutableName = "NxTools.exe";

    internal static readonly IReadOnlyDictionary<ShellVerb, Guid> VerbClasses = new Dictionary<ShellVerb, Guid>
    {
        [ShellVerb.Open] = new("01E01D02-96B5-4338-901E-2C55D944A7A8"),
        [ShellVerb.ExtractSmart] = new("104E8FCF-4BFC-47CA-9EE2-C8D5FC96D025"),
        [ShellVerb.ExtractHere] = new("FBBF077F-81DF-4AE8-A56E-89F7FF2A43B7"),
        [ShellVerb.ExtractName] = new("A0E1A23B-AF9E-4C5C-898D-0D486E6D170E"),
        [ShellVerb.ExtractDialog] = new("306F1051-91FA-4D1B-87BC-E02E9D68FB46"),
        [ShellVerb.Test] = new("133C1261-3EBC-4B3A-89C6-9AECEB6C352D"),
        [ShellVerb.Compress] = new("BC9707AF-8361-431D-82CC-36554BDDA04C"),
        [ShellVerb.CompressZip] = new("0E30B5E0-6767-4FFB-8198-D748035B3E6D"),
        [ShellVerb.CompressSevenZ] = new("55CCFF86-9D14-4750-9497-7A8D586A6402"),
    };

    /// <summary>Existing Desktop launch-ingress vocabulary; paths follow as separate argv items.</summary>
    internal static IReadOnlyList<string> IntentArguments(ShellVerb verb) => verb switch
    {
        ShellVerb.Open => ["--archive-open"],
        ShellVerb.ExtractSmart => ["--archive-extract", "smart"],
        ShellVerb.ExtractHere => ["--archive-extract", "here"],
        ShellVerb.ExtractName => ["--archive-extract", "name"],
        ShellVerb.ExtractDialog => ["--archive-extract", "dialog"],
        ShellVerb.Test => ["--archive-test"],
        ShellVerb.Compress => ["--archive-compress"],
        ShellVerb.CompressZip => ["--archive-compress-zip"],
        ShellVerb.CompressSevenZ => ["--archive-compress-7z"],
        _ => throw new ArgumentOutOfRangeException(nameof(verb)),
    };
}

/// <summary>
/// Resolves the exact packaged sibling desktop and verifies its identity. The helper never
/// accepts a caller-supplied executable path; the expected hash comes from the release manifest
/// the future package step generates (empty means unpackaged: delivery is refused).
/// </summary>
internal sealed class DesktopIdentity(string helperDirectory, string expectedDesktopSha256)
{
    internal string DesktopPath => Path.Combine(helperDirectory, HelperIdentity.DesktopExecutableName);

    internal (bool Approved, string Reason) Verify()
    {
        if (expectedDesktopSha256.Length != 64 || !expectedDesktopSha256.All(char.IsAsciiHexDigit))
            return (false, "release_identity_unavailable");
        var path = Path.GetFullPath(DesktopPath);
        if (!string.Equals(Path.GetDirectoryName(path), Path.GetFullPath(helperDirectory).TrimEnd('\\'), StringComparison.OrdinalIgnoreCase))
            return (false, "not_sibling");
        if (!File.Exists(path)) return (false, "desktop_missing");
        var attributes = File.GetAttributes(path);
        if ((attributes & FileAttributes.ReparsePoint) != 0) return (false, "desktop_reparse");
        using var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read);
        var actual = Convert.ToHexString(SHA256.HashData(stream));
        return string.Equals(actual, expectedDesktopSha256, StringComparison.OrdinalIgnoreCase)
            ? (true, "approved")
            : (false, "desktop_hash_mismatch");
    }
}

/// <summary>Runs the approved desktop; null means it was still running at the bound (new owner).</summary>
internal interface IDesktopLauncher
{
    int? Run(string executable, IReadOnlyList<string> arguments, TimeSpan timeout, CancellationToken cancellation);
}

/// <summary>
/// Forwards through the EXISTING Desktop ingress: the approved desktop's own startup parser,
/// single-instance check, peer-authenticated pipe, bounded queue and request ID. One process
/// launch per complete selection; no new IPC owner and no timer/coalescing.
/// </summary>
internal sealed class DesktopIntentForwarder(DesktopIdentity identity, IDesktopLauncher launcher) : ISelectionForwarder
{
    internal static readonly TimeSpan HandoffTimeout = TimeSpan.FromSeconds(30);

    public ForwardResult Forward(Guid correlationId, ShellVerb verb, IReadOnlyList<string> paths, CancellationToken cancellation)
    {
        if (cancellation.IsCancellationRequested) return new(ForwardStatus.Cancelled, 0);
        var (approved, _) = identity.Verify();
        if (!approved) return new(ForwardStatus.IdentityRejected, 0);
        var prefix = verb is ShellVerb.CompressZip or ShellVerb.CompressSevenZ
            ? new[] { "--archive-correlation-id", correlationId.ToString("D") }
            : Array.Empty<string>();
        var arguments = prefix.Concat(HelperIdentity.IntentArguments(verb)).Concat(paths).ToArray();
        int? exitCode;
        try { exitCode = launcher.Run(identity.DesktopPath, arguments, HandoffTimeout, cancellation); }
        catch (OperationCanceledException) { return new(ForwardStatus.Cancelled, 0); }
        catch (Exception error) when (error is System.ComponentModel.Win32Exception or IOException)
        {
            return new(ForwardStatus.Unavailable, error.HResult);
        }
        // Desktop startup: 0 forwarded to the running owner, 1 typed handoff failure,
        // 2 invalid request. A process alive at the bound became the cold-start owner.
        return exitCode switch
        {
            null => new(ForwardStatus.HandedToNewOwner, 0),
            0 => new(ForwardStatus.Delivered, 0),
            2 => new(ForwardStatus.Rejected, 2),
            { } other => new(ForwardStatus.Unavailable, other),
        };
    }
}

internal sealed class ProcessDesktopLauncher : IDesktopLauncher
{
    public int? Run(string executable, IReadOnlyList<string> arguments, TimeSpan timeout, CancellationToken cancellation)
    {
        var start = new ProcessStartInfo(executable)
        {
            UseShellExecute = false,
            CreateNoWindow = true,
            WorkingDirectory = Path.GetDirectoryName(executable)!,
        };
        foreach (var argument in arguments) start.ArgumentList.Add(argument);
        using var process = Process.Start(start) ?? throw new IOException("Desktop handoff process did not start.");
        // Never kill the launched desktop: it may already own the request. Cancellation only stops waiting.
        var deadline = DateTime.UtcNow + timeout;
        while (!process.WaitForExit(250))
        {
            if (DateTime.UtcNow >= deadline) return null;
            cancellation.ThrowIfCancellationRequested();
        }
        return process.ExitCode;
    }
}

/// <summary>Bounded JSON-lines diagnostics with path-free fields only. Sink failure is ignored.</summary>
internal sealed class FileHelperLog(string directory) : IHelperLog
{
    private const long MaxBytes = 1024 * 1024;
    private readonly object _gate = new();

    internal static FileHelperLog CreateDefault() => new(Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "NxTools", "NxTools.Next", "data", "logs"));

    public void Write(string eventName, Guid correlationId, IReadOnlyDictionary<string, object?> fields)
    {
        try
        {
            var entry = new Dictionary<string, object?>(fields)
            {
                ["timestamp"] = DateTimeOffset.UtcNow,
                ["event"] = eventName,
                ["correlation_id"] = correlationId,
            };
            var line = JsonSerializer.Serialize(entry) + "\n";
            lock (_gate)
            {
                Directory.CreateDirectory(directory);
                var path = Path.Combine(directory, "shell-selection-helper.jsonl");
                if (File.Exists(path) && new FileInfo(path).Length > MaxBytes) return;
                File.AppendAllText(path, line);
            }
        }
        catch (Exception) { /* diagnostics never change the result */ }
    }
}

/// <summary>
/// Out-of-process COM local server lifetime on one STA: CoInitializeEx, register every verb
/// class factory suspended, resume, pump until no objects/locks remain (or an idle bound),
/// then revoke and uninitialize. Entered only for the COM `-Embedding` activation contract.
/// </summary>
internal static class LocalServer
{
    private const uint CoinitApartmentThreaded = 0x2;
    private const uint ClsctxLocalServer = 0x4;
    private const uint RegclsMultipleUse = 1;
    private const uint RegclsSuspended = 4;
    internal static readonly TimeSpan InitialIdle = TimeSpan.FromSeconds(30);

    [DllImport("ole32.dll")] private static extern int CoInitializeEx(IntPtr reserved, uint init);
    [DllImport("ole32.dll")] private static extern void CoUninitialize();
    [DllImport("ole32.dll")] private static extern int CoRegisterClassObject(ref Guid clsid, [MarshalAs(UnmanagedType.IUnknown)] object unknown, uint context, uint flags, out uint cookie);
    [DllImport("ole32.dll")] private static extern int CoRevokeClassObject(uint cookie);
    [DllImport("ole32.dll")] private static extern int CoResumeClassObjects();
    [DllImport("user32.dll")] private static extern uint MsgWaitForMultipleObjects(uint count, IntPtr[] handles, bool waitAll, uint milliseconds, uint wakeMask);
    [DllImport("user32.dll")] private static extern bool PeekMessage(out Msg message, IntPtr window, uint min, uint max, uint remove);
    [DllImport("user32.dll")] private static extern bool TranslateMessage(ref Msg message);
    [DllImport("user32.dll")] private static extern IntPtr DispatchMessage(ref Msg message);

    [StructLayout(LayoutKind.Sequential)]
    private struct Msg
    {
        public IntPtr Window;
        public uint Message;
        public UIntPtr WParam;
        public IntPtr LParam;
        public uint Time;
        public NativePoint Point;
    }

    internal static int Run(ISelectionForwarder forwarder, IHelperLog log)
    {
        var correlation = Guid.NewGuid();
        var started = Stopwatch.GetTimestamp();
        var hr = CoInitializeEx(IntPtr.Zero, CoinitApartmentThreaded);
        if (hr < 0) return 3;
        var cookies = new List<uint>();
        using var lifetime = new ServerLifetime();
        try
        {
            foreach (var (verb, clsid) in HelperIdentity.VerbClasses)
            {
                var id = clsid;
                hr = CoRegisterClassObject(ref id, new ArchiveSelectionClassFactory(verb, forwarder, log, lifetime),
                    ClsctxLocalServer, RegclsMultipleUse | RegclsSuspended, out var cookie);
                if (hr < 0) throw new COMException("Class object registration failed.", hr);
                cookies.Add(cookie);
            }
            hr = CoResumeClassObjects();
            if (hr < 0) throw new COMException("Class object resume failed.", hr);
            log.Write("shell_selection.server_started", correlation, new Dictionary<string, object?> { ["classes"] = cookies.Count });
            Pump(lifetime);
            return 0;
        }
        catch (COMException error)
        {
            log.Write("shell_selection.server_failed", correlation, new Dictionary<string, object?> { ["native_error_code"] = error.ErrorCode });
            return 3;
        }
        finally
        {
            lifetime.Shutdown();
            foreach (var cookie in cookies) CoRevokeClassObject(cookie);
            log.Write("shell_selection.server_stopped", correlation, new Dictionary<string, object?>
            {
                ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds,
            });
            CoUninitialize();
        }
    }

    private static void Pump(ServerLifetime lifetime) =>
        PumpUntilIdle(lifetime, InitialIdle, ServerLifetime.MaxServerLifetime);

    /// <summary>Production decision boundary; bounds are parameters so tests stay short.</summary>
    internal static void PumpUntilIdle(ServerLifetime lifetime, TimeSpan initialIdle, TimeSpan hardLifetime)
    {
        // Wait for the first activation, then until every object/lock is released.
        var deadline = DateTime.UtcNow + initialIdle;
        var hardStop = DateTime.UtcNow + hardLifetime;
        var handles = new[] { lifetime.Idle.SafeWaitHandle.DangerousGetHandle() };
        var activated = false;
        while (true)
        {
            var remaining = Math.Max(0, (int)(deadline - DateTime.UtcNow).TotalMilliseconds);
            var wait = MsgWaitForMultipleObjects(1, handles, false, activated ? 1000u : (uint)remaining, 0x04FF);
            while (PeekMessage(out var message, IntPtr.Zero, 0, 0, 1))
            {
                TranslateMessage(ref message);
                DispatchMessage(ref message);
            }
            // F-AC02: a held LockServer(true) is an activation exactly like a live object.
            var held = lifetime.LiveObjects > 0 || lifetime.ServerLocks > 0;
            if (held) activated = true;
            if (activated && wait == 0 && !held) return;
            if (!activated && DateTime.UtcNow >= deadline) return;
            if (DateTime.UtcNow >= hardStop) return; // explicit hard server lifetime policy
        }
    }
}
