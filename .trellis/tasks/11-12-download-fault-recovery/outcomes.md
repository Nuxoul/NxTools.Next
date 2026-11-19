# HTTP segmented-download fault recovery

## Implementation

The HTTP segmented path uses a per-run claim/commit planner and a supervisor of reusable connection workers. Interrupted responses retain written progress; excess refused workers retire without failing viable peers; idle workers split unclaimed tails while the original response continues. The implementation follows the approved [solution](solution.md).

| File | Responsibility |
|---|---|
| `rust/crates/nxtools-download/src/engine/segment_planner.rs` | Contiguous half-open ranges; waiting/timed-adjacent/split assignment; 12KiB minimum halves; 4096 alignment; distinct reserved, written and synced positions. |
| `rust/crates/nxtools-download/src/engine/segment_workers.rs` | Per-connection assignment/retry loop, progress-renewed budgets, cancellable backoff, peer-aware retirement, fatal drain, serialized split persistence and unified two-second checkpoints. |
| `rust/crates/nxtools-download/src/http/segmented.rs` | Shared public/internal receiver; 256KiB batches and one-second flush; 64-byte overlap verification; claim-bounded writes; slow-response replacement; synchronized exits. |
| `rust/crates/nxtools-download/src/engine/segmented_runner.rs` | Existing probe/setup/resume/fallback/publication owners connected to the new supervisor. |
| `rust/crates/nxtools-download/src/engine/connection_ceiling.rs` | Host-only learning within existing route/policy/TLS/System-client scope; two refusal observations, 600-second expiry, bounded cache and captured run ownership. Receiving width excludes completed responses still syncing. |
| `rust/crates/nxtools-download/src/http.rs` | Typed 429/503 throttling, Retry-After parsing, other HTTP status classification and common segmented receiver entry. |
| `rust/crates/nxtools-download/src/artifact.rs` | Writer flush and unified artifact sync. |
| `rust/crates/nxtools-download/src/engine.rs` | Planner/worker registration and removal of the old HTTP adaptive controller. |
| `rust/crates/nxtools-download/src/engine/fault_recovery.rs`, `tests/support/fault_recovery_server.py` | Actual Engine/HTTP/SQLite/file fault scenarios and request/byte accounting. |
| `rust/crates/nxtools-download/src/engine/segment_planner_tests.rs`, `src/http/segmented_tests.rs`, `tests/fault_status_contract.rs` | Planner invariants, receiver policy/storage regressions and real HTTP status rejection. |
| `rust/crates/nxtools-download/src/engine/connection_experiments.rs`, `src/engine/connection_experiments/b601.rs`, `src/engine/connection_experiments/ref13_controls.rs`, `tests/support/b6_server.py` | Public-engine ceiling/lifecycle coverage and bounded native-refusal fixture. |
| `rust/crates/nxtools-download/tests/engine_contract.rs`, `tests/average_contract.rs`, `tests/support/mod.rs` | Resume/checkpoint and reliable/legacy statistics coverage at batched-progress boundaries; finite retry-budget timing. |
| `rust/crates/nxtools-download/tests/b4_paused_url_contract.rs`, `tests/support/b4_url.rs`, `tests/support/b4_url_cases.rs` | Actual partial pause/replacement/reopen fixtures and retained-validator Range assertions including the64-byte verification overlap. |
| `docs/reference-code/download.md`, `docs/reference-code/mister-download-manager.md`, `.trellis/spec/nxtools-download/backend/quality-guidelines.md` | Reference entry chains and current behavioral contracts. |

AB Download Manager's part-split/HTTP worker chain and legacy NxTools `Services/DownloadManagerService.cs` were inspected for the existing migration boundaries. Mister Download Manager's `planner.rs`, `download.rs` and `hosts.rs` supply the approved recovery behavior; its GPL-3.0 source is behavioral reference, not copied implementation, comments, tests or assets. Verified entries and invariants are recorded in the reference notes.

The new scheduler does not change single-stream/media execution, probe/request context, network policy, limiters, queue/completion owners, IPC, UI, settings or database schema. The existing schema14 stays unchanged. HTTP adaptive settings remain serialized but do not drive the new planner.

## Regression evidence

Every Cargo command uses `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0` and `RUSTFLAGS=-C codegen-units=256`; Cargo pipelines run serially.

### Previous scheduler versus recovery scenarios

Command: `cargo test -p nxtools-download --lib engine::fault_recovery -- --test-threads=1 --nocapture`.

The final integrated run passes all seven scenarios. [Previous-scheduler RED log](evidence/fault-recovery-red.log) and [final GREEN log](evidence/download-final-tests.log) retain the executed assertions.
The previous segmented scheduler executed seven scenarios: **0 passed, 7 failed**, exit101. Failures reached behavioral assertions, not compilation:

