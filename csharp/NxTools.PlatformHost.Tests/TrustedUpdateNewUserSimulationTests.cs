using System.ComponentModel;
using System.Security.Cryptography;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Continuous new-install scenario. Real signed files, EnsureCoreAsync, enrollment and transaction;
// only UAC consent, SCM registration/start and native protected deployment are fixture operations.
internal static class TrustedUpdateNewUserSimulationTests
{
    internal static async Task FreshInstallThenSignedUpdateRequestsConsentOnce()
    {
        using var user = new NewUser();
        var version1 = user.Publish(1);
        var version2 = user.Publish(2);

        user.InstallFiles(version1);
        Require(user.AuthorizationRequests == 0 && !user.ServiceExists && !user.Enrollment.IsComplete(),
            "new per-user install copies files without enrolling a service");
        Step("install-v1", user, "files-installed; no existing updater");

        var first = await user.OpenAsync();
        Require(first.Success && first.Data == "installed" && user.AuthorizationRequests == 1,
            "fresh first launch must request exactly one explicit enrollment authorization");
        Require(user.ServiceExists && user.Enrollment.IsComplete() && user.Transaction.ReadState().Sequence == 1,
            "accepted enrollment persists service fixture, trust and signed version");
        user.VerifyDeployed();
        Step("first-open-v1-consent-yes", user, first.Data!);

        var again = await user.OpenAsync();
        Require(again.Success && again.Data == "running" && user.AuthorizationRequests == 1,
            "second launch of same release must not prompt again");
        Step("second-open-v1", user, again.Data!);

        // New owner instances model a process restart. State must come from durable files, not flags.
        user.ReloadOwners();
        var reopened = await user.OpenAsync();
        Require(reopened.Success && user.AuthorizationRequests == 1 && user.Enrollment.IsComplete(),
            "process restart must retain enrollment without additional consent");
        Step("reopen-v1-new-owner", user, reopened.Data!);

        user.InstallFiles(version2);
        var prepared = await user.PrepareEnrolledUpdateAsync();
        Require(prepared.Success && user.AuthorizationRequests == 1 && user.TrustedUpdateRequests == 1,
            "later signed installer preparation updates through enrolled transaction without UAC");
        Require(user.Transaction.ReadState().Sequence == 2, "new signed release sequence commits");
        user.VerifyDeployed();
        Step("install-v2-prepare-update", user, prepared.Data!);

        var updatedOpen = await user.OpenAsync();
        Require(updatedOpen.Success && updatedOpen.Data == "running" && user.AuthorizationRequests == 1,
            "first launch after signed update must not request authorization");
        Step("first-open-v2", user, updatedOpen.Data!);

        // Manual replacement goes through the real EnsureCoreAsync trusted-upgrade branch as well.
        var version3 = user.Publish(3);
        user.InstallFiles(version3);
        var manual = await user.OpenAsync();
        Require(manual.Success && manual.Data == "upgraded" && user.AuthorizationRequests == 1 &&
            user.TrustedUpdateRequests == 2 && user.Transaction.ReadState().Sequence == 3,
            "manual signed replacement must use the same trusted transaction without UAC");
        user.VerifyDeployed();
        Step("manual-v3-first-open", user, manual.Data!);

        var invalid = user.Publish(4);
        user.InstallFiles(invalid);
        File.AppendAllText(Path.Combine(user.InstallDirectory, "browser_extension", "nxtools-interceptor", "manifest.json"), "tampered");
        var rejected = await user.PrepareEnrolledUpdateAsync();
        Require(!rejected.Success && user.AuthorizationRequests == 1 && user.Transaction.ReadState().Sequence == 3,
            "tampered release cannot trigger fallback authorization or advance state");
        Step("tampered-v4-rejected", user, rejected.Data!);
        Console.WriteLine("SIMULATION_RESULT scenario=fresh-install-update consent_requests=1 consent_granted=1 valid_releases=3 tampered_release=rejected native_services=not_executed");
    }

