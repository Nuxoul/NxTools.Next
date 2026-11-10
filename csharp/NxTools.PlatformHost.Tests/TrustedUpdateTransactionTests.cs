using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class TrustedUpdateTransactionTests
{
    public static async Task SignedUpdateCommitsOnceAndRejectsDowngrade()
    {
        using var fixture = new Fixture();
        var transaction = fixture.Transaction();
        transaction.Initialize(fixture.Package(10));
        var initialState = transaction.ReadState();
        File.Delete(Path.Combine(fixture.Store, "state.json"));
        transaction.Initialize(Path.Combine(fixture.Store, "active"));
        Assert(transaction.ReadState() == initialState, "enrollment resumes after active snapshot publish but before state write");
        var next = fixture.Package(11);
        var correlation = Guid.NewGuid();
        var result = await transaction.ApplyAsync(next, correlation, CancellationToken.None);
        Assert(result.Sequence == 11 && fixture.Deployment.Sequences.SequenceEqual(new long[] { 11 }), "verified newer release deploys once");
        Assert(fixture.Deployment.Correlations.All(value => value == correlation), "deployment correlation survives staging");
        await transaction.ApplyAsync(next, correlation, CancellationToken.None);
        Assert(fixture.Deployment.Sequences.Count == 1, "same signed identity is idempotent");
        await Reject(() => transaction.ApplyAsync(fixture.Package(9), correlation, CancellationToken.None));
        Assert(transaction.ReadState().Sequence == 11 && fixture.Deployment.Sequences.Count == 1, "downgrade cannot mutate approved state");
        var tampered = fixture.Package(12);
        File.AppendAllText(Path.Combine(tampered, "NxTools.exe"), "tamper");
        await Reject(() => transaction.ApplyAsync(tampered, correlation, CancellationToken.None));
        Assert(transaction.ReadState().Sequence == 11 && fixture.Deployment.Sequences.Count == 1, "tamper rejected before deployment");
    }

    public static async Task InitialDeploymentPendingRepair()
    {
        using var fixture = new Fixture();
        var transaction = fixture.Transaction();
        var initial = fixture.Package(10);
        transaction.Initialize(initial, requireInitialDeployment: true);
        var statePath = Path.Combine(fixture.Store, "state.json");
        var originalBytes = File.ReadAllBytes(statePath);
        Assert(transaction.ReadState().DeploymentPending && !Directory.Exists(fixture.Deployment.Target),
            "repair snapshot is staged, not an installed protected tuple");
        transaction.Initialize(initial, requireInitialDeployment: true);
        Assert(File.ReadAllBytes(statePath).SequenceEqual(originalBytes), "interrupted enrollment retains initial deployment truth");
        var correlation = Guid.NewGuid();
        var different = fixture.Package(10);
        await Reject(() => transaction.ApplyAsync(different, correlation, CancellationToken.None));
        await Reject(() => transaction.ApplyAsync(fixture.Package(9), correlation, CancellationToken.None));
        Assert(transaction.ReadState().DeploymentPending && fixture.Deployment.Sequences.Count == 0,
            "same-sequence different identity and downgrade fail before initial deployment");
        fixture.Deployment.FailSequence = 10;
        await Reject(() => fixture.Transaction().ApplyAsync(initial, correlation, CancellationToken.None));
        Assert(transaction.ReadState().DeploymentPending && File.ReadAllBytes(statePath).SequenceEqual(originalBytes) &&
            fixture.Deployment.Sequences.SequenceEqual(new long[] { 10 }), "failed native deployment leaves durable pending state for retry");
        fixture.Deployment.FailSequence = null;
        var installed = await fixture.Transaction().ApplyAsync(initial, correlation, CancellationToken.None);
        Assert(!installed.DeploymentPending && transaction.ReadState() == installed &&
            fixture.Deployment.Sequences.SequenceEqual(new long[] { 10, 10 }) &&
            fixture.Deployment.Correlations.All(id => id == correlation), "retry deploys real initial tuple before publishing completion");
        using (var release = TrustedReleaseManifest.VerifyPackage(initial, fixture.PublicKey, 0))
            foreach (var member in release.Files)
                Assert(File.ReadAllBytes(Path.Combine(fixture.Deployment.Target, member.Path)).SequenceEqual(
                    File.ReadAllBytes(Path.Combine(initial, member.Path))), "initial protected tuple matches signed release");
        Assert(!JsonDocument.Parse(File.ReadAllBytes(statePath)).RootElement.TryGetProperty("DeploymentPending", out _),
            "ordinary state serialization retains its original fields");
        await transaction.ApplyAsync(initial, correlation, CancellationToken.None);
        Assert(fixture.Deployment.Sequences.Count == 2, "completed same release never redeploys");
        var newer = fixture.Package(11);
        Assert((await transaction.ApplyAsync(newer, correlation, CancellationToken.None)).Sequence == 11 &&
            fixture.Deployment.Sequences.SequenceEqual(new long[] { 10, 10, 11 }), "newer signed release retains ordinary update sequence");
        await Reject(() => transaction.ApplyAsync(initial, correlation, CancellationToken.None));
        Assert(transaction.ReadState().Sequence == 11 && fixture.Deployment.Sequences.Count == 3, "downgrade remains refused");
        using var newerFirst = new Fixture();
        var repair = newerFirst.Transaction();
        repair.Initialize(newerFirst.Package(10), requireInitialDeployment: true);
        var nextPackage = newerFirst.Package(11);
        var advanced = await repair.ApplyAsync(nextPackage, correlation, CancellationToken.None);
        Assert(advanced.Sequence == 11 && !advanced.DeploymentPending &&
            newerFirst.Deployment.Sequences.SequenceEqual(new long[] { 10, 11 }),
            "newer request first deploys the pinned repair release before ordinary upgrade");
        using var tampered = new Fixture();
        var tamperTransaction = tampered.Transaction();
        tamperTransaction.Initialize(tampered.Package(10), requireInitialDeployment: true);
        File.AppendAllText(Path.Combine(tampered.Store, "active", "NxTools.exe"), "tampered");
        await Reject(() => tamperTransaction.ApplyAsync(tampered.Package(11), correlation, CancellationToken.None));
        Assert(tamperTransaction.ReadState().DeploymentPending && tampered.Deployment.Sequences.Count == 0,
            "changed durable repair snapshot cannot deploy or clear pending state");
    }

    public static async Task PartialDeploymentRestoresLastGoodAndCanRetry()
    {
        using var fixture = new Fixture();
        var transaction = fixture.Transaction();
        transaction.Initialize(fixture.Package(10));
        var next = fixture.Package(11);
        fixture.Deployment.FailSequence = 11;
        await Reject(() => transaction.ApplyAsync(next, Guid.NewGuid(), CancellationToken.None));
        Assert(transaction.ReadState().Sequence == 10 && fixture.Deployment.Sequences.SequenceEqual(new long[] { 11, 10 }),
            "partial native failure restores signed last-good release before returning failure");
        Assert(!File.Exists(Path.Combine(fixture.Store, "transaction.json")), "successful rollback retires journal");
        fixture.Deployment.FailSequence = null;
        Assert((await transaction.ApplyAsync(next, Guid.NewGuid(), CancellationToken.None)).Sequence == 11, "failed release remains retryable");
    }

    public static async Task RestartRecoversInterruptedPublishAndCommittedCleanup()
    {
        using var fixture = new Fixture();
        var transaction = fixture.Transaction();
        transaction.Initialize(fixture.Package(10));
        var previous = transaction.ReadState();
        var nextPackage = fixture.Package(11);
        using (var verified = TrustedReleaseManifest.VerifyPackage(nextPackage, fixture.PublicKey, 0))
        {
            var pending = Path.Combine(fixture.Store, "pending");
            Directory.CreateDirectory(pending);
            verified.CopyTo(pending);
            var next = new TrustedUpdateTransaction.State(11, verified.Manifest.ReleaseId, verified.ManifestSha256);
            File.WriteAllText(Path.Combine(fixture.Store, "transaction.json"), JsonSerializer.Serialize(new { Previous = previous, Next = next }));
            Directory.Move(Path.Combine(fixture.Store, "active"), Path.Combine(fixture.Store, "backup"));
            Directory.Move(pending, Path.Combine(fixture.Store, "active"));
        }
        await fixture.Transaction().RecoverAsync(Guid.NewGuid(), CancellationToken.None);
        Assert(transaction.ReadState() == previous && fixture.Deployment.Sequences.SequenceEqual(new long[] { 10 }),
            "restart restores old tuple when directory publish precedes durable state commit");
        var committed = await transaction.ApplyAsync(nextPackage, Guid.NewGuid(), CancellationToken.None);
        File.WriteAllText(Path.Combine(fixture.Store, "transaction.json"), JsonSerializer.Serialize(new { Previous = previous, Next = committed }));
        var before = fixture.Deployment.Sequences.Count;
        await fixture.Transaction().RecoverAsync(Guid.NewGuid(), CancellationToken.None);
        Assert(fixture.Deployment.Sequences.Count == before && transaction.ReadState() == committed,
            "committed cleanup does not redeploy or roll back");
    }

    public static async Task FramesRejectOversizeTruncationAndCancellation()
    {
        using var memory = new MemoryStream();
        var value = new { CorrelationId = Guid.NewGuid(), Stage = "synthetic" };
        await TrustedUpdateBroker.WriteFrameAsync(memory, value, CancellationToken.None);
        memory.Position = 0;
        var decoded = await TrustedUpdateBroker.ReadFrameAsync<JsonElement>(memory, CancellationToken.None);
        Assert(decoded.GetProperty("CorrelationId").GetGuid() == value.CorrelationId, "bounded frame round trip");
        using var oversized = new MemoryStream(BitConverter.GetBytes(16 * 1024 + 1));
        await Reject(() => TrustedUpdateBroker.ReadFrameAsync<JsonElement>(oversized, CancellationToken.None));
        using var truncated = new MemoryStream([1, 0]);
        await Reject(() => TrustedUpdateBroker.ReadFrameAsync<JsonElement>(truncated, CancellationToken.None));
        using var cancelled = new CancellationTokenSource(); cancelled.Cancel();
        await Reject(() => TrustedUpdateBroker.WriteFrameAsync(memory, value, cancelled.Token));
    }

    internal static async Task ArchiveMemberMutationRestoresCompleteOwnedSet()
    {
        using var fixture = new Fixture();
        var previous = fixture.Package(10);
        var transaction = fixture.Transaction();
        transaction.Initialize(previous);
        await fixture.Deployment.DeployAsync(previous, Guid.NewGuid(), CancellationToken.None);
        var next = fixture.Package(11);
        var foreign = Path.Combine(fixture.Deployment.Target, "foreign.keep");
        File.WriteAllText(foreign, "foreign bytes");
        foreach (var member in TrustedReleaseManifestTests.ArchiveMembers) {
            fixture.Deployment.FailSequence = 11;
            fixture.Deployment.FailAfterMember = member;
            await Reject(() => transaction.ApplyAsync(next, Guid.NewGuid(), CancellationToken.None));
            Assert(transaction.ReadState().Sequence == 10, "failed archive update cannot commit sequence");
            using var old = TrustedReleaseManifest.VerifyPackage(previous, fixture.PublicKey, 0);
            foreach (var file in old.Files)
                Assert(File.ReadAllBytes(Path.Combine(fixture.Deployment.Target, file.Path)).SequenceEqual(
                    File.ReadAllBytes(Path.Combine(previous, file.Path))), "complete last-good set restored: " + file.Path);
            Assert(File.ReadAllText(foreign) == "foreign bytes", "foreign target file preserved");
        }
        fixture.Deployment.FailSequence = null;
        Assert((await transaction.ApplyAsync(next, Guid.NewGuid(), CancellationToken.None)).Sequence == 11,
            "complete archive release remains retryable");
    }

    private sealed class Fixture : IDisposable
    {
        private readonly string root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath(), "trusted-update-" + Guid.NewGuid().ToString("N"));
        private readonly string key;
        internal byte[] PublicKey { get; }
        internal string Store => Path.Combine(root, "store");
        internal DeploymentFixture Deployment { get; }
        internal Fixture()
        {
            Directory.CreateDirectory(root); Directory.CreateDirectory(Store);
            using var rsa = RSA.Create(2048);
            key = Path.Combine(root, "temporary.pem");
            File.WriteAllText(key, rsa.ExportPkcs8PrivateKeyPem());
            PublicKey = rsa.ExportSubjectPublicKeyInfo();
            Deployment = new DeploymentFixture(PublicKey, Path.Combine(root, "fake-target"));
        }
        internal string Package(long sequence)
        {
            var directory = Path.Combine(root, "package-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(Path.Combine(directory, "browser_extension", "nxtools-interceptor"));
            foreach (var name in new[] { "NxTools.exe", "NxTools.PlatformHost.exe", "NxTools.Indexer.exe", "NxTools.BrowserHost.exe", "NxTools.Cli.exe", "LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY_NOTICES.md", "browser_extension/nxtools-interceptor/manifest.json" })
                File.WriteAllText(Path.Combine(directory, name.Replace('/', Path.DirectorySeparatorChar)), "synthetic " + sequence + " " + name);
            foreach (var name in TrustedReleaseManifestTests.ArchiveMembers)
                File.WriteAllText(Path.Combine(directory, name), "synthetic " + sequence + " " + name);
            TrustedReleaseManifest.SignPackage(directory, key, sequence, Guid.NewGuid());
            return directory;
        }
        internal TrustedUpdateTransaction Transaction() => new(Store, PublicKey, Deployment, new SilentLog());
        public void Dispose() => Directory.Delete(root, true);
    }
    private sealed class DeploymentFixture(byte[] publicKey, string target) : TrustedUpdateTransaction.IDeployment
    {
        internal long? FailSequence { get; set; }
        internal string? FailAfterMember { get; set; }
        internal string Target => target;
        internal List<long> Sequences { get; } = [];
        internal List<Guid> Correlations { get; } = [];
        public Task DeployAsync(string verifiedPackage, Guid correlationId, CancellationToken cancellationToken)
        {
            using var release = TrustedReleaseManifest.VerifyPackage(verifiedPackage, publicKey, 0);
            Sequences.Add(release.Manifest.ReleaseSequence); Correlations.Add(correlationId);
            Directory.CreateDirectory(target);
            foreach (var file in release.Files) {
                var destination = Path.Combine(target, file.Path);
                Directory.CreateDirectory(Path.GetDirectoryName(destination)!);
                File.Copy(Path.Combine(verifiedPackage, file.Path), destination, true);
                if (FailSequence == release.Manifest.ReleaseSequence && (FailAfterMember is null || FailAfterMember == file.Path))
                    throw new IOException("synthetic mutation-then-throw failure");
            }
            return Task.CompletedTask;
        }
    }
    private sealed class SilentLog : IPlatformLog
    {
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) { }
    }
    private static async Task Reject(Func<Task> action)
    {
        try { await action(); }
        catch (Exception error) when (error is IOException or InvalidDataException or CryptographicException or OperationCanceledException or System.ComponentModel.Win32Exception) { return; }
        throw new InvalidOperationException("Expected rejected update/frame.");
    }
    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
