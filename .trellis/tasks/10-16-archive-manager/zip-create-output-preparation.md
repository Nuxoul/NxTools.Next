# ZIP creation — side-effect-free invalid-output preparation

Date: 2026-10-18. Current phase: **Phase A — Engine contracts and safety foundation**.
Status: repaired and integrated green; Phase A is not complete. Current status owner is
[phase-a-checkpoint.md](phase-a-checkpoint.md). Retain the independent
[remaining-side-effect rejection](zip-create-remediation-acceptance.md), original
[admission rejection](zip-create-acceptance-review.md), and historical82/75
[remediation results](zip-create-remediation.md); those passing tests did not disprove this bug.

## Boundary / cause

Read live AGENTS, current task/contract/checkpoint, archive/native/shared specs and source/test
owners. Rechecked tracked legacy NxTools archive terms: package/FFmpeg ZIP and image/video
owners only, no general archive interaction identified (not exhaustive untracked/generated
search). No new legacy entry/call chain was discovered. This is native preparation ordering,
not writer/crypto/read-engine/overwrite semantics.

Before repair Runner constructs OutputBridge before worker source_acquire. Its session Open
creates missing output directories. Source admission later rejects inside-directory output,
so zero commits and CleanupComplete=true did not mean the source member set was unchanged.
No old file deletion/content loss was reproduced or claimed.

## Actual red -> green

New `ArchiveSourceBindingTests.MissingInternalOutput` runs actual C# Start -> suspended
Job-owned Rust worker -> native source/output. Before Start it records source relative member
names and SHA256, proves both new-output and nested absent, and does not precreate output.

```text
before=[empty/,keep.txt:38EC21EB58DDF7EBB66697ACECDC212D53B5C0B6FC57092F1C096874B73F7262]
red after=[empty/,keep.txt:<same hash>,new-output/,new-output/nested/]
red typed=OutputCollision committed=0 cleanup=True output_created=True
red final ZIP absent; owned partial absent
red selected1/run1/pass0/skip0/fail1/not_run0, exit1

green after=[empty/,keep.txt:<same hash>]
green typed=OutputCollision committed=0 cleanup=True output_created=False
green final ZIP absent; owned partial absent
```

Red command (worker env points to target/debug/nxtools-archive-worker.exe):
`dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures "archive source binding fixture missing internal output"`.
Build0warnings/0errors and list selected1/run0 both exit0 before actual red.
Logs: `prepare-order-red-{build,list,test}.log` in session scratch.

## Cohesive new order and retained protections

1. ZIP bridge calls `ArchiveOutputSession.Prepare`: validate bounded local components, open
   actual drive and existing ancestor chain using relative no-reparse/non-share-delete opens,
   stop at first missing component, save only bounded missing suffix; no directory creation.
2. At actual worker source_acquire, original `ArchiveCreationSources.Open` acquires bounded
   source root chains and metadata and runs the existing `RejectOutputCollisions` against
   this same prepared output session. Lexical dangerous-direction rejection is supplemented
   by full volume/128-bit identity comparison against real existing output ancestors. Source
   eligibility/budgets/overlap/no-reparse failures occur before output creation.
3. Only after that success call `CreatePreparedRoot`. Create relative to continuously held
   actual ancestors. Before and after each component, check run cancellation and revalidate
   protected source root identity/version. Reparse/wrong-kind/raced components fail closed.
4. Repeat the SAME collision check against completed actual output root/final component.
   Continue existing Rust bounded traversal/held-directory-derived stable namespace, relative
   source reads, full partial verification and unchanged native publication.

No second runtime, duplicated source safety implementation, ordinary final-path writes,
recursive delete, source-set snapshot/CAS, system mapping mutation or overwrite redesign.
Admission Capture remains before accepted; source/read handles, finite roots/depth/enumerator/
records/budgets, confirmation snapshot and target result projection are unchanged. Existing
extract/Gzip/XZ retain Open behavior; Open reuses Prepare+CreatePreparedRoot.

## Legal missing directories / failure policy

Legal source-external missing roots are still created and independently readable ZIP output
succeeds. Source-child/output-parent plus confirmed sibling overwrite remains successful.
Precreation invalid placement/cancel/source failure creates no output directory. After lawful
creation begins, directories already created are deliberately retained on failure/cancel;
CleanupComplete describes native/process/partial resource cleanup, NOT a global filesystem
rollback. Only exact owned unpublished partial handles are cleaned, leases are closed. Never
remove preexisting/raced/replaced directories, external files, or another process's added
content; no directory ownership journal or generic filesystem transaction is introduced.

