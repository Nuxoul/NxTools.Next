using System.Reflection;
using System.Runtime.Loader;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Windows;
using NxTools.ShellSelectionHelper;

namespace NxTools.PlatformHost.Tests;

// File-only release seams. No process launcher, Registry/COM/server/service/installer/product execution.
internal static class ArchiveReleaseTests
{
    internal static Task StableApprovalRejectsPartialMixedSet()
    {
        using (var runner = NxTools.PlatformHost.Windows.Archive.ArchiveWorkerRunner.ForPackage(_ => { })) {
            var helper = typeof(NxTools.PlatformHost.Windows.Archive.ArchiveWorkerRunner)
                .GetField("readHelper", BindingFlags.NonPublic | BindingFlags.Instance)!.GetValue(runner);
            var fixtureApphost = string.Equals(Path.GetFileName(Environment.ProcessPath), "NxTools.PlatformHost.Tests.exe", StringComparison.OrdinalIgnoreCase);
            Check(fixtureApphost ? helper is null : helper is not null,
                "production package runner retains approved helper; original test apphost stays fixture-only");
        }
        var root = Path.Combine(Path.GetTempPath(), "archive-release-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        try {
            var desktop = Path.Combine(root, "NxTools.exe");
            var desktopHash = Convert.ToHexString(SHA256.HashData(new byte[] { 1, 2, 3 }));
            Check(!new DesktopIdentity(root, desktopHash).Verify().Approved, "missing Desktop refuses");
            File.WriteAllBytes(desktop, [3, 2, 1]);
            Check(!new DesktopIdentity(root, desktopHash).Verify().Approved, "old Desktop bytes refuse");
            File.WriteAllBytes(desktop, [1, 2, 3]);
            Check(new DesktopIdentity(root, desktopHash).Verify().Approved, "exact same-directory Desktop passes");
            var hashes = new Dictionary<string, string>();
            foreach (var member in TrustedReleaseManifestTests.ArchiveMembers) {
                File.WriteAllText(Path.Combine(root, member), "current " + member);
                hashes[member] = Hash(Path.Combine(root, member));
            }
            using (IndexerServiceInstaller.OpenArchiveRelease(root, hashes)) {
                foreach (var member in hashes.Keys) Reject(() => File.Delete(Path.Combine(root, member)));
            }
            foreach (var member in hashes.Keys) {
                var path = Path.Combine(root, member);
                var bytes = File.ReadAllBytes(path);
                File.Delete(path);
                Reject(() => { using var missing = IndexerServiceInstaller.OpenArchiveRelease(root, hashes); });
                File.WriteAllText(path, "older member");
                Reject(() => { using var mixed = IndexerServiceInstaller.OpenArchiveRelease(root, hashes); });
                File.WriteAllBytes(path, bytes);
            }
            var absent = new Dictionary<string, string>(hashes); absent.Remove(hashes.Keys.First());
            Reject(() => { using var incomplete = IndexerServiceInstaller.OpenArchiveRelease(root, absent); });
            var empty = hashes.ToDictionary(pair => pair.Key, _ => "");
            Reject(() => { using var unpackaged = IndexerServiceInstaller.OpenArchiveRelease(root, empty); });
            var nullHash = new Dictionary<string, string>(hashes) { [hashes.Keys.First()] = null! };
            Reject(() => { using var invalid = IndexerServiceInstaller.OpenArchiveRelease(root, nullHash); });
            using var final = IndexerServiceInstaller.OpenArchiveRelease(root, hashes);
            return Task.CompletedTask;
        } finally { Directory.Delete(root, true); }
    }

