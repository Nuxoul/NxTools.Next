# Listary initial-volume indexing and Next build work

## Scope

Source inspection of the fixed Listary6.3.5.94 export at `D:\Code\Listary.Decompiled\6.3.5.94`, contrasted with the current Next initial-volume build. This distinguishes the active UniFFI engine boundary, legacy managed MFT code retained in the export, and Next implementation facts. It is not a cold-build timing comparison.

`M/` below means `managed/Listary/`; service files use `managed/Listary.Service/`. The fixed `expanded/Listary/` export corroborates async state-machine ordering. No Listary native engine source is available.

## Active Listary startup

| Boundary | Verified entry |
| --- | --- |
| App startup | `M/Main/App.xaml.cs:58` constructs/starts ListaryApp; `M/Core/ListaryApp.cs:116` connects the handle service, then `:126` calls `Class387.method_1(false)` without awaiting its Task |
| Engine construction | `M/Core/FileSearch/Class387.cs:42` obtains user identity/log path, constructs `Class65`, obtains native process information, then awaits `method_3` |
| Volume configuration | `Class387.cs:56` calls `DiskSearch/ListNtfsVolumes.ListVolumes`, skips disabled roots, builds the NTFS/custom-folder index lists and duplicated native handles |
| Unified start | `Class387.cs:91` submits the selected index lists, cache location and priority rules together through `Class65.imethod_14(new Class94(...))` |
| Actual native call | `M/uniffi/listary_engine/Class65.cs:245`/`:1339` reaches `Class64.uniffi_listary_engine_fn_method_listaryengine_start`; `Class64.cs:89` declares the Cdecl `listary_engine` DLL import |
| Privileged handle provider | `M/Core/Class139.cs:93` opens/duplicates a requested handle or delegates to `Listary.Service/ListaryService.cs:67`; the service WCF host is not itself proof of where the native scanning algorithm executes |
| State projection | `Class387.cs:425` calls native `get_indexer_infos`; `IndexerInfo.cs:173` projects Building/Ready/Err plus item count, update state and error |
| Search | `Class385.Search` calls `Class387.method_22/method_23` and native search; the inspected frontend does not wait for every volume to become Ready |

The managed caller assembles volumes in root order and submits them together. This does **not** establish native scan order, worker count or volume parallelism. Normal startup, explicit restart, forced rebuild, single-index rebuild and hot-add are separate requests. Cache restore versus cache-missing initial build is not explicitly selected in the inspected managed startup; both enter native start.

The generated `MiniCore/IndexedSearchService.cs` gRPC definitions do not replace this verified active call chain. The DLL's internal process/transport, scan algorithm, cache layout, USN catch-up policy and first-searchable publication point remain opaque.

## Legacy managed MFT reference

The export also retains `M/DiskSearch/DiskSearchService.cs`, `Class544.cs` and `MasterFileTable/Class553.cs`. No active connection from the inspected6.3.5.94 Core startup/provider chain to this legacy service was established. Its readable scanner is historical implementation evidence, not the current native engine algorithm.

Verified legacy scan chain:

1. `DiskSearchService.Start:43` attempts saved state unless forced, then constructs per-volume Class544 objects.
2. `Class544.method_6:231` wraps the raw volume handle in a stream, queries NTFS geometry and invokes `Class553.method_0`.
3. `Class553.method_1:120` obtains retrieval extents and constructs run descriptors. `method_0:27` reads those runs in chunks of at most64clusters, using one reusable `BytesPerCluster * 64` buffer. A4KiB cluster therefore gives a256KiB chunk, not a universal hardcoded size.
4. The scan loop walks file-record boundaries, checks the FILE signature/in-use flags and applies fixups (`:64–80`). It parses the resident attribute type16 (`Enum25.const_0`) and reads `Struct243.DateModified` (`:82–87`). The same record supplies parent reference, file name and namespace (`:89–103`).
5. Directory flag, file reference, parent reference, modification value, attributes and name are passed through the callback to the original index (`Class544.Class545.method_0:374`). The inspected scanner loop obtains the timestamp from record bytes, rather than issuing Next's per-record OpenFileById attribute lookup.

