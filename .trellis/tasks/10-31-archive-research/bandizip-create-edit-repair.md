# Bandizip creation, update, verification and repair

## Connected workflow

Bandizip7.32.0.1 separates the creation form, per-output task plan, background worker, compressor and extraction/test interface. An ordinary create and each-input-to-its-own-archive operation share the execution owner but construct different plans. ZIP update imports existing members into that plan and copies their encoded payloads when unchanged. Repair obtains a recoverable ZIP catalog before rebuilding output through the compressor.

These are static Ghidra12.1.4 observations of `Bandizip.exe` and `ark.x64.dll` in `D:\Tool\Bandizip`, not execution of these features. Complete records are in [the fixed evidence archive](evidence/README.md). Proprietary implementation code is not a product reuse candidate.

## Creation form and each-input outputs

`CArchiveDlg`'s command dispatcher `0x1400f3e20` sends Start/OK to `0x1400f6d10`. The submit routine checks an empty source list, output filename, a conflicting directory or existing archive, format-specific size constraints, split-size input, SFX limits and the destination. A missing destination is offered creation; failure, write-permission concerns and an observed very-low-free-space check have separate branches. The free-space check here compares available bytes with1024; it is not evidence of a reliable full-operation capacity forecast.

The single-file gzip/xz-family form branch rejects folders and multiple inputs unless each-item mode is chosen. SFX split configuration rejects a size above4GiB and raises a too-small segment to accommodate its prefix. Unsupported options therefore cannot be inferred from generic controls alone.

Compression levels in the form map selection indices to semantic backend choices `0/1/6/9` in this path. `0x1400f6d10` writes the operation descriptor, choosing operation `0x0b` for ordinary creation or `0x11` for each-input creation, then returns modal result1. The create form does not itself run a codec.

Coordinator `0x14013d910` treats `0x11` as a separate branch. It walks source items, derives per-item output names and extensions, chooses the configured destination/source-relative form, and avoids both existing filesystem names and names already planned in the same batch. Collisions use `%s (%d)%s`, starting at2 with the observed bounded search. Each input obtains its own output/task record. The ordinary `0x0b` branch combines sources into one output task. The coordinator constructs `CArchivingDlg`, supplies those records and opens the actual progress surface through `0x14013f800` (owner-modal template `0xa0`) or the corresponding modeless route.

`0x140100870` performs per-task completion, advances the next task pointer, resets per-task timing/status, prepares source members, binds the worker and starts it. It does not launch an independent compressor for every input simultaneously. Source traversal in `0x140101890` distinguishes files, directories and pre-described items, applies exclusion matching through `PathMatchSpecW`, and calls the compressor-add interfaces. Per-input archive creation is separate from [multi-archive extraction and volume filtering](bandizip-operation-lifecycle.md).

## Compressor plan and format dispatch

`CreateArkCompressor` at `0x1800017c0` validates the interface version and creates `CArkCompr` through `0x18001d1f0`. Constructor assembly places the primary interface at vtable `0x1801f1408`; same-named secondary interfaces at nearby addresses are not interchangeable.

Settings `0x18001fd80` copy format/method/options into compressor-owned state, normalize incompatible combinations and copy supplied auxiliary data. No user secret was supplied or read during analysis. Add-member `0x18001e180 → 0x18001e1a0 → 0x18001e1d0` prepares source metadata, normalized archive paths, a member record and a name-index structure. Payload encoding is deferred to the separate execute operation.

Primary virtual `+0x60 → 0x18001ee20` validates output state, constructs output naming/segment state, opens the first output through `0x18001f510`, prepares comment encoding, checks the accepted format/method combination and dispatches:

| Format ID | Writer entry | Connected evidence |
| --- | --- | --- |
| `0x00` ZIP | `0x180065a70` | Member/header loop, new encode versus old encoded copy, directory/end records and header updates |
| `0x05` LZH | `0x180068f80` | Execute dispatch binding |
| `0x0a` 7z | `0x180067de0` | Source ordering under configured compression/solid conditions, streamed data writer, metadata/header finalization |
| `0x0d` TAR | `0x1800642d0` | Member loop, source read/record write, two512-byte zero end blocks |
| `0x10` ISO | `0x18006ce10` | Execute dispatch binding |
| `0x15` gzip | `0x18006e760` | Execute dispatch binding |
| `0x1b` XZ | `0x18006ec80` | Execute dispatch binding |
| `0x22` compressed TAR | `0x180063eb0` | Execute dispatch binding; does not establish every outer codec setting |
| `0x2e` ZPAQ | `0x18006f090` | Execute dispatch binding |

