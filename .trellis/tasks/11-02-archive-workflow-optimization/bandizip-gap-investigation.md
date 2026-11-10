# Bandizip extraction gap — 2026-11-05

Parent: **Phase E — Format expansion and ZIP editing / in_progress**.

## Findings

The extraction gap remains reproducible with optimized binaries. On the supplied ZIP, the existing Release native Job takes a median 2780.91 ms versus 411.86 ms for Bandizip's complete CLI extraction. Next's controlled Root virtual-file route takes a median 7325.86 ms. Every operation produces the same 208 files and 206572163 decoded bytes, independently compared byte for byte.

Next's measured Deflate execution is approximately 0.45–0.48 seconds. The larger operation cost lies in repeated source admission, serialized native output/publication, and, for drag, per-member cache materialization, verification, consumer copying and retirement. The current path already uses binary payload transfer, 32 KiB extraction-write coalescing and one retained worker/catalog per drag. Those completed optimizations do not close the end-to-end performance gap.

The retained real Directory Opus Root-drag trace does **not** show physical scatter followed by collection. Its first event creates the wrapper; all 255 events are Created events at or beneath it, with zero rename or outside-wrapper events. The visible scatter/collect transition reported during that operation remains an unresolved receiver-presentation behavior.

Bandizip's ordinary CF_HDROP route and its mapped-destination fast route differ from Next's virtual descriptor/IStream route. Its smart/name destination is resolved before payload extraction, not by collecting already extracted output afterward.

## Comparable complete-output measurements

Read-only input: `D:\Tencent\xiugou-tv-windows-x64-v1.0.3-portable.zip`, 137737166 encoded bytes, 215 catalog records, 208 regular files, 206572163 decoded bytes. No extracted member is executed.

| Route | Trial 1 ms | Trial 2 ms | Trial 3 ms | Median ms |
| --- | ---: | ---: | ---: | ---: |
| Next native ordinary Extract Here | 3197.7701 | 2780.9098 | 2750.6578 | 2780.9098 |
| Next Root, controlled COM receiver | 7903.7645 | 7325.8614 | 7134.7629 | 7325.8614 |
| Bandizip 7.32.0.1 CLI, all members | 1224.6558 | 411.8610 | 405.1845 | 411.8610 |

The ordinary/native-to-reference CLI median ratio is 6.75 for these tested routes. The controlled Root result is not a Bandizip GUI-drag comparison.

### Executables and operation boundaries

- Next worker: `target/release/nxtools-archive-worker.exe`, 5822464 bytes, modified 2026-11-05T04:49:22Z.
- Existing Release test apphost: `csharp/NxTools.PlatformHost.Tests/bin/Release/net8.0-windows/NxTools.PlatformHost.Tests.exe`, modified 2026-11-04T10:17:05Z.
- Its PlatformHost assembly: `csharp/NxTools.PlatformHost.Tests/bin/Release/net8.0-windows/NxTools.PlatformHost.dll`, 2079232 bytes, modified 2026-11-04T10:01:40Z.
- Reference executable: `D:\Tool\Bandizip\bz.exe`, file version 7.32.0.1, 541240 bytes.
- Native selection: `archive R45 same-source extract and Root drag measurements`; exact LIST1, RUN1, PASS1, SKIP0, FAIL0, with three ordinary and three controlled Root operations.
- The native fixture includes original worker cleanup and verifies output outside the timed operation. Root timing includes demanded content, controlled receiver copies and actual cache retirement. The preliminary browser read is outside the individual route clocks.
- Each Bandizip trial launches a fresh process with `x -aoa -y -o:<fresh-owned-directory> <source>`. Process startup and exit are timed; independent content verification and owned-output deletion are outside that interval.
- All native trials finish before the reference trials. No system cache flush is performed. First-run observations remain in the table.

## Measured work and current mechanisms

### Ordinary extraction

Each of the three ordinary operations reports:

- 2 catalog loads, 0 edit-eligibility checks;
- 6 full source revision passes, totaling 826422996 bytes;
- 6436 acknowledged binary output records for 206572163 bytes;
- 208 committed files and completed cleanup.

