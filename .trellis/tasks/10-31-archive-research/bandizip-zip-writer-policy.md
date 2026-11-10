# Bandizip ZIP writer policy: ZIP64 and adaptive Deflate

## Boundary

The inspected subjects are `D:\Tool\Bandizip\Bandizip.exe` and the separate x64 `D:\Tool\Bandizip\ark.x64.dll`, version7.32.0.1. Ghidra12.1.4 pseudocode and assembly establish static shipped-artifact behavior, not original source, measured performance or runtime acceptance. Bandizip EULA§3 and applicable-law limits remain in [the reference note](../../../docs/reference-code/archive.md); no proprietary implementation reuse is proposed.

## Force ZIP64 reaches the archive footer decision

Settings helper `0x14016cd60` copies `forceZip64` at settings `+0x300` to native options `+0x28`. Compressor setter `0x18001fd80` retains it at compressor `+0x98`. Execute owner `0x18001ee20`, before writer dispatch, sets compressor `+0x130=1` when that option is nonzero. This is an actual policy consumer, not only a persisted setting. [Z1–Z3]

ZIP writer `0x180065a70` can also enable the same archive flag automatically: retained size `+0x540` greater than `0x7fffffff`, planned member count greater than59999, or a split-size estimate exceeding32767 volumes. Those conservative admission thresholds are distinct from the final ZIP field-width checks. [Z4]

Central-directory finalizer `0x1800666b0` counts written records. With archive flag `+0x130` set and at least one written record, it emits ZIP64 end record `0x06064b50` and locator `0x07064b50`, then the ordinary end record `0x06054b50`. Output-callback failure remains a write failure. The option therefore reaches a concrete ZIP64 archive-ending branch even when automatic admission did not set it. [Z6]

**Archive and member flags are different objects.** Member `+0x130` controls local/central ZIP64 extras, large-size placeholders and the later local-header patch. Local-header owner `0x1800672d0` sets that member flag for its source-size, output-offset or volume conditions; the finalizer writes the member's corresponding extra values. The archive-level force request must not be described as an unconditional assignment of ZIP64 extras to every small member. The observed branch also does not certify every imported-member, encrypted, split or empty-output combination. [Z4–Z6]

## The incompressible-file option is adaptive Deflate

`0x14016cd60` maps `doNotCompressUncompressible` from settings `+0x2d8` to options `+0x50`; `0x18001fd80` retains it at compressor `+0xc0`. Selector `0x180092b00` loads that field and writes it to the Win64 fifth-argument stack slot before calling either `0x180093020` or `0x180093de0`. The retained assembly resolves the argument omitted from the selector's reconstructed call expression. [Z1,Z2,D1]

Both routes inspect the first input bytes. Signature `0x46445025` is the literal byte sequence `%PDF`; it disables this adaptive check for that member. This is content-prefix handling, not an extension exclusion or a claim that all PDFs compress well. [D2,D3]

| Route | Activation condition | Connected change |
| --- | --- | --- |
| Single Deflate `0x180093020` | Option enabled, processed input strictly greater than `0xa00000` (10MiB), and floating-point `encodedBytes * 100 / processedInput > 98.5` | Under the valid codec-state branch, clear the level/tuning fields to level0 and clear the adaptive flag. Assembly at `0x18009384b` zeros eight bytes at state `+0xa8`, including level `+0xac`; `0x18009386c` clears the adaptive register. Earlier encoded data is retained. [D2,D4] |
| Parallel Deflate `0x180093de0` | Option enabled, ordered-consumer input strictly greater than `0x1e00000` (30MiB), and integer `encodedBytes * 100 / processedInput`, converted to double, is at least98.5 | Set the level used for subsequent work records to0 and disable further adaptive checks. For nonnegative counts, the integer comparison requires at least99 percent. Already queued/encoded records are not retroactively recompressed. [D3,D5–D10] |

This keeps the underlying compression method Deflate (`8`), rather than changing the member to ZIP Stored (`0`) or copying the whole source as a new Stored member. Encryption's outer method representation remains separate. Normal CRC, source-read, cancellation and output-result paths remain in the existing encoder loop. The option is not a preflight promise that every incompressible input bypasses all compression work. [Z5,D2,D3]

### Parallel counters and level ownership

The producer fills work-record input length at `+0x68`, selected level at `+0x88` and sequence at `+0x80`. It resets the codec and applies that level through `0x180101f70/0x180102150`. Initializer `0x180101ab0` validates levels0–9, defaults `-1` to6 and stores the level at codec state `+0xac`. The parameter setter updates the same field and its tuning table. [D3,D6–D8]

Worker entry `0x1800b4240` invokes `0x1800b4270`. Assembly establishes the result layout: work-record `+0x68` is input length; stream `+0x1c` is encoded length; result-slot `+8` receives encoded length and `+0xc` input length. Ordered consumer `0x1800b9ae0` takes the next sequence and, only after its output callback succeeds, accumulates these lengths at its parent `+0x98/+0xa0`. Relative to the selector's queue object these are `+0xb0/+0xb8`, precisely the numerator and denominator read by the adaptive comparison. The counts describe accepted ordered output, not merely submitted work or independently confirmed durable flush. [D3,D8–D12]

## Evidence locator

