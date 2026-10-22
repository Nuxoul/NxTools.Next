# Extraction metadata — exact source/API evidence

Read-only investigation2026-10-18; no proprietary Bandizip source. Locked:zip8.6.0,
sevenz-rust2=0.23.0,unrar_sys=0.5.8(UnRAR7.1),tar0.4.46,flate2=1.1.9,bzip2=0.6.1,
lzma-rust2=0.21.0,zstd-safe7.2.4. This is source evidence,not implemented/accepted matrix.

## Matrix and conversion policy

| Container | Actual metadata source | Time semantics / chosen policy | Windows flags |
| --- | --- | --- | --- |
| ZIP | original central46byte record DOS14/12,version-made-by4,external38; bounded extras0x000a/0x5455 | Prefer valid provided NTFS mtime,then UT,then DOS; malformed provided higher-priority value fails,not silent fallback. NTFS100ns since1601UTC(including0valid raw timestamp);UT unsigned32Unix seconds(including0valid epoch);DOS local2s,all-zero treated unavailable. Local-only extra not supported in this slice; central metadata is authoritative. | low1readonly/2hidden only verified DOS/Windows-origin host0/10/14; other origins no fabricated flags |
| 7z | ArchiveEntry has_last_modified_date,last_modified_date NtTime;has_windows_attributes,windows_attributes | presence flag controls absence;NtTime100ns since1601 despite erroneous upstream0001comment;0is valid if present | low1/2from K_WIN_ATTRIBUTES; upper POSIX encoding bits never replayed |
| RAR4/5 | RARReadHeaderEx HeaderDataEx mtime_low/high,host_os,file_attr | normalized FILETIME;0means absent in this API. Original precision/encoding unknown. RAR4DOS local conversion already performed by UnRAR Windows implementation; no extra local conversion. RAR5 may originate Unix/ns/FILETIME. | HOST_WIN32 only1/2;Unix raw mode not Windows bits |
| TAR | fixed Header.mtime plus bounded local/global PAX mtime | signed Unix seconds+decimal fraction;local overrides global overrides header. Floor sub100ns;preserve declared decimal precision. Invalid/out-of-range is explicit. | no Windows hidden;Unix mode is not a Windows attribute and is not replayed in this slice |
| TAR.GZ/BZ2/XZ/ZST | inner TAR only | same TAR semantics;outer header never overrides member metadata | same TAR |
| single GZ | first RFC1952MTIME bytes4..8,validated header | u32Unix secondsUTC;0unavailable. Output concatenates members,so first member's metadata describes stream output;later times do not overwrite. This explicit policy is not an upstream aggregate guarantee. | absent |
| single BZ2/XZ/ZST | none | genuine unsupported/not-provided;never source stat/current time | absent |

## Locked API/source chains

ZIP read.rs995-1018 last_modified()/unix_mode(),1073-1078 extra_data_fields();public ExtraField
Ntfs/ExtendedTimestamp in extra_fields/mod.rs35-43. ntfs.rs8-15epoch/units,34-67strict field
length/tag,70-106mtime/atime/ctime;extended_timestamp.rs59-145flags/length,148-163mod_time etc.
datetime.rs9-22no zone/2s,243-303try_from_msdos validates. Central extras read.rs492-567;
local data-start types.rs239-264only skips local extra. System upper made-by byte types.rs36-129;
external_attributes types.rs166-224,HasZipMetadata read.rs727-731. DOS readonly/directory mapping
in types.rs319-347does not prove arbitrary Unix low bits are DOS flags. No new ZipArchive parse.

7z archive.rs83-123,222-244presence/time/attrs;reader.rs587-645property bitsets/raw values;
time.rs22-35ticks/sec and88-120Unix offset134774days prove1601epoch. K_WIN_ATTRIBUTES is the
format's named Windows property,no host-OS field;the upper mode bits must never be copied.
Exact external interoperability fixture is required before the attribute row is accepted.

RAR worker_codecs::run_rar -> worker_codecs/rar.rs::execute -> RARReadHeaderEx;unrar_sys
src/lib.rs139-174public fields;bundled dll.cpp206-287exports normalized Windows/Unix host,
DOS GetDos and m/c/a GetWin low/high. headers.cpp3-10reset;timefn.hpp68-70IsSet itime!=0.
arcread.cpp275-280,387-421RAR4DOS/exttime;timefn.cpp3-33,70-110Windows local/DST conversion;
arcread.cpp811-850,1099-1128RAR5Unix/high-time variants. list.cpp440-476Windows1/2bit evidence.
Public DLL loses original source precision and RAR4zone ambiguity;do not claim recovered zone.

