# Independent acceptance — creation admission / namespace remediation

## Verdict

**Original mutable-source/mutable-overwrite/sibling-output findings are repaired; namespace binding is materially repaired and tested at its isolated seam. Overall creation-remediation slice remains CONDITIONAL: one newly reproduced pre-admission directory side effect must be fixed. Phase A and first release remain incomplete.** No product code changed by this review. One bounded directed read-only review only; no repeat review loop.

## Independently executed verification

All Rust commands inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256,serial; no contention/timeout/duplicate. Free space inspected before large test:C18.53GiB,D63.14GiB.

| Check | Actual result |
| --- | --- |
| archive full --features isolated-codecs --offline --no-fail-fast |82passed/0failed/0ignored,exit0;independently summed |
| Same-feature worker build |exit0 |
| C# test host build --no-restore |exit0,0warnings/0errors |
| Exact original62+new13, explicit fixture/oracles |list selected75/run0;execution75/run75/pass75/skip0/fail0/not_run0,exit0 |
| Actual4GiB+17FastDeflate ZIP64 |included and PASS;actual Job/worker/native + Python streaming oracle |
| Strict archive Clippy / fmt--all--check / workspace check--tests |exit0each;check is not workspace tests |
| Extensions archive_import / clipboard archive_ |7/7 and5/5PASS,39clipboard filtered |
| Spec/catalog/context/selector |PASS101docs/14owners;79pages/79rows/78locked;30implement/27check,no truncation;source selectorPASS |

Existing local7-Zip26.00 oracle configured explicitly, same path/hash previously inspected (4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f); no tool installation/download/PATH change. No user archive/system mapping/registry/service/release/install/commit/push. No full/default C#suite,full workspace tests/UI/DPI/UIA or broad resource/platform acceptance.

## Original independent probes re-executed

Rebuilt scratch-only zip-create-review-probe against current PlatformHost and actual worker, identical three interleavings from prior independent rejection. Results:

```text
SourcePaths changed in accepted callback:
  success,Committed1,archive approved.txt,content approved-source (original A)
ConfirmedOverwrites changed and target replaced in accepted callback:
  target_changed,FailedBeforeCommit,commits0,CleanupComplete=true,target retained
source=input directory,output=its parent/created.zip:
  success,Committed1,exact original directory members
```

These directly close prior reproduced observations, not only new developer tests. Probe exit0 records observations; original probe roots/helper removed. Snapshot source reviewed: Capture before validation/wire/Run/accepted, private copied collections and immutable scalar budgets; nested options fixed, bounded observable capture consistency checks. No requirement for arbitrary caller-defined accessor atomic transactions is invented.

Source enumeration now uses held-directory-derived volume-GUID path, reopens/verifies real identity, and child type comes from relative handles rather than original DOS attributes. Output collision identities use actual held output session. New root/nested source-substitution and held-output alias fixtures are included75PASS. No real SUBST/DefineDosDevice remapping was performed; evidence remains controlled namespace resolution plus actual native identity checks. No renewed target-ID CAS/source-set snapshot requirement.

## One remaining P2 — invalid missing output root mutates source before rejection

Sources:
- `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs:241-244`: output bridge/session is constructed before source acquisition/collision checks.
- `ArchiveOutputSession.cs:53-69` and OutputNative.OpenDirectory: Open creates missing output directory components.
- `ArchiveCreationSources.cs:621-626`: only later rejects output-at/below-directory-source.

Independent actual worker probe:
- create source directory with approved.txt and replacement.txt;
- request OutputRoot=<source>/new-output/nested, neither new directory exists;
- create_zip source root is that source directory,createName=created.zip.

Observed:

```text
Category=worker_failure,frame.error=OutputCollision
FilesCommitted=0,CleanupComplete=true,final archive absent
output_created=true
source members include new-output and new-output/nested
```

So the dangerous request is eventually refused and no archive/file payload published, **but the request already altered the protected source member set**. There is no demonstrated old-file deletion or content loss. Existing SelfInclusion fixture precreates its inside output root, therefore cannot detect this admission-order side effect.

Required bounded fix: add a red actual-worker regression with nonexistent nested output under source; collision/source eligibility must be checked before any output directory creation. Preserve identity-bound actual held output collision checks and namespace protection. For an allowed missing output directory outside source, define prepare/create ordering and owned rollback so failure does not blindly delete user/raced directories. Do not simply remove missing-directory support or collision guards, and do not recursively clean arbitrary output paths after failure. A read-only existing-ancestor/source-preflight stage or equivalent cohesive ordering is appropriate; no whole architecture/overwrite-contract change needed.

Tests should cover nonexistent internal root with nested chain and multiple sources,existing internal root still rejected,allowed sibling/parent output still succeeds,final no-replace/confirmed overwrite protection,source unchanged on early rejection and exact owned cleanup. Add only needed cases,retain75passing fixtures. Large ZIP64 need not rerun for every local order change; final integrated evidence may include it once when relevant/resources permit. No broad review restart requested.

## Scope / next action

Close the earlier three reproduced defects; preserve their historical failure record. Fix this single concrete output-creation ordering gap,then finish local remediation acceptance before7z/TAR/TGZ/TXZ creation. Remaining full Phase A/native/product/resources remain incomplete scope,not surprise defects. No new user architecture decision is required.

Review artifacts: create-repair-acceptance-{archive,worker,clippy,fmt,workspace,extensions,clipboard,native-list,native-tests}.log; scratch zip-create-review-probe. Its Program.cs was first run unchanged for original three cases,then changed only in scratch for nonexistent-output probe. No project test/source modifications. New probe also cleans its owned temporary root/helper after observing directory residue.
