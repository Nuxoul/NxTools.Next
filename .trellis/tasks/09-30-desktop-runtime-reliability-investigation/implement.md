# Desktop Runtime Reliability Investigation Plan

> Investigation only. Do not implement product behavior changes in this task. At the end, present evidence, cause ranking, and the proposed smallest fix scope for explicit user approval.

## 1. Planning and context

- [ ] Read existing Quick Launch, clipboard, PlatformHost, IPC, and GPUI lifecycle specs/task evidence; do not duplicate completed `09-27` pagination/image-format work.
- [ ] Record exact launch route/build identity under test (packaged Debug/Release, protected copy, executable hashes, branch/commit) without publishing secrets.
- [ ] Confirm related worktree changes and preserve the current dirty UI changes and unrelated Trellis task directories.

## 2. Quick Launch flicker child

- [ ] Inspect the complete double-Control gesture -> platform event -> Rust summon/session -> GPUI show/first-frame -> results animation -> dismissal/hide chain.
- [ ] Define payload-free timestamp events/trace fields and a screen-recording/frame capture protocol; no query/result text or filesystem paths.
- [ ] Reproduce with repeated double-Control open/dismiss cycles, including fast typing/results arrival and immediate second dismiss; capture a synchronized recording and lifecycle trace.
- [ ] Rank falsifiable causes and identify the first boundary where visible state diverges.
- [ ] Add/run a red-capable lifecycle regression gate if a valid seam exists; clearly separate source/unit guarantees from the human-visible frame oracle.
- [ ] Document fix options and stop for user review before changing animation/window behavior.

## 3. PlatformHost process count and resource child (existing task)

- [ ] Map all matching processes by canonical image path/hash, PID, parent PID, command line, session, start time, and associated desktop instance/pipe.
- [ ] Explain each process as protected, packaged, development, manual, supervised child, orphan, service/session process, or unrelated lookalike; never kill by name.
- [ ] Capture a bounded idle baseline and controlled active/clipboard-burst samples for CPU deltas, private bytes/commit, working set, handles/threads, process restarts, and clipboard/event throughput.
- [ ] Correlate resource slopes with supervisor restart logs, capture/publication tail admission/drops, event channel/pipe reconnects, and measured activity.
- [ ] Repair/reuse the existing interactive CPU/working-set harness as needed; record the exact command, environment, duration, and package hashes.
- [ ] State whether high resource use reproduces, which PID/path owns it, and what evidence distinguishes growth from cache/working-set behavior.

## 4. Missed clipboard capture child (existing task)

- [ ] Design a controlled copy matrix: Unicode text, file copy (`CF_HDROP`), PixPin screenshot, browser `Copy image`; include normal and controlled load conditions.
- [ ] Add/run a payload-free correlation/sequence ledger across native update/read, retries/exclusions/limits, canonical rewrite, staging/publication, bounded channel, pipe write/read, Rust worker/store/dedup, and UI refresh generation.
- [ ] Record the first missing transition for each miss and preserve native error codes/reason codes without payload data.
- [ ] Build a deterministic red-capable replay/stress seam at the confirmed loss boundary; if impossible, document exact manual repro steps and missing seam.
- [ ] Preserve PixPin/browser canonical image rewrite and safe custom formats/`CF_HDROP`; do not change behavior until user approves the evidence-backed proposal.

## 5. Cross-symptom synthesis and review

- [ ] Produce one evidence matrix with symptom, exact repro, build/hash, PID/HWND/session, timeline, metrics, first failed transition, ranked causes, confidence, and remaining gaps.
- [ ] Test whether the symptoms share a cause; do not assume correlation because they occur on the same workstation.
- [ ] Present findings and the smallest proposed fix boundary to the user; wait for explicit approval before creating/starting implementation work.

## 6. Validation and cleanup

- [ ] Run relevant existing regression suites and UI/native lifecycle gates for any instrumentation/harness touched.
- [ ] Check `git diff --check` and verify unrelated UI edits and untracked `09-28-*` directories remain untouched.
- [ ] Remove temporary probes/DEBUG markers from production sources; retain only approved payload-free tests/tools and investigation evidence.
- [ ] Update the existing clipboard task with newly verified evidence and keep original completed work/results intact.
