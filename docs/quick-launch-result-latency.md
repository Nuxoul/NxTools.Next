# Quick Launch result-display latency

## Production provider

The launcher requests independent file/directory candidate headroom through `retain_kinds_apart`. `search_launcher_snapshots_while` runs the original production file-kind query, sends its nonempty candidates through the existing `RuntimeEvent::SearchCompleted` and framed `IndexSearchBatch` path, then runs the directory-kind query. Directory candidates and exactly one terminal batch follow. The coordinator forwards each current-lease/current-task response without waiting for terminal completion.

The final candidate set and per-kind scores match the original complete provider; Finder owns display ranking. A no-normal-result query retains the existing explicitly permitted homophone fallback. Single-kind, directory-scoped and recent searches keep their original path. Cancellation before or after the first batch suppresses abandoned subsequent results and terminal delivery.

Source owners:

- `rust/apps/nxtools-indexer/src/lib.rs`: `search_launcher_snapshots_while`, `run_snapshot_search`, `send_search_completion` and original coordinator lease checks.
- `rust/apps/nxtools-desktop/src/quick_launch/search.rs`: launcher request shape and generation-scoped accumulated candidates.
- `rust/apps/nxtools-desktop/src/quick_launch.rs`: current-generation first/terminal provider diagnostics and existing merged-publication owner.
- `rust/apps/nxtools-desktop/src/quick_launch/panel.rs`: original debounce, input, focus and result publication. No layout/control/component change is included.

The reference index is `docs/reference-code/quick-launch-research.md` and `quick-launch-listary-results.md`: Listary's managed launcher awaits providers and SwiftList's managed callback streams. Listary's native search engine source is unavailable; no native implementation claim is inferred from managed references.

## Measured worker and framed transport

Debug production query worker over40,000snapshot rows with20,000distinct matching filenames and20,000distinct directory names,40candidates per kind:

| Milestone | Elapsed |
| --- | ---: |
| First provider batch | 15.079 ms |
| Terminal provider batch | 23.370 ms |
| Receiver finishes framed decoding | 31.217 ms |
| Returned candidates | 80 |

`launcher_search_sends_real_file_matches_before_directory_pass_finishes` compares the complete candidate/score set. `launcher_worker_emits_framed_first_batch_before_terminal` measures the original worker and round-trips its actual framed responses with command correlation. Other launcher regressions cover cancellation and empty/homophone fallback. These four tests pass without injecting ready provider batches.

The separate Desktop ready-batch test validates arrival-to-publication, selection/action eligibility and paint. It cannot measure provider computation. Desktop refresh/selection16tests and the lifecycle source gate pass; deferred cross-entity callbacks, passive preview and focus owners remain unchanged. [Full verification and archive results](audits/2026-11-06-large-count-and-launcher.md).

## Installed-runtime observations

Sanitized installed `desktop.log.2026-11-04` through `2026-11-06` contains75same-generation command-enqueue-to-first-merged-publication pairs: November4:31samples, median177.7ms, maximum411.1ms; November5:42samples, median176.8ms, maximum834.6ms; November6:2samples,276.9–286.7ms. The extract retains only timestamps, generation, booleans and counts. Installed binary correspondence to this working tree was not established.

That timestamp begins after synchronous local Finder search and enqueue. It excludes input/debounce and does not isolate provider, IPC, panel commit or paint. It is not a before/after comparison with the Debug fixture above.

## Verification commands

Use the real toolchain with `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0` and `RUSTFLAGS=-C codegen-units=256` on the single Cargo lane:

```text
cargo test -p nxtools-indexer --lib launcher_ -- --nocapture --test-threads=1
cargo test -p nxtools-indexer --lib search_ -- --test-threads=1
cargo test -p nxtools-desktop search_latency_ready_indexed_rows_publish_before_terminal_batch -- --test-threads=1
cargo test -p nxtools-desktop quick_launch::tests::refresh_tests -- --test-threads=1
```

## Remaining acceptance

The Computer Use tool reports `cua-driver not installed`. Actual input-to-first-visible-results, physical keyboard/focus/DPI and user acceptance remain unverified. File-pass latency and directory-only matches still require installed measurement; first-file publication only removes the wait for the subsequent directory pass. No package, installation, service restart or replacement occurred. The installed application/service has not acquired these source changes, and packaged LocalSystem acceptance remains separate.
