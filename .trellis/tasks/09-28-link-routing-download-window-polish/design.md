# Technical Design

## Evidence and boundaries

The request contains two separate interaction contracts:

1. `ClipboardApplication::handle_link_detector_capture` calls `link_detector::detect_links`, then `show_link_panel` mounts the reusable `LinkPanelView`. `LinkPanelView::render` currently copies the clicked value and, when enabled, always invokes `cmd /C start` after manufacturing a browser URL. The existing `ShellOpenAction::SelectInExplorer` path already validates an existing filesystem target and launches `explorer.exe /select,"target"` in PlatformHost.
2. Browser/download handoff opens `NewDownloadView` from `browser_handoff_impl.rs`. Probe completion changes its fixed layout through `SetWindowBoundsCentered`, currently followed by five Rust timer retries. Submission stashes the source position, removes the source window, and opens the keyed task window in a later turn. `DownloadWindowLifecycle` already owns one task window per task ID and transforms progress/terminal content in place.

Ownership stays explicit:

- Rust owns target classification, routing intent, download layout state, generation guards, task-window lifecycle, and GPUI motion.
- C# owns `ShellTarget`, Explorer invocation, HWND placement, native visibility, and the existing drag/modal-loop resize retry.
- IPC keeps the existing `OpenShellTarget`, bounds, placement, and visibility commands.

## Link-routing design

Add a target-kind value to the detector result rather than deriving behavior from the display string in the panel. Classification remains conservative:

```text
http:// / https://        -> Web
known bare domain         -> Web
existing drive/UNC file   -> LocalFile
existing drive/UNC folder -> LocalDirectory
anything else             -> ignored
```

The panel maps the kind to one action after it has closed and recorded the copy echo:

```text
LocalFile      -> OpenShellTarget(SelectInExplorer)
LocalDirectory -> OpenShellTarget(Open)
Web            -> existing configured browser launch, unless copy-only
```

The route decision should be a pure helper so tests can prove it without spawning a browser. The row callback must continue to remove the window before clipboard write; the existing echo guard remains authoritative. Link diagnostics record only kind, length, and whether a browser or shell command was accepted. Raw values are excluded.

No change is needed in `ClipboardShellTarget`, `PlatformCommandParser`, or the wire schema. The existing C# validation is the failure boundary for a file that disappears between detection and click; Rust should surface the existing command failure without falling back to a browser.

## Download geometry and handoff design

### Single layout authority

Introduce a small Rust-side layout request guard in the new-download view. It tracks the current probe/layout generation and last requested logical size. Every post-HWND layout change goes through the native-aware helper; identical requests in the same generation are coalesced. Direct `Window::resize` remains only as a no-HWND test/fallback path.

A probe result may apply only when its generation matches. On a real HWND Rust sends one `SetWindowBoundsCentered` for the final target size. The C# side retains its existing `PendingCenteredResizes` / `WM_EXITSIZEMOVE` behavior, so a user drag or native modal loop can recover a lost resize without Rust repeatedly issuing visible recenter operations. Error and submit-validation paths use the same helper, preserving the fixed footer contract.

### Probe-to-task handoff

The handoff is performed from a deferred application turn so the current `NewDownloadView` entity is no longer leased:

```text
accepted task
  -> stash source native visible rectangle under task id
  -> open/register task window hidden, with its final logical size
  -> native MoveWindowCenteredOverKey consumes the anchor
  -> native SetWindowVisibility(true) reveals the placed task window
  -> remove the source probe window exactly once
  -> optional activation follows the existing task-window activation policy
```

The command order is serialized through the existing PlatformBridge. If no native handle is available, retain the test/fallback path and use GPUI's local visibility behavior without pretending that native placement was confirmed. A stale close or failed replacement must still go through the existing keyed cleanup path.

This changes the source-close/deferred-open ordering only after the red lifecycle regression is added. It does not allow `DownloadApplication` to update the leased view synchronously.

### Motion

Reuse `ui::motion` tokens and existing GPUI `Animation`/`ease_out_quint` patterns:

- probe content/layout changes: one short transition keyed by probe generation;
- newly revealed task surface: approximately 120 ms opacity from `0.94` to `1.0` plus a 3-4 px upward settle;
- no frame-by-frame native `SetWindowPos` and no perpetual timer.

The native window is placed atomically before visibility. The visual motion is therefore limited to rendered content and cannot expose the default centered HWND or make the window physically wobble. Reused task windows continue to transform in place; phase changes may use the same bounded content transition key while preserving their native identity.

## Diagnostics

Rust events use the task id/window generation as bounded correlation fields:

- layout request accepted/coalesced, with target state and generation;
- stale probe result ignored;
- handoff anchor stashed, placement queued, reveal accepted/failed;
- transition completed or aborted because the window was closed/replaced.

PlatformHost continues to report native command lifecycle and native error codes through the existing correlation ID. No paths, URLs, headers, cookies, clipboard text, or animation frames are logged.

## Compatibility and rollback

The link change is additive to `DetectedLink` and leaves the C# shell contract untouched. The download change is limited to presentation/lifecycle orchestration; engine state, task protocol, taskbar progress, scheduled tasks, browser Blob cleanup, and completion actions remain unchanged. Roll back independently by reverting the link-kind route helper or the hidden-target handoff/layout guard; no database or wire migration is required.

## Verification boundary

Add red tests at the detector/panel route boundary and the real probe submission/task-window boundary before implementation. Then run focused desktop tests, the full relevant desktop test target, PlatformHost focused/full tests, and a manual Windows run covering the supplied `.exe`, probe confirmation, a dragged probe window, submit, retry, and terminal transform.
