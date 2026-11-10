# OPT-05 — independent acceptance

## Decision

**Partially accept OPT-05 — Reusable format and update plans; do not accept closeout.** Operation-local format reuse, selection semantics, ZIP payload responsibility and the tested native publication/cleanup boundaries pass. Two in-scope items remain: **O5-AC01**, a production diagnostic contract defect, and **O5-AC02**, missing red-capable worker regression evidence. Neither is an observed corrupt publication or loss of source protection.

This independent decision governs current status. The [delivery acceptance](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt05-acceptance.md) remains its original delivery verification record. [Ordered work](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/implement.md), [requirements](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/prd.md) and the parent's [checkpoint](D:/Code/NxTools.Next-main/.trellis/tasks/10-16-archive-manager/phase-a-checkpoint.md) retain the canonical OPT order and **Phase E — Format expansion and ZIP editing**.

Scope: relevant OPT-05 working-tree changes against HEAD `0940eec23286c1a750b3aff4284f32e76da68164`, including untracked plan/selection/observation helpers. Prior OPT-02/03/04 independent acceptance and unrelated dirty changes are preserved. This review changed acceptance documents only, not product or test source.

## Open findings

### O5-AC01 — Medium: production native diagnostics omit source reads and merge target/output roles

**Confirmed diagnostic defect.** Ordinary same-path ZIP editing reaches this boundary. The source lease records its five fresh reads in a separate [ReadObservation](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveSourceLease.cs:162). No production caller exports those counters. [ArchiveOutputSession.Diagnostic](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs:682) exports only that output owner's `native_full_reads` and `native_read_bytes`, combining target admission/begin with output fingerprint/postpublication.

The current successful native owner trials contain **source5 / target2 / output2** complete reads. The correlated output completion exports only the output owner's four reads, not separate source, target and output counts/bytes. The source-only work and interrupted source-read bytes are unavailable to production terminal diagnostics. The existing [completion assertion](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost.Tests/Opt05NativeObservationTests.cs:427) explicitly accepts the merged output-owner totals, so the passing fixture does not close this defect.

This violates the [native observation diagnostic contract](D:/Code/NxTools.Next-main/.trellis/spec/platformhost/archive-read-observation.md:25): actual source/target bytes, output fingerprint/postpublication bytes and partial readback bytes must remain separate under the original operation correlation. The fixture's role-separated `Snapshot()` establishes the measurement, but is not a production diagnostic export.

**Required repair:** aggregate the existing bounded observations at the original bridge/session's low-frequency terminal boundary; expose separate source, target, output and partial-readback counts/bytes with the original correlation, elapsed time and native errors. Preserve actual returned-byte accounting for interrupted reads. No new hashing, source authority, per-chunk events or payload/path logging is needed.

**Acceptance evidence:** a real owned same-path edit must assert production diagnostic fields for source5/target2/output2 and exact role bytes, in addition to one publication and actual cleanup. An interrupted already-established lease read must report its returned bytes without full-read credit and preserve the original cancellation result. Removing a role export must turn the diagnostic assertion red.

### O5-AC02 — Medium: no-change Add fresh qualification lacks a protecting worker assertion

**Regression evidence gap, not a confirmed product defect.** The supported sequence is ZIP Add with all incoming files skipped and directories merged. No candidate is built. [worker_zip_edit::execute](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_zip_edit.rs:246) correctly runs a fresh read-only `modification_eligibility` for this branch, because candidate admission cannot discharge it.

The existing [actual-worker test](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/tests/eic_repair_worker_contract.rs:42) delegates to a fixture whose [all-Skip assertions](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/tests/fixtures/eic-edit-repair.py:135) cover completion, prompt/summary, source preservation and zero begin/commit/incoming payload open. They do not assert eligibility or structural admission. Deleting the fresh qualification call leaves these assertions satisfied. Direct candidate refusal tests do not enter this no-candidate branch.

The [operation-local plan contract](D:/Code/NxTools.Next-main/.trellis/spec/nxtools-archive/backend/reusable-plans.md:24) explicitly retains this independent qualification; the [verification contract](D:/Code/NxTools.Next-main/.trellis/spec/shared/verification.md:36) requires a boundary assertion that turns red on the changed behavior.

**Required repair:** extend the checked-in actual-worker implicit/explicit all-Skip regression to assert exactly one fresh eligibility check and the expected local/descriptor structural admission, while retaining zero candidate payload/output/publication and unchanged source. Removing or bypassing that qualification must fail. The current production branch itself needs no behavior change for this finding.

