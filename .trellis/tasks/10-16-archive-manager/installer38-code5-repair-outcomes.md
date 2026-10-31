# Release38 installer process identity — real upgrade completed

2026-10-30. The normal formal Release38 installer completed an explicitly authorized real upgrade. This is installed preparation/update/file verification, not merely a successful build or synthetic fixture.

## Delivered installer and actual result

- Normal installer: `artifacts/installer/NxTools.Next-setup-0.7.0.exe`.
- Revision: `cim-null-native-process-identity1`;26,724,105bytes.
- SHA256:`E3ACD0E9CD773A6BC3E1CA93E14EF26F44BCFA140ECF2093132E9C78416A51CE`.
- Same signed release38 / ID`53692ed3-7f88-4678-88d8-3275441d4a71`. All77 signed payload members and both sidecars unchanged; original RSA3072-PSS/SHA256 verified. No new key, release sequence or Cargo build.
- Actual installation correlation:`20261030112037-is-PYH4AB1T9T-tmp`; installer exit0. 32-bit helper ran elevated, completed process_close/file_preparation with native0; file installation, trusted_update and upgrade_files.finalized all completed.
- Existing Next AppId install location:`C:\Users\Administrator\AppData\Local\NxTools.Next-clipboard-fix-verification`. This is the existing registered directory, not a new test package or a newly chosen installation directory. The normal installer was used with explicit user authorization. Other legacy NxTools AppIds were not changed.
- Installed manifest/signature match the formal package; all77 installed members match signed size/hash. All13 deployed files in `C:\ProgramData\NxTools\NxTools.Next\Service` match the package. Indexer and updater Running after update; independent updater PID5164 preserved. Original installation journal and pending record retired normally. No manual recovery deletion or automatic product launch.

Persistent evidence: `.diagnostics/packaging/installer38-cim-identity-20261030-111028/`.
Actual successful installation evidence: `live-authorized-2/live-setup.log`, `live-install-result.json`, `installed-verification.json`, and `user-diagnostic/` below that directory.

## Failure and bounded repair

Original user correlation:`20261030104807-is-KYQR3HU6JV-tmp`, process_close/native5/scriptline199 before file replacement. The original helper treated a missing CIM ExecutablePath as access denied before checking whether the process had already exited. The retained actual log omitted PID/elevation; it cannot identify the historical offending process.

The original exit-race test covered only stale candidates whose image path remained available. A retained real exited child plus CIM-null discovery reproduces the same line/error on x86/x64. Query-only native image/exit discovery now precedes exact target membership; unknown live failures still reject. Original same-handle Stop, independently approved SCM updater exclusion and recovery/file transactions remain owners. Diagnostics now retain PID/fixed process name/native API/elevation without paths or raw errors.

Read-only ordinary32/64 and explicitly authorized elevated32 probes verified the existing updater identity. No SeDebugPrivilege enabling, blanket error suppression, process-name kill or updater termination was added. [Reference chain](../../../docs/reference-code/archive-shell.md#installer38-cim-null-process-identity-boundary-2026-10-30) and [executable contract](../../spec/platformhost/installer-preparation.md) record the boundary.

## Focused verification

- Original expanded process regression goes red with delivered helper on both architectures; repaired exact compiler-input helper passes41checks per architecture, including retained exited/CIM-null/live/foreign/denied/gone/query/wait and query-only access controls.
- Existing identity22, diagnostic7 and upgrade18 regressions pass. Diagnostics assert actual PID/API/elevation and privacy.
- Original real Inno PrepareToInstall differential: delivered helper fails with the CIM-null exited child; repaired helper completes the same disposable installation. Six related Inno owned/exited/null/protected-success/cancel/failure scenarios pass; expected negative exits do not mean installation success.
- Packaging permanently includes `exited-cim-null` before normal installer compilation. Existing tests were extended, and the obsolete CIM-null-equals-denied assumption replaced with native-identity/non-target preservation assertions. No duplicate test framework or unrelated thousands-test rerun.
- Exact compiler-input helper SHA256:`16062B20D23495D67F4BD1905E3E3DE7CF10251333DFF9ACB451BF6BC0C189BD`.

## Recurrence prevention

The missed boundary was discovery before native Stop, not another codec feature. The old test's valid-image premise excluded the failing input. Relevant process/installer regressions remain necessary even when older than the current feature; outdated premises must be replaced, not accepted as product requirements. A successful fixture or huge unrelated test total is not a real installed result.

`PACKAGING.md` and the PlatformHost spec index link this record/contract so subsequent packaging starts from known installation failures. Keep actual user logs and prior failed installers immutable. Diagnose each new correlation by phase/native API/PID/elevation before selecting a repair; no repeated installation without renewed authorization after cancellation.

## Limitations and historical records

The original PID was not retained, so the old user's specific process state is not provable retrospectively. The current real successful upgrade is independently observed. Earlier31 code5 updater and32 native31 records remain historical evidence, not proof that every failure shared their cause. An earlier authorized live attempt returned started0/error1223 before helper execution;17installed root files and service PIDs were unchanged. Only a renewed explicit user authorization permitted the successful attempt. Physical product UI/Explorer interactions and indexer search/idle-CPU acceptance were not run by this installer-only repair. Windows Authenticode remains NotSigned, distinct from the verified internal signature.
