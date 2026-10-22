# Phase A independent-acceptance remediation

Historical remediation checkpoint: strict overwrite FAIL32 below was correct at this time.
The later user-approved target-contract amendment and current path-overwrite results are in
[overwrite remediation](overwrite-remediation.md); source lease/budget/codec repairs here remain.
No historical FAIL is rewritten as a past PASS.

Date:2026-10-17. Scope: the three concrete findings and encrypted extraction evidence from
[acceptance-review](acceptance-review.md). **Phase A is still NOT PASSED; first release A–F is
NOT ACCEPTED.** Historical FAIL records remain unchanged. No UI/phase reorder, dependency
upgrade/patch, Release/package/install/registry/default-association/service action, commit or push.
Unrelated dirty work was preserved. Initial process inspection found no Cargo/rustc/project-test
build; installed NxTools and Unity processes were left alone.

## 1. Stable source identity — repaired at the real worker/native seam

Root cause: fingerprint,7z reader and RAR enumeration/decode independently opened a mutable path.
Matching path hashes did not bind the bytes actually decoded and could miss A→B→A replacement.

Files: `rust/crates/nxtools-archive/src/{worker,worker_output,worker_codecs}.rs`,
`src/worker_codecs/rar.rs`; C# `Windows/Archive/{ArchiveSourceLease,ArchiveWorkerRunner,ArchiveOutputSession}.cs`;
`ArchiveRemediationTests.cs`, `tests/worker_wire.rs`, test registration/effects.

Schema2 negotiates `source_lease_v1` and `budget_reservations_v1`, then awaits source_acquire
before source reads. C# holds each no-reparse parent relative to the previous handle without
FILE_SHARE_DELETE, and a read-only source allowing FILE_SHARE_READ only. Existing write/delete
access prevents acquisition. Every subsequent fingerprint/7z open and both UnRAR path opens is
constrained to the same protected namespace/file object. Handles remain until actual child,
pumps and Job cleanup, not the terminal codec frame. Rust owns codecs/domain; C# owns Windows
handles/API. No snapshot or password persistence. Local-drive/depth64 restrictions remain
explicit; this is not UNC/large-path acceptance. Schema1 ZIP consumers retain prior behavior;
schema1 isolated codecs return UnsupportedOperation rather than silently running unprotected.
Rust wire tests exercise capability/source denial and legacy compatibility.

Red: build with lifecycle rendezvous but no lease, then
`--fixtures "archive remediation actual source lease"`: selected1/run1/pass0/skip0/fail1,
exit1. All3formats (7z,RAR4,RAR5) allowed path replacement and writable opens at all4barriers.
Test-only changes preceded the lease fix; no commit was made. Green: same fixture1/1, exact
independent payload bytes, all four stages reached. RAR codec_opened is inside the real
RAROpenArchiveEx handle lifetime; source_decoding runs in UCM_PROCESSDATA. Mutations include
rename/new same-name object/restore, write+restore and parent rename. Protection must deny them,
not merely notice an mtime/hash difference afterward. After exit the source can be reopened for
write. The separate writable-mapping fixture returns SourceChanged before codec/no output on
this workstation. Earlier draft mapped-fixture failures did not reach validation and were
corrected to assert this allowed safe refusal; they are **not** evidence of a mapping bypass.

SDK/source basis: NtCreateFile RootDirectory/single-component opens, FILE_OPEN_REPARSE_POINT,
share-access compatibility. Rechecked exact UnRAR7.1 dll.cpp RAROpenArchiveEx→FMF_OPENSHARED and
file.cpp CreateFile read/write sharing; no proprietary code or Bandizip reverse engineering.

## 2. Working / metadata / process budget — repaired and bounded-fixture verified

Root cause: Rust accepted max_working_bytes but7z never used it; C# independently chose default
512MiB MemoryBytes. An unrelated process ceiling did not honor a low requested codec budget.

Files: `ArchiveWorkerBudget.cs`, `ArchiveWorkerRunner.cs`, `worker_codecs.rs`, new
`worker_codecs/sevenz_budget.rs`, worker/Job fixtures and exact tests.

