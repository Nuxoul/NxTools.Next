# M8 — shared immutable overlay rows

## Measurement and decision

Both runs use a one-million-row synthetic base, the production apply/classification boundaries, setup excluded, one test thread and `profile.test.package.nxtools-indexer.opt-level=3`. This is an optimized Indexer package in the test profile, not a complete Release-profile product. Values are individual controlled fixture samples, not a latency distribution or installed-service benchmark.

```powershell
cargo --config 'profile.test.package.nxtools-indexer.opt-level=3' test -p nxtools-indexer --lib -- m8_million l11_million --ignored --nocapture --test-threads=1
```

| Overlay | Metadata changed | Apply µs before → after | Allocations before → after | Requested bytes before → after | Classify µs before → after |
| ---: | :---: | ---: | ---: | ---: | ---: |
| 1,000 | No | 622 → 314 | 2,013 → 13 | 330,059 → 72,067 | 50 → 13 |
| 1,000 | Yes | 477 → 485 | 2,015 → 16 | 330,104 → 72,240 | 17 → 11 |
| 100,000 | No | 62,663 → 30,125 | 200,019 → 19 | 22,719,491 → 4,588,003 | 15,368 → 1,072 |
| 100,000 | Yes | 60,787 → 30,125 | 200,021 → 22 | 22,719,536 → 4,588,176 | 6,339 → 340 |

Classification allocates zero bytes in all cases. Cursor-only apply peak allocations decline from320,751 to62,759bytes at1k and22,129,479 to3,997,991bytes at100k. The100k synchronous apply+classify cost changes from67.126–78.031ms to30.465–31.197ms. The measured coordinator occupancy justified sharing unchanged rows; map/set cloning and child-index construction remain O(n), so the remaining approximately31ms is retained as a measured tradeoff rather than an O(1) claim. The1k metadata case is effectively unchanged in this sample.

Native unchanged-cursor polling returns NoChange before buffer allocation/apply/clone. A read without records or cursor advancement also returns early. The idle-copy suspicion is false; cursor advancement is the measured case.

## Ownership and source

`MftOverlay.replacements` is `HashMap<u64, Arc<MftEntry>>`. Snapshot and durable-checkpoint clones retain independent transactional maps/deletion sets while sharing immutable row/path/name allocations. Actual metadata/path mutations use `Arc::make_mut`; structural replacements use new Arcs. Unchanged metadata keeps its row. `visible_entry` still returns an owned requested row; compaction retains its owned-base representation.

Shared Arc equality short-circuits identical allocations and retains content equality for distinct allocations. Search/recent borrowing explicitly reaches MftEntryRef through the Arc. The base/delta byte formats, successful durable checkpoint owner, stale-cursor rejection, ranking and base residency remain unchanged.

Sources: `rust/apps/nxtools-indexer/src/{mft.rs,usn.rs,recent.rs,lib.rs,mft/cache.rs,mft/cache/journal.rs}`. Test-only allocation observation is in `mft/allocation_probe.rs`; million-row fixtures are in USN measurements and `mft/cache/measurements.rs`. There is no product high-frequency instrumentation.

## Regression evidence

`usn::overlay_sharing_tests::m8_snapshot_clone_shares_unchanged_overlay_rows_and_updates_are_isolated` compiled against deep-copy behavior and failed `allocations.calls < 16`: cloning2,000rows requested573,635bytes in4,002allocations. It now passes allocation, pointer-sharing, changed-row isolation, distinct-Arc content equality, classification and owned-lookup isolation assertions. It is included in the133passing safe Indexer library tests.

Logs under `D:\.pi-desktop\data\scratch\6a21b011-4230-4aa5-a2ed-f2dc1b4153e6\`:
- `indexer-m8-sharing-red.log`: intended behavioral failure.
- `indexer-integrated-focused.log`: sharing regression and affected integrity tests pass.
- `indexer-measurements.log`: baseline measurements,2passed.
- `indexer-measurements-final.log`: final measurements,2passed.
- `indexer-m3-handoff-safe-lib.log`: integrated safe library suite,133passed.

## L11 memory measurement

One million unique-name rows produce a117,000,260byte base file. Final allocator observations:

| Representation | Allocation calls | Requested bytes | Retained bytes | Peak bytes |
| --- | ---: | ---: | ---: | ---: |
| Owned | 2,199,726 | 303,159,935 | 226,040,775 | 244,176,924 |
| Mapped | 199,684 | 33,079,365 | 467 | 18,136,531 |

Final process counters, bytes `(working_set, private_commit)`:

| Point | Counters |
| --- | --- |
| Before owned | (4,468,736, 876,544) |
| After owned | (271,491,072, 274,857,984) |
| After drop / before mapped | (5,402,624, 1,716,224) |
| After mapped | (10,416,128, 1,888,256) |
| After mapped row iteration | (110,419,968, 1,888,256) |

The high-unique mapped fixture abandons the optional bounded search-data build and retains None, so467retained allocator bytes is not a universal mapped-snapshot heap cost. File mapping bytes are not private commit; row iteration is not proof that every mapped byte is resident. Rebuilt/compacted product snapshots remain owned in this batch.

## Completed-base publication follow-up — 2026-11-06

Rebuild and compaction now release their newly built owned base after durable persistence and
reopen the exact new file as mapped before service publication. Bounded runtime search auxiliaries
are rebuilt in persisted row order. Old service/search leases, base+delta recovery and last-good
failure behavior retain their original owners. Temporary-file regressions cover both publication
paths, owned release, mapped row ordering, delta continuation, stale compaction and reopen failure.

The tables above remain historical M8/L11 measurements. This follow-up has no new million-row,
private-commit, working-set or installed-service performance measurement; no trim is added.
Current contract: `.trellis/spec/nxtools-indexer/backend/database-guidelines.md`.
