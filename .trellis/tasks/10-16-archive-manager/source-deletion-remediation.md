# Repair evidence — original source session and lifecycle-bound qualification

Date2026-10-19. Canonical phase: **Phase A — Engine contracts and safety foundation**.
Scope only the P1/P2 in [independent acceptance](source-deletion-acceptance.md). That report is
unchanged, SHA25686072db3c9206645bbde645c6c5a23a77bf87d243c8a49f9f249758c78c72b27.
Historical108Rust/117CSharp did not cover these defects and is NOT this repair's verification.
No native backend/UI/productIPC/queue/directory-coverage-proof or dependency changes.

## P1 — private production Run owns original source authority

### Root cause and smallest owner change

The old ConditionalWeakTable keyed caller/request references;static Qualify accepted worker JSON,
caller archiveSucceeded=true and fresh source capabilities. A new lease could assign old operation
ID to a replacement;request clone created another weak-key session. Existing test reopened after
real completion without replacing source,so tested the defective factory pattern instead of the
original production authorization boundary. Category B cross-layer authority contract + D test gap
+ E implicit assumption: matching IDs/request reference are not original source-object authority.
The earlier weak-key patch blocked only same-reference repetition;it could never establish
original Run/source lifetime. No process-global unbounded spent-ID set is a repair.

Implementation now lives in `ArchiveWorkerRunner.SourceDeletion.cs` as a **private nested**
RecycleSession in the runner partial. No externally callable qualification/constructor/factory.
- At actual source_acquire,original private Run captures original C#source capability array and
  fixed backend/log before acknowledgement. Single held source version and tree original root
  records are unchanged;no terminal reopen/replacement/new sources and no source sharing change.
- Complete accepts only that private Run and actual resolved terminal,not caller request/bool/
  JSON/source set. It reads Run's already-bound terminal evidence and original sources;success,
  consent,temps,operation/finite exactIDs/fullversion all checked. Captured -> Qualified or Refused
  happens once. All refusal paths cannot later qualify. No weak-table reference authorization.
- Owner ExecuteOwnedAsync is the sole first execution. Test observer IArchiveRecycleAttempt exposes
  ID/token/rejection response only,never qualification/source/backend mutation. Qualified observer
  returns owner_required and cannot steal first execution. Matching original session locks once;
  InFlight/Completed reject repeats,Retired is irreversible. Wrongjob/token never consumes permission.
- Original runner awaits actual isolated execution,retire clears bound source references BEFORE
  source Dispose/slot retirement. In-flight backend retains original leases/runner slot and global
  max2STA slots. No replacement timeout threads,recycle retry,permanent fallback or Shell call.
- Old static ArchiveRecycleAuthorization/factory/ConditionalWeakTable removed. This signature
  narrowing is structural prevention,not by itself behavior proof;actual regressions below drive
  original production Run/session and record zero extra backend calls.

### Actual original P1 red -> green

Project regression RetiredEvidenceCannotBindReplacement was first added against old API:
actual ForPackage Start->Job->worker->native extract success/DeleteSource=true ->
native_protection_unproven -> real drain/original lease released -> preserve original aside ->
write unrelated replacement -> newlease.RecycleSource(oldoperation) -> old complete JSON + original
caller request and request with{} -> record backend. **selected1/run1/pass0/skip0/fail1/not_run0,
exit1,actual calls=2 versus required0**. Log source-delete-repair-p1-red.log. Compiler passed with
0warnings/0errors;failure is real behavioral symptom,not missing interface/0tests.

Green keeps the original real operation/drain/replacement/old evidence/clone attack,then exercises
original captured production session observer,never an after-the-fact authority:
- null production backend:original NotAttempted/native_protection_unproven,calls0;retired old/new
  request,JSON Write and session replay refuse,calls remain0.
- recording backend:legal original success exactly1;after drain/replacement/newlease/clone+JSON+
  original retired-session attempts,calls remain1. Original bytes,never-archived replacement,
  published extracted bytes and foreign .nxpart bytes all unchanged.
- OnceOnlyUnknownAndBinding rewritten in place,not removed:captured original session not_qualified;
  same instance at qualified;wrongjob/token rejected;8concurrent observations during actual backend
  -> already_consumed;clone Start rejects;backend throws -> OutcomeUnknown once;after terminal
  session retired and no retry. finally barrier always releases owned recording backend.
- OriginalSessionRejectsChangedEvidence retains old operation/source-ID refusal coverage through
  actual worker terminal input seam enabled only in current test-apphost ForRecycleFixture.
  wrongoperation,wrongsource,duplicateID,extrafield return fixed refusal/0backend while archive
  success and source/output remain. Observer after refusal/drain cannot revive session.
