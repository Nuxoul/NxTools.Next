# Phase E — exact ZIP multi-disk and raw-copy capability gate

2026-10-22. **Required locked-backend gaps verified; no Phase E product implementation or acceptance.**
Read with [plan](../implement.md), [design](../design.md), [reference](../../../../docs/reference-code/archive.md)
and the sole [current checkpoint](../phase-a-checkpoint.md). This is additive research, not a rewrite of
Phase A–D evidence or permission to upgrade dependencies, change backend/native authority or drop P202/P203.

## Scope and immutable inputs

- Workspace lock: zip8.6.0, checksum `2d04a6b5381502aa6087c94c669499eb1602eb9c5e8198e534de571f7154809b`;
  archive consumer enables aes-crypto plus inherited Deflate. No manifest/lock/vendor/product edits.
- Source root: `C:\Users\Administrator\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\zip-8.6.0\src`.
- Reference first: archive note identifies no general legacy archive-browser implementation; that
  established absence was not re-researched. Re-read Archiver `core/src/lib.rs::delete`763–825:
  by_index_raw782 -> raw_copy_file784 -> finish/flush787 -> fs::rename818; predictable File::create779
  and path rename are not Next's transaction or metadata guarantee. SupaZip nested core has no
  matching split/raw-copy/edit entry found by the bounded source search; no alternate proven backend.
- One unique scratch run: `C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\phase-e-20261022-093639`.
  Contains append-only commands.jsonl, separate logs, original447-file hashes, fixture/API sources,
  input/output manifests and exact loaded-library hashes. No overwritten failure log.

## Exact source/API evidence

| Concern | Exact entry and observed contract |
| --- | --- |
| Multi-disk reader | read/zip_archive.rs::read_central_header177–205 explicitly returns UnsupportedArchive at190–193 when end-disk differs from central-directory disk. Equality does NOT implement mapping all disks; it seeks one global directory_start and member header offset. |
| Multi-disk writer | write.rs::write_central_and_footer1896–1958 emits disk_number=0, disk_with_central_directory=0, ZIP64 number_of_disks=1. types.rs::central_header_block691–718 emits entry disk_number=0. No public volume callback/size/disk-index API is provided in these paths. |
| ZIP64 disk start | extra_fields/zip64_extended_information.rs25–35 has commented-out disk_start, not an implemented member-volume locator. |
| Current public engine | nxtools-archive formats/zip.rs::directory_candidate84–85 rejects nonzero/mismatched disk counts as MissingVolume; ZIP64 locator101–102 and disk header127–131 likewise. list197/258–268 rejects nonzero member start disks; ZipLocation holds a single-file header_offset. engine.rs31–61 opens exactly one File. |
| Raw copy | write.rs::raw_copy_file1691–1693 -> raw_copy_file_rename1623–1632 -> file.options()/start_entry1641–1652 -> take_raw_reader, not a byte-for-byte local/central record copy. |
| Options preservation | read.rs::options1046–1069 returns SimpleFileOptions: method, normalized Unix mode/time and AES metadata only; arbitrary extras, raw names/encoding, internal/external DOS attributes and ZipCrypto mode are not all represented. File comment is copied separately as decoded text1629–1630. |

A segmented generic Write/Seek sink does not change the writer's fixed disk0 records or translate
central/local offsets into disk-relative coordinates. Byte-chopping its output would not satisfy P202.
Similarly raw_copy_file's promise to avoid recompression is not a promise to preserve every original
record/flag/extra; its success must not be presented as safe P203 completion.

## Independent tiny real-format evidence

`zip-capability-fixture.py` uses CPython struct/zlib to encode actual local/central/end records from
scratch, NOT create an ordinary ZIP then cut it. Two Store members, each23bytes, known literal bytes:

- two disks: .z01 begins split marker PK0708; first member disk0/offset4, second disk1/offset0;
  central directory and EOCD on disk1; sizes64+188=252bytes.
- three disks: same member disks, central directory on disk1, EOCD alone on disk2; sizes64+166+22=252.
  This is a legal layout with unequal end/central disks, not a forged missing-volume condition.

Independent oracle: recorded explicit portable 7-Zip26.00 x64, hash
`4BCC005758C5E604C0AAA257B6097846A4A708E62A9C0C7471FA360FAD863F1F`; identity rechecked before use.
Only scratch fixture list/test/extract was invoked; not product backend, GUI, installer or Shell.
For EACH family l -slt, t, x -y -o<owned-scratch> returned exit0 (**6/6commands**).
Listing observes Multivolume=+, Volumes=2/3 and member Volume Index0/1, offsets4/0.
Both extracted files in both families match the independently declared SHA256s:

