# Bandizip solid extraction and output lifecycle

## Data-path model

Bandizip7.32.0.1 retains the opened input, loaded catalog and format decoder for an extraction attempt. Selection changes the decode plan and output creation, not the identity of the underlying archive. The inspected paths differ by their dependency unit:

| Format/path | Selection and decode unit | Unselected data | Reuse and parallelism |
| --- | --- | --- | --- |
| Ordinary ZIP Stored/Deflate | Selected member at its recorded data offset | Unselected payloads are outside the selected loop | Retained catalog/input; conditional member parallelism. The supplied single small JAR takes the sequential/direct-output path. |
| Solid RAR/RAR5 | One ordered selected-range call | Walks catalog indices from zero through the last selected index; preceding/intervening members advance decode state without creating their filesystem outputs | Retained decoder/dictionary; the specialized member loop is sequential. Codec-internal workers are distinct from independent member jobs. |
| 7z | Selected indices grouped by their folder/solid block | Decode from the block's first file through its last selected file; intervening bytes advance the stream but do not create unselected outputs | One block decode for several selected files in that block; compatible coder graph reuse, with per-block setup. ST/MT coder mixers operate inside this plan. |
| Conditional buffered filesystem output | Binary producer queue and native consumer | Small producer writes are coalesced | Separate32-KiB consumer staging; dequeue/write overlap; high-water backpressure; drain/join before final close. |

The ZIP mechanism is recorded in [the shared core trace](bandizip-core-pipeline.md). These are connected static implementation paths, not new format benchmarks.

## RAR and RAR5

### Open, catalog and solid dispatch

`CArkMan` open `0x180006460` constructs the format object through `0x18000eb80`. RAR reads successive headers through `0x18002f410 → 0x18002fc50`; RAR5 uses `0x180033c80 → 0x180033e90 → 0x180032a20 → 0x180032730`. Both retain member pointers in the decoder's catalog at `+0x28` and retain the input interfaces.

RAR's archive-header flag `&8` sets decoder byte `+0x790`. RAR5 admission copies compression-information bit6 to member field `+0x88` and sets the same decoder byte when present. Base initialization/reset clears it. The actual assembly writes are `0x18002f7be` and `0x180032899`.

Shared selected extraction `0x180059a90` tests `+0x790` at `0x180059be9`. The true branch invokes virtual `+0xe0` once: RAR vtable `0x1801f24d0` resolves it to `0x180030ad0`; RAR5 vtable `0x1801f2630` resolves it to `0x180034920`. The false branch uses ordinary per-member handling through `0x18005a120 → 0x18005a4c0 →` virtual `+0x30`, resolving respectively to `0x1800314b0` and `0x180035570`. The separate `+0x792` tests participate in the ordinary/conditional-parallel decision, not the solid-dispatch flag.

### Ordered predecessors and selected output

The specialized loops start at catalog index zero, compare each encountered index with the next selected index, and stop after the final supplied selected index. Selection is consumed in ascending encounter order. These loops do not establish a nearest-solid-boundary seek optimization or equivalence for arbitrary unsorted indices.

The output factory is installed through virtual `+0x68 → 0x180057b20`, storing it at decoder `+0x778`. Manager setup supplies `0x18001aec0`. In both specialized loops, only a selected-index match calls this factory. The unselected predecessor path clears the output pointer and advances stored/compressed processing without creating a filesystem output. Stored paths `0x180030830` and `0x1800345e0` use up to8-KiB blocks, condition writes on an existing output, and continue integrity accounting when output is absent.

### Decoder, dictionary and terminal state

RAR lazily retains its compressed decoder at `+0x1da8` through `0x180031c60`; RAR5 retains one at `+0x8f8` through `0x1800363c0`. The latter also creates an internal worker facility using the processor count, capped at8 for the observed initialization. This does not parallelize the outer solid-member sequence.

Both use dictionary configurator `0x1800caab0`. It returns without reallocating when the existing capacity suffices. On growth it distinguishes the continuation parameter; its contiguous-buffer continuation branch copies retained circular history into the larger allocation. The selected-range path passes continuation state for predecessor decoding, while the ordinary RAR5 per-member compressed path configures without that continuation parameter. Allocation reuse and history continuation are separate decisions.

The selected-range routines check member integrity after compressed work and report decode/checksum/output failures through error fields and callbacks. `0x1800583c0` consults callback-updated cancellation/error state. Generic per-member handling additionally checks output result and decoded size. Releasing the selected output at a file boundary does not authorize resetting the solid decoder before the next dependent member.

## 7z

### Retained handler and catalog

