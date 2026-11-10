# Installer preparation — stopped Demand updater

## Actual failure

Release61 attempt `20261109110405-is-9R3J9TM5Z7-tmp` completed bootstrap detection with
`compatible`/exit0. The elevated32-bit preparation helper then failed at process_close,
InvalidDataException/scriptline187, attributed to NxTools.PlatformHost.exe PID39164.
No native terminate API is recorded at this boundary. The PID was absent at later observation;
its former role is not established from the log.

The original read-only Get-UpgradeUpdaterProcessId reproduces the same exception and line on the
report machine. The fixed ImagePath, LocalSystem account, publisher and bootstrap hash match,
while SCM reports Demand/Stopped/PID0. The resolver incorrectly required Running/positive PID
before allowing discovery of any CIM-null Host candidate to continue.

## Correct boundary

TrustedUpdateBroker registers Demand and stops after one operation. A correctly identified
Stopped/PID0 updater therefore has no process to exclude. Resolve-UpgradeUpdaterProcessId now
performs its original path/account/key/hash/status-shape/type checks first, then returns0 for
that normal stopped state. Running exclusion still requires the original positive SCM PID.

Stop-UpgradeTargets continues the original candidate QueryImage/exact-target/independent Stop
handle boundary. It does not skip every Host, start/stop a service, suppress native errors or
change recovery authority. Stopped/nonzero PID, foreign registration/hash/type, malformed status
and unknown live query failures still refuse. Existing diagnostics retain correlation, stage,
exception, native API/code and candidate PID without adding paths or raw exception messages.

The original identity regression covers both resolver and CIM-null discovery: owned Host reaches
exact native stop, foreign Host stays untouched, native exited proof remains harmless and a live
query error retains its code/PID. The original x86/x64 package preflight now runs this regression
alongside the same-handle process-exit gate before Inno compilation.

## Verification

- Original read-only production call and focused regression reproduce the concrete rejection.
- Identity regression30checks passes separately on x86 and x64; original exit-race47checks also
  passes separately on both architectures.
- Original isolated upgrade fixture18/18 and preparation diagnostics8scenarios pass.
- Corrected production read-only replay returns excluded_pid0 while the actual updater remains
  stopped/PID0. No service, registry, installed-file or external-process mutation is performed.

Evidence: `D:\.pi-desktop\data\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\installer-stopped-updater/`.
Reference chain: [archive Shell](../../../docs/reference-code/archive-shell.md#installer-process-exclusion-with-a-stopped-demand-updater--2026-11-09).

## Limits

The user-reported preparation failure and the read-only reproduced prerequisite are distinct from
complete installed-upgrade acceptance. Actual UAC/preparation, subsequent trusted update,
installation/finalization and signed-file readback still require a real installation. No manual
service start, recovery cleanup, trust replacement or automatic installation retry is added.

## Release62

The formal [Release62 installer](release62-package-outcomes.md) includes this corrected helper.
The frozen8,605-file source and independently copied cache produce matching signed artifacts.
Both30-check identity and47-check exit regressions pass again on x86/x64 during packaging,
followed by16Inno and5BrowserHost scenarios. Actual report-machine x86/x64 read-only replay
returns0 with the service still stopped. The production helper SHA256 is
`61BF530E71493B4715C5174FBF9052988A7B73BBCCF87B85F6178B2353688EA1`.
Evidence is retained under `.diagnostics/packaging/20261109-111508-d6303d3a/`.
Real installation and complete installed-file readback remain unexecuted.
