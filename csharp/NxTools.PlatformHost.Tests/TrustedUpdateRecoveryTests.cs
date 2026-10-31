using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// GUID-owned files and ephemeral RSA only; no enrollment, registry, service or product operations.
internal static class TrustedUpdateRecoveryTests
{
    private static readonly string[] LegacyPaths =
    [
        "NxTools.exe", "NxTools.PlatformHost.exe", "NxTools.Indexer.exe", "NxTools.BrowserHost.exe", "NxTools.Cli.exe",
        "LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY_NOTICES.md", "browser_extension/nxtools-interceptor/manifest.json"
    ];

    public static async Task LegacySnapshotRollsBackFailedUpdateAndCommitsStrictRetry()
    {
        using var fixture = new Fixture();
        var previous = fixture.SeedLegacy();
        var transaction = fixture.Transaction();
        var next = fixture.Package(22);
        fixture.Deployment.FailSequence = 22;
        await Reject(() => transaction.ApplyAsync(next, Guid.NewGuid(), CancellationToken.None));
        Assert(fixture.Deployment.Sequences.SequenceEqual(new long[] { 22, 21 }),
            "new deployment must actually start, then restore the historical signed snapshot");
        Assert(transaction.ReadState() == previous, "failed deployment must preserve all durable identity fields");
        Assert(!File.Exists(Path.Combine(fixture.Store, "transaction.json")), "successful recovery retires journal");
        foreach (var path in LegacyPaths)
            Assert(File.ReadAllBytes(Path.Combine(fixture.Active, path)).SequenceEqual(
                File.ReadAllBytes(Path.Combine(fixture.Deployment.Target, path))), "old artifact restored: " + path);
        Assert(TrustedReleaseManifestTests.ArchiveMembers.All(name => !File.Exists(Path.Combine(fixture.Deployment.Target, name))),
            "failed current-only archive members must not remain beside the restored legacy release");
        Reject(() => TrustedReleaseManifest.VerifyPackage(fixture.Active, fixture.PublicKey, 0).Dispose());
        fixture.Deployment.FailSequence = null;
        var committed = await transaction.ApplyAsync(next, Guid.NewGuid(), CancellationToken.None);
        Assert(committed.Sequence == 22 && transaction.ReadState() == committed, "strict retry commits newer release");
        using var strict = TrustedReleaseManifest.VerifyPackage(fixture.Active, fixture.PublicKey, 0);
        Assert(TrustedReleaseManifestTests.ArchiveMembers.All(member => strict.Files.Any(file => file.Path == member)),
            "committed new active contains the complete current required set");
        Assert(fixture.Deployment.Sequences.SequenceEqual(new long[] { 22, 21, 22 }), "retry deploys exactly once");
    }

    public static Task SupersededCleanupPreservesForeignFiles()
    {
        using var fixture = new Fixture();
        var prior = fixture.SeedLegacy();
        var next = fixture.Package(22);
        using var previous = fixture.Verify(fixture.Active, prior);
        using var failed = TrustedReleaseManifest.VerifyPackage(next, fixture.PublicKey, 0);
        Directory.CreateDirectory(fixture.Deployment.Target);
        var foreign = Path.Combine(fixture.Deployment.Target, "NxTools.ArchiveWorker.exe");
        File.WriteAllText(foreign, "foreign bytes");
        Reject(() => TrustedReleaseManifest.RemoveSupersededArchiveFiles(fixture.Deployment.Target, previous, failed));
        Assert(File.ReadAllText(foreign) == "foreign bytes", "wrong-hash current-only member survives recovery cleanup");
        return Task.CompletedTask;
    }

    public static async Task RestartRestoresPreciselyBoundLegacySnapshot()
    {
        using var fixture = new Fixture();
        var previous = fixture.SeedLegacy();
        var nextPackage = fixture.Package(22);
        using (var next = TrustedReleaseManifest.VerifyPackage(nextPackage, fixture.PublicKey, 0))
        {
            var pending = Path.Combine(fixture.Store, "pending");
            Directory.CreateDirectory(pending);
            next.CopyTo(pending);
            var nextState = new TrustedUpdateTransaction.State(22, next.Manifest.ReleaseId, next.ManifestSha256);
            fixture.Journal(previous, nextState);
            Directory.Move(fixture.Active, Path.Combine(fixture.Store, "backup"));
            Directory.Move(pending, fixture.Active);
        }
        await fixture.Transaction().RecoverAsync(Guid.NewGuid(), CancellationToken.None);
        Assert(fixture.Transaction().ReadState() == previous && fixture.Deployment.Sequences.SequenceEqual(new long[] { 21 }),
            "interrupted publish restores legacy backup, not the uncommitted current-format active");
        Assert(!Directory.Exists(Path.Combine(fixture.Store, "backup")), "recovery releases handles before moving backup");
        using var recovered = fixture.Verify(fixture.Active, previous);
        Assert(recovered.Manifest.ReleaseSequence == 21, "published recovered active has the durable old identity");
    }

