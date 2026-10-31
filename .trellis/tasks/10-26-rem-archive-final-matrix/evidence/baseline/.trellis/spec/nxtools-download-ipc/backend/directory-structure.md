# Download IPC structure

Evidence: `repo:rust/crates/nxtools-download-ipc/src/lib.rs#DownloadIpcClient`, `repo:rust/crates/nxtools-download-ipc/src/lib.rs#run_download_host`.

One library owns current-user named pipe, Hello/version handshake, readiness/activity and typed command/result/event routing. `command_envelope` creates command request identity; host delegates to DownloadEngine, never implements transfer policy. Client/readiness state isn't a second engine. BrowserHost and CLI depend on this boundary; avoid host-start or native window logic here. [Architecture](../../shared/architecture.md).
