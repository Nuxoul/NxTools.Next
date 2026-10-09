# Investigation: Quick Launch Flicker and Desktop Clipboard/Runtime Reliability

## Goal

Identify evidence-backed causes for three user-visible reliability problems without changing product behavior during the investigation:

1. The Quick Launch results surface visibly flashes while opening or dismissing through the double-Control gesture.
2. More than one `NxTools.PlatformHost.exe` can appear, and PlatformHost CPU/private-memory usage can become unusually high after the desktop runs for a long time.
3. Clipboard history intermittently misses copied content.

The user will review findings and any proposed fix scope before implementation begins.

## Confirmed Facts

- Quick Launch is a native Rust GPUI 0.2.2 window. `rust/apps/nxtools-desktop/src/quick_launch/panel.rs` conditionally mounts `result_surface` when `results_visible` and keys its entrance animation by `results_appearance_generation`; the animation currently lasts 120ms through `RESULTS_APPEAR_DURATION` (`rust/apps/nxtools-desktop/src/quick_launch.rs`). The user reports a perceptible flash during double-Control open/close, but no reproduction recording or event trace is attached yet.
- The packaged desktop chooses one PlatformHost path in `platform_host_path` and starts it from one `PlatformHostSupervisor` in `DesktopRuntime` (`rust/apps/nxtools-desktop/src/runtime.rs:311-415`). The child receives `--parent-pid` and is restarted if it exits (`runtime.rs:417-456, 593-606`). A single launch site does not prove that only one process exists on the machine: protected and development copies, orphaned children, manual launches, and another desktop instance still need to be distinguished by PID, executable path/hash, parent PID, command line, start time, and session.
- PlatformHost registers one Windows clipboard format listener in `WindowsPlatformAdapter.EventWindow` (`csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs:166-173`), receives `WM_CLIPBOARDUPDATE` and calls `TryPublishClipboardContent` (`:394-403`), retries selected external clipboard read failures up to five attempts (`:637-640`), bounds its publication tail at 64 pending events (`:54-55, 759-785`), and logs capture completion/skips/drops without clipboard payloads.
- The platform event channel is bounded (512) and uses `FullMode.Wait`, one reader, and multiple writers (`csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.cs:67-73`). Forwarding/drop outcomes are handled in the same adapter (`:116-205`). These controls do not yet identify whether the reported missed copy is lost at the Windows notification, native read, capture decision, publication queue, IPC boundary, Rust worker/storage, or UI refresh.
- Existing `.trellis/tasks/09-27-clipboard-performance-platformhost-crash/` already contains clipboard-history performance work, PlatformHost exception hardening, and a PixPin/browser image compatibility fix. Its measured deterministic database and bounded UI-window work is recorded, but its interactive packaged CPU/working-set soak and live runtime evidence remain open. This task reuses that task rather than duplicating it.
- Rust owns Quick Launch UI/state and clipboard history/storage; C# PlatformHost owns Win32 clipboard access, native event handling, and IPC publication. Keep that boundary intact.
- Current worktree contains unrelated uncommitted UI and generated-manifest changes plus three pre-existing untracked `09-28-*` Trellis directories. Preserve all of them; do not stage or overwrite them as part of planning or later investigation.

## Requirements

### R1 — Quick Launch flicker

- Reproduce the exact double-Control open/dismiss interaction at the real GPUI/native window boundary, including short queries that populate results quickly and cases where dismissal overlaps result arrival/animation.
- Capture a time-correlated screen recording or frame sequence plus payload-free lifecycle timestamps for summon, native show/activation confirmation, result visibility/generation, animation start/end, and dismiss/hide. Do not log query text or result contents.
- Determine whether the visible flash is an entrance/removal animation artifact, stale result generation, native show/activation ordering, or overlapping open/close lifecycle. State which evidence supports or rejects each candidate.
- Establish a deterministic/red-capable regression seam. If an automated GPUI/native test cannot observe the user-visible flash, document that limitation and provide a repeatable human capture procedure with bounded, generation-scoped instrumentation rather than claiming a source-only test proves the visual behavior.

### R2 — PlatformHost process count and resource growth

- Reproduce the user's runtime topology without killing unrelated processes. Sample each matching process by PID, full executable path, approved/runtime hash, parent PID, command line, session, start time, CPU delta, private bytes, and working set over a documented idle/active interval.
- Distinguish the desktop-supervised packaged/development child from protected copies, manual launches, stale/orphaned processes, and service/session copies. Explain why each process exists and which one owns clipboard listening.
- Determine whether CPU/private-memory growth correlates with clipboard update rate, clipboard rendering/icon work, publication-tail depth, IPC reconnects, event queue backpressure, helper activity, repeated PlatformHost restarts, or another workload.
- Measure baseline and soak using a packaged Debug or Release desktop appropriate to the user's normal launch route. Do not infer a leak from working-set size alone; include private bytes/commit and process lifetime. Never terminate by process name alone.
- Preserve clipboard privacy: diagnostics may include counts, sizes, event kinds, elapsed time, correlation IDs, process identity fields, and native error codes, but never clipboard text, payload bytes, file paths, window titles, credentials, or raw native data.

