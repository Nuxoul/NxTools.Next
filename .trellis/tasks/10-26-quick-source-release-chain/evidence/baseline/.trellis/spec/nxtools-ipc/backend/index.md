# IPC backend

Evidence: `repo:rust/crates/nxtools-ipc/src/lib.rs#PlatformBridge`, `repo:rust/crates/nxtools-ipc/src/local_pipe.rs`.

## Pre-Development Checklist
Read [architecture](../../shared/architecture.md), [engineering](../../shared/engineering.md), [structure](directory-structure.md), [persistence](database-guidelines.md), [errors](error-handling.md), [logging](logging-guidelines.md), [quality](quality-guidelines.md). Preserve pending-before-send and generation isolation; pipe ACL does not authorize service identity.

## Quality Check
`cargo test -p nxtools-ipc --lib`; immediate-result, queue-full, timeout/replacement/disconnect and temporary-pipe deadline cases. Use [verification](../../shared/verification.md) memory settings and scoped strict lint/check/fmt; never real protected-service commands without approval. [Entry](../../index.md).
