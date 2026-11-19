# Download finishing progress

## Goal
Users can distinguish transfer, media merging and final publication rather than interpreting a long 100% transfer bar as completion.

## Requirements
Reuse existing HLS/DASH/media process and event owners. Show actual merge/finalization stage; show progress only when the operation supplies a meaningful denominator. Preserve original cancellation, no-replace publication and same-window completion. Do not add a checksum workload or pretend an unused Verifying state executes.

## Acceptance
After `11-13-download-disk-backpressure`, a controlled real child/engine regression observes merge before Completed and advancing progress when known; unknown duration stays indeterminate. Production progress surface reports that stage, existing pause/cancel/timeout/failure paths still drain and preserve artifacts correctly.