- Existing consent/default/failure/cancel,whole directory/exclusions/stripempty/temp/native refusal,
  codecs,mixed outcomes,lease writer refusal,diagnostic privacy and throwing-sink cases retained.

First green C#8/8;enhanced final targeted9/9,0skip/fail/not_run,LIST9/RUN9/PASS9.
New2entries add to original117;OnceOnlyUnknownAndBinding remains same registered case with all
original semantic scenarios moved to real session/worker-evidence boundaries. No fake Shell claim.

## P2 — finish must belong to the current lifecycle

### Root cause and complete propagation

Evidence.begin cleared facts but returned no ownership;finish wrote unconditionally. A mutex
serialized writes,not whole lifecycle. A clone's later open/test/failedextract could invalidate,
then older extraction finalized a directory callback and restored eligible. Category E implicit
lifecycle assumption + B state ownership + D deterministic-interleaving test gap.

Begin now returns private non-cloneable Lifecycle(Arc<()>). Evidence keeps only current Arc;every
begin clears facts/replaces identity without resetting consumed. Finish receives original marker,
compares Arc identity under same state lock,and only current marker may record,then retires marker.
Old marker stays allocated while older lifecycle runs,so no wrapping counter/ABA reuse or forgery
through operationUUID. SourceDeleteEligibility remains output-only,not native authority.

All production sites audited/applied:
- engine.open/test/extract begin before validation/I/O,including failed open/foreign plan/source
  revision failure;extract carries its marker through metadata/finaldirectory to finish.
- worker dispatcher invalidates before source_acquire. It has no final fact. Normal internal
  open->extract/test is legal SEQUENTIAL ownership;only final extract marker can grant.
- isolated RAR/7z begin once per run before budget/source/decoder;record_extraction carries same
  marker after full integrity/pending publication/finaldirectories/final source fingerprint.
- single Gzip/XZ,ZIP,7zcreate_with_reader,TAR/TGZ/TXZ begin at implementation entry before options/
  budgets/I/O;after original verified readback/source/native publish record same marker.
  CreationPlan projects original roots and requires caller marker,never begins a new one at finish.
- consumed remains operation-wide,cancel denial unchanged;selected/skips/link/empty/partial/
  metadata/MOTW/postcommit/unknown failures never become full eligible success. No codec/native
  publication/output TOCTOU/error-policy changes.

### Actual original P2 red -> green

New project public test late_extraction_finish_cannot_restore_facts_invalidated_by_reentrant_test:
actual deflated.zip open/plan/extract,3actual file commits,first real directory-finalization
ArchiveOutput callback calls successful ArchiveRequest::test on same control clone. Inside callback
qualification lifecycle_incomplete;after original extract returns actual eligible before repair.
**run1/pass0/fail1/ignored0,5filtered,exit101**,log source-delete-repair-p2-red.log. No sleep/random
schedule,no synthetic boolean facts. After marker repair SAME test and original5 ->6/6PASS.

Added deterministic callback matrix (4newRusttests total,not scenario-added counts):
- later test success;later open success/open missing failure;later extract foreignplan or changed
  source identity failure -> incomplete inside callback AND after oldfinish,3prior commits retained.
- older selected-only extraction calls later full successful extract;newer eligible wins instead
  of oldselected overwriting it. Consume once,subsequent valid extraction/test/cancel stay consumed.
- cancellation during finalization cannot be undone by oldfinish.
- original full sequential success,defaultfalse,selected/skips/link/metadata/CRCpartial/source
  mutation,cancel and8consumers exactly1grant remain in the9case public suite.
Focused public9/9 plus single/ZIP/treecreationworker3/3PASS (12total),0failed/ignored.

## Prevention and bounded review

| Priority | Mechanism | Completed action |
| --- | --- | --- |
| P0 | Structural C# authority | Private nested originalRun/session;no static/new-source qualification factory or weakrequest keys |
| P0 | Runtime lifecycle | Captured/Qualified/Refused/InFlight/Completed/Retired;owner-only execute;retire before original lease disposal |
| P0 | Typed Rust ownership | Every begin returns private marker;finish signature requires original marker across all fact-producing paths |
| P0 | Real boundary tests | Production session replacement/clone/retire;actual callback test/open/failedextract interleavings;zero extra calls |
| P1 | Owner code-spec | Exact session/marker signatures,state/error/test contracts updated,not abstract generic guidance |

One bounded read-only code-reviewer (12toolcalls,no code writes) reviewed only P1/P2 and returned
**no concrete in-scope findings**. It checked originalRun capture/ownercomplete/execution/retire,
lease ordering,old factory removal and Rust marker sites/tests. Static review did NOT independently
execute tests;nativeShell and codec algorithms outside scope. No second/broad review loop.
No expansion to unrelated modules/templates,Trellis runtime,UI or extra speculative features.

