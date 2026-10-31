using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Real temporary signed packages/files; fake service only. Never SCM/Registry/product/installer.
internal static class TrustedBootstrapMaintenanceTests
{
    internal static Task SameTrustMaintenancePreservesState()
    {
        using var fixture = new Fixture();
        var before = File.ReadAllBytes(fixture.StatePath);
        fixture.Maintenance().Apply(fixture.Package, Guid.NewGuid());
        Require(File.ReadAllBytes(fixture.Bootstrap).SequenceEqual(fixture.NewBytes), "signed bootstrap replaced");
        Require(File.ReadAllBytes(fixture.StatePath).SequenceEqual(before), "release state never reset or advanced by maintenance");
        Require(fixture.Service.Events.SequenceEqual(new[] { "validate", "stop", "gate", "set", "start", "ready" }), "validate before mutation and ready before commit");
        Require(string.Equals(fixture.Service.Hash, Hash(fixture.NewBytes), StringComparison.OrdinalIgnoreCase) && fixture.Service.Running, "new service identity running");
        Require(!File.Exists(Path.Combine(fixture.Root, "bootstrap-maintenance.json")), "completed journal retired");
        fixture.Maintenance().Apply(fixture.Package, Guid.NewGuid());
        Require(fixture.Service.Events.Count(value => value == "stop") == 1, "same maintenance idempotent");
        return Task.CompletedTask;
    }

    internal static Task StartFailureRestoresOriginalBootstrap()
    {
        using var fixture = new Fixture();
        var before = File.ReadAllBytes(fixture.StatePath);
        var enrollment = File.ReadAllBytes(Path.Combine(fixture.Root, "enrollment.json"));
        fixture.Service.FailReady = true;
        try { fixture.Maintenance().Apply(fixture.Package, Guid.NewGuid()); throw new InvalidOperationException("Expected readiness failure"); }
        catch (IOException) { }
        Require(File.ReadAllBytes(fixture.Bootstrap).SequenceEqual(fixture.OldBytes), "failed maintenance restores original bytes");
        Require(File.ReadAllBytes(fixture.StatePath).SequenceEqual(before), "failed maintenance preserves sequence");
        Require(File.ReadAllBytes(Path.Combine(fixture.Root, "enrollment.json")).SequenceEqual(enrollment), "exact enrollment restored");
        Require(fixture.Service.Running && fixture.Service.Hash == Hash(fixture.OldBytes), "old service restored");
        Require(!File.Exists(Path.Combine(fixture.Root, "bootstrap-maintenance.json")), "successful rollback retires journal");
        fixture.Service.FailReady = false;
        fixture.Maintenance().Apply(fixture.Package, Guid.NewGuid());
        Require(File.ReadAllBytes(fixture.Bootstrap).SequenceEqual(fixture.NewBytes), "maintenance retry succeeds");
        return Task.CompletedTask;
    }

    internal static Task RejectsWrongTrustDowngradeAndTamperBeforeStop()
    {
        foreach (var fault in new[] { "key", "downgrade", "bootstrap", "candidate" })
        {
            using var fixture = new Fixture();
            if (fault == "downgrade")
            {
                var state = JsonSerializer.Deserialize<TrustedUpdateTransaction.State>(File.ReadAllBytes(fixture.StatePath))!;
                File.WriteAllText(fixture.StatePath, JsonSerializer.Serialize(state with { Sequence = 22 }));
            }
            if (fault == "bootstrap") File.AppendAllText(fixture.Bootstrap, "tamper");
            if (fault == "candidate") File.AppendAllText(Path.Combine(fixture.Package, "NxTools.PlatformHost.exe"), "tamper");
            using var other = RSA.Create(2048);
            var maintenance = fault == "key" ? new TrustedBootstrapMaintenance(fixture.Root, other.ExportSubjectPublicKeyInfo(), fixture.Service, NullPlatformLog.Instance) : fixture.Maintenance();
            var rejected = false;
            try { maintenance.Apply(fixture.Package, Guid.NewGuid()); }
            catch (Exception failure) when (failure is InvalidDataException or CryptographicException) { rejected = true; }
            Require(rejected && !fixture.Service.Events.Contains("stop"), fault + " rejects before service mutation");
        }
        return Task.CompletedTask;
    }

    internal static Task InterruptedRollbackRecoversOnNextMaintenance()
    {
        using var fixture = new Fixture();
        fixture.Service.FailReady = true;
        fixture.Service.FailStopAt = 2;
        try { fixture.Maintenance().Apply(fixture.Package, Guid.NewGuid()); throw new InvalidOperationException("Expected interrupted rollback"); }
        catch (IOException) { }
        Require(File.Exists(Path.Combine(fixture.Root, "bootstrap-maintenance.json")), "incomplete rollback retains journal");
        fixture.Service.FailReady = false;
        fixture.Service.FailStopAt = 0;
        fixture.Maintenance().Apply(fixture.Package, Guid.NewGuid());
        Require(File.ReadAllBytes(fixture.Bootstrap).SequenceEqual(fixture.NewBytes) && fixture.Service.Running, "next maintenance recovers then applies");
        Require(!File.Exists(Path.Combine(fixture.Root, "bootstrap-maintenance.json")), "retry retires journal");
        return Task.CompletedTask;
    }

