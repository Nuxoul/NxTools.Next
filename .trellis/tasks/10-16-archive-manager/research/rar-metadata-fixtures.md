# RAR extraction metadata — independent synthetic fixtures

Date: 2026-10-18. This is bounded fixture/oracle evidence for the Phase A extraction
metadata slice, not product acceptance. The generator is CPython standard-library code and
does not call NxTools, UnRAR, WinRAR, 7-Zip, or another archiver to create its output.
No RAR creation capability is added to the product.

## Generator and exact fixture contract

Generator: `repo:rust/crates/nxtools-archive/tests/fixtures/generate-rar-metadata.py`.
It requires an existing caller-owned directory, rejects a symlink directory, and uses exclusive
creation by default. `--force` replaces only its three known existing regular-file outputs.
It emits JSON with every member, field expectation, payload hash, archive hash, and time precision.

Command used in the unique session scratch tree:

```powershell
$scratchRoot = Join-Path $env:PI_SCRATCH_DIR ('rar-metadata-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $scratchRoot | Out-Null
$outputDir = Join-Path $scratchRoot 'generated'
New-Item -ItemType Directory -Path $outputDir | Out-Null
python rust\crates\nxtools-archive\tests\fixtures\generate-rar-metadata.py $outputDir `
  > (Join-Path $scratchRoot 'generator-result.json')