## Final integrated verification — this repair, not historical baseline

Actual foreground serial batch2026-10-19 **13:21:50.535 -> 13:33:14.303(+08:00)**,all steps exit0,
no timeout/interruption/unreturned wrapper. Script source-delete-repair-final-verify.ps1,ledger
source-delete-repair-final-ledger.txt,logs source-delete-repair-final-*.log in session scratch.
Every Cargo set same-process CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256;
globalCargo/rustc lane checked before each command. External wallhaven wh-desktop builds observed/
waited,never killed or duplicated. C#actualworker execution serial after finalisolated binary build.

| Gate | Actual result |
| --- | --- |
| archive full isolated-codecs --offline --no-fail-fast -- --test-threads=1 |112pass/0fail/0ignored across23resultgroups;twoemptygroups not tests |
| same-feature actual worker build |exit0;final binary rebuilt after all feature-unifying commands |
| default/isolated strict archive Clippy including tests --offline -- -D warnings |both exit0 |
| fmt --all --check |exit0 |
| workspace check --tests --offline |exit0;NOT workspace tests |
| extensions --test archive_import |7pass/0fail/0ignored |
| clipboard --lib archive_ |5pass/0fail/0ignored,39filtered |
| C#product/test build --no-restore |0warnings/0errors |
| exact original117+new2 LIST |selected119/run0/pass0/skip0/fail0/not_run119 |
| SAME exact execution |selected119/run119/pass119/skip0/fail0/not_run0 |
| LIST/RUN/PASSname comparison |0differences;original117preserved,2new,0duplicate |
| actual4GiB+17FastDeflateZIP64 |RUN1/PASS1,only final119 this repair;realJob/nativepublication+independent Pythonstream size/SHA256 |

Original TARtime,typed-I/O,MOTW,finaldirectory,publication/cleanup,outputpreparation and2extension
processcases retained. New4Rustpubliccases are included in112,not additional to it;new2C#entries
contain several small transactions,not extra harness counts. No unrestrictedC#/workspace tests.
Independent existing7-Zip26.00 reverified SHA2564bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f;
Python3.13.2 SHA2567d96a4ed35d6e596cd9dd8933feafc66349cc21f75be3b15c89fd336e50140c1.
Prelarge Cfree~28GB/Dfree~49GB;owned ZIP64 attempt marker prevents repeat. No local-loop bigZIP64.
No compiler/test/environment failure beyond actual P1/P2 red was counted as repaired behavior;
one runner Edit range collision was safely rejected and corrected after classification. Two
intermediate method/fixture code assembly mistakes were caught by source inspection before
compilation/execution and corrected;not product test reds. No temporaryDEBUG probes or lint allows.

## Native decision / stopping boundary

P1/P2 code defects and nativeAPI limitations are separate. Production null backend still returns
native_protection_unproven/NotAttempted. No actual Shell recycle/source permanentdelete (even owned
fixture source),no lease weakening/close->Shell race approval,no permanentfallback/blindretry/bin
emptying. Only GUID-owned fixture creation/rename/replacement and ordinary finally cleanup;
recordingbackend never mutates files. No userarchives,package/install/commit/push,registry/
association/service/drivemapping/UI/productIPC/queue/directorycoverageproof operations.
Full PhaseA/A–F release/native-user/performance/resource exits remain open. Stop after these two
repairs and final static/hygiene evidence;do not enter PhaseB or next feature.

## Final static gates / preservation / cleanup

All four requested gates actually exit0: spec-contracts105documents/14owners;componentcatalog
79unique pages/79table rows/78locked entries;taskvalidate51implement/48check,no truncation;
PlatformHostfixtureselector sourcegatePASS. Reference32746bytes/checkpoint~31800bytes remain
below32768. Scoped27code/test/docs files pass whitespace/conflict/temporaryDEBUG checks.
Program/TestEffects differ from preturnsnapshot by exactly2new registrations,0old removals or
renames. Existing OnceOnlyUnknownAndBinding keeps name/delegate and scenarios,now production
session-owned rather than after-the-fact factory. Independent acceptance SHA256 unchanged.
Gitdiff--checkactualexit0;unrelated existingCRLFnormalization warnings are not product failures.
31archivefixture-rootprefixes inspected,0remaining;fixedtesthelperabsent;postbatchobserved0
Cargo/rustc/archiveworker/testhost/7z. All self-owned fixture data cleaned by finally;source
snapshots/scripts/red/green/review/final logs retained in scratch,not user data cleanup.
No code/test changes after final112/119integration;only contracts/evidence/context/checkpoint
documentation synchronized. Large ZIP64 remained exactlyoneRUN/PASS. Originalfailure records
and unrelateddirty changes preserved;no package/install/commit/push or nextfeature started.
