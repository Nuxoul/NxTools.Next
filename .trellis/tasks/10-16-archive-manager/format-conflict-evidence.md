# 7z/RAR/TAR conflict integration — scoped evidence

2026-10-20. **Implemented, scoped checks PASS; independent acceptance pending.** Still
Phase A — Engine contracts and safety foundation; NOT fullB2/PhaseA/A–F completion.
[Verified sources/lifecycle](research/format-conflict-sources.md); current state only in
[checkpoint](phase-a-checkpoint.md). Prior ZIP/password/risk/readmission and C1/C2/C3/C5
independent acceptances are preserved, not reimplemented or rerun as full matrices.

## Delivered scope and boundary

| Format | Integration / actual evidence |
| --- | --- |
|7z|Shared Ask/Overwrite/Skip/AutoRename/apply-all in real for_each_entries admission. Native tiny solid3member input: two conflicts Skip/apply-all asks once, both old files retained, third member exact bytes published; waitcancel stops later outputs.|
|RAR4/5|Same policy before original second-pass RAR_TEST. Both versions actual Ask→Overwrite and explicit Skip: exact independent literal bytes, correct commit/skip/warning counts, members_skipped refusal.|
|TAR|Existing planned-file loop uses begin_extracted_file; native AutoRename exact old/newbytes, Skip2/zero commits, single-member no-replace race, internalcase/unsafe/prefix refusal. Domain Skip asserts0partials/0committedbytes/members_skipped.|
|TAR.GZ/BZ2/XZ/Zstd and supported aliases|Same TAR inner-output path integrated after existing complete outer-integrity spool. This slice's native representative is TGZ AutoRename; other wrappers are source/compiled shared-path integration, NOT a fresh per-wrapper×strategy native matrix. Existing accepted reads/integrity remain historical evidence.|
|Standalone Gzip/XZ/BZip2/Zstd|NOT extended. Opt-in rejected; absent option retains old behavior. No password extraction retry/risk/readmission expansion.|

Small necessary shared rename ZipConflict→ExtractionConflict; same strategy/tickets/native calls,
no registry/scheduler/framework or second transaction. PrivateOutput installs original policy and
shared immutable entries Arc (reserved names) and emits original extraction_summary. Preserve
zip_conflict_v1 marker; additive extraction_conflict_v1 is required by new host. Old omitted policy
emits no new conflict capability/summary and original no-replace/preconfirmed behavior remains.
Private schema2 extract-only/read_contract refusal unchanged; no ArchiveRead write authority.

7z fully reads skipped member, counts ALL actual decoded bytes and verifies CRC/size, preserving
solid decoder state. RAR Skip leaves token absent but still executes RAR_TEST (not RAR_SKIP),
with unchanged per-entry/total callback budgets/native integrity. Native handle closes before
moving pending ownership. TAR bounded copy consumes skipped payload to sink under original limits.
Output file/byte counts exclude skipped tokens; test continues returning full decoded counts.
All original preflight/path/internal collision rules precede disk prompt. No fabricated ratio for
unknown packed_size; TAR still has no format payload checksum.

Full decode/source checks still precede ANY pending file publication in7z/RAR/TAR. Keep source,
Job, helper and native-output ownership; Ask holds original slot, typed Cancel wakes wait. Existing
C#single observation token/full target signature and BeginFile/Commit recheck-close→one atomic-path
publication unchanged. AutoRename retains numbering/reserved names/no-replace; racing new file
survives typed OutputCollision, no retry. Cancel retains true native commits and only cleans owned
unpublished partials; no rollback/external-file deletion. Same Rust task-local apply-all/oldticket
logic; C#does not decide policy. Skip explicitly feeds source eligibility members_skipped.
Qualification tests use a never-called recording backend ONLY; no Shell/recycle/source deletion.
Diagnostics reuse original awaiting_conflict/conflict_finished/native terminal correlation/count/
elapsed/native codes, no payload/path/member/byte-tick logs or temporary production probes.

## Real red → green

Before production edits, actual original runner/Job/currentworker new fixture failed:
`real format must enter existing AwaitingConflict: solid_skip`.
`format-conflict-red-list/run.log`: selected1/run1/pass0/skip0/fail1/not_run0,exit1.
C#apphost first0warnings/errors, matchingisolatedworker then LIST/RUN. Same real test now green.
No mock/preconfirmed dictionary was substituted for Ask.

