# Ordinary extraction output alignment

Parent phase: **Phase E — Format expansion and ZIP editing / in_progress**. This is stabilization inside the existing phase and does not add or renumber an OPT package.

Status: **ordinary-output alignment and bounded parallel ZIP supplied-sample acceptance pass**. Current button/Root medians are2370/2283ms versus Bandizip1827.211ms,1.297×/1.249×, both<=1.5×; [parallel results](parallel-extraction-results.md) retain exact nine-output bytes, lifecycle verification, cleanup and restoration. The [initial results](ordinary-extraction-output-results.md) and [initial per-trial evidence](evidence/ordinary-extraction-output-2026-11-07.json) remain historical:4474/4378ms versus1512.826ms,2.957×/2.894×. Overall Phase E and installed/human acceptance remain separate.

## Authorized follow-up — 2026-11-07

The user authorizes bounded Phase E measurement of ordinary output begin, close, path-open, directory and directory-metadata cost, followed by the necessary measured optimization. A duplicated-output-handle boundary is within that authorization only if the measurements causally support it; it is not the selected current change.

The measured current choice is synchronous I/O in the **original independent C# pumps** over the existing synchronous `CreatePipe` handles:

- the original input pump constructs its `FileStream` with buffer size 1 and uses synchronous `stream.Write` after a cancellation precheck;
- the original stdout pump keeps `ReadFrames`/`ReadBulkWrite` and uses synchronous `stream.Read`/`ReadExactly`;
- that completed synchronous-pump slice adds no pipe, channel, runtime, thread pool, request map, protocol/capability change, handle-authority change or native-flag change.

Ordinary final-name `CREATE_NEW`, <=32KiB raw records on the original pipe, zero write ACKs, sticky first native error, metadata+close and incomplete-output-only cleanup remain. Create/Edit candidates, `ArchiveEntryCache` and HZ5 remain unchanged. The synchronous-pump route used two serial begin/commit controls; its [historical genuine Release trials](ordinary-extraction-pump-results.md) were byte-correct at4241/4214ms button/Root medians against Bandizip1476.011ms,2.873×/2.855×, failing that runtime's<=1.5× gate.

## Authorized cross-file parallelism

The subsequent user request selects [bounded parallel ZIP extraction](parallel-extraction.md). Its private negotiation, finite lane/request limits, independent source cursors, serialized decisions, aggregate progress and multi-incomplete cleanup extend the ordinary path under the same owners. Synchronous pumps and direct-final-name output remain. Compiled regressions and genuine nine-trial acceptance pass: button2370ms/1.297× and Root2283ms/1.249× versus Bandizip1827.211ms, with exact4492files /313,474,195bytes in each output. [Current report](parallel-extraction-results.md), [evidence](evidence/parallel-extraction-2026-11-09.json). Historical serial reports are unchanged.

## Scope

This contract applies to ordinary extraction with a known destination directory:

- toolbar, dialog and context extraction;
- Smart, Here, Name, Desktop and custom-destination plans;
- destination-directory drag extraction;
- all supported extraction formats through the shared output owner.

It does not change virtual descriptor/IStream drag cache materialization, preview/entry cache sealing, archive creation, archive editing, split-family candidate publication or HZ5 reclamation. Archive creation and editing keep candidate verification, held readback and replacement. HZ5 keeps its append/flush/readback and recovery authority. `ArchiveEntryCache` keeps Seal and verified-cursor behavior.

## Reference alignment

The retained Bandizip evidence establishes one opened source/catalog, selected/dependency-aware decode, incremental codec integrity, direct final-path output or a bounded producer/consumer output queue, native write-error retention, drain before close and current-output deletion on cancellation. The queue drain is a completion barrier, not `FlushFileBuffers`, and cancellation cleanup does not roll back earlier successfully closed files.

The retained output-factory/direct-open evidence does not establish every proprietary creation flag. NxTools' approved Windows contract is explicit: C# opens the final relative component with `CREATE_NEW` semantics under the held safe parent.

References:

- [shared core pipeline](../10-31-archive-research/bandizip-core-pipeline.md)
- [solid output lifecycle](../10-31-archive-research/bandizip-solid-output.md)
- [failure and cleanup](../10-31-archive-research/bandizip-failure-cleanup.md)
- [operation lifecycle](../10-31-archive-research/bandizip-operation-lifecycle.md)
- [archive reference note](../../../docs/reference-code/archive.md)

## Output ownership and naming

Rust continues to own selection, path policy, conflict policy, codec integrity, source checks, metadata semantics and operation state. C# continues to own held Windows namespace capabilities, file handles, native writes, metadata APIs, close/delete and native results.

For every ordinary file output, C# resolves each validated relative component beneath the continuously held no-reparse destination namespace and opens the **final requested name** with `CREATE_NEW` semantics. Ordinary extraction has:

