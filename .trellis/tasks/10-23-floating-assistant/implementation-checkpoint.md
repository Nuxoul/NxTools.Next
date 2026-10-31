# Floating assistant implementation checkpoint — 2026-10-23

## Current status
Canonical current phase: **Phase D — Integrated verification and handoff**. The settings-file editing/compilation blocker is **resolved** after the user requested continuation in a new turn. Phase A owned native feasibility, Phase B source integration and Phase C existing-owner adapters now have compiled production-boundary regression evidence. This is not installed, physical, visual or user acceptance. The task remains `in_progress` while those acceptance levels and the unrelated Quick Launch source gate remain open.

The original blocker was three failed Edit attempts, ending in `EDIT_NO_CHANGE`, followed by five new settings UI compile errors. In the authorized continuation, fresh anchors were read and the missing Focusable import, sibling-test visibility, explicit selector-array type and AnyElement conversion were fixed. `cargo check -p nxtools-desktop --tests` passed; none of these five errors remains. Existing work was resumed, not reset or rewritten.

## Implementation delivered locally
- Dedicated serde-default Core configuration, typed action choices, normalization and locked latest-disk section save. Default disabled; no unrelated configuration overwrite.
- Official 180×160 Other Tools tile and separate Small outlined Configure action; keyed 460×320 feature settings shell with real Form/Field, 32-DIP NumberInput/Select and controlled Switch. Configure does not toggle; settings last control is reachable by the actual scroll owner.
- One Rust controller/session and retained hidden GPUI renderer. Pending precedes dispatch; native final client readback precedes current-size frame and explicit passive show. Disable, disconnect, timeout, retirement and stale acknowledgements cannot reveal an obsolete generation.
- Bounded off-render PNG/JPEG/BMP/WebP/GIF decode on the existing pinned image 0.25.10, real GIF disposal/transparency/source-delay fixtures and one monotonic scheduler. Source/frame/retained-memory budgets, 1–60 FPS display cap and approved 0.25–4×/zero-speed/baseline mapping remain explicit. Static, disabled, hidden and reduced-motion playback has no frame timer.
- Download projection reads guarded live tasks/latest progress; only currently Downloading speed contributes. Unknown total is `None`, not a fake percentage. Known progress uses the actual official ProgressCircle; active count remains visible when progress is unknown.
- Click/drop actions reuse normal Add Download prefill/probe/confirmation, original Quick Launch input/session/query generation, original main visibility/activation native commands and copy-only PasteImage. No silent download submission or remote fetch solely to copy an image.
- One bounded pending action, original operation correlation and once-only drop IDs. Main activation waits for a matching successful visibility reply. Deferred actions/image preparation are cancelled before dispatch on disable/click-through/retry. Already admitted native copies retain normalized image ownership until actual reply/deadline; cancellation cannot retract an admitted Windows operation and does not claim that it can. Retired/cancelled replies do not display stale success notices.
- C# owns one bounded 16-command fixed STA/OLE coordinator, authenticated source/renderer/session/generation, input-only NOREDIRECTIONBITMAP passive circular receiver and exact operation staging. Accepted staging survives hide until release; retire/disconnect/rejected delivery clean it. OLE capture and Rust event share correlation. Capture loss emits terminal move without click.
- Explicit optional wire `edge_hidden` defaults false. Prepare/ordinary updates fully clamp to work area; only intentional subsequent edge-hide retains eight device pixels. Existing bounded 500 ms observer reports changed monitor/work-area/DPI/fullscreen signature and retries failed publication. Rust reveals/reapplies a recoverable anchor before continuing. Physical display changes were not performed.
- No additional GPUI/framework patch, new engine, global hook, cross-process WndProc injection or SetParent. Cargo.lock unchanged by this slice. The current Kit 0.8.0/pre 0.3.9 graph retains exactly the approved two framework deltas.

## Verification actually performed
All Rust commands inherited `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`; Cargo operations were serial.

