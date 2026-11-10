# U3 acceptance follow-up

## 1. Scope / Trigger

Four accepted gaps in the delivered health-recovery path: a dropped health transition under native-event backpressure; retry control busy state differing from command admission; an unavailable episode first observed during recording not notifying when recording ends; and a disconnected re-enable operation without its correlated terminal diagnostic.

## 2. Signatures

Reuse the existing QuickLaunchKeyboardHealthChangedEvent snapshot, PlatformPipeClient event forwarding, QuickLaunchApplication pending configuration/re-enable tuple, original Settings Alert/Button and NotificationCenter. Preserve outer protocol1 and quick_launch_keyboard_health_v1 negotiation. No new command, saved setting, native monitor or notification owner.

## 3. Contracts

- The native hook publisher remains nonblocking. The existing event-reader recovery path coalesces and forwards the latest immutable health snapshot after queue saturation, without a timer/heartbeat or unbounded queue. Queued older health facts must not overwrite the latest snapshot, and an already sent identical fact is not repeated.
- Retry loading/disabled and actual admission share busy state, including automatic ConfigureQuickLaunch pending.
- A fault suppressed during recording emits one deferred notice when it becomes relevant/visible after recording ends; the same unavailable episode does not notify repeatedly. Recovery allows a future failure episode to notify.
- Disconnect takes pending re-enable ownership and closes its original correlation/elapsed diagnostic exactly once. A command already terminated by its matching result is not terminated again.

## 4. Validation & Error Matrix

| Boundary | Result |
| --- | --- |
| Native event queue full, then reader resumes | Latest health forwarded in the current session; hook never waits |
| Peer did not accept health capability | No unknown health event sent |
| Older health queued, newer snapshot retained | Only current health can become authoritative |
| Automatic or manual configuration pending | Retry visibly busy/disabled and no duplicate dispatch |
| Failure during recording, then cancel/end | Alert and one notice when eligible |
| Repeated failure/state refresh | No duplicate notice for the same episode |
| Re-enable pending, then disconnect | Original ID/elapsed/fixed disconnected terminal and cleared pending |
| Matching result before disconnect | No duplicate terminal |

## 5. Good / Base / Bad Cases

Good: latest health survives full transport and a temporary recording suppression. Base: one original configure operation, native health owner and deferred notification. Bad: blocking the hook to preserve an event, a clickable action that is always rejected, or losing terminal correlation during cleanup.

## 6. Tests Required

Add behavior-red regressions at the actual event-reader projection and production Quick Launch/TestApp state boundaries before fixes. C# fixtures use only bounded in-memory channels/fake operations and require explicit harness/effect registration. Rust tests cover automatic pending UI, recording notification dedup and correlated disconnect terminal. Run affected C# fixtures, filtered Quick Launch/settings/hotkey-reset tests, strict scoped Clippy, changed-file rustfmt, lifecycle/spec and diff gates. Extension, protocol shapes and unrelated download/Archive work are unchanged.

## 7. Wrong vs Correct

Wrong: TryWrite failure is ignored until a later handshake. Correct: the current reader forwards the retained latest health when existing queued events resume. Wrong: notify only at the instant of the fault. Correct: remember the episode and notify once when it becomes eligible. Wrong: clear pending on disconnect. Correct: take pending, emit its correlated terminal, then retire it.

## Baseline and constraints

107dirty entries at follow-up entry. Ten relevant source copies and git status are saved in session scratch/u3-followup-baseline. Preserve all existing changes. No product launch, native hook/SendInput/UAC/service operation, real browser profile/network/user data, commit, packaging, vendor/dependency/palette change or unrelated audit repair. Rust verification retains one job, incremental disabled, codegen-units256, offline and desktop test debug=0.
