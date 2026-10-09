# Finder usage storage

Evidence: `repo:rust/crates/nxtools-finder/src/lib.rs#FinderCatalog`, `repo:rust/crates/nxtools-finder/src/lib.rs#record_activation`, `repo:rust/crates/nxtools-finder/src/lib.rs#clear_history`.

rusqlite connection owned by catalog; tables quick_launch_usage, quick_launch_query_usage and quick_launch_directory_history store activation/query/directory ranking metadata, not raw file index. Bind values with params and preserve entry/query keys and Windows path normalization. Clear history commits transaction before clearing caches; writes update in-memory projection only after success. No imagined numbered schema/WAL guarantee: current tables initialized idempotently in open. Last-good refresh retains catalog on scan errors via apply_refresh_snapshot. Recent directory count bounded64. Test durable reopen/clear/ranking with temp DB; never user history. [Engineering persistence](../../shared/engineering.md).
