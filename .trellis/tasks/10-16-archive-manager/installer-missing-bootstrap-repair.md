# Installer missing updater directory repair

2026-11-06. User authorizes repairing the installer and producing a repair package, not modifying
the workstation service/registry or running an actual installation.

## Observed failure

The supplied Setup logs001/002 show complete existing-install preparation, normal file install,
trusted update and finalization. Log003 rejects twice at `installer.bootstrap_migration.detecting`,
without reaching the close-owned-processes helper or file replacement. It lacks child exit and
internal exception details, so that historical log alone does not establish the internal cause.

Read-only execution of the original detector on this machine independently reproduces exit1.
The fixed updater registration retains LocalSystem, matching publisher key and registered hash,
while native attributes return ERROR_PATH_NOT_FOUND3 for the entire Updater tree and its bootstrap.
The original detector requires the old executable hash and the original maintenance command
requires its existing enrollment. Ordinary broker enrollment also treats surviving registration
as already enrolled. There was no explicit installer recovery for this incomplete state.

## Repair boundary

- No registered updater: first-install detection exits0 without accessing old updater files.
- Compatible updater: existing path remains unchanged. Original maintenance remains exit10.
- Exact same-publisher/fixed-path/LocalSystem registration and genuinely missing whole root:
  detector exits11; Inno explains repair and requires confirmation followed by UAC.
- The extracted Host is pinned by the installer and verifies its full signed package and exact
  caller. The repair GUID is carried from helper through C# lifecycle diagnostics.
- The repair stops only the approved registered updater, acquires the original deployment gate,
  then rechecks registration, parent and actual absence. A dedicated explicit enrollment entrance
  reuses the original provisioning/enrollment state machine; ordinary enrollment does not change.
- Completion requires real bootstrap bytes/hash, completed enrollment and the original compatible
  protocol/version marker. Same-release installation can then detect a compatible completed repair.
- Existing roots/files/journals, foreign registration, denied reads, bad signature/caller and native
  failures refuse without deleting evidence or pretending success. A surviving partial tree is
  not authorization to clear it.
- Initial provisioning records the original signed snapshot as deployment-pending, not already
  installed. After releasing the parent gate, the repair waits on the original LocalSystem
  broker to deploy and verify that snapshot. Only actual completion clears pending state;
  failures preserve it for retry. Ordinary state serialization and signed rollback remain.

Detection now persists bounded stage/reason/exception/native metadata and Inno actual started/exit
before early rejection. Main file preparation keeps its original logging, process and rollback owners.
No paths, command lines, secret payloads or raw exception messages are added to production events.

## Verification

- Actual detector entry on both Windows PowerShell architectures plus original decisions and helper
  command dispatch:49assertions pass, including first install, missing root, compatible, foreign
  publisher, access rejection, nonempty repair GUID and native failure propagation.
- Actual production Inno callback, isolated resources and injected elevation/SCM:9scenarios pass:
  repair success/cancel/failure/tamper, detector failure, and the four original migration scenarios.
  These are not real installations or UAC/service acceptance.
- C# final Debug test-project build:0warnings/0errors. Exact reviewed LIST/RUN13fixtures pass,
  skip0/fail0:3repair,4maintenance,5original transaction and1pinned partial-enrollment selection.
  The actual transaction exercises initial deployment, old protected bytes, failed-deploy pending
  retention/retry, same-release idempotence, supersession and signature/downgrade refusal. Registry,
  SCM and deployment effects remain injected into GUID-owned signed temporary packages.
- Missing-marker regression retains one compiled behavioral failure and then passes with the
  final13case selection. Original detector diagnostic absence is retained as RED evidence.
- Original helper diagnostics8, process identity22 and upgrade-process18checks pass; no real process,
  service or installed file is mutated by these effects-reviewed fixtures. Exact counts remain in logs.
- PowerShell syntax and bootstrap-maintenance source contracts pass. Package generation and final
  artifact checks are recorded separately when complete.

Evidence: `D:\.pi-desktop\data\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\installer-bootstrap-repair/`
and the parent scratch `bootstrap-repair-red.log`/`bootstrap-repair-green.log`.

## Remaining acceptance

No real service or registry was changed and no actual repair installation was performed. User UAC
acceptance, actual restored registered service and installed signed-file readback remain necessary
before claiming the reported machine is recovered. New-user screenshots alone do not establish a
clean machine: updater registration is machine-wide and can remain across accounts or removed files.

## Repair package

[Formal Release56](release56-package-outcomes.md) contains this repair. The original package entry
and child exit0;16Inno/5BrowserHost checks and independent package/signature/identity checks pass.
No real repair installation was run. Use the new installer and explicitly accept its repair/UAC
prompt; retain the actual detector/helper/Setup logs to establish installed recovery.