The gate reports unsupported combinations instead of assuming every recognized reader is writable. Writer dispatch is not runtime certification of every compression, encryption or volume setting. The separate [reader-family map](bandizip-format-families.md) records recognition/read capability.

### ZIP and reusable encoded data

ZIP writer `0x180065a70` walks planned records, skips deletion-marked records, constructs local headers, writes through the shared binary output route, and distinguishes new data from imported old members. New content enters `0x180091070`; imported data enters `0x180090e20`. The latter locates/caches the original local data offset if needed, seeks the retained old input, and copies the recorded compressed extent in blocks up to16KiB. It preserves the member's recorded CRC and compressed size rather than decoding/re-encoding the payload.

After members, `0x1800666b0` emits the central-directory/end material. The writer then updates applicable local headers/ZIP64 fields and handles output errors/cancellation. Header rewrite and metadata work remain even when unchanged payloads can copy. This establishes why an update can be cheaper than extract-and-recompress; it does not certify byte-for-byte retention of every unusual ZIP metadata record.

7z writer `0x180067de0` has a different route: optional source ordering, setup `0x180020080`, data writer `0x1800952a0`, metadata/header finalization `0x180096790`, then output header update `0x180020770`. Reusing ZIP's encoded-copy route is not evidence of cheap solid7z updating.

### Split and SFX output

`0x18001f510` creates/advances native output objects and segment names. The observed7z segment naming uses `.%03d`; other applicable segment paths use `%c%02d`. `0x180020470` consumes remaining segment capacity, writes only the fitting portion and opens the next segment when needed. This is format-aware output routing during construction, not arbitrary chopping of a finished ordinary ZIP.

With an SFX prefix configured, `0x18001f510` requires the ZIP writer branch, opens the prefix read-only, copies it to the new output in4-KiB pieces, appends the `BANDIZIPSFX` marker and checks the applicable size boundary. Native open/write errors return through compressor state. SFX support is distinct from permission to update an existing SFX archive.

## Existing ZIP modification

Browser admission `0x140154360` requires a suitable opened ZIP and separately refuses multi-volume, SFX and broken state, showing the corresponding typed explanation. This is actual control flow, not only language strings.

Compressor import `0x18001dd30` obtains `CArkUnZip`, walks its retained member table and constructs records containing original indices, names, sizes, CRC/method/encryption-related metadata and offsets. Imported members carry an old-data marker at `+0x140` and original index at `+0x144`.

Rename `0x180020cb0` validates the original-index relationship, changes the planned name and clears its encoded-name cache. Delete `0x180020da0` validates the index, sets deletion marker `+0x148` and adjusts planned totals. No payload is rewritten by those setters alone. The execute pass skips deleted records, updates renamed properties and copies retained encoded data through `0x180090e20`; replacements/new data take the encoding path.

[Browser replacement](bandizip-nested-writeback.md) connects this output to temporary-open/configured-test checks, original-to-backup and temporary-to-original `MoveFileW` calls, reopen/focus restoration and separate backup cleanup. The second-move failure attempts restoration without checking its return; this is not an atomic swap or a verified rollback guarantee. NxTools retains its native source/output identity, finish/verification and publication owners.

## Test after creation and optional source deletion

The source-delete option is not silently equivalent to mandatory testing. Form handler `0x1400f9a40` handles the delete-source checkbox. Under its prompting condition, accepting `TEXT_ASK_DELSRC_TURN_ON_TEST_ARCHIVE` checks the test control and sets the local test flag. The Help branch unchecks source deletion and opens documentation. Other choices can leave deletion selected without enabling test. This is a configurable warning, not an invariant that every delete-source operation is tested.

