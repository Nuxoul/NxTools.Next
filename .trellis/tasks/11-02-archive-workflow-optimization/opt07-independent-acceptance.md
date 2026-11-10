# OPT-07 — independent acceptance

## Decision

**Partially accept OPT-07 — Required batch and browser workflow delivery; do not accept closeout.** The executed batch/native secret/picker, Retry baseline, Locate, Up, Find identity and browser/preview preservation evidence passes at its stated automated/owned-native boundaries. Three reachable Desktop interaction defects remain: **O7-AC01**, **O7-AC02** and **O7-AC03**. They require directed repairs and protecting original-callback regressions before this package can close.

The findings are source-confirmed product behavior, not observed corrupt publication, secret disclosure or native source-ownership failure. Existing green tests do not exercise the precise failing transitions described below. No new failing test or physical product gesture was executed for those three cases by this read-only acceptance.

Scope: current OPT-07 working-tree changes against HEAD `0940eec23286c1a750b3aff4284f32e76da68164`, distinguished from earlier accepted changes using `opt07-before` and `opt07-delivery`. The review changed acceptance documents only; product/test source and unrelated user changes were preserved. [Delivery verification](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt07-verification.md), [workflow contract](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt07-workflows.md), original failing logs, coverage and freezes retain their attribution. OPT-02/03/04/05/06 independent acceptance and closed O5-AC01/02 remain intact.

Overall remains `in_progress`, **Phase E — Format expansion and ZIP editing**. G-18 physical interaction, real G-28 system-picker interaction and G-29's original-object recycle prerequisite remain distinct unaccepted boundaries, not newly discovered product defects.

## Standards

### O7-AC01 — Medium: refused Find Extract/Delete changes the browser model before validation

**Confirmed reachable interaction defect.** A loaded readable catalog may contain a safe-path row with `OutputCollision` or `LinkNotExtracted`, which is marked warning by [BrowserModel::reproject](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/model.rs:203). Find includes that original member and enables its result actions. Selecting the warning row and clicking Extract or Delete reaches [find_action](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/browser_find.rs:223).

The caller has already [closed the Find dialog](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/browser_find.rs:422). `find_action` then calls `model.enter(parent)` before checking the projected row's warning. [enter](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/model.rs:294) changes folder, clears filter/selection/focus/anchor and reprojects. Warning refusal returns before `sync_filter` and `projection_changed`, leaving the browser model changed without its presentation/cache/Tree transition. No form or Job is submitted, but the refused action loses the previous browsing state and can leave displayed state out of sync.

This violates the [original guarded result-action/projection boundary](D:/Code/NxTools.Next-main/.trellis/spec/nxtools-desktop/backend/archive-browser-workflows.md:18): warning Navigate is supported, while Extract/Delete must retain original safety refusal; model and presentation transitions remain one original owner. The passing [warning-row test](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/browser_tests.rs:3099) covers Navigate only.

**Minimal repair:** resolve original identity and action eligibility before closing Find or mutating model state. On refused Extract/Delete, preserve folder/filter/selection/focus/anchor/projection epoch and submit nothing. Keep Navigate-to-warning behavior unchanged. Add actual Find footer callback regressions for both warning actions from a different current folder/filter; assert no model/presentation mutation, no form/Job, and retained supported Navigate behavior.

### O7-AC02 — Medium: retried member is displayed as Running before the new attempt starts

**Confirmed reachable presentation defect.** After a valid single Test/full Extract failure, explicit Retry obtains fresh revalidation, creates a new operation/generation and reenters the existing queue. The old attempt has already set `ever_started=true`. [batch_snapshot_for](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/batch_jobs.rs:137) uses that historical value to choose Admitting versus Running once the queue slot is held.

[Retry setup](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/batch_jobs.rs:598) preserves this historical fact. [send_job_attempt](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:799) correctly resets `worker_state=None` and detail state to admission, but the summary immediately reports Running until the new original native Started event arrives. This is a normal asynchronous admission interval, not a forged event or millisecond-only race.

The [batch contract](D:/Code/NxTools.Next-main/.trellis/spec/nxtools-desktop/backend/archive-batches.md:89) preserves actual attempt/queue states; shared engineering distinguishes acceptance from completion. The new attempt must not inherit its predecessor's started state. The defect is the status projection; no incorrect Pause/native execution authority was established.

