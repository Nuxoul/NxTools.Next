# Protocol quality

Evidence: `repo:rust/crates/nxtools-protocol/src/lib.rs#tests`, `repo:csharp/NxTools.PlatformHost/Protocol/ProtocolEnvelope.cs`.

Roundtrip real enum/DTO payloads, version rejection, exact frame-size boundary and serde default/unknown-field compatibility. For a changed native command verify C# parser and actual owning callback/result, not only encode equality. Bounds/size units (logical/device pixels, bytes/ms, HWND/PID) are explicit in field contract; no name-only authorization. Keep optional source HWND/native result metadata coherent. `cargo test -p nxtools-protocol --lib` and affected IPC/native tests follow [verification](../../shared/verification.md); no new warning suppression.
