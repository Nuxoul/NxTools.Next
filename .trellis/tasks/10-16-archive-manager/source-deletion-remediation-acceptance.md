# Independent acceptance — source-delete P1/P2 remediation

Date:2026-10-19. Canonical **Phase A — Engine contracts and safety foundation**.
Scope: original Run/source authorization and lifecycle-bound Rust qualification only.
Original [independent findings](source-deletion-acceptance.md) remain unchanged; implementation
history is in [repair evidence](source-deletion-remediation.md).

## Verdict

**PASS for both original findings.** P1 original-object/replay authority and P2 delayed-finish
qualification are closed at the bounded source/isolated test level. No concrete new blocker was
found in this focused acceptance. This does not accept real native recycling, directory coverage,
Phase A completion, Phase B or A–F/release/user acceptance. No real recycle or source deletion
was performed. Product/project-test sources were not modified; only this report is added.
No additional subagent review was used in this acceptance.

## P1 — original production session now owns authority

Inspected ArchiveWorkerRunner.SourceDeletion.cs and runner capture/complete/retire wiring,
source capabilities, updated original tests and two new fixtures.
- Static Qualify factory and ConditionalWeakTable are gone. Session is a private nested type,
  created by original private Run at source_acquire with fixed original sources/backend/log.
- Complete reads that Run's actual terminal/evidence, checks owner identity, success/consent/
  finite exact IDs/version and transitions once; refused cannot requalify.
- Observer accepts only job/token and returns rejection/owner_required. It cannot supply evidence,
  sources, backend or archiveSucceeded, nor steal the first execution and release leases early.
- Owner awaits actual execution, then retires before source disposal/operation-slot retirement.
  Retained observer cannot revive the session. Unknown/cancel/mixed results remain independent
  of successful archive output; production backend remains null/fail-closed.

Old scratch attack cannot compile against the removed factory. That is structural evidence only,
not the controlling behavior proof. A NEW scratch probe independently exercised the current
actual Run/session: capture -> qualified observer denial -> owner execution -> drain -> original
source moved aside -> never-archived replacement/new lease -> old request and clone/session/JSON
attempts. It asserted original/replacement/output/foreign-partial bytes were preserved.

Actual independent probe results:
```json
{"recording":false,"Category":"success","deletion":"native_protection_unproven","replay":["retired","retired"],"backendCalls":0,"preserved":true}
{"recording":true,"Category":"success","deletion":"completed","replay":["retired","retired"],"backendCalls":1,"preserved":true}
```
Recording backend verifies capability and returns a synthetic result without modifying files.
Legal original execution occurs exactly once; replacement/clone attempts add zero calls.
Project fixtures additionally pass captured/qualified/in-flight/retired, wrong job/token,
concurrent/late/unknown and tampered original worker evidence scenarios through production seams.

## P2 — original unchanged public-API probe no longer restores old qualification

Inspected Evidence/OperationControl, engine entry/finish and all worker/codec/creation record sites.
Every begin replaces a private Arc identity and clears facts; finish requires its original
non-cloneable Lifecycle marker and compares current identity under the state lock. Older marker
cannot overwrite later invalidation or a newer valid result. Begin does not reset consumed.
Worker normal open->extract is sequential, with only final extraction marker producing facts.

Reran the ORIGINAL scratch Rust probe source unchanged, using a scratch copy of PROJECT Cargo.lock:
real deflated.zip decode/3commits, successful later test during directory-finalization callback,
then earlier extraction returns. Observed:
```text
AFTER_TEST=lifecycle_incomplete
EXTRACT=Ok(ExtractOutcome { files_committed: 3, bytes_committed: 275, skipped_special: 0 }) INJECTED=true COMMITS=3 FINAL_QUALIFICATION=lifecycle_incomplete
```
This was eligible before repair. Current deterministic callback/barrier project tests also cover
later open success/failure, failed extraction plan/identity, newer valid extraction, sequential
success, operation-wide consumed state and cancellation. No sleep-based race or fabricated facts.

