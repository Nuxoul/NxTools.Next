# Design

## References and boundary
ABDM `PartDownloader.kt::copyDataSync`251-303 reads, writes and only then advances the part cursor; `destination/DestWriter.kt::write`49-63 remains the sink owner. Legacy `Services/DownloadManagerService.cs::CopyToTargetAtOffsetAsync`2661-2732 places `ReadWithStallTimeoutAsync` around reads only, outside RandomAccess writes and throttling. Mister `crates/engine/src/download.rs::Pace`4620-4663 separates accumulated disk time from network time; `yardstick`4677-4690 uses total elapsed, while `should_shed`4696-4721 excludes recent disk pressure and limiter waits. Behavioral reference only.

Current chain: `segment_workers` -> `HttpDownloadClient::download_planned_segment` -> `ClaimedWriter::{flush,sync}` -> `SegmentWriter::{write_all,flush,sync}`. The original writer owns actual bytes; the planner commits only after writes and marks synced only after sync. Two defects share the same boundary: disk awaits lower the apparent network rate, and can consume the next read's stall budget.

## Correction
Track disk-await duration locally in ClaimedWriter and share only the latest significant disk-pressure instant across the existing receive state. The speed yardstick stays based on total elapsed. Retirement uses elapsed minus disk waits, keeps the original two-second network grace and suppresses retirement for two seconds after a significant (50ms) disk wait. Do not alter user/runtime limiter exclusions. Extend the body deadline only by actual awaited local IO/throttling time; periodic empty flushes must not refresh a dead network's deadline.

## Invariants
Keep the 256KiB write batch, current checkpoint cadence and synced prefixes. No large buffer, deferred durability, scheduler owner, persistent setting or new protocol. Claims protect in-flight writes, progress follows physical write, errors never replay an emptied batch, and every terminal/pause/cancel path drains and syncs before planner release. No desktop layout, HWND, placement, focus or native lifecycle change.

## Regression seam
Add a cfg(test), per-artifact delay copied into the existing SegmentWriter, analogous to its existing partial-write injection. Real localhost HTTP invokes the production planned-segment loop with a seeded peer yardstick and controlled disk waits. Require exact bytes and no SlowConnection/Stalled, then exercise pause during an await and verify synced checkpoints. Existing real slow-network retirement, limiter and split/pause tests remain required. Tests run red before the timing correction.

## Diagnostics
Retain original correlated transfer and connection-retirement terminal events and native error ownership. Do not log every wait/write or alter operation results for diagnostics.
