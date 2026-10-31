# Queue 01 — Extensions compile-blocker closeout outcome

## Outcome and exact attribution

The two requested historical blockers are resolved in the observed current source and passed the requested compiled checks. They were already repaired by independent work before this task's first edit:

- Desktop EventOutcome::from_result explicitly maps RuntimeError::UnsupportedTransport to Unavailable; no wildcard or success fallback is used.
- Extensions lib.rs publicly re-exports method_minimum_transport and uses it in manifest requirement validation. api.rs and its existing method/version semantics were not edited here. The unused-function warning did not recur in the performed Extensions tests or strict Clippy.

This task did **not** implement or claim those external production repairs. Its only existing-source change is a 56-line appended cfg(test) module in extensions/committed_events.rs. The original 18,134-byte dirty file is an exact prefix. All production logic, permissions, transport policies, process supervision and UI layout remain unchanged by this task. Live dirty baselines for eight inspected files were captured before editing; independent changes to event_history.rs/runtime.rs and Settings during verification are separately observed, never overwritten or attributed to this task.

## Added direct regression

`compile_closeout_unsupported_transport_is_unavailable_failure_without_replay` follows the original EventOutcome -> EventControl::record/latest/recent/is_paused/retry boundary with in-memory synthetic events. It asserts:

1. Unsupported transport is Unavailable and not a successful terminal.
2. Repeated terminal recording stays deduplicated.
3. Three admitted failures pause the original circuit.
4. Explicit retry resumes the circuit without replaying history.

Existing API contracts independently exercise the public minimum-transport lookup, supported/unknown method/version requirements and unchanged legacy identity. Existing transport declarations reject unsupported resident execution through the original runtime without launching the fake interpreter or broker.

## Historical red and current verification

The immutable prior New Download checkpoint log records the original E0004 and unused-function warning. No live source was rolled back to manufacture a new failure after the external repairs.

A scratch-only compiler replay used the hash-verified historical event_history.rs and real compiled nxtools_extensions types. It failed with E0004 for missing UnsupportedTransport (and the independently added InvalidSettings variant). The current event-history snapshot compiled against the same dependency selection, and its direct mapping assertion passed1/1. This is bounded source/dependency replay, not a new complete Desktop run.

An optional isolated mutation probe for the full EventControl regression did **not** reach a runtime assertion: two scratch compilation attempts selected incompatible uuid versions from the shared target directory (E0308). Those failures are retained as fixture/dependency errors, not behavioral reds or successful mutation testing. The authoritative new EventControl regression subsequently compiled and passed in the real Desktop crate. No live production mapping was mutated.

## Performed commands and results

All Rust invocations, including rustfmt/rustc replay, inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256. Cargo/rustc were checked for natural idle; one auxiliary replay attempt stopped before starting a compiler because another build was active. The later natural wait completed without killing/pausing processes or requesting resource reservations. Commands ran serially with adequate timeouts.

| Command / seam | Actual result |
| --- | --- |
| cargo test -p nxtools-extensions --test api_contracts --test transport_declarations -- --test-threads=1 | PASS3+3, zero ignored; initial and final scoped runs both pass |
| Initial cargo test -p nxtools-desktop compile_closeout_ -- --test-threads=1 | FAIL exit101 before tests: external extensions_settings.rs used SharedString without its import; not the requested error and not this task's behavioral red |
| Final same Desktop focused command | PASS1/1, zero ignored, after independent Settings import repair;220.95s whole command including compilation |
| cargo test -p nxtools-desktop extensions::committed_events:: -- --test-threads=1 | PASS4/4, zero ignored; includes new regression and original history/circuit/commit boundaries |
| cargo clippy -p nxtools-desktop --tests -- -D warnings | PASS exit0,43.65s |
| cargo check --workspace --tests | PASS exit0,61.18s |
| cargo clippy -p nxtools-extensions --tests -- -D warnings | PASS exit0,9.23s |
| rustfmt --edition 2024 --config skip_children=true --check committed_events.rs | PASS; initial new assertion wrapping failure was corrected only in appended lines |
| Scoped git diff --check | PASS |
| cargo fmt --all -- --check | FAIL exit1: external main_window/extensions_settings.rs formatting at739; not formatted or changed by this task |

The missing SharedString import was independently corrected while this task verified current code. Its first failed log is retained; the later success is not attributed to this task and does not erase that failure. The observed external source changes are captured separately from own delta. The final formal tests use actual compiled project owners; replay warnings for unused isolated snapshot items are not project Clippy results.

No complete Desktop suite was run. Earlier complete-suite failures1237/1, MainWindow185/1, QuickLaunch source-gate failure, canceled deliveries and rem checkpoints remain historical records and receive no new acceptance from this task. No New Download rem test, UI redesign or later queue task was started.

## Evidence and limits

`evidence/baseline-manifest.json` names the captured dirty baselines. The task's append-only delta reconstructs forward and reverse; old source prefix is independently checked. Evidence includes raw successful/failed logs, replay source/dependency hashes, interim/final external observations, and current source snapshots. The evidence manifest hashes documents and artifacts without updating any previous task's manifest.

Source/compiled unit and temporary/in-memory fixtures only. No subagent or independent session was used. No product/PlatformHost/service launch, real extension execution, user configuration/database, network/download/archive/clipboard/picker/Shell/Registry/native operation, packaging/install, commit or push occurred.

Requested compilation/strict-check blockers are closed at the performed verification point. Remaining separate boundary: workspace-wide formatting is not green because of independent Settings edits; this task intentionally leaves it untouched. Other queue tasks and native/physical/DPI/visual/user/release acceptance remain pending their own dispatch/authorization.
