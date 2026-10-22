using System.Collections.Concurrent;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using Microsoft.Win32.SafeHandles;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Extensions;

namespace NxTools.PlatformHost.Windows;

// Explicit version requests only. One long-lived existing runner retains its four bounded
// startup/Job/pump/cleanup slots even when native cleanup cannot complete. No replacement runner.
internal sealed class ExtensionRuntimeInspector : IDisposable
{
    private readonly ConcurrentDictionary<Guid, Probe> pending = new();
    private readonly ExtensionProcessRunner runner;
    private readonly IPlatformLog? log;
    private readonly SemaphoreSlim admission = new(4, 4);

    internal ExtensionRuntimeInspector(IPlatformLog? log = null)
    {
        this.log = log;
        runner = new ExtensionProcessRunner(OnEvent, log, outputLimit: 4096, closeInitialInput: true);
    }

    private sealed class Probe(Guid id, IPlatformLog? log)
    {
        internal readonly Stopwatch Clock = Stopwatch.StartNew();
        internal readonly TaskCompletionSource<ExtensionProcessEvent> Completion = new(TaskCreationOptions.RunContinuationsAsynchronously);
        internal readonly object Gate = new();
        internal string? Version;
        internal bool InvalidOutput;
        internal int CleanupConfirmed, Returned, AdmissionReleased;
        internal void Started() => ExtensionRuntimeDiagnostics.Write(log, "inspect", "started", id, Clock);
    }

