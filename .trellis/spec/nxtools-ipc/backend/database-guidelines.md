# IPC state (SQL not applicable)

Evidence: `repo:rust/crates/nxtools-ipc/src/lib.rs#CommandRouter`, `repo:rust/crates/nxtools-ipc/src/local_pipe.rs#LocalPipeReader`.

Pending command HashMap and connection queues are ephemeral. Reconnect retires old generation/pending results; do not persist or replay native actions from a database. Task durability belongs to Download/Clipboard/Indexer. Caller-assigned IDs cannot overwrite in-flight ID. Initial-frame absolute deadline is disarmed at newline, not extended per trickle byte or enforced forever on healthy sessions. [Quality contracts](quality-guidelines.md).
