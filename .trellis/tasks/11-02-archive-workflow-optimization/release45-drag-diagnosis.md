# Release 45 — drag extraction diagnosis

## Decision

The installed optimized Release matches package sequence 45. The reported drag title, missing dual progress and developer-oriented explanatory text are implementation defects in the drag presentation path.

A controlled Directory Opus 13 Root drag reproduced the user's visible scatter/collect experience. Its destination filesystem trace shows wrapper-first creation throughout, not files created at the destination root and subsequently moved into the wrapper. Preserve the existing wrapper and publication behavior; do not add a collect/move phase.

The largest measured interval is demand-to-verified-stream materialization. Binary bulk transfer is active, but small decoder outputs still incur individual synchronous acknowledgements. This is a concrete performance experiment target, not an established exclusive cause or a Bandizip speed ratio.

Parent status remains **Phase E — Format expansion and ZIP editing / in_progress**. Prior OPT-02–OPT-07 scoped acceptance is unchanged; this feedback adds open runtime findings and does not close G-00, complete G-04 or G-18.

## Controlled operation

| Field | Observation |
| --- | --- |
| Date | 2026-11-04 |
| Package | Sequence 45, optimized Release; installed Desktop, PlatformHost, ArchiveWorker and Indexer match their packaged counterparts |
| Package record | [status](../../../.diagnostics/packaging/20261104-151404-22f10991/status.json), [build log](../../../.diagnostics/packaging/20261104-151404-22f10991/package.log) |
| Source | User-supplied `xiugou-tv-windows-x64-v1.0.3-portable.zip`; source read-only |
| Archive metadata | 137,737,166 archive bytes; 215 central records; 208 files; 206,572,163 expanded file bytes |
| Action | User performed one left Tree Root drag into a newly owned empty target displayed by Directory Opus 13 |
| Correlation | `89f43045-e6f4-42f7-a828-91f3930eb94f` |
| Result | Native and Desktop terminal success; cleanup confirmed |
| Destination metadata | One top-level wrapper; 208 files; 206,572,163 bytes; zero missing/extra paths or size mismatches |
| Directory trace | 255 Created events; zero Renamed, Deleted or Error events; no event outside the wrapper |

[Topology audit](../../../.diagnostics/archive-root-drag/20261104-89f43045/topology-audit.json), [raw relative-path events](../../../.diagnostics/archive-root-drag/20261104-89f43045/filesystem-events.jsonl), [observer summary](../../../.diagnostics/archive-root-drag/20261104-89f43045/summary.json).

## Confirmed presentation defects

### R45-UI01 — drag inherits Test title

[Drag presentation creation](../../../rust/apps/nxtools-desktop/src/archive/drag_progress.rs:86) sets the native window title to `归档拖出进度 — NxTools`, then constructs the generic `JobProgressView`.

[The constructor](../../../rust/apps/nxtools-desktop/src/archive/progress.rs:194) initializes `kind` to `ArchiveJobKind::Test`. [apply_drag](../../../rust/apps/nxtools-desktop/src/archive/progress.rs:462) updates only drag data and elapsed time. The [visible custom titlebar](../../../rust/apps/nxtools-desktop/src/archive/progress.rs:1412) chooses its title from `kind`, so it displays `测试压缩包`.

Repair the explicit drag presentation identity and both title surfaces. Do not change the generic default for unrelated Test jobs.

### R45-UI02 — dual progress excludes drag and lacks current-byte ingress

The [dual-progress branch](../../../rust/apps/nxtools-desktop/src/archive/progress.rs:1047) requires `self.drag.is_none()` and `kind == Extract`; drag therefore renders the [single fallback Progress](../../../rust/apps/nxtools-desktop/src/archive/progress.rs:1092).

The [drag wire](../../../rust/crates/nxtools-protocol/src/archive_virtual_drag.rs:341) carries current member identity and completed sealed-byte totals, but no current-member byte counters. [VirtualContent](../../../csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualContent.cs:68) reports member start and sealed completion only. [Session publication](../../../csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualDragSession.cs:93) increments completed totals only at that completion milestone.

The worker already configures [per-member extraction progress](../../../rust/crates/nxtools-archive/src/worker_drag.rs:352). [OnVirtualRunner](../../../csharp/NxTools.PlatformHost/Protocol/ArchiveEntrySession.Virtual.cs:97) handles output, member-ready and terminal frames without forwarding current extraction progress into the drag presentation.

Connect those original worker snapshots through the current demand identity, then render official current-file and overall Progress controls. Keep unknown totals truthful; cache readiness must not become receiver-completion or measured destination-write progress.

### R45-UI03 — implementation-boundary prose appears in the primary UI

[The drag render branch](../../../rust/apps/nxtools-desktop/src/archive/progress.rs:1100) unconditionally adds the reported cache/receiver/path/close explanation. This is application copy, not a Directory Opus or Windows message.