`CWorker` setup `0x1401b1ec0` binds the compressor, descriptor and output path. `_beginthreadex` starts `0x140087900 → 0x140087940`. Worker init `0x1401b2100` chooses operation type2, calls `0x1401b2450`, stores its result at worker `+0x10c` and posts completion to the window.

`0x1401b2450` calls compressor virtual `+0x60` to write the archive. Only after a successful write, with test configured and a reader available, does it post test-start, reopen the generated output, attach its event callback, call the archive test interface, detach the callback, close the reader and post test-end. Test return replaces the operation result; reopening failures are sent through the error callback. Progress handler `0x140100ea0` switches to the testing presentation and resets phase timing. Encoding progress alone therefore is not the complete configured operation.

Per-task completion `0x140102f50` requires source-delete enabled, no per-task error flag (`dialog+0x304==0`) and successful worker result (`dialog+0x41c==1`) before collecting eligible sources. It keeps source entries already marked unsuitable out of that list. `0x140103170` processes the list through `0x14002e820 → SHFileOperationW` with delete operation3, then clears it. These are Shell path-list operations, not a held-original-object deletion guarantee. The caller does not consume the final helper return in this inspected body; successful archive creation and successful deletion remain different facts.

Final progress handling `0x1401013d0` stops its timer, releases task state, disables active controls, exposes Close and chooses success/error/retained-window behavior from accumulated flags and settings. A completed encode must not be substituted for successful test, deletion or publication in NxTools diagnostics.

## ZIP repair

`CArkRepair` is constructed at `0x18001be40`. Its open `0x1800610a0` uses the retained Windows input and saves the separately supplied output name. Execute `0x180061110` constructs `CArkZipRepair`, binds the input through `0x180057cd0` and rejects a missing/unsuccessful recovered catalog before invoking a fresh compressor. It imports the recoverable member records through `0x18001dd30` and rebuilds the configured output via `0x18001ee20`.

`CArkZipRepair` overrides ZIP catalog loading at `0x18008de70`. Scanner `0x18008dcc0` searches `PK\x03\x04` in source order, checks candidate header fields and advances one byte after a rejected candidate, with periodic cancellation/progress checks. The catalog path parses candidates, handles incomplete header/data-descriptor cases through `0x18008e330`, and for eligible Deflate members uses `0x18008e760` to determine consumed compressed bytes, produced size and CRC from the decode stream. The latter uses4-KiB input/output work buffers and observes cancellation/error state.

Repair is therefore recoverable-record scanning plus output reconstruction, not a promise to reconstruct arbitrary lost payloads or an alias for normal integrity testing. The repaired output still needs independent verification before any product can claim recovery; no damaged archive was created or repaired at runtime here.

## Implications

- Plan once, then execute through the operation's original worker and result owner. Separate form validation, source traversal, encoding, testing and final output outcome.
- Expose operation capabilities by format and archive state. Reader availability, writer availability and existing-archive update are different contracts.
- Reuse unchanged encoded ZIP members where supported, while preserving path/ordinal/source authority and metadata fidelity. Do not replace safe updates with whole-tree extraction.
- Treat each-item compression and multi-archive extraction as different plan shapes. Both can use one progress owner without becoming independent concurrent jobs.
- Include split/SFX/header finalization in actual output completion. Segment naming and callback progress are not proof that all volumes finished correctly.
- Keep destructive source actions behind NxTools' existing native identity and verification requirements. The reference's optional test warning and Shell path deletion are not sufficient authority to weaken them.

## Evidence map

Complete files below are under `evidence/raw/`; the [index](evidence/index.json) retains target/provider identity and query parameters.

