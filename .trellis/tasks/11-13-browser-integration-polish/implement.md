# Implementation

User approved implementation, task creation and tests at the existing production event/message and Popup/content DOM boundaries.

## Ordered slices
1. Non-GET retention: add a red production-listener regression, preserve method evidence, reject before credentials/native demand, retain GET/redirect compatibility.
2. Connection diagnosis: red public message/Popup regressions, fixed native error classification and actionable copy; retain transport lifetime.
3. Explicit link batches: red extraction/runtime and DOM selection tests, bounded same-page cache publication, review in Capture and original batch retry.
4. Demand-only media inspection and Popup shortcuts: red explicit-message/DOM tests, bounded page-context inspection, retry/protection summaries, local search/view persistence/home pause.
5. Full extension tests, production DOM regression, scoped native diagnostic tests, one bounded correctness review, spec/reference updates and scoped commit.

## Verification
- Targeted `node --test` files per vertical slice: incorrect browser cancellation, native reconnect, unsafe source publication or unintended fetch blocks progression.
- Recursive `*.test.js`: existing Cookie, cancellation, Blob, batch, sender, cache and discovery regressions.
- Production DOM gates in temporary profiles with synthetic browser APIs/network blocking: visual geometry, keyboard/focus and asynchronous states. Assets/intermediate evidence go to session scratch.
- Rust verification, if diagnostic schema changes, inherits `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`; no parallel Cargo invocation.
- `python tests/architecture/spec-contracts.py` checks documentation contracts after spec edits, not browser/product acceptance.

## Exclusions
No packaging, registry/profile modification, real credentials/downloads, downloaded reference code incorporation or blanket formatting of unrelated files.