`CArk7Zip` constructor `0x18004c2f0` creates the retained `NArchive::N7z::CHandler` through `0x1800ef080`. Its base decoder interface is at object `+0x28`, vtable `0x1801f4ab8`; constructor assembly separately establishes the extract-callback and output-stream interfaces, avoiding ambiguity between multiple tables with the same class name.

Open `0x18004c7a0` wraps the retained input in `CMyInStream` and invokes handler `Open` at `0x1800ebdd0`. That handler reads into its database through `0x180134660 → 0x180138050` and retains the input at `+0x48`. The adapter then enumerates properties into its own member table. Extraction reuses this state rather than reparsing the catalog per selected file.

### Selected indices to one decode per block

Selected adapter `0x18004f0b0` constructs a sorted, deduplicated index array through `0x18004f220`, then calls `0x18004e170`. The retained handler's virtual `+0x38` resolves to `0x180139760` using the `IInArchive` table at `0x1801f9248`.

That function uses file-to-folder mapping `handler+0x1f0` and folder-first-file mapping `handler+0x1e8`. It groups subsequent selected indices belonging to the same folder and determines the last selected file. Its required decoded-byte total covers the folder's first file through that file, including unselected predecessors and gaps. The group enters `0x1801611c0` once, not once per selected member. Entries without a data folder take the separate empty/directory handling path.

`NArchive::N7z::CFolderOutStream` tracks current file index, remaining file bytes and remaining selected indices. Its `Write` at `0x180139580` splits produced bytes at file boundaries. `0x180139270` chooses extract/test/skip callback mode. After the final selected item, the stream returns `0x20000010` on excess output; both inspected mixer paths recognize this stop result. The planned output size is the selected prefix, not an instruction to publish the rest of the block.

### Discarded output, CRC and native completion

The adapter's `GetStream` at `0x18004e350` searches its selected-index array. Only a selected match invokes the output factory; an unselected member releases/clears the native output pointer. It may still return its bridge stream to the folder decoder. Bridge `Write` at `0x18004ee80` consumes bytes without a native write when that pointer is absent, preserving decode advancement without a filesystem output.

The folder stream initializes a per-file CRC accumulator when its configured CRC/metadata conditions apply, updates it as decoded bytes pass through `Write`, and compares it at `0x1801393f0`. It reports mismatch result3 through `SetOperationResult`. Empty entries are also handled through `0x1801394a0`; decode/error completion uses `0x1801396d0`. This is per-file integrity, not a whole-source digest in the file loop.

Adapter `SetOperationResult` at `0x18004e940` maps checksum/decode errors, requests cleanup for failed output where applicable, applies metadata, invokes output virtual `+0x40` for its pending-write result, and releases the output. Bridge write failure is reported by `0x18004ee80`, including native disk-full mapping. Progress callback `0x18004ed20` propagates cancellation/error as `0x80004004`; the separate progress-ratio callback `0x18004eda0` also consults its caller's cancellation result. Handler/adaptor return paths release remaining outputs and report terminal failure rather than inferring success from the existence of a file.

### Reuse, initialization and internal concurrency

`0x180139760` keeps one decode context for its folder loop. In `0x1801611c0`, compatible stream/coder graph mappings permit mixer reuse; mismatched topology rebuilds the mixer. Each block still configures coder properties, sizes, input offsets and limited input streams before invoking its code operation.

The ST and MT mixers are different internal execution strategies, chosen by decode-context configuration. Their relevant interface tables are `0x1801fe928` and `0x1801fe040`. MT reset `0x180171f80` resets pipe/event state for reuse. MT code `0x180172650` starts or wakes coder workers, runs its main coder, waits for the others, then aggregates cancellation, allocation and decode results. ST code `0x1801711c0` calls its coder with the configured streams and handles the same selected-prefix stop result. The inspected outer handler loop processes folders sequentially; these records do not establish independent parallel extraction of selected files within a solid block.

LZMA `Code` `0x1800ed8b0` resets input/output counters, dictionary position and decoder state before its streaming helper `0x1800ed610`. LZMA2 `Code` `0x1800ee0c0` reuses its allocated context but supplies new per-call streams/settings to `0x18013bc00`. Its ordinary decode path `0x18013b860` calls `0x18013b770`, which preserves suitable allocations while resetting decode position/state before processing. Multiple files within one solid-block invocation therefore share continuous decoding; retaining the coder allocation is not permission to retain arbitrary previous-block decoding history.

## Buffered filesystem output

### Producer and consumer