    internal async Task<WindowsCommandResult> InspectAsync(InspectExtensionRuntimeCommand command,
        Guid correlationId, CancellationToken cancellationToken)
    {
        var id = correlationId == Guid.Empty ? Guid.NewGuid() : correlationId;
        var probe = new Probe(id, log);
        ExtensionRuntimeDiagnostics.Write(log, "inspect", "accepted", id, probe.Clock);
        var lease = false;
        var started = false;
        var registered = false;
        WindowsCommandResult result;
        try
        {
            cancellationToken.ThrowIfCancellationRequested();
            if (!PlatformCommandParser.IsExtensionRuntimeProgram(command.Program)) throw new ExtensionFailure("invalid_program", 87);
            if (!admission.Wait(0)) throw new ExtensionFailure("busy", 170);
            lease = true;
            var executable = ResolveExecutable(command.Program, command.Executable);
            using var image = OpenImage(executable);
            executable = FinalPath(image.SafeFileHandle);
            ValidateImagePath(executable);
            var before = await HashAsync(image, cancellationToken);
            cancellationToken.ThrowIfCancellationRequested();
            if (!pending.TryAdd(id, probe)) throw new ExtensionFailure("duplicate_request", 87);
            registered = true;
            var request = new ExtensionStartRequest
            {
                RunId = id, PluginId = "runtime-" + id.ToString("N"), Executable = executable,
                Arguments = VersionArguments(command.Program), WorkingDirectory = Path.GetDirectoryName(executable)!,
                TimeoutMs = 3000, MemoryBytes = 256L * 1024 * 1024, MaxProcesses = 2,
                Stdin = "" // Validated JSONL seed; scoped runner closes stdin, sends no code.
            };
            started = runner.Start(request);
            if (!started) throw new ExtensionFailure("admission_failed", 170);
            // Session cancellation targets only this request's Job. Await its bounded cleanup;
            // never race a cancelled WaitAsync against resource/slot disposal.
            using var cancellation = cancellationToken.Register(() => runner.Cancel(id));
            ExtensionProcessEvent terminal;
            try { terminal = await probe.Completion.Task.WaitAsync(TimeSpan.FromMilliseconds(8500)); }
            catch (TimeoutException) { runner.Cancel(id); throw new ExtensionFailure("cleanup_unconfirmed", 1460); }
            if (!terminal.CleanupComplete) throw new ExtensionFailure("cleanup_unconfirmed", terminal.NativeErrorCode);
            cancellationToken.ThrowIfCancellationRequested();
            if (!terminal.Success) throw new ExtensionFailure(FailureClass(terminal.Reason), terminal.NativeErrorCode);
            string? version;
            lock (probe.Gate) version = probe.InvalidOutput ? null : probe.Version;
            if (version is null || !ValidVersion(command.Program, version)) throw new ExtensionFailure("invalid_version");
            // Locked handle disallows write/delete while executing. Reopen exact resolved path and
            // hash again after the runner has observed Job empty and all pumps/resource cleanup.
            using var afterImage = OpenImage(executable);
            if (!string.Equals(FinalPath(afterImage.SafeFileHandle), executable, StringComparison.OrdinalIgnoreCase) ||
                !string.Equals(await HashAsync(afterImage, cancellationToken), before, StringComparison.Ordinal))
                throw new ExtensionFailure("identity_changed");
            result = new(true, Data: JsonSerializer.Serialize(new { program = command.Program, executable, version, sha256 = before }));
        }
        catch (OperationCanceledException) { result = new(false, "extension_runtime_cancelled", NativeErrorCode: 995); }
        catch (ExtensionFailure e) { result = new(false, "extension_runtime_" + e.Reason, NativeErrorCode: e.NativeCode); }
        catch (Win32Exception e) { result = new(false, "extension_runtime_native_failure", NativeErrorCode: e.NativeErrorCode); }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException or ArgumentException or NotSupportedException)
        { result = new(false, "extension_runtime_file_failure", NativeErrorCode: e.HResult); }
        catch (Exception e) when (e is InvalidOperationException or ExternalException or CryptographicException)
        { result = new(false, "extension_runtime_probe_failed", NativeErrorCode: e.HResult); }
        finally
        {
            if (registered && !started) pending.TryRemove(id, out _);
            // Hash verification remains inside admission. On a timed-out native call, its
            // eventual confirmed cleanup alone can release the retained permit.
            Interlocked.Exchange(ref probe.Returned, 1);
            if (lease && (!started || Volatile.Read(ref probe.CleanupConfirmed) != 0)) ReleaseAdmission(probe);
        }
        ExtensionRuntimeDiagnostics.Write(log, "inspect", result.Success ? "completed" : result.NativeErrorCode == 995 ? "cancelled" : "failed",
            id, probe.Clock, result.Error, result.NativeErrorCode);
        return result;
    }

    private void OnEvent(ExtensionProcessEvent value)
    {
        if (!pending.TryGetValue(value.RunId, out var probe)) return;
        if (value.Op == "started") probe.Started();
        if (value.Op is "stdout" or "stderr")
        {
            lock (probe.Gate)
            {
                // Reject rather than echo/truncate arbitrary child text. Never publish output to IPC/log.
                if (value.Op == "stderr" || probe.Version is not null || value.Data is null || value.Data.Length is < 1 or > 128 ||
                    value.Data.Any(c => c is < ' ' or > '~')) probe.InvalidOutput = true;
                else probe.Version = value.Data.Trim();
            }
        }
        if (value.Op == "exited")
        {
            probe.Completion.TrySetResult(value);
            pending.TryRemove(value.RunId, out _);
            if (value.CleanupComplete)
            {
                Interlocked.Exchange(ref probe.CleanupConfirmed, 1);
                if (Volatile.Read(ref probe.Returned) != 0) ReleaseAdmission(probe);
            }
        }
    }

    private void ReleaseAdmission(Probe probe)
    {
        if (Interlocked.Exchange(ref probe.AdmissionReleased, 1) == 0) admission.Release();
    }

    internal static string[] VersionArguments(string program) => program switch
    {
        "python" => ["-I", "-S", "-c", "import sys;print('Python '+'.'.join(map(str,sys.version_info[:3])))"],
        "node" => ["--version"],
        "powershell" or "pwsh" => ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", "$PSVersionTable.PSVersion.ToString()"],
        _ => throw new ExtensionFailure("invalid_program", 87)
    };

    private static bool ValidVersion(string program, string version) => Regex.IsMatch(version, program switch
    {
        "python" => @"\APython [0-9]{1,4}\.[0-9]{1,4}\.[0-9]{1,4}\z",
        "node" => @"\Av[0-9]{1,4}\.[0-9]{1,4}\.[0-9]{1,4}(?:-[A-Za-z0-9.-]{1,64})?\z",
        _ => @"\A[0-9]{1,4}\.[0-9]{1,4}\.[0-9]{1,4}(?:\.[0-9]{1,8}|-[A-Za-z0-9.-]{1,64})?\z"
    }, RegexOptions.CultureInvariant, TimeSpan.FromMilliseconds(50));

    private static string FailureClass(string? reason) => reason switch
    {
        "timeout" => "timeout", "cancelled" or "broker_disconnected" => "cancelled",
        "stdout_total_limit" or "stderr_total_limit" or "output_total_limit" or "stdout_line_limit" or "stderr_line_limit" => "output_limit",
        "process_exit" => "process_exit", "token_not_ordinary" => "ordinary_token_required",
        _ => "probe_failed"
    };

    private static string ResolveExecutable(string program, string configured)
    {
        // Only an absent selection allows discovery; a supplied but invalid selection
        // (including whitespace) must fail rather than silently choose another executable.
        if (!string.IsNullOrEmpty(configured))
        {
            var path = Environment.ExpandEnvironmentVariables(configured.Trim());
            if (path.StartsWith(@"\\?\", StringComparison.Ordinal) && path.Length > 6 && char.IsAsciiLetter(path[4]) && path[5] == ':') path = path[4..];
            ValidateImagePath(path);
            return Path.GetFullPath(path);
        }
        // Fixed runtime filename only: no cwd, registry, shell aliases or PATHEXT.
        // Bound both allocation/scanning and candidate count. Never split an unbounded PATH
        // or treat a character-budget-truncated final segment as a real directory.
        var searchPath = Environment.GetEnvironmentVariable("PATH") ?? "";
        var length = Math.Min(searchPath.Length, 32767);
        var start = 0;
        for (var count = 0; count < 256 && start < length; count++)
        {
            var separator = searchPath.IndexOf(Path.PathSeparator, start, length - start);
            if (separator < 0 && searchPath.Length > length) break;
            var end = separator < 0 ? length : separator;
            var directory = searchPath[start..end].Trim();
            start = end + 1;
            if (directory.Length >= 2 && directory[0] == '"' && directory[^1] == '"') directory = directory[1..^1];
            try
            {
                if (directory.IndexOfAny(Path.GetInvalidPathChars()) >= 0 || !Path.IsPathFullyQualified(directory)) continue;
                // Normalize separators/dot segments before the existing drive/EXE/alias checks.
                var candidate = Path.GetFullPath(Path.Combine(directory, program + ".exe")).Replace('/', '\\');
                ValidateImagePath(candidate);
                if (File.Exists(candidate)) return candidate;
            }
            // Bad PATH entries are not an explicit selection. Skip just this candidate;
            // failures opening/hashing/probing the first valid image still fail closed.
            catch (ExtensionFailure) { }
            catch (Exception e) when (e is ArgumentException or NotSupportedException or IOException or UnauthorizedAccessException) { }
        }
        throw new ExtensionFailure("not_found", 2);
    }

    private static void ValidateImagePath(string path)
    {
        if (path.Length is < 3 or >= 32767 || !char.IsAsciiLetter(path[0]) || path[1] != ':' ||
            !Path.IsPathFullyQualified(path) || !ExtensionRuntimePicker.IsExe(path) || path.Contains('\0') ||
            path.AsSpan(2).Contains(':') || path.Contains("\\WindowsApps\\", StringComparison.OrdinalIgnoreCase))
            throw new ExtensionFailure("invalid_executable", 87);
    }
    private static FileStream OpenImage(string path) => new(path, FileMode.Open, FileAccess.Read, FileShare.Read, 8192, true);
    private static async Task<string> HashAsync(FileStream image, CancellationToken cancellation)
    {
        if (image.Length is < 1 or > 256L * 1024 * 1024) throw new ExtensionFailure("image_size_limit");
        return Convert.ToHexString(await SHA256.HashDataAsync(image, cancellation)).ToLowerInvariant();
    }
    private static string FinalPath(SafeFileHandle image)
    {
        var buffer = new StringBuilder(32768);
        var count = GetFinalPathNameByHandle(image, buffer, (uint)buffer.Capacity, 0);
        if (count == 0) throw new Win32Exception(Marshal.GetLastWin32Error());
        if (count >= buffer.Capacity) throw new ExtensionFailure("invalid_executable", 87);
        var path = buffer.ToString();
        return path.StartsWith(@"\\?\", StringComparison.Ordinal) ? path[4..] : path;
    }
    [DllImport("kernel32.dll", EntryPoint = "GetFinalPathNameByHandleW", ExactSpelling = true, CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern uint GetFinalPathNameByHandle(SafeFileHandle file, StringBuilder path, uint size, uint flags);
    public void Dispose() => runner.Stop(); // Runner workers retain their leased handles until cleanup.
}
