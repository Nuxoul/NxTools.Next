# Phase A — approved rechecked atomic-path overwrite remediation

Date:2026-10-17. **Overwrite slice verified on this workstation; Phase A and A–F first release
remain NOT PASSED.** Scope is the explicitly approved target-overwrite amendment, native
publication/state/cancellation and related fixtures/docs. No UI, dependency, codec/source-lease
or budget redesign; no Release/install/registry/default-association/service action, commit/push.
Unrelated dirty work preserved. Initial process check found no project Cargo/rustc/test build;
Unity's unrelated dotnet process was not touched.

## 1. Approved contract and authority

User replaces strict confirmed-old-target-ID atomic compare-and-swap with:
**complete validated sibling temporary + immediate target recheck + one atomic path publication**.
The recheck-handle-close→native-replace TOCTOU window is expressly accepted. Do not claim the
confirmed file ID is an atomic replacement condition, absolute race freedom or power-loss safety.
No delete/truncate/old-target-backup-first. Parent/no-reparse/temp and source-lease protections,
full codec/solid integrity, budgets, cancellation and owned cleanup remain mandatory.

Approval synchronized **before production edits** into design§4.3, acceptance SAFE05–SAFE07,
PlatformHost native owner, phase-a-checkpoint and native-overwrite-gate; the reference comparison
records this later decision. Historical independent acceptance/remediation/native probe FAIL32
records are retained and marked historical/superseded requirement, never rewritten as past PASS.
Original fixture is renamed `ArchiveOutputTests.RecheckedAtomicPathOverwrite`, not deleted.

## 2. Root cause and implementation

Old code held the confirmed destination with share0 while renaming the verified temporary over
that name. Its own target handle caused `replace_file:32` on NTFS. New BeginFile verifies the
confirmed snapshot but does not retain a target lease; Commit reopens relative to the still-held
parent immediately before publish and compares volume/full128-bit fileID/size/mtime/SHA256.
Changed/missing/directory/reparse/busy/denied are typed recheck failures; a previously absent
no-replace target appearing returns target_exists80/183. Only the old-target recheck handle
closes. All source/ancestor/temporary protections remain. Arbitrary caller overwrite is not
inferred: C# local `ConfirmedOverwrites` maps exact relative paths to confirmed snapshots;
default remains no-replace, and this map is not serialized into the Rust protocol.

Core files:
- `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs`: target recheck, complete
  fullID, explicit commit states, single Win32 publication and exact cleanup rules.
- `ArchiveWorkerOutputBridge.cs`: local confirmed-overwrite capability, cancellation checkpoint,
  authoritative native publication count/state/native error.
- `ArchiveWorkerRunner.cs`: preserves FilesCommitted/CommitState in terminal event and metadata-only
  diagnostics; cancellation with publication is `cancelled_with_committed_output`.
- Existing `ArchiveOutputTests.cs` renamed fixture; new `ArchiveOverwriteTests.cs` and
  `ArchiveRenameAbiTests.cs`; Program/TestEffects exact reviewed registration.

No Rust product or private-wire schema changed. Rust still owns codec/integrity and invokes
native commit only after full verification. C# knows the actual native publication even when
cancel/disconnect/worker termination prevents Rust from consuming its acknowledgement.

## 3. Main API / SDK / bounded native evidence

Windows SDK10.0.26100.0 `um/minwinbase.h` and `um/winbase.h` checked:
- Win32 FILE_INFO_BY_HANDLE_CLASS FileRenameInfoEx=22; NT FileRenameInformationEx=65 is different.
- FILE_RENAME_FLAG_REPLACE_IF_EXISTS=1, POSIX_SEMANTICS=2.
- FILE_RENAME_INFO DWORD unionFlags,HANDLE RootDirectory,DWORD FileNameLength,WCHAR FileName[];
  x64 offsets0/8/16/20, computed from the managed sequential structure.
- FileIdInfo=18, FILE_ID_INFO uses64-bit volume and full128-bit identifier.

Main path: `SetFileInformationByHandle` on the exclusively held complete temporary, class22,
flags3. Derive destination from `GetFinalPathNameByHandleW(heldParent,VOLUME_NAME_GUID)` plus one
validated component, RootDirectoryNULL. Parent ancestor chain remains held/non-share-delete/
no-reparse; the temporary never closes/reopens before publication. This full-target form is not
MoveFileEx fallback and does not relax the temporary capability. No DELETE-first or backup move.

Two finite, discriminating self-owned probes (sources/results retained in session scratch):
1. Relative name+held RootDirectory returned87, relative+trailing NUL also87; full target/
   RootDirectoryNULL succeeded.87 is parameter failure, not capability absence. SDK docs currently
   describe relative RootDirectory, but the local Win32 wrapper's actual result is recorded.
