# TAR mtime P2 remediation — execution evidence

Canonical phase: **Phase A — Engine contracts and safety foundation**. This repair is limited to
GNU signed base-256 header mtime and PAX mtime empty-value deletion/override. Original independent
findings in extraction-metadata-acceptance.md remain unchanged. Final status belongs only to
phase-a-checkpoint.md; the historical96 Rust/106 C# baseline is not this run.

## Boundary and preserved invariants

Production owner: formats/tar.rs only: fixed12-byte mtime decode and mtime-only PAX state merge.
Public ArchiveEngine.open/metadata and actual C# Start -> Job -> Rust worker -> native output
are the already-approved regression seams. New tar_time_contract.rs, ArchiveTarTimeTests and
caller-owned Python fixtures cover these seams; Program/TestEffects only register reviewed
self-owned fixture effects. No native C# production workaround, dependency/third-party edit,
size/other-key rewrite, UI, source delete/recycle, IPC/queue or release work.

Keep fixed512-byte checksum/extension count+length/payload/padding/end/metadata+work budgets;
ordinary paths/collisions/no-reparse/source leases; full integrity -> source/cancel recheck ->
metadata on owned unpublished token -> original single native publication; approved target
close->publish TOCTOU; exact partial cleanup and real committed/unknown facts; MOTW/wire policy.
No Archive.entries/unpack, Rust Windows FFI or final-path metadata mutation. Placement/focus/
show-hide invariants are not applicable: no UI surface is changed.

## Source verification before production changes

Rechecked tracked legacy D:\Code\NxTools archive terms: BrowserExtensionPackageService ZIP and
FFmpeg/image/video owners, no general archive restore chain in those matches. Archiver
core/src/lib.rs561-562,677-683 delegates entries/unpack_in; not reused because it bypasses Next's
budget/native boundaries. Existing source matrix/reference remain the migration index.
Locked tar0.4.46 header.rs553-554 returns u64; num_field_wrapper_from1512-1517 checks high bit;
numeric_extended_from1536-1550 ignores upper bytes of12-byte fields and keeps last8 unsigned.
This is not a signed/range-checked mtime API.

GNU tar1.35 src/list.c::from_header885-913 explicitly accepts first byte0200 positive or0377
negative base256 and checks sign-extension/overflow while consuming the full field. Source:
https://ftp.gnu.org/gnu/tar/tar-1.35.tar.xz . Independent CPython3.13.2 local Lib/tarfile.py
nti/itn recognizes the same marker bytes and signed big-endian numeric value; GNU_FORMAT writer
produces all twelve0xff for-1. Read-only fetched sources retained in session scratch
`tar-time-research`; no downloaded executable is installed or run.

## Actual first red (production unchanged)

Scratch logs tar-time-gnu-red-rust.log/tar-time-red-worker.log/tar-time-red-csharp-build.log,
tar-time-gnu-red-list.log/tar-time-gnu-red-native.log; ledger tar-time-red-ledger.txt.
- `cargo test -p nxtools-archive --features isolated-codecs --offline --test tar_time_contract -- --test-threads=1`:
  run1/pass0/fail1/ignored0,exit101; Some(Invalid) != UnixUtc(-1), not a compile failure.
- Same-feature actual worker build exit0; C# build exit0.
- Exact GNU fixture list selected1/run0/pass0/skip0/fail0/not_run1,exit0.
- Same selection actual native execution selected1/run1/pass0/skip0/fail1/not_run0,exit1:
  worker_failure/InvalidMetadata/committed0/cleanupTrue. Independent CPython generator and raw
  12-byte0xff assertion succeeded before actual product execution. Native expected literal
  1969-12-31T23:59:59Z and byte'x', not product self-readback.
All Cargo inherited required1/0/codegen-units256 settings and checked the single build lane.

## PAX semantics verified before merge changes

POSIX.1-2024 pax: https://pubs.opengroup.org/onlinepubs/9799919799/utilities/pax.html ,
Extended Header Records and Keyword Precedence (saved HTML1061-1083): zero-length value
"shall delete any header block field, previously entered extended header value, or global
extended header value of the same name"; a deleted/overridden ustar field must be ignored.
Local x applies to the following member, global g persists until updated; last same-key record
wins. Therefore local empty mtime suppresses global AND raw header only for that member
(metadata absence,not header fallback); next member retains its global context. Global empty
removes the persistent override as required by this repair's explicit set->clear compatibility
contract; later members use their own header unless a new local/global setting applies.
Do not describe the local deletion clause alone as proving every implementation agrees with
global reset semantics. Nonempty malformed digits remain Invalid,not deletion.

Inspectable independent implementations are not interchangeable oracles: CPython3.13.2
tarfile.py::_proc_pax/_apply_pax_info keeps empty numeric strings then ValueError->0 (saved
1695-1714); it cannot certify deletion. GNU tar1.35 xheader.c::xheader_decode700+ applies global
then local decoders; mtime_decoder uses decode_time,whose empty-time path is not a conformance
oracle. Inspected libarchive archive_read_support_format_tar.c global-attribute restrictions
and local numeric paths likewise do not prove global precedence. Historical7-Zip/bsdtar raw
header outputs are explicitly NOT the precedence oracle. Expectations use raw record bytes,
the POSIX local deletion clause and the user-approved global-clear contract,not product readback.

