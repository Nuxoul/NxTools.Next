# Download IPC diagnostics

Evidence: `repo:rust/crates/nxtools-download-ipc/src/lib.rs#DownloadIpcFrame`, `repo:rust/crates/nxtools-download-ipc/src/lib.rs#command_envelope`.

Caller/engine owns correlated feature events. Transport metadata can include readiness/generation/sequence/client count and fixed failure code, not full DownloadCommand/request/header/path or events with filenames. No byte/progress frame spam. Requested CLI/browser result output remains separate from diagnostic log. [Shared privacy](../../shared/engineering.md).