## Directed regressions / one review

Original source-binding4 retained; new5 harness entries cover:
- single source with missing internal two-level output, exact unchanged source list/hash;
- multi-source with internal missing output; synthetic harmless output-check path still
  rejected by actual held ancestor identity before creation (not strings-only proof);
- existing internal output with preexisting foreign content remains byte/member-identical;
- missing external output succeeds with independent .NET ZIP exact names/bytes/empty directory;
- real creation boundary cancellation before/after first directory, injected denied5,
  source member mutation with real source rename denied by held lease, and external file
  racing the missing directory component. Zero commits/no final/partial; external additions
  retained. Five interleavings in one harness entry are not counted as five tests.

Directed list selected9/run0; execution selected9/run9/pass9/skip0/fail0/not_run0,exit0.
One bounded read-only review of exactly four production files/new tests found no concrete
correctness/security regression. No repeat review or broad design restart.

## Integrated verification ledger

Every Rust command in same execution environment:
CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256. Inspected Cargo/rustc
ownership first; serial, no timeout/duplicate/build kill. Resources before large run:
C35.78GiB/D63.02GiB free, FreePhysicalMemory16896168KiB/FreeVirtualMemory17362732KiB.

| Actual command/check | Result |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --offline --no-fail-fast |82passed/0failed/0ignored,exit0,all result lines summed |
| cargo build -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs --offline |exit0 |
| cargo clippy -p nxtools-archive --features isolated-codecs --tests --offline -- -D warnings |exit0 |
| cargo fmt --all -- --check |exit0 |
| cargo check --workspace --tests --offline |exit0,NOT workspace tests |
| cargo test -p nxtools-extensions --test archive_import --offline |7passed/0failed/0ignored,exit0 |
| cargo test -p nxtools-clipboard --lib archive_ --offline |5passed/0failed/0ignored,39filtered,exit0 |
| dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore |product+test0warnings/0errors,exit0 |
| Exact original75+new5, same filters as historical checkpoint |list selected80/run0;execution selected80/run80/pass80/skip0/fail0/not_run0,exit0 |
| Actual4GiB+17 FastDeflate ZIP64 |PASS in final80,run exactly once this repair;Job/worker/native plus Python streaming exact length/SHA256 |

Final verification logs `prepare-order-final-{archive,worker,clippy,fmt,workspace,extensions,
clipboard,native-build,native-list,native-tests}.log`; exact commands/timestamps/exit codes
`prepare-order-verification-ledger.txt`. Native final run16:25:15–16:34:56(+08:00).

An earlier integrated attempt FAILED at AES independent oracle:selected80/run19/pass18/skip0/
fail1/not_run61,exit1;ZIP64 had NOT run. Root cause was scratch PowerShell script's UTF8-without-
BOM Chinese7z path read through Windows PowerShell default encoding:child Test-Path=false,
correct inherited process environment Test-Path=true. Product/test code unchanged. Exact AES
fixture then list1/run0 and run1/pass1/skip0/fail0,exit0. Fixed only scratch environment wiring,
then reran entire integrated batch above; retain `prepare-order-oracle-config-failed-ledger.txt`,
`prepare-order-oracle-config-failed-tests.log` and `prepare-order-oracle-config-retest.log`. No test removal/weakening/skip.

Independent tools: CPython3.13.2 SHA256
`7d96a4ed35d6e596cd9dd8933feafc66349cc21f75be3b15c89fd336e50140c1`;
existing7-Zip26.00 SHA256
`4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f` at historical explicit
portable path. Password only private stdin; no download/install/PATH change.

Final process inspection:0Cargo/rustc/worker/test-host/7z;fixed test helper absent;0source-binding
fixture roots. Fixtures remove only their own synthetic roots; no owned partial remains.
Documentation gates:spec-contracts101documents/14owners,catalog79pages/79rows/78locked,
task context32implement/29check,no truncation;fixture-selector source gate all exit0.
Scoped tracked git diff--check and task-file whitespace/DEBUG hygiene were checked separately.

## Remaining / continuation

No UI,release/package/install,real registry/association/service/SUBST/system mapping,user archive,
commit/push. Full/default C#suite,full workspace tests,ZIP64large-count/RSS/performance/UNC/long
paths/crash/native-user acceptance are not established. ZIP creation repair checkpoint is green,
not Phase A exit. Continue approved7z/TAR/TGZ/TXZ creation reusing sources/output,then remaining
native attributes/MOTW/source-delete eligibility/product IPC/resources. Never enter B early.