    internal static async Task DeclinedFirstConsentLeavesNoEnrolledService()
    {
        using var user = new NewUser { Consent = false };
        user.InstallFiles(user.Publish(1));
        var declined = await user.OpenAsync();
        Require(!declined.Success && declined.Data == "authorization_cancelled" && user.AuthorizationRequests == 1 &&
            user.AuthorizationGrants == 0 && !user.ServiceExists && !user.Enrollment.IsComplete(),
            "declined first consent must not register a service or persist completed trust");
        Step("first-open-consent-no", user, declined.Data!);
        user.Consent = true;
        var accepted = await user.OpenAsync();
        Require(accepted.Success && user.AuthorizationRequests == 2 && user.AuthorizationGrants == 1,
            "user may explicitly retry after declining; there is still only one successful enrollment");
        user.VerifyDeployed();
        var nextOpen = await user.OpenAsync();
        Require(nextOpen.Success && user.AuthorizationRequests == 2, "successful retry stops further consent requests");
        Step("retry-consent-yes-then-reopen", user, nextOpen.Data!);
        Console.WriteLine("SIMULATION_RESULT scenario=decline-then-retry consent_requests=2 consent_granted=1 service_created_before_consent=false native_services=not_executed");
    }

    private static void Step(string step, NewUser user, string outcome) => Console.WriteLine(
        $"SIMULATION_STEP {step} outcome={outcome} consent_requests_total={user.AuthorizationRequests} consent_granted_total={user.AuthorizationGrants} trust_completed={user.Enrollment.IsComplete().ToString().ToLowerInvariant()}");