Replace that primary-body prose with concise operation states. Preserve the real non-cancelling background/close behavior, meaningful failures and receiver-completion semantics.

## Root wrapper and visible staging

The [Root freezer](../../../rust/apps/nxtools-desktop/src/archive/model_drag.rs:408) prefixes every presentation descriptor with the archive stem and inserts a descriptor-only wrapper at index 0 before IPC submission. [ArchiveVirtualPlan](../../../csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualPlan.cs:18) retains those names; [descriptor HGLOBAL creation](../../../csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualFileDataObject.cs:98) publishes the same prefixed paths. FileContents is an indexed native stream, not a destination path.

The current pipeline has no Directory Opus destination-path grant or destination collect/move command. Member extraction writes into the original private cache, and the receiving application places the supplied streams.

The first observed target event was wrapper creation. Every subsequent event was at or beneath that wrapper. The user nevertheless observed the scattered-list/collected-folder transition during this same run. This establishes a visible receiving/presentation issue, not physical scatter/collect in the watched target. Its exact Directory Opus presentation step remains unresolved.

The [saved Bandizip drag finding](../10-31-archive-research/optimization-review.md:52) records ordinary temporary real-file `CF_HDROP` delivery and a separate mapped-destination fast branch. Those differ from Next's virtual descriptor/stream route. They do not establish which branch Bandizip uses with Directory Opus or that Bandizip exhibits the same visual behavior.

## Measured performance boundary

[Redacted lifecycle events](../../../.diagnostics/archive-root-drag/20261104-89f43045/lifecycle-events-redacted.json) and [work counters](../../../.diagnostics/archive-root-drag/20261104-89f43045/work-counters.json) belong to the single operation above.

| Boundary / work | Observation |
| --- | ---: |
| Desktop submission → terminal | 9,578 ms reported elapsed |
| First FileContents demand → last verified content-ready | 6,150.628 ms |
| Last content-ready → worker cleanup terminal | 1,751.897 ms |
| Worker cleanup terminal → Desktop drag terminal | 1,049.727 ms |
| First → last destination Created event | 1,468.419 ms |
| Worker starts / catalog loads / eligibility checks | 1 / 1 / 0 |
| ZIP member decodes | 208 |
| Source revision passes / bytes / accumulated time | 4 / 550,948,664 / 419,692 µs |
| Binary output records / payload bytes | 16,980 / 206,572,163 |
| Mean binary payload | 12,165.616 bytes |
| Binary framed bytes | 207,930,563 |
| Stdin JSON frames / bytes | 18,887 / 4,925,542 |
| Payload conversion input bytes | 0 |

The ZIP [Deflated reader](../../../rust/crates/nxtools-archive/src/formats/zip.rs:881) consumes an 8 KiB encoded buffer and returns produced chunks. [PrivateOutput::write_file](../../../rust/crates/nxtools-archive/src/worker_output.rs:712) forwards each chunk directly; [call_payload](../../../rust/crates/nxtools-archive/src/worker_output.rs:559) sends one binary record and waits for its acknowledgement. Binary encoding removes decimal-JSON payload conversion but does not coalesce those small writes or remove their round trips.

Separate decoder execution, native write/ACK waiting and cache verification before assigning the dominant cost. Output coalescing is a bounded experiment within the existing 64 KiB transport limit and resource reservation, not a Bandizip-default block size or an established primary fix. Compare 32 KiB coalescing with the current path and, if useful, the existing 64 KiB ceiling; drain before metadata, finish, verification and commit. Preserve cancellation checkpoints, first failure, exact token/sequence/ACK ordering and actual cleanup. Retain a change only with lower same-workload materialization latency and correct output; do not raise concurrency, buffers or deadlines, remove integrity work, or introduce a destination-handoff framework.

### Bandizip 7.32.0.1 ZIP buffering reference

The [saved ZIP trace](../10-31-archive-research/bandizip-core-pipeline.md:28) and [output lifecycle](../10-31-archive-research/bandizip-solid-output.md:72) distinguish these units:

| Layer | Observed policy |
| --- | --- |
| Windows input object | Initial 64 KiB input-cache capacity; buffered seeks do not always reach the OS |
| ZIP Deflate | 16 KiB encoded-input and decoded-output working capacities; first encoded-input request is at most 4 KiB, subsequent requests at most 16 KiB; actual produced output can be smaller |
| ZIP Stored | Copy blocks up to 8 KiB |
| Direct output | In-process binary write helper, with no private cross-process acknowledgement per codec callback |
| Conditional background output | Normally coalesces 32 KiB per consumer write; drain/close also writes the partial tail |
| Background queue | Initial 64 KiB ring; producer backpressure threshold clamped to 128 KiB–4 MiB, not a hard allocation cap or normal write-block size |

Original saved records were checked at `0x18007db60`/`0x18007dca0` (raw Deflate initialization, input/output loop and incremental CRC), `0x18001aec0`/`0x18001a9c0` (member output size and direct/buffered selection), `0x1800230f0`/`0x180023950` (32 KiB consumer allocation and dequeue), and `0x180023340`/`0x180060920`/`0x180022db0` (backpressure, direct write and native short-write handling). Record names and Evidence IDs are retained in the two reference notes.

