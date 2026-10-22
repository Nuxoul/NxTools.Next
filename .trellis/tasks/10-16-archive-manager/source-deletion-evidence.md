# Source-delete qualification / protected recycle foundation — evidence

Date2026-10-19. Canonical **Phase A — Engine contracts and safety foundation**. Foundation
implemented and isolated checks green; actual native recycle remains deliberately refused.
This is not SAFE08/PhaseA/A–F/product/native-user/release acceptance.

## Scope, sources and implementation

Approved [internal contract](source-deletion-contract.md), [bounded native/API evidence and pending
choices](research/source-recycle-native-capability.md). Read archive reference -> legacy NxTools /
Archiver/SupaZip -> installed WindowsSDK10.0.26100.0 and Microsoft public API docs. No reusable
legacy filesystem recycle chain found; legacy clipboard history deletion is not filesystem deletion.
Public IFileOperation and real ITransferSource.RecycleItem both use IShellItem. No inspected public
HANDLE+full128ID/version-bound recycle contract. Source lease omits delete sharing and conflicts with
Shell move/delete access. Closing it creates unapproved source-delete TOCTOU, not the separately
approved output publication race. No claim about every undocumented/future/provider API.
Production backend stays absent -> native_protection_unproven + per-source NotAttempted, source
retained. No real recycle call (including own fixtures), permanent-delete fallback, bin-empty,
user archive, registry/association/service/drive mapping, package/install/commit/push operation.

Rust lifecycle facts:
- ArchiveEngine open/test/extract invalidates prior state. Full extraction records after original
  integrity/source/per-file publication and explicit directory finalization; selected-all IDs still
  selected-only, links/skips/empty/count mismatch refuse. No fact on CRC/metadata/MOTW/finalization/
  cancellation/partial/postcommit/unknown failure. RAR/7z complete after whole decode and final source.
- Creation records after complete held-partial structure/decode/hash readback, original revalidation,
  cancellation check and actual native publication acknowledgement, not encoder.finish. Original
  roots are separate from archive-relative names. Actual exclusions refuse; every directory root
  refuses whole-root permission, including stripped/empty/multi-root. No deletion of unarchived
  Thumbs.db/user exclusions/new members or substitution of descendant-file deletion semantics.
- Fixed private result operation_id/reason/source_ids, private optional consent/schema2 capability;
  old/absent remains no-delete. Rust fields cannot construct native authority or call Windows.
  Shared control consumes one grant with concurrent CAS/mutex semantics; test-only cannot reuse it.

C# boundary:
- Original admitted operation/source set retained. Single source captures same-handle fullID/size/
  mtime/ChangeTime/attrs; tree reuses original root records. No source sharing change, arbitrary
  worker path/parent/recursive authority. Full-success private terminal and IDs must match original
  capability, active temp provenance refuses. Qualification happens after actual process/pump drain,
  before source disposal. Source-version rechecked again immediately before recording backend.
- Original immutable request weak-key session prevents qualification twice; random token matches
  operation, one execution via CAS. Wrong token/job/concurrent/late replay cannot redispatch.
  Runner max2 retained operations plus global2STA execution slots; no timeout replacement workers.
  Cancellation before source starts yields NotAttempted; in-flight backend Cancelled preserved;
  unknown exception/ack result stops remaining sources and is never blindly retried.
- Per-source Recycled/Cancelled/Unsupported/Failed/OutcomeUnknown/NotAttempted + native codes are
  independent of archive success/count/state. Partial or failed recycle leaves published archive
  success/output intact. Injected backend is only internal test apphost construction; ForPackage
  cannot call Shell. No future UI/product IPC/queue/startup expansion.
- Correlation/qualification/accepted/start/terminal/elapsed/fixed reason/native/count only. No
  filename/path/content/token/credentials/raw backend exceptions; throwing sink is observational.

## Actual red -> green slices and bounded evidence

All commands/logs below are in this session scratch with source-delete-* prefix, not historical
102Rust/110CSharp baseline. Tests first at approved public/actual worker boundaries.

