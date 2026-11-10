# Bandizip compressed-WIM payload reconstruction

## Scope and subject identity

Static reconstruction of the compressed-WIM route used by Bandizip 7.32.0.1. Addresses are virtual addresses in the subject named in each row; they are not interchangeable across the two DLLs.

| Role | Canonical file | File version | SHA-256 |
| --- | --- | --- | --- |
| Ark classifier, adapter and native output owner | `D:\Tool\Bandizip\ark.x64.dll` | `7.32.0.1` | `51840ddda5c8aa15dc9e6bf74094936ef2ef13bf9442d5bfd7162d7333b5303a` |
| Shipped compressed-WIM handler and codecs | `D:\Tool\Bandizip\ark.x64.lgpl.dll` | `7.32.0.1` | `79900c2b4c2ef2ac8720e2fc38f71a21591346e62f0e556eabd442f304c8f0e9` |

The static join is cross-DLL: `ark.x64.dll` selects the route and `ark.x64.lgpl.dll` owns resource/chunk reconstruction. Ghidra output is reconstructed pseudocode, not original source or execution. The shipped `ArkLicense.txt` identifies `7z(Compressed WIM)`. Method names describe the observed decoder routes; they do not establish unobserved branches or authorize proprietary implementation reuse. Bandizip EULA§3/applicable-law limits remain in [the reference note](../../../docs/reference-code/archive.md).

## Joined dispatch

`ark.x64.dll` classifier `0x180005030` treats a complete `MSWIM` header differently by header compression flag: `(flags & 2) == 0` returns format ID `1c`, while `(flags & 2) != 0` returns ID `1d`. ID `1c` constructs native `CArkUnWim`; its catalog `0x180036600` rejects a compressed header and selected `0x180036bd0` only seeks member `+0x78` and calls Stored copy `0x18007e180`. That path remains the uncompressed-WIM path.

ID `1d` constructs `CArk7Zip` with the `E6` class identifier at `0x1801f48e8`. Its registered create callback `0x18004b620` ensures `ark.x64.lgpl.dll` is loaded, obtains the helper export stored as `Create7zArc`, and calls it with `"wim"`. Helper export `Create7zArc` `0x18001d6b0` accepts that four-byte name, allocates `0x170` bytes and constructs `NArchive::NWim::CHandler` at `0x18000d190`. The handler's primary table at `0x18008ad80` supplies `Open` at virtual `+0x18 → 0x18000ab00` and `Extract` at `+0x38 → 0x18000c1c0`, matching the generic main-DLL calls from `0x18004c7a0` and `0x18004e170`.

The native `0x180036bd0 → 0x18007e180` routine is the uncompressed selected-copy path. A compressed WIM with an admitted complete header is routed to the helper handler instead.

## Selected member to resource reconstruction

Helper `Extract` `0x18000c1c0` receives the selected index list, totals the selected resource sizes and creates a local progress object. For an ordinary selected data item it:

1. calls the extraction callback's `GetStream` slot (`+0x28`) for that selected index;
2. resolves the catalog item to a stream/resource index, then to the retained `CResource`, owning volume and volume header;
3. calls `PrepareOperation` (`+0x30`);
4. invokes `0x1800193e0` with the source stream, selected resource, header/database, selected output stream, progress object and optional resource digest;
5. maps decode/integrity status to an operation result and calls `SetOperationResult` (`+0x38`).

`0x1800193e0` wraps the selected output in `COutStreamWithSha1`, initializes SHA-1 state, calls resource unpacker `0x180018d20`, finalizes the digest when the resource carries one, and releases the wrapper. The hash is the WIM resource integrity boundary already present in the format/handler; it is not an added whole-archive verification pass.

### Resource plans

| Resource shape | Observed reconstruction |
| --- | --- |
| Uncompressed, non-solid | `0x180018d20` requires packed size equal to unpacked size, seeks the resource offset, limits the input to that size, runs `NCompress::CCopyCoder`, and rejects a copied total different from the declared unpacked size. |
| Compressed, non-solid | It derives chunk count from unpacked size and header chunk bits. Offset entries are 4 bytes below 4 GiB unpacked size and 8 bytes otherwise. Table size is `(chunk_count - 1) * entry_width`; table size beyond packed size, decreasing offsets, or offsets beyond the payload fail. Each chunk seeks to `resource.Offset + table_size + packed_offset`, reports progress, and calls `0x180018860`. |
| Solid/shared backing stream | A solid resource must resolve a valid `SolidIndex`. The selected resource offset is translated to a solid chunk index and intra-chunk offset. The retained solid descriptor provides method, chunk bits and a `NumChunks + 1` offset array. The unpacker can reuse the already decoded chunk only when both solid index and chunk index match; otherwise it seeks the shared backing resource, decodes that chunk, caches it, and writes only the selected resource slice. |
| Shared/deduplicated resource reference | Catalog items converge on the same resource index and digest. The observed selected path reconstructs that resource into the requesting callback stream; it does not establish cloning a previously published output. This is distinct from the explicit solid-chunk cache above. |

