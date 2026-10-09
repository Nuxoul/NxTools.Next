# Finder backend

Evidence: `repo:rust/crates/nxtools-finder/src/lib.rs#FinderCatalog`, `repo:rust/crates/nxtools-finder/src/custom_command.rs`.

## Pre-Development Checklist
Read [Quick Launch reference](../../../../docs/reference-code/quick-launch-research.md), [architecture](../../shared/architecture.md), [structure](directory-structure.md), [database](database-guidelines.md), [errors](error-handling.md), [logging](logging-guidelines.md), [quality](quality-guidelines.md). Preparation/ranking belongs here; GPUI session and Windows execution don't.

## Quality Check
`cargo test -p nxtools-finder`; in-module history/catalog and tests/file_query/custom_command/calculator/environment_variable; prepared argv/last-good refresh/ranking assertions. Strict checks follow [verification](../../shared/verification.md). Synthetic roots/SQLite only. [Entry](../../index.md).
