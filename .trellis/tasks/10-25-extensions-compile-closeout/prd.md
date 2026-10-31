# Queue 01 — Extensions compile-blocker closeout

## Goal and boundary

Validate and close only the historical Desktop compile error for RuntimeError::UnsupportedTransport and the unused method_minimum_transport warning. Direct execution in the current session; no delegated work. Current source inspection shows both production repairs already exist: EventOutcome maps UnsupportedTransport explicitly to Unavailable; lib.rs re-exports and uses method_minimum_transport during manifest validation. Preserve those external fixes without repeating or claiming them as this task's implementation.

## Smallest remaining gap

The existing transport declaration test rejects resident execution through the original runtime, and API contracts verify minimum transport and unknown methods/versions. Desktop committed-event history lacks a direct regression for the new error's unavailable/failure classification and original circuit/retry behavior.

Add one test at the existing EventOutcome -> EventControl::record/latest/recent/is_paused/retry boundary. It must prove unavailable classification, no successful terminal, duplicate suppression, three-failure pause and retry without replay. This task does not change transport support, permissions, process launching, user-facing layout, native lifecycle or runtime scheduling.

## Expected own changes

- Append a cfg(test) module to Desktop extensions/committed_events.rs; preserve its original task-start bytes as a prefix.
- New English task documentation and bounded verification evidence only.
- Production event_history.rs, api.rs, lib.rs and runtime.rs remain observed external code, not task edits. A new unrelated change is preserved and reported rather than overwritten.

## Verification

Record the frozen historical E0004/unused warning log, current source hashes and actual current focused results. Do not revert already repaired live code merely to manufacture red. When feasible, replay the frozen historical event-history file against real compiled extension dependencies in scratch; distinguish that replay from current behavior. A production mapping mutation must be detectable by the boundary assertion if replayed in isolation.

Run the new Desktop boundary regression and related committed_events tests; Extensions api_contracts and transport_declarations tests; strict Desktop Clippy --tests -- -D warnings; workspace check --tests; task-scoped fmt/diff and document contracts. No complete Desktop suite. Each Rust invocation inherits CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256 and executes serially after natural Cargo/rustc idle with adequate timeout. Never kill/pause another process or request a resource reservation.

## Evidence and non-goals

Captured live dirty baselines before editing eight relevant files, not HEAD. Retain exact own delta and hashes separately from observed external changes. Historical canceled, blocked and failed records stay immutable. Only in-memory event control and temporary fixtures are allowed; no product/PlatformHost/service, real extension execution, user configuration, network, clipboard, native picker/Shell/Registry, packaging/install or commit/push. Complete only this queue task; subsequent work belongs to later queue dispatch.
