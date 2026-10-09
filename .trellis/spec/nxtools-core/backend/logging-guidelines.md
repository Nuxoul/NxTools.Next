# Core diagnostics

Evidence: `repo:rust/crates/nxtools-core/src/lib.rs#init_file_logging`, `repo:rust/crates/nxtools-core/src/lib.rs#LogGuard`.

init_file_logging creates daily tracing_appender and nonblocking writer; retain WorkerGuard in process/runtime owner until shutdown. Logging must not change operation result. Environment filtering is configured at initialization, not per hot-path event. Library config/paths errors can carry sensitive context; emit sanitized stage/code, never raw config/path/credentials. Shared process names/IDs and counts are metadata; user window titles/clipboard values are not. Follow [engineering diagnostics](../../shared/engineering.md); no extra per-read/write logging is necessary for pure normalization.
