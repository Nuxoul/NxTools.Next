# Phase F F-RC01/F-RC02 — independent repair acceptance

Date2026-10-23. **F-RC01 and F-RC02 CLOSED at source/isolated boundaries.**
[Original negative acceptance](phase-f-release-chain-acceptance.md) and [repair outcomes](phase-f-release-chain-repair-outcomes.md)
remain immutable. Current status belongs to [checkpoint](phase-a-checkpoint.md).

## Independent source and boundary verification

Read current package key ownership/try-finally, Prepare-UpgradeBrowserHost recovery, Inno
UpgradePreparationReady guard and all three executable regression scripts.

- F-RC01: ownedTestKeyPath assigned inside main try before creation; create/trust/label/preflight
  exceptions all unwind through exact-owned-key cleanup, not a scan or production-key path.
  Independent unchanged-wrapper-copy regression **20assertions PASS**; four failure modes stop before
  Cargo, leave no owned marker/output, and preserve neighbouring foreign-owner marker.
- F-RC02: real default helper call now enters Prepare, validates OLD bounded schema/root/operation/
  entry facts, resumes postcommit cleanup or original precommit compensation, verifies complete
  recovery, then starts fresh Suspend. New manifest cannot authorize old transaction deletion.
  Fresh Suspend still refuses unknown existing journal; Inno only exit-finalizes after successful
  preparation. Independent same-entry-two-invocations regression **100assertions PASS**, covering
  committed/committing, precommit restore, failure/all-items attempted, foreign-file and corrupt /
  mismatched journal refusal. No manual Finalize substitutes for the second preparation.
- Original whole-set/commit-point/foreign-file regression **632assertions PASS**.
- Wrapper rejection, package-memory-policy, exact-file rollback, Inno lifecycle, spec/task/diff gates
  PASS;7PowerShell files parse without errors. No new confirmed in-scope defect found; no further
  optional review/delegate loop. Tests are LibraryOnly/fake system/file fixtures, not real installation.

Evidence:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\release-repair-independent-20261023-123317`.
Scratch runner `release-repair-independent.ps1`, individual logs, commands.jsonl, parser.json,
preservation.json. All exits0;325baselined source/history/artifact/Generated/ledger files unchanged
before recording acceptance. Seq22 installer hash remained
`8FD5602A067292FD73A338591A840FD29F09F1046A32EA61D564521743AAA41B`.
No C#/Rust edits or test reruns, fullDesktop/codec/performance/Release builds during this acceptance.

## Critical separate installed-update blocker

The user's sequence22 installation actually failed. Parent previously extracted the ACTUAL installed
bootstrap assembly read-only and called only VerifyPackage offline:
- oldbootstrap+sequence21:accept;
- oldbootstrap+sequence22:InvalidDataException `Unapproved release path.`;
- currentverifier+sequence22 with SAME old public key:accept.
Evidence parent scratch `install-failure-22/offline-verifier.log`.
The installed72files and protectedDesktop/PlatformHost/Indexer/BrowserHost matched sequence21 after
rollback; no upgradejournal/update transaction remained (`recovery-state.json`).
**These two repairs do NOT change that oldbootstrap allowlist.** A newly built package with the new
archive members remains incompatible with this old verifier. This is not a signature-key problem;
re-signing/rebuilding is not a bootstrap maintenance fix. No service/trust/Registry/COM mutation or
bootstrap replacement authorized/performed. Do not promise installation success or retry the same
upgrade as acceptance. Existing bootstrap maintenance is a separate bounded task and authority.

## Subsequent user-authorized packaging

User explicitly asked to accept then repackage. After the bounded acceptance above, a separate new
release may be built with an increasing reserved production sequence, existing signing key and old
package backup. Its metadata must state the oldbootstrap blocker and testing-only intended use.
The protective seq22 baseline above applies BEFORE that separately authorized packaging, not a
claim that artifacts remain seq22 after a new package is deliberately produced. Packaging success
is not installed-update, liveShell/P208, PhaseF or first-release acceptance.