Files are relative to `evidence/raw/`. Subjects below are under `D:\Tool\Bandizip`; EXE and DLL address spaces are separate. IDs are the actual canonical record values.

| Ref | Canonical JSON | Evidence ID | Subject |
| --- | --- | --- | --- |
| Z1 | `bandizip-advanced-advform-procedure_pseudo_code-0x14016cd60.json` | `ev_c6006b79ece012951a9ee78391ba16ee9eadd4e7d9da1b7237e1fc756b933324` | `Bandizip.exe` |
| Z2 | `ark-create-p-0x18001fd80.json` | `ev_18ba71c120a3f8473c027e3c25d10901ae261da6a36ea953025b6f4b3a92656c` | `ark.x64.dll` |
| Z3 | `ark-create-p-0x18001ee20.json` | `ev_23013171e9ae52d95818e477b1c8aff773bb765dda0087f6edf494567e831c98` | `ark.x64.dll` |
| Z4 | `ark-advanced-writer-procedure_pseudo_code-0x180065a70.json` | `ev_0e43e8e7ec7379c6f9e3e14e6b15a51a06f95bdc16a98f82e03d767fd1e96bad` | `ark.x64.dll` |
| Z5 | `ark-advanced-effects-procedure_pseudo_code-0x1800672d0.json` | `ev_ebabf169ab11163f68637a24a48bacf0533499ca96259729dc3ef3734f6e8b13` | `ark.x64.dll` |
| Z6 | `ark-advanced-writer-procedure_pseudo_code-0x1800666b0.json` | `ev_9c38eec1d66663d16c6dd9b0a9e13899e8b243e25fc13c8b5f4ef4e5139fc522` | `ark.x64.dll` |
| D1 | `ark-zippolicy-asm-0x180092b00.json` | `ev_d5d1c70ad8acf1c480e0f423d5d038fa18a401a9159939e4ec67f224ba94cab3` | `ark.x64.dll` |
| D2 | `ark-advanced-codecs-procedure_pseudo_code-0x180093020.json` | `ev_8c2ad762821b600125d1fa1a254c81034a0185adf78553cca70aca5001766cba` | `ark.x64.dll` |
| D3 | `ark-advanced-codecs-procedure_pseudo_code-0x180093de0.json` | `ev_f809cdd67e32256f074d2ff481351ba6c1243ac1342831a83587ad06926d7f0d` | `ark.x64.dll` |
| D4 | `ark-zippolicy-asm-0x180093020.json` | `ev_fdc8cde327c73bae39046654191f5456ada0afdf36e8820e255b7feaaa6c1a10` | `ark.x64.dll` |
| D5 | `ark-zippolicy-asm-0x180093de0.json` | `ev_ed17ef1449a1adb5cb3e51f769434a1f8292b8a30b97b130df57b5f44c07752f` | `ark.x64.dll` |
| D6 | `ark-zippolicy-p-0x180101f70.json` | `ev_8810219d29b5f9f11374b249a9f85ca7902946e6769e4a4561f7a939d576b686` | `ark.x64.dll` |
| D7 | `ark-zippolicy-p-0x180102150.json` | `ev_def6fc781fc6a17f4d77f335e12ab01578608b9a89f5eb3a2fa2abfc0d507439` | `ark.x64.dll` |
| D8 | `ark-zippolicy-p-0x180101ab0.json` | `ev_20f70b6f3dcab41a6e355ef0657818b5eaf4f91828cef85ba8206304d2e1c63c` | `ark.x64.dll` |
| D9 | `ark-zippolicy-p-0x1800b3840.json` | `ev_e95fa70e5cdc702f09cd77194d577d27d3c44036c8dceee1e1cc7b1772386aa5` | `ark.x64.dll` |
| D10 | `ark-zippolicy-p-0x1800b9ae0.json` | `ev_bf2d33d5e6e65fc320a81f7320926a04d2ca9354cc7f328ab19ee1330796224c` | `ark.x64.dll` |
| D11 | `ark-zippolicy-asm-0x1800b4270.json` | `ev_70e61607b332bd4d7a34e3385344ae49c28f17adcf73a5beda229d95cc5ea016` | `ark.x64.dll` |
| D12 | `ark-zippolicy-asm-0x1800b9ae0.json` | `ev_a1fd37eec0f53a55367fc6bc4461ec1f903311067bf398f3a535ce7fc60e1a3b` | `ark.x64.dll` |

## Replay and limitations

Private `ark-zippolicy-session-snapshot.json` retains the target-matching analysis cache. `recipes/rea-ark-zippolicy-parameters.json`, `rea-ark-zippolicy-worker.json`, `rea-ark-zippolicy-worker-body.json`, `rea-ark-zippolicy-final-boundaries.json` and `rea-ark-zippolicy-counter-layout.json` retain the exact queries. Canonical individual observations remain readable independently of that cache.

No archive was created, no large-member fixture or compression ratio was measured, and no Bandizip executable was run. Alternate selector `+0xd0`, ZIPX codecs and imported encoded-data reuse are not certified by these two Deflate routes. Encrypted/split/update cross-products, native failures and final ZIP bytes remain runtime acceptance work. No product logic, dependency, installed setting, service, package or binary was changed.