## Actual PAX reds

With GNU repair green, public global set->clear test run1/pass0/fail1/ignored0,exit101:
second time Some(Invalid) != UnixUtc1600000001. Exact native list selected1/run0,exit0; execution
selected1/run1/pass0/skip0/fail1/not_run0,exit1: worker_failure/InvalidMetadata/committed1/
cleanupTrue. Logs tar-time-pax-red-{rust,list,native}.log. Original first-file committed fact
is preserved,not a whole-job rollback. Additional local-empty scope test run with global test:
run2/pass0/fail2/ignored0,exit101; local Some(Invalid) != None. Log tar-time-pax-local-red.log.
GNU original red command after fix run1/pass1/failed0/ignored0,exit0; actual native selected1/
run1/pass1/skip0/fail0/not_run0,exit0. Extra raw header boundary test24cases passed1/1,exit0.

## Exact repair and focused coverage

formats/tar.rs::header_mtime(&[u8;12]) consumes the full fixed field: allNUL/space is None;
octal permits leading ASCII spaces/trailing NUL+space and only0..7digits; GNU0x80 starts0,
0xff starts-1 in checkedi128 multiply/add over all11remainingbytes,then checked i64 conversion.
Illegal marker/full-width overflow is Some(Invalid); decoded native-out-of-range seconds retain
typed Unix value and fail unchanged ModifiedTime::validate. No unsigned reinterpretation or
size/other-number change. Added leadingNUL+digits case found a new parser relaxation red1fail
(case8became1),then leading-space-only correction passed;log tar-time-octal-red.log. Final raw
matrix25cases includes epoch,1900,negative days,i64min/max,native min/max/outside,octal limits,
missing,illegal octal/embeddedNUL/marker and highbytes whose low64would appear harmless.

PaxModified::{Unspecified,Set(ModifiedTime),Delete} is mtime-only. Empty becomesDelete,not
unix_decimal(empty); localDelete returnsNone without parsing/falling back to global/header;
globalDelete resumes per-member header. Local is reset only on real member; global persists;
last same-key record wins. Nonempty malformed remainsSet(Invalid),including invalid global
after local override ends. Orphan local empty and malformed record length stillCorruptArchive.
Original checksum/extension/size/payload/end/budget/path/source/native commit code unchanged.

New public suite6tests green,exit0 before final integration. New C#4fixtures execute27small
actual Start/Job/worker/native transactions: GNU-1in5TARvariants;globalset/clearin5variants;
localoverride/delete/globalrestore/headerreset5members;12numeric/absence/invalidcases;
2invalidsecond-member/oldtargetcases;2deterministic post-file invalid-directorycases.
Independent Windows GetFileTime UTC oracle,exact bytes/terminal/count/cleanup;native range
1601+1second through9999-12-31T23:59:59Z. Original PAX signedfraction fixture is retained and
reexecuted. Last focused selection list5/run0 then selected5/run5/pass5/skip0/fail0/not_run0,
exit0;logs tar-time-coverage-final-{build,list,native}.log. No local-loopZIP64execution.

Preserved non-product/environment/test failures:
- Initial coverage selected5/run2/pass1/skip0/fail1/not_run3,exit1: .NETGetLastWriteTimeUtc
  at9999throws via DateTimeOffset.ToLocalTime;context rerun1fail proved stack. Product had
  already succeeded. Test-only direct WindowsGetFileTime avoids local-zoneoverflow;expected
  timestamp unchanged,not a product red/fix or a skipped boundary.
- Next coverage same5/run2/pass1/fail1/not_run3,exit1: test assumed first archive member must
  commit first. Existing pendingBTreeMap is randomtokenordered;invalid member may fail before
  another commits. Corrected test independently counts actually publishedfirst(0or1),retains
  exact oldtarget/foreignpartial,and added invaliddirectoryfinalization after a validfile to
  deterministically assert1/Committed. No production ordering change/global rollback required.
- First strictClippyexit101 flagged2octal-looking NUL escapes in tests;spelled\x00instead,
  identical fixture bytes,no lint suppression. Original product reds and successful focused
  runs remain separately recorded,not overwritten.

Read-only source delegate was stopped after source collection without a final report;not a
completed review or acceptance claim. Parent inspected the cited source windows directly.
No optional broad code review was launched. External wallhaven Cargo activity observed between
steps;waited for an empty lane,never killed or started a duplicate build. Final script repeats
the lane check before everyCargo and inherits all requiredmemoryflagsin its own process.

## Final integrated run — this repair, not historical baseline

