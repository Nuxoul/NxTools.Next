# Quick Launch result-display latency

## Production provider

The launcher requests independent file/directory candidate headroom through `retain_kinds_apart`.
The production name/full-path query shares a matching walk and retains each kind separately, then
uses the original bounded `RuntimeEvent::SearchCompleted` and framed `IndexSearchBatch` path.
Cold filters can still require a complete recovery scan for an underfilled kind; that recovery
does not replace the other kind's complete candidates. Scores, exclusions, delta suppression,
preferred-directory ranking and single-kind/scoped semantics remain unchanged.

Ordinary Desktop input dispatches next turn without a fixed debounce. All current index/extension
providers must finish before the existing merged ranking replaces results once. Partial batches
remain private. Only no ordinary results plus explicit permission enters bounded homophone
fallback, retaining its original combined result cap. Cancellation suppresses abandoned delivery.

Source owners:

- `rust/apps/nxtools-indexer/src/lib.rs`: shared-kind search, cold recovery, `run_snapshot_search` and coordinator lease checks.
- `rust/apps/nxtools-desktop/src/quick_launch/search.rs`: request shape and current-generation accumulation.
- `rust/apps/nxtools-desktop/src/quick_launch.rs`: ordinary local hit probe, provider completion and merged-publication owner.
- `rust/apps/nxtools-desktop/src/quick_launch/panel.rs`: deferred input, focus and result publication. InputState/InputGroup, layout and native lifecycle are unchanged.

The reference index is `docs/reference-code/quick-launch-research.md` and `quick-launch-listary-results.md`: Listary's managed launcher awaits providers and SwiftList's managed callback streams. Listary's native search engine source is unavailable; no native implementation claim is inferred from managed references.

## Historical worker and framed transport fixture (2026-11-06)

Debug production query worker over40,000snapshot rows with20,000distinct matching filenames and20,000distinct directory names,40candidates per kind:

| Milestone | Elapsed |
| --- | ---: |
| First provider batch | 15.079 ms |
| Terminal provider batch | 23.370 ms |
| Receiver finishes framed decoding | 31.217 ms |
| Returned candidates | 80 |

These measurements describe the earlier two-kind-pass implementation. Current `launcher_search_preserves_per_kind_candidates_across_framed_batches` and `launcher_worker_preserves_framed_identity_and_one_terminal` retain complete candidate/score, actual worker, framed correlation and terminal coverage without requiring a second matching pass.

The separate Desktop ready-batch test validates arrival-to-publication, selection/action eligibility and paint. It cannot measure provider computation. Desktop refresh/selection16tests and the lifecycle source gate pass; deferred cross-entity callbacks, passive preview and focus owners remain unchanged. [Full verification and archive results](audits/2026-11-06-large-count-and-launcher.md).

## Current query-work fixtures (2026-11-17)

- The same64-row mixed-kind query returns all64rows while calls at the existing current-query boundary fall from393to199. `launcher_search_matches_both_kinds_with_one_candidate_walk` detects a repeated matching walk without a timing threshold.
- With512synthetic application shortcuts, actual allocation requests around the public local search/probe calls are1,838,199bytes for complete ranking and2,053bytes for the short-circuit existence probe. Setup and catalog construction are excluded; built-in system entries remain present.
- Independent-kind result comparisons cover cold underfill, exclusions, delta changes/deletes, scopes, limits and preferred-directory scoring. Local probe parity covers catalog/instant/command/package/environment sources; final merged ranking is unchanged.
- These are Debug synthetic work/allocation observations, not resident-memory measurements or installed input-to-visible latency.

## Installed-runtime observations

Sanitized installed `desktop.log.2026-11-04` through `2026-11-06` contains75same-generation command-enqueue-to-first-merged-publication pairs: November4:31samples, median177.7ms, maximum411.1ms; November5:42samples, median176.8ms, maximum834.6ms; November6:2samples,276.9–286.7ms. The extract retains only timestamps, generation, booleans and counts. Installed binary correspondence to this working tree was not established.

That timestamp begins after synchronous local Finder search and enqueue. It excludes input/debounce and does not isolate provider, IPC, panel commit or paint. It is not a before/after comparison with the Debug fixture above.

## Verification commands

Use the real toolchain with `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0` and `RUSTFLAGS=-C codegen-units=256` on the single Cargo lane:

```text
cargo test -p nxtools-indexer --lib launcher_ -- --nocapture --test-threads=1
cargo test -p nxtools-indexer --lib search_ -- --test-threads=1
cargo test -p nxtools-desktop quick_launch::tests::normal_search_tests -- --test-threads=1
cargo test -p nxtools-desktop quick_launch::tests::refresh_tests -- --test-threads=1
```

## Remaining acceptance

No package, installation, protected-tuple replacement or real service restart is authorized by these fixtures. The unchanged installed application/service does not contain the new source changes. Installed input-to-visible A/B, physical input/focus/DPI and packaged LocalSystem restart/base+delta/idle CPU+I/O/search/hash acceptance remain separate. Never certify new-source service behavior by testing the old installed binary.
