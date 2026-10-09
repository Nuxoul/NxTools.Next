# Clipboard backend

Evidence: `repo:rust/crates/nxtools-clipboard/src/lib.rs#ClipboardHistory`, `repo:rust/crates/nxtools-clipboard/src/archive.rs`, `repo:rust/crates/nxtools-clipboard/src/storage.rs`.

## Pre-Development Checklist
Read [clipboard reference](../../../../docs/reference-code/clipboard.md), [engineering](../../shared/engineering.md), [structure](directory-structure.md), [database](database-guidelines.md), [errors](error-handling.md), [logging](logging-guidelines.md), [quality](quality-guidelines.md). Portable ZIP/moving storage are Next-specific; preserve native-format/asset references and serial worker ownership.

## Quality Check
`cargo test -p nxtools-clipboard --lib`, real temporary SQLite/archive/shared-asset/retention/cursor/backup/migration fixtures; relevant Desktop worker/session regressions. Scoped strict checks follow [verification](../../shared/verification.md). Never inspect live user clipboard/history for tests. [Entry](../../index.md).
