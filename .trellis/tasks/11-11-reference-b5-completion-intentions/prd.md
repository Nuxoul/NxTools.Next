# B5 — Completion intentions

## Scope

Implement only REF-07 task completion action expansion and REF-11 queue-level completion intentions. B1–B4 remain accepted at their recorded boundaries. Preserve the original request/task/plan/checkpoint/artifact/queue/SQLite/IPC/window owners, B4 default Header/Reset, replacement revision/validator, templates and actual coverage, existing browser admission, limits, Server and archive behavior.

Stop after B5 at **ready_for_acceptance / not_accepted**, pending independent coordinator acceptance. B6/B7 are excluded. Canonical Archive remains **Phase E — Format expansion and ZIP editing / in_progress**. Coordinator-owned acceptance reports and `docs/reference-improvement-coordination.md` are read-only.

## REF-07 — Task completion intentions

- Add Sleep, Hibernate and normal application Exit to the existing applicable task completion actions.
- Persist intention with the original task/settings owner. Closing/reopening progress presentation must not clear intention or alter task execution state.
- Trigger only from genuine successful completion. Failure, cancellation and window close never trigger completion actions.
- Power and Exit use the existing single completion coordinator and cancellable countdown. The countdown names its action and exposes an accessible explicit Cancel.
- Actual acceptance of new work cancels any pending action. Concurrent task completions cannot race into duplicate execution or competing countdowns.
- Unsupported, permission-denied and native-failed actions show accurate unavailable/failure states, without success or silent substitution.
- Exit follows the existing normal shutdown/resource-drain chain, not force termination. Requested close is not proof of actual exit.
- Rust owns intention/coordination, GPUI presentation/user input, C# Windows power APIs.

## REF-11 — Queue completion intentions

- Persist intention with the original queue owner, independently of the last visible task window.
- Support applicable REF-07 actions and explicit Open directory feedback through the existing coordinator and bounded directory deduplication.
- Initial automatic execution requires all work accepted into that queue's current completion round to succeed. Waiting, scheduled, failed or cancelled work cannot be treated as successful queue completion. An empty queue is not a completed round.
- Distinguish durable intention, current round and one-shot pending action. Startup/reload of old terminal records cannot execute early or replay an action.
- Multiple queues share one power/Exit countdown. Use one documented stable merge rule.
- New task insertion, retry, cancellation/removal, queue switching and explicit countdown Cancel update eligibility through the original owners.
- Any additive persistence/wire fields use original stores/commands and missing-field compatibility defaults. No alternative state database or transport.

## Interaction and persistence

Use exact released official Kit0.8.0 Settings/Form/Menu/Dialog/Button components and public slots. Record the layout shape, visible variant/size and parent adaptations. Retain original task identity, progress window, same-window completion, focus/IME, cancellation, current geometry and theme. Defer GPUI cross-entity work to the next application turn, including immediate/cache-hit results.

Configuration changes use the owning original transaction and merge only changed fields. Persist before publishing; failure retains draft, previous state and truthful error.

Verify pointer, first focused Enter/Space, Escape, repeated activation, presentation close/reopen and minimum supported bounds. Discover actual focused controls rather than fixed Tab counts.

## Required acceptance evidence

Use real original task/queue, SQLite, completion coordinator and production TestPlatform boundaries for:

- Saved intention, window close/reopen and engine/application reload.
- Genuine success versus failure/cancel; simultaneous tasks and queues.
- Waiting/scheduled work, empty queue, insertion of new work and queue switching.
- Exhausted retry, cancellation/removal, restart and no premature/duplicate execution.
- Explicit countdown Cancel, new-work cancellation, stale generation and late reply rejection.
- Existing bounded directory feedback deduplication.
- Normal Exit ordering across tasks/writers/IPC and actual cleanup observations.
- Native support/unavailable/success/failure/permission and exact native error-code propagation.

Add a real behavior regression before each missing-behavior implementation, then run the identical selection after the fix. Compiler/API/fixture prerequisites are not product REDs; first-pass coverage is not invented RED. DTOs, screenshots and source tokens alone do not establish lifecycle correctness.

All ordinary tests inject results at the original typed native boundary and verify calls/state. They must not actually shut down/sleep/hibernate or exit the user's desktop process. C# harness selections require reviewed effects, exact nonzero LIST and identical RUN; do not select tests that perform real power effects. Controlled results are not physical/native power or physical Exit acceptance.

## Diagnostics and validation

Carry original task/queue/correlation/generation through intent acceptance, meaningful eligibility changes, countdown start/cancel reason, execution request, terminal outcome, elapsed time and native code. No URL/path/Header/Cookie/credential/content payloads and no per-frame/countdown-tick logs.

Run affected Download/Core/IPC/Protocol/Desktop/PlatformHost tests and necessary preserved Download-IPC/callers, strict lint/check, scoped formatting and the four spec/component/Kit/editor source gates. Each behavior selection must be nonzero LIST then identical RUN; SKIP/not_run is not PASS. Attribute existing unrelated lint/history separately without suppression or unrelated repairs.

One Cargo lane. Every Rust execution initializes jobs1/incremental0/codegen-units256 and scratch TEMP/TMP in that same PowerShell process, uses a full-operation timeout and checks survivors before retry. Temporary files remain in session scratch.

## Delivery

Record exact changed paths, original owner chains, real RED/GREEN, exact commands/counts/logs, actual owned-resource exit and remaining acceptance levels. No connection/performance experiments, product/reference GUI launch, package/install, service/registry changes, real power actions, commit or push.