| Scenario | Previous behavior | Required result |
|---|---|---|
| `body_drop_resumes_from_written_offset` | Retried from an incorrect prefix. | Exact output, written-offset minus64 resume, bounded duplicate bytes; repeated progress also renews a single connection's retry budget beyond five disconnections. |
| `throttle_learns_cap_and_retires_extra_connections` | Task failed on HTTP503. | Complete with cap2; same host on a new port reuses two workers without503. |
| `retry_after_is_honoured_and_cancellable` | Retried before the one-second Retry-After elapsed. | Honor both waits and promptly pause during backoff. |
| `exhausted_connection_retires_while_others_work` | Task failed with HTTP500. | Peer continuation completes; all-failing responses terminate within the finite budget. |
| `slow_connection_is_shed_and_tail_is_split` | Took16.026s in the controlled scenario. | Exact output below4s, live tail requests/aborted excess bodies, no slow replacement under task limiting. |
| `split_while_writing_never_overlaps_and_survives_pause` | No required live split. | Durable contiguous split plan, bounded checkpoints and exact output after resume. |
| `overlap_mismatch_is_fatal` | Published completion despite changed overlap bytes. | Fatal FileChanged and no retry/publication. |

### Additional boundary regressions

- `cargo test -p nxtools-download --test fault_status_contract -- --nocapture`: RED **0 passed, 1 failed**, exit101, because HTTP500 was `RangeNotHonored`; the test also covers429/503 and unchanged artifact bytes. [RED evidence](evidence/fault-status-red.log); final integrated GREEN: **1 passed**, exit0.
- `partial_write_failure_never_replays_the_batch_into_an_adjacent_owner`: a real prefix write followed by IO failure must not replay the consumed batch during exit cleanup. RED overwrote adjacent bytes with `[99,100,204,204]` instead of `[204,204,204,204]`; GREEN preserves adjacent ownership and the old written/synced checkpoint for both partial and fully-written-before-error cases.
- `synced_response_lease_is_not_an_extra_receiving_connection`: three live leases but only two receiving assignments retain cap2. Focused run: **1 passed**, exit0.
- `learned_direct_width_does_not_cross_actual_manual_proxy_route`: focused run **1 passed**, exit0; learned direct cap2 stays separate from the new manual proxy scope and exact outputs/drain remain required.
- `ref13_successful_width_prevents_repeated_extra_connection_refusals`: focused run **1 passed**, exit0; refusal counts `[2,0,0,0]`, server receiving peak2, exact output and drained workers.

Resume fixtures use256KiB payloads with a temporary64KiB/s task cap to establish genuine partial checkpoints; the cap is removed before replacement/resume. Retry exhaustion asserts retained written bytes against the actual artifact rather than requiring a zero checkpoint. Original task/artifact/plan identity, validators, rollback, cancellation and exact final output assertions remain required.

Focused receiver command: `cargo test -p nxtools-download --lib partial_write_failure_never_replays_the_batch_into_an_adjacent_owner -- --nocapture`. The planner and slow-policy unit tests are first-pass coverage of new modules, not an old-engine RED claim. The seven real-engine cases and status contract supply the previous-scheduler RED/GREEN boundary.

## Final verification

| Command | Result | Exit |
|---|---|---|
| `cargo clippy -p nxtools-download --tests -- -D warnings` | Pass | 0 |
| `cargo test -p nxtools-download -- --list` | 414 listed across21 executable targets;0 doc-tests | 0 |
| `cargo test -p nxtools-download --no-fail-fast -- --test-threads=1 --nocapture` | 410 run and passed;0 failed;4 ignored | 0 |
| `cargo fmt -p nxtools-download -- --check` | Pass | 0 |
| `python tests/architecture/spec-contracts.py` | 129 documents,14 workspace owners; navigation/evidence/contracts pass | 0 |
| Scoped `git diff --check` | Pass | 0 |
| `cargo fmt --all -- --check` | Archive-related files outside this task have formatting differences; see Limitations | 1 |
| `cargo check --workspace --tests` | Archive desktop test calls private `navigate`; see Limitations | 101 |

| Target | Listed | Passed | Failed | Ignored |
|---|---:|---:|---:|---:|
| `lib` | 160 | 156 | 0 | 4 |
| `artifact_contract` | 4 | 4 | 0 | 0 |
| `average_contract` | 11 | 11 | 0 | 0 |
| `b4_coverage_contract` | 1 | 1 | 0 | 0 |
| `b4_paused_url_contract` | 11 | 11 | 0 | 0 |
| `b4_request_policy_contract` | 3 | 3 | 0 | 0 |
| `b5_completion_contract` | 9 | 9 | 0 | 0 |
| `domain_contract` | 12 | 12 | 0 | 0 |
| `engine_contract` | 105 | 105 | 0 | 0 |
| `fault_status_contract` | 1 | 1 | 0 | 0 |
| `http_client_contract` | 26 | 26 | 0 | 0 |
| `http_filename_contract` | 1 | 1 | 0 | 0 |
| `http_fixture_contract` | 7 | 7 | 0 | 0 |
| `message_contract` | 16 | 16 | 0 | 0 |
| `queue_order_contract` | 1 | 1 | 0 | 0 |
| `rate_limit_contract` | 2 | 2 | 0 | 0 |
| `resolution_contract` | 15 | 15 | 0 | 0 |
| `runtime_speed_cap_contract` | 8 | 8 | 0 | 0 |
| `server_info_contract` | 11 | 11 | 0 | 0 |
| `storage_capacity_contract` | 3 | 3 | 0 | 0 |
| `storage_contract` | 7 | 7 | 0 | 0 |
| `doc-tests` | 0 | 0 | 0 | 0 |
| **Total** | **414** | **410** | **0** | **4** |

