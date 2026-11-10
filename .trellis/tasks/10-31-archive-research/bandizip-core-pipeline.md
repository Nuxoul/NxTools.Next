# Bandizip shared ZIP extraction pipeline

## Core behavior

The inspected7.32.0.1 `ark.x64.dll` path retains an input-stream object and a format decoder, loads ZIP member metadata into that decoder, then processes selected indices against the retained table. Its ordinary ZIP file path is:

```text
source stream + format decoder
    -> bounded end-record lookup -> central-directory member table
    -> selected member indices
    -> cached/validated local-header data offset
    -> Stored copy or Deflate stream -> incremental CRC -> output object
    -> decoded-size check -> metadata/close/error cleanup
```

### Source and catalog reuse

`CArkMan` open (`0x180006460`) creates the input through`0x180017c30` and constructs the ZIP decoder through`0x18000eb80` →`0x180023d70`. `CArkUnZip` stores the input interfaces in its own state through`0x180057cd0`; the manager releases its transient reference after passing it into the decoder. Selected extraction later uses the same decoder object and its member-pointer table.

The Windows input object is `CArkWinIn`, with a retained native file handle and an initial64-KiB buffer capacity. `0x18000e550` handles seek operations inside the current buffer by changing buffer/logical positions; only seeks outside it reach `SetFilePointerEx`. `0x18000dac0` opens a read-only native handle, normally with a sequential-scan hint, and contains sharing-error fallbacks.

`0x1800570c0` searches backward for EOCD within the final`min(file_size,0xffff)` bytes in overlapping roughly1-KiB windows. `0x180024120` uses the resulting directory location; `0x1800244c0` walks central-directory records and handles ZIP64/end records. Metadata remains in the member table. Member admission`0x1800576f0` reads the signature and fixed local header only when the cached data offset is absent; later decoding seeks to the member data offset.

This is an indexed, stateful pipeline, not an instruction to decode all payloads merely to list an ordinary valid archive.

### Per-member work and integrity

`0x180059a90` walks selected indices against the loaded table, prepares each output object, and calls per-member handling`0x18005a120` →`0x18005a4c0`. Compression-method dispatch`0x180071330` selects the codec; method8 enters`0x18007db60` →`0x18007dca0`.

The Deflate wrapper initializes a raw Deflate stream. Its data loop has16-KiB input/output working blocks, with an initial input request capped at4KiB and subsequent requests capped at16KiB. It consumes encoded input, inflates into the reusable output buffer, updates a32-bit CRC accumulator over produced bytes, then calls the output object's write method with those bytes. The final accumulator is compared with the member record under the observed CRC-check conditions. Decode errors, input/output errors and cancellation are checked along the loop.

Stored copy (`0x18007e180`) uses up to8-KiB data blocks, updating the same checksum family before passing each block to output. Generic handling compares output size with member size and performs output metadata/close handling (`0x18005a4c0`, `0x18005a6a0`).

The inspected loop validates the current member through its header, decoded stream, checksum and output result. It does not contain a whole-source SHA-256 revision call for every member. This is a bounded statement about the inspected path, not a claim that every library operation has no hash or authentication.

### Output is direct or conditionally queued

The output factory (`0x18001aec0` →`0x18001a9c0`) selects `CArkWinOut` or `CArkWinOutBuf`. The buffered path requires configuration/type eligibility and size thresholds: Stored files below100MiB and other-method files below20MiB take the direct branch at the inspected size check. The supplied8,324,412-byte JAR is below the latter threshold; its speed cannot be explained by assuming the large-file buffered branch is always used.

`CArkWinOut` write (`0x180060920`) calls native helper`0x180022db0`, which writes binary bytes with `WriteFile` and rejects a failed or short write. The helper limits a single system request to16MiB; the actual codec blocks are smaller. Under numeric output mode2, crossing a10-MiB position interval calls `FlushFileBuffers`. This is conditional milestone flushing, not unconditional flushing after every Deflate block. Close (`0x180060a10`) handles pending data, output EOF adjustment when required, timestamps and handle close; its requested error-cleanup branch deletes the output path.

`CArkWinOutBuf` starts with64KiB of ring storage and a separate32-KiB consumer staging block. Producer `0x180023340` enqueues under a critical section and applies backpressure above a128-KiB–4-MiB high-water threshold; the enqueue happens before the threshold test, so this is not a hard allocation ceiling. Oversized input blocks first drain, then use direct write.

Byte-producing open `0x1800230f0` starts consumer `0x180023950`. The consumer normally coalesces output into32-KiB native writes, signals producer progress after dequeue, releases the lock before `0x180022db0 → WriteFile`, and records native write failure. Drain `0x180023580` waits for queue completion/error, not an OS durability flush. Close `0x180023620` requests stop, waits for queued data and worker exit, then finalizes/closes the native file; requested cleanup deletes afterward. The queue does not itself consult the extraction task's cancellation callback.

The [solid-format and output lifecycle trace](bandizip-solid-output.md) connects the complete consumer/drain/close/resource path and the separate RAR/RAR5 and7z dependency-aware extraction paths.

### Parallelism is conditional

