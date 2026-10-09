# IPC structure

Evidence: `repo:rust/crates/nxtools-ipc/src/lib.rs#platform_bridge`, `repo:rust/crates/nxtools-ipc/src/local_pipe.rs#LocalPipeConnection`.

lib.rs owns bounded async notifications/commands, active connection generations, pending IDs, sweep/timeouts and bounded sync/async line helpers. Windows-only local_pipe owns ACL/native handles, actual client PID/session and initial-frame timeout. Domain/native feature actions stay in owning apps; no UI callback state or SQL here. Contract-test seams feed the same production queues, not a parallel router. [Shared architecture](../../shared/architecture.md).
