# Indexer diagnostics

Evidence: `repo:rust/apps/nxtools-indexer/src/lib.rs`, `repo:rust/apps/nxtools-indexer/src/mft/cache.rs`, `repo:rust/apps/nxtools-indexer/src/windows_service_host.rs`.

Record generation/session/operation IDs, published counts, bounded maintenance state, duration/native fixed error and denial reason. No filename/path/query/window-title/raw command or hash-protected payload logs. No per-file MFT/USN/search-hit/poll idle spam; state changes, coarse counts and backoff suffice. Rebuild reason separate from actual last_error. Existing anyhow path context isn't log-safe automatically; keep gap honest. [Shared diagnostics](../../shared/engineering.md).
