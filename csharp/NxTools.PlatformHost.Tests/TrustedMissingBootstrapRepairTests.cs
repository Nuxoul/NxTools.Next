using System.Security.Cryptography;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Signed GUID-owned packages and an injected SCM/registry/gate; no installed service or product path.
internal static class TrustedMissingBootstrapRepairTests
{
    internal static Task MissingRootEnrollsExactlyOnce()
    {
        using var fixture = new Fixture();
        Require(fixture.Run() == 0, "missing root accepted");
        Require(fixture.Events.SequenceEqual(["parent", "stop", "gate", "parent", "enroll", "validate", "release", "deploy"]), "deployment after original gate release");
        var repairEvents = fixture.Log.Events.Where(value => value.Name.StartsWith("trusted_update.missing_bootstrap.", StringComparison.Ordinal)).ToArray();
        Require(repairEvents.Select(value => value.Name).SequenceEqual([
            "trusted_update.missing_bootstrap.accepted", "trusted_update.missing_bootstrap.service_stop_started",
            "trusted_update.missing_bootstrap.enrollment_started", "trusted_update.missing_bootstrap.deployment_started", "trusted_update.missing_bootstrap.completed"]), "accepted/stages/terminal diagnostics");
        Require(repairEvents.All(value => value.Id == fixture.Id && !value.Fields.ContainsKey("path") &&
            value.Fields.ContainsKey("elapsed_ms") && value.Fields.ContainsKey("native_error_code")), "private correlated diagnostic fields");
        Require(Directory.Exists(fixture.Root), "enrollment created root");
        var versionPath = Path.Combine(fixture.Root, "bootstrap-maintenance-state.json");
        Require(File.Exists(versionPath), "repair records compatible protocol before success");
        using (var version = System.Text.Json.JsonDocument.Parse(File.ReadAllBytes(versionPath)))
        {
            Require(version.RootElement.GetProperty("ProtocolVersion").GetInt32() == 1 &&
                version.RootElement.GetProperty("Sequence").GetInt64() == 22 &&
                version.RootElement.GetProperty("BootstrapSha256").GetString() == fixture.Registration.BootstrapHash,
                "repair protocol marker matches completed signed enrollment");
        }
        Require(fixture.Run() == 1, "existing root rejects retry");
        Require(fixture.Events.Count(value => value == "enroll") == 1, "one enrollment");
        return Task.CompletedTask;
    }
    internal static Task MissingProductParentsUseActualDirectoryValidation()
    {
        using var fixture = new Fixture();
        Require(!Directory.Exists(Path.Combine(fixture.CommonDataRoot, "NxTools")), "no vendor or product directory in fixture");
        Require(fixture.Run() == 0, "actual native parent validation accepts genuinely missing vendor/product parents");
        Require(fixture.Events.Contains("deploy"), "missing-parent repair reaches actual original deployment transaction");
        using (var blocked = new Fixture())
        {
            var vendor = Path.Combine(blocked.CommonDataRoot, "NxTools");
            File.WriteAllText(vendor, "existing parent file");
            Require(blocked.Run() == 1 && File.ReadAllText(vendor) == "existing parent file" &&
                !blocked.Events.Contains("stop"), "existing non-directory ancestor remains untouched");
        }
        using (var blocked = new Fixture())
        {
            Directory.Delete(blocked.CommonDataRoot);
            Require(blocked.Run() == 1 && !Directory.Exists(blocked.CommonDataRoot) &&
                !blocked.Events.Contains("stop"), "base data root absence is not product creation authority");
        }
        return Task.CompletedTask;
    }


    internal static Task ExistingRootAndBadRegistrationNeverStop()
    {
        using (var fixture = new Fixture())
        {
            Directory.CreateDirectory(fixture.Root);
            var marker = Path.Combine(fixture.Root, "enrollment.json");
            File.WriteAllText(marker, "owned journal");
            Require(fixture.Run() == 1 && File.ReadAllText(marker) == "owned journal" && !fixture.Events.Contains("stop"), "existing journal preserved");
        }
        using (var fixture = new Fixture())
        {
            Directory.CreateDirectory(Path.GetDirectoryName(fixture.Root)!);
            File.WriteAllText(fixture.Root, "existing file at root");
            Require(fixture.Run() == 1 && File.ReadAllText(fixture.Root) == "existing file at root" && !fixture.Events.Contains("stop"), "existing non-directory root preserved");
        }
        foreach (var bad in new[] { "path", "key", "account", "hash" })
        {
            using var fixture = new Fixture();
            fixture.Registration = bad switch
            {
                "path" => fixture.Registration with { ImagePath = "\"foreign.exe\" --trusted-update-service" },
                "key" => fixture.Registration with { KeyHash = new string('0', 64) },
                "account" => fixture.Registration with { Account = "LocalService" },
                _ => fixture.Registration with { BootstrapHash = "not-a-hash" }
            };
            Require(fixture.Run() == 1 && fixture.Events.Count == 0 && !Directory.Exists(fixture.Root), bad + " rejected without effects");
        }
        return Task.CompletedTask;
    }