The selected-extraction function chooses a parallel branch only when configuration and metadata conditions permit. For ordinary ZIP and fewer than two selected members, the inspected condition chooses the sequential loop. `0x180023fe0` derives a capability flag by inspecting loaded member metadata after directory parsing. It does not reread encoded data in that metadata loop.

This supports batch parallelism as a conditional optimization, not an explanation that a single normal Deflate member is fast because it uses many extraction threads.

## Implications for NxTools

1. **Reuse validated source/catalog state within an attempt.** Our measured shared open path makes four whole-source revision passes, including automatic edit-eligibility checks. Capability discovery and extraction preparation should reuse the already acquired source/snapshot rather than independently repeat archive-sized work.
2. **Keep checks at the right granularity.** Preserve native source authority and entry identity, structural checks, payload CRC, actual output publication and cleanup. Avoid putting archive-sized verification inside the member loop when the existing held-source lease already proves stability for that attempt.
3. **Treat decode, checksum and output as one streamed data path.** Bandizip's inspected path sends binary buffers through in-process calls. Our synchronous64-KiB decimal-JSON byte-array/native-reply path is an additional boundary to measure. Its cost must be separated from decode and source revision before changing the private transport.
4. **Do not assume larger buffers or more threads solve this case.** The reference uses16-KiB Deflate working blocks and the supplied single small file takes the ordinary branch. The measured evidence favors first eliminating repeated work and avoidable data conversion/round trips.

The efficient model is one retained source, one loaded catalog/selection plan, per-member streaming integrity, and appropriately buffered output. It does not require replacing the original Rust/C#/GPUI ownership or weakening its native result guarantees.

## Evidence map

Complete records and the reusable snapshot are in [the fixed local archive](evidence/README.md); [index.json](evidence/index.json) maps file names, target/provider identities and exact Evidence IDs.

| Boundary | Functions / record | Evidence ID |
| --- | --- | --- |
| Buffered source seek |`ark-core-code-0x18000e550.json` |`ev_a7d2684a2145ef63ce6ed6f77e737f1cc7efb48785ac28aa5ea48c6b241aa619` |
| Input ownership in decoder |`ark-core-code-0x180057cd0.json` |`ev_fd55001816cb236fb23fb0eb489193bdfc4105d1933af88a4f7c878c9fd3e063` |
| EOCD lookup |`ark-core-code-0x1800570c0.json` |`ev_d8ded21f65c4f2b0920e523c7699e0b286a7053c959bd965b6663e3b42ebc507` |
| Central-directory walk |`ark-core-code-0x1800244c0.json` |`ev_9c0e556cb6fdf86cc83be778c95d43ab4b50419ea3a2c090b7be4bb3eb1f706e` |
| Cached/local-header admission |`ark-core-code-0x1800576f0.json` |`ev_ec494a033b7212d8f98ab3d60b684f4339ba1bc4d26b6c63a5de492de1ca5c2d` |
| Selection / conditional parallel path |`ark-core-code-0x180059a90.json` |`ev_0ed3e4fab106bcb8db718e0dcc39844d0ad1582df471980676529f86fd9e79c3` |
| Deflate stream / CRC / output handoff |`ark-core-code-0x18007dca0.json` |`ev_fdbd355dc0e373d46d5f033f93a75d6d334bf1f2f205e2384af09eafd3cf8009` |
| Output factory |`ark-core-function-0x18001a9c0.json` |`ev_21526f4966fba904f760b8202954252da02f74a267221e2017d39526468f6e07` |
| Direct write / milestone flush |`ark-core-code-0x180060920.json` |`ev_c824ab545ed3990ecd503b41a517c97dec2778749bda2faf9392e1fe90663095` |
| Buffered-output backpressure |`ark-core-code-0x180023340.json` |`ev_846243ed27632088d4386bbb979d096c60c4095275930432bd057e8f7438d2e8` |
| Actual system write |`ark-core-code-0x180022db0.json` |`ev_21f641c0746be4065f3db721dbf81db221942734ffb6c6a9c4214e6be7651ff4` |
| Output close / metadata |`ark-core-code-0x180060a10.json` |`ev_6a65cd62820ad25a5643c36a76470513b005f409ba31acdac8019aef547c678d` |

Additional manager, constructor, method-table, codec-dispatch, Stored-copy and metadata records are retained under`raw/ark-core-*`. Public MCP query recipes and the local analysis driver are retained under`raw/recipes/` and`raw/tools/`.

## Limitations

- These are static Ghidra reconstructions, not original source or new runtime observations. Numeric configuration fields and some callback signatures remain partially named. The consumer lifecycle and solid-format paths are recorded in the linked follow-up.
- The ZIP path contains recovery, ZIP64, encryption and CRC-check exceptions. The bounded tail search is an observed implementation choice, not a ZIP specification limit to copy blindly. Other codecs and untraced encrypted/recovery or split-volume transitions are outside the connected paths.
- Source sharing, conditional CRC behavior and direct output handling do not replace NxTools' required identity, CRC and native publication/cleanup contracts. The useful adaptation is reduced redundant work, not weaker guarantees.
- The prior Bandizip runtime and NxTools Debug measurements remain separate evidence. Comparable build modes and native stage durations are still required to quantify an actual product speedup. No product code, budget, service, package or protected binary was changed by this source investigation.
