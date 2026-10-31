# Protocol backend

Owner: typed PlatformHost/indexer wire model, not transport/state/persistence. Evidence: `repo:rust/crates/nxtools-protocol/src/lib.rs#Envelope`.

## Pre-Development Checklist
Read [engineering](../../shared/engineering.md), [structure](directory-structure.md), [persistence](database-guidelines.md), [errors](error-handling.md), [logging](logging-guidelines.md), [quality](quality-guidelines.md). Schema changes trace Rust/C#/caller/test, preserve version/defaults and bounds.

## Quality Check
`cargo test -p nxtools-protocol --lib`; affected IPC/C# roundtrips and lifecycle tests; scoped lint/check/fmt per [verification](../../shared/verification.md). No actual native command required for DTO tests. [Standards entry](../../index.md).
