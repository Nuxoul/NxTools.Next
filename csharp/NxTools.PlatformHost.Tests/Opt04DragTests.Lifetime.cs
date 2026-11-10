using System.Collections;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;
using static NxTools.PlatformHost.Tests.ArchiveVirtualDragTests;

namespace NxTools.PlatformHost.Tests;

internal static partial class Opt04DragTests
{
    // Original runner handles and owned native pipes only; never name-based process discovery.
    internal static async Task RetainedIdleWorkAndFailureLifetime()
    {
        foreach (var mode in new[] { "seal_timeout", "idle", "work_timeout", "seal_cancel", "worker_exit", "input_eof" }) {
            var root = Root(false); var helper = InstallWorker(root);
            try {
                var browser = await Browse(root); var command = Command(browser, root);
                var log = new Events(); STGMEDIUM held = default; Exception? consumerFailure = null;
                using var reached = new ManualResetEventSlim(); using var release = new ManualResetEventSlim();
                await using (var entries = ArchiveEntrySession.ForOwnedTestPackage(root, log)) {
                    var runner = (ArchiveWorkerRunner)ArchiveResourceProbe.Field(entries, "runner");
                    if (mode == "work_timeout") entries.ProbeWriteForFixture(_ => {
                        reached.Set(); Check(release.Wait(TimeSpan.FromSeconds(40)), "bounded native write probe");
                    });
                    if (mode is "seal_cancel" or "seal_timeout") entries.SealedCheckpointForFixture(() => {
                        reached.Set(); Check(release.Wait(TimeSpan.FromSeconds(mode == "seal_timeout" ? 40 : 15)), "bounded original Seal probe");
                    });
                    await using (var drag = new ArchiveVirtualDragSession(entries, log, validSource: (_, _) => true,
                        prepare: _ => null, pressed: () => true, escape: () => false, loop: (data, _) => {
                            var pointer = Marshal.GetComInterfaceForObject(data, typeof(System.Runtime.InteropServices.ComTypes.IDataObject));
                            try {
                                var get = Slot<NativeGetData>(pointer, 3);
                                var format = Format(ArchiveVirtualFileDataObject.ContentsFormat, 0, TYMED.TYMED_ISTREAM);
                                var result = get(pointer, ref format, out held);
                                if (mode is "work_timeout" or "seal_cancel" or "seal_timeout") {
                                    var supplied = result >= 0 || held.unionmember != IntPtr.Zero;
                                    if (held.unionmember != IntPtr.Zero) { ReleaseStgMedium(ref held); held = default; }
                                    Check(!supplied, "failed or cancelled work cannot supply medium: " + mode);
                                    return (0x00040101, 0);
                                }
                                Check(result == 0 && ReadBytes(ref held, 1)[0] == Payload(0)[0], "actual first ready native cursor");
                                Check(WorkerActive(entries, log) && WorkerTerminals(log).Length == 0, "ready retains original active Run");
                                var workerId = log.Named("archive.worker.started").Single().Id!.Value;
                                if (mode == "idle") {
                                    var idle = Stopwatch.StartNew(); Thread.Sleep(TimeSpan.FromSeconds(31));
                                    Check(idle.ElapsedMilliseconds >= 31000 && WorkerActive(entries, log) && WorkerTerminals(log).Length == 0,
                                        "receiver idle is not the original 30-second work deadline");
                                    format.lindex = 15;
                                    Check(get(pointer, ref format, out var next) == 0, "new demand after 31 seconds idle");
                                    try { Check(ReadBytes(ref next, MemberSize).SequenceEqual(Payload(15)), "second original ordinal after idle"); }
                                    finally { ReleaseStgMedium(ref next); }
                                    Check(log.Named("archive.worker.started").Length == 1, "idle never restarts worker");
                                    return (0x00040100, 1);
                                }
                                if (mode == "worker_exit") {
                                    using var worker = ArchiveResourceProbe.Worker(runner, workerId, helper);
                                    worker.Kill(); worker.WaitForExit(5000);
                                } else {
                                    lock (ArchiveResourceProbe.Field(runner, "gate")) {
                                        var active = (IDictionary)ArchiveResourceProbe.Field(runner, "active");
                                        var resources = (List<IDisposable>)ArchiveResourceProbe.Field(active[workerId]!, "Resources");
                                        var input = resources.OfType<FileStream>().Single(stream => stream.CanWrite && !stream.CanRead);
                                        input.Dispose();
                                    }
                                }
                                var deadline = Stopwatch.StartNew();
                                while (WorkerTerminals(log).Length == 0 && deadline.Elapsed < TimeSpan.FromSeconds(10)) Thread.Sleep(10);
                                Check(WorkerTerminals(log).Length == 1, "worker exit/EOF reaches actual terminal");
                                format.lindex = 1;
                                Check(get(pointer, ref format, out var late) < 0 && late.unionmember == IntPtr.Zero, "late demand refused after failed original Run");
                                Check(log.Named("archive.worker.started").Length == 1, "failure cannot launch hidden replacement worker");
                                return (0x00040100, 1);
                            } catch (Exception failure) { consumerFailure = failure; throw; }
                            finally { Marshal.Release(pointer); }
                        })) {
                        var terminalTask = drag.Dispatch(Wire(command), 1);
                        try {
                            if (mode is "work_timeout" or "seal_cancel" or "seal_timeout") {
                                Check(await Task.Run(() => reached.Wait(TimeSpan.FromSeconds(15))), "actual work/Seal boundary reached");
                                if (mode is "work_timeout" or "seal_timeout") {
                                    await Task.Delay(TimeSpan.FromSeconds(31));
                                    Check(log.Named("archive.worker.failed").Length == 0 && runner.ActiveCount == 1,
                                        "blocked pump retains slot before actual resource drain");
                                    Check(!terminalTask.IsCompleted, "no terminal before blocked resource drains");
                                } else {
                                    var cancel = Task.Run(() => drag.Dispatch(Wire(command with { Action = "cancel", SourceWindowHandle = 0, Items = null }), 1));
                                    await Task.Delay(100);
                                    Check(!terminalTask.IsCompleted, "no terminal before original Seal releases");
                                    release.Set();
                                    Check((await cancel.WaitAsync(TimeSpan.FromSeconds(5))).Success, "exact cancellation while original Seal is running");
                                }
                            }
                        } finally { release.Set(); }
                        var terminal = await terminalTask.WaitAsync(TimeSpan.FromSeconds(mode == "idle" ? 60 : 20));
                        Check(consumerFailure is null, "controlled consumer " + mode + ": " + consumerFailure?.Message);
                        Check(mode == "idle" ? terminal.Success && terminal.Data!.Contains("dropped") :
                            !terminal.Success || terminal.Data!.Contains("cancelled"), "truthful drag outcome " + mode + "/" + terminal.Error);
                        Check(entries.ActiveWorkerCountForFixture == 0 && !WorkerActive(entries, log), "actual terminal releases original slot");
                        var workers = WorkerTerminals(log);
                        Check(workers.Length == 1 && workers[0].Fields.GetProperty("cleanup_complete").GetBoolean(), "one real drained worker terminal");
                        if (mode is "work_timeout" or "seal_timeout") Check(workers[0].Fields.GetProperty("category").GetString() == "timeout", "original 30-second work timeout remains enforced");
                        using (File.Open(Path.Combine(root, "sixteen.zip"), FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
                        if (held.unionmember != IntPtr.Zero) Check(ReadBytes(ref held, MemberSize - 1).SequenceEqual(Payload(0)[1..]), "transferred target cursor survives source failure/retirement");
                    }
                    if (held.unionmember != IntPtr.Zero) ReleaseStgMedium(ref held);
                }
                Check(!Outputs(root).Any() && !Directory.GetFiles(root, "manifest.json", SearchOption.AllDirectories).Any() &&
                    !Directory.GetFiles(root, ".nxpart-*", SearchOption.AllDirectories).Any(), "owned member outputs and partials released");
                Console.WriteLine("ARCHIVE_OPT04_LIFETIME " + JsonSerializer.Serialize(new { mode, worker_starts = log.Named("archive.worker.started").Length, work = log.Named("archive.worker.work").Select(line => line.Fields) }));
            } finally { if (File.Exists(helper) && !Directory.GetFiles(root, "manifest.json", SearchOption.AllDirectories).Any()) File.Delete(helper); }
        }
    }
}
