# Phase F — F-RC01/F-RC02 repair outcomes

Date: 2026-10-23. **Two release-chain repairs pending independent acceptance.** Same canonical
**Phase F — Explorer integration and genuine native drag**; not a new phase or full release-chain PASS.
Authority and smallest change boundary were recorded in [existing plan](implement.md#approved-f-rc01--f-rc02-repair-boundary--2026-10-23)
and [sole checkpoint](phase-a-checkpoint.md) before source edits. The historical
[negative independent acceptance](phase-f-release-chain-acceptance.md) and
[original outcomes](phase-f-release-chain-outcomes.md) are unchanged.

## Direct evidence and scope

Read AGENTS, applicable shared/native specs, current plan/checkpoint, negative acceptance, prior
outcomes, original package/setup/recovery owners and their tests. Read original independent scripts
`staging-key-cleanup-probe.ps1` and `installer-reentry-probe.ps1` in the supplied independent scratch
root; did not run their real signing/building path. The already confirmed causes required no broad
research, competing framework or optional review. This round used no delegate/review cycle.

Only production source changes: `scripts/package.ps1`, `installer/close-owned-processes.ps1` and
`installer/nxtools-next.iss`; new two focused PowerShell regressions, existing installer source gate,
applicable spec/reference and plan/checkpoint/new report. No C#/Rust/business/COM/dependency/vendor edit.

## F-RC01 — exact temporary test-key lifetime

Before: TEST-ONLY-private.pem was created before the main try/finally; trust generation or outside-package
label failure bypassed cleanup. After: `$ownedTestKeyPath` starts empty, is assigned the exact fresh GUID
build's key path INSIDE the main try before creation, and is the only key path removed by success/finally.
Create-then-throw, test-trust generation, label and preflight failures all unwind through the same cleanup.
No scan-derived/recursive key deletion, production SigningKeyPath inference, new signing system or
workaround that merely creates the missing output parent. Fresh scratch/SkipInstaller/SkipVerification/
no supplied production key/sequence/LegacyUnsigned restrictions stay unchanged.

Regression: `tests/architecture/staging-key-cleanup.ps1` copies the actual package wrapper byte-identically
and executes its original control flow. Only external key/trust/build-tool scripts are replaced in that
GUID-owned copy: the key stub creates a synthetic marker, not private-key material; trust/preflight
stubs inject fixed failures. Exact label WriteAllText failure uses a genuinely missing parent. No source
control-flow rewrite, real key-tool build, Cargo, product/service/installer operation or private PEM read.
A neighbouring foreign-owner marker must survive; test emergency cleanup removes only its recorded
exact synthetic marker after observing leakage in the old implementation.

Observed RED before source fix:
- `rc01-red-label.log`: exit1, expected label failure=true, key_remaining=true, no output.
- `rc01-red-trust.log`: exit1, expected trust failure=true, key_remaining=true, no output.
- `rc01-red-key.log`: exit1, create-then-throw failure=true, key_remaining=true, no output.

Final GREEN `rc01-final.log`: **20 assertions**, four cases (create/trust/label/preflight), exit0;
failed call + expected failure + exact owned key absent + no output + foreign file intact. All stop
before Cargo. The first green16-assertion version is retained in `rc01-green-all.log`.

## F-RC02 — recovery reachable from the actual next preparation

The helper default preparation call reached from Inno now invokes
`Prepare-UpgradeBrowserHost(InstallRoot, StateFile, OperationId, NewManifestPath, NewSignaturePath)`
instead of directly fresh-suspending. Fresh `Suspend-UpgradeOwnedSet` deliberately STILL rejects an
existing journal. Preparation is a bounded composition of the original journal validator,
`Complete-UpgradeOwnedSet` / `Undo-UpgradeOwnedSet`, then original fresh Suspend — not another installer
or recovery framework.

Order and authority:
1. Validate fixed sibling journal location, no reparse path, bounded file, schema2, original root,
   original operation/phase, exact owned entry names/backups/hash/type facts using original validator.
   Unknown/malformed/mismatched or unproved `.pending` publication refuses and retains evidence.
2. `committing`/`committed`: call original Complete with committed=true; verify complete NEW set and
   durable cleanup evidence, resume exact old-backup cleanup only. Never postcommit Undo.
3. Valid precommit phase: reject contradictory cleanup evidence; call original reverse Undo (every
   recoverable item attempted even if one fails). Only after success verify the ENTIRE old set and
   absence of backups, then retire only that old journal. Historical failure_count does not force
   permanent manual cleanup once the actual complete old set has been independently revalidated.
4. Confirm old journal/pending gone before calling original fresh Suspend with the NEW manifest.
   New manifest hashes never grant deletion authority for the old transaction. Failure cannot admit
   a fresh transaction or continue file replacement.
5. Inno sets `UpgradePreparationReady` only after successful helper return; failed PrepareToInstall
   cannot run Deinitialize's Finalize against a rejected/unowned old journal. Failure copy requests
   retaining logs for diagnosis, not manual journal deletion. Existing Finalize commit-point semantics,
   exact-hash deletion/foreign-file preservation and all old file owners remain intact.

Production recovery diagnostics: `installer.upgrade_set.recovery_started` and `recovered`, current
correlation + validated original operation/phase only; original aggregate failure/terminal events remain.
No payload, key bytes, file names or repeated retry stream added.

### Actual two-entry regression

`tests/architecture/installer-preparation-recovery.ps1` dot-sources only `-LibraryOnly`, forbids process
discovery and stubs process-stop boundary. It extracts the EXACT default helper preparation call used
by Inno from current production source; both first and second invocations use that call.
First attempt -> suspend -> publish complete v2 -> inject backup-cleanup or journal-delete failure.
Second attempt -> same preparation entry with different v3 manifest -> recover original facts -> new
transaction admitted. The test never manually calls Complete/Finalize to substitute for second-entry
recovery. First-attempt Complete is used only to create the specified postcommit failure.

Observed RED before recovery source fix:
- `rc02-entry-red-committed.log`, exit1: committed journal + complete v2, second preparation rejected
  by `Previous owned-set journal requires recovery; refusing new transaction.`
- `rc02-entry-red-committing.log`, exit1: partially cleaned committing journal + complete v2, same rejection.

Final GREEN `rc02-final.log`: **100 assertions**, exit0. Both postcommit states recover before first new
file move; hook at that first move verifies all12 v2 files still intact and the fresh second-operation
journal, then all12 fresh backups equal v2 (not v1). Also covers precommit restore, one restore failure
with all12 reverse attempts/no new admission, successful next preparation after repair, new manifest
unable to authorize old foreign-file deletion, invalid JSON/.pending and root/schema/phase/operation
mismatches retained/rejected. Structured result file records exact entry, assertion count and work root.
The first97-assertion green remains in `rc02-green-entry.log`.

Honest harness note: initial `rc02-red-{committed,committing}.log` stopped during FIRST suspension due
to long temporary backup paths, not the requested defect. These are retained as invalid target repros,
not claimed reds. Test work moved to short GUID-owned `ipr-*` paths under session scratch; valid target
reds are specifically `rc02-entry-red-*`. No production path policy was relaxed to pass the harness.

## Commands / verification

Unique evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\phase-f-release-repair-20261023-113158`.
Raw command logs and `.exit.txt` files retained; final executable regression commands in `commands.jsonl`.

| Actual check | Result |
| --- | --- |
| `staging-key-cleanup.ps1 -EvidenceRoot <root>` | exit0,20 assertions; exact copied original wrapper prebuild flow |
| `installer-preparation-recovery.ps1 -EvidenceRoot <root>` | exit0,100 assertions; real preparation call twice, LibraryOnly/fake process boundaries |
| `installer-owned-set-rollback.ps1 -EvidenceRoot <root>` | exit0,632 assertions; existing all-member/commit cleanup/foreign/old API evidence retained |
| `archive-release-chain.ps1` | exit0; unsafe wrapper invocations reject without production trust mutation |
| `package-memory-policy.ps1` | exit0; source/environment contract only, no Cargo |
| `installer-exact-file-rollback.ps1` | exit0,2 isolated exact-handle deletion assertions |
| PowerShell Parser::ParseFile | 7/7 PASS: wrapper/helper/two new regressions/old set/runtime/replacement scripts; no execution of installer scenarios |
| `installer-update-lifecycle.py` | exit0; Inno helper preparation, recovery-before-fresh, ready guard and original commit wiring source checks |
| `spec-contracts.py` | exit0,109documents/14owners; docs/source evidence only |
| `task.py validate .trellis/tasks/10-16-archive-manager` | exit0,82implement/79check; existing over32KiB injection warnings remain, explicit relevant ranges read |
| scoped diff check | PASS; no unrelated cleanup or format |

No C#/Rust edits, so no host rebuild,30-fixture repeat,Cargo,Release staging,fullDesktop,P208worker,
codec/split/ZIP-edit/100k/performance reruns. No formal key creation/signing/sequence reservation, no
installer compile/run, real process discovery/termination/Registry/service/COM/Explorer/GUI operation.

## Protected current release and preservation

Current user package is **sequence22**, not sequence21. New opening baseline2977 paths was captured
before edits; Git non-ASCII path quoting initially skipped2 paths, then complete unquoted2977 baseline
was regenerated before source changes. Initial incomplete attempt is not final preservation evidence.
`final-preservation.json` records all per-file hashes and owned/unrelated attribution.

- `artifacts/win-x64` + `artifacts/installer`: **83/83 unchanged**, no extra/missing files.
- Sequence22 installer SHA256 unchanged:
  `8FD5602A067292FD73A338591A840FD29F09F1046A32EA61D564521743AAA41B`.
- Existing production Generated **3/3 unchanged** (core/archive approval and public trust).
- Exact located publisher ledger
  `%USERPROFILE%\.nxtools-release-keys\nxtools-next-release.pem.release-sequence.json`:
  sequence**22** unchanged; SHA256**AF51E49F4CE1F5E6CF65ED544E64D418273C3268D12F820851D3EFC8137B20B1**
  unchanged. Only ledger bytes/sequence were read; private PEM never read, printed or modified.
- **56/56 historical archive acceptance/outcomes unchanged**, including the negative F-RC report and
  original release-chain outcomes. Existing user artifacts/backups were not overwritten.
- No baseline files deleted; no unrelated task files changed. Test-owned synthetic key markers have
  zero remaining instances after observed-result cleanup; no actual private key was generated by tests.

## Stop

**Two release-chain repairs pending independent acceptance.** F-RC01/F-RC02 are implemented and their
original failure boundaries now green at source/isolated level, not independently accepted yet.
No claim that full release chain, P206/P207 live, P208, Phase F or first release is complete.
Current sequence22 binaries still contain their pre-repair source snapshot; they were intentionally
NOT rebuilt or replaced. This source repair does not silently repair the running user's package.
Old enrolled bootstrap compatibility/maintenance remains a separately authorized later prerequisite,
not a third task. ProductApplyAuthorized=false and P208 production rows remain unchanged. Earlier
A–E/JOB07/SAFE08/native/user obligations retained. No live install/update/deploy/Registry/COM/Shell,
source recycle, bootstrap replacement/trust rotation, commit or push. Stop here, no automatic packaging.