    public static async Task RecoverySnapshotRejectsWrongIdentityAndInvalidArtifacts()
    {
        foreach (var fault in new[] { "sequence", "id", "hash", "missing", "tamper", "extension", "listed-current", "signature", "required", "path" })
        {
            using var fixture = new Fixture();
            // Include a current optional artifact to prove recovery still validates every listed file.
            var previous = fixture.SeedLegacy(includeCurrentMember: fault == "listed-current");
            var expected = fault switch
            {
                "sequence" => previous with { Sequence = 20 },
                "id" => previous with { ReleaseId = Guid.NewGuid() },
                "hash" => previous with { ManifestSha256 = new string('0', 64) },
                _ => previous
            };
            if (fault == "missing") File.Delete(Path.Combine(fixture.Active, "NxTools.Cli.exe"));
            if (fault == "tamper") File.AppendAllText(Path.Combine(fixture.Active, "NxTools.exe"), "changed");
            if (fault == "extension") File.AppendAllText(Path.Combine(fixture.Active, "browser_extension/nxtools-interceptor/asset.js"), "changed");
            if (fault == "listed-current") File.AppendAllText(Path.Combine(fixture.Active, "NxTools.ArchiveWorker.exe"), "changed");
            if (fault == "signature") File.WriteAllBytes(Path.Combine(fixture.Active, TrustedReleaseManifest.SignatureName), new byte[256]);
            if (fault is "required" or "path")
            {
                var files = fixture.Manifest(fixture.Active).Files.ToList();
                if (fault == "required") files.RemoveAll(file => file.Path == "NxTools.Cli.exe");
                else
                {
                    File.WriteAllText(Path.Combine(fixture.Active, "foreign.exe"), "not allowed");
                    files.Add(fixture.FileEntry(fixture.Active, "foreign.exe"));
                }
                expected = fixture.Resign(fixture.Active, fixture.Manifest(fixture.Active) with { Files = files });
            }
            File.WriteAllText(Path.Combine(fixture.Store, "state.json"), JsonSerializer.Serialize(expected));
            fixture.Journal(expected, new TrustedUpdateTransaction.State(22, Guid.NewGuid(), new string('1', 64)));
            Reject(() => fixture.Verify(fixture.Active, expected).Dispose());
            await Reject(() => fixture.Transaction().RecoverAsync(Guid.NewGuid(), CancellationToken.None));
            Assert(fixture.Deployment.Sequences.Count == 0, fault + ": recovery rejects before deployment");
            Assert(fixture.Transaction().ReadState() == expected && File.Exists(Path.Combine(fixture.Store, "transaction.json")),
                fault + ": failed recovery retains state and journal");
        }
    }

