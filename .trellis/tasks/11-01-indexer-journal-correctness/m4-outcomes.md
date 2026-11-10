# M4 — responsive authenticated replacement sessions

## Ownership and behavior

The Windows service owns one coordinator for its lifetime. It retains the original active build, coalesced pending build, compaction set, journal schedule, search executor and immutable snapshots across desktop connections. A disconnect or failed writer retires an endpoint, not the coordinator or its worker completions. Completed snapshots publish without a client and remain searchable after reconnect.

An authenticated, valid Hello registers a monotonically assigned service-side lease before HelloAck and current status are queued. Takeover cancels the prior search task and resets the incoming client's generation once. Requests, EOF, failures and search completions carry their originating lease; old leases cannot alter the replacement client's generation or receive/reroute its results. Commands use the current endpoint's handler and its captured authenticated native identity.

Each admitted endpoint has a 16-frame output queue. The coordinator only performs nonblocking queue delivery; it holds no state lock across write/flush. A full queue retires that endpoint. Native service writes use PIPE_NOWAIT with a 500ms absolute frame deadline and retirement checks. Idle readers observe retirement through the existing PeekNamedPipe loop. The admission slot remains owned until the writer loop and reader worker have both exited: four slots bound endpoint workers, including replacement teardown.

The existing actual PID/session/path/hash authorization remains before registration, together with the three-second initial-frame deadline, four admission slots and 1MiB input frames. No HelloAck is synthesized from a status cache. Standalone/stdio composition and service sessions execute the same run_coordinator, rather than maintaining separate build/search/maintenance loops.

The lifetime change is necessary because a command-scoped service mutex would still leave build completion and maintenance ownership in the old session. The service-owned coordinator permits B to attach while A's asynchronous scan and stopped output remain held, without losing the completed scan or starting a duplicate one.

## Source map

- `rust/apps/nxtools-indexer/src/lib.rs:154,197`: lease-tagged endpoint events and search jobs.
- `rust/apps/nxtools-indexer/src/lib.rs:1929,1973`: standalone composition enters the extracted original coordinator; the service also calls this coordinator.
- `rust/apps/nxtools-indexer/src/lib.rs:2087,2101,2184,2202,2286,2572`: registration, current-lease request/terminal handling and lease-scoped search completion. Build/compaction completions remain service-owned.
- `rust/apps/nxtools-indexer/src/session_runtime.rs:8,70,143,152`: shared lifetime owner, admitted I/O workers, endpoint state and output routing.
- `rust/apps/nxtools-indexer/src/session_runtime.rs:198,208,228,241,290`: lease assignment, correlated accepted/retired/read-failure diagnostics, nonblocking bounded output and retirement-aware input delivery. Writer terminal diagnostics retain correlation, elapsed time, fixed reason and native error code where present.
- `rust/apps/nxtools-indexer/src/windows_service_host.rs:133,152,167,191,213`: one runtime; unchanged native identity admission and initial Hello deadline; retiring native I/O replaces the lifetime service mutex/status cache.
- `rust/crates/nxtools-ipc/src/local_pipe.rs:57,74,99,401`: retiring endpoint conversion, bounded writer, idle reader retirement and shared deadline-write implementation. Existing secondary desktop writes retain blocking-mode restoration.
- `rust/apps/nxtools-indexer/src/tests.rs:46,3039,3253,3350,3404,3504`: retiring ChannelReader plus five actual admitted-runtime regressions.
- `rust/crates/nxtools-ipc/src/local_pipe.rs:732,760,786`: temporary native-pipe reader retirement, nonreading write deadline and retirement-before-deadline regressions.

Build/compaction completion publishes through the original coordinator. The integrated M3 cleanup waits for worker quiescence; U4 projects service-owned maintenance before status delivery.

Reference basis is the completed review recorded in `docs/reference-code/quick-launch-research.md`, section `2026-11-01 journal correctness and maintenance status`: Listary managed session/status boundary only, SwiftList inspectable independent pipe/session and snapshot ownership, and no matching legacy NxTools indexer entrypoint. Listary native engine source was unavailable.

## Regression evidence

All Cargo invocations were serialized, offline and used the following environment in the same PowerShell process, with a21600second tool timeout:

```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
$env:CARGO_NET_OFFLINE='true'
```