2. Full counted target bytes followed by known `TAIL`: no WCHAR NUL returned success but
   published targetTAIL, leaving old bytes. Explicit NUL beforeTAIL replaced only the real target.
   Production buffer now allocates counted bytes+sizeof(WCHAR), explicitly writes zero; byte
   length excludes terminator. A checked-in ABI fixture independently demonstrates both cases.
   API FileName docs say NUL-terminated despite the length paragraph's no-NUL wording.

URLs and exact historical/source evidence: [native-overwrite-gate](research/native-overwrite-gate.md).
Source reference comparison was inspected, not proprietary/native reference runtime behavior.

**Compatibility:** no MoveFileExW fallback implemented. Only explicit native capability codes
1/50/120/124 map overwrite_not_supported.32sharing,5access and87invalid parameter are real errors,
never fallback triggers. Local NTFS primary path is verified; FAT/SMB/old Windows are **not tested**,
not falsely marked unsupported or supported. WRITE_THROUGH is not used as atomicity/durability proof.

## 4. State, postcommit failure, cancellation and cleanup

Preparing→Verified (full caller integrity+FlushFileBuffers)→CommitInFlight→Committed;
prepublication failures are FailedBeforeCommit. Final cancellation checkpoint is after target
recheck closes and before CommitInFlight. Once inside, native success wins: cancellation does
not undo/relabel published data as unchanged. C# terminal includes authoritative FilesCommitted
and CommitState even if the Rust worker is cancelled afterward; partial native output can coexist
with a cancelled job. Slot/source resources stay rooted until real process/pump/Job cleanup.

Immediately after native success set Committed/Replaced before closing the write handle or
postcheck. Observe final target fullID/size/SHA256, then save post-close final mtime for optional
unchanged-new-output cleanup. Windows may settle mtime only at last writer close; comparing
pre-close mtime caused a real directed regression and was fixed without dropping ID/content checks.

Postcheck API failure→CommittedButVerificationFailed, observed replacement/missing/type change→
TargetChangedAfterCommit. Neither means old target was preserved. No automatic retry/rollback,
no path delete; Cancel returns retained_published. Network-style uncertain native failure→
CommitOutcomeUnknown, retaining evidence and blocking writes/retry/partial delete.32/5known local
failure remains FailedBeforeCommit. Error category/state/native code are retained; injected64
covers conservative unknown policy, not a claim that SMB was exercised.

Precommit cancel removes only exact own partial, preserves prior target. After successful
replacement Cancel(cleanupNewOutput:true) retains replacement. Optional deletion only applies to
unchanged newly-created output after full identity/version/hash match. User same-name .nxpart,
other task partial and already-published/external outputs are preserved.

## 5. Red → green evidence and coverage

Before production fix, renamed original fixture (same normal-byte/stale-target/cancel assertions,
plus no-partial check) was listed and run: selected1/run1/pass0/skip0/fail1, exit1,
`replace_file:32`. Logs overwrite-red-build.log/overwrite-red.log. After fixing publication,
same fixture selected1/run1/pass1/skip0/fail0,exit0. No test removed/skip/default exclusion.

Intermediate real failures retained in logs:
- Initial relative Win32 call87; corrected to bounded-probe-verified full-target form.
- Missing WCHAR NUL returned success/wrong name under deterministic suffix probe; corrected
  explicit termination and checked-in ABI fixture. Earlier apparent POSIX occupancy successes
  were **not** accepted final results; after termination fix, real exclusive occupancy32 and
  original bytes are asserted.
- Postclose timestamp mismatch misclassified a successful commit; final ID/content check and
  settled mtime corrected it. All targeted cases rerun afterward.

| Fixture | Actual scenarios / boundary |
| --- | --- |
| RecheckedAtomicPathOverwrite | normal replacement bytes/no partial; stale confirmation rejection; cancellation cannot delete replacement; pending cancel preserves original |
| ChangesBeforeRecheck | new object at same name; same-size/same-mtime content edit; existing→missing; file→directory; file→owned junction; absent→appeared no-replace |
| FailuresAndNativeOccupation | bounded write failure; injected flush112 before publish; real exclusive target32 at native commit; real recheck-busy32; injected unsupported50/denied5/invalid87/unknown64; zero fallback/retry, original bytes, exact cleanup/evidence |
| PostCommitAndCancellation | cancel before/ready/in-flight/after; injected post-ID failure5; immediate external replacement; explicit accepted TOCTOU competitor overwritten; one call/no retry, parent/temp still protected, published cleanup retained |
| ExclusivePartialOwnership | user partial and other-task partial retained; successful unrelated publish retained; actual NT CREATE_NEW collision80/183 cannot truncate user partial |
| ActualWorkerOverwriteLifecycle |8actual Job→Rust7z→native operations: normal; damaged CRC; cancel ready/during/after; postcommit failure5; unsupported50; unknown64; immediate publication count before postcheck; exact bytes/count/state/native code |
| CountedNameMustStillBeTerminated | x64 SDK offsets; bounded raw Win32 nonterminated counted name publishes knownTAIL; explicit NUL publishes exact target |

