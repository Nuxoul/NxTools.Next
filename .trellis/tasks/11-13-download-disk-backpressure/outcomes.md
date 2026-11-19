# Outcomes

## Implementation
The existing ClaimedWriter measures actual write/flush awaits. Network stall budgets and relative retirement timing exclude local writer/limiter waits; the peer yardstick retains total elapsed. Significant disk waits share a2s retirement grace across the receive state. Empty periodic flushes never refresh a stalled body's deadline.

The original batch, claims, commit-after-write, checkpoint cadence and sync-before-release remain unchanged. Test-only per-artifact latency injection has no production setting. Single-stream read deadlines already exclude local writes and are unchanged. No UI, protocol, schema or native lifecycle change.

## Verification
- Real localhost planned-transfer red: healthy network failed with Stalled and SlowConnection under injected disk delay.
- Planned-transfer/segment contracts:7 passed, including exact bytes, disk wait exclusions, pressure grace expiry, limiter exclusions, true body stall, partial-write ownership and synced pause prefix.
- Original real HTTP fault recovery:7 passed, including true slow retirement, live split/pause, overlap, refusal/throttle and retry cancellation.
- Existing integration: engine105, HTTP26, web-page6 passed (137 total).
- Strict download Clippy all targets, scoped rustfmt, scoped whitespace check and spec-contracts passed.

## Known issues
The independent full-library B6 native-refusal-only experiment reported in the web-page slice remains unresolved; this slice does not change that connection-learning strategy. Memory acceptance is the preserved bounded batching/no-new-buffer invariant, not a new process-memory benchmark. No package, installation or product launch was performed.
