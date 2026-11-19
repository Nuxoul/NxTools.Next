# Release initial-index build

## Result

The isolated C: Release base-build pipeline completed in **9,138.53ms** and returned its first verified search hit in **9,144.27ms**, both below the10,000ms target. It indexed1,623,169visible rows and durably wrote368,371,842bytes before reopening the exact new file and publishing it to an isolated in-process IndexerService.

| Metric | Release baseline | Optimized Release |
| --- | ---: | ---: |
| Visible rows | 1,623,158 | 1,623,169 |
| Complete base build and publication | 12,435.12ms | 9,138.53ms |
| First successful real search, from build start | 12,440.02ms | 9,144.27ms |
| First query execution | 4.86ms | 5.70ms |
| Raw MFT block reads | 7,192 | 1,875 |
| Full-ID attribute reads / freshness exceptions | 145 / 145 | 69 / 69 |
| Independently verified system-file identities/times | 3 | 3 |

Both runs used compiler opt_level3 with debug assertions disabled. The previously reported38,530ms was an unoptimized Debug fixture constructing a searchable in-memory snapshot; it was not a Release production-pipeline measurement and did not include durable persistence/publication.

## Phase costs

| Phase | Release baseline | Optimized Release |
| --- | ---: | ---: |
| Native enumeration, including metadata | 7,456ms | 6,258ms |
| Metadata subset of native enumeration | 4,086ms | 2,810ms |
| Path/entry construction | 3,185ms | 1,081ms |
| Scan subtotal: enumeration plus construction | 10,642ms | 7,340ms |
| Durable persistence | 1,515ms | 1,583ms |
| Exact mapped reopen | 276ms | 214ms |
| Isolated publication and cleanup | 0.21ms | 0.24ms |

The metadata and scan rows are inclusive, not additional work to sum. Native enumeration remains the largest stage. Persistence retains its original durable-write boundary.

## Implementation

- `mft::unsorted_entries_from_records` reuses traversal scratch, borrows cached parent paths, allocates the completed path at its exact capacity and moves original names/paths into entries. Full-FRN duplicate selection, root sequence handling, orphan/cycle omission and self-parent behavior remain.
- The persistence-only constructor does not path-sort an unpublished base. The existing writer owns full-FRN row order, child order and masks. Direct searchable owned snapshots retain path sorting. Exact mmap row identities, children and search candidates are unchanged.
- `bulk_times::BulkTimes` batches up to256clusters, capped at1MiB plus alignment storage. The prior64-cluster configuration read256KiB on a4KiB-cluster volume. Sector alignment, fragmented-extent boundaries, record reassembly, complete USA validation, full identity and true STANDARD_INFORMATION LastWriteTime checks remain.
- Raw standard-information USN still must match the enumerated USN. Different/missing USN uses the existing full-ID current-attribute reader. Invalid metadata remains unknown; native read errors fail the build and retain last-good. Incremental USN budgets and cancellation are unchanged.

Names, hard-link selection, query ranking, cache/protocol formats, volume concurrency and service lifecycle are unchanged. No production per-file logs, new dependencies or query-time volume work were added.

Reference: `docs/reference-code/listary-initial-indexing.md`. The fixed Listary6.3.5.94 export exposes the active managed-to-native boundary and a retained historical managed MFT scanner; its current native engine source is unavailable. The path-construction and writer-order decisions are verified Next ownership, not inferred Listary internals.

## Verification

The Windows-only fixture `mft::cache::initial_profile_tests::initial_build_release_phase_profile` is ignored by default, rejects Debug, requires explicit C: authorization and writes only a fresh scratch index. It calls the production constructor-selection/scanning seam, real durable writer, exact mmap reopen, isolated service publication and real search. It checks all six existing phase events and uses a run ID for the numeric report.

Three stable system files are independently opened before/after scanning to verify full file identity and true modification time. The first query must find the actual indexed primary path for the witness identity, including hard-link alias behavior. All69full-ID attribute reads in the final run were freshness exceptions. Eleven raw metadata rows remained unknown under the existing invalid/missing metadata rules.

Resource regressions use4160synthetic rows, with setup and oracle allocations outside observation:

| Boundary | Reference | Optimized |
| --- | ---: | ---: |
| Path-construction allocation calls | 29,061 | 4,166 |
| Path-construction requested bytes | 2,659,604 | 1,985,367 |
| Sorted entry construction vs persistence-only requested bytes | 1,985,367 | 1,519,826 |
| Full searchable vs writer-only construct/persist/reopen requested bytes | 3,853,858 | 2,811,798 |
| Full searchable vs writer-only pipeline allocation calls | 7,532 | 5,691 |
| Sequential1025-record raw reads | 5 | 2 |

These are cumulative current-thread allocator requests, not retained heap or peak private commit. Byte-backed regressions also cover fragmented/cross-block records, reused identities, malformed metadata, sparse regions, bounded growth, native failure and pre-cursor dirty metadata.

Final checks:

- Indexer library:173passed,0failed,5ignored; two old real-volume entry points explicitly filtered.
- Strict Indexer tests Clippy passed.
- Debug and optimized Release persistence/reopen/publication/search seam passed.
- Spec contracts, Indexer lifecycle, GPUI release contracts and Markdown editor contracts passed.
- Scoped Rust formatting passed.

All Cargo commands used `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`. The final Release build preflight observed40.09GiB committed-memory headroom, above the6GiB floor.

Evidence: `artifacts/performance/initial-index-release-20261119/verification.json`, per-run numeric profiles, build metadata and UTF-8 verification logs.

## Limitations

Each variant has one authorized scan of a live volume. The product index was new for each run, but Windows/device caching was not controlled and the row set changed slightly. The observations are not a cache-controlled cold-build speedup ratio or a Listary A/B benchmark.

The measured boundary starts immediately before scanning and includes durable base persistence, exact reopen, isolated publication and a real successful query. It excludes process startup, installed-service IPC and post-build USN catch-up. The saved pre-scan cursor remains available for the unchanged incremental replay path.

No installation, deployment, service restart, existing-index overwrite or commit was performed. Matching packaged LocalSystem restart/base+delta/idle CPU+I/O/search/hash acceptance remains a separate release gate. This isolated benchmark does not certify installed startup latency or all-volume completion.
