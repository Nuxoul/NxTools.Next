# Installer preparation process identity

## 1. Scope / Trigger
Changes to `repo:installer/close-owned-processes.ps1`, Inno PrepareToInstall or installer process regression gates. Inspect the actual correlation's launch/preparation/setup logs and [reference evidence](../../../docs/reference-code/archive-shell.md#installer38-cim-null-process-identity-boundary-2026-10-30) before assuming an access error has the same cause as an earlier installation.

## 2. Signatures
- `Stop-UpgradeTargets($Targets, $ProtectedRoot)` admits only exact paths selected by `Get-UpgradeTargets`.
- `NxToolsUpgradeProcess.QueryImage(int id) -> string|null` uses SYNCHRONIZE|PROCESS_QUERY_LIMITED_INFORMATION (`0x00101000`), never TERMINATE.
- `NxToolsUpgradeProcess.Stop(int id, string expected) -> bool` opens/revalidates its own process handle before terminating an exact target.
- Inno `PrepareToInstall` waits for the helper; only exit0 allows file replacement. ShellExec started=false/error1223 is cancelled elevation, not helper failure5.

## 3. Contracts
CIM-null ExecutablePath does not establish access denial or process ownership. First exclude only the independently approved updater through original fixed SCM/path/account/hash/PID evidence. Otherwise query the native object: gone/error87 or same-handle signalled exit returns null; live image success returns its canonical path; live open/query/wait failures retain their native error. Recheck target membership and let original Stop revalidate its acquired handle. No name-wide termination, blanket5/31 suppression, service stop fallback or recovery-record deletion.
After native query/terminate error5 or31, an object may be terminating but not yet signalled. Reuse the original5000ms exit bound on the SAME acquired handle; only a signalled result permits treating it as exited. Timeout preserves the captured original native error; wait failure remains a native failure. No repeated query, replacement handle, new termination or blanket error suppression. Zero-timeout observation alone is insufficient for this failure branch.

Diagnostics retain original correlation, lifecycle stage, elapsed/native code, process_bits and actual token `elevated`. Identity failures around the FULL candidate query/target/Stop sequence add positive process_id, fixed allowlisted process_name and native_api (`open`, `query_image`, `terminate`, `wait`); no image paths, command lines, raw exception messages or payload secrets. Sink failure never changes the operation.

## 4. Validation & Error Matrix
| Observation | Required behavior |
| --- | --- |
| CIM-null + retained exited object | Prove native exit; no termination or fabricated access error |
| CIM-null + readable live native image | Exact target selection; unrelated process remains untouched |
| Unknown live open/query failure | Preserve real native failure and process identity diagnostics |
| Query/terminate5/31, same handle signals within5000ms | Treat confirmed exit as absent; never retry termination |
| Approved independent updater | Exclude without opening it for termination |
| UAC cancelled /1223 | No helper/file replacement; preserve launch result, no automatic retry |
| Helper completed /0 | Original recovery, file transaction and trusted update remain authoritative |

## 5. Good/Base/Bad Cases
Good: native image resolves a CIM-null candidate and original exact-path Stop owns any termination. Base: readable CIM image uses unchanged Stop. Bad: treating all CIM-null entries as denied, treating error5 as success, or claiming fixture success is actual installation success.

## 6. Tests Required
- Extend `repo:tests/architecture/installer-process-exit-race.ps1` rather than creating duplicate process harnesses: actual owned live/exited children with both present/null CIM image; query-only access mask; gone/denied/query/wait controls; x86 and x64.
- `repo:tests/architecture/installer-process-identity.ps1`: updater proof and unknown live failure remain strict.
- `repo:tests/architecture/installer-preparation-diagnostics.ps1`: exact PID/API/elevation plus privacy and sink failure.
- `repo:tests/architecture/installer-runtime.ps1 -Scenario exited-cim-null`: original Inno callback and exact compiler-input helper, no real system mutation. `repo:scripts/package.ps1` invokes this before compilation.
- Separately authorized real installation: actual helper exit0, full installation/trusted update/finalization, installed signed members and protected deployment hashes. Never infer this from unrelated Rust/C# totals. Authorization cancellation is not success or permission to retry.
Real upgrade acceptance must also cover a RUNNING installed Desktop/protected restart target. Record the verified launch path, retained source handle/protected descendant PID and actual pre-install running state; an idle installation cannot establish this result. The medium-user readiness probe must not equate missing CIM image with absence. Keep readiness observation separate from the installer's exact native termination authority. Latest real evidence: [running upgrade41](../../tasks/10-16-archive-manager/installer41-running-upgrade-outcomes.md).

