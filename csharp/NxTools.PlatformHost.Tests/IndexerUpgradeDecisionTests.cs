using System.ComponentModel;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class IndexerUpgradeDecisionTests
{
    public static Task ProtectedTupleUpgradeDecisions()
    {
        var current = Tuple('A', 'B', 'C');
        Assert(Decide(false, false, null, current) == IndexerServiceInstaller.ServiceDeploymentDecision.Install,
            "absent service requests initial explicit UAC");
        Assert(Decide(true, true, current, current) == IndexerServiceInstaller.ServiceDeploymentDecision.Reuse,
            "healthy current protected tuple is reused without UAC");
        foreach (var old in new[] { Tuple('D', 'B', 'C'), Tuple('A', 'D', 'C'), Tuple('A', 'B', 'D'), Tuple('D', 'E', 'F') })
            Assert(Decide(true, true, old, current) == IndexerServiceInstaller.ServiceDeploymentDecision.Upgrade,
                "every mismatching Desktop/PlatformHost/Indexer identity requires explicit upgrade UAC");
        Assert(Decide(true, false, current, current) == IndexerServiceInstaller.ServiceDeploymentDecision.RepairRequired,
            "matching hashes never override unhealthy registration/protected integrity");
        Assert(Decide(true, false, Tuple('D', 'E', 'F'), current) == IndexerServiceInstaller.ServiceDeploymentDecision.RepairRequired,
            "tampered/unreadable tuple cannot authorize upgrade or admission");
        Assert(Decide(true, true, null, current) == IndexerServiceInstaller.ServiceDeploymentDecision.RepairRequired,
            "missing identity fails closed");
        Assert(Decide(true, true, current with { Desktop = "not-a-hash" }, current) == IndexerServiceInstaller.ServiceDeploymentDecision.RepairRequired,
            "invalid approval fails closed");
        Assert(Decide(true, true, current with { PlatformHost = current.PlatformHost.ToLowerInvariant() }, current)
            == IndexerServiceInstaller.ServiceDeploymentDecision.Reuse, "case-insensitive SHA256 equality");
        return Task.CompletedTask;
    }

    public static Task ProtectedTupleHashChangeDetection()
    {
        var root = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath();
        var directory = Path.Combine(root, $"nxtools-tuple-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        try
        {
            var desktop = Path.Combine(directory, "desktop.fixture");
            var platform = Path.Combine(directory, "platform.fixture");
            var indexer = Path.Combine(directory, "indexer.fixture");
            File.WriteAllText(desktop, "synthetic desktop v1");
            File.WriteAllText(platform, "synthetic platform v1");
            File.WriteAllText(indexer, "synthetic indexer v1");
            IndexerServiceInstaller.ReleaseTuple Read() => new(
                IndexerServiceInstaller.ComputeBinaryHash(desktop),
                IndexerServiceInstaller.ComputeBinaryHash(platform),
                IndexerServiceInstaller.ComputeBinaryHash(indexer));
            var approved = Read();
            Assert(Decide(true, true, approved, Read()) == IndexerServiceInstaller.ServiceDeploymentDecision.Reuse, "same binary tuple");
            File.WriteAllText(desktop, "synthetic desktop v2");
            Assert(Decide(true, true, approved, Read()) == IndexerServiceInstaller.ServiceDeploymentDecision.Upgrade,
                "healthy v1 cannot silently admit a new v2 client");
            Assert(Decide(true, false, approved, Read()) == IndexerServiceInstaller.ServiceDeploymentDecision.RepairRequired,
                "approval integrity is independent of source version/hash");
        }
        finally { Directory.Delete(directory, true); }
        return Task.CompletedTask;
    }

    public static async Task ProtectedTupleAuthorizationLifecycle()
    {
        var argumentCorrelation = Guid.NewGuid();
        Assert(IndexerServiceInstaller.TryParseInstallArguments(["--install-indexer-service", "synthetic", argumentCorrelation.ToString("D")],
            out var source, out var parsedCorrelation) && source == "synthetic" && parsedCorrelation == argumentCorrelation,
            "elevated installer parses correlation before IPC startup");
        Assert(IndexerServiceInstaller.TryParseInstallArguments(["--install-indexer-service", "synthetic"], out _, out parsedCorrelation)
            && parsedCorrelation is null, "legacy formal installer invocation remains supported");
        Assert(!IndexerServiceInstaller.TryParseInstallArguments(["--install-indexer-service", "synthetic", "invalid"], out _, out _),
            "malformed correlation rejected");
        var current = Tuple('A', 'B', 'C');
        var correlation = Guid.NewGuid();
        var log = new LogFixture();
        var reuse = new DeploymentFixture(new(true, true, current, current));
        var result = await IndexerServiceInstaller.EnsureCoreAsync(reuse, log, correlation, CancellationToken.None);
        Assert(result.Success && reuse.Starts == 1 && reuse.Installs == 0, "matching healthy tuple starts without elevation");
        var upgrade = new DeploymentFixture(new(true, true, Tuple('D', 'E', 'F'), current));
        var pending = IndexerServiceInstaller.EnsureCoreAsync(upgrade, log, correlation, CancellationToken.None);
        Assert(!pending.IsCompleted && upgrade.Installs == 1 && upgrade.Starts == 0, "upgrade stays pending for explicit authorization");
        Assert(upgrade.Correlation == correlation, "elevated boundary carries caller correlation");
        upgrade.Completion.SetException(new Win32Exception(1223));
        result = await pending;
        Assert(!result.Success && result.Data == "authorization_cancelled" && result.NativeErrorCode == 1223,
            "declined UAC cannot reuse old tuple or report success");
        upgrade = new DeploymentFixture(new(true, true, Tuple('D', 'E', 'F'), current));
        pending = IndexerServiceInstaller.EnsureCoreAsync(upgrade, log, correlation, CancellationToken.None);
        upgrade.Completion.SetResult(new WindowsCommandResult(true, Data: "installed"));
        Assert((await pending).Data == "upgraded", "retry requires completed verified atomic installer result");
        var damaged = new DeploymentFixture(new(true, false, current, current));
        result = await IndexerServiceInstaller.EnsureCoreAsync(damaged, log, correlation, CancellationToken.None);
        Assert(!result.Success && result.Data == "repair_required" && damaged.Starts == 0 && damaged.Installs == 0,
            "unhealthy state neither starts nor elevates");
        var absent = new DeploymentFixture(new(false, false, null, current));
        pending = IndexerServiceInstaller.EnsureCoreAsync(absent, log, correlation, CancellationToken.None);
        absent.Completion.SetResult(new WindowsCommandResult(false, Data: "tuple_verification_failed"));
        Assert(!(await pending).Success, "installer acceptance/exit alone does not certify tuple verification");
        using var cancelled = new CancellationTokenSource();
        cancelled.Cancel();
        var notStarted = new DeploymentFixture(new(true, true, current, current));
        Assert((await IndexerServiceInstaller.EnsureCoreAsync(notStarted, log, correlation, cancelled.Token)).Data == "cancelled"
            && notStarted.Reads == 0, "cancel before dispatch leaves state untouched");
        Assert(log.Correlations.All(value => value == correlation) && log.Stages.Contains("upgrade_authorization_requested")
            && log.Stages.Contains("authorization_cancelled"), "bounded correlated decision and terminal transitions");
        log.ThrowOnWrite = true;
        Assert((await IndexerServiceInstaller.EnsureCoreAsync(reuse, log, correlation, CancellationToken.None)).Success,
            "diagnostic failure cannot change service result");
    }

    private sealed class DeploymentFixture(IndexerServiceInstaller.DeploymentState state) : IndexerServiceInstaller.IDeploymentPlatform
    {
        public int Reads { get; private set; }
        public int Starts { get; private set; }
        public int Installs { get; private set; }
        public Guid Correlation { get; private set; }
        public TaskCompletionSource<WindowsCommandResult> Completion { get; } = new(TaskCreationOptions.RunContinuationsAsynchronously);
        public IndexerServiceInstaller.DeploymentState ReadState() { Reads++; return state; }
        public bool StartService() { Starts++; return true; }
        public Task<WindowsCommandResult> InstallAsync(Guid correlationId, CancellationToken cancellationToken)
        { Installs++; Correlation = correlationId; return Completion.Task; }
    }
    private sealed class LogFixture : IPlatformLog
    {
        public List<Guid?> Correlations { get; } = [];
        public List<string> Stages { get; } = [];
        public bool ThrowOnWrite { get; set; }
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (ThrowOnWrite) throw new InvalidOperationException("synthetic log failure");
            Correlations.Add(correlationId);
            Assert(fields is not null && fields.ContainsKey("elapsed_ms") && fields.ContainsKey("native_error_code"), "elapsed/native diagnostic contract");
            Assert(fields!.Keys.All(key => key is "stage" or "success" or "native_error_code" or "elapsed_ms"), "diagnostics contain no payload secrets");
            Stages.Add((string)fields["stage"]!);
        }
    }

    private static IndexerServiceInstaller.ReleaseTuple Tuple(char desktop, char platform, char indexer) =>
        new(new string(desktop, 64), new string(platform, 64), new string(indexer, 64));
    private static IndexerServiceInstaller.ServiceDeploymentDecision Decide(bool exists, bool healthy,
        IndexerServiceInstaller.ReleaseTuple? approved, IndexerServiceInstaller.ReleaseTuple current) =>
        IndexerServiceInstaller.DecideDeployment(exists, healthy, approved, current);
    private static void Assert(bool condition, string message)
    { if (!condition) throw new InvalidOperationException(message); }
}
