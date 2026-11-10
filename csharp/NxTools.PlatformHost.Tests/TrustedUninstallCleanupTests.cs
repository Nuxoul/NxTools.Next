using System.Security.Cryptography;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Signed temporary packages, owned fixture trees and fake SCM/process effects only.
internal static class TrustedUninstallCleanupTests
{
    internal static Task OwnedServicesAndProtectedDeploymentsAreRemoved()
    {
        using var fixture = new Fixture();
        Require(fixture.Run() == 0, "owned protected uninstall succeeds");
        Require(fixture.Events.SequenceEqual([
            "gate", "stop:NxToolsIndexerV1", "stop:NxToolsUpdateV1", "processes",
            "delete:NxToolsIndexerV1", "delete:NxToolsUpdateV1", "tree:Service", "tree:Updater", "release"
        ]), "validation precedes exact native effects and tree deletion");
        Require(!Directory.Exists(fixture.ServiceRoot) && !Directory.Exists(fixture.UpdaterRoot),
            "owned protected roots removed");
        var lifecycle = fixture.Log.Events.Where(value =>
            value.Name.StartsWith("installer.protected_uninstall.", StringComparison.Ordinal)).ToArray();
        Require(lifecycle.Select(value => value.Name).SequenceEqual([
            "installer.protected_uninstall.accepted", "installer.protected_uninstall.completed"
        ]), "bounded accepted and terminal diagnostics");
        Require(lifecycle.All(value => value.Id == fixture.Id && value.Fields.ContainsKey("elapsed_ms") &&
            value.Fields.ContainsKey("native_error_code") && !value.Fields.ContainsKey("path")),
            "diagnostics carry correlation without protected paths");
        using var trailing = new Fixture { Fault = "trailing" };
        Require(trailing.Run() == 0, "directory separators do not reject the same signed caller");
        return Task.CompletedTask;
    }

    internal static Task ForeignOrRecoveryStateNeverMutates()
    {
        foreach (var fault in new[] { "foreign_indexer", "foreign_updater", "foreign_file", "transaction", "changed" })
        {
            using var fixture = new Fixture { Fault = fault };
            Require(fixture.Run() == 1, fault + " refused");
            Require(!fixture.Events.Any(value => value.StartsWith("stop:", StringComparison.Ordinal) ||
                value.StartsWith("delete:", StringComparison.Ordinal) || value.StartsWith("tree:", StringComparison.Ordinal)),
                fault + " has no destructive effect");
            Require(Directory.Exists(fixture.ServiceRoot) && Directory.Exists(fixture.UpdaterRoot),
                fault + " preserves protected evidence");
        }
        using (var remainder = new Fixture())
        {
            remainder.Indexer = null;
            remainder.Updater = null;
            File.WriteAllText(Path.Combine(remainder.ServiceRoot, "foreign.bin"), "foreign");
            Require(remainder.Run() == 1 && remainder.Events.Count == 0,
                "registration-free remainder with foreign file is retained");
        }
        using (var recovery = new Fixture())
        {
            recovery.Indexer = null;
            recovery.Updater = null;
            File.WriteAllText(Path.Combine(recovery.UpdaterRoot, "Releases", "transaction.json"), "recovery");
            Require(recovery.Run() == 1 && recovery.Events.Count == 0,
                "registration-free remainder with recovery record is retained");
        }
        return Task.CompletedTask;
    }

    internal static Task NativeFailureIsNotSuccessfulCleanup()
    {
        foreach (var fault in new[] { "stop", "delete", "tree" })
        {
            using var fixture = new Fixture { Fault = fault };
            Require(fixture.Run() == 1, fault + " reports failure");
            if (fault == "stop")
                Require(!fixture.Events.Any(value => value.StartsWith("delete:", StringComparison.Ordinal)),
                    "stop failure prevents registration deletion");
            if (fault == "delete")
                Require(Directory.Exists(fixture.ServiceRoot) && Directory.Exists(fixture.UpdaterRoot),
                    "registration deletion failure preserves deployment roots");
        }
        using (var retry = new Fixture { Fault = "tree" })
        {
            Require(retry.Run() == 1 && retry.Indexer is null && retry.Updater is null,
                "first tree failure occurs after both exact services are deleted");
            retry.Fault = null;
            Require(retry.Run() == 0 && !Directory.Exists(retry.ServiceRoot) && !Directory.Exists(retry.UpdaterRoot),
                "second real Run removes signed exact-owned deployment remainders");
        }
        return Task.CompletedTask;
    }

