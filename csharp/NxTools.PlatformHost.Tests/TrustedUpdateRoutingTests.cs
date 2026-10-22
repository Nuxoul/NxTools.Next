using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class TrustedUpdateRoutingTests
{
    public static async Task EnrolledUpgradeNeverCallsElevatedInstaller()
    {
        var platform = new DeploymentFixture();
        var correlation = Guid.NewGuid();
        var result = await IndexerServiceInstaller.EnsureCoreAsync(platform, new SilentLog(), correlation, CancellationToken.None);
        Assert(result.Success && result.Data == "upgraded", "signed resident update must complete");
        Assert(platform.TrustedUpdates == 1 && platform.Installs == 0,
            "an enrolled release update must use the trusted broker, never the UAC installer");
        Assert(platform.Correlation == correlation, "trusted operation preserves correlation");
    }

    public static async Task RejectedTrustedUpdateCannotFallBackToUac()
    {
        var platform = new DeploymentFixture { Outcome = new(false, "签名验证失败。", Data: "signature_rejected") };
        var result = await IndexerServiceInstaller.EnsureCoreAsync(platform, new SilentLog(), Guid.NewGuid(), CancellationToken.None);
        Assert(!result.Success && result.Data == "signature_rejected" && platform.Installs == 0,
            "authentication rejection must not become user-approved unsigned installation");
    }

    public static async Task SameReleaseMigratesTrustExactlyOnce()
    {
        var platform = new DeploymentFixture { SameRelease = true, EnrollmentNeeded = true };
        var result = await IndexerServiceInstaller.EnsureCoreAsync(platform, new SilentLog(), Guid.NewGuid(), CancellationToken.None);
        Assert(result.Success && platform.Installs == 1 && platform.TrustedUpdates == 0,
            "same-release legacy installation still needs one explicit trust enrollment");
        platform.EnrollmentNeeded = false;
        result = await IndexerServiceInstaller.EnsureCoreAsync(platform, new SilentLog(), Guid.NewGuid(), CancellationToken.None);
        Assert(result.Success && platform.Installs == 1 && platform.Starts == 1,
            "enrolled same release starts without additional authorization");
    }

    private sealed class DeploymentFixture : IndexerServiceInstaller.IDeploymentPlatform
    {
        public int Installs { get; private set; }
        public int TrustedUpdates { get; private set; }
        public Guid Correlation { get; private set; }
        public WindowsCommandResult Outcome { get; init; } = new(true, Data: "updated");
        public bool SameRelease { get; init; }
        public bool EnrollmentNeeded { get; set; }
        public int Starts { get; private set; }
        public bool NeedsTrustEnrollment() => EnrollmentNeeded;
        public IndexerServiceInstaller.DeploymentState ReadState() => new(true, true,
            new(new string('A', 64), new string('B', 64), new string('C', 64)),
            SameRelease ? new(new string('A', 64), new string('B', 64), new string('C', 64))
                : new(new string('D', 64), new string('E', 64), new string('F', 64)));
        public bool StartService() { Starts++; return true; }
        public Task<WindowsCommandResult> InstallAsync(Guid correlationId, CancellationToken cancellationToken)
        {
            Installs++;
            return Task.FromResult(new WindowsCommandResult(true, Data: "installed"));
        }
        // Real EnsureCoreAsync dispatch; no service, UAC or native mutation in this fixture.
        public bool HasTrustedUpdater() => true;
        public Task<WindowsCommandResult> UpdateTrustedAsync(Guid correlationId, CancellationToken cancellationToken)
        {
            TrustedUpdates++;
            Correlation = correlationId;
            return Task.FromResult(Outcome);
        }
    }
    private sealed class SilentLog : IPlatformLog
    {
        public void Write(string level, string eventName, Guid? correlationId = null,
            IReadOnlyDictionary<string, object?>? fields = null) { }
    }
    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
