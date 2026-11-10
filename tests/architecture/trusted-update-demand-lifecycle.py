"""Source-only updater demand lifecycle gate; no SCM or installed service operations."""
from pathlib import Path

root = Path(__file__).resolve().parents[2]
broker = (root / 'csharp/NxTools.PlatformHost/Windows/TrustedUpdateBroker.cs').read_text(encoding='utf-8')
maintenance = (root / 'csharp/NxTools.PlatformHost/Windows/TrustedBootstrapMaintenance.cs').read_text(encoding='utf-8')
windows = (root / 'csharp/NxTools.PlatformHost/Windows/TrustedBootstrapMaintenanceWindows.cs').read_text(encoding='utf-8')
registration = broker.split('private static void RegisterEnrollment', 1)[1].split('private static void ValidateEnrollment', 1)[0]
request = broker.split('internal static async Task<WindowsCommandResult> RequestAsync', 1)[1].split('internal static void EnrollIfConfigured', 1)[0]
serve = broker.split('internal static async Task<Guid?> ServeAsync', 1)[1].split('private sealed class NativeDeployment', 1)[0]
assert '"start=", "demand"' in registration and '"start=", "auto"' not in registration
assert 'RunSc(["start", ServiceName])' not in registration
assert request.index('ValidateEnrollment();') < request.index('VerifyPackage(') < request.index('StartIfStopped();') < request.index('ConnectAsync(')
assert 'ServiceControllerStatus.Stopped' in registration and 'ServiceControllerStatus.StartPending' in registration
assert 'handshake.CancelAfter(' in serve and 'readDeadline.CancelAfter(' in serve
assert 'while (' not in serve and 'continue;' not in serve
assert 'transaction.ApplyVerifiedAsync(' in serve and 'request.CorrelationId, CancellationToken.None)' in serve
assert 'Stop();' in broker.split('private sealed class BrokerService', 1)[1].split('internal static async Task<Guid?> ServeAsync', 1)[0]
assert 'Environment.Exit(1)' not in broker
assert 'service.Start(); service.ConfirmReady()' in maintenance
assert 'TrustedUpdateBroker.ConfigureDemandStart();' in windows
assert 'service.WaitForStatus(ServiceControllerStatus.Stopped' in windows
print('PASS demand registration, verified demand start, one-shot handshake, settlement, maintenance readiness (source-only)')
