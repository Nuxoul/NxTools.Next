# IPC errors

Evidence: `repo:rust/crates/nxtools-ipc/src/lib.rs#PlatformBridgeError`, `repo:rust/crates/nxtools-ipc/src/lib.rs#failed_platform_result`, `repo:rust/crates/nxtools-ipc/src/local_pipe.rs#LocalPipeReader`.

Disconnected/Busy/DuplicateCommandId reject before leaked/replaced pending state. Timeout/disconnect terminal notifications are generation-scoped, not empty success. Frame decode/version/native IO errors retain bounded category; broken pipe yields EOF, initial deadline TimedOut. Do not log entire failed frame or turn arbitrary native failure into a retry loop. Feature owner decides retry/focus/error presentation. [Shared engineering](../../shared/engineering.md).
