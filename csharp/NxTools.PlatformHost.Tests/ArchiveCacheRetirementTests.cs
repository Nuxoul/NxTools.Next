using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static class ArchiveCacheRetirementTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    public static async Task ObstructedRetirementAndRestart()
    {
        var root = ArchiveProductReadTests.Root();
        try {
            string sessionRoot;
            using (var cache = ArchiveEntryCache.ForOwnedTestCache(root)) {
                var lease = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1); sessionRoot = lease.Root;
                var output = Path.Combine(sessionRoot, "selected.txt"); File.WriteAllText(output, "selected"); lease.Seal(output);
                var unknown = Path.Combine(sessionRoot, "viewer-added.txt"); File.WriteAllText(unknown, "unknown");
                Check(!cache.Release(lease.Id), "output deletion is not complete session retirement");
                Check(!File.Exists(output) && File.ReadAllText(unknown) == "unknown", "preserve only unknown obstruction");
                File.Delete(unknown);
                cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1);
                Check(!Directory.Exists(sessionRoot), "next legal sweep must finish owned retirement");
            }
            string restartRoot;
            using (var cache = ArchiveEntryCache.ForOwnedTestCache(root)) {
                var lease = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1); restartRoot = lease.Root;
                var output = Path.Combine(restartRoot, "selected.txt"); File.WriteAllText(output, "selected"); lease.Seal(output);
                File.WriteAllText(Path.Combine(restartRoot, "viewer-added.txt"), "unknown");
                Check(!cache.Release(lease.Id), "retirement must remain recoverable");
            }
            File.Delete(Path.Combine(restartRoot, "viewer-added.txt"));
            using (var restarted = ArchiveEntryCache.ForOwnedTestCache(root)) {
                await restarted.StartupCleanup;
                Check(!Directory.Exists(restartRoot), "restart sweep must recover explicit ownership facts");
            }
            string replacedRoot;
            using (var original = ArchiveEntryCache.ForOwnedTestCache(root)) {
                var lease = original.Create(Guid.NewGuid(), Guid.NewGuid(), 1); replacedRoot = lease.Root;
                var output = Path.Combine(replacedRoot, "selected.txt"); File.WriteAllText(output, "selected"); lease.Seal(output);
                File.WriteAllText(Path.Combine(replacedRoot, "unknown.txt"), "unknown");
                Check(!original.Release(lease.Id), "obstructed root must remain recoverable");
            }
            Directory.Move(replacedRoot, replacedRoot + "-original"); Directory.CreateDirectory(replacedRoot);
            using (var restarted = ArchiveEntryCache.ForOwnedTestCache(root)) { await restarted.StartupCleanup; restarted.Sweep(); }
            Check(Directory.Exists(replacedRoot), "retired record must not authorize replacement directory");
            var orphan = Path.Combine(root, "archive", "sessions", Guid.NewGuid().ToString("D")); Directory.CreateDirectory(orphan);
            using (var restarted = ArchiveEntryCache.ForOwnedTestCache(root)) await restarted.StartupCleanup;
            Check(Directory.Exists(orphan), "GUID directory without manifest is not cleanup authority");
        } finally { Directory.Delete(root, true); }
    }

    private sealed class Clock : TimeProvider
    {
        internal DateTimeOffset Now = new(2026, 10, 21, 0, 0, 0, TimeSpan.Zero);
        public override DateTimeOffset GetUtcNow() => Now;
    }
    public static async Task ExpiryStartupAndRetainedOwners()
    {
        var root = ArchiveProductReadTests.Root(); var clock = new Clock();
        try {
            string expiredPath, editedPath;
            using (var first = ArchiveEntryCache.ForOwnedTestCache(root, clock)) {
                var expired = first.Create(Guid.NewGuid(), Guid.NewGuid(), 1); expiredPath = Path.Combine(expired.Root, "selected.txt"); File.WriteAllText(expiredPath, "baseline"); expired.Seal(expiredPath);
                var edited = first.Create(Guid.NewGuid(), Guid.NewGuid(), 1); editedPath = Path.Combine(edited.Root, "selected.txt"); File.WriteAllText(editedPath, "baseline"); edited.Seal(editedPath); File.WriteAllText(editedPath, "edited");
            }
            using (var early = ArchiveEntryCache.ForOwnedTestCache(root, clock)) {
                await early.StartupCleanup;
                Check(File.Exists(expiredPath), "unexpired output cleaned at startup");
            }
            clock.Now += ArchiveEntryCache.Retention + TimeSpan.FromTicks(1);
            using (var locked = File.Open(expiredPath, FileMode.Open, FileAccess.Read, FileShare.Read)) {
                using var restart = ArchiveEntryCache.ForOwnedTestCache(root, clock); await restart.StartupCleanup;
                Check(File.Exists(expiredPath) && File.ReadAllText(editedPath) == "edited", "expiry bypassed lock or content verification");
            }
            using (var restart = ArchiveEntryCache.ForOwnedTestCache(root, clock)) {
                await restart.StartupCleanup;
                Check(!File.Exists(expiredPath) && File.ReadAllText(editedPath) == "edited", "startup did not clean unlocked expired output or lost edit");
                var active = restart.Create(Guid.NewGuid(), Guid.NewGuid(), 1); var activePath = Path.Combine(active.Root, "active.txt"); File.WriteAllText(activePath, "active"); active.Seal(activePath);
                clock.Now += ArchiveEntryCache.Retention + TimeSpan.FromTicks(1);
                using (var peer = ArchiveEntryCache.ForOwnedTestCache(root, clock)) {
                    await peer.StartupCleanup;
                    Check(File.ReadAllText(activePath) == "active", "active exclusive manifest was cleaned");
                }
                Check(restart.Release(active.Id) && !Directory.Exists(active.Root), "active owner explicit release did not retire");
            }
        } finally { Directory.Delete(root, true); }
    }
    public static async Task AncestryAndMalformedRecovery()
    {
        var root = ArchiveProductReadTests.Root(); var clock = new Clock();
        try {
            using (var cache = ArchiveEntryCache.ForOwnedTestCache(root, clock)) {
                var lease = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1); var files = Path.Combine(lease.Root, "files");
                var output = Path.Combine(files, "selected.txt"); File.WriteAllText(output, "baseline"); lease.Seal(output);
                Directory.Move(files, files + "-original"); Directory.CreateDirectory(files); File.Move(Path.Combine(files + "-original", "selected.txt"), output);
                Check(!cache.Release(lease.Id) && File.ReadAllText(output) == "baseline", "replacement ancestor must reject before output deletion");
            }
            string nestedRoot;
            using (var cache = ArchiveEntryCache.ForOwnedTestCache(root, clock)) {
                var lease = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1); nestedRoot = lease.Root;
                var output = Path.Combine(nestedRoot, "files", "selected.txt"); File.WriteAllText(output, "baseline"); lease.Seal(output);
                File.WriteAllText(Path.Combine(nestedRoot, "files", "unknown.txt"), "unknown");
                Check(!cache.Release(lease.Id) && !File.Exists(output), "nested obstruction must not report retirement");
                Check(File.Exists(nestedRoot + ".retire.json"), "nested obstruction lost fixed-parent recovery authority");
            }
            File.Delete(Path.Combine(nestedRoot, "files", "unknown.txt"));
            using (var restart = ArchiveEntryCache.ForOwnedTestCache(root, clock)) { await restart.StartupCleanup; Check(!Directory.Exists(nestedRoot), "nested retirement restart recovery"); }
            string legacyManifest, nullManifest, eligibleOutput;
            using (var cache = ArchiveEntryCache.ForOwnedTestCache(root, clock)) {
                var legacy = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1); legacyManifest = Path.Combine(legacy.Root, "manifest.json");
                var malformed = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1); nullManifest = Path.Combine(malformed.Root, "manifest.json");
                var eligible = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1); eligibleOutput = Path.Combine(eligible.Root, "files", "selected.txt"); File.WriteAllText(eligibleOutput, "safe"); eligible.Seal(eligibleOutput);
            }
            File.WriteAllText(legacyManifest, "{\"Version\":1}"); File.WriteAllText(nullManifest, "null");
            clock.Now += ArchiveEntryCache.Retention + TimeSpan.FromTicks(1);
            using (var restart = ArchiveEntryCache.ForOwnedTestCache(root, clock)) { await restart.StartupCleanup; restart.Sweep(); }
            Check(File.ReadAllText(legacyManifest) == "{\"Version\":1}" && File.ReadAllText(nullManifest) == "null", "malformed/legacy state was overwritten");
            Check(!File.Exists(eligibleOutput), "malformed manifest prevented later eligible cleanup");
        } finally { Directory.Delete(root, true); }
    }
    public static async Task LocallyRetiredLeasesRotateWithoutStarvation()
    {
        var root = ArchiveProductReadTests.Root();
        try {
            using var cache = ArchiveEntryCache.ForOwnedTestCache(root);
            await cache.StartupCleanup;
            var leases = new List<ArchiveEntryCache.Lease>();
            for (var index = 0; index < 9; index++) {
                var lease = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1);
                var output = Path.Combine(lease.Root, "files", "selected.txt");
                File.WriteAllText(output, "sealed output"); lease.Seal(output);
                File.WriteAllText(Path.Combine(lease.Root, "unknown.txt"), "unknown obstruction");
                leases.Add(lease);
            }
            foreach (var lease in leases) {
                Check(!cache.Release(lease.Id), "unknown file must prevent full retirement");
                Check(!File.Exists(Path.Combine(lease.Root, "files", "selected.txt")), "sealed output should be removed");
            }
            File.Delete(Path.Combine(leases[8].Root, "unknown.txt"));
            for (var batch = 0; batch < 2; batch++) cache.Sweep();
            Check(!Directory.Exists(leases[8].Root), "ninth locally retired lease starved behind eight obstructed leases");
            Check(!File.Exists(leases[8].Root + ".retire.json"), "ninth retirement did not complete");
            foreach (var lease in leases.Take(8)) {
                Check(File.ReadAllText(Path.Combine(lease.Root, "unknown.txt")) == "unknown obstruction", "sweep deleted an unknown file");
                Check(File.Exists(lease.Root + ".retire.json"), "obstructed lease lost recovery authority");
            }

            // Removal of the ninth entry and admission of another must not invalidate progress.
            var added = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1);
            var addedOutput = Path.Combine(added.Root, "files", "selected.txt");
            File.WriteAllText(addedOutput, "sealed output"); added.Seal(addedOutput);
            File.WriteAllText(Path.Combine(added.Root, "unknown.txt"), "unknown obstruction");
            Check(!cache.Release(added.Id), "new obstruction must remain incomplete");
            var remaining = leases.Take(8).Append(added).ToArray();
            foreach (var lease in remaining) File.Delete(Path.Combine(lease.Root, "unknown.txt"));
            cache.Sweep();
            Check(remaining.Any(lease => Directory.Exists(lease.Root)), "one batch exceeded the eight-local-lease budget");
            for (var batch = 0; batch < 2; batch++) cache.Sweep();
            foreach (var lease in remaining) {
                Check(!Directory.Exists(lease.Root), "rotation stalled after collection removal/admission");
                Check(!File.Exists(lease.Root + ".retire.json"), "completed retirement left its recovery record");
            }
        } finally { Directory.Delete(root, true); }
    }
}