- one: `B764F8E887AC45F59E9870733B4839276A56ED851D0B6FDF8C22359D0416BFBE`
- two: `C07AE924477EE709E61E58B0F0D427C70E635EADB44A40B5C8C95DA5C4B2A12A`

Manifest includes every volume's length/hash. These samples prove two small ZIP multi-disk structures,
not all ZIP64, encryption, spanning-header, missing-volume, source-identity or production lifecycle gates.

## Runtime capability probe and genuine negative results

`zip-api-probe.rs` calls the real locked ZipArchive/ZipWriter and public ArchiveEngine, linking existing
workspace rlibs. This is **not a fresh Cargo/source verification**: probe-libraries.json records archive
rlib hash `A98446EE6C9DD89634A518FA4597CEF85CBFE806542E3C888C36F768455B0671` and ZIP rlib hash
`BF24582A30960CB4C33C715EFA96D3D45B606A798ADDA26F5DCF4EDF713516F5`. Exact source findings above
are independently inspected; no compiled freshness claim is made. Required Rust memory variables
were set, lane checked before rustc, no Cargo/build/service/helper/Job launch.

1. Naively concatenating the two-disk ZIP lets ZipArchive construct2entries, but first member open
   fails Invalid local file header while the last-disk member reads23bytes. It is not split support.
2. Concatenating the three-disk ZIP is explicitly rejected with multi-disk UnsupportedArchive.
3. Public current ArchiveEngine opening either last .zip returns MissingVolume, despite all siblings
   being present. No production volume discovery or set-wide revision exists yet.
4. Plain CPython metadata control raw_copy_file returns success and preserves packed SHA256 and
   explicitly copied archive/entry comments. **It loses0x5455 timestamp extra**, changes external
   attributes35(readonly/hidden/archive) to2166620160 and internal attribute1 to0.
5. Existing independently produced ZipCrypto control, by_index_raw -> raw_copy_file, also returns
   success and preserves all3packed payload hashes. Encryption mask0x0001 is nevertheless cleared:
   input flags2049 become2048/0/0. Independent7Zip t reports2Data Errors, **exit2**, not PASS.
   No password argument is used/needed to establish that the supposedly raw copied ZIP now claims
   unencrypted data. No AES preservation blanket claim: AES metadata API exists, runtime not tested here.

The library API probe itself exits0 because it records results; that exit is NOT a feature test PASS.
Oracle exit2 and exact metadata/flag differences are retained as real capability negatives.

## Attempts, checks and exclusions

- Initial PowerShell5.1 wrapper failed before fixtures: non-BOM UTF8 CJK path decoded as ANSI;
  use the already recorded explicit UTF8 JSON oracle path instead. No system encoding change.
- Initial rustc probe compilation exit1: set_raw_comment takes Box<[u8]>, not &[u8]; fixed only
  probe to .into().expect(...); distinct corrected compilation exit0, probe run exit0.
- Fixture generator and independent metadata inspector each exit0; six volume oracle commands exit0;
  raw-copy ZipCrypto oracle exit2. Per-command logs are separate; no failed observation overwritten.
- No missing/replaced/wrong-order native volume test, no P203 product regression, no C# LIST/RUN,
  no matching-worker rebuild/full Desktop/Clippy/Cargo, no GUI/native acceptance was run.
- P202/P203 remain required and unimplemented. P204/P205 unimplemented by scope. No Phase F,
  registry/service/source recycling, package/install/signing, commit/push or first-release claim.

## Decision requested (not implemented)

**Recommended:** retain locked dependencies and approve a narrow project-owned ZIP format adapter,
not a fake segmented sink: real disk-relative ZIP/ZIP64 reader/writer and record-preserving ZIP
rewriter. Reuse existing payload codecs, finite budgets, original worker/Job/queue/native transactions;
no external executable backend, vendor patch, new process framework or blanket permission expansion.

The adapter must retain immutable set revision/entry IDs; hold each bounded exact volume's native
identity for the operation; copy unchanged compressed/encrypted ranges and permitted raw headers/extras;
regenerate only offset/name-dependent fields; reject unsupported signed/SFX/broken/application containers
before editing; verify complete exclusive candidate; publish through original native final recheck and
single replacement. Missing/changed volume and partial multi-file publication facts remain explicit.
A production native regression must prove same-source replacement with its held source lease and
POSIX_SEMANTICS; do not infer success OR failure from share flags alone. If that existing operation
cannot support it, return precise native evidence before changing native authority.

Alternative: separately research a replacement/additional backend only after user selection, then prove
exact version/features/license/Windows build/resources and all required semantics. **No verified
replacement is proposed here**, and an arbitrary version upgrade is not evidence or authorization.

Stop at this genuine dependency capability gate pending the explicit decision; do not silently omit ZIP
volumes/encrypted preservation or claim ordinary supported ZIP editing is complete by refusing all edits.
