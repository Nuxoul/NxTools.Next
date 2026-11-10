# REF-04 — Explicit link-panel download intake

## Implementation

The original420DIP/28DIP ListItem panel adds the named released0.8.0 XSmall ghost **使用 NxTools 下载** Button only for admitted HTTP(S) targets. Independent Copy/Open and B1 ordinary-row settings remain unchanged. The Button closes its source panel locally, then deferred ClipboardApplication identity validation returns the original weak DownloadApplication binding. Retired/replaced/ownerless sources cannot open an intake.

`NewDownloadInitialSource::Url` initializes the original NewDownloadView URL InputState without an external browser request, clipboard read or automatic probe. Continue and final confirmation retain authorization. Duplicate activation reuses the original Root/view/window; deferred fill checks original window and existing pristine-draft eligibility. Populated or edited-then-cleared drafts remain unchanged. A supplied URL retires the blank window's pending clipboard read. Native topmost/activation, deferred input focus, original paste/IME and fixed confirmation owners are retained. No new controller, window queue, transport, limiter or C# command is added.

Bounded `tools.link_panel.download_started/download_finished` events share the action correlation; native activation/topmost commands carry that same correlation. Fixed outcomes and elapsed time contain no target URL/path or clipboard payload.

## Verification

Source baseline RED: `cargo test -p nxtools-desktop ref04_ -- --nocapture --test-threads=1` failed at the actual missing named production Button. Log: `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4\b2-ref04-red.log`.

| Current command filter (Desktop --bin nxtools-desktop) | Result | Scratch log |
| --- | --- | --- |
| `ref04_ -- --nocapture --test-threads=1` | 6 passed | `b2-ref04-green-final.log` |
| `ref01_ -- --test-threads=1` | 4 passed | `b2-ref04-adjacent-ref01_.log` |
| `tools::polish_tests -- --test-threads=1` | 11 passed | `b2-ref04-adjacent-tools--polish_tests.log` |
| `download_intake_ -- --test-threads=1` | 11 passed | `b2-ref04-adjacent-download_intake_.log` |

All logs are under `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4`. Rust verification used the single Cargo lane with jobs1, incremental0 and codegen-units256.

The six production GPUI tests cover pointer and semantic Tab/Enter/Space activation, original URL focus, stable window/view/InputState, duplicate populated-draft preservation, pristine blank-window fill, late one-shot clipboard rejection, actual user edit-then-clear, close-observer replacement before deferred dispatch, local/unsupported targets and final cleanup. Original in-memory engine task queries remain empty; an actual nonblocking localhost listener receives no connection; only original topmost/activation commands are queued, with no clipboard/Shell command. Rendered name/action bounds stay inside the original row and retain at least40DIP for the name lane. B1 exact-once Copy and ordinary settings regressions remain green.

## Acceptance level

Compiled GPUI TestPlatform and owned worker/engine/socket fixtures are verified. Native/physical keyboard, clipboard, IME, DPI, accessibility provider, human visual and packaged/installed acceptance were not performed. B2 remains in_progress; REF-05/06/12 and final checks precede coordinator acceptance.
