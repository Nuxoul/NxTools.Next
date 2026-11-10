# Bounded parallel ZIP extraction

Parent phase: **Phase E — Format expansion and ZIP editing / in_progress**. This is ordinary-extraction stabilization in the existing task.

Status: **implemented; compiled lifecycle, independent bytes and genuine supplied-ZIP performance acceptance pass**. The matching isolated Release completed button3, Root external drag3 and Bandizip parallel=1 controls3: medians2370/2283/1827.211ms, ratios1.297×/1.249×, both<=1.5×. All nine outputs match4492files /313,474,195bytes; verified outputs are cleaned and the original Desktop/Host/DOpus baseline is restored. [Current results](parallel-extraction-results.md), [per-trial evidence](evidence/parallel-extraction-2026-11-09.json). The completed [synchronous-pump trials](ordinary-extraction-pump-results.md) remain the historical serial baseline:4241/4214/1476.011ms,2.873×/2.855×.

## Reference and scope

The user authorizes cross-file parallel extraction after the same-machine Bandizip `useParallelExtraction` contrast. [Parallel reference](../10-31-archive-research/bandizip-parallel-extraction.md) records the configuration and actual ZIP scheduler evidence. The reference uses independent member inputs, dynamically assigned whole files, bounded workers, serialized callbacks, cumulative progress and joining all workers before resource release.

Initial product admission is ordinary ZIP with at least two selected regular files, no encryption and only Stored/Deflate members, one protected ordinary source and the original native destination owner. A selected member is never split between workers. Encrypted/special-method ZIP, split sources, creation/editing, HZ5, virtual-entry cache materialization and other format owners keep their current paths. RAR/7z solid integrity remains sequential. This does not increase global running Jobs or FIFO slots.

## Private negotiation

Production ordinary extraction requests parallel execution by default through private `parallel_extraction:true` inside bulk-v1; the worker must advertise `parallel_extraction_v1` before output mutation. Absent/false requests retain the existing serial route. `extraction_output_v1` remains required. Public Job/UI schemas and original pipes do not change; no additional transport, runtime or payload authority is introduced.

Only the parallel ordinary path extends private control rendezvous from one to at most32 outstanding call IDs. Replies are routed to their original attempt/call; duplicate, stale and mismatched replies remain rejected. A lane waits for each begin/commit reply, not for ordinary payload ACKs. Other acknowledged transfer directions retain their original semantics.

## Scheduling and memory

Rust owns a fixed scoped worker group. Width is the minimum of selected regular files,32,`max(1,min(logical CPUs,64)/2)` and remaining admitted working credit divided by the complete per-lane reservation. The reservation includes an explicit bounded thread stack, codec workspace,64KiB decode buffer,32KiB output buffer and lane/control state. Shared catalog, extraction-plan and validated-record retention are deducted before lane admission. Insufficient credit or width1 uses the existing serial route.
Current lane reservation is4MiB: an explicit2MiB thread stack plus2MiB for Stored/Deflate state,64KiB decode,32KiB output,control/frame and lane state. Shared catalog/plan/record retention and256KiB coordinator allowance precede this division. A9MiB worker fixture admits two lanes; lower credit retains sequential streaming.

The coordinator validates the original catalog/selection/record plan and source revision once through the original engine boundary. Under the continuously held native source lease, each lane opens an independent source cursor. Cloned Windows seek cursors are not independent readers. The retained record proof accepts only its original member identities, not caller-supplied offsets or arbitrary readers.

Workers dynamically claim whole members in original queue order. Active declared sizes apply reference-aligned100MiB admission pressure while allowing progress for a large first member. This is I/O pressure control, not a replacement for actual-byte or working-memory limits. Decoders retain original end/CRC/size checks. Aggregate actual decoded bytes enforce the original total output budget across every lane, including a later failed member.

## Output and decisions

Each lane keeps its original metadata/token owner and <=32KiB ordinary coalescing buffer. Complete JSON/binary frames are atomic on the original stdout pipe. Global binary sequence assignment is ordered with emission. No reply-state lock is held across pipe backpressure, a native call, or a prompt.

C# permits bounded native begin/close operations for independent files to overlap. The original stdout pump still consumes every payload record; a token's commit is dispatched only after all preceding payload for that token has been consumed. Payload for a token cannot follow its close. Native begin opens the final name with held-relative `CREATE_NEW`; ordinary output still has no temporary publication, fingerprint/hash, reopen verification or durability flush.

Uncontended initial `CREATE_NEW` is outside the shared decision lock. Only collision resolution, Ask, ApplyAll and AutoRename reservation/retries serialize. Risk questions use the same operation-level decision gate, so conflict/risk prompts never install competing tickets. ApplyAll and pending tickets remain attempt-scoped. Existing future-name reservation and stale-ticket rules survive.

C# protects short shared-state/cache/admission updates and per-file operations. It does not hold one session-wide lock across native file open, metadata or close. The existing bounded stdin queue admits concurrent replies without dropping one merely because the queue is temporarily full.