### R3 — Intermittently missed clipboard captures

- Build a repeatable clipboard copy matrix across plain text, file copy, PixPin screenshot, and browser “Copy image”; include source apps and timing/load conditions where the user observed a miss.
- Trace one correlation/sequence identity through `WM_CLIPBOARDUPDATE`/retry, native capture decision, staging/publication queue, bounded PlatformHost event channel, PlatformPipeClient write, Rust event receive, clipboard worker/storage commit, and visible history refresh.
- For each attempted copy, produce a payload-free outcome ledger that distinguishes notification received, content recognized, excluded/limit-rejected, rewrite/retry, publication queued/dropped, IPC accepted/failed, Rust received, storage committed/deduplicated, and UI refreshed/stale.
- Establish a red-capable test or harness at the narrowest real seam for any confirmed loss point. Preserve safe native formats and `CF_HDROP` compatibility. Do not alter filtering/retry/queue behavior until evidence identifies the failing boundary and the user approves the proposed fix.

### R4 — Integration and user review

- Reuse and update the existing clipboard/PlatformHost task for R2 and R3; do not duplicate its completed cursor-pagination or image-normalization work.
- Keep Quick Launch flicker in its child task so it can be reproduced and accepted independently.
- The parent owns a concise cross-symptom evidence matrix and final cause ranking. Correlation is a hypothesis to test, not an assumption.
- Before implementation, present findings, reproduction evidence, ranked causes, and a narrowly scoped proposed fix for user approval.

## Out of Scope

- Changing UI animation timing, window lifecycle, clipboard capture/retry policy, event queue capacity, process supervision, or logging behavior before investigation findings and implementation scope are approved.
- Broad UI redesign, clipboard feature work, database/search redesign, or unrelated PlatformHost cleanup.
- Terminating processes based only on executable name, changing protected install/service state, or modifying the user's existing install during evidence collection without a safe, explicit procedure.
- Logging clipboard contents, file paths, window titles, credentials, or raw payloads.
- Claiming a managed exception root cause without faulting-module, stack, dump, or matching WER evidence.

## Acceptance Criteria

- [ ] Quick Launch double-Control flicker is reproduced with a timestamped visual artifact and correlated lifecycle trace, or the precise environmental blocker and repeatable user capture steps are documented.
- [ ] The visual symptom has a deterministic regression gate where technically possible; any human-only visual assertion is explicitly separated from automated lifecycle assertions.
- [ ] PlatformHost process inventory explains every observed matching process using path/hash/parent/session evidence, with no process killed by name alone.
- [ ] Idle/active/soak samples include CPU deltas and private-bytes/working-set time series and identify whether the high-resource report is reproducible and which process/workload drives it.
- [ ] Clipboard copy attempts have a payload-free end-to-end outcome ledger that locates misses at a named boundary or explicitly records an unresolved evidence gap.
- [ ] At least one unattended red-capable regression harness is run against the confirmed clipboard-loss or resource-growth seam; if no automated seam exists, the gap and manual protocol are explicit.
- [ ] Existing `09-27` task artifacts are updated with new clipboard/PlatformHost evidence without duplicating or undoing completed fixes.
- [ ] No product behavior was changed during investigation; proposed fixes are presented for separate user approval.
- [ ] Targeted tests, scripts, and diff checks for any investigation tooling pass, and unrelated dirty worktree changes/directories remain untouched.

## Decisions and Deferred Items

- Treat the three symptoms as separate until evidence proves a shared cause.
- The current user report supplies the trigger descriptions, but exact application list, frequency, and a user-side screen recording are not yet available. First use existing local code/log/process evidence and prepare a safe capture protocol; ask the user only for evidence that cannot be collected from this machine.
- Initial resource acceptance is based on reproducible slopes and boundedness versus an idle baseline, not a guessed absolute memory threshold. Absolute thresholds can be agreed after sampling.
- Investigation is the current deliverable. Any behavior fix requires a new reviewed proposal and explicit approval before `task.py start` for the relevant implementation child.

## Task Map

- `09-30-quick-launch-result-panel-flicker` — new child: visual repro, lifecycle trace, and regression seam for double-Control result reveal/dismiss.
- `09-27-clipboard-performance-platformhost-crash` — existing child reused: extend with duplicate/high-resource PlatformHost and missed-capture runtime evidence.
- `10-01-one-time-uac-trusted-updates` — new child: trace repeated UAC triggers and design a user-approved trusted silent update path; retain fail-closed protected deployment.
