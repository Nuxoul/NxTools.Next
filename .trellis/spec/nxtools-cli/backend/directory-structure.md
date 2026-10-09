# CLI structure

Evidence: `repo:rust/apps/nxtools-cli/src/main.rs`, `repo:rust/apps/nxtools-cli/src/lib.rs#parse_invocation`, `repo:rust/apps/nxtools-cli/src/lib.rs#CliCommand`.

main is process/output composition; library parses invocation/options to typed command and controls DownloadIpcClient. CliHostPolicy ConnectOrStart vs ConnectOnly is explicit; only startup-capable caller may launch host. Use DownloadTaskId/request/schema helpers rather than handwritten engine state. Native Windows actions and SQLite writes stay with owners. No new feature simply by shell passthrough. [Architecture](../../shared/architecture.md).
