using System.ComponentModel;
using System.Diagnostics;
using NxTools.PlatformHost.Windows;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Protocol;

// Only the returned native Process is evidence; never PID/name lookup, kill or polling.
internal abstract class ArchiveEntryChild : IDisposable
{
    public abstract Task WaitForExitAsync(CancellationToken cancellationToken);
    public abstract void Dispose();
    internal static ArchiveEntryChild? Capture(Process? process)
    {
        if (process is null) return null;
        try { _ = process.SafeHandle; return new NativeChild(process); }
        catch (Exception failure) when (failure is InvalidOperationException or Win32Exception or NotSupportedException) { process.Dispose(); return null; }
    }
    private sealed class NativeChild(Process process) : ArchiveEntryChild
    {
        public override Task WaitForExitAsync(CancellationToken cancellationToken) => process.WaitForExitAsync(cancellationToken);
        public override void Dispose() => process.Dispose();
    }
}

internal sealed partial class ArchiveEntrySession
{
    private static int nativeLaunches;
    private readonly CancellationTokenSource externalStop = new();
    private readonly Dictionary<Guid, ExternalUse> external = [];
    private Func<string, ArchiveEntryChild?>? fixtureLauncher;
    private sealed class ExternalUse(ArchiveEntryCommand command)
    {
        internal readonly ArchiveEntryCommand Command = command;
        internal readonly Stopwatch Clock = Stopwatch.StartNew();
        internal readonly TaskCompletionSource<WindowsCommandResult> Done = new(TaskCreationOptions.RunContinuationsAsynchronously);
        internal Task Observation = Task.CompletedTask;
        internal bool CleanupOnExit;
    }
    internal static ArchiveEntrySession ForOwnedTestOpen(string root, ArchiveEntryCache cache, Func<string, ArchiveEntryChild?> launcher)
    {
        ArchiveReadHelper.GuardOwnedRoot(root);
        return new(ArchiveReadHelper.OwnedPackage(root), cache, null) { fixtureLauncher = launcher };
    }
    internal Task ExternalObservationsForFixture { get { lock (gate) return Task.WhenAll(external.Values.Select(item => item.Observation)); } }
    private WindowsCommandResult OpenResult(ExternalUse use, bool success, string? code = null, int? native = null) =>
        ArchiveEntryWire.Result(new(1, use.Command.OperationId, use.Command.SessionId, use.Command.Generation, success, use.Command.LeaseId, null, code, native));
    // Called under gate; admission establishes the cache hold BEFORE a browser release can race.
    private Task<WindowsCommandResult> OpenLease(ArchiveEntryCommand command)
    {
        var use = new ExternalUse(command); var id = command.LeaseId!.Value;
        if (active.ContainsKey(command.OperationId) || external.ContainsKey(id)) return Task.FromResult(OpenResult(use, false, "busy"));
        if (!cache.HoldExternal(id, command.OperationId, command.SessionId, command.Generation)) return Task.FromResult(OpenResult(use, false, "invalid_request"));
        if (Interlocked.Increment(ref nativeLaunches) > 2) {
            Interlocked.Decrement(ref nativeLaunches); cache.EndExternal(id, false);
            return Task.FromResult(OpenResult(use, false, "busy"));
        }
        external.Add(id, use); OpenDiagnostic("accepted", use);
        var thread = new Thread(() => LaunchLease(use)) { IsBackground = true, Name = "NxTools.ArchiveEntry.Open" };
        try { thread.SetApartmentState(ApartmentState.STA); thread.Start(); }
        catch (Exception failure) {
            Interlocked.Decrement(ref nativeLaunches); external.Remove(id); cache.EndExternal(id, false);
            use.Done.TrySetResult(OpenResult(use, false, "io_failure", failure.HResult));
        }
        return use.Done.Task;
    }
    private void LaunchLease(ExternalUse use)
    {
        ArchiveEntryChild? child = null; var id = use.Command.LeaseId!.Value;
        string? code = null; int? native = null;
        try {
            lock (gate) { if (stopped) { use.Done.TrySetResult(OpenResult(use, false, "disconnected")); return; } }
            OpenDiagnostic("started", use);
            using (var output = cache.OpenShellOutput(id, out var path, out use.CleanupOnExit)) {
                if (fixtureLauncher is { } launch) child = launch(path);
                else new ShellTarget(ShellTargetKind.Open, path).LaunchCaptured(process => child = ArchiveEntryChild.Capture(process));
            }
        } catch (ArchiveOutputException failure) { code = failure.Stage is "source_changed" ? "source_changed" : "io_failure"; native = failure.NativeCode; }
        catch (Win32Exception failure) { code = "io_failure"; native = failure.NativeErrorCode; }
        catch (Exception failure) { code = "io_failure"; native = failure.HResult; }
        finally { Interlocked.Decrement(ref nativeLaunches); }
        lock (gate) {
            if (stopped) { child?.Dispose(); use.Done.TrySetResult(OpenResult(use, false, "disconnected")); return; }
            OpenDiagnostic(code is null ? "launched" : "launch_failed", use, code, native);
            if (child is null || code is not null) {
                child?.Dispose(); cache.EndExternal(id, false); external.Remove(id);
                OpenDiagnostic(code is null ? "untrackable" : "terminal", use, code, native);
            } else {
                var tracked = child;
                OpenDiagnostic("tracked", use);
                use.Observation = Task.Run(() => ObserveChild(use, tracked));
            }
            use.Done.TrySetResult(OpenResult(use, code is null, code, native));
        }
    }
    private async Task ObserveChild(ExternalUse use, ArchiveEntryChild child)
    {
        var exited = false;
        try { await child.WaitForExitAsync(externalStop.Token).ConfigureAwait(false); exited = true; }
        catch (OperationCanceledException) when (externalStop.IsCancellationRequested) { }
        catch (Exception failure) { OpenDiagnostic("untrackable", use, "observation_failed", failure is Win32Exception win32 ? win32.NativeErrorCode : failure.HResult); }
        finally {
            child.Dispose();
            lock (gate) {
                // Disconnect is not external exit: leave original non-retired manifest for
                // expiry/recovery and do not make Dispose/drain wait for an arbitrary app.
                if (!stopped) {
                    var removed = cache.EndExternal(use.Command.LeaseId!.Value, exited && use.CleanupOnExit);
                    OpenDiagnostic(exited ? "exited" : "fallback", use);
                    OpenDiagnostic(removed ? "retired" : "retained", use);
                    external.Remove(use.Command.LeaseId.Value);
                }
                else OpenDiagnostic("observation_cancelled", use);
            }
        }
    }
    private void OpenDiagnostic(string phase, ExternalUse use, string? code = null, int? native = null)
    {
        try { log?.Write("info", "archive.entry.open." + phase, use.Command.OperationId, new Dictionary<string, object?> {
            ["generation"] = use.Command.Generation, ["elapsed_ms"] = use.Clock.ElapsedMilliseconds,
            ["category"] = code, ["native_error_code"] = native }); } catch { }
    }
}
