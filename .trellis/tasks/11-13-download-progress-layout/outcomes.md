# Download progress layout outcomes

## Layout and behavior

The compact client remains 440×184 DIP. The expanded client is 440×400 DIP, with persistent aggregate progress and transfer actions surrounding three mutually exclusive official Kit pages: 连接详情, 限速 and 完成后. The connection page has byte-coordinate coverage and a scrollable segment table. The speed page keeps the original InputState, a Medium32-DIP Input with KiB/s suffix, draft validation and Enter/Blur/Escape semantics. The completion page retains persisted power/application intentions and the independent open-directory intention. The footer contains Options and download actions, without duplicate completion controls.

ABDM's `ProgressDownloadPage.kt` and its keyed dialog/window chain, legacy `DownloadProgressWindow.axaml[.cs]` and its actual show/close path, and Mister's `crates/app/src/progress.rs` supplied the behavioral references recorded in design.md and docs/reference-code/download.md. Mister is a behavior-only GPL reference.

Same-attempt downloading views retain their last complete live range projection during authoritative snapshot gaps. A new split plan replaces the old projection; task, attempt, source or downloading-state retirement clears it. The regression asserts live512-byte coverage does not fall back to a durable64-byte checkpoint, replacement plans are not unioned, and source/attempt/pause changes retire live coverage. Rendering performs no IO and adds no sampling or animation timer.

The same task-keyed window/generation, native centered-resize owner, presentation-only close and same-window completion remain intact.

## Verification — 2026-11-13

- Focused production-view, limiter, completion-intention/countdown, keyboard, persistence and window-lifecycle selection: **59 passed, 0 failed**.
- Final integrated `downloads::` run: **322 passed, 10 failed, 0 ignored**, 250.47 seconds. The remaining failure names were also present in the pre-layout run.
- Debug desktop build passed. The isolated preview uses an in-memory task engine, disconnected platform bridge and no download runner. It starts before product runtime initialization.
- Eight actual owned-window captures at scale1 were acquired and visually inspected: dark/light × connection/speed/completion/compact. Generated evidence is in `artifacts/ui-preview/download-progress-{dark,light}-{connections,speed,completion,compact}.png`.
- GPUI release, Markdown editor provenance, download UI lifecycle and spec-contract gates passed.
- Strict Desktop Clippy remains blocked by the two existing findings listed below.

## Known issues and acceptance limits

The integrated suite remains failing. Its ten remaining failures are:

```text
queue02_headers_have_one_styled_textarea_frame
background_limit_setting_keeps_manual_limit_and_validates_only_when_enabled
b4_e2_native_close_retires_held_probe_without_changing_reopened_task
b4_ref08_actual_close_and_stale_identity_preserve_reopened_window
b4_ref08_actual_first_focused_enter_preserves_writer_and_sqlite
b4_ref08_actual_first_focused_space_preserves_writer_and_sqlite
b4_ref08_actual_pointer_confirmation_preserves_writer_and_sqlite
b4_ref08_actual_refusal_retains_input_retry_and_cancel
b4_ref10_actual_paused_window_exposes_written_range_coverage
b4_ref10_real_parallel_events_refresh_original_window_through_rebalance
```

These cover the request-header textarea width, background-limit validation, paused-source lifecycle fixtures and a parallel-event assertion. They remain outside this progress-layout slice.

Strict Clippy findings: `downloads/browser_handoff_impl.rs:436` (`nonminimal_bool`) and `downloads/settings_task_rem_tests.rs:614` (`unwrap_used`). Neither expression belongs to the layout change.

The captures are synthetic initial-size windows at scale1. They do not certify installed downloads, live-network flicker, native expansion/center transitions, mixed-DPI behavior, physical keyboard/pointer input, screen readers or user visual approval. No Release package, installed binary replacement, service operation or real download was performed by the preview.