    internal static Task FailureMatrixPreservesOriginalBoundary()
    {
        foreach (var fault in new[] { "admin", "signature", "caller", "access", "stop", "changed", "appeared", "validate", "deploy" })
        {
            using var fixture = new Fixture();
            fixture.Fault = fault;
            var expected = fault is "admin" or "signature" or "caller" or "access" ? 0 : 1;
            Require(fixture.Run() == 1 && fixture.Events.Count(value => value == "stop") == expected, fault + " fails at correct stage");
            Require(fixture.Events.Count(value => value == "enroll") == (fault is "validate" or "deploy" ? 1 : 0), fault + " never claims premature enrollment");
            if (fault == "appeared") Require(File.ReadAllText(Path.Combine(fixture.Root, "enrollment.json")) == "other owner", "new root preserved");
            else if (fault is not ("validate" or "deploy")) Require(!Directory.Exists(fixture.Root), fault + " has no root mutation");
        }
        return Task.CompletedTask;
    }

    private sealed class Fixture : IDisposable
    {
        private readonly string directory = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath(),
            "missing-bootstrap-" + Guid.NewGuid().ToString("N"));
        internal readonly byte[] key;
        private readonly string caller;
        internal string CommonDataRoot => Path.Combine(directory, "ProgramData");
        internal string Root => Path.Combine(CommonDataRoot, "NxTools", "NxTools.Next", "Updater");
        private string Package => Path.Combine(directory, "package");
        private string Bootstrap => Path.Combine(Root, "Bootstrap", "NxTools.PlatformHost.exe");
        internal string ServiceRoot => Path.Combine(directory, "Service");
        internal TrustedMissingBootstrapRepair.Registration Registration { get; set; }
        internal Guid Id { get; } = Guid.NewGuid();
        internal RecordingLog Log { get; } = new();
        internal List<string> Events { get; } = [];
        internal string? Fault { get; set; }
        internal Fixture()
        {
            Directory.CreateDirectory(Path.Combine(Package, "browser_extension", "nxtools-interceptor"));
            Directory.CreateDirectory(CommonDataRoot);
            Directory.CreateDirectory(ServiceRoot);
            File.WriteAllText(Path.Combine(ServiceRoot, "NxTools.exe"), "older protected desktop");
            using var rsa = RSA.Create(2048);
            key = rsa.ExportSubjectPublicKeyInfo();
            var pem = Path.Combine(directory, "key.pem");
            File.WriteAllText(pem, rsa.ExportPkcs8PrivateKeyPem());
            foreach (var name in new[] { "NxTools.exe", "NxTools.PlatformHost.exe", "NxTools.Indexer.exe", "NxTools.BrowserHost.exe", "NxTools.Cli.exe",
                "LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY_NOTICES.md", "browser_extension/nxtools-interceptor/manifest.json" }
                .Concat(TrustedReleaseManifestTests.ArchiveMembers))
                File.WriteAllText(Path.Combine(Package, name), "signed fixture " + name);
            TrustedReleaseManifest.SignPackage(Package, pem, 22, Guid.NewGuid());
            caller = Path.Combine(Package, "NxTools.PlatformHost.exe");
            Registration = new($"\"{Bootstrap}\" --trusted-update-service", "LocalSystem",
                Convert.ToHexString(SHA256.HashData(key)), new string('A', 64));
        }
        internal int Run()
        {
            if (Fault == "signature") File.AppendAllText(Path.Combine(Package, "NxTools.exe"), "tampered");
            var operations = new TrustedMissingBootstrapRepair.Operations(
                () => Fault != "admin",
                () => Registration,
                () =>
                {
                    Events.Add("parent");
                    if (Fault == "access") throw new UnauthorizedAccessException();
                    IndexerServiceInstaller.ValidateMissingProductParents(CommonDataRoot);
                },
                () => { Events.Add("stop"); if (Fault == "stop") throw new IOException("SCM stop failed"); },
                () =>
                {
                    Events.Add("gate");
                    if (Fault == "changed") Registration = Registration with { KeyHash = new string('0', 64) };
                    if (Fault == "appeared")
                    {
                        Directory.CreateDirectory(Root);
                        File.WriteAllText(Path.Combine(Root, "enrollment.json"), "other owner");
                    }
                    return new Gate(Events);
                },
                (_, _, _) =>
                {
                    Events.Add("enroll");
                    using var release = TrustedReleaseManifest.VerifyPackage(Package, key, 0);
                    var bootstrapHash = release.Files.Single(file => file.Path == "NxTools.PlatformHost.exe").Sha256;
                    var identity = new TrustedUpdateEnrollment.Identity(Convert.ToHexString(SHA256.HashData(key)),
                        bootstrapHash, release.Manifest.ReleaseSequence, release.Manifest.ReleaseId);
                    new TrustedUpdateEnrollment(Root).Run(identity,
                        () => Directory.CreateDirectory(Path.GetDirectoryName(Bootstrap)!),
                        () =>
                        {
                            File.Copy(caller, Bootstrap, false);
                            new TrustedUpdateTransaction(Path.Combine(Root, "Releases"), key, new Deployment(this), Log)
                                .Initialize(Package, requireInitialDeployment: true);
                        },
                        () => Registration = Registration with { BootstrapHash = bootstrapHash },
                        () => { using var file = IndexerServiceInstaller.TrustedBinary.Open(Bootstrap, bootstrapHash); });
                },
                () =>
                {
                    Events.Add("validate");
                    Require(new TrustedUpdateEnrollment(Root).IsComplete(), "real enrollment completed");
                    Require(File.ReadAllBytes(Bootstrap).SequenceEqual(File.ReadAllBytes(caller)), "enrolled actual signed caller bytes");
                    if (Fault == "validate") throw new IOException("native enrollment validation failed");
                },
                (_, _, _) =>
                {
                    Events.Add("deploy");
                    Require(Events.IndexOf("release") < Events.IndexOf("deploy"), "initial deployment cannot retain parent gate");
                    var transaction = new TrustedUpdateTransaction(Path.Combine(Root, "Releases"), key, new Deployment(this), Log);
                    Require(transaction.ReadState().DeploymentPending, "enrollment has not published initial deployment success");
                    transaction.ApplyAsync(Package, Id, CancellationToken.None).GetAwaiter().GetResult();
                    Require(!transaction.ReadState().DeploymentPending, "native completion clears initial deployment pending");
                    Require(File.ReadAllBytes(Path.Combine(ServiceRoot, "NxTools.exe")).SequenceEqual(
                        File.ReadAllBytes(Path.Combine(Package, "NxTools.exe"))), "older protected desktop replaced before repair success");
                });
            return TrustedMissingBootstrapRepair.Run(Log, Id, Package,
                Fault == "caller" ? Path.Combine(Package, "NxTools.exe") : caller, Root, Bootstrap, key, operations);
        }
        public void Dispose() => Directory.Delete(directory, true);
    }
    private sealed class Deployment(Fixture fixture) : TrustedUpdateTransaction.IDeployment
    {
        public Task DeployAsync(string package, Guid correlationId, CancellationToken cancellationToken)
        {
            if (fixture.Fault == "deploy") throw new IOException("fixture deployment failed");
            using var release = TrustedReleaseManifest.VerifyPackage(package, fixture.key, 0);
            foreach (var file in release.Files)
            {
                var destination = Path.Combine(fixture.ServiceRoot, file.Path);
                Directory.CreateDirectory(Path.GetDirectoryName(destination)!);
                File.Copy(Path.Combine(package, file.Path), destination, true);
                Require(File.ReadAllBytes(destination).SequenceEqual(File.ReadAllBytes(Path.Combine(package, file.Path))), "deployed protected fixture bytes");
            }
            return Task.CompletedTask;
        }
    }

    private sealed class Gate(List<string> events) : IDisposable
    {
        public void Dispose() => events.Add("release");
    }
    private sealed class RecordingLog : IPlatformLog
    {
        internal List<(string Name, Guid? Id, IReadOnlyDictionary<string, object?> Fields)> Events { get; } = [];
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) =>
            Events.Add((eventName, correlationId, fields ?? new Dictionary<string, object?>()));
    }
    private static void Require(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