TAR header.rs501-518mode,552-568mtimeUnix;entry.rs120-143pax_extensions andpax.rs87-147raw
key/value;mtime/atime/ctime keys pax.rs9-20. Internal helper pax.rs64-84onlyu64 andarchive.rs
327-355does not apply fractional mtime;Next must parse bounded decimal,not floating point.
Next fixed512byte records/pax parser remains;no Archive.entries/unpack. Compound tgz.rs17-114
spools full outer integrity before TAR;source authority stays compressed original.
Gzip flate2src/gz/mod.rs59-86MTIME GMT/zero unavailable,118-143raw population. BZip2/XZ/Zstd
container/API inspection found no genuine time/Windows attrs;do not invent fields.

## Reference behavior and limits

Legacy tracked grep still finds only BrowserExtensionPackageService ZIP/package replacement,
File.SetAttributes Normal at152-173andunrelated time consumers;no general archive restore chain.
Archiver core/src/lib.rs588-640ZIP/7zmanual writes:no restore;642-662RAR extract_with_base
reaches UnRAR extract.cpp972-1004time+attrs;672-686TAR unpack_in reaches tar Archive default
preserve_mtime archive.rs53-66,entry.rs693-707time and823-846Windows readonly mapping of Unix
owner-write. Single streams lib.rs664-670manual write:no restore. This slice deliberately does
not replay Unix mode as Windows attrs;no hidden is inferred from dotfiles.
SupaZip nested supazip/supazip-core formats/zip.rs184-196,293-342;sevenz.rs64-101,353-414;
tar.rs152-170,209-258;tar_gz.rs171-189,231-270list times but manual extraction does not restore.
Its missing7ztime->epochSome behavior is incorrect and not copied.
No tracked Zone.Identifier/MOTW runtime implementation found in these3projects. Bandizip
SimpChinese.lang Security setting is only resource-level evidence,not runtime proof.

## Native capability evidence

SDK10.0.26100.0FILE_BASIC_INFO winbase.h9077,FileBasicInfo0minwinbase.h179,NtCreateFile
winternl.h475. Self-owned NTFS scratch probes:fixed :Zone.Identifier relative to held source
and exclusive partial succeeds0. Readonly set0/rename0/delete5/clear readonly same handle0/
owned delete0. No mutable DOS source reopen,arbitrary ADS authority or system changes.
These probes do not certify product lifecycle/FAT/SMB. Exact Start/Job/native fixtures follow.

SDK capability addition: fileapi.h889-898 `GetVolumeInformationByHandleW(HANDLE,...,flags,...)`
queries the volume of the held object (no path reopen);winnt.h15017 FILE_NAMED_STREAMS=0x40000.
An absent bit is a capability decision,not a failed native call;report motw_not_supported with
no invented native failure code. Failed query preserves GetLastWin32Error. Source fixed ADS
read uses share-read only;target/cleanup ADS may share-delete to coexist with their held base
DELETE access. Guarded new-output cleanup adds actual attribute and fixed-MOTW digest/absence
credentials before clearing readonly;externally changed metadata must be retained.

ZIP UT addition: locked extended_timestamp.rs74-88 explicitly permits length5 while flags
refer to local header times. Accept central mtime-only length5 with mtime flag,or exact full
flag-count length;unknown flag bits/truncated supplied mtime still fail. Native red fixture
flags7+mtime-only independently reaches InvalidMetadata before this correction.

Actual native zero-time edge: ZIP NTFS FILETIME0 reaches SetFileTime success but same held
object's last-write remains unchanged on this Windows/NTFS workstation. Zero remains a valid
typed archive value,not absence;native application verifies the observed exact value and fails
metadata_time_not_supported(no invented error code) before publication when Windows cannot
represent it. Unix1970zero is nonzeroFILETIME and restores normally. No fallback/fabricatedtime.

RAR ABI correction discovered by native red: unrar_sys0.5.8 vendor/unrar/dll.hpp4 applies
pack1 to RARHeaderDataEx78-117,while src/lib.rs139 uses repr(C). Onwin64 its first pointer
CmtBuf follows offset6188 and is unaligned;binding inserts4bytes and mtime reads wrongwords.
Exact bundled packed header has MtimeLow offset6268,size10244;also includes modern ArcNameEx/
FileNameEx slots+Reserved982. Implement a private codec-FFI packed header,not a WindowsAPI,
zero-initialize pointer/reserved slots,and copy packed arrays before forming Rust references.
Do not relabel incorrect smallticks as a timezone difference or change the independent oracle.

