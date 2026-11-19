# Project-owned ZIP-SFX recipient

`nxtools-zip-sfx` is a standalone binary target of `nxtools-archive`. Its private modules do not call the archive worker, IPC, PlatformHost or an external codec. Filesystem access uses `std` and the locked safe tempfile 3.27.0 no-replace publication API; payload decoding uses locked flate2 1.1.9 with zlib-rs and crc32fast 1.5.0. Notices are embedded from `../../notices/zip-sfx/`.

## Wire and reader

The x64 PE raw end is `max(SizeOfHeaders, section.PointerToRawData + section.SizeOfRawData)`. The security/certificate directory must be absent. Exactly at that raw end is a 48-byte descriptor:

| Offset | Little-endian value |
| ---: | --- |
| 0 | 16 bytes `NXTOOLS-ZIP-SFX1`, without NUL |
| 16 | u32 version 1 |
| 20 | u32 descriptor length 48 |
| 24 | u32 physical disk count 1–1024 |
| 28 | u32 flags 0 |
| 32 | u64 physical prefix length: raw PE end + 48 |
| 40 | u64 reserved 0 |

Ordinary form has only the EXE and no span marker. Split disk zero is the EXE, intermediate disks use `.z02`, `.z03` onward, and the last disk is `.zip`, all at fixed same-directory sibling names. The split marker follows the descriptor. Coordinates are disk-local physical offsets, including any prefix on that disk. Payloads and descriptors can cross disks; local/central/end records are intact. Record extents must account for the complete physical ZIP region. No member-magic scan or sibling discovery is used.

`Archive::open(path, Limits)` retains read-only source handles, validates the descriptor/family/end records, enumerates original central ordinals and validates local records, descriptors and the full name/collision catalog. `Archive::test()` streams every entry, including empty files/directories, through complete packed consumption, Deflate StreamEnd, size and CRC checks. Dropping the archive releases its handles. Limits are at most 64 MiB charged metadata, 65,536 entries, 4,096 UTF-8 name bytes and 1,024 disks. Metadata admission includes retained catalog/projection storage; stream buffers are fixed 64 KiB plus the Deflate window. No whole-member buffer or decoded/encoded spool is used.

Only clear Store/Deflate, UTF-8 names (ASCII when unflagged), regular files and directories are admitted. Encrypted, other-method and special-type records are refused. ZIP64 local/central/end/locator coordinates are supported; small forced-ZIP64 fixtures do not establish large physical executable support.

## CLI and effects

- `--licenses`: print embedded project, dependency and Rust standard-library notices, without opening an archive or extracting files.
- `--test`: read-only validation of the current executable's own archive.
- `--output <absolute-absent-directory>`: extraction request.
- No arguments: destination prompt defaults to the executable's sibling stem; a second explicit Enter confirms the request.

No member, process, network, elevation, Registry or association operation is invoked. Diagnostics have fixed stage/error/count/native-code/elapsed fields and no persistent sink. The prompt displays only its requested default destination.

Extraction requires a fresh absolute root and validates the complete catalog before creating it. Each file uses a catalog-reserved sibling candidate opened with `create_new`, then full decode/size/CRC verification, flush and close. `TempPath::try_from_path(...).persist_noclobber(...)` publishes by native Windows `MoveFileExW` with flags zero. A publication error retains the original native code; automatic TempPath cleanup is disabled on that error so the explicit candidate removal owns the sole cleanup result. Earlier completed files and directories remain; no family-wide rollback is claimed.

## Rebuild

Read workspace `PACKAGING.md`, then from the repository root:

```powershell
.\rust\crates\nxtools-archive\scripts\build-zip-sfx.ps1 -EvidenceDirectory "$env:PI_SCRATCH_DIR\zip-sfx-rebuild"
```

This is an isolated recipient build, not a package/install operation. The helper waits for the shared build lane, requires 8 GiB available commit, sets the workstation Rust environment and scratch TEMP/TMP, retains exact source/build/toolchain evidence, and runs:

```text
cargo rustc --locked --offline -p nxtools-archive --bin nxtools-zip-sfx --release -- -C target-feature=+crt-static
```

The static inspector requires x64 PE machine 0x8664, raw PE end equal to file length, no certificate/overlay and only Windows system-library imports. It copies the exact owned `target/release/nxtools-zip-sfx.exe` bytes to `assets/zip-sfx-x64.bin`, never trims a PE, never executes it and refuses an unequal existing asset. Evidence includes source copies, sizes, command/environment, toolchain, import list and raw-image facts. A rebuilt binary can differ in linker metadata; replacement of a differing bundled asset requires explicit owner review. Embedded Rust notices match the 1.96.0 toolchain.

## Limitations

The bundled asset is bare PE bytes; the writer appends the descriptor and coordinate-correct archive. The reviewed own Windows loader accepts a physical first EXE up to4,294,967,295bytes and refuses4GiB/4GiB+1with Windows193. The835,584-byte asset plus48-byte descriptor and split marker requires at least835,636bytes per split volume, subject to intact-record admission. Small forced-ZIP64 fixtures establish64-bit record handling, separately from large-payload correctness.

Abrupt interruption can leave an owned `.partial` candidate; it does not imply success or rollback. Controlled native Test, exact extraction, existing-root refusal and exclusive source release pass on six specification-built and four actual-product profiles. This is not independent product, physical UI, installed or Release acceptance.