| Counter, inclusive where stated | Trial 1 ms | Trial 2 ms | Trial 3 ms |
| --- | ---: | ---: | ---: |
| Full-source revision work | 593.813 | 600.883 | 583.451 |
| Raw Deflate execution | 477.356 | 462.742 | 453.117 |
| Output IPC/ACK, including native writes | 696.469 | 475.429 | 451.586 |
| Native commit, including fingerprint/postpublication | 859.438 | 817.872 | 844.917 |
| Output fingerprint | 166.770 | 154.156 | 152.408 |
| Postpublication verification | 161.193 | 154.107 | 153.293 |

`PrivateOutput::flush_output` sends one coalesced record. `call_payload` installs its binding, sends the record and waits for the corresponding reply before returning. Binary transfer removes decimal JSON conversion but not these serialized round trips.

`ArchiveOutputSession.CommitCore` flushes each partial, captures identity and a content fingerprint, publishes by native rename, closes the writer and independently verifies the published file. This work is paid for each ordinary output and each drag-cache output. Native write time alone is approximately 77–90 ms per operation; it is not the whole publication cost.

Evidence: `repo:rust/crates/nxtools-archive/src/worker_output.rs#PrivateOutput::flush_output`, `repo:rust/crates/nxtools-archive/src/worker_output.rs#PrivateOutput::call_payload`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs#CommitCore`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs#Fingerprint`.

### Drag adds a complete cache delivery lifecycle

Each Root operation retains one worker/catalog with four full source revision passes, totaling 550948664 bytes. Descriptor-only requests do not materialize payload. Each distinct requested file then follows:

`FileContents GetData` → `ArchiveVirtualContent.Open` → `DemandVirtual` → original worker extraction/native cache publication → `Lease.Seal` → independently verified native IStream → receiver copy → receiver End → worker/cache retirement.

`Open` waits for the complete member task before returning the native stream. This is demand-triggered full-file materialization, not progressive decoding into the receiver's final file. The existing materialization semaphore admits one distinct member at a time.

| Drag observation | Trial 1 | Trial 2 | Trial 3 |
| --- | ---: | ---: | ---: |
| First demand to first ready, ms | 569.506 | 542.951 | 521.352 |
| First demand to last ready, including interleaved receiver copies, ms | 6369.177 | 5999.313 | 5887.636 |
| Seal count / total ms | 208 / 764 | 208 / 741 | 208 / 730 |
| Verified cursor count / total ms | 208 / 297 | 208 / 276 | 208 / 280 |
| Receiver copy total, ms | 274.688 | 233.946 | 232.595 |
| Receiver End to complete cleanup, ms | 1482.528 | 1292.026 | 1211.650 |

Each operation records 206572163 bytes hashed at Seal and another 206572163 bytes at cursor opening. These are additional to native publication verification. Cache release also validates content before deleting the owned file. Cache manifest persistence uses `Flush(true)` at lifecycle transitions; retirement is a real per-file workload, not merely progress-window presentation.

The worker waits for new drag demands on a notified condition variable, not a fixed per-member sleep. The runner's 20 ms polling monitors process exit and cleanup; its presence is not evidence of 20 ms charged for every file.

Evidence: `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualContent.cs#Open`, `repo:csharp/NxTools.PlatformHost/Protocol/ArchiveEntrySession.Virtual.cs#DemandVirtual`, `repo:csharp/NxTools.PlatformHost/Protocol/ArchiveEntrySession.Virtual.cs#OnVirtualRunner`, `repo:csharp/NxTools.PlatformHost/Protocol/ArchiveEntrySession.Virtual.cs#CloseVirtual`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs#Seal`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs#OpenVirtualStream`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs#Release`, `repo:rust/crates/nxtools-archive/src/worker_drag.rs#DragPort::wait`.

These intervals overlap. They must not be added as exclusive contributions to wall time.

## Folder appearance and Bandizip behavior

### Next already declares the wrapper before OLE

`BrowserModel::freeze_drag_roots` prefixes every presentation name with the archive stem and inserts a directory descriptor at index 0. `ArchiveVirtualFileDataObject.Descriptors` delivers the complete hierarchical names in FILEGROUPDESCRIPTORW. This sample exposes 255 descriptors, including the wrapper and all descendants. FileContents requests address that frozen array; they do not grant Next a final target path.

The original worker writes private cache files. The receiving file manager creates its destination from the descriptors and streams. Next has no target-root collection/move command in this route.

