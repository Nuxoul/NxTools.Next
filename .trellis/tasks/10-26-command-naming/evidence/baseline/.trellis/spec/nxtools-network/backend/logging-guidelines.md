# Network diagnostics

Evidence: `repo:rust/crates/nxtools-network/src/lib.rs#proxy_mode_name`, `repo:rust/crates/nxtools-network/src/lib.rs#source`, `repo:rust/crates/nxtools-network/src/lib.rs#route_key`.

Network library exposes mode/source labels for feature-owner lifecycle events and otherwise needn't log client construction. Do not log route_key/manual endpoint, URLs/proxy credentials or raw request errors. Consumer correlation accompanies accepted/start/terminal elapsed metadata; no per-byte/client-pool/frame logs. [Shared privacy](../../shared/engineering.md) applies even when route metadata originates from settings.