| Check | Result |
| --- | --- |
| Final focused Desktop floating-assistant tests | **41 passed, 0 failed**, after decode-disable/re-enable cache repair |
| Final full Desktop suite | **1091 passed, 1 failed**, unrelated new archive password-drag test at browser_drag_tests.rs:556; earlier pre-concurrent-change suite passed1088/1088 |
| Core full library tests | **82 passed, 0 failed** |
| Protocol full library tests | **74 passed, 0 failed** |
| Strict Core/Protocol/Desktop test Clippy | Earlier PASS; latest FAIL only archive/password.rs:139 `collapsible_if` introduced by concurrent archive work; no lint suppression or unrelated fix |
| `cargo check --workspace --tests` | Latest **PASS** after cache repair |
| Format | Task-owned files **PASS**; global fmt earlier PASS, latest FAIL only concurrent archive/application.rs:845, archive/browser.rs:1862, archive/browser_drag_tests.rs:523 |
| PlatformHost build (native delegate) | **PASS**, zero warnings/errors |
| Final exact safe C# union `floating input fixture` + `floating coordinator fixture` | **13 selected/run/pass, 0 skip/fail/not_run** |
| Separately authorized exact `floating input native actual owned OLE routing` | **1 selected/run/pass, 0 skip/fail/not_run** |
| New `tests/architecture/floating-assistant-lifecycle.py` | **32 source contracts PASS** (final handoff rerun) |
| Download UI, Desktop shell, UI state, UI motion, UI consistency source gates | **PASS** |
| Quick Launch lifecycle source gate | **FAIL**, existing elevated-navigation contract at line 1765, outside assistant-owned files |
| Spec, component catalog, GPUI release, Markdown editor provenance gates | **PASS**; spec/task/diff rerun after documentation refresh |

Red/green evidence retained: original three action failures passed after repair; three cancellation regressions captured missing release/premature image deletion/retry ordering before their fixes. Unknown-progress/count test failed before presentation repair. Final decode-disable/re-enable test failed on stale loaded-cache ownership, then passed after clearing cancelled cache, suppressing disabled decode and reloading on enable. Full Desktop was rerun only after this concrete fix; its final unrelated archive failure remains a failure, not erased by earlier1088pass.

An earlier native fixture intermittently failed its baseline renderer-pixel assertion before receiver creation. The owned pixel fixture was hardened and the final exact combined safe set plus OLE test passed; the earlier setup failure is historical evidence, not erased. Full C# fixtures were not run; only reviewed exact selections were executed.

## Remaining boundaries / blockers
1. Final full Desktop suite is **1091pass/1fail** in `archive::browser::drag_tests::production_password_drag_unlock_success_requires_fresh_gesture_and_revokes_old_authority` (`archive/browser_drag_tests.rs:556`, observed1 expected4). Latest strict lint fails only `archive/password.rs:139` collapsible-if; global formatting fails only concurrent archive files listed above. These changed externally during this turn. An intermediate transient archive/password.rs delimiter error cleared without our editing that file. Preserve all archive work; do not fix or globally reformat it in this task. Existing `quick-launch-lifecycle.ps1:1765` elevated-navigation gate also remains failing. Current full worktree is not green.
2. Real GPUI layered opacity/transparent corners/input hit testing, physical browser/Shell text/image drag, mixed-DPI/monitor unplug/reconnect, live OS reduced-motion toggle and user visual/accessibility acceptance are **not verified**. Owned synthetic HWND/OLE and GPUI TestAppContext results are not substitutes.
3. The one optional bounded read-only review was cancelled after more than14minutes without a report. It is **not passed**; no second review was started. Parent performed concrete source-boundary checks and regression repairs described above.
4. No normal product launch, packaging, installation, service/registry operation, actual clipboard/user history/configuration mutation, commit or push was performed. These remain outside the current authorization.

## Reproducible evidence
Latest logs: `floating-focused-post-cache.log` (41pass), `floating-desktop-post-cache.log` (1091pass/1archivefail), `floating-clippy-post-cache.log`, `floating-workspace-post-cache.log`, `floating-format-post-cache.log`, `floating-decode-reenable-red2.log`; previous compiled/native/doc/source evidence remains in `floating-*-final.log`, `floating-native-safe-final.log`, `floating-native-ole-final.log`, `floating-source-gate-results.json` and `floating-doc-*.log`.

Next continuation should start with this current Phase D checkpoint, not the historical settings-file guard. Do not restart Phase A, renumber phases, reset the worktree or duplicate a surviving Cargo process.

Final process observation: another workspace task was running `cargo test -p nxtools-desktop --bin nxtools-desktop production_password_drag --jobs 1 -- --test-threads=1` (observed Cargo78904/rustc64584). This is not a surviving assistant verification command. It was not stopped; do not start another Cargo operation until that build finishes. All static handoff gates, task context validation and scoped diff checks passed; no temporary DEBUG markers were found in task-owned Rust/C# files.
