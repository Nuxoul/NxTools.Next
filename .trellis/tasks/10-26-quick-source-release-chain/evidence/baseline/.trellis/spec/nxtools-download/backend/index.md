# Download backend

Evidence: `repo:rust/crates/nxtools-download/src/lib.rs`, `repo:rust/crates/nxtools-download/src/engine.rs`, `repo:rust/crates/nxtools-download/src/storage.rs`.

## Pre-Development Checklist
Read [download reference](../../../../docs/reference-code/download.md) then indexed ABDM/legacy sources; [architecture](../../shared/architecture.md), [engineering](../../shared/engineering.md), [structure](directory-structure.md), [database](database-guidelines.md), [errors](error-handling.md), [logging](logging-guidelines.md), [quality](quality-guidelines.md). Preserve task/plan/artifact identity and writer/checkpoint/publication order.

## Quality Check
Download lib plus affected domain/engine/HTTP/storage/artifact/message/rate/capacity contracts in tests; engine_contract serial. Strict scoped lint/check/fmt use [verification](../../shared/verification.md). Local deterministic IO, not real URLs/credentials. FFmpeg fixtures don't certify installed FFmpeg/live media. No package/launch unless separately authorized. [Entry](../../index.md).
