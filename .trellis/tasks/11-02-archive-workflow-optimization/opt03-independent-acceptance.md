# OPT-03 — independent acceptance

## Decision

**Partial acceptance; OPT-03 is not accepted for closeout.** G-21/G-22 pass within the tested automated/owned-native scope. Input, Add-result and post-create Test paths have substantial passing evidence, but G-12 needs one connection-binding repair and G-13/G-23 need two real-boundary regression corrections. The latter are evidence gaps, not confirmed production publication defects.

The task remains `in_progress`; OPT-04 through OPT-07 remain not started. G-00, OPT-00/01 overall closeout, complete G-04, Phase E and physical/Release acceptance remain open. Previously accepted OPT-02 behavior and historical evidence are retained.

Scope: uncommitted OPT-03 changes against HEAD `0940eec23286c1a750b3aff4284f32e76da68164`, including input/form observations, each-input output plans, Add display results and native Create facts through post-Test. No product or test source was modified by this acceptance review. Existing unrelated changes and earlier archive slices were preserved.

Requirements and delivery: [implementation](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/implement.md), [acceptance](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/acceptance.md), [delivery evidence](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt03-intake-results.md), [Create mapping](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt03-create-options.md).

## Standards

### O3-AC01 — P1: final Add submission does not retain the original connection

**Supported trigger:** open an Add form on a valid browser, then replace/reconnect PlatformHost while the application has not yet applied the old connection's Disconnected notification. Submit the existing form or begin another intake request from it. The source file can remain unchanged throughout.

The Add origin contains window ID, browser generation and snapshot only. The final guard checks path, browser/source identity, snapshot and folder, but not the connection generation on which that draft was created. Individual Paste/picker requests capture their own current connection; this does not bind the lifetime of the Add draft itself. On a new connection the unchanged source may be acquired again with the same native identity, so native source revalidation is not a substitute for draft connection isolation.

Evidence:
- [Add draft/origin construction](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/application.rs:1362).
- [Stored edit-form origin](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:136).
- [Final submission guard](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:421).
- [Intake target guard](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/clipboard_requests.rs:14).
- [Router disconnection delivery](D:/Code/NxTools.Next-main/rust/crates/nxtools-ipc/src/lib.rs:479): connection state and application notification delivery are separate; the notification is not a synchronous form invalidation.

This violates G-12's stale-input refusal and [the documented original-connection contract](D:/Code/NxTools.Next-main/.trellis/spec/nxtools-desktop/backend/archive-batches.md:55). Existing tests apply a Disconnected notification or test a pending intake's connection, not the draft's final submission after an actual router connection replacement before that notification is processed.

**Repair:** retain the captured connection generation in the existing edit-form origin/context, and require it at intake and final submission. Keep original browser/source/folder checks and deferred updates; do not add a parallel form controller or weaken native validation.

**Required regression:** open Add through the original command/router/form path, replace the connection without first applying the old disconnect event, then invoke the actual submit control. Use an unchanged source/browser identity. Assert no logical job and no ArchiveJob Start is queued. Cover a new picker/Paste action from that stale draft, plus the normal same-connection case.

### Confirmed retained boundaries

- Add Remove uses the official Small outlined Button, guards busy state in both rendering and callback, changes only the local draft and preserves existing Create Remove.
- HDROP-only intake bounds the complete list, balances borrowed Open/Close and Lock/Unlock without freeing the borrowed handle, and does not treat Cut as source-deletion authority. Text-input paste keeps its original owner.
- Picker/Paste pending state precedes send and is rechecked after deferred application against its request connection and original folder/snapshot. New-all retains the full source list.
- Each-input output planning precedes all admission; disk/requested/generated names and split-family reservations are separate from native write authority. The directory namespace is indexed once. Full/near-full queue cases refuse atomically.

## Spec

### O3-AC02 — P2: several no-refresh tests are rejected before the business boundary

The negative cases in [opt03_add_selection_tests.rs](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/opt03_add_selection_tests.rs:100) do not establish all advertised G-13 terminal behaviors:

