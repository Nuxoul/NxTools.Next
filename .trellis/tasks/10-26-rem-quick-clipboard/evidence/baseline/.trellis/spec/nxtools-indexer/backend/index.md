# Indexer backend

Evidence: `repo:rust/apps/nxtools-indexer/src/lib.rs`, `repo:rust/apps/nxtools-indexer/src/windows_service_host.rs`, `repo:rust/apps/nxtools-indexer/src/mft/cache.rs`.

## Pre-Development Checklist
Read [Quick Launch reference](../../../../docs/reference-code/quick-launch-research.md), [architecture](../../shared/architecture.md), [structure](directory-structure.md), [cache/persistence](database-guidelines.md), [errors](error-handling.md), [logging](logging-guidelines.md), [quality](quality-guidelines.md). Listary managed boundary only, not native source. Protected-session authorization differs from dev stdin harness.

## Quality Check
Focused synthetic state/admission/USN/cache tests and strict checks per [verification](../../shared/verification.md). Real-volume test suites may scan; do not run unrestricted without permission. Changed MFT/USN/service/cache release needs packaged LocalSystem idle/restart/base+delta/search/hash gate when authorized. Ready/search unit result isn't release acceptance. [Entry](../../index.md).
