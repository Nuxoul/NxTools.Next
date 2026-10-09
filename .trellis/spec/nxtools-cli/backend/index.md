# CLI backend

Evidence: `repo:rust/apps/nxtools-cli/src/lib.rs#CliInvocation`.

## Pre-Development Checklist
Read [architecture](../../shared/architecture.md), [structure](directory-structure.md), [persistence](database-guidelines.md), [errors](error-handling.md), [logging](logging-guidelines.md), [quality](quality-guidelines.md). Preserve explicit output/exit and --connect-only contracts; parsing is separate from side effects.

## Quality Check
`cargo test -p nxtools-cli --lib`; parse/exit/JSON/watch/batch-action tests and affected Download IPC tests. [Verification](../../shared/verification.md) flags required for code checks. Don't execute CLI against user's running engine without approval. [Entry](../../index.md).