    public static async Task RecoverySnapshotCannotAdmitLegacyAsNewPackage()
    {
        using var fixture = new Fixture();
        var previous = fixture.SeedLegacy();
        var transaction = fixture.Transaction();
        Reject(() => TrustedReleaseManifest.VerifyPackage(fixture.Active, fixture.PublicKey, 0).Dispose());
        Reject(() => transaction.Initialize(fixture.Active));
        var unenrolled = Path.Combine(fixture.Root, "unenrolled");
        Directory.CreateDirectory(unenrolled);
        Reject(() => new TrustedUpdateTransaction(unenrolled, fixture.PublicKey, fixture.Deployment, new SilentLog()).Initialize(fixture.Active));
        Assert(!File.Exists(Path.Combine(unenrolled, "state.json")), "new enrollment cannot use historical completeness");
        var unsignedLegacy = fixture.LegacyPackage(23, sign: false);
        Reject(() => TrustedReleaseManifest.SignPackage(unsignedLegacy, fixture.Key, 23, Guid.NewGuid()));
        await Reject(() => transaction.ApplyAsync(fixture.Active, Guid.NewGuid(), CancellationToken.None));
        using (var sameLegacy = fixture.Verify(fixture.Active, previous))
            await Reject(() => transaction.ApplyVerifiedAsync(sameLegacy, Guid.NewGuid(), CancellationToken.None));
        var newerLegacy = fixture.LegacyPackage(23);
        var newerIdentity = fixture.Identity(newerLegacy);
        using (var verifiedLegacy = fixture.Verify(newerLegacy, newerIdentity))
            await Reject(() => transaction.ApplyVerifiedAsync(verifiedLegacy, Guid.NewGuid(), CancellationToken.None));
        var currentFormat = fixture.Package(24);
        using (var recoveryOnly = fixture.Verify(currentFormat, fixture.Identity(currentFormat)))
            await Reject(() => transaction.ApplyVerifiedAsync(recoveryOnly, Guid.NewGuid(), CancellationToken.None));
        await transaction.ApplyAsync(fixture.Package(22), Guid.NewGuid(), CancellationToken.None);
        using (var old = fixture.Verify(fixture.ActiveLegacyCopy, previous))
            await Reject(() => transaction.ApplyVerifiedAsync(old, Guid.NewGuid(), CancellationToken.None));
        await Reject(() => transaction.ApplyAsync(fixture.Package(20), Guid.NewGuid(), CancellationToken.None));
        Assert(transaction.ReadState().Sequence == 22 && fixture.Deployment.Sequences.SequenceEqual(new long[] { 22 }),
            "legacy recovery never authorizes a new request, same-identity shortcut or downgrade");
    }

    private sealed class Fixture : IDisposable
    {
        internal string Root { get; } = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath(), "trusted-recovery-" + Guid.NewGuid().ToString("N"));
        internal string Store => Path.Combine(Root, "store");
        internal string Active => Path.Combine(Store, "active");
        internal string ActiveLegacyCopy { get; private set; } = "";
        internal string Key => Path.Combine(Root, "ephemeral.pem");
        private readonly RSA rsa = RSA.Create(2048);
        internal byte[] PublicKey { get; }
        internal DeploymentFixture Deployment { get; }
        internal Fixture()
        {
            Directory.CreateDirectory(Store);
            PublicKey = rsa.ExportSubjectPublicKeyInfo();
            File.WriteAllText(Key, rsa.ExportPkcs8PrivateKeyPem());
            Deployment = new DeploymentFixture(PublicKey, Path.Combine(Root, "fake-target"));
        }
        internal TrustedUpdateTransaction Transaction() => new(Store, PublicKey, Deployment, new SilentLog());
        internal VerifiedRelease Verify(string path, TrustedUpdateTransaction.State expected) =>
            TrustedReleaseManifest.VerifyRecoverySnapshot(path, PublicKey, expected.Sequence, expected.ReleaseId, expected.ManifestSha256);
        internal ReleaseManifest Manifest(string path) => JsonSerializer.Deserialize<ReleaseManifest>(File.ReadAllBytes(Path.Combine(path, TrustedReleaseManifest.ManifestName)))!;
        internal TrustedUpdateTransaction.State Identity(string path)
        {
            var manifest = Manifest(path);
            return new(manifest.ReleaseSequence, manifest.ReleaseId, TrustedReleaseManifest.Hash(File.ReadAllBytes(Path.Combine(path, TrustedReleaseManifest.ManifestName))));
        }
        internal ReleaseFile FileEntry(string path, string name)
        {
            var bytes = File.ReadAllBytes(Path.Combine(path, name));
            return new(name, TrustedReleaseManifest.Hash(bytes), bytes.LongLength);
        }
        internal TrustedUpdateTransaction.State Resign(string path, ReleaseManifest manifest)
        {
            var bytes = JsonSerializer.SerializeToUtf8Bytes(manifest);
            File.WriteAllBytes(Path.Combine(path, TrustedReleaseManifest.ManifestName), bytes);
            File.WriteAllBytes(Path.Combine(path, TrustedReleaseManifest.SignatureName), rsa.SignData(bytes, HashAlgorithmName.SHA256, RSASignaturePadding.Pss));
            return Identity(path);
        }
        internal string LegacyPackage(long sequence, bool sign = true, bool includeCurrentMember = false)
        {
            var path = Path.Combine(Root, "legacy-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(Path.Combine(path, "browser_extension/nxtools-interceptor"));
            var names = LegacyPaths.Append("browser_extension/nxtools-interceptor/asset.js").ToList();
            if (includeCurrentMember) names.Add("NxTools.ArchiveWorker.exe");
            foreach (var name in names) File.WriteAllText(Path.Combine(path, name), "synthetic historical " + sequence + " " + name);
            // Deliberately do not call the current strict SignPackage to manufacture a historical manifest.
            if (sign) Resign(path, new ReleaseManifest(1, "nxtools-next", sequence, Guid.NewGuid(), TrustedReleaseManifest.Hash(PublicKey), names.Select(name => FileEntry(path, name)).ToArray()));
            return path;
        }
        internal TrustedUpdateTransaction.State SeedLegacy(bool includeCurrentMember = false)
        {
            ActiveLegacyCopy = LegacyPackage(21, includeCurrentMember: includeCurrentMember);
            Directory.CreateDirectory(Active);
            foreach (var source in Directory.EnumerateFiles(ActiveLegacyCopy, "*", SearchOption.AllDirectories))
            {
                var target = Path.Combine(Active, Path.GetRelativePath(ActiveLegacyCopy, source));
                Directory.CreateDirectory(Path.GetDirectoryName(target)!);
                File.Copy(source, target);
            }
            var state = Identity(Active);
            File.WriteAllText(Path.Combine(Store, "state.json"), JsonSerializer.Serialize(state));
            return state;
        }
        internal string Package(long sequence)
        {
            var path = LegacyPackage(sequence, sign: false);
            foreach (var member in TrustedReleaseManifestTests.ArchiveMembers) File.WriteAllText(Path.Combine(path, member), "synthetic current " + sequence + " " + member);
            TrustedReleaseManifest.SignPackage(path, Key, sequence, Guid.NewGuid());
            return path;
        }
        internal void Journal(TrustedUpdateTransaction.State previous, TrustedUpdateTransaction.State next) =>
            File.WriteAllText(Path.Combine(Store, "transaction.json"), JsonSerializer.Serialize(new { Previous = previous, Next = next }));
        public void Dispose() { rsa.Dispose(); Directory.Delete(Root, true); }
    }