## Accepted boundaries

| Gate | Independent result |
| --- | --- |
| G-03 | Scoped pass: one operation/attempt-local TAR spool/catalog/ordinal plan; ZIP plan exclusively borrows its actual reader and immutable entries; readmission enters the original extraction finalization; original source/codec/metadata/budget/cancellation checks remain. |
| G-04 optimization/preservation | Executed payload/format/native boundaries pass: retained/new final-candidate integrity, removed/replaced-away source integrity, corrupt candidate/readback refusal, exact record representation, five fresh native stages and actual publication truth. Closeout remains pending O5-AC02 and the production diagnostic contract. Complete G-04 is still open. |
| G-05 | Scoped automated pass: original IDs/order, noncontiguous ordinals, nested/overlapping/empty/implicit roots, component boundaries, slash projection, collision/stale refusal and one catalog expansion. Descriptor presentation remains a distinct ordered projection. |
| G-24 | Scoped pass: original browser selection and destination/form rules; actual selected ZIP/7z/RAR source binding and readmission refusal/success. |
| G-19 touched | Tested source/output ownership, single publication, failed/unknown/no-retry/late-cancel precedence, source exclusive reopen, owned cleanup and retained split/preview controls pass. O5-AC01 remains a diagnostic defect, not a publication failure. |

The attribute-defined directory Rename, ordinary edit duplicate-admission and ZIP readmission regressions have effective boundary assertions. The reader-bound ZIP plan preserves original ordinals, local/central/descriptor/ZIP64/AES/encoding/extent/overlap checks. TAR execution admits live locator/catalog memory with the existing codec/buffer allowance. No validated offset or diagnostic observation becomes browser, global-cache, edit, split-volume or reclamation authority.

Native equivalence remains confined to complete live version equality under continuous protection within the same fresh stage. Read completion stays single-use; source FILE-only release follows actual reader close, parents/candidate remain protected, and flush/fingerprint/prepublication/native publication/postpublication retain their original owners.

## Independent execution