## Independently executed checks

Final foreground serial batch13:46:27->13:57:25(+08:00), all steps exit0/no timeout.
Scratch exact script/ledger/logs: source-delete-repair-acceptance-verify.ps1,
source-delete-repair-acceptance-ledger.txt,source-delete-repair-acceptance-*.log.
Every Cargo inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256;
machine-wide Cargo/rustc checked before each invocation. One earlier guard detected occupancy
BEFORE any test and stopped; retained blocked-ledger.txt. Reinspection showed empty lane, ownership
not established, nothing killed and no duplicate build. That guard ran no ZIP64.

| Gate | Actual independent result |
| --- | --- |
| focused public source_deletion_contract |9pass/0fail/0ignored |
| full isolated-codecs archive,offline,no-fail-fast,serial |112pass/0fail/0ignored across23groups |
| same-feature actual worker build |exit0 |
| isolated/default strict archive Clippy including tests |both exit0 |
| fmt --all --check |exit0 |
| workspace check --tests --offline |exit0; NOT workspace tests |
| extensions --test archive_import |7pass/0fail/0ignored |
| clipboard --lib archive_ |5pass/0fail/0ignored |
| C# product/test build |0warnings/0errors |
| original117+new2 exact list |selected119/run0/pass0/skip0/fail0/not_run119 |
| same exact execution |selected119/run119/pass119/skip0/fail0/not_run0 |
| LIST/RUN/PASS name comparison |0differences |
| actual4GiB+17FastDeflateZIP64 |RUN1/PASS1,once only this acceptance |
| independent current original-session replacement/clone probe |retired/zero extra calls;normal0or1 stays0or1 |
| unchanged independent Rust reentrant-test probe |final lifecycle_incomplete;earlier extraction still3commits |
| independent missing-inside-output probe |OutputCollision/0commits/output_created=false/unchanged sources |
| spec gate |PASS105documents/14owners |
| component catalog |PASS79pages/79rows/78locked |
| task context |PASS51implement/48check |
| PlatformHost selector source gate |PASS |
| git diff --check |exit0;existing unrelated CRLF normalization warnings only |
| scoped source/test whitespace/conflict/DEBUG hygiene |PASS |

Focused9 are included in112,not additional unique tests. C# scenario operations are not harness
counts. Original117 names are retained alongside new2; old OnceOnlyUnknownAndBinding retains
its registered scenario while using the corrected production owner. Existing archive/TAR/MOTW/
metadata/source/output/cancel/cleanup regressions remain in119.
Explicit local7-Zip26.00 SHA256 matched
4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f.
No oracle download/install/PATH change. Project lock/source untouched by scratch Rust probing.

## Cleanup, preservation and native pending boundary

Postprobe0Cargo/rustc/archiveworker/testhost/7z/probe processes;fixed product/probe helpers absent;
0inspected source-delete/recycle-race/independent/tar/metadata/tree/source-binding fixture roots.
C26.0GiB/D45.8GiB free. C# synthetic tree cleaned in finally;Rust probe writes no output files.
Scratch sources/logs/build intermediates retained. Original acceptance SHA256 unchanged:
86072db3c9206645bbde645c6c5a23a77bf87d243c8a49f9f249758c78c72b27.
Extensive unrelated dirty changes preserved;no product fixes made during acceptance.

Production null backend remains native_protection_unproven/NotAttempted. Strict original-object
Shell recycle and never-permanent guarantees remain separately undecided/unverified; directory
roots/exclusions still conservatively refuse whole-root recycling. No lease weakening or source
close->Shell TOCTOU approval is inferred from this acceptance.
No actual recycle/permanent-source-delete,user archives,package/install/commit/push,registry/
association/service/drive-map changes. Full workspace tests,unrestrictedC#,UI/native-user/release
acceptance not performed. Remaining Phase A native contract/acceptance,directory coverage,
productIPC/helper mapping/FIFO/backpressure/conflicts and broad resources/performance exits stay open.
