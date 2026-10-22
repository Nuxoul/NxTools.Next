# Exact codec capability gates — Phase A implementation checkpoint

## Evidence and approval boundary

User approved A–F first release, with RAR4/5 core in A, real volumes/ZIP edits in E and genuine native drag in F. This note does not reduce scope. Original read-only audit fetched exact sources under required memory variables; later Windows worker builds and real C# Job→Rust codec/output fixtures passed for ZIP Store/Deflate,RAR4/5 and7z as recorded in [checkpoint](../phase-a-checkpoint.md). GPUI stays exactly0.7.1, ZIP lock8.6.0; no arbitrary upgrade.

Source root used for the audit: `$PI_SCRATCH_DIR/archive-audit-cargo-home/registry/src/index.crates.io-1949cf8c6b5b557f/`. Full raw report is `archive-dependency-capability-audit.md` in that scratch root's session parent. Main agent independently re-read the specific hard gaps below. Scratch output is not release evidence.

## Exact candidates (not all adopted)

| Need | Version/features | License / Windows build status |
| --- | --- | --- |
| ZIP | existing caret8.3.0, lock8.6.0; existing Deflate; candidate consumer features aes-crypto,bzip2,deflate64,lzma,xz,zstd | MIT; existing Store/Deflate graph built; new features not built |
| 7z | sevenz-rust2=0.23.0, no defaults, compress/aes256 | Apache-2.0; Windows isolated worker build passed; solid/non-solid/LZMA/AES fixture open/test passed |
| RAR read-only | unrar_sys=0.5.8 bundles UnRAR7.1; public wrapper replaced by narrow codec callback adapter | MIT/Apache wrapper+UnRAR restriction retained; C++14/RAR_SMP/advapi32 Windows build passed; RAR4/5/plain/encrypted fixture open/test passed |
| TAR | tar=0.4.46, no default xattr | MIT/Apache-2.0; not built |
| Gzip/Deflate | flate2=1.1.9, no defaults/zlib-rs | MIT/Apache-2.0; already locked, directly adopted for bounded ZIP payload decoding; Windows build passed |
| CRC | crc32fast=1.5.0 | MIT/Apache-2.0; already locked, directly adopted; Windows build passed |
| Bzip2 | bzip2=0.6.1 default pure-Rust libbz2-rs-sys | MIT/Apache-2.0; not built |
| XZ/LZMA2 | lzma-rust2=0.21.0 through sevenz-rust2 | Apache-2.0; Windows LZMA/LZMA2 worker graph built; standalone XZ adapter not delivered |
| Zstd | zstd=0.13.3, no defaults | MIT; aligns ZIP dependency, avoids unnecessary0.14.1 second line; native build not run |
| XZ fallback only | liblzma=0.4.8 static/parallel if justified | MIT/Apache-2.0; C/bindgen cost, not adopted/built |

## Concrete blockers, not optional omissions

### 7z allocation contract

`sevenz-rust2-0.23.0/src/reader.rs:18` defines `MAX_MEM_LIMIT_KB = usize::MAX / 1024`. Calls at471/1406/1607 pass it into header/data decoding. `src/decoder.rs` supports a limit internally, but public ArchiveReader has no configurable limit constructor. Encoded/encrypted headers are decoded before caller inspection of block properties. A512MiB metadata/dictionary claim cannot be made from public API usage or a thread timeout.
User explicitly approved the recommended bounded-worker boundary after these concrete gaps were presented. Use C#-owned Windows Job Object limits, suspended admission before execution, bounded private channel with no password argv/disk persistence; Rust retains codec/state ownership. Cancellation requests cooperative stop then terminates only the owned worker after bounded grace; actual cleanup precedes slot release. This is not permission to patch third-party source or reduce required RAR/7z scope.

### RAR allocation and cancellation

`unrar-0.5.8/src/open_archive.rs:503-534` hardcodes UCM_PROCESSDATA to return0, and public wrapper exposes no cancellation callback. Native UnRAR documentation `unrar_sys-0.5.8/vendor/Documentation/RARCallback.md` allows returning-1 to cancel. Public wrapper also has no allocation/dictionary cap; OOM error after allocation failure is not a preventive budget. RAR4/5 and volumes/typed error codes exist, but this does not satisfy Phase A cancellation/resource safety. Same bounded-worker decision covers RAR without silently patching dependencies. Use of lower-level UnRAR callback needs a reviewed Rust codec boundary, not codec business logic in C#.

