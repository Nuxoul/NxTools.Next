# Repair Link Detector Floating Panel

## Goal

Make copied-host recognition and the single reusable link panel's actions, topmost state, auto-hide, and transparent rounded rendering reliable.

## Confirmed Context

- Clipboard capture only triggers detection for allowed non-empty text captures. `detect_links` recognizes bare hosts with `.net`, but currently tests `image.routerapi.net`; the user's exact input was `mage.routerapi.net` and was not verified in the packaged UI this session.
- Defaults are copy-only, browser auto-open off, and 6-second configurable auto-hide. A row click closes, queues the configured route, and writes the text to clipboard. Preserve these defaults; a silent successful copy currently explains the no-feedback report.
- Creation requests topmost but discards the result; restaging a live panel does not reapply/confirm topmost.
- Restaging arms an additional uncancelled timer, so an older capture can hide new content early.
- Transparent WindowOptions are wrapped in gpui-component Root, which paints an opaque full-client background. Rounded child clipping does not establish real Windows per-pixel transparency.
- Keep this auxiliary window on the app's existing single GPUI runtime; no GPUI Kit/second platform loop.

## Requirements

1. Add detector regressions for exact `mage.routerapi.net`, case, punctuation, whitespace, and surrounding copied text; preserve typed local-file and explicit URL behavior.
2. Keep copy-only as the default. Keep optional browser auto-open opt-in. Show a clear success/failure action result for click-to-copy/open so a successful action is not visually silent.
3. Maintain one reusable HWND and set final device-pixel bounds before reveal; reapply and handle a confirmed topmost result on both creation and reuse.
4. Scope the hide timer to capture generation; restaging resets the configured dwell, stale timers cannot close newer content, and the normal configured timeout is not extended by pointer hover.
5. Use a transparent GPUI root that does not paint an opaque rectangular background. Preserve mouse hit testing on visible content; verify real transparent corners on Windows.
6. Define payload-safe correlation and lifecycle diagnostics across Rust, IPC and C# for native presentation; never log clipboard text or link/path values.

## Out of Scope

- Changing persisted default route, making browser launch implicit, or altering clipboard capture/persistence behavior.
- Moving the panel into a second GPUI runtime or another process.
- Changing system-wide window shape policy for other windows.

## Acceptance Criteria

- [ ] Exact `mage.routerapi.net` test passes and the user-visible capture path can show the panel for this bare host.
- [ ] Row click obeys configured route, gives visible success/failure feedback, writes copy value when appropriate and closes only according to the existing panel action contract.
- [ ] Repeated capture reuses one window, places/resizes it before display and confirms topmost; native topmost failure is observable and does not get reported as success.
- [ ] A timer regression proves capture N's timer cannot hide capture N+1 before N+1's configured dwell.
- [ ] Focused GPUI tests cover expansion, click action/feedback, reuse and timer generation; architecture/source gates cover event boundaries.
- [ ] Windows smoke verifies true transparent rounded corners, corner hit testing, pointer clicks, topmost over normal applications and auto-hide.
- [ ] Diagnostics are bounded/correlated and omit clipboard/link payloads.