```

Observed result: `fixture_count=3`; Python compilation/generation exit 0. A second default run
exited 1 and refused the first existing output. A nonexistent output directory exited 1. A
`--force` run exited 0 and reproduced identical SHA-256 values.

All archives contain exactly one explicit directory and one regular file. Expected payload is
23 bytes, UTF-8/ASCII `synthetic rar metadata\n`, CRC32 `34262377`, SHA-256
`25cc62e156bfbddc8c25fc93097d447a482ceb04de6daf5a9c9caf59f2959705`.

| Fixture | Members and format fields | Time contract | Generated SHA-256 |
| --- | --- | --- | --- |
| `synthetic-rar4-windows.rar` | RAR4, STORED; `metadata` directory and `metadata\payload.txt`; raw host OS 2 / UnRAR `HOST_WIN32`; directory attrs `0x13` = readonly+hidden+directory, file attrs `0x03` = readonly+hidden | DOS local wall clock `2020-02-03 04:05:06`, genuine 2-second precision. It is not UTC. UnRAR on Windows applies the reader's local time-zone rules when normalizing it to FILETIME. | `352d718e915ccecdbe964e9f7cbf36d14093ee221916459cce1a2eab4dea7130` |
| `synthetic-rar5-windows.rar` | RAR5, STORED; same logical members; raw host OS 0 / UnRAR `HOST_WIN32`; attrs `0x13` and `0x03` | Base Unix UTC seconds plus high-time Unix nanoseconds encode `2022-03-04T05:06:07.123456789Z`, declared 1ns precision. `RARReadHeaderEx` can expose only normalized 100ns FILETIME, expected `2022-03-04T05:06:07.1234567Z` after truncation toward the earlier instant. | `0821ba66541ffef9757d71f39d3e5976f24b0d691e9d18acbed6fe6cfd2e26e2` |
| `synthetic-rar5-unix.rar` | RAR5, STORED; `metadata` and `metadata/payload.txt`; raw host OS 1 / UnRAR `HOST_UNIX`; directory mode `040777`, file mode `100777` | Same RAR5 UTC time and precision as above | `582cdbf8c0509cbba0c81791863c5c758379898f54d8748537c638611f69e6e0` |

The Unix variant deliberately has low raw mode bits 1 and 2 set. Those are POSIX permission/type
bits under a Unix host and must not become Windows readonly/hidden. This proves the host gate is
material, rather than allowing `file_attr & 3` for every RAR entry.

## Locked UnRAR 7.1 / `unrar_sys=0.5.8` source evidence

The lock is `Cargo.lock` `unrar_sys 0.5.8`, checksum
`8b77675b883cfbe6bf41e6b7a5cd6008e0a83ba497de3d96e41a064bbeead765`.
The inspected bundled source is under the local Cargo registry exact crate directory
`unrar_sys-0.5.8/vendor`; it is read-only dependency evidence, not copied product code.

- RAR4 signature/header sizes and flags are in `vendor/unrar/headers.hpp:4-10,39-68,80-99`.
  `arcread.cpp:268-280` reads packed/unpacked sizes, raw host, file CRC, DOS time, method,
  name length, and attrs. `arcread.cpp:387-421` applies DOS local time and optional subsecond
  extended time. Header CRC is the low 16 bits of standard CRC32 over header bytes after the
  stored CRC (`rawread.cpp:165-170`).
- RAR5 common/file flags, high-time type and flags are in
  `vendor/unrar/headers5.hpp:4-35,81-99`. `arcread.cpp:630-678` shows a block's stored CRC32
  covers encoded header-size plus body; `rawread.cpp:174-179` confirms standard CRC32.
  `arcread.cpp:694-714,792-884` gives exact common order: optional extra/data sizes, file flags,
  unpacked size, attrs, optional Unix seconds/CRC32, compression info, host, UTF-8 name, then
  the extra area. `arcread.cpp:962-980` defines each extra field as vint field-size, vint type,
  then field bytes. `arcread.cpp:1099-1128` reads Unix or FILETIME high times and optional ns.
- `headers.hpp:93-104` maps RAR4 Windows/Unix and RAR5 Windows/Unix hosts to unified host types.
  `dll.cpp:255-287` exposes directory flags, maps unified Windows to public host 2 and everything
  else to public host 3, exports normalized `mtime.GetWin()` low/high, and returns raw attrs.
  The public Rust binding fields are `unrar_sys/src/lib.rs:139-173`.
- Public `RARHeaderDataEx` documentation states `MtimeLow/High` is UTC FILETIME and zero means
  unavailable (`vendor/Documentation/RARHeaderDataEx.md:283-288`). Internal `RarTime` keeps ns
  but `GetWin` returns 100ns units (`timefn.hpp:21-40`; `timefn.cpp:150-160`). Therefore the DLL
  cannot prove whether a RAR5 timestamp originated as Unix seconds/ns or FILETIME and cannot
  recover source precision. RAR4 local-zone ambiguity likewise is not recoverable from the DLL.

## External 7-Zip oracle

Explicit oracle (used only against the three generated scratch archives):

`D:\Tool\Directory Opus\Directory Opus\_Green\7z极限压缩\bin\x64\7z.exe`

- `7z i`: **7-Zip 26.00 (x64), 2026-02-12**; exit 0.
- File/product version: `26.00`.
- SHA-256: `4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f`.

For each archive the oracle ran `7z l -slt`, `7z t`, and `7z x -y -o<owned-dir>`.
Counts: 3 list + 3 test + 3 extract = **9 operations**, all exit 0; each test reported
one folder, one file, 23 unpacked bytes. Every extraction produced the exact payload hash above.

| Fixture | `l -slt` evidence | Extracted Windows metadata |
| --- | --- | --- |
| RAR4 Windows | members 2; host `Win32`; method `m0`; modified `2020-02-03 04:05:06.0000000`; attrs directory `RHD`, file `RH` | Local `2020-02-03T04:05:06+08:00`, UTC `2020-02-02T20:05:06Z`; file `ReadOnly, Hidden`; directory `ReadOnly, Hidden, Directory` |
| RAR5 Windows | members 2; host `Windows`; method `v6:m0`; listed local time `2022-03-04 13:06:07.123456789`; attrs `RHD` / `RH` | UTC `2022-03-04T05:06:07.1234567Z`; file `ReadOnly, Hidden`; directory `ReadOnly, Hidden, Directory` |
| RAR5 Unix | members 2; host `Unix`; method `v6:m0`; same listed ns time; attrs `D drwxrwxrwx` / `-rwxrwxrwx` | Same UTC FILETIME; file `Normal`; directory `Directory`, with no readonly or hidden replay |

The observed RAR4 UTC is workstation-specific evidence for the current `+08:00` zone, not an
archive-level UTC expectation. The portable expectation remains the DOS local calendar.

## Limits

- These are tiny non-solid STORED samples, not broad RAR compatibility, encryption, volumes,
  compression, memory, cancellation, corruption, or performance acceptance.
- 7-Zip independently validates format/test/extract/content/time/attributes, but it is not the
  product's Start -> Job -> Rust -> C# native lifecycle. That lifecycle must consume these
  fixtures in its own separately authorized regression.
- The generator encodes a precise RAR5 source timestamp so format behavior is testable, while the
  extraction contract must retain the honest DLL boundary: normalized FILETIME is available;
  original source encoding/precision is not.
- Readonly cleanup during oracle work was limited to the generated scratch extraction files before
  removing/recreating their owned extraction directories. No user archives or system state were
  touched.

## Canonical RAR5 path correction and product result

The initial WindowsRAR5 fixture above used backslash,which7-Zip accepted but locked UnRAR
arcread.cpp1403-1436 replaces with underscore forRAR5. Corrected generator now uses the
format-native `metadata/payload.txt` forbothRAR5host variants;RAR4retains backslash.
This was fixture correction,not a product path-policy relaxation. Initial fixture evidence
and the product's missing-member failure remain history rather than being called metadata red.
Corrected WindowsRAR5SHA256:
`65474c8955100bce8a6aad469c5091705149797c6ceefe5bccc1de5c81cf177c`.
RAR4/UnixRAR5hashes unchanged. Fresh canonical scratch generation plus3lists/3tests/3extracts
all exit0;independent fileUTC/attributes/payload same asabove. Own canonical scratch removed.
Actual Start->Job->Rust->native all3variants passed following exact packed-ABI correction
documented in extraction-metadata-sources.md. Current directed collection17/17,skip0/fail0,
exit0 (`metadata-all-directed-green.log`);not final integration/PhaseAacceptance.