- no sibling `.nxpart` or other temporary output;
- no final rename or replacement publication step;
- no per-write SHA/content hash;
- no output content-hash or file-ID fingerprint;
- no per-file `VerifyDirectory` walk after the held-relative open;
- no published-file reopen;
- no `FlushFileBuffers` durability step.

Traversal, rooted/drive/UNC/ADS/device-name, invalid-component, reparse and link-escape refusal remain. The original source lease remains held through required source checks and actual worker cleanup.

## Conflict flow

The first `begin` attempts `CREATE_NEW` on the final relative path.

- A new file returns a live owned output token.
- An existing target returns `success:false`, `error_code:target_exists` and bounded `existing_bytes`; it does not allocate an output token or mutate the target.
- Rust alone applies the existing Ask, Overwrite, Skip or AutoRename policy and current-attempt ApplyAll state.
- There is no per-file `observe_target` round trip.
- Overwrite uses `begin_overwrite` for the same observed relative path and directly opens/truncates that path. It has no target signature, file-ID/content CAS or replacement candidate.
- AutoRename attempts bounded numbered final names with `CREATE_NEW`; `target_exists` advances Rust to the next candidate, while other native failures stop and no candidate ever becomes an overwrite.
- Skip produces no file handle and remains ineligible for source recycling.

The accepted overwrite tradeoff is direct path truncation after the current conflict decision. Existing Windows namespace/reparse checks still apply at the held parent/component boundary.

## Bounded streaming and close ordering

Ordinary extraction negotiates the private `extraction_output_v1` capability inside the existing bulk-v1 agreement. Capability mismatch fails before output mutation. This is not a public protocol field and does not create a second channel, reader, worker runtime or queue.

Rust coalesces output into binary records of at most 32 KiB and writes them to the **original OS anonymous pipe** without waiting for a per-record native ACK. The original C# input and stdout pumps remain independent over synchronous `CreatePipe` handles. The input pump uses a buffer-1 `FileStream` and synchronous `Write` after a cancellation precheck; the stdout pump's `ReadFrames`/`ReadBulkWrite` use synchronous `Read`/`ReadExactly`. The OS pipe provides bounded producer/consumer backpressure.

The host retains the first native write failure and its native code. Subsequent transport consumption cannot erase it. The next synchronous file close reports that failure. Logical `commit` remains ordered as:

1. the stdout pump consumes all earlier records for that token;
2. the sticky first native write error is returned when one exists;
3. C# applies the Rust-retained mtime, readonly/hidden attributes and optional MOTW on the same open handle;
4. C# closes the handle and marks the journal entry complete.

This drain is an ordering barrier, not an OS durability flush. The `native_commit_ms` stopwatch starts only when the single stdout pump calls `session.Commit`, after preceding payload has already been consumed; it measures the native metadata/handle-close call, not earlier queued-payload wait. Rust `output_close_wait_us` is the full commit await and therefore includes control serialization/send/reply wait and any queued payload the Host must consume before close. An uncontended newly created file still has exactly two synchronous per-file JSON round trips: `begin` and `commit`.

## Metadata

Rust retains file metadata per live output token until codec/source integrity permits commit. The ordinary `commit` carries that metadata to C#. C# applies the fixed MOTW stream, time and whitelisted readonly/hidden attributes on the open final file, verifies API/observed metadata results required by the metadata contract, then closes it.

Explicit created-directory metadata remains finalized deepest-first by its existing owner. The implementation does not add a per-file directory reverification pass.

## Integrity, cancellation and failure

Codec end, CRC/MAC where defined, decoded size, actual-byte budgets, path/reparse policy, source lease/revision, conflict semantics, progress and cancellation remain required.

A file is complete only after all integrity required for that format unit is known and the ordinary close succeeds. ZIP and independent members may close at their valid member boundary. Solid RAR predecessors, 7z folders and other delayed-integrity units keep the affected owned final-name handles incomplete until their required integrity result.

On cancellation, CRC/codec/size/source failure or native write/close failure:

- close and delete only the current incomplete owned ordinary output or bounded set of incomplete outputs still pending the required solid/format integrity;
- keep every successfully closed file and finalized directory;
- do not scan completed outputs, compare fingerprints or roll them back;
- never delete a pre-existing skipped target or an output not owned by the current attempt.

Any visible “delete extracted output on cancel” setting means only current unfinished owned output under this contract. Whether that setting is presently exposed is a separate parent investigation.

Late cancellation cannot erase successfully closed journal facts. A task with completed output is not eligible for generic zero-output Retry or source recycling.

## Journal and result truth

The native journal retains bounded result paths, created counts and per-output completion state needed for terminal projection, Locate and cleanup truth. It does not retain direct-file content hashes or file-ID fingerprints for ordinary output.

