# HZ2 — Truthful, readable task progress

[Research authority](../hatchzip.md); external root `D:\Code\hatchzip-main`; [source hashes](source-manifest.json). Inspected state/render/test source only, no live GUI run or screenshot acceptance.

## Source and call chain

| Source | Behavior inspected |
| --- | --- |
| `crates/gui/src/work.rs:18` Counters / `:91` Update | atomic done/total/archive/stop/reading, low-frequency typed transitions |
| `crates/gui/src/work.rs:312` Run wiring | on_bytes increments processed count; on_commit changes remaining archive size |
| `crates/gui/src/runner.rs:19` TICK / `:177` launch |100ms UI sampling; worker sends state through event loop, no per-write messages |
| `runner.rs:242` on_update | sample once before transition, apply state, advance only completed queue item, stop timer at terminal |
| `crates/gui/src/task.rs:139` apply / `:271` tick | stage-specific heading/actions, clear stale speed, render numbers only while working/verifying/pausing |
| `crates/gui/src/text.rs:35` duration / `:149` Rate | coarse readable ETA;5second sample window; >=1second history before estimate |
| `crates/core/src/engine.rs:1722` human | decimal B/KB/MB/GB/TB; do not silently mix with NxTools binary units |
| `crates/gui/ui/parts.slint:152` Progress / `:181` Outcome / `:204` TaskButtons | shared progress/outcome/actions used in main and task windows |
| `crates/gui/ui/task-window.slint:6` TaskWindow |480x270preferred/minwidth420; state-dependent body, bottom actions |

Run callbacks update atomics -> Runner timer -> Task::tick(now,counters) -> TaskView -> shared Slint parts. State events go work::run -> invoke_from_event_loop -> Task::apply. Reuse this *separation*, not its Slint components or thread-local singleton.

## Actual user-visible semantics

Stages: checking/verifying/working/pausing/paused/done/failed/no-space. Pause request changes heading immediately but says 'pausing' until worker reports Paused. Finished extraction says whether source was kept/removed and enables Open folder. Failed state clears rate and conditionally says progress saved. Checking a long archive changes to reading percent after500ms when a reliable pass fraction exists; it is pass progress, not overall extraction progress.

Rate keeps samples from about the last5seconds, computes delta bytes / monotonic elapsed with >=1second history. No rate yet => empty label; <1B/s => waiting-for-disk message. Stage changes clear samples. ETA uses remaining/observed speed; time formatting rounds up minutes and coarsens short waits. These are reference UX choices, not measured guarantees. Reference has known total and `max(1)` for its calculation; do not copy that for unknown totals in NxTools.

Counters.done counts processed output, not all durably published output. `archive` is separate and generally changed by source commit; allocation freed by punch and logical file length are distinct. Do not label decoded progress as committed output or show logical source length as physical disk-space reclaimed.

The reference task window still has fixed preferred height and a stretch Rectangle. It is NOT proof of an optimal compact layout. Borrow hierarchy and conditional sections, not blank-area dimensions.

## NxTools adaptation

Current re-entry: `rust/apps/nxtools-desktop/src/archive/progress.rs::JobProgressView` receives ProgressSnapshot from `jobs.rs`; `rust/crates/nxtools-protocol/src/archive_job.rs` supplies real progress/terminal fields. At inspection the window still printed raw bytes/ms and both progress and terminal lines. Other dialogues may be fixing it: reread latest source before editing.

