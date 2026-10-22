# Zstd bounded-read capability continuation

Starts only after the public XZ slice's actual52Rust/43precise C# green integration and documentation gates. This is the next approved Phase A gap, not a new phase or scope decision.

Reference inspected: `D:\Code\Archiver-main\core\src\lib.rs` decoder at236 uses ordinary zstd::stream::read::Decoder and extension aliases tar.zst/tzst at133. Do not copy unbounded ordinary decoder admission or extension-only validity. SupaZip actual `supazip/supazip-core/src/formats/mod.rs` lists tar.zst detection suffix but registers no Zstd backend; it is not Zstd capability evidence. No old NxTools archive interaction was established by earlier reference review.

Available exact cached crates: zstd-safe7.2.4 (MIT OR Apache-2.0), zstd-sys2.0.16+zstd.1.5.7 (wrapper MIT/Apache, bundled Zstandard1.5.7 BSD-3-Clause OR GPL-2.0; choose BSD and retain full notice). Prefer direct safe7.2.4,std with default features disabled: no bindgen/legacy/dictionary-builder/multithread/experimental feature. Native C build needs real Windows verification before advertised capability. No ZIP/GPUI upgrade or upstream edits.

Inspected safe/src/lib.rs: DCtx::try_create(906) fails with None instead of panic; set_parameter(WindowLogMax)(1127), decompress_stream(1174) returns0 only at complete frame; InBuffer/OutBuffer position tracks consumed/produced. Standard headers and block extents can be parsed in fixed memory before context/data allocations; reject external dictionary IDs as typed unsupported. Native getErrorCode enum is stable but unsafe FFI is denied by the crate policy; exact upstream error_private.h49–54 encodes error as size_t-negative enum, so wrapping_neg plus public zstd_sys::ZSTD_ErrorCode constants gives fixed enum classification without strings/unsafe. Memory allocation/window-too-large→ResourceLimit; dictionary/unsupported-version/parameter→UnsupportedMethod; malformed/end/checksum→CorruptArchive.

Native zstd_decompress.c1993–1998 estimates DCtx + input block + window/extra output block. Conservatively reserve8MiB fixed plus2×admitted power-of-two window; set WindowLogMax before streaming and preflight every frame's exact window/metadata/block extent. Single-threaded fixed64KiB I/O, actual output/member limits/cancellation and existing C# Job ceiling remain. No whole-frame Vec. Standard/skippable frame concatenation needs checked extents/counts; no trailing garbage or truncated EOF success. Checksums are verified only if format supplies one; no fabricated payload-integrity guarantee when absent.

Independent oracle: Windows System32/tar.exe reports bsdtar3.7.7/libarchive3.7.7/libzstd1.5.4, supports --zstd --format=raw. generate-zstd.py produces small independent standalone/multi and compressed existing CPython PAX TAR, using GUID temp inputs and no installation. Expected bytes come from literals and existing Python payload, not NxTools encoder. Public red must be run before implementation; actual native/safety/resource checks remain required.

## Executed red→green and integration

Before code, public zstd_read_contract1run/0pass/1fail/0ignored,exit101 UnsupportedFormat,
scratch zstd-public-red.log. Same red after bounded adapter+compound+engine integration1/1,exit0.
Payload size expectation changed from a mistakenly typed277 to the exact literal UTF8 length+256,
not to observed NxTools output; initial red had failed earlier at open, never at this expectation.
Independent generator bsdtar has no zstd:checksum option (actual exit1); Python adds RFC XXH64
low32 checksum with published empty/a vectors, and independent bsdtar -tf checked.tar.zst passes,
exit0. No third-party oracle install, no NxTools compression supplies expected input/output.

zstd_read_contract1 + zstd_safety_contract4 pass: single/multi/checksum/skippable prefix+tail,
compound aliases, unknown size/exact bytes/files, malformed header/checksum/truncate/garbage,
external dictionary UnsupportedMethod, huge window/work/metadata/count/actual-output/spool,
cancel/output I/O/original compressed-source changes, exact-token cleanup, empty/single-segment
raw/declared-size mismatch/skippable-only/truncated skippable. Test/extract consumes entire frames.
ActualZstdReads is reviewed Fixture:50operations (2formats×8modes×open/test/extract +2cancel)
in actual C#Job→Rust→ArchiveEngine→native output, exact bytes/count/state, source lease denied
write at native prepublication point, cancellation0commits, no invalid output/partial and lease
released after drain. Listed selected1/run0, then selected1/run1/pass1/skip0/fail0,exit0.

Current-source final integration (all serial Cargo with required3memory environment values):
- cargo test -p nxtools-archive --features isolated-codecs --offline --no-fail-fast:
  57passed/0failed/0ignored,exit0; all prior XZ tests retained and rerun.
- same-feature worker build; strict archive Clippy --tests/--offline/-- -D warnings;
  cargo fmt --all -- --check; cargo check --workspace --tests --offline:each exit0.
  Workspace check is NOT workspace tests. Clippy first rejected nonminimal Boolean only;
  explicit stalled state fixed it without suppression.
- C# build --no-restore0warnings/0errors,exit0. Precise original40+XZ3+Zstd1 list selected44/run0;
  execution selected44/run44/pass44/skip0/fail0/not_run0,exit0. No unrestricted/default suite.
- extensions archive_import7/7; clipboard --lib archive_5/5,39filtered,0ignored,exit0each.
Logs: session scratch zstd-final-{archive,worker,clippy,fmt,workspace,extensions,clipboard}.log
and zstd-final44-{list,tests}.log. No timeout, duplicate build, Release/install/real registry/
association/service/user archive/commit/push. Phase A not passed; next remaining ZIP methods/
encryption, general creation/traversal/native/contracts/resources. No current decision blocker.

Independent SHA256:single979BC89A84D29CE53EAB2256B872B866366CFD81E7650DCBEC8E96C9B70B1156;
multi5ED9BEC15627B68978A94CBBB641DE2E840AF5A56BB278C243266EADA3DA5DC4;
tarEDD12639584DAAD33ACC545655658EA394361F0F5BC0D18F5C5B21BE08AAB199;
checked8C88DA08F382EB8AADEC830395F281815887F8DE0CB18E6F308BDCC971248DF4;
checkedTarCC1155E66903D50434E5D6D1AF4DA6E61EB979C6A743D3F48AE8FD6D992DC501.

Final cleanup/hygiene inspected after integrated44run:0Cargo/rustc/archive worker/test-host,
exclusive fixed helper absent,0XZ/stream/oracle temporary fixture roots. Native spools are
anonymous delete-on-close and dropped before worker cleanup; no retained product spool paths.
45related text files passed newline/trailing-whitespace/NUL checks; task JSON valid; scoped
diff--check exit0 (existing Program.cs CRLF advisory only). New5Zstd fixtures not git-ignored.
ZIP remains8.6.0; new lock entries only safe7.2.4/sys2.0.16. Reference32471/design32671/native
owner32522/archive quality24555bytes below32768context limit. No DEBUG probes added.
Final document gates rerun: spec-contracts101docs/14owners,component catalog79/79/78,task context
24implement/21check(no truncation),fixture-selector:all exit0. Existing low-noise correlated
worker accepted/start/source-acquire/terminal and native output lifecycle events are reused;
no new public IPC operation or payload logging was introduced.
