# Design

## Reference and ownership
The verified chain and entries are recorded in `docs/reference-code/mister-download-manager.md`, Finishing stages. ABDM was inspected first; legacy named, cancellable merging is the behavior baseline. Mister supplies behavioral evidence only for child-output progress and honest unknown totals.

## Event contract
Add optional event-only finishing progress to DownloadProgress. Concatenating counts actual output bytes against real fragment lengths. Muxing counts FFmpeg out_time_us against a known selected-track duration. Publishing has no denominator. No phase units enter downloaded_bytes, no persistent table or checksum workload is added. Existing command negotiation carries the additive event shape.

## Lifecycle
Use bounded copy loops with pause/cancel checks for both HLS and DASH. Drain and close writers before cleanup or publication. Preserve task/artifact identity, synchronized prefixes, original no-replace publication and same-window status transition. FFmpeg keeps bounded stderr diagnostics, timeout, kill/wait and both pipe-reader drains; progress stdout is parsed incrementally with bounded line storage. Existing correlated start/terminal events remain; phase transitions may be logged, not progress ticks.

## Desktop composition
Retain the existing official Progress and task-window container, controls and geometry. Finalizing displays the named finishing stage and its own ratio when meaningful; absent totals are indeterminate. Transfer size text remains bytes. No Toolbar, substitute control, new window, placement, focus or generation owner.

## Acceptance
Obtain a red at actual HTTP/child -> engine event and production desktop projection before changing behavior. Verify advancing known progress before Completed, unknown-duration indeterminate rendering, HLS mid-merge stop, child cancellation/timeout/failure drains, exact output and original publication semantics. TestPlatform does not certify native or visual acceptance.
