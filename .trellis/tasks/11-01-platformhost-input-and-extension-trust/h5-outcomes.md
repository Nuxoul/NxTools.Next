# H5: Detached process and indexer commands

## Implementation
PlatformCommandSequenceDispatcher includes launch_process and ensure_indexer_service in its original detached collection. Existing Task.Run admission handles native calls that block before returning Task; original failure observation and task tracking remain. WindowsPlatformAdapter.LaunchProcess disposes the Process.Start result while preserving its success/error result semantics.

## Concurrency and result ownership
IndexerServiceInstaller.EnsureAsync holds the named Local\\NxToolsIndexerServiceInstallV1 semaphore across EnsureCoreAsync. AcquireServiceInstallGateAsync uses the existing30000ms operation deadline and cancellation. TrustedUpdateBroker.RequestAsync has a linked5minute request budget (pipe connect10000ms). No installer or service behavior is changed.

QuickLaunchApplication.ensure_indexer_service begins indexer_service_bootstrap with a generated command ID before send_command_with_id and suppresses duplicate pending requests. Its CommandResult handler matches that ID; a later independent command need not wait for the service result. Existing indexer connection/status lifecycle remains.

The detached operation enters the dispatcher's task set. DrainAsync observes actual completion up to its original default2second budget, swallowing failures only for teardown observation. Session shutdown calls BeginDrain after cancelling the session and stopping existing owners; it starts bounded drain observation rather than waiting indefinitely for UAC. Cancellation or drain timeout does not prove an uncooperative native call was terminated.

## Verification
`dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore`: passed,0errors.
`dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list 'extension runtime fixture detached reader responsiveness'`: one reviewed Fixture,run0.
The same command without --list was behavior-red with run1/pass0/fail1 (launch_process blocked the reader), then green run1/pass1/skip0/fail0. The single fixture loops picker/probe/launch/service names, holds the delegate, observes the following visibility command, asserts Drain is pending until release and then verifies actual completion. No process/service/UAC operation is invoked.
Logs: session scratch/input-h5-{build-red,build-green,list,red,green}.log.

## Limits
These are isolated scheduler/delegate checks and source-traced install/result ordering, not real UAC, service installation or product launch acceptance.