| Slice | Actual red | Actual green |
| --- | --- | --- |
| real complete public extract/default-off/once-only | missing source_delete_eligibility compile exit101;0tests executed |1pass/0fail/0ignored |
| extract then only test cannot reuse earlier success |3selected:2pass/1fail,actual eligible instead of lifecycle_incomplete,exit101 |3/3PASS after production open/test invalidation |
| actual ZIP creation private consent + verified partial |0pass/1fail,old parser rejected field/EOF,exit101 |same1PASS across8small file/directory/normal/readback/finish/sourceIO/ZIP64header scenarios |
| actual C# qualification/backend projection |missing interface4compiler errors,exit1;0tests executed |list1/run0 -> selected1/run1/pass1/skip0/fail0/not_run0 |
| legacy raw extract without metadata/MOTW authority |focused worker_wire0pass/1fail,8filtered,exit101 |new negotiation validation green;final wire10/10 |

Expanded Rust public suite5PASS covers default/off, full selected ID set, actual badCRC partial2,
metadata/commit failure, deterministic directory-finalization failure after3commits, cancel, changed
source, independent TAR link skip and8concurrent consumers exactly1grant/7already_consumed. These
five plus new worker-wire regression add6unique cases to historical102 -> final108, not duplicate
counts. Existing ZIP worker test is extended,not an extra new test group count.

New C#7entries include multiple small actual Start/Job/Rust/native transactions, not7native Shell
operations: consent null/false/true with zero/zero/once calls; directory/default exclusion/stripempty/
active temp/native refusal; original IDs/op/token mismatches; concurrent/late replay/unknown; native
metadata/MOTW/directory/postcommit/uncertain/cancel; RAR4/RAR5/7z positive whole extraction;
2source mixed results,cancel stops second,unknown stops second,lease denies writers throughout,
archive output success retained,log sentinels absent and throwing sink unchanged. Final focused
selected7/run7/pass7/skip0/fail0/not_run0;LIST names match. Recording backend cannot certify Shell.

Minimal actual no-Shell capability fixture: CreateFileW requests DELETE access only (no disposition)
while production ArchiveSourceLease is live -> sharing32. Final held identity verified,lease closes;
ordinary owned rename preserves original then new source path is written. Independent identity
read observes different fullID and exact original/replacement bytes both remain. No Shell operation,
recycle call or source permanent-delete is used by this probe; ordinary fixture cleanup is confined
to its GUID-owned directory.

Preserved non-product/intermediate failures:
- Test nullable CS8602 compile refusal,then explicit throw-if-null fixed test only;0tests had run.
- Initial codec qualification probe overlapped default Cargo test relinking shared worker; it failed
  archive_not_fully_succeeded and is NOT a valid codec red. Corrected execution is serial after
  isolated worker build;all three attributed codec positive transactions pass. No product failure
  or source protection relaxation is inferred from that invalid probe.
- External wallhaven Cargo lanes were waited,never killed/duplicated. Every direct wrapper inherited
  CARGO_BUILD_JOBS1/CARGO_INCREMENTAL0/RUSTFLAGS=-C codegen-units=256 and checked global lane.
- PowerShell inline regex quotation preflight failed parsing;no build/test ran. Scratch script fixed
  it. Final JSON parser wrapper initially @(...ConvertFrom-Json) nested117names as oneelement;
  stopped before LIST/execution/ZIP64. Existing Rust/build results retained,only C#tail continued
  after confirmed117parse. Not a passed initial wrapper or another ZIP64 attempt.
- ArchiveWorkerRunner.cs hit3EDIT_LINES_UNSEEN errors cumulatively (last unseen anchor49),after
  successful wiring edits. Per editing rule,parent stopped editing that path for the turn;no shell
  bypass/delegate edit. The already-written runner stayed intact and compiled/tested. One-shot
  qualification session strengthening stayed in its dedicated owner,not a runner path workaround.

## Final integration — actually run once

2026-10-19 11:06:31.790 -> 11:19:58.264(+08:00). Exact command/step timestamp/exit ledger:
`source-delete-final-ledger.txt`; scripts `source-delete-final-verify.ps1` and
`source-delete-final-csharp-continue.ps1`; logs `source-delete-final-*.log`. All product commands
below exit0,full-operation timeouts,actual terminal statuses. Wrapper selection failure above is
preserved separately. No code changes after this integration unless explicitly recorded later.