Latest overwrite group: **selected7/run7/pass7/skip0/fail0**, exit0, list selected7/run0.
Existing16safe and6prior source/budget fixtures remain **16/16** and **6/6**, no skips. This
is a scoped set, not full/default C# suite acceptance. Native error/capability/flush faults are
injected only at their real prepublication boundary; real target occupation, target changes,
publication, CRC, cancellation and cleanup use actual Windows/worker operations.

## 6. Actual verification commands

All Rust commands serial in the same environment:
```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
```
No timeout or duplicate build. Logs `overwrite-final-*.log` in session scratch.

| Command | Actual result / exit |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --offline |31passed/0failed/0ignored,exit0 |
| cargo build -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs --offline |exit0 |
| cargo clippy -p nxtools-archive --tests --features isolated-codecs --offline -- -D warnings |exit0 |
| cargo fmt --all -- --check |exit0 |
| cargo check --workspace --tests --offline |exit0,not full workspace tests |
| cargo test -p nxtools-extensions --test archive_import --offline |7passed/0failed/0ignored,exit0 |
| cargo test -p nxtools-clipboard --lib archive_ --offline |5passed/0failed/0ignored,39filtered,exit0 |
| dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore |exit0,0warnings/0errors |
| historical safe16 exact filters --list then execute |list16/run0;selected16/run16/pass16/skip0/fail0,exit0 |
| archive remediation exact filter --list then execute |list6/run0;selected6/run6/pass6/skip0/fail0,exit0 |
| rechecked overwrite + archive overwrite exact filters --list then execute |list7/run0;selected7/run7/pass7/skip0/fail0,exit0 |
| Final integrated C# exact selection after bounded review fixes (--list then execute) |list selected29/run0;selected29/run29/pass29/skip0/fail0,exit0 |

Reproduce C# fixture inputs/selection:
```powershell
$env:NXTOOLS_ARCHIVE_WORKER_FIXTURE=(Resolve-Path target\debug\nxtools-archive-worker.exe).Path
$env:NXTOOLS_ARCHIVE_CODEC_FIXTURES=(Resolve-Path rust\crates\nxtools-archive\tests\fixtures\upstream).Path
$env:NXTOOLS_ARCHIVE_ZIP_FIXTURES=(Resolve-Path rust\crates\nxtools-archive\tests\fixtures).Path
$safe=@('archive output fixture native failure','archive output fixture nested','archive output fixture existing','archive output fixture concurrent','archive worker','archive codec actual Rust','archive extraction actual Rust','extension process fixture owned echo','extension process fixture input and native process limits')
$overwrite=@('archive output fixture rechecked','archive overwrite fixture')
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list @safe
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures @safe
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list 'archive remediation'
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures 'archive remediation'
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list @overwrite
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures @overwrite
```

## 7. Remaining scope / evidence boundary

No current local overwrite blocker under the amended contract. Strict identity-conditioned CAS
is intentionally not provided. Untested filesystems/old systems, power-loss/crash durability,
full storage/native user acceptance are not claimed. Confirmed overwrite is currently an internal
C# capability exercised through actual worker fixtures, not finished product conflict-dialog IPC/UI.

Phase A still needs required ZIP methods/encryption, TAR/compound/streams, creation/traversal,
MOTW/timestamps/attributes/Recycle Bin/source deletion policy, product IPC/capabilities/backpressure/
full diagnostics, large metadata/RSS/>=4GiB/long-path/UNC and broad codec safety/performance/cancel
acceptance. No B–F reorder or scope reduction. Full/default C# suite, UI/DPI/UIA/user/native Shell,
Release/install and first-release acceptance not performed/claimed.

## 8. One bounded review and final closure

One directed read-only review found publication count updated after postcheck, unsupported50
terminal collapsed to worker_failure, and insufficient exact end-to-end assertions. All three
were addressed without another review loop. Added real-worker native-committed boundary count
and full terminal state/code assertions (8scenarios). Before fix selected1/run1/pass0/fail1,
exit1 `actual publication count lost: normal`; after immediate idempotent native_published
token accounting the next red exposed `unsupported/worker_failure`,exit1. Preserving the
bridge's last commit category fixed it. Native fact now increments once **before** close/hash
and external checkpoints; result handling cannot double-count. Full final29run passed.
Logs: overwrite-review-red.log, overwrite-review-category-red.log,
overwrite-final29-{build,list,tests}.log. No second review was run.

Final gates all exit0: spec-contracts.py100documents/14owners; gpui-component-catalog.py79pages/
79rows/78locked entries; task.py validate17implement/14check, **no truncation warnings**;
platformhost-test-selection.ps1; scoped git diff --check (only pre-existing Program.cs line-ending
warning).20related text files:0whitespace/final-newline issues. design32671bytes and native owner
32522bytes remain below32768injection limit. Final process/helper check:0Cargo/rustc/archive
worker/test-host, fixed helper absent,0overwrite fixture temp roots. Probes/logs live only in
session scratch. No package/install/service/registry/commit/push or UI/native-user acceptance.