Every Cargo invocation used `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, offline mode, one Cargo lane and full-operation timeouts. Native selections used effects-reviewed owned fixtures, the rebuilt matching test host/worker, exact LIST followed by identical RUN and zero skips.

| Check | Observed result |
| --- | --- |
| Archive ZIP/TAR plan + extraction/edit worker contracts | ZIP7, TAR12, extraction10, edit1:30 executed passes; explicitly executed opt-in TAR measurement1/1. |
| Shared selection library filter |17/17; with the preceding contracts,48 executed passes. |
| Desktop production modules |Browser58/model21/forms41/preview4:124passes; separate Add-result selection6/6. Groups retain their selections and are not represented as new unique features. |
| Protocol/IPC |Protocol unit104 plus reclamation-contract2; IPC unit55. No doc-test zero-selection result is counted as coverage. |
| Worker and C# host builds |PASS; C#0warnings/0errors. |
| Current native observation setup control |LIST1/RUN1/PASS1/SKIP0; same current code, not historical before evidence. |
| Current native core |LIST12/RUN12/PASS12/SKIP0: identity/version/cancellation, equivalent observations, compound reuse, selected source binding, readmission refusal, retained split ZIP, preview adoption, Add namespace ZIP, ZIP all-actions/stale-source, handoff gates, failed/unknown/no-retry and same-path cancellation truth. |
| Native owner and compound facts |Three current owner trials each source5/target2/output2, five fresh stages, one native publication, FILE-only/reader-close and actual exclusive reopen. Ten current TGZ/TXZ routes each outer1/scan1/index1 with exact expected output facts and cleanup. |
| Strict lint |Archive isolated-codecs and default Desktop all-target Clippy, `-D warnings`:PASS. |
| Source contracts |Tree-root drag3, Phase F ownership, archive CLI16, Kit catalog/release and pinned editor:PASS. These are source/document gates, not native or visual acceptance. |

Primary Rust commands were:

```text
cargo test -p nxtools-archive --features isolated-codecs --offline --test opt05_zip_plan_contract --test opt05_tar_plan_contract --test worker_extraction_contract --test zip_edit_worker_contract -- --test-threads=1
cargo test -p nxtools-archive --features isolated-codecs --offline --lib selection -- --test-threads=1
cargo test -p nxtools-archive --features isolated-codecs --offline --test opt05_tar_plan_contract opt05_tar_baseline_three_completed_trials -- --ignored --exact --test-threads=1 --nocapture
cargo test -p nxtools-protocol -p nxtools-ipc --offline -- --test-threads=1
cargo clippy -p nxtools-archive --features isolated-codecs --all-targets --offline -- -D warnings
cargo clippy -p nxtools-desktop --all-targets --offline -- -D warnings
```

Desktop ran the actual `archive::browser::tests::`, `archive::model::tests::`, `archive::forms::tests::`, `opt04_preview_` and `opt03_add_selection_tests` selections. Unrelated integration targets selecting zero tests are excluded.

## Measurements

The [measurement JSON](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt05-measurements.json) and [tables](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt05-measurements.md) match their retained raw sources: **240 raw samples,118 statistical summaries**, all principal before/after boundaries with three completed trials. The referenced before/delivery binaries and raw reports are present. Medians/minima/maxima, per-route work, cleanup scope and the Markdown tables were independently recomputed. This is a retained-data audit, not a new execution of the historical before binaries.

| Retained boundary | Before → After | Interpretation |
| --- | --- | --- |
| TGZ/TXZ outer decode |2→1|Actual owner work decreases. |
| Selection expansion visits |268,435,456→65,536|One catalog expansion; not all projection work. |
| Extraction plan median |164.915s→5.919s|The three retained ranges are disjoint. |
| Drag model median |102.370s→37.991s|Large improvement with substantial remaining descriptor/projection cost. |
| Native old-object full reads/bytes |14→7;117,442,220→58,721,110bytes|Excludes the separate two full output reads; role-separated fixture observations substantiate the result. |
| ZIP metadata open/Test medians |57.825→44.788ms;33.328→23.080ms|Both retained ranges improve. |
| ZIP metadata single extract median |24.261→24.970ms|Not an improvement. |
| Plain TAR worker extract median |437.514→463.174ms|The delivered range is slower; reduced/indexed work does not imply universal end-to-end speedup. |

A further **58 current samples** reconcile the explicitly run TAR trials45, compound routes10 and native-owner trials3 with their operation-local work and lifecycle assertions. They validate current behavior without supplying an additional historical performance ratio. The native setup file named `measurements-before.json` in this review was deliberately generated by the current implementation; `independent-acceptance-scope.json` records its control-only role.

Evidence root: `D:\.pi-desktop\data\scratch\aab2f188-352d-4f1c-b7a3-5375fe0df2d8`. Logs use `opt05-acceptance-*`; the retained-data audit is `opt05-acceptance-measurement-audit.json` from `audit_opt05_acceptance.py`. Current native compound data is `opt05-acceptance-native-compound.json`; owner trials are under `archive-product-opt05-native-5f95fcfa-df07-42da-88b2-b2756e2db8f1`; current TAR trials are `opt05-tar-QMLQlQ/independent-current-baseline.json`.

Frozen current worker: `opt05-independent-acceptance/Debug/NxTools.ArchiveWorker.exe`, SHA256 `43A3081CE6D35679C1E29E6EF9EF4250229AF30225D269E9D9B601449DA7B5ED`. Native invocations verified that freeze against the current build. Existing pair identities are execution provenance, not additional archive checksums or source authority.

## Limitations and stop point

O5-AC01/02 remain open and require a directed repair/acceptance pass before OPT-05 closeout. Previously accepted functionality and the green executed matrix remain valid. No additional broad review or implementation package is authorized by this report.

Total source/internal codec I/O, exclusive codec time, peak memory/RSS and separate native compound cleanup duration remain unavailable. ZIP record-admission seeks, ordinary TAR parser scans, cumulative locator capacity and owned normalization results retain their narrow definitions. Other slower/overlapping routes remain in the full table. The native owner latency includes fixture staging/checkpoint work. Large current Desktop drag projections still take tens of seconds; one expansion is not physical UI responsiveness.

Split ZIP still conservatively validates every external volume. Locked7z complete-block decoding and RAR backward replay remain disclosed costs. The full previously accepted OPT-04 native matrix and the delivery's additional71-test preservation/32-file formatting groups are retained evidence, not newly rerun counts in this review.

Overall status remains `in_progress`. G-00, OPT-00/01 overall closeout, complete G-04, Phase E and physical/Release acceptance stay open. OPT-06/07 remain unstarted. No reference-program execution/new reverse engineering, real GUI/Explorer/clipboard/picker, user archive mutation/recycling/HZ5, service/registry, package/sign/install, commit or push occurred.