The conditional output factory and producer are described in [the ZIP trace](bandizip-core-pipeline.md). The byte-producing buffered open is virtual `+0x20 → 0x1800230f0`: it opens the native file, allocates a separate32-KiB staging block and starts worker `0x180023950`. Virtual `+0x28 → 0x180023050` is the alternate filesystem/setup path, not this worker-creation entry.

The ring starts with64KiB of storage. Producer `0x180023340` enqueues under a critical section, signals the data event, and waits when occupancy exceeds its128-KiB–4-MiB high-water threshold. It enqueues before testing that threshold, so the threshold is not a hard allocation ceiling. Oversized producer blocks first drain the queue, then use the native direct-write helper. The stalled-progress wait resets when occupancy changes; it is not an unconditional fixed delay per block.

Worker `0x180023950` normally dequeues32KiB only when that much is available. During drain/close it takes the remaining partial block as well. It copies under the lock, signals producer-progress event `+0x90`, releases the lock, then calls `0x180022db0 → WriteFile`. Producer work can resume while that native write runs. Failure saves `GetLastError()` at `+0x9c` and sets failure `+0x98`; the worker continues consuming/attempting queued work rather than immediately abandoning its lifecycle.

### Drain, close and cancellation

Drain `0x180023580` sets request `+0x24`, wakes event `+0x80` and waits for completion event `+0x88`. The worker signals completion when the queue is empty or a writer failure is present. This is a queue barrier, not `FlushFileBuffers`; an error arising during the wait must still be considered through output error/final close state.

Close `0x180023620` requests worker stop `+0x20` and joins it before final file handling. Stop means drain queued bytes and then exit, not discard queued bytes. Only after the join does it finalize EOF/timestamps and close the file. The nonzero cleanup argument requests path deletion after that sequence; it is not the extraction task's cancellation token. Writer failure makes close fail. The saved native-error field is cleared at close entry and the failure field is reset as close completes, so the active failure path must consume diagnostics rather than expect them to remain after close.

Close retains reusable queue/events. Final release reaches destructor `0x180022f50`, which calls `Close(0)`, frees staging and ring storage, closes all three events, and deletes the critical section. The events are data/drain wakeup (`+0x80`), drain completion (`+0x88`) and producer-progress wakeup (`+0x90`).

No task-cancellation callback is consulted inside this queue. Upstream cancellation stops producing; this object then performs drain/join/optional deletion. This explains why decode completion, accepted output bytes and completed native output are separate milestones. It does not justify copying an indefinite join or treating a drain signal alone as durable success in NxTools.

## Implications for NxTools

1. Keep one validated source/catalog and one ordered selection plan per attempt. Do not repeat archive-sized admission work for each member or consumer.
2. Preserve dependency-aware decoding. ZIP member seeking, RAR solid predecessor processing and7z block-prefix decoding are different plans; one generic independent-file scheduler would be wrong.
3. Create outputs only for selected items, while retaining required predecessor decoding, per-member integrity, cancellation and format-level terminal validation.
4. Reuse binary working storage and decoder allocations without confusing capacity reuse with valid decode-history continuation.
5. Measure source/admission, decode, transport, output drain and publication separately. The current synchronous JSON/native byte transfer is an additional boundary; larger buffers or more threads are not substitutes for removing redundant work.
6. Keep Rust domain/scheduling ownership, C# native output/publication and GPUI rendering. The useful reference is the data path and completion semantics, not proprietary codec implementation reuse.

## Evidence map

Complete canonical JSON, target/provider metadata and query parameters are retained under [the fixed local evidence root](evidence/README.md). The [index](evidence/index.json) maps these IDs to files.