UnRAR license paragraph2 permits handling, forbids RAR-compatible compressor/recreation and requires full notice (retained). Bundled C++ Windows build and narrow advapi32 link passed. Callback actual-byte/cancellation plus actual native output fixtures passed for small RAR4/5/encrypted samples; dictionary/RSS and comprehensive method/security evidence remain open.

### Genuine ZIP volumes (Phase E)

`zip-8.6.0/src/read/zip_archive.rs:190-193` rejects differing disk/central-directory disk. `src/write.rs:1917-1955` emits single-disk fields. No verified drop-in volume backend was found in this bounded audit. Required `.z01 + .zip` read/create needs a project ZIP-spec multi-disk adapter or separately verified backend. No arbitrary zip upgrade and no byte-chopped normal ZIP. This is not a reason to skip required E.

### 7z volumes (Phase E)

Reader and writer are generic global Read+Seek/Write+Seek. A project segmented global-address-space reader/writer is feasible; finish rewrites signature at offset0 (`writer.rs:390-419`). Upstream has no discovery/naming/missing-part API. This work remains E, not completed capability.

## Public source APIs verified

- ZIP typed errors and AES read/write; raw_copy_file[_rename]/merge_archive permit transactional compressed/encrypted-byte preservation. Read config has no metadata allocation limit. IndexMap collapses duplicate names; project enumerates original central records.
- sevenz solid `push_archive_entries`, nonsolid `push_archive_entry`, `set_encrypt_header`, public block/coder/file stream mapping and typed PasswordRequired/MaybeBadPassword/MaxMemLimited. No public cancellation callback.
- RAR public code+when error typing, listing_split/VolumeInfo/split flags. No RAR creation.
- TAR entries/path_bytes/link types and builder follow_symlinks(false); never unrestricted unpack.
- lzma-rust2 XzReader/LZMA2 new_mem_limit can reject dictionary allocation; Zstd window_log_max bounds its back-reference window; cancellation must wrap streaming I/O.

Reference projects actually inspected: Archiver core/src/lib.rs and core/Cargo.toml; SupaZip's actual nested path is `D:\Code\supazip-master\supazip\supazip-core` (traits.rs, error.rs, formats). Their whole-buffer/string-matched error/extract overwrite paths are not copied. No proprietary Bandizip code/DLL used.

## Authorized next work

Continue Phase A under approved bounded-worker decision; no exit. RAR4/5 and7z open/test/native-output fixtures pass including password/budget/corruption/target protection. ZIP extra methods/encryption, other formats/create/source traversal, product IPC/backpressure, diagnostics/native attrs/MOTW/recycle and full safety/performance remain required. Atomic overwrite retains native32 red; [evidence/unapproved amendment](native-overwrite-gate.md). All A–F cases remain required.

## Acceptance remediation (later evidence, no upstream patch)

The public allocator gap above remains an upstream fact. Current project enforcement adds
schema2 source lease and coherent reservations; process cap=work+metadata+64MiB overhead
(default832MiB), max1GiB. Explicit MemoryBytes must equal this value; sub64MiB work requests
are rejected as unsupported reservation granularity, not silently rounded. Rust preflights
public coder properties before data decode and encoded-header descriptors before header decode;
LZMA costs dictionary/literal tables plus state, LZMA2 dictionary plus conservative4MiB state,
other supported fixed-state filters conservative1MiB each, summed per block at one thread.
Metadata preflight reserves16×raw/declared decoded header bytes for parser buffers/state; this
is conservative admission accounting, not measured peak RSS. Final process-commit Job bounds
all allocations, and an allocation/process failure is not reported as codec success.

Exact locked sources additionally inspected: sevenz-rust2/src/{reader,block,decoder}.rs
Archive::read/Block::coders/Coder::properties and encoded-header construction;
lzma-rust2/src/{lzma2_reader,decoder,lib}.rs dictionary/range/literal tables;
UnRAR7.1 vendor/unrar/{dll,file}.cpp RAROpenArchiveEx→FMF_OPENSHARED→CreateFile read/write
sharing. Project uses these public APIs and C# native protection, no dependency edits.
[Remediation results](../acceptance-remediation.md) retain original red evidence and exact
current tests; identity-bound atomic overwrite remains FAIL32 and Phase A is not passed.