    internal static Task AbsentComponentsAreANoOp()
    {
        using (var fixture = new Fixture())
        {
            Directory.Delete(fixture.ServiceRoot, true);
            Directory.Delete(fixture.UpdaterRoot, true);
            fixture.Indexer = null;
            fixture.Updater = null;
            Require(fixture.Run() == 0 && fixture.Events.Count == 0,
                "absent registrations and roots are cleanly absent");
        }
        using (var missingUpdater = new Fixture())
        {
            Directory.Delete(missingUpdater.ServiceRoot, true);
            Directory.Delete(missingUpdater.UpdaterRoot, true);
            missingUpdater.Indexer = null;
            Require(missingUpdater.Run() == 0 && missingUpdater.Updater is null &&
                missingUpdater.Events.SequenceEqual([
                    "gate", "stop:NxToolsUpdateV1", "delete:NxToolsUpdateV1", "release"
                ]), "exact updater registration with whole root absent is removed under the original gate");
        }
        using (var foreignUpdater = new Fixture())
        {
            Directory.Delete(foreignUpdater.ServiceRoot, true);
            Directory.Delete(foreignUpdater.UpdaterRoot, true);
            foreignUpdater.Indexer = null;
            foreignUpdater.Updater = foreignUpdater.Updater! with { Account = "LocalService" };
            Require(foreignUpdater.Run() == 1 && foreignUpdater.Updater is not null &&
                foreignUpdater.Events.Count == 0, "foreign missing-root updater tuple is retained without effects");
        }
        return Task.CompletedTask;
    }

    private sealed class Fixture : IDisposable
    {
        private readonly string directory = Path.Combine(
            Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath(),
            "trusted-uninstall-" + Guid.NewGuid().ToString("N"));
        private readonly byte[] publicKey;
        private readonly string package;
        private readonly string caller;
        private int indexerReads;
        private int updaterReads;

        internal Guid Id { get; } = Guid.NewGuid();
        internal string ProductRoot => Path.Combine(directory, "ProgramData", "NxTools", "NxTools.Next");
        internal string ServiceRoot => Path.Combine(ProductRoot, "Service");
        internal string UpdaterRoot => Path.Combine(ProductRoot, "Updater");
        internal TrustedUninstallCleanup.IndexerRegistration? Indexer { get; set; }
        internal TrustedUninstallCleanup.UpdaterRegistration? Updater { get; set; }
        internal string? Fault { get; set; }
        internal List<string> Events { get; } = [];
        internal RecordingLog Log { get; } = new();

        internal Fixture()
        {
            package = Path.Combine(directory, "package");
            Directory.CreateDirectory(Path.Combine(package, "browser_extension", "nxtools-interceptor"));
            using var rsa = RSA.Create(2048);
            publicKey = rsa.ExportSubjectPublicKeyInfo();
            var pem = Path.Combine(directory, "key.pem");
            Directory.CreateDirectory(directory);
            File.WriteAllText(pem, rsa.ExportPkcs8PrivateKeyPem());
            foreach (var name in new[] {
                "NxTools.exe", "NxTools.PlatformHost.exe", "NxTools.Indexer.exe", "NxTools.BrowserHost.exe",
                "NxTools.Cli.exe", "LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY_NOTICES.md",
                "browser_extension/nxtools-interceptor/manifest.json"
            }.Concat(TrustedReleaseManifestTests.ArchiveMembers))
            {
                var path = Path.Combine(package, name.Replace('/', Path.DirectorySeparatorChar));
                Directory.CreateDirectory(Path.GetDirectoryName(path)!);
                File.WriteAllText(path, "signed uninstall fixture " + name);
            }
            TrustedReleaseManifest.SignPackage(package, pem, 43, Guid.NewGuid());
            caller = Path.Combine(package, "NxTools.PlatformHost.exe");
            using var release = TrustedReleaseManifest.VerifyPackage(package, publicKey, 0);
            Directory.CreateDirectory(ServiceRoot);
            foreach (var file in release.Files)
            {
                var destination = Path.Combine(ServiceRoot, file.Path.Replace('/', Path.DirectorySeparatorChar));
                Directory.CreateDirectory(Path.GetDirectoryName(destination)!);
                File.Copy(Path.Combine(package, file.Path.Replace('/', Path.DirectorySeparatorChar)), destination);
            }
            Directory.CreateDirectory(Path.Combine(UpdaterRoot, "Bootstrap"));
            Directory.CreateDirectory(Path.Combine(UpdaterRoot, "Releases"));
            var bootstrap = Path.Combine(UpdaterRoot, "Bootstrap", "NxTools.PlatformHost.exe");
            File.Copy(caller, bootstrap);
            var keyHash = Convert.ToHexString(SHA256.HashData(publicKey));
            var bootstrapHash = release.Files.Single(file => file.Path == "NxTools.PlatformHost.exe").Sha256;
            var identity = new TrustedUpdateEnrollment.Identity(keyHash, bootstrapHash,
                release.Manifest.ReleaseSequence, release.Manifest.ReleaseId);
            new TrustedUpdateEnrollment(UpdaterRoot).Run(identity, () => { },
                () => new TrustedUpdateTransaction(Path.Combine(UpdaterRoot, "Releases"), publicKey,
                    new NoDeployment(), Log).Initialize(package), () => { }, () => { });
            var archive = release.Files.Where(file => TrustedReleaseManifestTests.ArchiveMembers.Contains(file.Path))
                .ToDictionary(file => file.Path, file => file.Sha256, StringComparer.Ordinal);
            Indexer = new(
                $"\"{Path.Combine(ServiceRoot, "NxTools.Indexer.exe")}\" --service", "LocalSystem", 3,
                "D:(A;;CCLCSWRPWPLORC;;;AU)", Path.Combine(ServiceRoot, "NxTools.exe"),
                release.Files.Single(file => file.Path == "NxTools.Indexer.exe").Sha256,
                release.Files.Single(file => file.Path == "NxTools.exe").Sha256,
                release.Files.Single(file => file.Path == "NxTools.PlatformHost.exe").Sha256,
                release.Files.Single(file => file.Path == "NxTools.BrowserHost.exe").Sha256,
                System.Text.Json.JsonSerializer.Serialize(archive));
            Updater = new($"\"{bootstrap}\" --trusted-update-service", "LocalSystem", 3,
                "D:(A;;CCLCSWRPLORC;;;AU)", keyHash, bootstrapHash);
        }