| Boundary | Canonical file | Evidence ID |
| --- | --- | --- |
| Creation submit | `bandizip-create-p-0x1400f6d10.json` | `ev_29c9b5824565a99417cfa96db74ea39211e224d3927e29afba3a12c1f679aa27` |
| Each-input plan | `bandizip-create-p-0x14013d910.json` | `ev_8d959a85156d95878d749c29b64d963bcb4e3dd6a56d603a777d919c722758e8` |
| Per-task advancement | `bandizip-create-p-0x140100a4a.json` | `ev_01363407c06ae71d228a3fa59298e0488cf1d5a219374d7c00bcf3c930eda334` |
| Compressor factory | `ark-create-p-0x1800017c0.json` | `ev_d07a2ddb7bc6eac9751b7f5e78a6bb68dc53fa71756b48e0a3d3f9d029ebb524` |
| Compressor constructor/interfaces | `ark-create-a-0x18001d1f0.json` | `ev_fdd25cfcdc2bbc1d9bffbcb39fe02708d681f74ab5c6632d9e8a46c710fafa66` |
| Member planning | `ark-create-p-0x18001e1d0.json` | `ev_7ddec4ad910b189815e01bd50f2c8140e7e64548b03c635115129012f8ebfcb3` |
| Execute/format dispatch | `ark-create-p-0x18001ee20.json` | `ev_23013171e9ae52d95818e477b1c8aff773bb765dda0087f6edf494567e831c98` |
| ZIP write/finalization | `ark-create-p-0x180065a70.json` | `ev_0e43e8e7ec7379c6f9e3e14e6b15a51a06f95bdc16a98f82e03d767fd1e96bad` |
| Encoded-member copy | `ark-create-p-0x180090e20.json` | `ev_bc6faf3d0e134d7af1390f00fe0402750b3f3efe7fca248fc6fade4652087b31` |
| Existing-member import | `ark-create-p-0x18001dd30.json` | `ev_ce6183a9fb4bee856482872bb2580eaf236ab501f5b3b5c9e148a31fb9d68479` |
| Segment/SFX output | `ark-create-p-0x18001f510.json` | `ev_cbc8031d964e1b730a08e4af6e8890285ac650cbee48783a802ea2c2f545e7dc` |
| Split write boundary | `ark-create-p-0x180020470.json` | `ev_d20f2c11fb81ac05c4162fd681dadbf2f3599385ed436a7ccd4f020884fd3eb7` |
| Browser modification admission | `bandizip-create-p-0x1401543d1.json` | `ev_d833706692151b02c27563522d5af9e7f94fa62b39c40860feda70599e8cc880` |
| Test-option warning | `bandizip-create-p-0x1400f9c3a.json` | `ev_3db7743388b478663d74e4ec9339e3364917c1e56ba251026bfc77096db5307c` |
| Write then optional test | `bandizip-create-p-0x1401b2450.json` | `ev_15b7ccd6470c8f863223b67245616e25e6a97e4b0058dcb7460e39d4670eb9ce` |
| Delete-source admission | `bandizip-create-p-0x140102f50.json` | `ev_48bda2360865f5fa81d9b49138108d6488b6235120c5a80048d17d5232b6c85c` |
| Source deletion list | `bandizip-create-p-0x140103170.json` | `ev_000681f5f3a382c98001c2c653bc1c37c668e3ab7f9e3c1316f677d51e04a974` |
| Shell deletion | `bandizip-create-p-0x14002e820.json` | `ev_f6ef42c80f087506944d2d7f61f0635453d54f9966385fa11ba98e45a1b3b2b2` |
| Repair reconstruction | `ark-create-p-0x180061110.json` | `ev_5d6f80919875bd28314cf60a1e94d675b3d38fae35828d681a719a13c0f8accb` |
| Repair catalog | `ark-create-p-0x18008de70.json` | `ev_9ffc53f939010410cde6d6e03c42d3fb816c711ac8aec0a280c5d0b08aba80ed` |
| Recoverable header scan | `ark-create-p-0x18008dcc0.json` | `ev_c5696dc0507788c87cb53709ebb31a56252f4816d1f83b0559a4d59418276bbc` |
| Deflate extent/CRC recovery | `ark-create-p-0x18008e760.json` | `ev_2952607e253978ca0d6707d35968c270614fdca438aaa19e5591c2addec949c4` |

## Limitations

No create/update/repair, source deletion, Shell action or new codec benchmark was executed. The paths do not certify every format option, unusual metadata case, encrypted member or volume transition. The connected browser replacement is a path-based two-move operation; crash recovery and exact application-wide concurrency outside these owners remain unverified. Source-copy/update efficiency is a mechanism finding, not a measured NxTools speedup. Product logic, settings, services, installed binaries and release artifacts are unchanged.
