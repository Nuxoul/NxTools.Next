# Protocol diagnostics

Evidence: `repo:rust/crates/nxtools-protocol/src/lib.rs#Envelope`, `repo:rust/crates/nxtools-protocol/src/lib.rs#PlatformEvent`.

Protocol is a serialization library; caller owns lifecycle logging. Envelope/command Debug may contain clipboard/URL/path/native-format values, so never log full messages. IDs/version/kind/frame byte length/native fixed code are sufficient. Carry one operation correlation across IPC/native without swapping task/session/generation identity. Do not log ping/frame streams at high frequency. [Shared diagnostics](../../shared/engineering.md).