- `all_skip` and `cancelled` use `commit_state="Prepared"`; `unknown_commit` uses `CommitStateUnknown`. Neither string belongs to the [valid commit-state vocabulary](D:/Code/NxTools.Next-main/rust/crates/nxtools-protocol/src/archive_job.rs:1019).
- `unpublished`, `unknown_publication` and `unclean` retain a successful `add_result` while contradicting its required publication/cleanup conditions. The [public terminal validator](D:/Code/NxTools.Next-main/rust/crates/nxtools-protocol/src/archive_job.rs:966) rejects those combinations.
- [job_notification](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:1109) rejects an invalid event before `job_terminal`. Consequently unchanged browser state in these cases can result from rejecting the whole terminal, not from applying a valid terminal and correctly refusing refresh.

Normal native Skip/no-change cases pass, and the production refresh guard is present. This finding concerns the claimed unpublished/unknown/unclean Desktop lifecycle evidence, not a confirmed unwanted refresh in production.

**Repair and acceptance:** construct schema-valid terminal facts without `add_result` for allSkip, precommit failure/cancel, CommitOutcomeUnknown and cleanup-incomplete outcomes. Before delivery assert `event.validate().is_ok()`. Through the original notification path, assert that the matching job terminal was installed, its publication/cleanup facts were retained and no ArchiveBrowserRead/selection change occurred. Keep malformed-frame rejection as a separately named test. Published late cancel remains a distinct positive refresh case.

### O3-AC03 — P2: post-Test cleanup conjunction is not exercised on the Test side

[post_test_case](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/opt03_result_tests.rs:76) always delivers a Test terminal created with cleanup=true. The `unclean` branch in [the other result test](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/opt03_result_tests.rs:139) changes the Create terminal, so Test never starts. The disconnect case correctly preserves a still-running Test, but is not a completed cleanup=false Test terminal.

Production [job_terminal](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:1494) merges Create facts and conjuncts cleanup, then passes that fact to queue retirement. No error in those lines was confirmed. The explicit G-23 requirement still needs a regression capable of detecting accidental cleanup=true or early slot release after Test cleanup failure.

**Repair and acceptance:** consume a real successful C# Create export, observe the original Test Start, then deliver a schema-valid matching Test terminal with cleanup=false. Assert retained Create counts/journal/commit/publication/location, final cleanup=false, the original running reservation still occupied, and no automatic location, generic Retry or recycling grant. Repeat for the applicable ordinary/split primary cases. The Test outcome/error remains its own; no invented rollback or source reacquisition.

### Confirmed positive result and creation behavior

- Actual five-format Add results carry final added/overwritten/renamed names; skipped and merge-only entries do not become results. The 301-member business operation is independently decoded in full while display output remains limited to256 members/64KiB.
- C# admits the private result only for the original successful committed/published/clean Add context. Display names do not grant filesystem authority.
- Current browser refresh maps names to new snapshot IDs under original source/window/generation/folder guards. Confirmed published late cancellation refreshes without a success result selection.
- Native Create locations are captured before output disposal. Split ZIP uses `.zip`, split7z uses `.001`. Missing location is not guessed from a request.
- Actual C# exports feed the Desktop post-Test path. Success/failure/cancel keep Create publication/count/journal/location; tested disconnect keeps the original slot and saved Create facts.
- Seven supported Create families, admitted option combinations, same-basename separate outputs, compound names, future reservations, late native target confirmation and queue limits pass the listed tests. No source recycling, new writer backend or family-atomic split guarantee is introduced.

## Independent checks performed

All Rust invocations used jobs1/incremental0/codegen-units256 in the same PowerShell call, one Cargo lane and full-operation timeouts. C# build and exact LIST→identical RUN used this review's frozen worker and current test host. Temporary inputs and exports were confined to the session scratch. Real system clipboard/picker/Explorer/product windows were not operated.

