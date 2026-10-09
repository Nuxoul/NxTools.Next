# Indexer cache and settings (SQL not applicable)

Evidence: `repo:rust/apps/nxtools-indexer/src/mft/cache.rs#MappedSnapshot`, `repo:rust/apps/nxtools-indexer/src/settings.rs`, `repo:rust/apps/nxtools-indexer/src/usn.rs`.

No SQL connection/schema: cache VERSION3/magic/layout bounds validates mapped base plus persisted delta and mount/journal identity. Keep immutable searchable base and overlay, version compatibility and atomic staged persistence/compaction; restart restores both, not rebuild due desktop reconnect. Native mmap unsafe is narrow with file lifetime and bounds checks. Settings use synced temporary and last-good backup; corrupt existing primary+backup fails closed instead of broad default scope. A bounded RebuildRequired with searchable last-good is maintenance, not last_error. Genuine persistence/compaction/poll worker failures remain faults. Don't invent Listary native algorithms or universal ORM. [Verification](../../shared/verification.md).