The four retained ignored entries are the real ten-minute expiry observation and the explicit REF-13/REF-14/REF-16 serial measurement entries. They are not counted as passing. Counts use each outer test target's final result, excluding duplicate summaries from subprocesses launched by tests.

Evidence: [test list](evidence/download-final-list.log), [test run](evidence/download-final-tests.log), [counts](evidence/download-final-summary.json), [Clippy](evidence/download-final-clippy.log), [download formatting exit](evidence/download-final-scoped-exit.txt), [workspace exits](evidence/download-final-exits.txt), [workspace check](evidence/download-final-workspace-check.log), [workspace formatting](evidence/download-final-workspace-fmt.log).

## Retired coverage and replacements

| Retired test | Current coverage |
|---|---|
| `adaptive_segments_grow_only_after_stable_speed_samples` | `planner_largest_unclaimed_tail_splits_at_aligned_midpoint`; `slow_connection_is_shed_and_tail_is_split`. |
| `adaptive_segments_use_only_post_growth_samples_for_the_benefit_window` | `planner_split_respects_minimum_and_uses_unaligned_midpoint_when_needed`; `planner_operation_sequences_preserve_complete_tiling`; receiver slow-policy threshold tests. |
| `adaptive_segments_stop_after_a_segment_network_retry` | `body_drop_resumes_from_written_offset`; `exhausted_connection_retires_while_others_work`. |
| `adaptive_segments_respect_the_user_disable_switch` | `stored_adaptive_policy_does_not_disable_http_claim_bounded_tail_splits`; settings serialization remains covered separately. |
| `synced_worker_identity_survives_pending_clear_but_not_stops_or_completion` | `planner_reserved_bytes_survive_split_until_commit_or_release`; `planner_clamps_written_and_synced_independently_and_checkpoints_only_sync`; live split/pause regression. |
| `sampled_runner_shrinks_a_grown_saved_plan_and_resumes_exact_artifact` | `split_while_writing_never_overlaps_and_survives_pause`; `concurrent_ranges_resume_all_persisted_segments_after_engine_restart`; `b4_coverage_contract`. |
| `ref15_current_tail_split_keeps_exact_output_and_durable_plan` | `slow_connection_is_shed_and_tail_is_split`; `split_while_writing_never_overlaps_and_survives_pause`; `b4_coverage_contract`. |
| Ignored `ref15_trial` adaptive experiment entry | The old A/B experiment remains historical evidence; the approved seven-scenario Release comparison is the current performance acceptance gate. |

The old “503 never trains a ceiling” rule is replaced by typed429/503/403/connect refusal learning with assigned peers. Probe-only/status-only failures still cannot prove a ceiling. B601/cache eviction/pause/cancel/route tests retain real sockets, SQLite, files and worker joins. Historical B6 outcome files are unchanged.

## Performance acceptance

The R66 evidence at `.diagnostics/download-bench/20261112-125137-release66-retest/` motivated replacing REF-13's native-only learning and REF-15's retained slow-tail behavior. These are recorded baseline results from the approved solution, not measurements of this implementation:

| Scenario | Recorded R66 | Recorded MDM | New Release / same-day MDM |
|---|---|---|---|
| `slowconn-32m` | 16.04s | 2.43s | Pending |
| `maxconns-50m` | 0/3 success | Not recorded here | Pending |
| `drops-50m` | 0/3 success | Not recorded here | Pending |
| `fast-1g`, `capped-100m`, `latency-100m`, `small-256k` | Existing baseline | Same-day comparison required | Pending |

## Limitations

- Release packaging and the same-day alternating MDM comparison have not been run. Controlled debug regression thresholds are functional evidence, not Release throughput acceptance.
- Workspace test checking is blocked by `rust/apps/nxtools-desktop/src/archive/browser_icon_layout_tests.rs:126` calling private `navigate`, defined in `rust/apps/nxtools-desktop/src/archive/browser.rs:1604` (`E0624`). These archive files are outside this task and were left unchanged.
- Whole-workspace formatting reports differences in `rust/apps/nxtools-desktop/src/archive/browser_drag.rs`, `browser_drag_tests.rs`, `browser_preview_tests.rs` and `rust/crates/nxtools-protocol/src/archive_virtual_drag.rs`. Download-package formatting passes; the archive files were left unchanged.
- No product/native-service launch, package or commit was performed.