`ArchiveFormatConflictTests.ActualFormatConflicts` executes13tiny scenarios:
solid_skip,solid_cancel,rar4_overwrite,rar5_overwrite,rar4_skip,rar5_skip,tar_rename,tgz_rename,
tar_skip,tar_race,collision,unsafe,prefix. Checks exact old/newbytes, native FilesCommitted,
FilesSkipped/category, extraction_summary/all_members_written, source-delete members_skipped,
zero backend calls, one apply-all prompt, previous-job and retired tickets, wait-no-partial/write,
source_precommit after solid consumption, foreignpartial retention/ownpartial cleanup, actual
exited→DrainAsync/ActiveCount0/source exclusive reopen. Old target never used as a corruption oracle.

Independent inputs generated in GUID-owned temporary tree by CPython3.13.2 tarfile (USTAR/TGZ)
and existing stock7-Zip26.00x64. Solid command:
`7z a -t7z -m0=LZMA2 -ms=on -mhc=off solid.7z a.txt b.txt c.txt`;
stock `l -slt` proves Solid=+/Blocks=1/3members; payloads are literal incoming-a/b/c.txt,14bytes each.
No self-authored format encoder. Existing upstream RAR4 VERSION=`unrar-0.4.0`11bytes;
RAR5 unicodefilename❤️.txt=`foobar\n`7bytes, independent oracles in upstream README/manifest.
No RAR producer/version claim beyond retained upstream attribution; no compressor invoked for RAR.

## Executed checks and exact collection

Every Cargo uses jobs1/incremental0/RUSTFLAGS=-C codegen-units=256,offline/serial,checked machine
lane. C#isolated apphost built first,matchingisolatedworker rebuilt last after lint before native
RUN. No unrelatedprocess killed, no fullworkspace suite/old ZIP16matrix/largeinput executed.

| Check | Actual final result |
| --- | --- |
|C#Debug apphost --no-restore|exit0,0warnings/errors|
|Rust affected5targets|47PASS,0fail/ignored,exit0: interaction4,source_deletion9,stream12,transaction5,worker_wire17|
|StrictClippy isolated/default --tests --examples --offline -- -D warnings|both exit0|
|Archive scoped fmt --check /matchingisolatedworker build|both exit0|
|C#exact LIST→same-setRUN|selected7/run7/pass7/skip0/fail0/not_run0,exits0|
|spec-contracts/taskcontext/fixtureselector/gitdiff--check|PASS; source/document gates,not UI/release acceptance|

Exact C#catalog order:
1. archive conflict fixture actual 7z RAR TAR decisions
2. archive output fixture rechecked atomic path overwrite
3. archive outcome fixture actual worker operation isolation
4. archive stream fixture actual worker TAR extraction safety
5. archive extraction actual Rust handle-session fixture
6. archive extraction actual Rust 7z handle-session fixture
7. archive extraction actual Rust RAR handle-session fixture

The6existing representatives preserve original native target checking/isolation, no-policy ZIP/
7z/RAR/TAR extraction and small corruption/limit refusals. No need to replay ZIP16interaction or
password/risk/readmission matrices. Rust worker_wire additionally checks new+old capability marker,
standalone refusal/legacy omission and unchanged read-only boundary; stream Skip asserts qualification.

An earlier final native collection was6PASS/1environmentSKIP because WindowsPowerShell read a
UTF8-noBOM script's Chinese7Zip path incorrectly. That is NOT PASS; original logs retained as
format-conflict-environment-skip-*. Correct explicit verified oracle path inherited from parent,
then SAME7LIST/RUN final7PASS. Local compile-preparation fixes are not extra product-red claims.

## Evidence and preservation / stop

Scratch: C:\Users\Administrator\.pi-desktop\scratch\949da288-1ef9-46b3-a698-90f543613cf5.
format-conflict-verify.ps1, command-results.json, red/green/final/environment-skip logs and opening
hashes preserve exact commands/exits/names/counts. Final closing assertions check124pre-existing
historical taskfiles unchanged, prior checkpoint snapshot byte hash, allowed changed paths,
new summary/capability and no owned archiveprocess/helper/pipes. Original results/reports/ledger
and Failed/Stopped facts retained. No independent acceptance or broad reviewer claim.

No standalone-streamconflicts/password/risk/readmission/newruntime/UI/FIFO/PhaseB/UNC,
100k/largefiles/longpathmatrix/performance,sourceRecycle/delete/system/dependencychange,
formalapp/package/install/sign/commit/push. Stop for independent scoped acceptance, no nextfeature.