## 7. Wrong vs Correct
Wrong: `if (!$entry.ExecutablePath) { throw Win32Exception(5) }` before checking the native object.
Correct: SCM-proved updater exclusion -> query-only handle/exit/image -> exact target membership -> original same-handle Stop -> original recover/replace/finalize. Only authorized real-install logs and installed-file readback establish installed acceptance.

## Explicit missing-root repair (2026-11-06)

The separately approved installer repair covers an existing updater registration whose entire
protected `Updater` directory is genuinely absent. `bootstrap-migration.ps1` distinguishes no
registration (exit0), approved legacy maintenance (exit10), approved missing root (exit11), and
failure (exit1). Path/account/publisher/registered hash are checked before reporting repair.
Access errors, existing directories/files, foreign registration and surviving recovery records
must not become absence. Detection records bounded stage/reason/exception/native facts and the
same installer correlation; Inno persists `bootstrap-detection.jsonl` and actual launch/exit even
when preparation exits early.

Only explicit user confirmation and UAC permit the extracted same-release Host command
`--repair-missing-trusted-bootstrap <GUID>`. Its original package/caller verification precedes
native effects. `TrustedMissingBootstrapRepair` rechecks the fixed registration, stops only that
approved service, acquires the existing deployment gate, and rechecks parent/root/registration.
`TrustedUpdateBroker.EnrollMissingRoot` reuses the original enrollment/provisioning implementation;
ordinary `EnrollIfConfigured` and bootstrap maintenance retain their original refusal semantics.
Completed enrollment, actual bootstrap hash and protocol/version marker must agree before success.
The marker lets a same-release installer retry recognize compatible completed repair.

Do not delete a service or recovery tree to implement repair. A surviving incomplete root remains
an inspectable recovery condition, not authorization to reset trust or erase its records. No
unknown path, process-name kill or key replacement is permitted. This source contract is not
installed repair acceptance; real installation requires separately authorized actual logs/readback.

Regression owners: original `installer-bootstrap-migration.ps1` now runs the actual detection
entry with injected registration and owned roots on x86/x64; `installer-runtime.ps1` covers repair
success/cancel/failure/tamper and detection refusal at the actual Inno callback; C# missing-bootstrap,
enrollment and maintenance fixtures use signed temporary packages and the real enrollment journal.

Repair enrollment stages its initial signed snapshot with `State.DeploymentPending=true`.
Only the original LocalSystem broker deployment and complete protected-file verification may
clear that durable fact. The repair caller releases its parent deployment gate before requesting
this initial deployment over the original pipe, and does not return success until that request
completes. Deployment failure preserves the pending snapshot for ordinary same-key retry; a
same-release request must not report already-current while its initial deployment is pending.
The false/default field is omitted, preserving ordinary stored-state serialization.

### Missing product ancestors

The2026-11-06actual repair rejection at correlation20261106233632 reports root_absence/native2
before service stop. Both NxTools and NxTools.Next ancestors were absent, not just Updater.
Explicit repair therefore uses `IndexerServiceInstaller.ValidateMissingProductParents` before
mutation and again before provisioning. It opens the existing CommonApplicationData anchor with
the original stable native directory lease, then validates each existing fixed vendor/product
ancestor and its protected ACL. Only file/directory-not-found at these fixed product nodes may
end the precreation check; access errors, files, reparse objects and bad existing ACLs refuse.
The base data root itself must exist. Creation and ACL hardening remain in the original
`HardenProductChildDirectory` after signed repair admission; ordinary validation is unchanged.

Missing-bootstrap fixtures must use this actual native parent validator with an initially absent
vendor/product tree, not a no-op parent callback. A read-only probe of the actual production
anchor is separate from service recovery or installed acceptance. Diagnostic parent_validation
and root_absence stages distinguish ancestor failure from the updater-root decision.

## Stopped Demand updater during process discovery

The fixed updater registration and bootstrap hash/type validation also applies when its Demand
service is stopped. Validated SCM Stopped/PID0 returns no excluded process; discovery then uses
the original candidate QueryImage and exact-target Stop boundary. It must not require starting
the updater just to close ordinary owned processes. Running exclusion retains the validated
positive own-process SCM PID. Stopped/nonzero PID, invalid registration/hash/type and malformed
status still refuse, and live native candidate failures retain their actual code/PID.

`repo:tests/architecture/installer-process-identity.ps1` covers both original resolver and
CIM-null Host discovery with the real resolver: owned, foreign, exited and query-denied outcomes.
The report-machine read-only Get-UpgradeUpdaterProcessId replay is distinct from actual install,
service startup, termination or complete installed-file readback acceptance.
