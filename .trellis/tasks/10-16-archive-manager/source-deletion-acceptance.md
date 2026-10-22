# Independent acceptance — source-delete qualification foundation

Date:2026-10-19. Canonical **Phase A — Engine contracts and safety foundation**.
Scope: lifecycle qualification and isolated original-object authorization only. No real recycle
or user-source deletion was authorized/executed. Product/test source was not modified by acceptance.

## Verdict

**Not accepted yet: one P1 authorization foundation defect and one P2 lifecycle defect reproduced.**
Existing full integration is green at108Rust/117C#. That does not cover the two independent
interleavings below. Production native backend remains absent and fail-closed; these probes do NOT
establish actual recycling of user files, a public IPC exploit or unsafe native Shell execution.
Native recycle original-object/recycle-only protection remains a separate documented pending
contract, not a new defect or permission to accept close->Shell TOCTOU.

## P1 — old worker evidence can authorize a newly captured replacement and be replayed

Locations: `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveSourceDeletion.cs:29,38–43,45–68`;
`csharp/NxTools.PlatformHost.Tests/ArchiveSourceDeletionTests.cs:101–126` currently tests the same
reopening pattern without replacing the object, so misses the original-object violation.

Sessions uses ConditionalWeakTable keyed by request reference, not the original private Run or a
source-acquisition session. Qualify accepts caller-provided originalSources and archiveSucceeded;
it compares evidence IDs to IDs on those newly supplied capabilities, but evidence does not bind
the original source version. ArchiveSourceLease.RecycleSource accepts an ID to associate with its
current captured object. A cloned request creates another session with no consumed-state link.

Independent scratch C# probe:
1. Actual ForPackage Start/Job/Rust/native full ZIP extraction with DeleteSource=true succeeds;
   production result is native_protection_unproven, with original source retained.
2. Wait for terminal/drain and lease disposal. Move the owned original aside and create a new
   unrelated regular file at exactly the same name. It was never archived.
3. Open a NEW lease on replacement; assign the old operation ID via RecycleSource, pass old
   complete-frame evidence plus original caller request to production Qualify.
4. Authorization is eligible; ExecuteAsync invokes a recording backend on replacement capability.
5. Repeat with `request with {}`: another eligible authorization invokes backend a second time.

Observed (source/output are synthetic literals; no backend filesystem mutation):
```json
{"archive":"success","originalReason":"native_protection_unproven","qualification":"eligible","result":"completed","backendCalls":1,"replacement":"replacement not archived","output":"original archived payload"}
{"archive":"success","originalReason":"native_protection_unproven","qualification":"eligible","result":"completed","backendCalls":2,"replacement":"replacement not archived","output":"original archived payload"}
```

Current production runner uses its own admitted request and retains leases through its execution,
which limits immediate reachability. Nevertheless the NEW authorization owner is not an
original-object/once-only boundary as contracted. This matters before any native backend is enabled.
The finding is about the tested internal authority API, not an asserted untrusted external caller.

Required repair: bind a privately created operation authorization session to the original source
capabilities/version at acquisition and to its actual completion owner. Worker JSON/old evidence,
a request clone, freshly supplied capabilities or caller `true` must not mint a new grant. Retain
one qualification/consumption state through original Run lifetime; after retirement refuse reuse.
Do not solve only with a process-wide unbounded ID set or weaken source sharing. Add regression
from actual production Run/session, replacement-after-drain, clone/replay/late/cross-job attempts,
assert zero extra backend calls and unchanged replacement. Keep normal once-only/mixed outcomes.

## P2 — delayed completion overwrites a newer lifecycle invalidation

Locations: `rust/crates/nxtools-archive/src/source_deletion.rs:42–49`,
`control.rs:68–82`, `engine.rs:456–479`.
Evidence.begin resets state but returns no generation; finish unconditionally writes eligible.
OperationControl is cloneable and public ArchiveOutput callback can perform another lifecycle
with its shared clone. Existing mutex serializes individual state edits, not lifecycle ownership.

Independent scratch Rust probe used public API + existing small deflated.zip and ArchiveOutput:
- full real extraction decodes and commits3 files;
- during first directory-finalization callback, execute a successful ArchiveRequest::test on the
  same control clone;
- immediately after test, qualification is lifecycle_incomplete as intended;
- return from callback: older extraction finish overwrites it and qualification becomes eligible.

Actual output:
```text
AFTER_TEST=lifecycle_incomplete
EXTRACT=Ok(ExtractOutcome { files_committed: 3, bytes_committed: 275, skipped_special: 0 }) INJECTED=true COMMITS=3 FINAL_QUALIFICATION=eligible
```