## Failure, cancellation and completion

The first real error is sticky across Rust tasks, native replies, write/close failure, EOF and cancellation projection. Stop new task admission, wake blocked claim/reply waiters, finish native work already accepted, and join every worker/native operation before terminal cleanup and source/destination lease release. Stopping admission is not cleanup completion.

Cancellation, integrity failure or native failure deletes every currently incomplete owned ordinary file and retains every successfully closed file. It does not roll back completed files or touch skipped/external targets. A close that already succeeded remains a journal fact even when cancellation follows. Original source-recycling eligibility and completed-output retry rules remain authoritative.

Directory preparation is coordinated before file work. Explicit directory integrity and deepest-first metadata finalization remain in the original owner, after file workers have joined successfully.

## Progress and diagnostics

One operation-level aggregate adds actual decoded/output deltas and completed-file counts. Lane-local totals never replace the overall snapshot. Bytes/files do not decrease; intermediate publication keeps the existing coalescing limit and terminal publication is exact. The current member label represents an active/progressing member, not a fabricated single serial current file.

Existing correlation and native errors remain. Terminal numeric aggregates include planned worker width, active-worker high water, completed members and native pending-operation high water. No per-file/path/payload/chunk logging is added. Existing inclusive timing counters remain overlapping observations, not exclusive IPC costs.

## Verification and acceptance

The agreed seams are the original worker wire and C# native output lifecycle. A red-capable controlled test holds the first native begin until another begins; serial execution must fail it. Assert real overlap and deterministic finite high water, serialized conflict/ApplyAll tickets, monotonic aggregate progress, independent exact bytes, CRC/native first-error retention, multiple-incomplete cancellation, completed retention, and joining before cleanup/source release. Tests use barriers and owned probes, not performance-time assertions.

Preserve ordinary output, source/path/reparse, metadata/MOTW, acknowledged transfer, create/edit candidate, HZ5/cache and RAR/7z/TAR-stream regressions. Source/spec checks, compiled native fixtures and actual GUI acceptance remain separate evidence levels.

Rust owners: `src/formats/zip/parallel.rs` schedules whole members; `src/formats/zip/plan.rs` binds independent cursors to the original validated proof; `src/worker_output_parallel.rs` owns complete frames and the shared collision/risk gate; `src/worker_output.rs` routes bounded keyed replies and per-token output; `src/control.rs` owns actual-byte/progress aggregates. Directed checks include real worker overlap/exact bytes/sequence, CPU/selection/memory bounds, serial compatibility, one ApplyAll with stale-ticket isolation, multiple pending cancellation and CRC/native error wakeup. Compiled native lifecycle and final timing are separate evidence.

C# owners: `ArchiveWorkerRunner.Parallel.cs` bounds accepted native tasks and physically sent replies; `ArchiveWorkerOutputBridge.Parallel.cs` separates short shared admission from per-file operations; `ArchiveOutputSession.Extraction.cs` owns held-relative final-name creation, payload and logical close. Completed tasks are removed before pending-operation accounting. A non-cancel native or worker operation failure replaces only a provisional cancellation, preserving the first real failure and native code. Production `archive.worker.work` retains all three Rust parallel aggregates.

Directed compiled verification: Rust isolated-codecs library103/103, actual-worker parallel contracts9/9 and strict all-target Clippy passed. C# parallel fixtures10/10 passed, including actual Rust/native mixed Stored/Deflate overlap and exact bytes, cancellation with two incomplete native files after one completed close, CRC failure retaining completed bytes, native/worker failure after cancellation,32-operation admission, late payload refusal and nonaccumulating pending diagnostics. Ordinary-output/source/conflict/long-path/metadata-MOTW and actual encrypted extraction preservation passed15/15 with no skips. Every actual chain drains original pipes/Job/native tasks and verifies source lease release. These are lifecycle checks, not performance trials.

The full isolated-codecs archive run completed60 targets:392passed,0failed,1existing opt-in benchmark ignored. It includes the original RAR/7z/TAR-stream native readmission chains. Default production Job conflict/cancellation, selected ZIP/7z/RAR source binding, source-bound output readmission and password/native-failure preservation passed4/4. Final strict all-target isolated-codecs Clippy, scoped rustfmt/diff checks and126-document spec contracts passed. Optional external corpus tests without their opt-in environment are not additional corpus acceptance.

The matching scratch Release completed three genuine toolbar/button trials, three genuine Root-to-external-directory drags and three Bandizip controls with parallel=1, without concurrent build/test/archive/disk-comparison load. Both entrance medians pass<=1.5× Bandizip. Every output's file set, lengths and bytes match; Root wrappers, complete OLE lifecycles, actual cleanup and original Desktop/Host/DOpus restoration are retained in the [separate result](parallel-extraction-results.md). Prior serial reports are unchanged. Overall Phase E, installed-package/deployment and human visual/interaction acceptance remain separate. No package, installation, service change, commit or push was performed.
