# CLI persistence (SQL not applicable)

Evidence: `repo:rust/apps/nxtools-cli/src/lib.rs#CliCommand`, `repo:rust/crates/nxtools-download-ipc/src/lib.rs#DownloadIpcClient`.

CLI owns no SQL/history. Engine/store persists tasks, schedule, idempotency and replacement IDs; CLI sends typed commands and renders authoritative results/events. Don't read or edit engine SQLite/config to bypass IPC. `--connect-only` cannot create product runtime; host-unavailable is an explicit exit. Local command parser tests use provided temp default_directory without probing real user files.
