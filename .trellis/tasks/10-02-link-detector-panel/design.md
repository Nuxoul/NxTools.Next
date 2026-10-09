# Link Detector Floating Panel Design

## Current chain and constraints

```text
Allowed clipboard capture with non-empty text
 -> ClipboardApplication::handle_link_detector_capture
 -> detect_links (typed Web/LocalFile/LocalDirectory)
 -> show_link_panel (one reusable GPUI popup + HWND bounds/topmost commands)
 -> LinkPanelView row callback
 -> close window, configured copy/browser/shell route, clipboard write
```

Rust owns classification, preferences, panel generation, route and layout. GPUI owns rendering/window entity. C# PlatformHost owns native Z order/bounds/visibility and reports native failures through existing IPC. Do not add a second GPUI event loop.

## Changes (only after baseline interaction regression)

- Pin `mage.routerapi.net` exactly in detector tests; keep host display/copy unchanged and synthesize `https://` only for opt-in browser route.
- Preserve default copy-only / opt-in auto-open. Add a bounded action result/notification or in-panel confirmation with clear text; do not silently change action semantics.
- Add capture generation to reusable view/session. Each restage increments generation, cancels/invalidates previous hide timer, updates rows, requests topmost and final physical bounds; reveal only after final bounds/topmost result policy is resolved.
- Timer callback compares the captured generation before removing the native GPUI window, so old timeout can never hide new content. Each capture gets its own configured dwell.
- Replace opaque `gpui_component::Root` wrapping only for this transparent window with a local transparent root using the existing view, notification/action infrastructure only if needed, and no opaque full-client fill. Keep visible rounded surface as the hit-test surface; prove alpha and Windows corner behavior before claiming success.
- Use existing topmost/bounds commands and correlate result lifecycle. Native failure leaves a diagnosable state and must not be logged as confirmed success. Logs include correlation ID, generation, target kind/length and native errors only—never target value.

## Validation

Pure detector/action tests, GPUI click/reuse/timer lifecycle test, real command-result path test, architecture gates, then interactive Windows test for alpha corner, input hit test, topmost vs normal applications, auto-hide and repeated capture.