The legacy FILE_NAME timestamp fallback (`Class553.cs:93`) is not used in Next. Its independent parser accepts only resident `$STANDARD_INFORMATION` LastWriteTime from the matching full file identity.

## Current Next initial-build path

Evidence: `repo:rust/apps/nxtools-indexer/src/mft.rs`, `repo:rust/apps/nxtools-indexer/src/mft/bulk_times.rs`, `repo:rust/apps/nxtools-indexer/src/mft/bulk_times/native.rs`, `repo:rust/apps/nxtools-indexer/src/mft/time_metadata.rs`, `repo:rust/apps/nxtools-indexer/src/mft/cache.rs`, `repo:rust/apps/nxtools-indexer/src/lib.rs`.

### 1. Batch hydration of true LastWriteTime

`windows::scan_volume` saves the journal identity/cursor before enumeration. `FSCTL_ENUM_USN_DATA` with an unrestricted upper USN remains the authority for names, parent references, directory flags and full file references. This preserves existing primary-name/hard-link behavior and includes preexisting rows changed during scanning.

`bulk_times` replaces initial enumeration's per-row OpenFileById calls. The native adapter queries NTFS geometry and `$MFT` retrieval extents, then reads the borrowed volume handle into one sector-aligned reusable block of at most256clusters, capped at1MiB. The safe layer reassembles records across blocks/extents, validates FILE/in-use/base identity and sequence bits, applies the complete USA fixup, and reads resident attribute0x10 LastWriteTime. Ordered extent lookup avoids a per-row linear extent walk. Sparse regions, missing/reused records and invalid metadata remain unknown; actual native setup/read errors fail the build with the original native error source. Extents can refresh once for MFT growth observed after the initial map.

Raw standard-information USN must equal the enumerated USN before accepting that LastWriteTime. Different or absent USN invokes the original full-ID FileBasicInfo reader for that exception only. A cached metadata change may precede the saved replay cursor while its disk bytes remain older; replay is not a freshness substitute. Failed exception reads remain unknown. No file is reopened solely because it is present in the enumeration.

The batch hydration seam propagates read failures before snapshot construction. It never substitutes USN event time or FILE_NAME time, and never moves volume-wide metadata work into a query. Incremental USN hydration retains its existing bounded full-ID FileBasicInfo reader.

### 2. Runtime auxiliaries belong to the final mapped representation

`unsorted_entries_from_records` reconstructs paths for both constructors. The shared production `scan_volume_and_persist_with` selects `snapshot_for_persistence_from_records` / `from_entries_for_persistence` before scanning constructs its result: unpublished rows, identity/cursor/count and writer metadata only. It does not path-sort rows or build reference/child maps, name masks or budgeted BaseSearchData for this temporary representation. Direct in-memory scan fixtures use `entries_from_records` to retain path sorting and the full searchable constructor.

The writer still computes persisted ordering, child ordering and masks. Durable persistence is followed by consuming `reopen_persisted_snapshot`, releasing owned rows before opening the exact new file. `from_mapped` builds bounded runtime BaseSearchData using persisted row order. No row-indexed auxiliary structure is transferred from the discarded representation. Cache/protocol formats, compaction policy, delta continuation and publication owners are unchanged.

`unsorted_entries_from_records` is the Next-owned path reconstruction boundary: collect by full file
reference with the last duplicate winning, seed NTFS root record5 including its sequence bits,
resolve parent references and omit missing/cyclic chains.
Self-parent records retain the existing mount-root behavior. Name, directory flag and true time
come from the authoritative enumerated record. Path-cache storage and traversal scratch are
temporary construction state; they do not change persisted row order, native enumeration,
publication, focus, cancellation, or failure owners. Path construction performs no native IO.
Traversal scratch is reused, cached paths are borrowed while extending a parent with exact string
capacity, and the completed path plus original name move into the final entry. No extra name or
complete-path clone is needed for a cache hit. The persistence writer owns full-FRN row sorting
and child sorting; only direct searchable owned snapshots need the separate path sort.

