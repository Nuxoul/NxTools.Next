# Indexer structure

Evidence: `repo:rust/apps/nxtools-indexer/src/main.rs`, `repo:rust/apps/nxtools-indexer/src/lib.rs`, `repo:rust/apps/nxtools-indexer/src/mft.rs`, `repo:rust/apps/nxtools-indexer/src/usn.rs`, `repo:rust/apps/nxtools-indexer/src/windows_service_host.rs`, `repo:rust/apps/nxtools-indexer/src/elevated_helper_broker.rs`.

lib orchestrates snapshot/search workers and maintenance; mft/cache owns scan/map/base+overlay, usn incremental journal, settings last-good durable scope, service host pipe admission and SCM lifecycle, broker protected native helper authorization. Rust owns raw-volume index, C# owns deployment/Windows UI helpers; no GPUI. Query generations/cancellation and bounded candidate/materialization are explicit; never move full scanning into desktop query callback. [Architecture](../../shared/architecture.md).
