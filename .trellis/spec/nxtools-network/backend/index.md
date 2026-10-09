# Network backend

Owner: product outbound HTTP route snapshot/client construction; no downloads, UI or SQL. Evidence: `repo:rust/crates/nxtools-network/src/lib.rs#NetworkPolicy`.

## Pre-Development Checklist
Read [architecture](../../shared/architecture.md), [engineering](../../shared/engineering.md), then [structure](directory-structure.md), [persistence](database-guidelines.md), [errors](error-handling.md), [diagnostics](logging-guidelines.md), [quality](quality-guidelines.md). Settings persist in Core; consumers freeze operation snapshot.

## Quality Check
`cargo test -p nxtools-network --lib`, immutable-snapshot and real local proxy tests; scoped lint/check/fmt per [verification](../../shared/verification.md). No external proxy/credential inspection. Docs-only spec-contracts. [Standards entry](../../index.md).