Terminal counts distinguish created/opened, successfully closed, skipped and deleted-incomplete outcomes. Completed files/directories survive later failure. Unknown native close/delete outcomes remain explicit and keep cleanup ownership until the original runner resolves them.

## Diagnostics and counters

Keep low-noise accepted/start/meaningful-state/terminal events with operation correlation, elapsed time and native error code. Never log archive/member paths, output paths, payloads, tokens, passwords or per-chunk events.

`ArchiveWorkObservation` records these Rust control observations:

- `output_begin_calls` / `output_begin_wait_us`: `begin` plus `begin_overwrite`;
- `output_close_calls` / `output_close_wait_us`: `commit`;
- `output_directory_calls` / `output_directory_wait_us`: `directory` plus `directory_new`;
- `output_directory_metadata_calls` / `output_directory_metadata_wait_us`: directory metadata finalization.

Each wait starts after flushing the previous Rust coalescing tail and covers control serialization, send and reply wait, including queued payload the Host must consume before close. Counts include calls returning failures or cancellation.

Native aggregate fields are `native_begin_ms` for the whole bridge begin action; `native_parent_resolution_ms` for validation/cache parent resolution as an inclusive subset; `native_final_open_ms` for ordinary final-file open allocation, syscall and inspection; `native_create_ms` for `NtCreateFile` plus P/Invoke marshalling only; `native_open_inspection_ms` for post-open inspection; and `native_directory_ms` / `native_directory_metadata_ms` for their complete native actions.

For ordinary extraction, `native_commit_ms` measures only `session.Commit` metadata and handle close after the stdout pump has already drained preceding payload. `output_close_wait_us` measures the full Rust await. `fingerprint_ms = 0`, `postpublication_ms = 0`, transport JSON frames remain `2N + operation overhead`, `output_write_ack_frames = 0`, and `output_ipc_ack_us` remains pipe send/backpressure time rather than ACK wait or exclusive native-write time. All listed durations are inclusive and may overlap; they are neither additive nor exclusive IPC attribution.

## Acceptance requirements

Acceptance must exercise the original Job/worker/C# owners across admitted ZIP, 7z, RAR, TAR/compound and standalone-stream routes. Required observations include:

- final-name `CREATE_NEW` with no `.nxpart`, rename, fingerprint, published reopen or durability flush;
- exact two synchronous JSON calls for an uncontended new file and zero write-ACK frames;
- bounded <=32 KiB records, pipe backpressure, single-pump ordering and sticky first native write error at close;
- Ask/Overwrite/Skip/AutoRename/ApplyAll, `target_exists` with `existing_bytes`, no `observe_target`, direct-truncate overwrite and bounded AutoRename `CREATE_NEW` progression without overwrite;
- metadata carried on commit and observed on the closed final file;
- CRC/codec/source/cancel/native failures deleting only incomplete owned outputs while preserving completed files/directories;
- exact journal counts/completion states, source preservation, actual cleanup and low-noise diagnostics;
- unchanged archive create/edit candidate verification, HZ5 reclamation and `ArchiveEntryCache` Seal/cursor contracts.

- exact Rust begin/close/directory/directory-metadata call and wait counters, including failure/cancel calls and the post-tail-flush start boundary;
- exact native begin/parent-resolution/final-open/create/inspection/directory/directory-metadata aggregates with inclusive-subset semantics;
- synchronous original input/stdout pump I/O over unchanged `CreatePipe` handles and unchanged protocol/handle authority;
- final genuine Release nine-trial comparison before any performance acceptance.

Source/spec checks and compiled/native fixtures remain separate from physical UI, installed product, Release performance and user acceptance.

The [initial verification report](ordinary-extraction-output-results.md) and [synchronous-pump report](ordinary-extraction-pump-results.md) retain their own boundary regressions, byte-correct genuine trials and failed<=1.5× verdicts. The current [bounded parallel report](parallel-extraction-results.md) and [evidence](evidence/parallel-extraction-2026-11-09.json) separately pass compiled lifecycle, genuine supplied-ZIP performance, independent bytes, actual cleanup and environment restoration. Overall Phase E, installed-package and human acceptance remain separate.

## Performance context and next cache work

The retained Bandizip-gap investigation is historical context for a different supplied ZIP sample, not current acceptance and not the current 4,492-file benchmark. On that sample:

- Release ordinary extraction median: **2780.91 ms**;
- Bandizip CLI median: **411.86 ms**;
- controlled Root route median: **7325.86 ms**;
- 208 Seal operations: **764 / 741 / 730 ms**;
- 208 verified cursor opens: **297 / 276 / 280 ms**;
- content volume: **206,572,163 bytes**, hashed once at Seal and once again at cursor opening.

After ordinary output alignment, the next separately owned cache work is the measured Seal/cursor double-hash and retirement path. This contract does not change that cache lifecycle or claim a current performance result.