    private sealed class NewUser : IndexerServiceInstaller.IDeploymentPlatform, TrustedUpdateTransaction.IDeployment, IDisposable
    {
        private readonly string root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath(),
            "new-user-simulation-" + Guid.NewGuid().ToString("N"));
        private readonly string signingKey;
        private readonly byte[] publicKey;
        private string ProtectedDirectory => Path.Combine(root, "protected-fixture");
        private string UpdaterDirectory => Path.Combine(root, "updater-fixture");
        private string Store => Path.Combine(UpdaterDirectory, "releases");
        private string RegistrationMarker => Path.Combine(UpdaterDirectory, "service-registration.fixture");
        internal string InstallDirectory => Path.Combine(root, "user-install");
        internal TrustedUpdateEnrollment Enrollment { get; private set; }
        internal TrustedUpdateTransaction Transaction { get; private set; }
        internal bool Consent { get; set; } = true;
        internal int AuthorizationRequests { get; private set; }
        internal int AuthorizationGrants { get; private set; }
        internal int TrustedUpdateRequests { get; private set; }
        internal bool ServiceExists => File.Exists(RegistrationMarker);
        internal NewUser()
        {
            Directory.CreateDirectory(root);
            using var rsa = RSA.Create(2048);
            publicKey = rsa.ExportSubjectPublicKeyInfo();
            signingKey = Path.Combine(root, "temporary-publisher.pem");
            File.WriteAllText(signingKey, rsa.ExportPkcs8PrivateKeyPem());
            Enrollment = new(UpdaterDirectory);
            Transaction = new(Store, publicKey, this, new SilentLog());
        }
        internal void ReloadOwners()
        {
            Enrollment = new(UpdaterDirectory);
            Transaction = new(Store, publicKey, this, new SilentLog());
        }
        internal string Publish(long sequence)
        {
            var package = Path.Combine(root, "release-" + sequence);
            Directory.CreateDirectory(Path.Combine(package, "browser_extension", "nxtools-interceptor"));
            foreach (var file in new[] { "NxTools.exe", "NxTools.PlatformHost.exe", "NxTools.Indexer.exe", "NxTools.BrowserHost.exe", "NxTools.Cli.exe",
                "LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY_NOTICES.md", "browser_extension/nxtools-interceptor/manifest.json" })
                File.WriteAllText(Path.Combine(package, file.Replace('/', Path.DirectorySeparatorChar)), $"synthetic-release-{sequence}:{file}");
            TrustedReleaseManifest.SignPackage(package, signingKey, sequence, Guid.NewGuid());
            return package;
        }
        internal void InstallFiles(string package)
        {
            using var verified = TrustedReleaseManifest.VerifyPackage(package, publicKey, 0);
            ReplaceFixturePackage(verified, InstallDirectory);
        }
        internal Task<WindowsCommandResult> OpenAsync() =>
            IndexerServiceInstaller.EnsureCoreAsync(this, new SilentLog(), Guid.NewGuid(), CancellationToken.None);
        internal Task<WindowsCommandResult> PrepareEnrolledUpdateAsync() =>
            HasTrustedUpdater() ? UpdateTrustedAsync(Guid.NewGuid(), CancellationToken.None) : Task.FromResult(new WindowsCommandResult(true, Data: "not_enrolled"));
        public IndexerServiceInstaller.DeploymentState ReadState()
        {
            using var current = TrustedReleaseManifest.VerifyPackage(InstallDirectory, publicKey, 0);
            if (!ServiceExists) return new(false, false, null, Tuple(current));
            using var approved = TrustedReleaseManifest.VerifyPackage(ProtectedDirectory, publicKey, 0);
            return new(true, true, Tuple(approved), Tuple(current));
        }
        public bool NeedsTrustEnrollment() => !Enrollment.IsComplete();
        public bool HasTrustedUpdater() => ServiceExists && Enrollment.IsComplete();
        public bool StartService() { VerifyDeployed(); return ServiceExists; }
        public Task<WindowsCommandResult> InstallAsync(Guid correlationId, CancellationToken cancellationToken)
        {
            AuthorizationRequests++;
            if (!Consent) throw new Win32Exception(1223);
            AuthorizationGrants++;
            using var verified = TrustedReleaseManifest.VerifyPackage(InstallDirectory, publicKey, 0);
            var identity = new TrustedUpdateEnrollment.Identity(verified.Manifest.PublicKeySha256,
                verified.Files.Single(file => file.Path == "NxTools.PlatformHost.exe").Sha256,
                verified.Manifest.ReleaseSequence, verified.Manifest.ReleaseId);
            ReplaceFixturePackage(verified, ProtectedDirectory);
            Enrollment.Run(identity,
                () => { Directory.CreateDirectory(UpdaterDirectory); Directory.CreateDirectory(Store); },
                () => Transaction.Initialize(InstallDirectory),
                () => File.WriteAllText(RegistrationMarker, "synthetic registration; not SCM"),
                () => Require(ServiceExists, "fixture service registration must exist"));
            return Task.FromResult(new WindowsCommandResult(true, Data: "installed"));
        }
        public async Task<WindowsCommandResult> UpdateTrustedAsync(Guid correlationId, CancellationToken cancellationToken)
        {
            TrustedUpdateRequests++;
            try
            {
                Require(HasTrustedUpdater(), "trusted update requires completed enrollment");
                await Transaction.ApplyAsync(InstallDirectory, correlationId, cancellationToken);
                VerifyDeployed();
                return new(true, Data: "trusted_updated");
            }
            catch (Exception error) when (error is InvalidDataException or CryptographicException or IOException)
            {
                return new(false, "Synthetic signed update rejected.", Data: "trusted_update_rejected");
            }
        }
        public Task DeployAsync(string verifiedPackage, Guid correlationId, CancellationToken cancellationToken)
        {
            using var verified = TrustedReleaseManifest.VerifyPackage(verifiedPackage, publicKey, 0);
            ReplaceFixturePackage(verified, ProtectedDirectory);
            return Task.CompletedTask;
        }
        internal void VerifyDeployed()
        {
            using var installed = TrustedReleaseManifest.VerifyPackage(InstallDirectory, publicKey, 0);
            using var deployed = TrustedReleaseManifest.VerifyPackage(ProtectedDirectory, publicKey, 0);
            Require(installed.ManifestSha256 == deployed.ManifestSha256, "every deployed file and release identity must match install");
        }
        private static IndexerServiceInstaller.ReleaseTuple Tuple(VerifiedRelease release) => new(
            release.Files.Single(file => file.Path == "NxTools.exe").Sha256,
            release.Files.Single(file => file.Path == "NxTools.PlatformHost.exe").Sha256,
            release.Files.Single(file => file.Path == "NxTools.Indexer.exe").Sha256);
        private static void ReplaceFixturePackage(VerifiedRelease source, string destination)
        {
            if (Directory.Exists(destination)) Directory.Delete(destination, true);
            Directory.CreateDirectory(destination);
            source.CopyTo(destination);
        }
        public void Dispose() => Directory.Delete(root, true);
    }
    private sealed class SilentLog : IPlatformLog
    {
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) { }
    }
    private static void Require(bool value, string message)
    {
        if (!value) throw new InvalidOperationException(message);
    }
}