2026-10-19 09:45:45.728->09:59:17.306(+08:00). Exact commands/step times/exits/count assertions:
session scratch `tar-time-final-verify.ps1`, `tar-time-final-ledger.txt`, `tar-time-final-*.log`.
Every row below actually executed exit0 on final code; no tests were skipped/ignored.

| Check | Actual result |
| --- | --- |
| focused public tar_time_contract |6pass/0fail/0ignored |
| archive isolated-codecs --offline --no-fail-fast -- --test-threads=1 |102pass/0fail/0ignored across22harness groups;two naturally empty bin/doc groups not counted as tests |
| same-feature actual worker build |exit0 |
| isolated/default strict archive Clippy --tests --offline -- -D warnings |both exit0 |
| cargo fmt --all -- --check |exit0 |
| cargo check --workspace --tests --offline |exit0;NOT workspace tests |
| extensions --test archive_import |7pass/0fail/0ignored |
| clipboard --lib archive_ |5pass/0fail/0ignored,39filtered |
| C# test build and referenced product |0warnings/0errors,exit0 |
| C# exact original106unionnew4LIST |selected110/run0/pass0/skip0/fail0/not_run110 |
| SAME exact execution |selected110/run110/pass110/skip0/fail0/not_run0 |
| LIST/RUN/PASS exact names |0differences;original106baseline reconstructed from preturnProgramsnapshot |
| actual4GiB+17FastDeflateZIP64 |RUN1/PASS1,once only final110;real Job/nativepublish+Pythonstreamingsize/SHA256 |

Original typed-I/O,metadata/MOTW,directoryfinalization,publication/cleanup,missing-internal-output
and2extensionprocess fixtures are retained in the110names,not replaced by newTARtests. No full
workspace tests/unrestrictedC# or native-user/release acceptance is claimed.
Python3.13.2 SHA2567d96a4ed35d6e596cd9dd8933feafc66349cc21f75be3b15c89fd336e50140c1;
existing7-Zip26.00 SHA2564bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f,
both reverified before execution. No install/PATHchange. ZIP64temp-volume headroom28661293056bytes.

Initial backgroundwrapper09:44terminated without terminalstatus while first test was compiling;
it is NOTPASS. Parent confirmed no surviving wrapper/ownCargo/rustc and noZIP64attemptmarker,
preserved original logs under scratch`tar-time-interrupted-final`,then ran full foregroundscript
with7200s timeout. No timeout/kill/duplicatebuild. During finalbatch externalCargolanes cleared
after10s and92s waits;no competing build was deliberately started. ZIP64attemptmarker prevents
automatic second execution. Final scriptstatusPASS and exact ledgercountassertions succeeded.

Postintegration inspection:0Cargo/rustc/archiveworker/testhost/7z,0fixedtesthelper;inspected
archive-tar-time,metadata,tree,source-binding,zip-create temp-root patterns all0. Fixtures assert
exact ownedpartial cleanup,oldtarget/foreignpartialpreservation and truepostcommitfacts. Scratch
logs/sources are retained;generated source-research fixture cleanup and static gates below.

## Remaining scope and stop

Both requested P2s are repaired;no further feature or review loop is authorized/started. Still
PhaseA,not fullA/A-F/PhaseB/UI. No source deletion/RecycleBin,productIPC/queues,dependencyupgrades,
thirdpartypatches,packaging/install/commit/push,userarchives,registry/service/drive-map mutation.
Broad FAT/SMB/UNC/DST/largecount/resources/performance/native-user acceptance remains separate.

## Final static gates and hygiene

All actually executed exit0 after evidence/context updates:
- python tests/architecture/spec-contracts.py:PASS103documents/14owners.
- python tests/architecture/gpui-component-catalog.py:PASS79pages/79rows/78lockedentries.
- python .trellis/scripts/task.py validate .trellis/tasks/10-16-archive-manager:
  PASS43implement/40checkentries,no truncation;reference32591bytes/checkpoint31673bytes.
- tests/architecture/platformhost-test-selection.ps1:PASSfixture-selection source gate.
- git diff --check:exit0;scoped13new/changedcode+docs checked for whitespace/conflicts and
  codeDEBUG markers;Program/TestEffects each differ from preturnsnapshot by exactly4new
  ArchiveTarTimeTests registrations,no removed/changedoriginalentries.
- Production snapshot diff has exactly5mtime-only edit blocks;no C#productionfileschanged.
- Final29archivefixture-rootprefixes derived from testsource all0;fixedhelper0 and observed
  Cargo/rustc/archiveworker/testhost/7z0. Removed only two self-owned researchTARs from this
  session scratch;retained fetched text/source,red/failed/interrupted/final logs and scripts.
  Hygiene scratch script's first JSON read failed defaultGBK;explicitUTF8 corrected it and
  wholecheckexit0. Initial failure retained in tar-time-hygiene-initial-gbk-failure.log;not
  counted as productred. No unrelated dirty cleanup/reset,dependency or generatedmanifestedit.

No code changed after final102/110integration. Original acceptance file was not edited. Stop
here with both requestedP2fixes verified and evidence saved;full PhaseA exits remain open.
