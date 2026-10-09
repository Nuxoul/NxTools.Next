# Download IPC backend

Evidence: `repo:rust/crates/nxtools-download-ipc/src/lib.rs#DownloadIpcFrame`.

## Pre-Development Checklist
Read [engineering](../../shared/engineering.md), [structure](directory-structure.md), [persistence](database-guidelines.md), [errors](error-handling.md), [logging](logging-guidelines.md), [quality](quality-guidelines.md). Typed Download commands and engine store remain authoritative; current-user pipe policy is distinct from Indexer protected-service admission.

## Quality Check
`cargo test -p nxtools-download-ipc --lib`; six existing host/frame tests plus changed caller/handoff consumers. Scoped lint/check/fmt per [verification](../../shared/verification.md). Do not start product host merely to test framing. [Entry](../../index.md).