Logs below are absolute files under `D:\.pi-desktop\data\scratch\6a21b011-4230-4aa5-a2ed-f2dc1b4153e6\`.

| Check | Actual command | Result | Log filename |
| --- | --- | --- | --- |
| Original-behavior build | `cargo test -p nxtools-indexer --lib m4_admitted_replacement --no-run` | Exit0, compiled | `indexer-m4-red-build.log` |
| Original-behavior regression | `cargo test -p nxtools-indexer --lib m4_admitted_replacement -- --test-threads=1` | Exit101;0passed,1failed | `indexer-m4-red.log` |
| Final interface build | `cargo test -p nxtools-indexer -p nxtools-ipc m4_ --no-run` | Exit0, all selected targets compiled | `indexer-m4-final-build.log` |
| Final M4 regressions | `cargo test -p nxtools-indexer -p nxtools-ipc m4_ -- --test-threads=1` | Exit0;5Indexer+3IPC passed | `indexer-m4-green-final.log` |
| Existing runtime contracts | Command below | Exit0;15passed | `indexer-m4-runtime.log` |
| Existing service admission | `cargo test -p nxtools-indexer --bin nxtools-indexer windows_service_host::admission_tests -- --test-threads=1` | Exit0;2passed | `indexer-m4-admission.log` |
| Native local-pipe contracts | `cargo test -p nxtools-ipc --lib local_pipe::tests -- --test-threads=1` | Exit0;8passed | `indexer-m4-local-pipe.log` |

The original-behavior run used the production shared admitted-session seam retaining the original lifetime mutex. It reached this specific assertion, not a compilation failure or mutex simulation:

```text
B HelloAck and current status must arrive within 500ms while A is nonreading and its scan is held
```

The green admitted regression checks B's acknowledgement/status within500ms, A's two I/O workers exiting while the scan remains held, exactly one completed retained snapshot searchable from B, generation1 after a stale A generation999request, and rejection of old EOF/error/delayed search completion. The delayed old completion is delivered through the actual coordinator queue with B's current task ID and A's old lease. Additional tests prove publication without any client, current authenticated-handler replacement, output saturation isolation and invalid-Hello rejection without resetting the current generation.

The native temporary-pipe tests keep their peer connected while proving idle reader retirement within500ms, a100ms write deadline completing within750ms, and retirement interrupting a five-second write deadline within500ms. These are compiled native fixture results, not installed-service acceptance.

Existing runtime command:

```powershell
cargo test -p nxtools-indexer --lib -- tests::a_reconnected_desktop_can_restart_its_query_generation tests::a_new_generation_makes_older_search_requests_stale tests::a_completed_mft_snapshot_is_searchable_through_the_process_protocol tests::preadmitted_session_rejects_non_hello_before_state_or_ack tests::worker_results_are_partitioned_into_ordered_complete_batches tests::a_pending_explicit_rebuild_absorbs_later_missing_volume_work tests::the_service_loop_publishes_usn_changes_before_the_next_search tests::continuous_protocol_traffic_cannot_starve_journal_maintenance tests::a_panicking_snapshot_worker_reports_failure_instead_of_stalling tests::closing_search_executor_cancels_a_completion_blocked_by_backpressure tests::the_real_coordinator_ tests::an_initial_snapshot_finishes_and_is_retained_after_the_client_disconnects tests::a_slow_initial_snapshot_does_not_block_handshake_or_ping tests::w4_classifier_budget_failure_is_correlated_through_search_worker --test-threads=1
```

There are30distinct passing tests across the final selections. The three new native pipe cases also run inside the eight-case local-pipe selection and are not counted twice.

Scoped formatting passed with `rustfmt --edition 2024 --config skip_children=true --check` on the five changed Rust files; log `indexer-m4-rustfmt.log`, exit0. Scoped `git diff --check` passed; log `indexer-m4-diff-check.log`, exit0.

## Acceptance limits

- The hard responsiveness bound covers asynchronous scan/rebuild ownership and stopped client I/O. Arbitrarily stalled synchronous native journal, persistence or command-handler calls are not bounded by this change.
- Tests use synthetic snapshots/scanners/pollers and temporary local pipes. The production-scanner stdio fixture and all real-volume/product-process cases are excluded.
- Final integrated verification includes the five admitted-runtime regressions in133passing safe Indexer library tests, two admission tests and the three native retirement cases in48passing IPC tests. Exact final commands and remaining release gates are in outcomes.md.
- No installed service, product launch, real-volume scan, packaging or installation ran. Packaged LocalSystem idle/restart/base+delta/search acceptance remains pending authorization.
