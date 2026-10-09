# Download IPC errors

Evidence: `repo:rust/crates/nxtools-download-ipc/src/lib.rs#DownloadIpcProtocolIncompatible`, `repo:rust/crates/nxtools-download-ipc/src/lib.rs#DownloadIpcErrorCode`.

ProtocolIncompatible must remain distinguishable from host unavailable; callers rely on typed downcast. InvalidFrame/Internal are wire errors, DownloadFault remains feature code/message/retryable. Require Hello before commands and matching version/request ID. Draining rejects mutating commands with Unavailable/retryable; reads/status can continue. EOF/disconnect is not successful terminal download. [Engineering](../../shared/engineering.md).
