# Indexer journal correctness and maintenance status

## Goal

Correct H2/H3/M3/M4 in the index service, expose per-volume maintenance state in Quick Launch settings (U4), measure M8 overlay costs, and fix the confirmed legacy-delta allocation issue in L11. Keep last-good search available and preserve service admission, durable base/delta identity and generation isolation.

## Requirements

### H2 — bounded recursive deletion

Deleting5000visible leaf files as individual journal records must remain incremental and remove every file. Invisible entries and direct leaf deletion must not consume recursive-expansion work. Directory candidate accounting excludes overlay-deleted or otherwise invisible children. Deleting a directory whose base contains more than4096children but only a bounded visible remainder must succeed. Truly large visible subtrees remain bounded and request maintenance rebuilding.

### H3 — initial enumeration and replay completeness

An item that existed before scanning and was modified during scanning must remain visible when subsequent journal records contain only metadata changes. Preserve the journal cursor captured before enumeration; replay create/update records idempotently without duplicates or omission. Verify directories under the same enumeration rule. Do not turn unknown metadata into a broad insertion or resurrect deleted rows.

### M3 — obsolete cache retirement

Successful rebuild/compaction publication retires obsolete cache versions during the service lifetime. Cleanup follows publication and release of the old owner's mappings, preserves the current base/delta pair and recovery authority, and tolerates files still mapped by searches. Consecutive synthetic rebuilds with released prior mappings leave only the current pair.

### M4 — responsive authenticated replacement sessions

When authenticated session A is connected but no longer reading, session B must receive HelloAck and status within a bounded interval, including while an old scan/rebuild is running. A's residual requests/results must not change B's search generation. Preserve PID/session/path/hash authorization, the3second initial-frame deadline,4pending slots and1MiB frame limit. Retain bounded worker ownership and completed last-good snapshots across reconnect.

### U4 — per-volume maintenance feedback

Add optional, serde-default-compatible per-volume status for initial building, rebuilding with a fixed reason code, and ready. Old desktop messages without the field remain readable and old desktops ignore the new field. Service state transitions emit low-noise maintenance diagnostics. A bounded RebuildRequired while last-good is searchable is maintenance, not last_error; genuine poll, persistence, compaction and worker failures remain faults.

The existing Quick Launch Index settings page displays each volume's state through official Kit0.8.0 Tag or Alert. Rebuilding copy includes `正在重建，搜索结果可能不完整` and a readable reason. Keep the existing settings hierarchy and all scan/search/configuration owners. The Quick Launch popup has no new status strip or geometry change.

### M8 and L11 — measured resource costs

Provide a default-ignored synthetic measurement using a1million-row base and1k/100k overlays. Report apply_journal_batch and classify_applied_update time and allocation counts, and verify whether no-change native polling copies the overlay. Optimize only if the measured100k case clearly harms search scheduling, with data and a stated judgment. No production high-frequency instrumentation.

Before allocating legacy delta row maps, constrain declared counts by actual file length. Corrupt counts are rejected and recovery retains the base snapshot. Measure and report heap-owned versus mapped persisted snapshot memory; do not change rebuilt/compacted snapshot residency in this task.

## Acceptance

- Each confirmed behavior repair has a regression that reaches its intended failure assertion on original behavior, followed by a passing run.
- Tests use synthetic scanner/journal inputs, temporary cache files and isolated in-memory/temporary-pipe session fixtures only.
- Protocol old-message/default/new-field roundtrips, service maintenance transitions and production Settings render bounds cover U4.
- Run safely scoped indexer, protocol and IPC tests; affected Desktop settings/index status tests; strict indexer/protocol/Desktop Clippy; changed-file rustfmt; spec and applicable lifecycle gates; git diff --check.
- Record root causes, changed symbols, H3/M4 decisions, compatibility, red/green evidence, measurements and exact command results in outcomes.md.
- Report inspected reference sources accurately: Listary managed boundary only, SwiftList inspectable source, and matching legacy NxTools entrypoints where present.

## Constraints and acceptance limits

Entry baseline139dirty entries, plus11relevant source copies, is retained in session scratch/indexer-correctness-baseline. Preserve all unrelated work, including previous download/input/extension changes. No commit/push, unrelated formatting, vendor/dependency/palette change, product launch, packaging, installation, Windows-service operation, real network/user-data access or real-volume MFT enumeration.

Cargo is serialized with CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256 and offline mode. Desktop tests use debug=0 and one test thread. After a command timeout inspect surviving Cargo/rustc before another build.

The packaged LocalSystem idle/restart/base+delta/search release gate is pending separate authorization; record exact commands without executing it. Do not claim real-volume, installed-service, physical/DPI or user visual acceptance. Retain and report the unrelated date-dependent downloads calendar assertion and existing Archive formatting differences without suppressing or repairing them.
