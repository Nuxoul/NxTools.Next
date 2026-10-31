# IPC diagnostics

Evidence: `repo:rust/crates/nxtools-ipc/src/lib.rs#PlatformNotification`, `repo:rust/crates/nxtools-protocol/src/lib.rs#Envelope`.

Log connection generation, command/correlation ID, fixed stage, queue/timeout counts and elapsed/native codes. No serialized Envelope, clipboard/path/URL/query/capability values or raw malformed frame. Coalesce repeated connection failures and do not log ping/frame streams. One correlation crosses IPC and native owner; task/session/generation remain distinct. [Shared diagnostics](../../shared/engineering.md).