        internal int Run()
        {
            if (Fault == "foreign_indexer") Indexer = Indexer! with { ImagePath = "\"C:\\Foreign\\Indexer.exe\" --service" };
            if (Fault == "foreign_updater") Updater = Updater! with { Account = "LocalService" };
            if (Fault == "foreign_file") File.WriteAllText(Path.Combine(ServiceRoot, "foreign.bin"), "foreign");
            if (Fault == "transaction") File.WriteAllText(Path.Combine(UpdaterRoot, "Releases", "transaction.json"), "recovery");
            var operations = new TrustedUninstallCleanup.Operations(
                () => true,
                () =>
                {
                    indexerReads++;
                    if (Fault == "changed" && indexerReads > 1) return Indexer! with { DesktopHash = new string('0', 64) };
                    return Indexer;
                },
                () => { updaterReads++; return Updater; },
                () => { Events.Add("gate"); return new Gate(Events); },
                name =>
                {
                    Events.Add("stop:" + name);
                    if (Fault == "stop") throw new IOException("fixture stop failure");
                },
                _ => Events.Add("processes"),
                name =>
                {
                    Events.Add("delete:" + name);
                    if (Fault == "delete") throw new IOException("fixture delete failure");
                    if (name == IndexerServiceInstaller.ServiceName) Indexer = null;
                    if (name == TrustedUpdateBroker.ServiceName) Updater = null;
                },
                path =>
                {
                    var leaf = Path.GetFileName(path);
                    Events.Add("tree:" + leaf);
                    if (Fault == "tree" && leaf == "Updater") throw new IOException("fixture tree failure");
                    Directory.Delete(path, true);
                });
            var requestedPackage = Fault == "trailing" ? package + Path.DirectorySeparatorChar : package;
            return TrustedUninstallCleanup.Run(Log, Id, requestedPackage,
                caller, ProductRoot, publicKey, operations);
        }

        public void Dispose()
        {
            if (Directory.Exists(directory)) Directory.Delete(directory, true);
        }
    }

    private sealed class NoDeployment : TrustedUpdateTransaction.IDeployment
    {
        public Task DeployAsync(string verifiedPackage, Guid correlationId, CancellationToken cancellationToken) =>
            Task.CompletedTask;
    }

    private sealed class Gate(List<string> events) : IDisposable
    {
        public void Dispose() => events.Add("release");
    }

    private sealed class RecordingLog : IPlatformLog
    {
        internal List<(string Name, Guid? Id, IReadOnlyDictionary<string, object?> Fields)> Events { get; } = [];
        public void Write(string level, string eventName, Guid? correlationId = null,
            IReadOnlyDictionary<string, object?>? fields = null) =>
            Events.Add((eventName, correlationId, fields ?? new Dictionary<string, object?>()));
    }

    private static void Require(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