- User-approved2026-10-28 display policy supersedes prior binary display advice: shared archive formatter uses1000-based B/KB/MB/GB/TB and /s, including browser/space/progress/preview limits. Raw byte budgets and persisted split-size MiB input are unchanged. Historical reports remain historical.
- Running: actual phase, processed amount/known total or unknown, smoothed rate; ETA only for reliable remaining work. Queue waiting/password/conflict/paused must not retain an advancing ETA.
- Completion: terminal committed file/byte counts, skipped/warnings/elapsed are primary. Hide stale '0/unknown' processed-file line rather than fabricate values or rewrite counters.
- Partial failure/cancellation: preserve committed results/journal and truthful cleanup/unknown state; do not show a success100% merely because worker exited.
- State/layout remains Rust, official GPUI Progress/Alert/DescriptionList/Buttons and existing Root/TitleBar owners. Content-sized/minimum-bounded layout, body scroll only when necessary, reachable footer, no duplicate title bars.
- Sample coalesced original counters/events; bound sample count as well as time (reference relies on its100ms timer rather than a hard sample cap), no per-byte logs/IPC, no new global notification clock, no timer when idle. UI close/background semantics must not become cancellation accidentally.
- Keep progress-window preference persistence in original configuration owner, not a second file. Stage metrics must carry original logical job/attempt/generation; stale attempts cannot regress current display or reset latest speed.

## Tests inspected / future red-capable gates

Reference: gui/text.rs tests for duration/rate/plan labels, gui/task.rs tests for running/pausing/terminal and reading delay, gui/work.rs actual temporary archive/update sequence tests. UI source read, no visual tests executed here.

HZ2-A: fake monotonic time through production presentation state, startup insufficient samples,5s window/zero speed/reset/pause/resume and stale event rejection. HZ2-B: known/unknown/zero total, processed!=committed, partial success/failure/cleanup unknown, completion summary. HZ2-C: actual GPUI render at min/default size, Light/Dark/long labels, compact successful and longest error states; footer accessible/no huge empty placeholder/no clipping. HZ2-D: buttons dispatch exactly once, closing keeps original job lifecycle, no idle timer; throwing diagnostic sink cannot alter task result. Physical input/DPI/user visual acceptance remains separate.

2026-10-28 decisions: speed basis is original processed output bytes, never committed bytes; elapsed is monotonic logical wall time, sampling is scoped to active attempt/generation and excludes waits by clearing/reseeding. Unknown/zero total suppresses ETA/percent. Existing JobProgressView retains one titlebar, scroll body and reachable fixed footer; physical visual acceptance is separate.

## Current-file and overall extraction progress

The approved Bandizip-shaped extraction view uses two official Medium Progress controls. Native reference chain and reconstructed function evidence are in [archive §1.1](../archive.md#11-selected-entry-runtime-and-native-follow-up). Ordinary running extraction uses the original window title rather than repeating a second stage heading; checking, waits, pause, reclamation and terminal states retain explicit status. Both groups, file count/elapsed and original fixed controls fit the560×260 default;480×260 minimum and long-name behavior have production-render regressions. Filename display is single-line with the full accessible label.

`OperationControl` retains one coherent snapshot of selected-plan totals, current member and current decoded bytes. The shared worker output begin/write/commit boundary resets member bytes, counts decoded output, and increments files only after native commit. Existing100ms writer/frame-boundary coalescing and latest-only C# forwarding remain. `ArchiveJobProgress.current_bytes_done/current_bytes_total` are optional; absent legacy data remains unknown. The UI never derives member bytes by subtracting lossy aggregate samples. Skip denominators remain pre-conflict selected-plan totals; neither a skipped job nor terminal forces100%. Speeds/ETA remain aggregate-only and processed is still distinct from committed output.

Evidence: `repo:rust/crates/nxtools-archive/src/control.rs`, `repo:rust/crates/nxtools-archive/src/worker_output.rs`, `repo:rust/crates/nxtools-archive/src/worker_job.rs`, `repo:rust/crates/nxtools-protocol/src/archive_job.rs`, `repo:csharp/NxTools.PlatformHost/Protocol/ArchiveJobContract.cs`, `repo:rust/apps/nxtools-desktop/src/archive/progress.rs`. Exact checks and known performance/native gaps: [repair outcomes](../../../.trellis/tasks/10-31-archive-research/outcomes.md).
