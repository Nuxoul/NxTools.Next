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
