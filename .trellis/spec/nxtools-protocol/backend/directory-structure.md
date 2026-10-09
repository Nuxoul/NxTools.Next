# Protocol structure

Evidence: `repo:rust/crates/nxtools-protocol/src/lib.rs#Envelope`, `repo:rust/crates/nxtools-protocol/src/lib.rs#Message`, `repo:rust/crates/nxtools-protocol/Cargo.toml`.

lib.rs defines Envelope, tagged snake_case message/command/event enums, native DTOs and encode/decode validation. Keep transport scheduling, Windows APIs and live feature state out. One typed field has one canonical shape across Rust and C# ProtocolEnvelope/parser; don't cast raw data independently in multiple consumers. Optional/default additions require compatibility tests; breaking field behavior requires approved version contract. [Shared architecture](../../shared/architecture.md).
