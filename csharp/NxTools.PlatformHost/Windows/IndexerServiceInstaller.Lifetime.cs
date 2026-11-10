using System.ComponentModel;
using System.Diagnostics;
using System.Security.Principal;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows;

internal static partial class IndexerServiceInstaller
{
    internal const string ServiceStartMode = "demand";

    internal static string[] RegistrationArguments(bool exists, string protectedIndexerPath) =>
        [RegistrationVerb(exists), ServiceName, "binPath=", $"\"{protectedIndexerPath}\" --service",
            "start=", ServiceStartMode, "obj=", "LocalSystem", "DisplayName=", "NxTools File Indexer"];

    private static async Task<WindowsCommandResult> ConfigureLifetimeWithAuthorizationAsync(
        IPlatformLog log, Guid correlationId, CancellationToken cancellationToken)
    {
        var hostPath = Environment.ProcessPath ?? throw new InvalidOperationException("PlatformHost image unavailable.");
        var start = new ProcessStartInfo(hostPath) { Verb = "runas", UseShellExecute = true, WindowStyle = ProcessWindowStyle.Hidden };
        start.ArgumentList.Add("--configure-indexer-lifetime");
        start.ArgumentList.Add(correlationId.ToString("D"));
        using var operation = Process.Start(start) ?? throw new InvalidOperationException("Service configuration helper unavailable.");
        using var deadline = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        deadline.CancelAfter(OperationTimeoutMs);
        await operation.WaitForExitAsync(deadline.Token);
        if (operation.ExitCode != 0)
            return new(false, "索引服务的按需启动设置未完成，请重试或使用正式安装程序修复。",
                Data: "lifetime_configuration_failed", NativeErrorCode: operation.ExitCode);
        var platform = new NativeDeploymentPlatform(log, correlationId);
        var state = platform.ReadState();
        return DecideDeployment(state.Exists, state.Healthy, state.Approved, state.Current) == ServiceDeploymentDecision.Reuse
                && !platform.NeedsLifetimeMigration()
            ? new(true, Data: "lifetime_configured")
            : new(false, "服务启动类型或受保护程序组未通过验证。", Data: "lifetime_verification_failed");
    }

    internal static int ConfigureLifetimeElevated(IPlatformLog log, Guid correlationId)
    {
        var started = Stopwatch.GetTimestamp();
        WriteDeploymentLog(log, correlationId, "lifetime_configuration_started", started);
        try
        {
            if (!new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator))
                throw new UnauthorizedAccessException("Service configuration requires administrator authorization.");
            using var deployment = TrustedDeploymentGate.Acquire();
            var platform = new NativeDeploymentPlatform(log, correlationId);
            var state = platform.ReadState();
            if (DecideDeployment(state.Exists, state.Healthy, state.Approved, state.Current) != ServiceDeploymentDecision.Reuse)
                throw new InvalidDataException("Current protected release required for lifetime configuration.");
            RunSc(["config", ServiceName, "start=", ServiceStartMode], [0]);
            if (platform.NeedsLifetimeMigration()) throw new InvalidDataException("Service start type readback failed.");
            WriteDeploymentLog(log, correlationId, "lifetime_configuration_completed", started, new(true));
            return 0;
        }
        catch (Exception failure)
        {
            WriteDeploymentLog(log, correlationId, "lifetime_configuration_failed", started,
                new(false, NativeErrorCode: failure is Win32Exception native ? native.NativeErrorCode : failure.HResult));
            return 1;
        }
    }
}
