# Windows local long paths — real-chain implementation evidence

Date: 2026-10-20. **Long-path regressions complete; pending independent acceptance.**
Canonical **Phase A — Engine contracts and safety foundation** remains incomplete. This closes
only the tested local ordinary-drive ZIP conditions of ENG08 and related SAFE01/03/05/06/07,
not those complete requirements, UNC/full formats/UI/Release/native-user/A–F.
[Contract](windows-long-path-contract.md); [result ledger](windows-long-path-results.json).

## Scope and reference-first findings

Read AGENTS, owner/shared specs, canonical plan, acceptance scenarios, current checkpoint,
[accepted100k observation](product-100k-post-status-acceptance.md), and reference note before code.
One read-only reference sweep followed recorded legacy NxTools, Archiver and SupaZip sources;
new verified entry files/call chains and limits were added to
[archive reference note](../../../docs/reference-code/archive.md#windows-long-path-source-follow-up-2026-10-20)
BEFORE test implementation. No general legacy archive manager was identified; package/FFmpeg
ZIP use and references' ordinary File paths/direct final writes do not establish long-path or
safe publication behavior. No reference executable/long-path runtime/Bandizip reverse engineering.

Opening SHA256 baseline covers362 scoped files, plus tracked binary diff and full status in
session scratch `long-path-opening-baseline.json`, `long-path-opening.diff` and `long-path-opening-status.txt`.
No production implementation defect was demonstrated in the tested legal long paths. Thus
**production Rust/C# policy/native/IPC code and dependencies are unchanged**. Added4 C# fixture
registrations/effects and one Rust actual-product test; existing Python ZIP oracle helper only
changed private->internal visibility for reuse. No third-party, resource-tool or UI changes.

## Actual paths and original boundaries

All new product/runner requests are ordinary local `C:\...` DOS paths, not public `\\?\`/UNC/
volume-GUID/device authority. Characters are ASCII and CJK (BMP); lengths below are UTF-16 code
units, with UTF-8 lengths retained separately in raw evidence. Every component <=255; actual
constructed components47 units, finite depth remains below native64. No truncation/sanitization.

| Boundary | Actual length / facts |
| --- | --- |
| Product open/test source ZIP |446 UTF16 /512 UTF8bytes; repeated CJK+ASCII47unit components |
| Native extraction source ZIP |432 UTF16 /498 UTF8bytes |
| Extraction output root |424 layout;428–439 publication matrix |
| Archive member relative path |342 UTF16,7nested components plus Unicode filename |
| Published extraction path |767 layout;771–782 publication matrix |
| Native creation source root /payload file |423 /766 UTF16 |
| Creation output root /ZIP final |427–438 /436–447 UTF16 |
| Internal creation enumeration |held-directory-derived `\\?\Volume{...}`;9enumerations per operation, identity checked by original code |

Product test `windows_long_source_path_uses_original_product_open_test_and_cleanup` uses original
ArchiveReadClient -> PlatformBridge original named pipe -> isolated TEST ArchiveProductHost
(single connect) -> production ArchiveReadSession -> original runner suspended process/Job ->
source_acquire/ArchiveSourceLease -> actual isolated-codecs Rust worker. Open compares3literal
members, ordinal/size/UTF8provenance/codepage/empty warnings/same real snapshot/Zip and2batches;
test verifies2files/12bytes. Both owner terminals equal original production remote terminal
files, success/nativeNull/cleanuptrue. Same connection generation persists across both requests;
source bytes unchanged and writable after cleanup; exact helper removable, original endpoint
retired, pipe EOF followed by host normal exit. No installed host/Desktop/global adapter.

Extraction/create use ArchiveWorkerRunner.ForPackage and actual worker (not a fake success
worker), existing private acknowledged native bridge and original source/output session.
ArchiveSourceLease opens drive then each component via no-reparse NtCreateFile; creation opens
protected ID sources, derives VOLUME_NAME_GUID through GetFinalPathNameByHandleW and rechecks
actual enumerated directory identity. Rust owns relative-name policy, plan/integrity/full encoded
partial readback/source recheck. C# owns held partials/metadata/FlushFileBuffers and publication:
new outputs NtSetInformationFile(relative no-replace); overwrite SetFileInformationByHandle
FileRenameInfoEx22/flags3, full name from held parent plus explicit NUL. No extra file runtime.

## Independent oracles, publication and failure facts

.NET ZipArchive independently produces tiny Store ZIP inputs from fixed literals, not Next's
writer. Extraction checks exact file AND directory sets,37literal payload bytes, empty file /
explicit empty directory and independent native DOS-calendar mtime2020-02-03 04:05:06. Source ZIP
SHA256 stays unchanged; source lease is released after terminal. Default source MOTW path remains
active, but no new source ADS capability claim is inferred from an absent stream.

Creation uses a long source tree with10expected records (7ancestor directories, explicit empty
directory, empty file,37byte payload), StripSingleRoot=true and original Deflate/normal. Existing
CPython verify-created-zip.py checks exact original expected names/kinds/sizes/UTF8flags, full
stream SHA256 and selected methods on every published result. Source structure and bytes remain
exact after every case. No contents/paths are diagnostic events.

Each extraction and creation publication fixture executes8small cases:

| Mode | Actual terminal /publication | Preservation |
| --- | --- | --- |
| new |success /Committed /1 |exact new payload or independently verified ZIP |
| approved overwrite |success /Committed /1 |original fullID/version/hash confirmation, one atomic path replace |
| existing unapproved |worker_failure /FailedBeforeCommit /0 |previous target bytes unchanged |
| changed at before_recheck |target_changed /FailedBeforeCommit /0 |external changed bytes retained |
| cancel at target_rechecked |cancelled /FailedBeforeCommit /0 |previous target unchanged |
| injected flush_file/native5 at before_flush |worker_failure /FailedBeforeCommit /0 |previous target unchanged; injection, not observed OS denial |
| cancel at commit_in_flight |cancelled_with_committed_output /Committed /1 |native publication retained, never relabeled zero |
| injected inspect_identity/native5 after native_committed |committed_but_verification_failed /CommittedButVerificationFailed /1 |published output retained with exact native5 |

Every native_published barrier asserts InspectOutputCommit.FilesCommitted==1 immediately; terminal
count and observed output agree. Deterministic fault/cancel barriers are reached once. External
`.nxpart-external` remains exact, all current owned partials gone; cleanuptrue, ActiveCount0 and
original worker/pumps/Job/leases drained. No uncertain output deletion, retry, backup-first or
fallback. Approved target-close->publish TOCTOU remains accepted, not an atomic target-ID CAS.
No system LongPathsEnabled/registry/policy/drive mapping change or source recycle/delete API.

UnsafeMembers runs8actual worker extractions at a long destination: traversal, root-absolute,
drive-absolute, drive-relative, UNC, device/verbatim, ADS and reserved-device. Exact worker
UnsafePath, zero commits, empty destination and outside sentinel unchanged.4internal/public
prefixes separately fail source/output authority checks; Rust product admission likewise rejects
those prefixes before dispatch. Supporting legal long paths does not relax unsafe members.
Diagnostics assertions compare same-correlation terminal/log category/native/count/state/elapsed,
check unescaped string values for source/member/content sentinels and bound lifecycle records.
Only counts/categories/IDs/elapsed/native codes are saved as diagnostic facts.

## Preserved first failures and test-infrastructure corrections

Do NOT present these as production long-path red->green fixes:

1. First C# LIST4/RUN3/PASS2/FAIL1/not_run1/exit1: create returned ResourceLimit, nativeNull,
   zero commits/cleanuptrue. New test erroneously set metadata exactly4MiB, equal to creation's
   existing fixed4MiB matcher/queue reservation before any member. Corrected ONLY the new test's
   explicit limit to16MiB; product defaults/limits unchanged. Original failure root retained.
2. One intermediate C# build exit1/oneerror: adjacent-line edit duplicated Limits; corrected the
   new test declaration. Original build log retained; final build0warnings/0errors.
3. Second C# RUN3/PASS2/FAIL1/not_run1/exit1: actual ZIP creation succeeded/1commit/cleanuptrue,
   but external CPython ordinary long-path oracle returned FileNotFoundError. Read-only saved-ZIP
   probe confirmed same10entries pass with oracle-only extended path. New test adapts ONLY the
   read-only external oracle path to `\\?\`; product/source/output requests stay ordinary DOS.
   No registry change or public-prefix widening. Original published ZIP/failure evidence retained.
4. Rust initial compile exit101 on wrong warning field in NEW test; corrected to released
   warning_codes. No product operation executed in that compile failure.
5. Next Rust test exit101/1failed: long open/test and owner/remote/source/helper assertions passed,
   but aborting listener did not close spawned connection, causing new5s host-wait assertion.
   Existing endpoint.set_connected_for_contract_test(false) now retires original connection after
   operations, followed by actual pipe EOF/normal host exit. No production shutdown change.
   Failed source and both remote terminals retained; no long-path-evidence file falsely marks it PASS.
6. A scratch selector syntax error occurred before tests; fixed script only. A proposed AES
   regression was removed BEFORE LIST/RUN because no stock oracle existed at checked locations;
   no dependency/format change needs it. No skipped test is counted as passed.
7. A background verification launch left only an incomplete compile log, no exit ledger; exact
   driver/Cargo/rustc absence was confirmed before the foreground equivalent. Cause unknown,
   not PASS. No interrupted result overwritten. One unrelated machine-wide Cargo workspace run
   was observed and allowed to finish before starting ours; never killed or counted as this slice.

## Actual verification

C# first built isolated apphost, then Cargo built matching isolated-codecs actual worker. Final
C# apphost build exit0/0warnings/0errors. All direct/background Rust environments inherited
CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256; no worker execution overlapped
our relinking. Exact commands/exits/timestamps in scratch logs and result ledger.

- New4C#fixtures LIST then RUN4/4PASS,0skip/fail. Final union of new4 plus19affected existing
  native-output/overwrite/smallproduct/plainZIP/metadata cases: **selected23/run23/pass23/skip0/
  fail0/not_run0**, exact LIST=RUN=PASS names, exits0. This is NOT all C#fixtures.
- New long-source product targeted1/1PASS/0ignored,0.63s; final product_read_chain22/22PASS.
- Affected archive9integrationgroups: **62PASS/0fail/0ignored**, one existing100k test deliberately
  filtered out. Includes creation_plan4,engine13,extraction_plan5,metadata_wire1,product_chain22,
  transaction4,worker_extraction1,worker_wire11,zip_creation1. NOT full archive/workspace tests.
- Protocol53/IPC30,Extensions archive_import7,Clipboard archive_5: all0fail/ignored,exit0.
- Strict Clippy archive tests+examples isolated/default both exit0 with -Dwarnings.
- cargo fmt --all -- --check exit0; cargo check --workspace --tests --offline exit0
  (**compilation only, NOT workspace tests**).

No new100k/1GiB/4GiB+17/performance sample; no resource-tool suite/optimization, third-party change,
UI/FIFO/nextformat, formalDesktop/installedPlatformHost, package/sign/install/commit/push/systemchange.
New small long-path inputs, failed and passed outputs/terminal records are retained for independent
inspection. Cleanup releases resources and removes exact self-copied helper only; not a recursive
sweep or source-delete acceptance. Historical accepted100k/newold raw files remain unchanged.

## Closing gates and remaining acceptance

Offline retained-file verifier:95checks PASS,17owned roots/186rawfiles including failures retained;
85historical taskfiles and accepted100k9new+12older resource files hash-identical. Unrelated tracked
dirty diff matches opening baseline. Final resources:0archivehost/worker/probe,0testpipes,0fixedhelpers.
One optional bounded read-only review was requested but stopped after it exceeded its intended
scope/time without a completed report. **No review PASS or independent acceptance is claimed.**
No second review was started; production remains unchanged and actual tests/static gates above pass.
Documentation gates passed spec106docs/14owners,context77implement/74checkentries,fixtureselector,
gitdiff--check. New reference detail moved into its linked research note to keep main index32654bytes
below32768; existing native-contracts35497byte injection warning remains. Git CRLF warnings retained.
Result ledger and scratch closing-hygiene retain exact hashes/scope. Independent slice acceptance
is pending. NativeRecycle remains **native_protection_unproven/NotAttempted**. Stop; no next feature.
