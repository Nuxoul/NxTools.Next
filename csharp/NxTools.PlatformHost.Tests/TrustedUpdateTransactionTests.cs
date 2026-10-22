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
            Deployment = new DeploymentFixture(PublicKey);
        }
        internal string Package(long sequence)
        {
            var directory = Path.Combine(root, "package-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(Path.Combine(directory, "browser_extension", "nxtools-interceptor"));
            foreach (var name in new[] { "NxTools.exe", "NxTools.PlatformHost.exe", "NxTools.Indexer.exe", "NxTools.BrowserHost.exe", "NxTools.Cli.exe", "LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY_NOTICES.md", "browser_extension/nxtools-interceptor/manifest.json" })
                File.WriteAllText(Path.Combine(directory, name.Replace('/', Path.DirectorySeparatorChar)), "synthetic " + sequence + " " + name);
            TrustedReleaseManifest.SignPackage(directory, key, sequence, Guid.NewGuid());
            return directory;
        }
        internal TrustedUpdateTransaction Transaction() => new(Store, PublicKey, Deployment, new SilentLog());
        public void Dispose() => Directory.Delete(root, true);
    }
    private sealed class DeploymentFixture(byte[] publicKey) : TrustedUpdateTransaction.IDeployment
    {
        internal long? FailSequence { get; set; }
        internal List<long> Sequences { get; } = [];
        internal List<Guid> Correlations { get; } = [];
        public Task DeployAsync(string verifiedPackage, Guid correlationId, CancellationToken cancellationToken)
        {
            using var release = TrustedReleaseManifest.VerifyPackage(verifiedPackage, publicKey, 0);
            Sequences.Add(release.Manifest.ReleaseSequence); Correlations.Add(correlationId);
            if (FailSequence == release.Manifest.ReleaseSequence) throw new IOException("synthetic mutation failure");
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
