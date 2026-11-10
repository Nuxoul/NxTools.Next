using System.ComponentModel;
using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows;

// The Host lease protects writes; the Desktop identity protects paths already sent over IPC.
internal sealed class ClipboardStagingDirectory : IDisposable
{
    internal const int RecoveryRunLimit = 64;
    internal const int RecoveryFileLimit = 256;
    internal const string OwnerFileName = "owner.json";
    internal static string DefaultRoot => Path.Combine(Path.GetTempPath(), "NxTools.Next", "clipboard-staging");
    private readonly FileStream lease;
    internal string DirectoryPath { get; }

    internal sealed record ProcessIdentity(int Id, long StartedUtcTicks)
    {
        internal static ProcessIdentity Current
        {
            get
            {
                using var process = Process.GetCurrentProcess();
                return new(Environment.ProcessId, process.StartTime.ToUniversalTime().Ticks);
            }
        }
        internal static ProcessIdentity? Read(int id)
        {
            try
            {
                using var process = Process.GetProcessById(id);
                return new(id, process.StartTime.ToUniversalTime().Ticks);
            }
            catch (Exception error) when (error is ArgumentException or InvalidOperationException or Win32Exception) { return null; }
        }
        internal bool IsGone()
        {
            try
            {
                using var process = Process.GetProcessById(Id);
                return process.HasExited || process.StartTime.ToUniversalTime().Ticks != StartedUtcTicks;
            }
            catch (ArgumentException) { return true; }
            catch (Exception error) when (error is InvalidOperationException or Win32Exception) { return false; }
        }
    }
    internal sealed record RunOwner(ProcessIdentity Writer, ProcessIdentity? Consumer);
    internal readonly record struct RecoveryResult(int Runs, int DeletedFiles, int FailedFiles);

    private ClipboardStagingDirectory(string root, RunOwner owner)
    {
        DirectoryPath = Path.Combine(root, Guid.NewGuid().ToString("N"));
        FileStream? acquired = null;
        Directory.CreateDirectory(DirectoryPath);
        var marker = Path.Combine(DirectoryPath, OwnerFileName);
        try
        {
            lease = new FileStream(marker, FileMode.CreateNew, FileAccess.ReadWrite, FileShare.Read);
            acquired = lease;
            lease.Write(JsonSerializer.SerializeToUtf8Bytes(owner));
            lease.Flush(flushToDisk: true);
        }
        catch
        {
            acquired?.Dispose();
            TryRemoveEmptyRun(DirectoryPath);
            throw;
        }
    }

    internal static ClipboardStagingDirectory Create(string root, int? consumerProcessId = null) =>
        Create(root, new RunOwner(ProcessIdentity.Current,
            consumerProcessId is { } id ? ProcessIdentity.Read(id) : null));

    internal static ClipboardStagingDirectory Create(string root, RunOwner owner) => new(root, owner);

    internal static RecoveryResult Recover(string root, IPlatformLog? log = null)
    {
        var correlation = Guid.NewGuid();
        var started = Stopwatch.GetTimestamp();
        var runs = 0; var deleted = 0; var failed = 0; var examined = 0;
        try
        {
            Diagnostic("started");
            if (!Directory.Exists(root)) return default;
            foreach (var directory in Directory.EnumerateDirectories(root).Take(RecoveryRunLimit))
            {
                if (!Guid.TryParseExact(Path.GetFileName(directory), "N", out _)) continue;
                var marker = Path.Combine(directory, OwnerFileName);
                try
                {
                    using (var claim = new FileStream(marker, FileMode.Open, FileAccess.ReadWrite, FileShare.None))
                    {
                        if (claim.Length > 1024) continue;
                        var owner = JsonSerializer.Deserialize<RunOwner>(claim);
                        if (owner?.Writer is null || !owner.Writer.IsGone()) continue;
                        // A missing/inaccessible Desktop identity is not permission to delete payloads.
                        if (owner.Consumer is null || !owner.Consumer.IsGone()) continue;
                        runs++;
                        foreach (var file in Directory.EnumerateFiles(directory))
                        {
                            if (examined >= RecoveryFileLimit) break;
                            examined++;
                            var extension = Path.GetExtension(file);
                            if (extension is not (".png" or ".bin") ||
                                !Guid.TryParseExact(Path.GetFileNameWithoutExtension(file), "N", out _)) continue;
                            try { File.Delete(file); deleted++; }
                            catch (Exception error) when (error is IOException or UnauthorizedAccessException) { failed++; }
                        }
                    }
                    TryRemoveEmptyRun(directory);
                }
                catch (Exception error) when (error is IOException or UnauthorizedAccessException or JsonException)
                { /* Active/unknown leases remain untouched and can be retried on a later startup. */ }
                if (examined >= RecoveryFileLimit) break;
            }
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException) { failed++; }
        finally { Diagnostic("completed"); }
        return new(runs, deleted, failed);

        void Diagnostic(string phase)
        {
            try
            {
                log?.Write(failed > 0 ? "warning" : "info", "clipboard.staging.recovery_" + phase, correlation,
                    new Dictionary<string, object?> { ["run_count"] = runs, ["deleted_file_count"] = deleted,
                        ["failed_file_count"] = failed, ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds });
            }
            catch { /* Recovery diagnostics cannot affect capture availability. */ }
        }
    }

    public void Dispose()
    {
        lease.Dispose();
        TryRemoveEmptyRun(DirectoryPath);
    }

    private static void TryRemoveEmptyRun(string directory)
    {
        try
        {
            if (Directory.EnumerateFileSystemEntries(directory).Any(path => Path.GetFileName(path) != OwnerFileName)) return;
            File.Delete(Path.Combine(directory, OwnerFileName));
            Directory.Delete(directory);
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException) { }
    }
}