| Check | Observed result |
| --- | --- |
| `opt03_add_result_worker_contract` | 1/1;35 actual-worker scenarios across five formats, including full301-member operation/bounded256-member result |
| `opt03_create_options_contract` | 2/2; seven-format admitted writer configurations and illegal options |
| C# test-host build, no restore/single MSBuild/no shared compilation | PASS;0 warnings/errors |
| Exact `OPT03` plus three original progress export/compatibility fixture names | selected8/run8/pass8/skip0/fail0; OPT03 five fixtures include21 Add scenarios, memory HDROP, held destination, actual Create locations/late collision and result DTO |
| Desktop original Job suite | 107/107 with newly exported native Create and C# progress observations |
| Desktop forms / draft / progress feedback | 41/41,12/12,13/13 |
| Desktop `opt03_` / output-plan selections | 17/17,15/15; these overlap the original Job suite and are not additive |
| Protocol `archive_` / IPC `archive` / CLI `archive` | 50/50,17/17,CLI3 unit+4 integration |
| Exact native seven-format Create interoperability plus OPT-02 preservation | selected11/run11/pass11/skip0/fail0;8 Create fixtures and3 namespace fixtures covering79 preserved scenarios |
| Five-package all-target strict Clippy, isolated archive codecs, offline | PASS |

The initial progress-feedback filter selected zero tests and is not evidence. The corrected `archive::progress::feedback_tests` selection ran13 and passed. Unrelated integration targets selecting zero are likewise excluded from coverage.

### Native identity and actual exports

Frozen worker SHA256: `E3DFF478BA9CBED906314E345685EECDAA4C1A4852F746D7019061D62F5487AE`, matching the delivery's final worker. Before each native selection the current and frozen files matched. Frozen copy: `opt03-independent-acceptance-worker/NxTools.ArchiveWorker.exe` under this session scratch. Existing Python, system bsdtar and documented stock7Zip were used only as independent fixture readers, not product backends.

Fresh `ordinary.zip.json`, `split.zip.json`, `split.7z.json` exports each have success, completed cleanup, confirmed publication, real result location and committed/published journal facts. Their committed file counts are1/3/3, and primary basenames are `ordinary.zip`, `split.zip`, `split.7z.001`. Desktop tests explicitly consumed these new exports and the two fresh progress exports; absence did not invoke a synthetic Create fallback.

This review's logs and exports remain under `D:\.pi-desktop\data\scratch\aab2f188-352d-4f1c-b7a3-5375fe0df2d8`, named `opt03-acceptance-*`. These are current independent runs, not a reclassification of the delivery's old logs. Owned retained namespace fixtures follow their original evidence policy; actual leases and successful partials retire through original owners.

## Gate disposition and follow-up

| Gate | Independent disposition |
| --- | --- |
| G-12 | Partial: supported intake and native memory-format checks pass; O3-AC01 remains |
| G-13 | Positive actual results/overflow/new snapshot/late-cancel evidence pass; complete no-refresh terminal gate awaits O3-AC02 |
| G-21/G-22 | Accepted for tested automated/owned-native scope |
| G-23 | Actual locations and tested post-Test outcomes pass; Test-side incomplete-cleanup evidence awaits O3-AC03 |
| G-18/G-19 touched | Tested source/production TestWindow/owned-native preservation only, not complete gate or user acceptance |
| G-00, OPT-00/01, complete G-04 | Remain open under their existing records |
| Physical GUI/Explorer/DPI/accessibility and Release | Not run; separate authorization required |

Repair only the original connection-binding boundary for O3-AC01; correct/add the narrow tests for O3-AC02/03 and rerun their affected subsets. Do not restart broad reviews, repeat all native matrices without a relevant change, or begin OPT-04. The three findings were confirmed from current call/data/validation paths; this read-only acceptance did not introduce and run new failing product tests for them.

No product code change, package/sign/install, user-archive mutation, source recycling/HZ5, service/registry action, commit or push was performed. Source/compiled/owned-native passes remain valid evidence, but do not close the unsupported assertions identified above.
