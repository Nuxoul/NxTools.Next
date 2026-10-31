# Core structure

Evidence: `repo:rust/crates/nxtools-core/src/lib.rs#AppConfig`, `repo:rust/crates/nxtools-core/src/paths.rs#AppPaths`, `repo:rust/crates/nxtools-core/src/instance.rs#SingleInstanceGuard`, `repo:rust/crates/nxtools-core/src/logging.rs#init_file_logging`.

lib.rs owns configuration models/defaults, normalize/validate, section writes and process-blacklist rules. paths.rs owns directory discovery/config location; instance.rs owns the process lock and advisory HWND hints; logging.rs owns nonblocking initialization and scoped retention. Root re-exports preserve existing caller interfaces. These are concrete responsibilities, not new forwarding managers. Configuration atomic-write/lock behavior and runtime guard lifetimes remain unchanged. Do not put download execution, GPUI entities or C# policy here; consumers own live application state. [Shared architecture](../../shared/architecture.md).