    private sealed class DeploymentFixture(byte[] publicKey, string target) : TrustedUpdateTransaction.IDeployment
    {
        internal string Target => target;
        internal long? FailSequence { get; set; }
        internal List<long> Sequences { get; } = [];
        public Task DeployAsync(string package, Guid correlationId, CancellationToken cancellationToken)
        {
            using var verified = TrustedReleaseManifest.VerifyPackage(package, publicKey, 0);
            return DeployAsync(package, verified, correlationId, cancellationToken);
        }
        public Task DeployAsync(string package, VerifiedRelease verified, Guid correlationId, CancellationToken cancellationToken)
        {
            Sequences.Add(verified.Manifest.ReleaseSequence);
            foreach (var file in verified.Files)
            {
                var destination = Path.Combine(target, file.Path);
                Directory.CreateDirectory(Path.GetDirectoryName(destination)!);
                File.Copy(Path.Combine(package, file.Path), destination, true);
                if (FailSequence == verified.Manifest.ReleaseSequence && file.Path == "NxTools.ShellSelectionHelper.runtimeconfig.json") throw new IOException("synthetic partial deployment failure after archive members");
            }
            return Task.CompletedTask;
        }
        public Task RemoveSupersededAsync(VerifiedRelease previous, VerifiedRelease failed, CancellationToken cancellationToken)
        {
            TrustedReleaseManifest.RemoveSupersededArchiveFiles(target, previous, failed);
            return Task.CompletedTask;
        }
    }
    private sealed class SilentLog : IPlatformLog
    {
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) { }
    }
    private static void Reject(Action action)
    {
        try { action(); }
        catch (Exception error) when (IsRejection(error)) { return; }
        throw new InvalidOperationException("Expected recovery/update rejection.");
    }
    private static async Task Reject(Func<Task> action)
    {
        try { await action(); }
        catch (Exception error) when (IsRejection(error)) { return; }
        throw new InvalidOperationException("Expected recovery/update rejection.");
    }
    private static bool IsRejection(Exception error) => error is IOException or InvalidDataException or CryptographicException or System.ComponentModel.Win32Exception;
    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
