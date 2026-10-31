# Protocol persistence (SQL not applicable)

Evidence: `repo:rust/crates/nxtools-protocol/src/lib.rs#Envelope`, `repo:rust/crates/nxtools-protocol/src/lib.rs#IndexerCommand`.

Wire DTOs are not database models and own no SQL/schema. JSON line transport state lives in IPC; download durable state lives in Download; index settings/cache in Indexer. Don't add persistence or default recreation to decode_line. MAX_FRAME_BYTES is a wire bound, not permission to log contents. Request IDs/correlation remain independent from persisted task identity.
