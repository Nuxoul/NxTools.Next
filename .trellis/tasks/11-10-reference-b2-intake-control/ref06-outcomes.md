# REF-06 — Deliberate speed-limit submission

## Original owners

The original global `DownloadSettingsView::speed_limit_kib` and task-window entity retain stable official InputStates. `SpeedLimitEdit` holds only presentation draft/applied/pending/error state. Input Change never changes a limiter. Complete Enter/Blur submits once through the original correlated global command or `DownloadTaskAction::SetSpeedLimit`; Escape restores the applied value. Explicit KiB/s and0-unlimited retain the original `Option<u64>` representation.

Original SQLite settings/task persistence precedes limiter publication and TaskChanged. The existing global/task controls and background runtime minimum ceiling remain authoritative. Original waiter revision signalling, HTTP/segmented/HLS/DASH runners, cancellation and task identities are unchanged. No limiter, scheduler, controller, transport or store is added.

Exact positive-byte limits use ceil-KiB presentation only. Untouched Settings reopen/unrelated Apply preserves `SpeedLimitEdit.applied` bytes; explicit edits retain checked KiB conversion. Compact task summaries omit request-only limits. Task-window creation uses the original full projection; every update reads current cap/probe through `DownloadEngine::task` outside render while retaining the original compact status/progress/artifact snapshot and phase lifecycle. Original queue eligibility and completed-artifact handling remain. Clean stable inputs and the effective label track current applied authority, including external typed commands; dirty/pending/error text survives. Unavailable full reads retain the last applied rate. Reopen restores persisted intent.

## Production interaction and lifecycle

Official Kit/Base0.8.0 Input uses Change/PressEnter/Blur/Focus subscriptions and the public input Escape action. Original input overlays/IME handle their own Escape first. The existing field shell forces Medium bordered/focus-bordered Input; a96DIP input and Styled Label with explicit units/0-unlimited fit the original32DIP detail row. Original440×184 collapsed and440×280 expanded windows, titlebar/footer/disclosure and segment table remain.

Cross-entity submission and result publication are deferred beyond the current view lease. Global commands check the original settings window. Task commands check original task ID, window handle, view entity and generation. A retired source cannot mutate the task. Pending UUID suppresses Enter-followed-by-Blur duplicates; unchanged values are no-ops. Invalid/empty/negative/fractional/overflow text and failed persistence preserve applied intent and retain an actionable local error/draft.

## Behavioral evidence

Logs are under `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4`.

| Boundary | Result | Log |
| --- | --- | --- |
| Initial production GPUI RED | 0/2 passed: Enter left the original rate unchanged and task editor was absent | `b2-ref06-red.log` |
| Exact-byte Settings Apply RED |1/1023B/s became None;1537B/s became1024 after actual unrelated Apply | `b2-acceptance-ref06-red.log` |
| Current task cap compact-refresh RED | Engine64KiB/s, actual capabilities label “不限速” | `b2-acceptance-ref06-round3.log` |
| Production GPUI limit editing including four acceptance regressions |11/11 passed | `b2-acceptance-final-ref06.log` |
| Original settings/task layout and interaction including REF-06 |19/19 passed | `b2-acceptance-final-settings-task.log` |
| Original task-window lifecycle/phase/generation and server-information |23/23 passed | `b2-acceptance-final-task-window.log` |
| Same-task real HTTP/HLS/DASH waiter integration |1/1 passed, three protocols | `b2-acceptance-rate-waiters.log` |

GPUI fixtures drive actual official focus/typing/Enter/Blur/Escape and original disclosure/Cancel actions. They assert no1/10 intermediate rate while typing100, exactly one durable TaskChanged/submission, stable InputState, independent64KiB/s manual and32KiB/s runtime ceilings, local errors, temporary SQLite rejection/retry, window/engine reopen, contained input bounds and source closure before deferred command dispatch.

The precise-rate cases drive original Settings Apply after editing an unrelated connection limit and reopen original SQLite/settings for1,1023and1537B/s. Task cases use real typed engine commands for64KiB/s,32KiB/s,None and1537B/s with both no global ceiling and128KiB/s global; original compact/progress refreshes retain the displayed effective minimum, applied value and stable clean InputState. A later2049B/s external command preserves an invalid dirty/error draft; Escape restores the latest3KiB display, and window/SQLite reopen retain2049exact bytes. The original capabilities accessibility label is observed through Kit's public test-support interface. Base0.8.0 `set_value` emits no Change; display synchronization is not an explicit edit.

The deterministic HTTP integration retains the same task and artifact for all three protocols. While any original1byte/s task/manual/runtime ceiling remains, bounded waits cannot complete. Removing the original caps wakes the existing transfer and produces exact HTTP/HLS/DASH artifact bytes without restarting the task; reopening SQLite retains removed manual/task intent.

Commands:

```powershell
cargo test -p nxtools-desktop --bin nxtools-desktop ref06_ -- --test-threads=1
cargo test -p nxtools-desktop --bin nxtools-desktop settings_task_rem_tests -- --test-threads=1
cargo test -p nxtools-download --test engine_contract ref06_ -- --test-threads=1
```

All Rust commands used one Cargo lane, CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256 and scratch TEMP/TMP.

## Diagnostics

`download.speed_limit.edit_started` shares the original command UUID with the engine boundary. It terminates as edit_applied, edit_failed or edit_cancelled with fixed global/task scope, command_rejected/window_retired outcome and elapsed time. Local invalid drafts dispatch no command. Diagnostics contain no input text, URL/path, Header/Cookie/credential or raw error and add no per-keystroke/frame/byte logging.

## Verification limits

These are compiled GPUI TestPlatform and deterministic local HTTP/SQLite results. Native HWND, physical keyboard/IME/DPI, provider, human visual and installed acceptance are separate. Final affected rate/runtime/B1/lint/format results are recorded in the B2 handoff, not inferred from these focused suites.