## Preserved compound-stream reference evidence

Moved verbatim from archive reference to keep its injected index below32KiB:
Compound TAR.GZ follow-up reuses bounded Gzip+TAR parsers with a disk spool, never whole
archive Vec. Exact already-locked tempfile3.27.0 MIT/Apache-2.0 has Windows create_new,
share_mode0,temporary/delete-on-close in file/imp/windows.rs42–60; no persist/MoveFileEx path.
Source revision/lease remains the compressed source; only after full outer CRC/ISIZE/end may
inner TAR parse/output begin, and source is rechecked before native publication. Spool bytes
are bounded by max_output_bytes, individual TAR files retain max_entry_bytes. No new secret
disk persistence: this slice is nonencrypted Gzip only; no Windows FFI added to Rust.

Standalone BZip2/TAR.BZ2 continuation uses the same reviewed bzip2=0.6.1 low-level
decompress/total_in/out/StreamEnd/MemNeeded interface, not ZIP12 support as inferred evidence.
Fixed buffers, small-mode8MiB admission, per-member explicit end/CRC and bounded member count;
multiple members checked in sequence, garbage/truncation rejected. CPython bz2 single/multi/
tar.bz2 fixtures independently verify bytes. Compound disk spool/source-recheck policy matches
TAR.GZ above; no BZip2 write capability is advertised or added.

## TAR time P2 source clarification

Independent findings remain in ../extraction-metadata-acceptance.md; repair source/call-chain,
exact quotes and red/green records: [TAR repair](../tar-time-remediation.md). Rechecked before
production changes: legacy tracked package/FFmpeg archive consumers and Archiver561/677/683;
no general legacy archive restore chain found in those matches, no proprietary Bandizip source.

- Locked tar0.4.46 header.rs553mtime returnsu64;1512-1550binary wrapper truncates12-byte fields
  to last8unsignedbytes. Use mtime-only fixed12checked decoder,not that API or Archive.entries.
- GNU tar1.35 src/list.c::from_header885-915 accepts0200/0377 markers,full signed big-endian
  base256 and overflow checks. Source https://ftp.gnu.org/gnu/tar/tar-1.35.tar.xz . CPython3.13.2
  Lib/tarfile.py::nti/itn independently confirms marker/encoding,including all0xff for-1.
- POSIX.1-2024 https://pubs.opengroup.org/onlinepubs/9799919799/utilities/pax.html Extended
  Header Records/Keyword Precedence: zero-length value deletes same-name header/extended/global
  value; overridden/deleted ustar field is ignored. Localx applies only to next member,globalg
  persists until changed,last record wins. Localmtimeempty therefore suppresses header+global
  for one member; globalempty removes persistent override per this slice's approved compatibility
  contract,set->clear->later member restores its own header. Never treat empty as numericInvalid.
- Independent implementations have limits: CPython _proc_pax/_apply_pax_info converts empty
  numericValueErrorto0,not a deletion oracle. GNU1.35 xheader.c::xheader_decode/mtime_decoder/
  decode_time does not certify empty handling. Inspected libarchive globalmtime restrictions and
  Go's empty skipping do not prove global precedence. Apache Commons Compress TarUtils.java
  parsePaxHeaders679-731 explicitly removes empty keys from a map copied from global state;
  TarArchiveInputStream521-538 constructs raw header then applies maps,so key removal alone does
  not prove local raw-header suppression. Inspectable source:
  https://github.com/apache/commons-compress/blob/master/src/main/java/org/apache/commons/compress/archivers/tar/TarUtils.java .
  Dynamic-source line numbers describe the fetched scratch snapshot,not an immutable release.
  No7Zip/bsdtar header-time output is claimed as PAXglobal/local precedence oracle.

All declared nonempty malformed/out-of-native-range times remain explicitInvalidMetadata; no
now/unsigned/epoch fallback. Native file time is queried independently via WindowsGetFileTime;
DateTimeOffsetlocal conversion inside .NETFile.GetLastWriteTimeUtc can overflow near9999even
after successful product restoration. Native FILETIME0 limitation remains unchanged. No other
PAXkey,size/checksum/extent/budget/publication or native-metadata contract was changed.
