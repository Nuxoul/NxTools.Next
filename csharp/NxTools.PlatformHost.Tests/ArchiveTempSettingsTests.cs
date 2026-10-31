using System.Diagnostics;
using System.Text.Json;
using System.Text.Json.Nodes;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects audit: GUID-owned synthetic files/native directory handles only. Injected Shell
// start/child completion; never invokes real Shell, processes, Registry, COM or product workers.
internal static class ArchiveTempSettingsTests
{
    private static void Check(bool value, string reason) { if (!value) throw new InvalidOperationException(reason); }
    private static ProtocolEnvelope Envelope(object command, Guid operation) => ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data = command }, operation);
    private static JsonObject Materialize(string root) => JsonSerializer.SerializeToNode(new {
        version = 1, operation_id = Guid.NewGuid(), session_id = Guid.NewGuid(), generation = 1, action = "materialize",
        path = Path.Combine(root, "source.zip"), ordinal = 0,
        browser = new { snapshot_id = Guid.NewGuid(), format = "Zip", comment = "", source = new { native = "v1:1:1:1:1", digest = new uint[32], size = 1, modified_unix_ns = 1 } }
    })!.AsObject();
    private static ArchiveEntryCommand Parse(JsonObject command) => ArchiveEntryWire.Parse(Envelope(command, command["operation_id"]!.GetValue<Guid>()));
    private static void Reject(JsonObject command) {
        try { Parse(command); } catch (Exception failure) when (failure is InvalidDataException or JsonException or ArchiveOutputException) { return; }
        throw new InvalidOperationException("unsafe temp/open wire accepted");
    }
    public static Task StrictPolicyWire()
    {
        var command = Materialize(@"C:\owned");
        Check(Parse(command).Action == "materialize", "legacy materialize default");
        foreach (var hours in new[] { 1, 6, 24, 72, 168 }) {
            command["temp_policy"] = JsonSerializer.SerializeToNode(new { base_directory = "", retention_hours = hours, cleanup_on_child_exit = true });
            Check(Parse(command).Action == "materialize", "valid immutable policy rejected");
            foreach (var basePath in new[] { @"C:\", @"C:\owned", @"C:\owned\" }) {
                command["temp_policy"] = JsonSerializer.SerializeToNode(new { base_directory = basePath, retention_hours = hours, cleanup_on_child_exit = true });
                Check(Parse(command).TempPolicy!.RetentionHours == hours, "valid drive-qualified base rejected");
            }
        }
        foreach (var path in new[] { "relative", @"C:relative", @"\\server\share", @"\\?\C:\temp", @"C:\safe\..\other", @"C:\safe\.", @"C:\safe\CON", @"C:\safe\ads:x", "C:/temp", @"C:\bad ", @"C:\safe\\double" }) {
            command["temp_policy"] = JsonSerializer.SerializeToNode(new { base_directory = path, retention_hours = 24, cleanup_on_child_exit = false }); Reject(command);
        }
        foreach (var policy in new[] { "null", "{}", "{\"base_directory\":\"\",\"retention_hours\":2,\"cleanup_on_child_exit\":false}", "{\"base_directory\":\"\",\"retention_hours\":24,\"cleanup_on_child_exit\":false,\"extra\":1}", "{\"base_directory\":\"\",\"retention_hours\":24,\"retention_hours\":6,\"cleanup_on_child_exit\":false}" }) {
            var raw = command.DeepClone().AsObject(); raw.Remove("temp_policy");
            var json = raw.ToJsonString(); json = json[..^1] + ",\"temp_policy\":" + policy + "}";
            using var document = JsonDocument.Parse(json);
            try { ArchiveEntryWire.Parse(Envelope(document.RootElement, raw["operation_id"]!.GetValue<Guid>())); }
            catch (Exception failure) when (failure is JsonException or InvalidDataException or ArchiveOutputException) { continue; }
            throw new InvalidOperationException("incomplete/duplicate/invalid policy accepted");
        }
        command["action"] = "authorize"; command.Remove("ordinal"); command["password_interaction"] = true;
        command["temp_policy"] = JsonSerializer.SerializeToNode(new { base_directory = "", retention_hours = 24, cleanup_on_child_exit = false }); Reject(command);
        return Task.CompletedTask;
    }
    public static Task StrictLeaseOpenWire()
    {
        var command = Materialize(@"C:\owned"); var lease = Guid.NewGuid();
        foreach (var key in new[] { "path", "ordinal", "browser" }) command.Remove(key);
        command["action"] = "open"; command["lease_id"] = lease;
        Check(Parse(command).Action == "open", "lease-scoped open missing");
        foreach (var key in new[] { "path", "executable", "uri", "password", "temp_policy", "browser", "ordinal" }) {
            var invalid = command.DeepClone().AsObject(); invalid[key] = key == "temp_policy" ? new JsonObject() : JsonValue.Create("forbidden"); Reject(invalid);
        }
        command.Remove("lease_id"); Reject(command);
        return Task.CompletedTask;
    }
    private sealed class Clock : TimeProvider {
        internal DateTimeOffset Now = new(2026, 10, 25, 0, 0, 0, TimeSpan.Zero);
        public override DateTimeOffset GetUtcNow() => Now;
    }
    private static (ArchiveEntryCache.Lease Lease, string Path) Sealed(ArchiveEntryCache cache, ArchiveTempPolicy? policy = null) {
        var lease = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1, tempPolicy: policy);
        var path = Path.Combine(lease.Root, "files", "selected.txt"); File.WriteAllText(path, "sealed synthetic payload"); lease.Seal(path); return (lease, path);
    }
    private static ArchiveEntryCommand Control(ArchiveEntryCache.Lease lease, string action) => new() {
        Version = 1, OperationId = lease.Operation, SessionId = lease.Session, Generation = lease.Generation, Action = action, LeaseId = lease.Id
    };
    private static async Task<ArchiveEntryResult> Send(ArchiveEntrySession session, ArchiveEntryCache.Lease lease, string action) {
        var command = Control(lease, action); var reply = await session.Dispatch(Envelope(command, command.OperationId)).WaitAsync(TimeSpan.FromSeconds(5));
        return JsonSerializer.Deserialize<ArchiveEntryResult>(reply.Data!, JsonDefaults.Options)!;
    }
    public static async Task PlacementRetentionAndRestart()
    {
        var root = ArchiveProductReadTests.Root(); var clock = new Clock(); var chosen = Path.Combine(root, "chosen");
        Directory.CreateDirectory(chosen); File.WriteAllText(Path.Combine(chosen, "foreign.txt"), "never disposable");
        try {
            string one, six, legacy;
            using (var cache = ArchiveEntryCache.ForOwnedTestCache(root, clock)) {
                await cache.StartupCleanup;
                var a = Sealed(cache, new(chosen, 1, true)); one = a.Path;
                Check(a.Lease.Root.StartsWith(Path.Combine(chosen, "NxTools.Next", "archive", "sessions") + "\\", StringComparison.OrdinalIgnoreCase), "custom root not applied");
                six = Sealed(cache, new(Path.Combine(root, "second"), 6, false)).Path;
                legacy = Sealed(cache).Path;
                Check(legacy.StartsWith(Path.Combine(root, "archive", "sessions") + "\\", StringComparison.OrdinalIgnoreCase), "legacy root changed");
                foreach (var hours in new uint[] { 1, 6, 24, 72, 168 }) {
                    var snapshot = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1, tempPolicy: new("", hours, true));
                    Check(snapshot.Expires == clock.Now.AddHours(hours) && snapshot.Policy.CleanupOnChildExit, "native Create did not snapshot exact policy");
                    Check(cache.Release(snapshot.Id), "empty explicit release should not wait retention");
                }
                foreach (var invalid in new[] { new ArchiveTempPolicy(@"C:\safe\..\escape", 24, false), new ArchiveTempPolicy("", 2, false) }) {
                    try { cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1, tempPolicy: invalid); throw new InvalidOperationException("native cache bypassed policy validation"); }
                    catch (Exception failure) when (failure is InvalidDataException or ArchiveOutputException) { }
                }
                clock.Now += TimeSpan.FromHours(2); cache.Sweep(); Check(File.Exists(one), "live manifest ignored");
            }
            clock.Now -= TimeSpan.FromHours(1) + TimeSpan.FromTicks(1);
            using (var early = ArchiveEntryCache.ForOwnedTestCache(root, clock)) { await early.StartupCleanup; for (var i = 0; i < 12; i++) early.Sweep(); Check(File.Exists(one), "retention before exact eligibility"); }
            clock.Now += TimeSpan.FromTicks(1);
            using (var restart = ArchiveEntryCache.ForOwnedTestCache(root, clock)) { await restart.StartupCleanup; for (var i = 0; i < 12; i++) restart.Sweep(); Check(!File.Exists(one) && File.Exists(six) && File.Exists(legacy), "root-switch restart or independent expiry failed"); }
            clock.Now += TimeSpan.FromHours(23);
            using (var restart = ArchiveEntryCache.ForOwnedTestCache(root, clock)) { await restart.StartupCleanup; for (var i = 0; i < 12; i++) restart.Sweep(); Check(!File.Exists(six) && !File.Exists(legacy), "default24h/custom6h eligibility"); }
            Check(File.ReadAllText(Path.Combine(chosen, "foreign.txt")) == "never disposable" && Directory.Exists(chosen), "chosen base deleted");
        } finally { Directory.Delete(root, true); }
    }
    public static async Task BoundedRootInventoryAndDiscovery()
    {
        var root = ArchiveProductReadTests.Root(); var clock = new Clock(); var paths = new List<string>();
        try {
            using (var cache = ArchiveEntryCache.ForOwnedTestCache(root, clock)) {
                await cache.StartupCleanup;
                for (var index = 0; index < ArchiveEntryCache.MaxCustomRoots; index++) paths.Add(Sealed(cache, new(Path.Combine(root, "base" + index), 1, false)).Path);
                try { Sealed(cache, new(Path.Combine(root, "overflow"), 1, false)); throw new InvalidOperationException("inventory overflow silently forgot root"); }
                catch (ArchiveOutputException failure) { Check(failure.Stage == "resource_limit", "overflow must fail closed"); }
                Check(!Directory.Exists(Path.Combine(root, "overflow")), "overflow created custom namespace");
            }
            clock.Now += TimeSpan.FromHours(1);
            using (var restart = ArchiveEntryCache.ForOwnedTestCache(root, clock)) {
                await restart.StartupCleanup; Check(paths.Any(File.Exists), "startup exceeded eight-entry global budget");
                for (var batch = 0; batch < 64; batch++) restart.Sweep(); Check(paths.All(path => !File.Exists(path)), "multi-root rotation stranded manifests");
            }
        } finally { Directory.Delete(root, true); }
    }
    public static async Task ReparseAndInventoryTamperPreserved()
    {
        var root = ArchiveProductReadTests.Root(); var target = Path.Combine(root, "target"); var link = Path.Combine(root, "link"); Directory.CreateDirectory(target);
        File.WriteAllText(Path.Combine(target, "foreign.txt"), "foreign");
        try {
            Directory.CreateSymbolicLink(link, target);
            using (var cache = ArchiveEntryCache.ForOwnedTestCache(root)) {
                await cache.StartupCleanup;
                try { Sealed(cache, new(link, 24, false)); throw new InvalidOperationException("reparse ancestry admitted"); }
                catch (ArchiveOutputException) { }
                Check(!Directory.Exists(Path.Combine(target, "NxTools.Next")), "followed reparse before reject");
                Sealed(cache, new(Path.Combine(root, "safe"), 1, false));
            }
            var inventory = Path.Combine(root, "archive", "temp-roots", "0.json"); File.WriteAllText(inventory, "malformed inventory");
            using (var restart = ArchiveEntryCache.ForOwnedTestCache(root)) {
                await restart.StartupCleanup;
                try { Sealed(restart, new(Path.Combine(root, "new-base"), 24, false)); throw new InvalidOperationException("corrupt inventory overwritten"); }
                catch (Exception failure) when (failure is JsonException or InvalidDataException or ArchiveOutputException) { }
            }
            Check(File.ReadAllText(inventory) == "malformed inventory" && File.ReadAllText(Path.Combine(target, "foreign.txt")) == "foreign", "foreign data/inventory replaced");
        } finally { if (Directory.Exists(link)) Directory.Delete(link); Directory.Delete(root, true); }
    }
    private sealed class Child : ArchiveEntryChild {
        internal readonly TaskCompletionSource Done = new(TaskCreationOptions.RunContinuationsAsynchronously);
        internal readonly TaskCompletionSource Cancelled = new(TaskCreationOptions.RunContinuationsAsynchronously);
        internal bool Disposed;
        internal readonly TaskCompletionSource DisposedSignal = new(TaskCreationOptions.RunContinuationsAsynchronously);
        public override async Task WaitForExitAsync(CancellationToken token) {
            try { await Done.Task.WaitAsync(token); } catch (OperationCanceledException) { Cancelled.TrySetResult(); throw; }
        }
        public override void Dispose() { Disposed = true; DisposedSignal.TrySetResult(); }
    }
    public static async Task LiveChildBrowserReleaseAndPromptReturn()
    {
        var root = ArchiveProductReadTests.Root();
        try {
            using var cache = ArchiveEntryCache.ForOwnedTestCache(root); var file = Sealed(cache, new("", 1, false)); var child = new Child();
            await using var session = ArchiveEntrySession.ForOwnedTestOpen(root, cache, path => { Check(path == file.Path, "launcher supplied non-lease target"); return child; });
            Check((await Send(session, file.Lease, "open")).Success && !child.Done.Task.IsCompleted, "open waited for child");
            var pending = await Send(session, file.Lease, "release"); Check(pending.Success && pending.Retained && File.Exists(file.Path), "browser release deleted live child");
            cache.Sweep(); Check(File.Exists(file.Path), "sweep bypassed external hold");
            child.Done.SetResult(); await session.ExternalObservationsForFixture.WaitAsync(TimeSpan.FromSeconds(5));
            Check(!File.Exists(file.Path) && child.Disposed, "browser-requested retirement missing on exit");
            Check((await Send(session, file.Lease, "release")).Success, "duplicate release not idempotent");
        } finally { Directory.Delete(root, true); }
    }
    public static async Task ExitOptionOnOffAndAlreadyExited()
    {
        var root = ArchiveProductReadTests.Root();
        try {
            foreach (var cleanup in new[] { false, true }) {
                using var cache = ArchiveEntryCache.ForOwnedTestCache(root); var file = Sealed(cache, new("", 24, cleanup)); var child = new Child();
                await using var session = ArchiveEntrySession.ForOwnedTestOpen(root, cache, _ => child);
                Check((await Send(session, file.Lease, "open")).Success, "open refused");
                child.Done.SetResult(); await session.ExternalObservationsForFixture.WaitAsync(TimeSpan.FromSeconds(5));
                Check(File.Exists(file.Path) == !cleanup, "cleanup-on-exit snapshot not applied");
                Check((await Send(session, file.Lease, "release")).Success && !File.Exists(file.Path), "browser release after exit");
            }
            using var immediateCache = ArchiveEntryCache.ForOwnedTestCache(root); var immediate = Sealed(immediateCache, new("", 24, true)); var exited = new Child(); exited.Done.SetResult();
            await using var immediateSession = ArchiveEntrySession.ForOwnedTestOpen(root, immediateCache, _ => exited);
            Check((await Send(immediateSession, immediate.Lease, "open")).Success, "already-exited launch failed");
            await immediateSession.ExternalObservationsForFixture.WaitAsync(TimeSpan.FromSeconds(5)); Check(!File.Exists(immediate.Path), "already-exited child lost retirement");
        } finally { Directory.Delete(root, true); }
    }
    public static async Task NoHandleLaunchFailureAndNativeCapture()
    {
        var root = ArchiveProductReadTests.Root();
        try {
            foreach (var fails in new[] { false, true }) {
                using var cache = ArchiveEntryCache.ForOwnedTestCache(root); var file = Sealed(cache, new("", 24, true));
                await using var session = ArchiveEntrySession.ForOwnedTestOpen(root, cache, _ => fails ? throw new System.ComponentModel.Win32Exception(5) : null);
                var opened = await Send(session, file.Lease, "open"); Check(opened.Success == !fails && (!fails || opened.NativeErrorCode == 5), "launch error/native code lost");
                Check(File.Exists(file.Path), "untrackable/failed launch interpreted as exit");
                Check((await Send(session, file.Lease, "release")).Success && !File.Exists(file.Path), "no-handle original fallback blocked");
            }
            var target = Path.Combine(root, "native-boundary.txt"); File.WriteAllText(target, "owned"); var captured = false; var started = false;
            var result = new ShellTarget(ShellTargetKind.Open, target).LaunchCaptured(process => { captured = true; Check(process is null, "invented child handle"); }, info => {
                started = true; Check(info.FileName == target && info.UseShellExecute && info.Arguments == "", "ordinary Shell semantics changed"); return null;
            });
            Check(started && captured && result.Success, "real Shell start capture seam not used");
            using var unassociated = new Process(); Check(ArchiveEntryChild.Capture(unassociated) is null, "unusable native Process is not a tracked child");
        } finally { Directory.Delete(root, true); }
    }
    public static async Task OpenIdentityAndEditedLockedExit()
    {
        var root = ArchiveProductReadTests.Root();
        try {
            using var cache = ArchiveEntryCache.ForOwnedTestCache(root); var file = Sealed(cache, new("", 24, true)); var launches = 0; var child = new Child();
            await using var session = ArchiveEntrySession.ForOwnedTestOpen(root, cache, _ => { launches++; return child; });
            var wrong = Control(file.Lease, "open") with { Generation = 2 };
            Check(!(await session.Dispatch(Envelope(wrong, wrong.OperationId))).Success && launches == 0, "stale lease launched");
            foreach (var bad in new[] { Control(file.Lease, "open") with { OperationId = Guid.NewGuid() }, Control(file.Lease, "open") with { SessionId = Guid.NewGuid() }, Control(file.Lease, "open") with { LeaseId = Guid.NewGuid() } })
                Check(!(await session.Dispatch(Envelope(bad, bad.OperationId))).Success && launches == 0, "foreign lease identity launched");
            var unsealed = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1);
            Check(!(await Send(session, unsealed, "open")).Success && launches == 0, "unsealed output launched");
            File.WriteAllText(file.Path, "modified before open");
            Check(!(await Send(session, file.Lease, "open")).Success && launches == 0, "changed sealed output launched");
            var replaced = Sealed(cache); var originalRoot = replaced.Lease.Root;
            Directory.Move(Path.Combine(originalRoot, "files"), Path.Combine(originalRoot, "files-original")); Directory.CreateDirectory(Path.Combine(originalRoot, "files"));
            File.WriteAllText(replaced.Path, "foreign replacement");
            Check(!(await Send(session, replaced.Lease, "open")).Success && launches == 0 && File.ReadAllText(replaced.Path) == "foreign replacement", "replacement root acquired Shell authority");
            foreach (var edited in new[] { false, true }) {
                using var retainedCache = ArchiveEntryCache.ForOwnedTestCache(root);
                var retained = Sealed(retainedCache, new("", 24, true)); var viewer = new Child();
                await using var owner = ArchiveEntrySession.ForOwnedTestOpen(root, retainedCache, _ => viewer);
                Check((await Send(owner, retained.Lease, "open")).Success, "safe file rejected");
                if (edited) File.WriteAllText(retained.Path, "external edits");
                using (var locked = edited ? null : File.Open(retained.Path, FileMode.Open, FileAccess.Read, FileShare.Read)) {
                    viewer.Done.SetResult(); await owner.ExternalObservationsForFixture.WaitAsync(TimeSpan.FromSeconds(5));
                    Check(File.Exists(retained.Path), "exit bypassed edit/lock verification");
                }
                if (!edited) { retainedCache.Sweep(); Check(!File.Exists(retained.Path), "unlocked retired output unrecoverable"); }
                else Check(File.ReadAllText(retained.Path) == "external edits", "modified data lost");
            }
        } finally { Directory.Delete(root, true); }
    }
    public static async Task DisconnectCancelsObservationWithoutDrain()
    {
        var root = ArchiveProductReadTests.Root(); var clock = new Clock();
        try {
            var cache = ArchiveEntryCache.ForOwnedTestCache(root, clock); var file = Sealed(cache, new("", 1, true)); var child = new Child();
            var session = ArchiveEntrySession.ForOwnedTestOpen(root, cache, _ => child);
            Check((await Send(session, file.Lease, "open")).Success, "open failed");
            Check((await Send(session, file.Lease, "release")).Retained, "known live child deleted");
            session.Stop(); session.Stop(); await session.DisposeAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(5));
            await child.Cancelled.Task.WaitAsync(TimeSpan.FromSeconds(5)); await session.ExternalObservationsForFixture.WaitAsync(TimeSpan.FromSeconds(5));
            Check(File.Exists(file.Path) && File.Exists(Path.Combine(file.Lease.Root, "manifest.json")) && child.Disposed && !child.Done.Task.IsCompleted, "disconnect killed/waited/deleted child or lost recovery");
            using (var early = ArchiveEntryCache.ForOwnedTestCache(root, clock)) { await early.StartupCleanup; Check(File.Exists(file.Path), "disconnect prematurely retired held manifest"); }
            clock.Now += TimeSpan.FromHours(1);
            using (var recovery = ArchiveEntryCache.ForOwnedTestCache(root, clock)) { await recovery.StartupCleanup; recovery.Sweep(); Check(!File.Exists(file.Path), "held manifest orphan recovery lost"); }
        } finally { Directory.Delete(root, true); }
    }
    public static async Task LaunchReleaseRaceAndDetachedDrain()
    {
        var root = ArchiveProductReadTests.Root();
        using var launchEntered = new ManualResetEventSlim(); using var finishLaunch = new ManualResetEventSlim();
        try {
            var cache = ArchiveEntryCache.ForOwnedTestCache(root); var file = Sealed(cache, new("", 24, true)); var child = new Child();
            var session = ArchiveEntrySession.ForOwnedTestOpen(root, cache, _ => {
                launchEntered.Set(); Check(finishLaunch.Wait(TimeSpan.FromSeconds(5)), "fixture launch not unblocked"); return child;
            });
            var command = Control(file.Lease, "open"); var launching = session.Dispatch(Envelope(command, command.OperationId));
            Check(launchEntered.Wait(TimeSpan.FromSeconds(5)), "detached launcher never entered");
            var release = await Send(session, file.Lease, "release"); Check(release.Success && release.Retained && File.Exists(file.Path), "release raced native launch hold");
            session.Stop();
            Check(!JsonSerializer.Deserialize<ArchiveEntryResult>((await launching.WaitAsync(TimeSpan.FromSeconds(2))).Data!, JsonDefaults.Options)!.Success, "disconnect left Shell receipt blocked on native launch");
            await session.DisposeAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(2));
            Check(File.Exists(file.Path), "drain deleted still-launching target");
            finishLaunch.Set();
            await child.DisposedSignal.Task.WaitAsync(TimeSpan.FromSeconds(5));
            Check(File.Exists(file.Path), "late native completion deleted disconnected lease");
        } finally { finishLaunch.Set(); Directory.Delete(root, true); }
    }
}
