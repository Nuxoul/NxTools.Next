# Network structure

Evidence: `repo:rust/crates/nxtools-network/src/lib.rs#NetworkPolicy`, `repo:rust/crates/nxtools-network/src/lib.rs#NetworkOperationSnapshot`, `repo:rust/crates/nxtools-network/Cargo.toml`.

One lib owns normalized/validated NetworkSettings behind shared lock, monotonic generation, immutable route snapshot and reqwest client construction. It depends on Core settings, not Desktop/download execution. Direct no_proxy, system default and explicit manual route are deliberate branches. Consumer owns HTTP/probe deadlines, certificate policy and operation logs. No new global mutable request header/credential state. Keep [shared boundaries](../../shared/architecture.md).