| Gate | Actual result |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --offline --no-fail-fast -- --test-threads=1 |108pass/0fail/0ignored across23harness result groups (two empty groups not tests) |
| same-feature actual worker build |exit0;final binary rebuilt after feature-unifying commands |
| default + isolated strict archive Clippy --tests --offline -- -D warnings |both exit0 |
| cargo fmt --all -- --check |exit0 |
| cargo check --workspace --tests --offline |exit0;NOT workspace tests |
| extensions --test archive_import |7pass/0fail/0ignored |
| clipboard --lib archive_ |5pass/0fail/0ignored,39filtered |
| C# test/product build --no-restore |0warnings/0errors |
| original110names union new7LIST |selected117/run0/pass0/skip0/fail0/not_run117 |
| SAME exact execution |selected117/run117/pass117/skip0/fail0/not_run0 |
| LIST/RUN/PASS exactnames |0differences;baseline captured before edits,0removed/duplicates |
| actual4GiB+17FastDeflate ZIP64 |RUN1/PASS1,once only final117;realJob/native output+Pythonstream size/SHA256 |

Original TARtime,typed-I/O,MOTW,finaldirectory,publication/cleanup and output-preparation cases
retained,not replaced. Two existing extension-process compatibility entries retained.7Zip26.00
SHA2564bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f;Python3.13.2
SHA2567d96a4ed35d6e596cd9dd8933feafc66349cc21f75be3b15c89fd336e50140c1 reverified this turn.
No installation/PATH mutation. Prelarge Cfree28.2GB/Dfree51.1GB;ZIP64 marker enforces one attempt.

## Remaining boundary and stop

Actual native recycling remains **NOT AUTHORIZED / NOT EXECUTED / NOT ACCEPTED**. User decision
needed before implementing any source recheck-close->Shell TOCTOU amendment; no such approval is
inferred. Preserve strict contract -> retain source is current behavior. Recycle-only capability
would still need separately authorized native/provider tests. No permanent option/fallback was
implemented. Directory source complete-coverage/snapshot proof is deliberately not claimed.

Product IPC/helper package mapping/FIFO/backpressure/conflicts and broad resources/performance/
largecount/UNC/longpaths/native-user/fullPhaseA exits remain. No B/UI/A-F/release completion claim.
One bounded read-only review disposition and final static/hygiene results are appended below.

## Final static gates and hygiene

Actual four requested gates exit0: spec-contracts105documents/14owners; GPUIcatalog79pages/
79rows/78locked; taskvalidate49implement/46check,no truncation;PlatformHostselector sourcegate.
Reference32746bytes/checkpoint31673bytes remain under32768. Scoped33code/docs files pass
whitespace/conflict/temporaryDEBUG checks. Program/TestEffects each differ from preturnsnapshot
by exactly7new ArchiveSourceDeletionTests registrations,none removed/changed. Gitdiff--check
actualexit0;existing unrelated CRLF normalization warnings do not represent an error.
31archivefixture-rootprefixes inspected,0remaining;fixedhelper absent;postintegration inspection
found0archiveworker/testhost/7z/ownbuild. ExternalCargo/rustc reappeared later and was left alone;
source/commandownership inspection,not process-name killing. No cleanup/reset of unrelated dirty
work. Session scratch scripts/API extracts/red/failure/final logs are retained as evidence.
No functionality or test code changed after final108/117integration;only owner/reference/task
documentation was synchronized. Native recycling remains unapproved/unexecuted and fail-closed.

## Sole optional read-only review

One bounded code-reviewer was launched after final integration. It remained read-only and did
not return a findings report within the bounded wait;parent cancelled it,termination confirmed.
This is **not a completed/passed independent review** and no second review was launched. The
native/API research was a separate source investigation,not a substitute review. Existing actual
108/117test/build/lint evidence stands at its stated level;real native recycle remains pending.

Final repeat after README/review-disposition sync: all4staticgates exit0 with same105/79/49/46
counts;scoped34files hygienePASS(includingREADME),Gitdiff--check0,31fixtureprefixes0remaining,
fixedhelperabsent,ownbuild/worker/testhost/7z0. ObservedCargo/rustcbelongs to external
D:\Code\wallhaven/wh-desktop,untouched. Exact log:source-delete-final-static-hygiene.log.

## Later independent findings — historical evidence boundary

[Independent acceptance](source-deletion-acceptance.md) reproduced P1newsource/cloneauthority and
P2stale lifecyclecompletion despite historical108/117green. The weakrequest-key qualification
description above is historical defective implementation,not current assurance. Original failure
report remains unchanged;[bounded repair/current evidence](source-deletion-remediation.md) replaces
that owner with original privateRun/session and same-lifecycle-onlyfinish. Current state only in
phase-a-checkpoint.md. Nativecontract remains separately pending,no actualrecycling authorized.