| Boundary | Canonical record under `evidence/raw/` | Evidence ID |
| --- | --- | --- |
| RAR4 solid admission | `ark-rar-asm-0x18002f410.json` | `ev_d3fe29471139e33641298355311afcf02ac6d4c209f88f3c2947d01b8da4e4d3` |
| RAR5 solid admission | `ark-rar-asm-0x180032730.json` | `ev_94505971e9903b889dccc05f8bef0308ce2b1648c826ec68a97083a5913e09e8` |
| Shared solid dispatch | `ark-rar-asm-0x180059a90.json` | `ev_53ea22db623e11fa02c513ba189555f37ba41b0042b5e0bc75bd94219b127941` |
| RAR selected range | `ark-rar-code-0x180030ad0.json` | `ev_81c34a6813833205f2ae8174f62e87bda3d039e59ccd561e16a29bf849bdc3bd` |
| RAR5 selected range | `ark-rar-code-0x180034920.json` | `ev_7d18cf2562f06a8bc1621088b217b0f39b467f6108742cf45da40345f3cf4465` |
| RAR dictionary capacity/history | `ark-rar-code-0x1800caab0.json` | `ev_3aadd84c2b3e5409813a7a08e3c31d9c78f3c561340d26626abacbc3df4eb45a` |
| 7z constructor/interfaces | `ark-7z-dossier-0x18004c2f0.json` | `ev_f63274bb6003adcf355b673ac2dae37d835d79b053bfbff6a7a66d951f7aaf2f` |
| 7z selected adapter | `ark-7z-code-0x18004f0b0.json` | `ev_2808e97a610a8726fb6fceaadf5ec71c0567aed908db92e84d8861eef8f5270b` |
| 7z folder grouping | `ark-7z-code-0x180139760.json` | `ev_816d5bc33a5c1fa18a7435cc9f7ec119544b60cdd52d633c9ba3f808393a5597` |
| 7z block decode/setup | `ark-7z-code-0x1801611c0.json` | `ev_e0ee6858a8b257a5d0d687c0b3bfe5446cd651bf3c513d6b7e3e7dd8e948369c` |
| 7z file write/CRC | `ark-7z-code-0x180139580.json` | `ev_a72bc8a9347f81448c62a99101c92d94e447e3cfbec00c6ddcef43f9c9e89281` |
| 7z selected-only output factory | `ark-7z-code-0x18004e350.json` | `ev_74d412a5434209b6e12acdc864f9dc93502ded90f2938265b74a07809e678161` |
| 7z member result/native drain | `ark-7z-code-0x18004e940.json` | `ev_48560cbf36b65851db43696e2ecd2d32ce8904c4cc98bf237950f5a92d314bea` |
| MT coder reset | `ark-7z-code-0x180171f80.json` | `ev_487e40e85e62b789c27113a9256804ad0ac1cd741c5ee7f40175a82704714e6c` |
| MT worker join/result | `ark-7z-code-0x180172650.json` | `ev_1cbbd2070198e711919222eaed3eec8cf7797af417a0572ffd2a51c4a77d38fc` |
| LZMA new decode state | `ark-7z-code-0x1800ed8b0.json` | `ev_25014b5044d675b2c5d58e9102f013ca8cafae9d7d0005f3c1ae4dce96f738ed` |
| LZMA2 reset connection | `ark-7z-code-0x18013b860.json` | `ev_d193920f27ada7d2fe805b17e16ddd1dfb42926c0d2bb8d6941ea8eda2eb3ef0` |
| LZMA2 allocation/state reset | `ark-7z-code-0x18013b770.json` | `ev_d42b7d46116fd6d059290f950f6b4cbd5f7638f01c26abfc1f9aca164c30062a` |
| Buffered worker creation | `ark-output-consumer-open-helper-0x1800230f0.json` | `ev_8b329bdb8a509513f9c2105e472eac793f66d97a5f367f215754232986b21459` |
| Buffered consumer/native write | `ark-output-consumer-thread-0x180023950.json` | `ev_b9654a78d0d00e5fbc8f5202b471763135cfc9f7e6d00484d173352eaa115a05` |
| Buffered drain | `ark-output-consumer-drain-0x180023580.json` | `ev_3d39b2658110fc28908db88db1dd298f31614d096baa61eda1a674e721e4b24d` |
| Buffered close/cleanup | `ark-output-consumer-close-0x180023620.json` | `ev_deef005e936c46e7ca136320149235e4092e76d10ae07fd684004097f96bd1c2` |
| Buffered destructor/resources | `ark-output-consumer-destroy-0x180022f50.json` | `ev_44818496bbd383a98590b42e0756364504c47bc16424aa80533cd4b298c8c8ee` |

Additional format admission, vtable bytes, stored/compressed helpers, CRC finalization and callback records remain indexed alongside these anchors. Independent `ark-rar-session-snapshot.json`, `ark-7z-session-snapshot.json` and `ark-output-consumer-session-snapshot.json` preserve their respective query caches; complete Evidence records remain the observations.

## Limitations

- Ghidra12.1.4 reconstructs the inspected7.32.0.1 binaries; these are not original source or runtime proof. Numeric settings and some virtual-interface signatures are only partially named. These traces do not cover every codec, encrypted/recovery branch or split-volume transition.
- Buffered ring-growth increments inside `0x18006f960` were not traced. The observed high-water threshold must not be presented as a hard memory cap.
- RAR/RAR5/7z dependency paths and buffered cancellation were not newly exercised at runtime. No speedup, responsiveness deadline or installed-release acceptance follows from this static trace.
- NxTools' approximately27second selected Debug extraction remains a separate product measurement. Comparable Release timing and native stage attribution remain open. This research changes no product code, validation budget, package, service or protected binary.