### 3. Volume scheduling and availability

`spawn_snapshot_build` scans its volume list sequentially. Every completed volume is sent as BuildVolume and applied to the searchable snapshot set before BuildFinished. The first usable volume does not wait for all later volumes. The corresponding `indexer.snapshot.first_search_ready` and all-volumes-complete events are distinct.

Native volume concurrency cannot be inferred from Listary's one start call. Raising Next concurrency without measuring physical-device contention and peak memory is not established by this reference.

### 4. Stage measurements already defined

`indexer.snapshot.native_enumeration` records rows, metadata_rows, metadata_mode, metadata_reads, metadata_unknown, metadata_fallback_reads, metadata_elapsed_ms and total elapsed_ms. Enumeration, construction, scan, persist, reopen, per-volume publication and first-search-ready retain the existing correlated build span. No per-file payload or high-frequency logging is added.

An authorized independent read-only C: scan using the unoptimized Debug test profile observed:

| Path | Visible rows | Per-file attribute calls | Scan fixture elapsed |
| --- | ---: | ---: | ---: |
| Per-file metadata | 1,621,611 | 1,621,621 | 93,700ms |
| Bulk MFT metadata with USN freshness | 1,622,496 | 107 | 38,530ms |

Both runs independently checked stable full identities and true LastWriteTime for three system files through ordinary file handles. All107bulk-path attribute reads were USN freshness exceptions. The live volume changed between scans; these are not equal frozen datasets. Sequential runs do not control Windows/device caching and are not a cold-build speedup ratio. The fixture constructs a searchable snapshot but does not persist/publish it or touch the installed index.

Cache-first restore, incremental USN updates and first-completed-volume availability already existed. Volume concurrency remains unchanged; reassess it only with physical-device contention and peak-memory evidence.

### 5. Optimized Release base-build measurement

The ignored C:-only `cache::initial_profile_tests::initial_build_release_phase_profile` uses the
production scanner/constructor-selection seam, durable temporary writer, exact mmap reopen,
isolated in-process publication and a real successful search. Compiler opt_level3 and disabled
debug assertions are verified before timing. All six phase events and independent full-ID/time
witnesses are required; a fresh product index is used for every authorized scan.

| Metric | Release baseline | Optimized Release |
| --- | ---: | ---: |
| Complete base build and isolated publication | 12,435.12ms | 9,138.53ms |
| First successful real search from build start | 12,440.02ms | 9,144.27ms |
| Visible rows | 1,623,158 | 1,623,169 |
| Raw MFT block reads | 7,192 | 1,875 |

Final path construction took1,081ms; native enumeration including metadata took6,258ms;
durable persistence1,583ms and exact reopen214ms. Metadata is an inclusive2,810ms subset of native
enumeration. The three stable system-file identity/time witnesses passed; all69attribute reads
were freshness exceptions. Both completion targets were below10,000ms.

This starts before scanning, not before process/service startup, and excludes installed IPC and
post-build USN catch-up. Each variant has one live-volume sample; Windows/device caching is
uncontrolled, so it is not a cold-build ratio or Listary A/B result. Details and evidence:
`docs/audits/2026-11-19-initial-index-release.md` and
`artifacts/performance/initial-index-release-20261119/verification.json`.

## Measurement caveats and verification

The Listary tutorial polls status every100ms (`M/Core/Tutorial/Class187.cs:20`). Its displayed seconds are `max(stopwatchSeconds * 0.75, 0.1)` (`:203`), not raw engine elapsed time. `AllIndexerInfo.IsAllIndexFinished:59` means no Building entry remains; it does not require every entry to be Ready. Tutorial time/completion is therefore not a sufficient cold-build benchmark.

Regression seams cover byte-backed fragmented/cross-block FILE records, identity reuse, USA/attribute bounds, sparse/unknown metadata, bounded growth refresh, I/O failure, and the real records→persist→exact mmap reopen allocation path. Release acceptance still requires the matching packaged LocalSystem restart/base+delta/idle CPU+IO/search gates. No package, deployment, service restart or existing-cache change is part of this source optimization.