    internal static Task StagedActualIdentities()
    {
        // Explicit fixture-only input, never consumed by any production resolver or approval owner.
        var root = Environment.GetEnvironmentVariable("NXTOOLS_TEST_ARCHIVE_RELEASE_STAGE");
        if (root is null) throw new TestSkippedException("Explicit isolated staging input is required.");
        var full = Path.GetFullPath(root);
        var scratch = Path.GetFullPath(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!).TrimEnd('\\') + "\\";
        Check(full.StartsWith(scratch, StringComparison.OrdinalIgnoreCase) &&
            File.ReadAllText(full + ".STAGING-NOT-INSTALLABLE.txt").Contains("TEST-KEY"), "only explicit test-key scratch stage");
        using var inputs = JsonDocument.Parse(File.ReadAllText(full + ".build-inputs.json"));
        var build = inputs.RootElement.GetProperty("build_directory").GetString()!;
        var context = new AssemblyLoadContext("archive-stage-" + Guid.NewGuid(), isCollectible: true);
        try {
            // Load managed metadata only; Program.Main and COM activation are never invoked.
            var helper = context.LoadFromAssemblyPath(Path.Combine(full, "NxTools.ShellSelectionHelper.dll"));
            var helperProgram = helper.GetType("NxTools.ShellSelectionHelper.Program", throwOnError: true)!;
            var expected = (string)helperProgram.GetField("ExpectedDesktopSha256", BindingFlags.NonPublic | BindingFlags.Static)!.GetRawConstantValue()!;
            Check(expected == Hash(Path.Combine(full, "NxTools.exe")), "helper embeds the actual current Desktop SHA");
            Check(new DesktopIdentity(full, expected).Verify().Approved, "actual sibling Desktop passes original helper identity boundary");
            Check(!new DesktopIdentity(full, "").Verify().Approved &&
                !new DesktopIdentity(full, new string('0', 64)).Verify().Approved, "empty and wrong Desktop hashes refuse");
            var hostDll = Path.Combine(build, "host-build", "bin", "NxTools.PlatformHost", "release_win-x64", "NxTools.PlatformHost.dll");
            var platform = context.LoadFromAssemblyPath(hostDll);
            var runnerType = platform.GetType("NxTools.PlatformHost.Windows.Archive.ArchiveWorkerRunner", true)!;
            using (var runner = (IDisposable)runnerType.GetMethod("ForPackage", BindingFlags.NonPublic | BindingFlags.Static)!
                .Invoke(null, [null, null])!) {
                Check(runnerType.GetField("readHelper", BindingFlags.NonPublic | BindingFlags.Instance)!.GetValue(runner) is not null,
                    "actual staged PlatformHost package runner retains approved helper owner");
            }
            var approval = platform.GetType("NxTools.PlatformHost.Generated.ArchiveReleaseManifest", true)!;
            var hashes = (IReadOnlyDictionary<string, string>)approval.GetField("Files", BindingFlags.NonPublic | BindingFlags.Static)!.GetValue(null)!;
            using (IndexerServiceInstaller.OpenArchiveRelease(full, hashes)) { }
            Check(hashes["NxTools.ArchiveWorker.exe"] == Hash(Path.Combine(full, "NxTools.ArchiveWorker.exe")), "fixed production worker output is approved");
            var trust = platform.GetType("NxTools.PlatformHost.Generated.UpdateTrust", true)!;
            var publicKey = Convert.FromBase64String((string)trust.GetField("PublicKeySpkiBase64", BindingFlags.NonPublic | BindingFlags.Static)!.GetRawConstantValue()!);
            using var verified = TrustedReleaseManifest.VerifyPackage(full, publicKey, 0);
            foreach (var name in hashes.Keys) Check(verified.Files.Any(file => file.Path == name && file.Sha256.Equals(hashes[name], StringComparison.OrdinalIgnoreCase)), "signed actual archive member: " + name);
            using var runtime = JsonDocument.Parse(File.ReadAllText(Path.Combine(full, "NxTools.ShellSelectionHelper.runtimeconfig.json")));
            Check(runtime.RootElement.GetProperty("runtimeOptions").GetProperty("framework").GetProperty("name").GetString() == "Microsoft.NETCore.App", "framework-dependent runtime contract retained");
            Console.WriteLine("STAGING_IDENTITY actual_desktop_binding=pass worker_approval=pass helper_runtime_members=4 signed_complete=pass production_trust=not_used");
            return Task.CompletedTask;
        } finally { context.Unload(); }
    }

    private static string Hash(string path) { using var stream = File.OpenRead(path); return Convert.ToHexString(SHA256.HashData(stream)); }
    private static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
    private static void Reject(Action action)
    {
        try { action(); }
        catch (Exception failure) when (failure is IOException or InvalidDataException or UnauthorizedAccessException or System.ComponentModel.Win32Exception) { return; }
        throw new InvalidOperationException("Expected release identity/lease rejection.");
    }
}
