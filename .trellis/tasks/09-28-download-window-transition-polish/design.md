# Technical Design

## Reference and current chain

`docs/reference-code/download.md` requires one keyed task window per task, progress-to-completion in-place transformation, center-preserving resize, and no stacked completion surface. `docs/reference-code/gpui.md` records the current probe resize boundary and the product motion language: short opacity plus slight vertical offset, without idle animation loops.

Current code follows those rules for task identity but has two presentation gaps:

- `NewDownloadView::begin_probe` sends a native centered resize and then resends it at five timer intervals, even though PlatformHost already parks a lost resize during a native move/size loop.
- `NewDownloadView::submit` stashes the source, removes it, and only opens the task window in a later deferred turn. `open_task_window` initially shows the new window at its centered creation bounds, then sends `MoveWindowCenteredOverKey`, which can expose a default-center flash or a short gap.

## Layout state and native boundary

Add a compact layout request guard owned by `NewDownloadView` (or a focused helper in `new_download_view.rs`) with:

- the current probe generation;
- the last requested logical size;
- a monotonically scoped transition key.

All size changes after a native handle exists use `SetWindowBoundsCentered`. Equal size/generation requests are coalesced. `window.resize` remains a no-HWND fallback for tests and the earliest construction path only. The C# `SetWindowBoundsCentered` retry remains the sole native recovery for a user drag/modal loop.

Apply the helper to probe success, probe failure, submission validation failure, and media-inspector sizes. A stale async probe may update neither layout nor animation state.

## Hidden replacement handoff

Keep the existing native stash key and task-window identity. Change the handoff order:

```text
submit accepted
  -> stash source native bounds
  -> open/register task window with show=false and final size
  -> MoveWindowCenteredOverKey
  -> SetWindowVisibility(true)
  -> remove source window once
  -> existing activation policy, if requested
```

The command ordering is serialized by PlatformBridge. The replacement view is mounted before visibility so the first revealed frame is valid. A no-HWND test fallback remains explicit. Native close and replacement failure continue through `clear_task_window_registration` and its anchor stash rather than introducing a second lifecycle owner.

The same source-to-task handoff rule applies to normal UI submissions and browser Blob imports. Scheduled/queue-only plans that intentionally do not open a task window keep their current close behavior.

## Motion contract

Use `ENTER_DURATION_MS`, `EXIT_DURATION_MS`, and `RESIZE_DURATION_MS` from `ui/motion.rs`, adding a named token only if the existing values do not express the transition. Apply `Animation::new(...).with_easing(ease_out_quint())` to:

- the new-download confirmed/inspector content when the probe generation changes;
- the newly revealed task-window shell, using a 3-4 px settle and a small opacity lift;
- optional phase-content replacement while the same task window remains registered.

Do not issue native size/position commands on every animation frame. The HWND appears at its final bounds first; only GPUI content animates. Animation keys include the probe or task-window generation, so stale callbacks cannot animate a replacement window.

## Diagnostics and tests

Rust lifecycle events carry task id, window generation, target layout state, accepted/coalesced/ignored result, and elapsed time. PlatformHost command results retain correlation and native error code. No path, URL, request header, Cookie, or per-frame event is logged.

Add red boundary tests for:

- one resize command for one probe generation;
- stale result ignored;
- error/footer geometry for compact and confirmed forms;
- hidden target placement/reveal command order;
- source removal after target registration;
- one task-window native handle/generation through progress, failed, completed, and retry phases;
- bounded animation key/transition without a repeating timer.

Completed-window Open/Reveal actions are result-driven. Carry the operation correlation ID and IPC command ID through the PlatformHost result; close only after success and only when the originating HWND + window generation still match. Failure, timeout, or disconnect leaves the completed window available for retry. A late result after timeout must not close a window.


- Tests also cover Shell failure retaining the window, timeout suppressing a late success, successful Open/Reveal closing only its originating generation, and re-opening creating a fresh keyed window. Keep target paths out of lifecycle diagnostics.
- The confirmed new-download footer test asserts exact vertical-baseline agreement across schedule, refresh, queue, download and cancel actions, plus the 16 px leading/trailing inset. No layout adjustment is justified unless this production-bounds regression turns red.

## Compatibility and rollback

The download engine, persistence, browser handoff protocol, taskbar overlay, and completion action commands remain unchanged. Roll back the layout guard, hidden handoff, or content animation independently without touching task data or IPC schema.
