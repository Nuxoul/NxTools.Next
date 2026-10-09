# Network persistence (SQL not applicable)

Evidence: `repo:rust/crates/nxtools-network/src/lib.rs#NetworkPolicy`, `repo:rust/crates/nxtools-core/src/lib.rs#update_network_settings`.

Policy is in-memory Arc/RwLock settings plus generation; Core owns durable network JSON. Freeze snapshot once per operation so policy updates affect new operations only. Cached clients in download consumer invalidate by generation/route/certificate and bounded TTL; no independent network database or hidden proxy registry writes. route_key contains internal endpoint identity and must not be logged. Test unchanged active snapshot and changed next snapshot; do not persist cached client state.
