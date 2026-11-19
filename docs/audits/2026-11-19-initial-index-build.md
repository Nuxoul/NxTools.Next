# Initial-volume build: bulk metadata and final-only auxiliaries

## Scope

Reduce no-cache initial-volume work without changing names/hard-link enumeration, query ranking,
cache/protocol formats, volume concurrency or service publication. Rust owns the scanner and snapshot;
Windows calls stay in the native adapter. No Desktop UI change.

Reference inspection is recorded in [Listary initial indexing](../reference-code/listary-initial-indexing.md).
Listary6.3.5.94's active managed startup reaches an opaque native engine. Its retained historical managed
MFT scanner demonstrates reusable raw-table reads and resident standard-information timestamps; it is
not the current native engine source. The Next reader is an independent Rust implementation.

## Implementation

### Bulk true LastWriteTime

`FSCTL_ENUM_USN_DATA` remains the authority for the record set, names, parent references, directory
flags and full file references. Keep its unrestricted upper USN and the journal identity/cursor saved
before scanning.

`mft/bulk_times.rs` maps MFT logical offsets through retrieval extents and hydrates the same bounded
batches. One sector-aligned buffer reads at most64clusters, capped at1MiB. Records can cross blocks
or fragmented extents. Full sequence identity, FILE/in-use/base checks, complete USA fixup and resident
`$STANDARD_INFORMATION` bounds precede time extraction. Sparse, missing, reused or malformed metadata
is unknown. Native setup/read errors propagate with their native error source and preserve last-good.

A raw standard-information USN must match the enumerated USN. Different/missing USN reads current
FileBasicInfo for that full identity only: a cached change before the replay cursor cannot be repaired
by later replay. Failed current-attribute reads remain unknown. Neither USN event time nor FILE_NAME
time substitutes for LastWriteTime. Incremental USN hydration retains its original bounded reader.

The existing correlated native-enumeration event adds aggregate mode, raw-read, freshness-exception
and unknown counts. No per-file diagnostics or query-time metadata scan is added.

### Persistence-only temporary construction

`entries_from_records` preserves path construction and sorting. `scan_volume_and_persist_with` owns
constructor selection and passes it into the scanner. Its unpublished owned representation omits
reference/child maps, name masks and runtime search groups/postings. The writer derives persisted
row order, child order and masks as before.

Durable commit and exact-file mmap reopen precede existing service publication. Owned rows are
released before reopening; runtime search data is built only for the final mapped row order. Direct
searchable scan fixtures retain the full constructor. Compaction, delta continuation and immutable
search leases retain their established owners.

## Measurements

Authorized independent read-only scans of C:, using three stable system-file full-ID/LastWriteTime
witnesses from ordinary file-handle APIs:

| Path | Visible rows | Per-file attribute reads | Scan fixture elapsed |
| --- | ---: | ---: | ---: |
| Per-record metadata | 1,621,611 | 1,621,621 | 93,700ms |
| Bulk metadata with USN freshness | 1,622,496 | 107 | 38,530ms |

All107attribute reads on the bulk path correspond to freshness exceptions. The ordinary matching-USN
path performs no per-record attribute lookup. Both scans verified all three stable system-file witnesses.

A separate4,160-row fixture uses4,096files,64directories and256names. Both paths perform real temporary
base persistence and exact mapped reopen; setup allocations are outside the observation window.

| Complete construction/persist/reopen path | Requested bytes | Allocation calls |
| --- | ---: | ---: |
| Full transient runtime structures | 4,528,147 | 32,428 |
| Production persistence-only construction | 3,951,955 | 30,587 |
| Reduction | 576,192 | 1,841 |

Final mapped rows, full identities, true times, parent/child lookup, journal cursor and selective search
candidates agree. The production constructor selection is exercised directly by the regression.

## Verification

- Batch-reader regressions:8passed, including fragmented/cross-block records, USA trailer repair,
  full identity, sparse/unknown metadata, bounded growth, I/O failure and pre-cursor dirty metadata.
- Complete non-real-volume Indexer library suite:168passed,0failed,4ignored. Two existing native
  executable-drive scan fixtures were explicitly filtered out.
- Authorized C:-only native regression: passed with current-time witnesses and attribute/freshness
  exception counts agreeing. No snapshot was persisted or published by that fixture.
- `cargo clippy --locked -p nxtools-indexer --all-targets -- -D warnings`: passed.
- `cargo check --locked --workspace --all-targets`: passed.
- Scoped rustfmt, spec contracts, Indexer lifecycle, GPUI release contracts and Markdown editor
  contracts: passed. Scoped `git diff --check`: passed.

Rust verification uses `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0` and
`RUSTFLAGS=-C codegen-units=256`. The core manifest declares its existing workspace `uuid` dependency.

Evidence: `artifacts/performance/initial-index-build-20261119/verification.json` and UTF-8 logs.

## Limitations

- Sequential C: runs use a changing live volume and uncontrolled Windows/device caches. Their elapsed
  times are observations, not a cache-controlled cold-build speedup ratio or an installed Listary A/B.
- The native fixture includes searchable in-memory construction, but excludes durable persistence and
  service publication. The allocation fixture measures that durable boundary separately.
- Allocation figures are cumulative current-thread requested bytes, not retained/peak heap, working
  set or private commit.
- No package, deployment, service restart, existing-cache change or index publication was performed.
  Release still requires the matching packaged LocalSystem restart/base+delta, idle CPU+IO and real
  service search/hash gates. Source tests and the isolated administrator scan do not replace them.