    internal sealed class Fixture : IDisposable
    {
        internal string Root { get; } = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "bootstrap-maintenance-" + Guid.NewGuid().ToString("N"));
        internal string Package => Path.Combine(Root, "candidate");
        internal string Bootstrap => Path.Combine(Root, "Bootstrap", "NxTools.PlatformHost.exe");
        internal string StatePath => Path.Combine(Root, "Releases", "state.json");
        internal byte[] NewBytes { get; } = "new signed bootstrap"u8.ToArray();
        internal byte[] OldBytes { get; } = "old approved bootstrap"u8.ToArray();
        internal byte[] PublicKey { get; }
        internal FakeService Service { get; }
        internal Fixture()
        {
            Directory.CreateDirectory(Path.GetDirectoryName(Bootstrap)!);
            Directory.CreateDirectory(Path.GetDirectoryName(StatePath)!);
            Directory.CreateDirectory(Path.Combine(Package, "browser_extension", "nxtools-interceptor"));
            using var rsa = RSA.Create(2048); PublicKey = rsa.ExportSubjectPublicKeyInfo();
            var key = Path.Combine(Root, "temporary.pem"); File.WriteAllText(key, rsa.ExportPkcs8PrivateKeyPem());
            foreach (var name in new[] { "NxTools.exe", "NxTools.PlatformHost.exe", "NxTools.Indexer.exe", "NxTools.BrowserHost.exe", "NxTools.Cli.exe", "LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY_NOTICES.md", "browser_extension/nxtools-interceptor/manifest.json" }.Concat(TrustedReleaseManifestTests.ArchiveMembers))
                File.WriteAllText(Path.Combine(Package, name), "candidate " + name);
            File.WriteAllBytes(Path.Combine(Package, "NxTools.PlatformHost.exe"), NewBytes);
            var active = Path.Combine(Root, "Releases", "active"); Directory.CreateDirectory(active);
            foreach (var source in Directory.GetFiles(Package, "*", SearchOption.AllDirectories))
            {
                var relative = Path.GetRelativePath(Package, source);
                if (relative is "NxTools.release.json" or "NxTools.release.sig") continue;
                var destination = Path.Combine(active, relative); Directory.CreateDirectory(Path.GetDirectoryName(destination)!); File.Copy(source, destination);
            }
            TrustedReleaseManifest.SignPackage(active, key, 21, Guid.NewGuid());
            using (var verified = TrustedReleaseManifest.VerifyPackage(active, PublicKey, 0))
                File.WriteAllText(StatePath, JsonSerializer.Serialize(new TrustedUpdateTransaction.State(21, verified.Manifest.ReleaseId, verified.ManifestSha256)));
            TrustedReleaseManifest.SignPackage(Package, key, 22, Guid.NewGuid());
            File.WriteAllBytes(Bootstrap, OldBytes);
            File.WriteAllText(Path.Combine(Root, "enrollment.json"), JsonSerializer.Serialize(new { Identity = new TrustedUpdateEnrollment.Identity(Hash(PublicKey), Hash(OldBytes), 1, Guid.NewGuid()), Completed = true }));
            Service = new FakeService(Hash(OldBytes), Hash(PublicKey)) { MaintenanceLockPath = Path.Combine(Root, "bootstrap-maintenance.lock") };
        }
        internal TrustedBootstrapMaintenance Maintenance() => new(Root, PublicKey, Service, NullPlatformLog.Instance);
        public void Dispose() => Directory.Delete(Root, true);
    }
    internal sealed class FakeService(string hash, string keyHash) : TrustedBootstrapMaintenance.IService
    {
        internal string Hash = hash;
        internal bool Running = true;
        internal bool FailReady;
        internal string? MaintenanceLockPath;
        internal int FailStopAt;
        private int stops;
        internal List<string> Events = [];
        public void Validate(string expectedBootstrapHash, string expectedPublicKeyHash) { Events.Add("validate"); Require(string.Equals(Hash, expectedBootstrapHash, StringComparison.OrdinalIgnoreCase) && string.Equals(keyHash, expectedPublicKeyHash, StringComparison.OrdinalIgnoreCase), "registered original identity"); }
        public void Stop() { Events.Add("stop"); if (++stops == FailStopAt) throw new IOException("fixture stop interrupted"); Running = false; }
        public IDisposable EnterDeploymentGate() { Events.Add("gate"); return new EmptyLease(); }
        private sealed class EmptyLease : IDisposable { public void Dispose() { } }
        public void SetBootstrapHash(string value) { Events.Add("set"); Hash = value; }
        public void Start() { Events.Add("start"); Running = true; }
        public void ConfirmReady()
        {
            Events.Add("ready");
            if (MaintenanceLockPath is not null)
            {
                var blocked = false;
                try { using var admission = new FileStream(MaintenanceLockPath, FileMode.Open, FileAccess.ReadWrite, FileShare.None); }
                catch (IOException) { blocked = true; }
                Require(blocked, "normal update admission remains blocked during maintenance readiness");
            }
            if (FailReady) throw new IOException("fixture readiness failed");
        }
    }
    internal static string Hash(byte[] bytes) => Convert.ToHexString(SHA256.HashData(bytes));
    internal static void Require(bool value, string message) { if (!value) throw new InvalidOperationException(message); }
}
