# Download IPC state (SQL not applicable)

Evidence: `repo:rust/crates/nxtools-download-ipc/src/lib.rs#DownloadHostActivity`, `repo:rust/crates/nxtools-download-ipc/src/lib.rs#DownloadHostSnapshot`.

Atomic readiness/client activity and engine generation/sequence are live projection, not persistent records. Download owns SQLite task/queue/transfer/idempotency data. Hello may carry generation/sequence for synchronization; authoritative snapshot/event recovery replaces guesses, not a transport journal. No SQL schema or durable queue in this crate.