Solid chunk decode permits a chunk-level `S_FALSE` data status to continue far enough for the selected resource SHA-1 to identify the bad member. Other failures propagate immediately. Invalid solid flags/indexes return `E_NOTIMPL` rather than falling back to Stored copy.

## Chunk method dispatch and size boundaries

`0x180018860` first handles a chunk whose packed and requested output sizes are equal as a raw chunk copy. Otherwise the observed method codes are:

| Method code | Observed decoder route | Boundary |
| --- | --- | --- |
| `1` | XPRESS decoder `0x18001d260` | Reconstructs literals/matches directly into the fixed requested output buffer. Malformed tables, input exhaustion, invalid distances or failure to finish at the requested output/end marker return data failure. |
| `2` | `NCompress::NLzx::CDecoder`, decode setup/body `0x180005a20` with WIM E8 translation finalizer `0x1800047a0` | Chunk bits outside `15..21` return `E_NOTIMPL`; the external window is the chunk output buffer and each chunk starts without retained LZX history. |
| `3` | LZMS decoder `0x180002250`, with `0x180002070` terminal state/output step | Allocates retained decoder state plus a `0x40000` workspace; malformed headers/tables or decode inconsistency return data failure. |
| other | no decoder | Returns `0x80004001` (`E_NOTIMPL`). |

The chunk output buffer capacity is `1 << chunk_bits`. If a decoder produces more than requested, the chunk fails. If it produces less, the remainder is zeroed for a bounded buffer state but the function preserves a non-success data status. Non-solid extraction propagates that status; solid extraction still relies on the selected resource digest before reporting success.

These are the three method codes reached in the inspected 7.32.0.1 helper. They are not a claim about every WIM version, third-party extension or future method.

## Output, cancellation, error and cleanup join

The helper writes only through the output stream returned for the selected index. Its per-chunk progress call is wired through the main adapter: `ark.x64.dll` `0x18004ed20` updates progress and returns `0x80004004` when operation error/cancellation state is set. Input seek/read errors, decoder errors and output write errors therefore leave the helper as HRESULT failures rather than successful files.

Helper `Extract` maps the resource result before `SetOperationResult`:

- success plus matching/absent resource digest → operation result `0`;
- `S_FALSE`/data failure → result `2`;
- SHA-1 mismatch → result `3`;
- `E_NOTIMPL` → result `1` (recognized resource, unsupported method/shape);
- other HRESULTs propagate immediately.

The main adapter completes the native boundary:

- selected indices enter `0x18004f0b0 → 0x18004e170`, which invokes handler virtual `+0x38`;
- `0x18004e350` creates a native output only for a selected item;
- `0x18004ee80` forwards helper writes to the Ark output object and converts native write/disk-full failures;
- `0x18004e940` maps helper results (`1 → 0xa0000019`, `2 → 0xa0000035`, `3 → 0xa0000018`), requests failed-output cleanup, applies metadata, finalizes/drains the output and releases it;
- `0x18004e170` releases the output and selection buffer on every terminal path and requests `DeleteFileW` for the failed output state it owns;
- direct or queued native writes then use the previously traced `0x180060920 → 0x180022db0 → WriteFile` boundary, with `0x180060a10` / buffered `0x180023620` final close and requested deletion.

A created path, a decoded chunk and an accepted output write are therefore not terminal success. Handler result, digest/result mapping and native close all remain required.

## Evidence

Canonical JSON names below are relative to `evidence/raw/`.