**Minimal repair:** derive Admitting/Running from the current attempt's existing reset/Started facts, retaining `ever_started` only for historical never-started accounting. Extend the original Retry callback test through fresh recheck, queue/dispatch, acceptance receipt and new Started. Assert Admitting throughout pre-Started admission, then Running exactly for the current attempt; preserve queued and historical aggregate counts.

**Standards result:2findings, both Medium.** Official Form/InputGroup/ListItem/Table/Progress and direct outlined Button composition, original Root/window owners and deferred cross-entity paths otherwise retain the reviewed structure. No competing runtime, dependency, palette or source authority was introduced by the reviewed slice.

## Spec

### O7-AC03 — Medium: expired checked password-reuse consent is silently downgraded to an ordinary reply

**Confirmed reachable contract defect.** The [password Dialog](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/progress.rs:627) retains the user's checked reuse choice in an `Rc<Cell<bool>>`. Its [confirmation callback](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/progress.rs:697) combines that choice with current eligibility, but treats ineligibility as `remember_batch_password=false`. It still constructs the password reply, clears the input, closes the Dialog and sends the ordinary member-only reply.

A supported sequence is: failed member B begins explicit Retry's [asynchronous native recheck](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/batch_jobs.rs:359); before its result returns, same-batch member A presents its initial password prompt and the user checks reuse. B's valid recheck returns and [retires the whole batch's reuse eligibility](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/batch_jobs.rs:615). A's same prompt/ticket remains open; [progress refresh](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/progress.rs:351) does not recreate the Dialog for an unchanged ticket. Confirming A now consumes its ticket with the checked request silently downgraded.

The [explicit consent contract](D:/Code/NxTools.Next-main/.trellis/spec/nxtools-desktop/backend/archive-batches.md:91) states: “Invalid consent leaves the current ticket intact and sends nothing.” [reply_job](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:1123) correctly enforces this for a true remembered reply, but the UI masks the choice before that guard. The passing [retired-generation regression](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/opt07_batch_tests.rs:499) invokes that guarded entry directly; it does not exercise an already-checked Dialog across sibling retirement.

**Minimal repair:** when the current Dialog's reuse choice is checked but eligibility has expired, refuse confirmation before clearing input, closing or scheduling a reply. Preserve the ticket and an explicit current choice, as well as the final `reply_job` eligibility guard. Add an actual Dialog confirmation regression: B Retry recheck pending -> A prompt checked -> B recheck retires batch -> A confirm. Assert zero reply, intact A prompt/input and no retained candidate; a subsequent explicit eligible member-only answer must still be possible through the original owner.

**Spec result:1finding, Medium.** The C# protected-memory/first-prompt/batch-generation isolation and controlled picker/native paths pass their independently executed boundaries. Source recycling remains unavailable under its declared prerequisite; no real-picker or physical acceptance is inferred.

## Independent execution

Every Rust call used explicit1.96.0 selected by the workspace, offline mode, one Cargo lane and `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256` in the same invocation with full-operation timeouts. TEMP/TMP and new fixture outputs were confined to this session scratch.

| Check | Independently observed result |
| --- | --- |
| Exact Desktop directed names |57/57PASS,0ignored; includes26OPT-07-named and31original preservation cases. No broad448rerun is claimed. |
| Protocol library |105/105PASS. |
| IPC Hello/connection capability selection |3/3PASS. |
| Loaded-catalog matcher |1/1PASS. |
| CLI library |15/15PASS, synthetic bridge/owned temporary pipes/test subprocesses. |
| Desktop/CLI/archive/protocol/IPC strict all-target Clippy |PASS, `-D warnings`. |
| Current isolated-codecs archive worker |Build PASS; independently frozen current worker used for native fixtures. |
| C# Debug test-host build |0warnings/0errors, `--no-restore -m:1 -p:UseSharedCompilation=false`. |
| Controlled password/picker group |Exact LIST14/RUN14/PASS14/SKIP0/FAIL0/not_run0. |
| Actual worker/native group |Exact LIST5/RUN5/PASS5/SKIP0/FAIL0/not_run0; includes13actual empty-directory location scenarios and the five actual batch Test/Extract Runs. |
| Source/document contracts |Spec125documents/14owners, Kit catalog/release, pinned editor, CLI16, tree drag3 and Phase F ownership:PASS. Source/provenance scope only. |

Controlled14 uses original Session/Run/serialized tickets/input pump/SAME_PROCESS protected memory, plus owner HWND/parser/Pick/bounded STA/options/controlled modal returns and original strict picker. Actual5 covers native password diagnostics/failure, empty-directory location, real batch password reuse/other batch/retirement/generation2, and original encrypted ZIP lazy mixed-stream/ambiguity/revoke/cleanup. Exact LIST preceded identical RUN. The groups do not exercise a real system folder dialog or Explorer/OLE loop.

The native results retain byte-for-byte source checks, exclusive source/output reopen and actual owner drain. No skip or zero-selection result is counted as native acceptance. New Desktop findings do not invalidate these specific positive native facts, but prevent package closeout.

## Evidence, counts and projection observations

The delivery freeze contains787listed files:509source-context,31Debug runtime,176documents,70evidence and1input. Before this review's acceptance-document updates, all source/context, documents/evidence/inputs and other runtime artifacts matched their recorded origins directly. The only origin-byte difference was the newly rebuilt isolated-codecs worker, independently retained without overwriting the delivery freeze: current12,376,576bytes versus delivery10,516,992bytes. No source mismatch was found and no equality between those two differently built worker binaries is claimed.

Current frozen worker: `opt07-independent/Debug/NxTools.ArchiveWorker.exe`, SHA256 `CAABB44A73CE8AD0BF00A9A0C1C66EE62F590021A9935EEA712919D144223B3C`. The current rebuilt Host/test apphost and this explicit worker form the actual native execution pair. Existing identities are execution provenance, not new source-content authority.

Raw/index reconciliation confirms the historical broad run's424passing/24failing names, the directed57unique passing names and all24historical failure names within this review's fresh57passes. The original broad run remains its original result; individual failures having current same-name passes is not a new448/448result. All45coverage row identities are present; reconciliation is not acceptance of every row.

Retained projection data agrees with its raw lines:10000-entry Find performs10000catalog visits/query checks/matches with30virtual controls across three delivered observations; repeated current drawing has0catalog checks with45controls, while install/configuration records30000visits/normalizations,1folder build and2projections. Those current invariants are also asserted by the independently executed tests. The delivery's120003→0 repeated-render comparison remains its historical baseline attribution; no before binary or new Find speedup comparison was executed here. The three delivered Find timings are105829/108190/106994µs, not source-I/O or physical responsiveness evidence.

Evidence root: `D:\.pi-desktop\data\scratch\aab2f188-352d-4f1c-b7a3-5375fe0df2d8`. Logs and exact Desktop name list use `opt07-independent-*`; file/name/coverage reconciliation is `opt07-independent-evidence-audit.json` from `audit_opt07_acceptance.py`. Original saved C# progress/Create terminal inputs were replayed read-only, not reattributed as new publications. No original delivery evidence was overwritten.

## Gate status and stop point

| Boundary | Independent status |
| --- | --- |
| G-16 batch surface/password workflow |Partial: positive batch/native reuse evidence passes; O7-AC02 attempt-status and O7-AC03 current checked-consent boundary remain open. |
| G-17 Retry/Locate/Up |Tested baseline/fresh recheck, location facts/pending receipt and Up paths pass scoped; Retry summary still needs O7-AC02. |
| G-20 / touched G-26 |Positive matching/identity/virtualization/final-submit/refresh tests pass; warning Find action refusal remains partial under O7-AC01. |
| Touched G-15/G-19/G-25/G-27 |Executed original Stop/prompt/post-Test/Add/native lease/preview preservation remains green at its stated boundary; no complete overall gate closure. |
| G-28 |Implemented native adapter and controlled vector/owner/STA boundaries pass; real system multiselect/focus/keyboard/DPI remains pending. |
| G-18 / G-29 |Physical acceptance unperformed; original-object production recycle and Create-through-required-post-Test continuity unresolved. |

O7-AC01/02/03 require directed repair and original-boundary red-capable tests. This review does not authorize product/test fixes or another broad review pass. G-00, OPT-00/01 overall, complete G-04, overall Phase E and physical/Release remain open. No OPT-08 is added. No product GUI/Explorer/system clipboard/interactive system picker, user archive/source mutation/recycling/HZ5, service/registry, package/sign/install, reference-program execution/new reverse engineering, commit or push occurred. Standards:2Medium findings; Spec:1Medium finding.
