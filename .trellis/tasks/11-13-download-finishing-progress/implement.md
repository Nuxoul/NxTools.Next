# Implementation

1. Carry additive, event-only finishing stage/units through the original bounded progress stream.
2. Count concatenated bytes at the real HLS/DASH writer, preserve bounded copies and forward media-finalization pause/cancel through the existing control.
3. Read bounded FFmpeg progress incrementally, retaining timeout, cancellation, exit diagnostics and both output drains.
4. Project named stages through existing official Progress/taskbar surfaces with unchanged transfer bytes and retained media controls.
5. Verify real HTTP/child/engine/artifact and production GPUI boundaries, strict affected lint and lifecycle/source gates.