| Boundary | Canonical evidence | Evidence ID | Subject |
| --- | --- | --- | --- |
| WIM compression-flag classifier | `ark-families-code-0x180005030.json` | `ev_dc74d48363ec0a29fcbf4b6674a3ce05465691c08ded5a243e2951047bc4f571` | `ark.x64.dll` |
| Factory ID `1d` / E6 adapter construction | `ark-families-asm-0x18000eb80.json` | `ev_df616d37448c65f4354e65047d51c194bf0f94409dd0fd3177a5a611f609b85d` | `ark.x64.dll` |
| Main CArk7Zip constructor/class factory | `ark-families-detail-code-0x18004c2f0.json` | `ev_bf3eb37d67a93a060115a0406d68d955ff3b9384da29431bd3b742f23ab2b89e` | `ark.x64.dll` |
| Helper `Create7zArc("wim")` | `ark-wimfollow-lgpl-code-0x18001d6b0.json` | `ev_fb87c2d42ae7fe05317ce965b2e4ee0199a446553b263df95de305b41fd24e86` | `ark.x64.lgpl.dll` |
| WIM handler constructor | `ark-wimfollow-lgpl-code-0x18000d190.json` | `ev_e4b8dffd95f422c783a08ccaa583cb65f8618265d9a6c460a687365040126e20` | `ark.x64.lgpl.dll` |
| WIM handler interface table | `ark-wimfollow-lgpl-bytes-0x18008ad80-104.json` | `ev_c2c23618f462d6e11e854849b67fe334393cd08027cd3a8e9899e1090dcefa5f` | `ark.x64.lgpl.dll` |
| Helper WIM Open/catalog owner | `ark-wimfollow-lgpl-code-0x18000ab00.json` | `ev_24022c6d0e2e4db628f3510127d2a8bd7e3d7a03a542549c2c74d9a537206fdf` | `ark.x64.lgpl.dll` |
| Selected item/resource/result loop | `ark-wimfollow-lgpl-code-0x18000c1c0.json` | `ev_00028fd40f30b2a376f56df3f88acb45903a5331dd876848054fb00bf048bc7d` | `ark.x64.lgpl.dll` |
| Selected SHA-1 output wrapper | `ark-wimfollow-lgpl-code-0x1800193e0.json` | `ev_c613d28c14ce2a4a5bb688b61f9e186a27a36e8481c29594e211db7c4b2da189` | `ark.x64.lgpl.dll` |
| Stored/non-solid/solid resource unpacker | `ark-wimfollow-lgpl-code-0x180018d20.json` | `ev_45b2a3ee263a16b7099d03a1d90a6d38a85b1998b8cb81d666981b87b9511ffa` | `ark.x64.lgpl.dll` |
| Chunk method/size dispatch | `ark-wimfollow-lgpl-code-0x180018860.json` | `ev_8768a645a1b3052240068b0c91482cd1cefa8c997d9a60b246fc52c5f65f32fc` | `ark.x64.lgpl.dll` |
| XPRESS decoder | `ark-wimfollow-lgpl-code-0x18001d260.json` | `ev_b73c49a97436bf905a540acd515bf8190f557681d3f24b03a7dc2c85d33f9efc` | `ark.x64.lgpl.dll` |
| LZX WIM translation finalizer | `ark-wimfollow-lgpl-code-0x1800047a0.json` | `ev_8f629f4c75ba112ee4d4fb2879b46d7fd3dcf888ddde7713cb751034b8051c21` | `ark.x64.lgpl.dll` |
| LZMS decoder body | `ark-wimfollow-lgpl-code-0x180002250.json` | `ev_69d7a7f8a7c7acec9dde78402bfa6950f63acb095b80fbc02ef8a7943e4aedaa` | `ark.x64.lgpl.dll` |
| Main selected adapter/error cleanup | `ark-families-detail-code-0x18004e170.json` | `ev_da3efe9406ef9632d453690fc3e54fb57f4eb4a495b64c6fcbb3e885a3ea01ea` | `ark.x64.dll` |
| Main selected-only output creation | `ark-7z-code-0x18004e350.json` | `ev_74d412a5434209b6e12acdc864f9dc93502ded90f2938265b74a07809e678161` | `ark.x64.dll` |
| Main result/native drain | `ark-7z-code-0x18004e940.json` | `ev_48560cbf36b65851db43696e2ecd2d32ce8904c4cc98bf237950f5a92d314bea` | `ark.x64.dll` |
| Main write failure boundary | `ark-7z-code-0x18004ee80.json` | `ev_469f4d24725f82d92f723b4c1abdbf1d71f3eb0ce0ae02ff69e451e180fe7c55` | `ark.x64.dll` |
| Main cancellation/progress boundary | `ark-7z-code-0x18004ed20.json` | `ev_cddee6984c1caeb046b9f8a62b859005fa03a1d300cf7a84c121a462559fd49b` | `ark.x64.dll` |

`ark-wimfollow-main-static-excerpts.txt` retains main registration/helper-loader instruction excerpts; these are static shipped-byte observations without separate REA Evidence IDs.

## Replay

The helper recipe is `evidence/raw/recipes/rea-ark-wimfollow-lgpl.json`; its closed cache is `evidence/raw/ark-wimfollow-lgpl-session-snapshot.json`. Individual canonical observations remain the evidence independently of the cache.

```powershell
$env:GHIDRA_HEADLESS_MAXMEM = '768M'
$env:REA_SESSION_LABEL = 'ark-wimfollow-lgpl'
python .trellis\tasks\10-31-archive-research\evidence\raw\tools\rea_follow_session.py `
  .trellis\tasks\10-31-archive-research\evidence\raw\recipes\rea-ark-wimfollow-lgpl.json
```

## Limitations

- This is static evidence for the shipped x64 7.32.0.1 pair. It does not prove runtime acceptance of a particular WIM/SWM/ESD archive, installed-edition behavior or GUI behavior.
- `0x18000ab00` is a large Open routine and Ghidra reports bad-instruction control-flow recovery in its pseudocode. The handler identity, retained catalog/resource use and selected reconstruction are joined, but the complete multi-volume Open state machine and the exact constructor of every `Solids[]` entry were not decomposed into separate records.
- The observed selected decoder dispatch covers method codes `1`, `2` and `3`, the raw-chunk bypass, and explicit unsupported fallback. XPRESS header chunk-size admission and every LZMS/LZX format variant were not exhaustively enumerated.
- Shared-resource catalog identity is established; native publication as hard links or alternate streams is outside this payload question. No claim is made that every pair of names sharing a resource is materialized by one filesystem operation.
- No target, GUI, helper, archive, extraction/test action, registry, setting, credential, user log, service, build, package, install or product source was executed or modified.
