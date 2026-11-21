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
Ordinary preparation runs as the current user. Indexer service registration alone is not an elevation trigger. Explicit Bootstrap maintenance/repair/rotation retains its authorization; otherwise only actual helper access-denied exit5 uses the existing runas fallback. UAC cancellation stops before file replacement. Invoke Bootstrap migration once per preparation with both repair and rotation switches. Signature/identity, original-user, recovery, SCM and protected deployment authority remain unchanged. Source/runtime-fixture owners: `repo:tests/architecture/installer-update-lifecycle.py`, `repo:tests/architecture/installer-runtime.ps1`.

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

## Scenario: Stable Bootstrap compatibility floor

### 1. Scope / Trigger
Routine signed releases must reuse an already compatible protected Bootstrap. Increasing the
package release sequence alone must not request Bootstrap maintenance authorization.

### 2. Signatures
`scripts/package.ps1` sets `$migrationBootstrapMinimumSequence = 82` and generates
`BootstrapMigrationMinimumSequence` in the original migration include. Inno passes it to
`bootstrap-migration.ps1 -MinimumBootstrapMaintenanceSequence <long>`.
`Get-BootstrapMigrationDecision(..., ExpectedSequence, MinimumBootstrapMaintenanceSequence)`
compares the protected maintenance marker against that explicit floor when it is positive.

### 3. Contracts
Floor82 is the first compatible maintenance generation containing the historical-key recovery
anchor. It remains constant across ordinary product releases and repackaging. Raise it to the
first release providing newly required Bootstrap behavior when the compatibility requirement
changes; do not derive it from the current package sequence, binary hash or product version.
The existing marker remains bound to protocol1 and the registered actual Bootstrap hash.
No marker/schema/enrollment rewrite or real system mutation occurs during detection.
Signed application release sequences retain their original downgrade/replay protection.

### 4. Validation & Error Matrix
| Observation | Detection result |
| --- | --- |
| Package999, marker82 or86, floor82, approved Demand registration | compatible / exit0 |
| Marker81 or absent, floor82 | migration_required / exit10 |
| Compatible marker plus unfinished maintenance journal | migration_required / exit10 |
| Marker82, newly required floor83 | migration_required / exit10 |
| Bootstrap hash or marker identity mismatch | failed / exit1 |
| Approved registration under another publisher key | trust_rotation_required / exit12 |

### 5. Good/Base/Bad Cases
Good: ordinary release87 reuses compatible Bootstrap86. Base: an older Bootstrap receives
explicit maintenance once. Bad: every repackaged sequence is treated as a Bootstrap upgrade.

### 6. Tests Required
`tests/architecture/installer-bootstrap-migration.ps1` exercises the actual detection entry on
x86/x64 with package999 and marker82, then old/missing/journal/tampered/rotation controls.
The original Inno runtime fixture must prove maintenance success, cancellation and failure
before application file replacement. Fixture success is not actual installed acceptance.

### 7. Wrong vs Correct
Wrong: compare the maintenance sequence to every new application release sequence.
Correct: compare it to the explicitly required Bootstrap compatibility floor while retaining
original publisher, registration, hash, marker, recovery and signed-package verification.

## Required runtime admission

Normal and maintenance Setup call `CheckRuntimeDependencies` from
`repo:installer/runtime-dependencies.iss` in `PrepareToInstall`, before bootstrap recovery,
process closing, service maintenance or installed-file replacement. Extract the original
bundled PlatformHost and run `--check-runtime`; framework resolution must succeed before
`Program.Main` returns0 without pinning an image, creating logs, initializing Windows APIs
or contacting services. Do not substitute registry presence, PATH dotnet or a newer major/x86
runtime for the actual x64 application's framework resolution.

The mandatory dependency is .NET8 Windows Desktop Runtime x64, including its core runtime.
On failure, return a blocking Setup error naming that dependency, the native launch/exit code
and the official .NET8 download page. Interactive Setup offers to open the page only after
consent; failed browser delivery leaves the address visible. Silent/suppressed Setup must not
open a browser or install prerequisites automatically. Extraction failure reports package
repair rather than asserting a missing runtime. WebView2 is feature-specific PDF export
support, not a whole-product installation prerequisite.

Setup diagnostics own `installer.dependency.started/completed/failed` with the install
correlation, fixed dependency/stage, elapsed time and actual launch/exit code. Optional
`download_failed` retains its browser native code. The probe itself has no side effects or
logging dependency. Original trusted-update authentication, recovery and postinstall checks
remain unchanged.

`repo:tests/architecture/installer-runtime.ps1` covers actual Setup callbacks, child exit,
missing/load failure, consent/decline, silent mode and browser failure. `-Maintenance` exercises
the real maintenance admission callback without service effects; `-RuntimeProbePath` accepts
a freshly published Host for actual framework resolution. Original BrowserHost replacement
fixtures retain their production dependency callback. Package gates run these before compiling
the normal installer. Fixtures are not actual installed upgrade or visual acceptance.

## Installed archive integration (Phase F P206/P207)

`repo:installer/nxtools-next.iss#CurStepChanged` starts `--install-archive-integration <GUID>
fresh|upgrade` with `ExecAsOriginalUser(..., ewWaitUntilTerminated)` after trusted preparation
and committed file finalization. This post-install action is required even in silent Setup and
has no dependency on the optional Desktop launch. Package finalization remains committed on
integration error. Exit 0 is observed Apply (or acknowledged opt-out), 10 is an applied plan
with an unconfirmed Windows default association, 11 is unconfirmed/failed Apply, and 12 means
the original user was not established. Interactive Setup shows the corresponding message;
silent Setup retains the exit code and correlation in its log.

`repo:rust/apps/nxtools-desktop/src/install_archive_integration.rs#run` waits through an approved
matching protected Desktop redirect. A Windows original-user probe rejects SYSTEM or a token
not matching the session Shell user before profile access; a pre-elevated launch without
the original account cannot substitute another profile. Fresh untouched archive intent is
initialized through the original config lock; upgrades reuse the saved intent without writing
other settings. The stand-alone entry uses only the original platform bridge/Host on cold start
and the same authenticated desktop pipe when another instance holds the lock. Only a validated
correlated native terminal (including desired/applied revisions) completes the install action.

`repo:installer/uninstall-cleanup.ps1#Restore-NxToolsArchiveIntegration` waits for the installed
PlatformHost's `--uninstall-archive-integration <GUID>` under the current interactive user after
owned-process shutdown. The original native Revert service, not PowerShell, owns the applied
ledger, prior choice restoration and exact Classes cleanup. Failure aborts before personal data
or installed-file removal and preserves retry facts. No raw association cleanup or separate
ledger deletion is allowed in the installer. The injected uninstall cleanup fixture asserts
that an incomplete native result stops the actual script boundary. `installer-runtime.ps1`
provides isolated silent Setup callbacks with a fake Desktop child and exits 0/10/11/12;
running those installers and real original-user/alternate-elevation Shell acceptance require
separate authorization.
