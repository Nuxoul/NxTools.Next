# Indexer errors

Evidence: `repo:rust/apps/nxtools-indexer/src/lib.rs#JournalMaintenance`, `repo:rust/apps/nxtools-indexer/src/windows_service_host.rs`, `repo:rust/apps/nxtools-indexer/src/settings.rs`.

Native scan/IO/cache uses anyhow context; wire exposes bounded ProtocolFault/status. First failed build with no snapshot cannot claim Ready; maintenance keeps last-good searchable and records actual poll/persist/worker errors separately from rebuild reason. Reject unknown/session0/unapproved PID/path/hash before mutex; Hello payload cannot authorize. Four pending slots/three-second initial frame are bounds, not a recovery security bypass. No unlimited reconnect/scanner/worker replacement. Context may carry path—sanitize diagnostics. [Engineering](../../shared/engineering.md).