Contract:
- Work reserves codec dictionary + conservative decoder state. Current unconfigurable isolated
  backend admission granularity is64MiB minimum; smaller positive requests reject ResourceLimit
  (C# resource_limit) **before launch**, never silently round up.
- Metadata is a separate finite reservation.7z preflight charges16×raw/declared decoded header
  bytes for retained parsing buffers/state; retained entries are also checked. This conservative
  admission rule is not a measured RSS claim or a per-allocation allocator fence.
- Native process-commit ceiling=work+metadata+64MiB fixed runtime/IPC overhead. Defaults are
 512+256+64=832MiB, maximum1GiB,2active/aggregate2GiB. Explicit MemoryBytes must equal the
  derived ceiling; zero means derive. Invalid/unknown/duplicate budget fields reject
  invalid_budget; inconsistent MemoryBytes rejects budget_mismatch; aggregate/slot bounds stay.
- Rust and C# agree on these reservations. Public7z coder properties are checked before data
  decode; encoded-header StreamsInfo is wrapped into a bounded synthetic ordinary header and
  parsed with public Archive::read **without payload decode**, then dictionary/expanded-header
  budgets are checked before ArchiveReader::new. One decoder thread; LZMA dictionary/literal
  state, LZMA2 dictionary+4MiB conservative state, supported fixed-state coders1MiB each, summed
  per block. Unknown methods remain typed unsupported. No dependency patch or new dependency.
- Job limits are the final whole-process hard ceiling. A reservation is not falsely described
  as a separately measured allocator cap; uncooperative/OOM process exits remain failures, not
  fabricated codec ResourceLimit/native codes. Known worker ResourceLimit maps resource_limit;
  native failures retain their actual stage/code. Slot remains until cleanup.

Red, separately listed/executed: budget admission1/run1/pass0/fail1 exit1 (one-byte accepted);
actual7z working budget1/run1/pass0/fail1 exit1 (tiny LZMA completed under one-byte request).
Green: both1/1. Admission tests also check0invalid, Limits/MemoryBytes mismatch, default832MiB
read back from actual Job. Controlled dictionary/encoded-header128MiB declarations under64MiB
work reject before allocation; expanded encoded metadata over finite budget rejects. No unknown
compression bomb or unbounded stress. Controlled native fixture makes one VirtualAlloc request
148MiB in an actual144MiB Job; Windows refuses it with actual native memory code, typed
resource_limit, cleanup complete. Cancellation of two actual running children holds both slots
at post-termination/pre-disposal barrier; third admission fails, exit arrives only after release.
Existing suspended-admission/timeout/cancel and shared Spawn regressions remain passing.

## 3.7z structural classification — repaired

Root cause: next-header size-budget and invalid/overflow/EOF extent shared ResourceLimit branch.
`worker_codecs.rs` now validates start-header CRC and checked structural extent against EOF as
CorruptArchive, then applies finite metadata admission. No backend error-message matching.
Existing typed password/ambiguous authentication, cancellation and SourceChanged paths remain;
missing/wrong passwords and source-denial/cancel fixtures pass.

Red: exact actual C# Job→Rust fixture selected1/run1/pass0/skip0/fail1 exit1; past_eof,
valid_crc_bad_extent and overflow incorrectly returned ResourceLimit. After fix:1/1. The fixture
now executes9cases: past EOF; valid start CRC/illegal extent; offset+size overflow; truncated
header; valid extent/metadata deficit; normal; ordinary data dictionary deficit; encoded-header
dictionary deficit; decoded-header metadata deficit. Mutations recompute independent fixture
CRCs. One test-preparation error encoded an invalid variable integer and correctly got
CorruptArchive; corrected legal0xbfff declaration gets ResourceLimit. No assertion was weakened.

## 4. Encrypted exact-output evidence — strengthened, not universal compatibility

`ArchiveExtractionWorkerTests.cs` now checks independent exact regular-file counts, sizes and
SHA256 for encrypted7z,RAR4,RAR5; RAR4 also literal bytes. Independent7-Zip24.09 x64 extracted
all3inputs with exit0 in unique scratch directories; synthetic passwords via stdin, not argv.
Tool/source hashes and every expected file/hash are in
`rust/crates/nxtools-archive/tests/fixtures/upstream/README.md`. No NxTools output produced an
oracle. System libarchive3.7.7 encrypted reads failed; those were not used as expected values.

Encrypted7z has2files; encryptedRAR4/RAR5 each1 .gitignore, exact bytes `target\nCargo.lock\n`.
Corrupted encrypted7z/RAR4/RAR5 copies return AuthenticationOrDataError; failures retain unrelated
pre-existing sentinel bytes and publish no invalid final or partial. Correct password and
corruption cannot always be separated, so no guessed WrongPassword. Each fixture remains a
small-sample claim, not full encrypted-method coverage.

## 5. Atomic overwrite — still blocked; original contract/test preserved

No overwrite implementation changes, native flag experiments, target-sharing relaxation,
early target-handle release, check→replace race or backup/publish/restore implementation.
The only OutputNative.Open change is an opt-in read-only source-open branch; existing output
access/share defaults are unchanged. Exact original fixture remains registered/default-selected.
Final standalone list1/run0; execute selected1/run1/pass0/skip0/fail1, exit1,
**replace_file:32**. See [native gate](research/native-overwrite-gate.md) for earlier SDK/probe
basis. No additional unbounded native API combinations were tried. Non-atomic backup design
remains unapproved; it is not implemented or called an atomic solution.

## 6. Actual commands and results

All direct Rust commands inherited in the same PowerShell execution environment:
```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
```
Serial execution; full-operation timeouts1800–3600s, none timed out. Logs in session scratch
`archive-remediation-*.log`, plus source-red/green,budget-red/green,header-red/green logs.

| Command | Actual result / exit |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --offline |31passed/0failed/0ignored; exit0 |
| cargo build -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs --offline |exit0 |
| cargo clippy -p nxtools-archive --tests --features isolated-codecs --offline -- -D warnings |exit0 |
| cargo fmt --all -- --check |exit0 |
| cargo check --workspace --tests --offline |exit0; not full workspace tests |
| dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore |exit0,0warnings/0errors |
| Exact historical safe16 filters --fixtures --list |selected16/run0,exit0 |
| Same16 execution |selected16/run16/pass16/skip0/fail0,exit0 |
| New archive remediation filters --fixtures --list |selected6/run0,exit0 |
| Same6 execution |selected6/run6/pass6/skip0/fail0,exit0 |
| Final combined safe16+new6 after nonnumeric-budget/RAR5 additions (--list then execute) |selected22/run22/pass22/skip0/fail0,exit0; list selected22/run0 |
| Encrypted7z/RAR extraction exact filters (included in16) |selected2/run2/pass2/skip0/fail0,exit0; final7z10operations/RAR11operations |
| Atomic overwrite exact filter |selected1/run1/pass0/skip0/fail1,exit1,replace_file:32 |
| cargo test -p nxtools-extensions --test archive_import --offline |7passed/0failed/0ignored,exit0 |
| cargo test -p nxtools-clipboard --lib archive_ --offline |5passed/0failed/0ignored,39filtered,exit0 |

An initial extension command without `--test` selected0 tests (exit0); it is not counted as
verification and was corrected with the7-test target above. Intermediate combined directed
run failed on malformed test data (selected8/run6/pass5/fail1/not_run2); corrected9-case fixture
and final exact16+6 runs passed. No test skipped/excluded to hide the atomic red.

Reproduction:
```powershell
$env:NXTOOLS_ARCHIVE_WORKER_FIXTURE=(Resolve-Path target\debug\nxtools-archive-worker.exe).Path
$env:NXTOOLS_ARCHIVE_CODEC_FIXTURES=(Resolve-Path rust\crates\nxtools-archive\tests\fixtures\upstream).Path
$env:NXTOOLS_ARCHIVE_ZIP_FIXTURES=(Resolve-Path rust\crates\nxtools-archive\tests\fixtures).Path
$fixtureFilters=@('archive output fixture native failure','archive output fixture nested','archive output fixture existing','archive output fixture concurrent','archive worker','archive codec actual Rust','archive extraction actual Rust','extension process fixture owned echo','extension process fixture input and native process limits')
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list @fixtureFilters
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures @fixtureFilters
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list 'archive remediation'
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures 'archive remediation'
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list 'archive output fixture identity bound atomic overwrite'
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures 'archive output fixture identity bound atomic overwrite'
```

## 7. Remaining Phase A scope

Atomic overwrite remains an actual blocker. Also still required: ZIP extra methods/encryption;
TAR/compound/streams; creation/bounded source traversal; MOTW/time/attributes/Recycle Bin and
source-deletion identity; product IPC/capabilities/orchestration/backpressure/full diagnostics;
large metadata/RSS/performance,>=4GiB/UNC/long paths and broad codec safety/cancel coverage.
ENG01–09,SAFE01–08,P201/IPC01 are not accepted as complete scenarios. B–F retain their canonical
names/order and approved required scope; no B/UI work started. No full/default C# all-green,
UI/DPI/UIA/user/native Shell, Release/install or first-release acceptance claim.

Final gates (all exit0): `python tests/architecture/spec-contracts.py`100documents/14owners;
`python tests/architecture/gpui-component-catalog.py`79pages/79rows/78locked entries;
`python .trellis/scripts/task.py validate .trellis/tasks/10-16-archive-manager`16implement/13check,
no truncation warnings; `powershell -NoProfile -ExecutionPolicy Bypass -File
tests/architecture/platformhost-test-selection.ps1`; scoped `git diff --check` (only existing
Program.cs line-ending warning). No DEBUG probes were added. Final process check found0
Cargo/rustc/archive-worker/test-host/7z processes, fixed copied helper absent,0matching fixture
temp roots. Downloaded independent tools/scripts/logs remain solely in session scratch.
No full/default suite or UI/native/release acceptance follows from these gates.

Final C# rerun logs: `archive-remediation-last-build.log`, `archive-remediation-22-{list,tests}.log`.
The final combined22 excludes only the separately retained atomic FAIL; it is not the default
suite. No Rust product sources changed after the successful31-test/Clippy/workspace/fmt run.
