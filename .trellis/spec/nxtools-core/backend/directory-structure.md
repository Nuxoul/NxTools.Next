# Core structure

Evidence: `repo:rust/crates/nxtools-core/src/lib.rs#AppPaths`, `repo:rust/crates/nxtools-core/src/lib.rs#AppConfig`, `repo:rust/crates/nxtools-core/Cargo.toml`.

lib.rs owns settings types/serde defaults, normalize/validate, paths, section writes, ProcessBlacklistSettings, SingleInstanceGuard and LogGuard. Add settings here only when product configuration genuinely shares them. Do not put download execution, GPUI entities or C# policy here; consumers own live application state. Source currently uses one large module; no file-size-driven split is prescribed. Public APIs stay independent of UI runtime. Use [shared architecture](../../shared/architecture.md) and existing in-module tests as the change boundary.
