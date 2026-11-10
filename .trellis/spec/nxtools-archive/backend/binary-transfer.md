# Private bounded archive transfer and ordinary output queue

## 1. Scope / Trigger

Read before changing Create/Add ID-source payload reads, archive candidate readback, retained-cache output writes or ordinary known-destination extraction. The private mixed framing stays on the original worker stdio/Run. Rust owns archive policy; C# owns source/output/native authority. Typed JSON controls,prompts and terminals remain. Read-only open/test,schema1 ZIP and explicit HZ5 reclamation retain separate contracts.

Owners: `repo:rust/crates/nxtools-archive/src/worker_bulk.rs`, `repo:rust/crates/nxtools-archive/src/worker_output.rs`, `repo:rust/crates/nxtools-archive/src/worker_output_parallel.rs`, `repo:rust/crates/nxtools-archive/src/formats/zip/parallel.rs`, `repo:rust/crates/nxtools-archive/src/worker_sources.rs`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveBulkFrame.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.Bulk.cs`. Reference and task contracts: [saved source chain](../../../../docs/reference-code/archive.md#opt-06-bounded-binary-bulk-transfer), [historical OPT-06](../../../tasks/11-02-archive-workflow-optimization/opt06-binary-transfer.md), [ordinary output](../../../tasks/11-02-archive-workflow-optimization/ordinary-extraction-output.md), [bounded parallel ZIP](../../../tasks/11-02-archive-workflow-optimization/parallel-extraction.md).

## 2. Signatures

- Existing acknowledged directions retain `Binding { kind,operation/call/token,sequence/offset,length }`, one pending ReplyPort binding and the fixed bounded binary record.
- C# `ArchiveBulkBinding`, `ArchiveBulkFrame.Parse`, `ReadRequest`, `Pack`, original `ReadBulkWrite`/`ReadBulkRequest` and `SendInput` remain unique owners for acknowledged source/candidate/cache directions.
- `ArchiveCreationSources.Execute` and candidate/retained `ArchiveWorkerOutputBridge.Execute` retain native validation and original replies. A raw payload grants no source,edit,volume or reclamation authority.
- Ordinary extraction requires accepted private `extraction_output_v1` inside accepted `binary_bulk_v1`. It reuses the existing raw write-record identity/range validation but installs no per-record reply binding.
- `product_job::execute_with_output_readmission` keeps `bulk_transfer:Some(1)` on the operation-local modern request; public Job schema and finite retry policy remain unchanged.
- Private `WorkerRequest.parallel_extraction:Option<bool>` / native `ArchiveWorkerRequest.ParallelExtraction` negotiates `parallel_extraction_v1` only inside ordinary bulk-v1. Production ordinary Jobs request it by default; absent/false remains serial. It is not a public Job schema setting.

## 3. Contracts

Modern schema2 operations require exactly one `binary_bulk_v1` acceptance before source payload reads or output mutation. Ordinary destination extraction additionally requires `extraction_output_v1`; missing,duplicate,malformed or unsupported agreement fails before output effects. Output preparation may protect parents but must not create destination directories before agreement. No automatic JSON data fallback.

The original u32LE high-bit binary framing, fixed header bounds, UUID/sequence/offset/range checks and <=64KiB ceiling remain for acknowledged source replies,candidate writes/readback and retained-cache writes. Those directions keep one outstanding call,matching ACK/raw reply before their next authority-bearing call and <=2transport payload buffers per active direction. Create/Edit candidates retain full readback/verification/publication. Virtual cache retains commit/Seal before member-ready.

Ordinary extraction is the narrower exception:

- C# opens the final held-relative safe component with `CREATE_NEW` or the explicitly decided direct-truncate overwrite; there is no held sibling partial or rename publication.
- Rust coalesces payload into records <=32KiB and writes the **original OS anonymous stdout pipe** without waiting for a native ACK.
- The original C# input and stdout pumps remain independent over unchanged synchronous `CreatePipe` handles. Input uses a buffer-1 `FileStream` and cancellation-prechecked synchronous `Write`; stdout `ReadFrames`/`ReadBulkWrite` use synchronous `Read`/`ReadExactly`.
- No second pipe/channel/runtime or change to handle authority/native flags. Without parallel negotiation, control rendezvous remains one call. Negotiated ordinary parallel extraction permits at most32 keyed pending controls on the original pipe, with complete-frame writes and global sequence assignment ordered through emission.
- Exact operation/token/sequence/offset/range validation remains. `output_write_ack_frames` is zero because the Host sends no per-record success result.
- The Host records the first native write failure and native code. It continues the required pump lifecycle and reports that sticky failure at the next synchronous close; later cancel/protocol errors cannot replace it.
- Logical ordinary commit remains prior-payload drain then metadata/MOTW and close. The stdout pump consumes preceding records before invoking `session.Commit`; the native commit stopwatch therefore excludes prior payload queue wait. An uncontended new ordinary file still has two synchronous JSON calls: begin and commit.

The original control reader still processes pause/resume/cancel while producers are backpressured. Progress is inserted only between whole records. EOF/cancel/transport failure wakes the exact producer; actual pipe/process/Job drain,not logical terminal or file close alone,retires Run slots. First accepted native failure and completed/unknown output facts remain authoritative.

Ordinary metadata stays in Rust per token until required codec/source integrity permits commit. Solid RAR/7z and other delayed-integrity units may retain a bounded set of open final-name outputs. Abort/failure closes and deletes only incomplete owned outputs; completed files/directories remain. Create/Edit candidates,HZ5 and cache cleanup keep their independent semantics.

Parallel ZIP admission requires at least two selected regular Stored/Deflate members, no selected encryption/special methods, a single continuously protected source and sufficient working credit. Scoped width is min(selection,32,half logical CPUs clamped1..32,remaining credit/4MiB). Shared catalog/plan/validated-record retention and256KiB coordinator allowance are deducted; each lane reserves2MiB explicit stack plus2MiB decoder/buffer/control credit. Independent source opens remain tied to the original validated record proof; never clone a Windows seek cursor for independent lanes. Unsupported/width1 paths retain sequential streaming. Whole files use a dynamic queue with100MiB active-size pressure, not a hard allocation promise. Original CRC/end/actual-byte limits remain; one aggregate enforces actual total output bytes across lanes.

Initial exclusive begins do not take the shared decision gate; only collisions,Ask/ApplyAll/AutoRename and risk questions serialize. First error stops admission and wakes replies/queue waits. Join all Rust and accepted native tasks before cleanup/terminal/lease release; delete every incomplete owned output and retain completed closes. Aggregate bytes/files are monotonic and cannot be replaced by codec-local totals. The original stdout pump still owns payload consumption and token payload-before-close order while bounded independent native begin/close tasks may overlap.

Production terminal aggregates remain numeric and low-noise. Rust `ArchiveWorkObservation` adds `output_{begin,close,directory,directory_metadata}_{calls,wait_us}`: begin includes begin/begin_overwrite, close is commit and directory includes directory/directory_new. Each wait starts after the previous Rust tail flush and covers control serialization/send/reply; close wait also includes queued payload the Host must consume before close. Counts include returned failures/cancel. Native aggregates are `native_begin_ms`, inclusive `native_parent_resolution_ms`, `native_final_open_ms`, `native_create_ms`, `native_open_inspection_ms`, `native_directory_ms` and `native_directory_metadata_ms`. `native_commit_ms` measures only `session.Commit` metadata/handle close after payload drain; `output_close_wait_us` is the full Rust await. JSON frames remain `2N + operation overhead`, `output_write_ack_frames=0`, `output_ipc_ack_us` remains Rust pipe send/backpressure, and fingerprint/postpublication remain zero. No chunk/path/payload/token/secret logs. Durations are inclusive/overlapping, not additive and not exclusive IPC.
Parallel terminal work additionally reports numeric `parallel_workers_planned`, `parallel_workers_high_water`, `parallel_members_completed`; native pending-operation high water is owned by the Host. These are bounded lifecycle aggregates, not per-member logs.

## 4. Validation & Error Matrix

| Condition | Result |
| --- | --- |
| Missing/unsupported bulk or ordinary-output capability | Request rejection before source/output effect |
| Body/header/version/kind/identity/sequence/range disagreement | Original typed protocol failure; zero invalid completion |
| Modern JSON data success | Protocol refusal; no downgrade |
| Acknowledged source/candidate/cache short read/write or mismatch | Original native failure; no successful ACK |
| Ordinary short/native write failure | Sticky first native category/code returned by commit/close |
| Ordinary producer cancel/EOF while blocked | Wake producer; close/delete incomplete owned output; actual drain owns slot |
| Ordinary metadata/commit before earlier payload consumption | Original stdout pump consumes earlier records before `session.Commit`; full Rust close wait includes that queued payload |
| Multi-output delayed-integrity failure | Delete only incomplete owned handles; retain completed journal facts |
| Failure/unknown/late cancel after completed close or Create/Edit publication | Preserve actual journal/output truth; no unsafe retry or rollback |
| Parallel requested outside ordinary bulk | Invalid request before effects |
| Parallel capability absent, duplicate or malformed | Native protocol refusal before output mutation |
| Insufficient lane credit or unsupported ZIP selection | Original serial route, not a new output failure |
| Pending control33 or stale/duplicate/mismatched reply | Busy/protocol rejection; no invalid completion |

## 5. Good/Base/Bad Cases

Good: Create Store retains acknowledged source/candidate behavior; ordinary Extract uses <=32KiB no-ACK records through the original synchronous pumps and receives one metadata+close result. Base: absent parallel request is serial; empty ordinary file uses begin+commit without data. Bad: add another pipe/runtime, use async wrappers over synchronous handles, interleave frame bodies, hold reply-state locks across backpressure/prompts, treat pipe send as native success, or apply ordinary semantics to Create/Edit/cache/HZ5.

## 6. Tests Required

- Preserve existing literal-frame/source/candidate/readback/retained-cache negotiation,bounds,sequence,short-read,ACK,cancel,EOF,first-failure and cleanup regressions.
- Ordinary boundary coverage records exact begin/close/directory/directory-metadata waits, native begin/open/directory subsets, <=32KiB coalescing, original-pipe backpressure, zero write ACKs and two controls per uncontended file.
- Pump coverage fixes the existing synchronous `CreatePipe` handles and original independent pumps: input buffer1+synchronous cancellation-prechecked Write; stdout synchronous Read/ReadExactly through ReadFrames/ReadBulkWrite.
- Native short/write failure remains sticky through later record consumption and close; cleanup removes only the incomplete owned output.
- Metadata/MOTW, delayed-integrity abort, prior completed-file retention, actual runner cleanup and Create/Edit/HZ5/cache preservation remain required.
- Comparable final timing uses the parent-owned genuine Release nine-trial set; historical samples remain immutable and do not establish a performance pass.
- Parallel real-worker/native-boundary coverage holds the first begin until another enters, asserts finite CPU/selection/memory/request caps, reverse reply routing/stale rejection, serialized ApplyAll/risk decisions, monotonic aggregate progress, exact bytes/CRC/native first error, all-incomplete cleanup/completed retention and joins before source release. `repo:rust/crates/nxtools-archive/tests/parallel_extraction.rs` and `repo:csharp/NxTools.PlatformHost.Tests/ArchiveParallelExtractionTests.cs` own directed boundary fixtures.

## 7. Wrong vs Correct

Wrong: `write record -> wait JSON ACK -> fingerprint partial -> rename -> reopen` for every ordinary file.

Correct: `begin final name -> bounded original-pipe records/backpressure -> codec/source integrity -> synchronous drain+metadata+close`; acknowledged source/candidate/cache directions retain their existing protocol and lifecycle.