The buffered output class additionally requires configuration/type eligibility. At the inspected size gate, Stored outputs below 100 MiB and other methods below 20 MiB choose direct output. Read-only central metadata for this ZIP gives 204 Deflate files below 20 MiB, totaling 90,451,354 output bytes, and four at or above it, totaling 116,120,809 bytes. Those four satisfy the size gate only; this is not proof that the live Bandizip configuration selects the buffered branch.

Both paths perform streamed raw Deflate and incremental CRC checks. Next explicitly configures [flate2 1.1.9 with zlib-rs](../../../rust/crates/nxtools-archive/Cargo.toml:35) and crc32fast 1.5.0; the saved Bandizip wrapper calls its native inflate entry. Equal format/working-block descriptions do not establish equal decoder CPU cost. The saved artifacts do not contain a complete comparison of the two inflate hot loops.

The [Bandizip selected-ZIP dispatch](../10-31-archive-research/bandizip-core-pipeline.md:48) also has conditional parallel selection based on configuration, catalog capability and member count. Its use for this actual whole-Root drag has not been observed. Compare the same selection, output mode and completion boundary; historical single-member CLI results do not supply that comparison.

Consequently, the measured 12,165.616-byte mean IPC payload does not establish that Next's decoder buffers are undersized or that coalescing is the dominant optimization. It establishes many serialized IPC round trips. Codec execution, cross-process waiting, native/cache work and reference scheduling remain distinct costs to measure.

## Additional admission finding

**R45-N01 — Root wrapper component-length units disagree.** [Rust component admission](../../../rust/crates/nxtools-protocol/src/archive_virtual_drag.rs:11) permits up to 255 UTF-16 units; [C# command admission](../../../csharp/NxTools.PlatformHost/Protocol/ArchiveVirtualDragContract.cs:92) also rejects components over 255 UTF-8 bytes. The Root wrapper comes from the archive filename: a Windows-valid basename of 86 ordinary CJK characters is 86 UTF-16 units but 258 UTF-8 bytes, so a short safe member can pass Desktop freezing and fail native admission. The supplied ASCII-named ZIP does not encounter this defect. Align the existing contract and cover the real serialized command boundary without changing member identity or silently shortening an uncolliding user name.

## Repair order and acceptance boundary

1. Add failing regressions through the original serialized drag-progress ingress and actual view render: explicit drag title, two semantic Progress controls, absence of developer prose, current-file reset, unknown totals, stale demand and completed-cache/receiver-pending distinction.
2. Repair R45-UI01–03 using the existing progress owner, deferred cross-entity updates and official component slots. Preserve native hidden creation → no-activate → final bounds → show, source focus and original OLE lifetime.
3. Measure decoder execution, IPC/write/ACK waiting and cache verification on the same real Root workload, then run the bounded output-coalescing comparison. Record reference direct/buffered and conditional parallel policy separately; keep stage-specific latency, work, record counts, output integrity and actual cleanup distinct.
4. Retain wrapper-first placement and investigate the Directory Opus visible transition without adding physical collection. A physical and visual replay remains required after changes.
5. Resolve R45-N01 at the existing cross-language admission boundary.

## Limitations

- This is diagnosis, not a product patch or repaired-feature acceptance. No product/test source was changed and no Cargo build or repackaging was run for this diagnosis.
- The destination watcher records topology, not process identity or file-write completion. Zero missing paths and matching sizes are metadata checks, not independent content verification.
- The 9,578 ms lifecycle includes native gesture/drop admission, materialization, receiver work and cleanup. The Created-event span is not extraction duration. There is no same-route current Bandizip timing or exclusive decoder/IPC/cache cost attribution.
- Retained-run terminal `native_output_*` and `output_payload_write_*` counters describe the final active output bridge, not every member's aggregate I/O. Zero native source observations do not mean zero Rust source reads.
- The separate ordinary Extract completion-window screenshot remains uninspected; no ordinary completion-window defect is inferred from the drag diagnosis.

## Repair addendum — 2026-11-04

[Repair and verification](release45-repair.md) records the implemented explicit drag title, original-worker current/overall progress, concise status and UTF-16 Root-name admission. [Measured performance](release45-performance-repair.md) records aggregate phase timings and the retained extraction-only32KiB output coalescing, with all20 successful same-source operations and sequential/ABBA results. [Native progress and staging observations](release45-native-progress.md) documents the original correlated descriptor/async/OLE boundaries. These are new repair evidence; the installed Release45 observations above retain their original scope.

DOpus visible scatter/collect is still unresolved. The earlier wrapper-first/no-move filesystem finding is unchanged; controlled COM results do not establish physical receiver acceptance. A scoped authorized visual replay remains necessary. Phase E stays in_progress.