Evidence: `repo:rust/apps/nxtools-desktop/src/archive/model_drag.rs#BrowserModel::freeze_drag_roots`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualFileDataObject.cs#Descriptors`, `repo:csharp/NxTools.PlatformHost.Tests/Opt04DragTests.Release45.cs#R45Command`.

The retained real Directory Opus trace was independently replayed from all raw events: first event is wrapper creation, 255 Created events, zero outside-wrapper and zero rename events. That proves the recorded destination topology, not the file manager's internal visual staging step.

### Bandizip uses different handoffs

The retained shipped-binary evidence connects ordinary drag to temporary real-file paths and CF_HDROP. The fast-success branch checks DRAGDROP_S_DROP, reads a mapped destination via `FUN_1401aedd0`, then invokes common selected extraction `FUN_1401481d0 → FUN_14013ce80` with that destination.

Smart extraction mode 2 inspects the opened catalog's root shape; named mode 3 always chooses the archive-named subdirectory. `FUN_14011acd0` writes the chosen destination into the operation descriptor before worker start. These are distinct behaviors: smart extraction must not be assumed to run for every outgoing drag.

The ZIP engine retains its input/catalog, seeks selected member records and performs streamed Deflate/CRC with direct in-process output or an eligible buffered path. It does not require Next's per-write private process acknowledgement. The inspected buffered path coalesces 32 KiB and drains its tail; qualifying large outputs and optional parallel extraction depend on configuration and format eligibility.

References: `repo:.trellis/tasks/10-31-archive-research/optimization-review.md`, `repo:.trellis/tasks/10-31-archive-research/bandizip-browser-integration.md`, `repo:.trellis/tasks/10-31-archive-research/bandizip-core-pipeline.md`, `repo:.trellis/tasks/10-31-archive-research/bandizip-solid-output.md`.

## Repair boundary

1. Prioritize shared ordinary extraction work: consolidate equivalent source/catalog observations under the original continuously held source authority and reduce serialized output-control overhead. Keep full codec/CRC/size validation, actual publication and cancellation truth. The measured revision count is a work target, not permission to remove every fresh check.
2. Optimize the existing drag context's per-file cache metadata, ownership transitions and retirement I/O. It already retains one worker; introducing another retained-worker layer would not address this result. Current first/repeated cursor integrity and transferred-stream lifetime contracts remain binding.
3. Reproduce the visible transition with the same real file manager for Next and Bandizip, correlating descriptor delivery, demand, receiver completion and target events. Preserve wrapper-first output; adding a collect/move phase would create a new operation absent from the observed filesystem behavior.
4. Bandizip-style destination handoff would be a delivery-protocol change requiring an explicit product/native decision. The current approved plan does not adopt a competing Shell gesture framework. Buffer size, timeout or global concurrency increases do not resolve that design difference.

## Evidence

Durable current measurements:

- `repo:.diagnostics/archive-gap/20261105-bandizip-comparison/native-measurements.json`
- `repo:.diagnostics/archive-gap/20261105-bandizip-comparison/comparison.json`
- `repo:.diagnostics/archive-gap/20261105-bandizip-comparison/native-run.log`

Retained physical trace:

- `repo:.diagnostics/archive-root-drag/20261104-89f43045/filesystem-events.jsonl`
- `repo:.diagnostics/archive-root-drag/20261104-89f43045/topology-audit.json`
- `repo:.trellis/tasks/11-02-archive-workflow-optimization/release45-drag-diagnosis.md`

## Limitations

The current native measurements use the identified prebuilt Release test host and newer Release worker, not a rebuilt latest-source host or the installed product. They establish the tested chain's gap; they are not installed Release50 acceptance. Native Job, controlled COM and Bandizip CLI have different startup, publication and completion semantics.

Bandizip drag/destination mechanisms are static reconstructed evidence from shipped binaries, not original source. This investigation executes its complete CLI extraction, not ordinary/fast physical GUI dragging. Which branch Bandizip takes with Directory Opus and the exact visual scatter/collect step remain unverified. The retained physical topology observation belongs to the identified prior operation, not a new replay on the user's present window.

The result covers this supplied ZIP. It does not establish RAR/7z/TAR performance, a universal speed ratio, exclusive attribution of every millisecond or overall Phase E/Release acceptance.