This deterministic reentrant ordering models the same delayed-completion race as two threads;
no sleep/racy fixture needed. Reran against a scratch copy of PROJECT Cargo.lock, same result.
First probe dependency resolution differed in some versions, so the project-lock rerun is the
controlling reproduction; project lock/source were never edited.

Required repair: generation-bound begin/finish ownership (or reject overlapping lifecycle use)
so a completion cannot restore facts invalidated by a later open/test/extract. Apply consistently
to every qualification-producing path. Keep operation-wide one-shot consumption and cancellation.
Add deterministic callback/barrier regression for test/open/mismatched failed later operation,
plus sequential valid flow. Do not remove invalidation or only add a mutex around finish.

One bounded Rust read-only reviewer proposed P2; parent independently read implementation and
reproduced it. Parent independently found/reproduced P1. No second review loop was launched.

## Independently executed integration

Scratch script source-delete-acceptance-verify.ps1 and logs/ledger source-delete-acceptance-*.
Foreground serial batch12:36:49->12:47:45(+08:00), every step exit0, no timeout/interruption.
All Cargo inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256;
machine-wide lane checked before each command. C# fixture sources inspected for recording-only
backend and no Shell recycle. Dedicated no-Shell DELETE-access/reresolution fixture is permitted.

| Gate | Actual independent result |
| --- | --- |
| focused source_deletion_contract |5pass/0fail/0ignored |
| full isolated-codecs archive,offline,no-fail-fast,serial |108pass/0fail/0ignored across23groups |
| same-feature worker build |exit0 |
| isolated/default strict archive Clippy including tests |both exit0 |
| fmt --all --check |exit0 |
| workspace check --tests --offline |exit0, NOT workspace tests |
| extensions --test archive_import |7pass/0fail/0ignored |
| clipboard --lib archive_ |5pass/0fail/0ignored |
| C# product/test build |0warnings/0errors |
| original110+new7 list |selected117/run0/pass0/skip0/fail0/not_run117 |
| same exact execution |selected117/run117/pass117/skip0/fail0/not_run0 |
| LIST/RUN/PASS names |0differences |
| actual4GiB+17FastDeflateZIP64 |RUN1/PASS1, once only this acceptance |
| original missing-internal-output probe |OutputCollision/0commits/output_created=false/unchanged source members |
| spec gate |PASS105documents/14owners |
| component catalog |PASS79pages/79rows/78locked |
| task context |PASS49implement/46check |
| PlatformHost selector source gate |PASS |
| git diff --check |exit0;existing unrelated CRLF normalization warnings only |
| scoped source/test whitespace/conflict/DEBUG hygiene |PASS |

The focused5 are included in108. Scenario counts within7C#entries are not extra harness tests.
Oracle environment uses explicit existing7-Zip26.00 hash
4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f and local Python.
No installation/download/PATH change. Independent C# probe is source-delete-review-probe;
Rust probe is source-delete-rust-probe (project-lock log probe-project-lock.log). Both are in
session scratch; no project tests were silently patched to obtain red/green.

## Native decision and cleanup

Inspected source/API record distinguishes IFileOperation/RecycleItem IShellItem contracts from
HANDLE/full128ID/version binding. Strict no-delete-share leases and unapproved close->Shell gap
remain unresolved. Current null backend returns NotAttempted/native_protection_unproven, which is
correct fail-closed behavior at this authorized stopping point. This report neither authorizes
nor claims real recycle/permanent delete, nor declares directory coverage solved.

Both probes used owned synthetic data and recording outputs only. C# original/replacement tree
cleaned in finally;recording backend never changed files. Rust ArchiveOutput probe writes no files.
Postprobe0Cargo/rustc/archive-worker/test-host/7z/probe processes;fixed product-test/probe helper
absent;0inspected source-delete/recycle-race/independent/tar/metadata/tree/source-binding roots.
C26.2GiB/D47.1GiB free. Scratch sources/logs/build intermediates retained. Unrelated dirty state kept.

No real recycling/user source deletion,package/install/commit/push,registry/association/service/
drive-map changes. Full workspace tests,unrestrictedC#,UI/native-user/release acceptance not run.
Remaining PhaseA:fix the two foundation defects,decide/verify actual guarded recycle and directory
coverage,productIPC/helpermapping/FIFO/backpressure/conflicts and broad resources/native exits.
No PhaseB/A–F completion claim.
