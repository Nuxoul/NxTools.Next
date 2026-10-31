# HZ3 — Batch task summary without replacing scheduling

[Authority and evidence limits](../hatchzip.md); reference paths relative to `D:\Code\hatchzip-main`, [hash manifest](source-manifest.json). Code/test definitions inspected; no CLI/GUI execution.

## Source map

| Entry | Inspected behavior |
| --- | --- |
| `crates/cli/src/main.rs:233` distinct | session::locate -> first-volume absolute path -> HashSet; one split archive counts once |
| `main.rs:501` extract_all | precheck named archive existence, install Ctrl+C once, serial per-archive extract, stop on first noncompletion |
| `main.rs:578` summary | completed / current paused-or-not-extracted (+saved) / remaining not-started; output even quiet |
| `main.rs:247` each | list/test/check-style iteration continues through failures; returns first nonzero status |
| `crates/gui/src/task_window.rs:39` job_for / `:77` build | original menu target strategy + saved choices -> jobs -> one task-window Runner |
| `crates/gui/src/runner.rs:103` start_queue / `:115` step / `:136` next_in_queue | VecDeque, position/total, next only when phaseDone |
| `runner.rs:69` give_back / `:219` may_close | abandoning current destructive task clears rest; closing active write asks pause before quit |
| `crates/gui/src/work.rs:228` extract | new space plan immediately before each job starts, not one batch-wide stale observation |
| `crates/gui/src/args.rs` | task request inputs; not evidence of a Windows whole-selection singleton |
| `crates/cli/tests/several.rs` | real CLI multiarchive outputs, keep-source, failure summary, missing archive and volume dedup scenarios |

## Verified behavior and caveats

CLI extraction is serial. A missing later path is caught before consuming the first archive. Each item is re-opened/replanned before starting; source space freed by a prior item is available to a later one. On failure/pause extraction stops and returns that status with an itemized summary; list/test/check use a different continue-all policy. GUI queue likewise advances only after Done; failed/paused/no-space holds remaining work until retry completes. Its compact step label can say 'Archive2 of3;1not started'. It is not a full persistent task-history center.

Dedup uses located primary *path*, not stable Windows file identity or case/alias/hardlink equivalence. Do not overstate it as object-safe dedup. README/TODO says Windows Explorer can still spawn one app per file; queue support within a request does NOT prove whole-selection forwarding/single-instance behavior. NxTools has its own single-instance/selection helper; retain it.

A right-click chosen task starts after the window is shown. This reference treats that as consent to its selected mode; NxTools destructive HZ5 must require its own explicit trustworthy intent, not infer consent from arbitrary shell args or reuse ordinary extract menu.

## NxTools design seam

Reread `rust/apps/nxtools-desktop/src/archive/jobs.rs::{submit_batch,validate_batch,submit_job,pump_jobs,job_terminal}` and `queue.rs::JobQueue`. At inspection admission allowed2running/32queued and2GiBaggregate working-memory budget; queue head blocks on overlaps. Do not replace it with HatchZip's serial Runner or change fairness/concurrency silently.

Add batch identity/membership/status projection around original logical jobs. Distinguish batch_id, logical_job_id and active operation/attempt IDs. Original finite readmission may run a new attempt for one logical job; it must not double-count a member or generate duplicate terminal summary. Queue acceptance is not started; worker terminal without cleanup proof is not a free execution slot. Retry after partial output must preserve actual commit facts and original qualification.

A parallel batch needs counts such as queued/running/awaiting-user/completed/warnings/failed/cancelled/not-started, not a misleading single sequential position. If UX wants stop-on-error, define what happens to already-running siblings and future queue members; do not automatically terminate siblings, block unrelated batches or discard completed outputs. First implement summary without changing existing task policy unless explicitly approved.

Preserve original raw per-job outcomes; overall aggregate must not say success when a member is failed, cleanup unknown or partially committed. A known failed logical job does not become 'not started' because its next retry was never admitted. Report partial submissions explicitly; decide atomic batch admission versus per-item rejects using existing validate_batch behavior. Unknown total bytes precludes fabricated aggregate percent; file totals and archive-job totals have separate labels.

GUI summaries should offer original Open output/Show error/Retry eligible task through existing commands, not arbitrary file paths from untrusted terminal JSON. CLI product output may include requested task paths; structured diagnostic logs must remain payload-safe. Durable cross-restart batch history is an additional design decision, not implied by summary.

## Tests to follow and add

Reference definitions: `crates/cli/tests/several.rs::{extracts_several_archives_one_after_another,several_archives_can_all_be_kept,a_failure_in_the_middle_stops_with_a_summary,a_missing_archive_is_refused_before_anything_starts,volumes_of_one_split_archive_count_once}`. Read `layout.rs::a_colliding_archive_among_several_is_refused_untouched`. No test executed here.

HZ3-A: submit3original jobs,2concurrent, one waiting, then fail/pause/cancel middle and assert every member's exact state/count; unrelated jobs unaffected. HZ3-B: split-volume aliases count once only after original source-set recognition; same basename in different directories remains separate. HZ3-C: prompt wait, finite readmission and repeated/stale terminal do not inflate done count; unknown cleanup retains slot. HZ3-D: queue-full/memory/duplicate/target-overlap/partial submission and changed target capacity at actual start. HZ3-E: batch completion summary reflects successful-with-warning/partial output/failed/never-started accurately; all-cancelled is not success. HZ3-F: actual CLI exit/JSON contracts and original launch/whole-selection routes remain valid; no duplicate worker per Explorer request.

## Decisions before implementation

Existing scheduler policy versus explicit serial batch, stop/continue-on-error behavior, retry eligibility, retention/durability of summary and aggregate progress denominator. These are future product decisions; do not ask the user to reconfirm repository facts already recorded above.

2026-10-29 Step5 implementation recheck: actual main.rs233/501/578, GUI runner69/103/115/136/219,
task_window39/77, work228 and several.rs definitions reread against this same source-manifest.
NxTools integration now traces original submit_form additional_requests, submit_batch defer_pump,
send_job_attempt/finish_attempt post-create-test, readmission::Delivery/Finished, job_terminal64
retirement, JobProgressView and job_location::locate_job_result. Original ArchiveSpacePreflight
already calls ArchiveVolumeSources.Open with held native source-set recognition; any batch
dedup/retry observation must reuse that boundary, not copy HatchZip's primary-path HashSet or
guess source identity by basename/case. These are source routes, not runtime acceptance. Scope
and red-capable seams are recorded in the task's hz3-route.md before implementation.

2026-10-29 HZ3-AC01–03 repair source recheck: `ArchiveOutputSession.CreatePreparedRoot` retains
created roots; `TrackDirectory` creates ancestry and `EnsureDirectory` marks completed directory
requests outside file-only `CommitJournal`; `ResultLocation` preserves directory-only output.
`ArchiveWorkerRunner.ExecuteAsync` freezes native facts after real cleanup then `ResolveTerminal`
and `ArchiveJobSession` project them. Rust `job_notification` delivers native Terminal before
readmission `Delivery::Finished` publishes logical terminal; finite attempts replace operation ID
without changing generation. `job_terminal` pops original retired IDs, while
`batch_retirement_held` may still protect the record. Repair uses these original owners, no
HatchZip scheduler/recovery import. Parent native and extracted-body counterexamples and evidence
limits are immutable in `hz3-acceptance.md`; actual-owner red/green route is recorded separately
in `hz3-ac01-ac03-repair-route.md` before product changes.
