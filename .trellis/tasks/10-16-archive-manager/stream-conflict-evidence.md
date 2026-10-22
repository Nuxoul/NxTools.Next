# Standalone Gzip/XZ/BZip2/Zstd conflicts — scoped evidence

2026-10-21. **Implemented; scoped checks PASS; independent acceptance pending.** This closes the
standalone-stream connection to existing ordinary-file conflict handling, not fullB2/PhaseA/A–F.
[Inspected source/contract](research/stream-conflict-sources.md),
[prior format-conflict independent PASS](format-conflict-acceptance.md). Only
[phase-a-checkpoint.md](phase-a-checkpoint.md) owns current state and full extraction coverage table.

## Minimal implementation / invariant

Each original format extract calls existing begin_extracted_file instead of begin_file. Shared
ExtractionConflict/InteractionPort/PrivateOutput/native target observation and publication are reused,
no StreamConflict/protocol/framework/codec refactor. Existing extraction_conflict_v1+ZIP marker and
schema2extract-only/ArchiveRead read-only constraints stay. Worker admits only the additional four
known standalone formats. Original omitted-policy path still defaults to begin_file/no-replace.

Present token: unchanged fixed-buffer stream -> full format end/applicable integrity and actual-byte
checks -> source revision/checkpoint -> original metadata when supported -> native commit. Error
aborts only own token. No token (Skip): SAME full stream to sink, same limits/end/CRC/checksum/source
checks; no partial/write/metadata/commit/abort. Successful Skip returns0publishedfiles/bytes,
files_skipped1/all_members_writtenfalse/succeeded_with_warnings and members_skipped. CRC/actual-limit
failure is terminal, never a successful skipped archive. Unknown expanded size remains None; no
new ratio/risk/budget or filename/path policy. Single output reuses typed apply-all response without
new state. Existing decoder integrity guarantees are not enlarged (Zstd checksums only when supplied).

Original C#runner/Job/sourcelease/helper/slot lifetime, one target-observation token/full identity,
recheck-close→single atomic-path overwrite, bounded numbered no-replace, cancellation and fixed
correlated diagnostics unchanged. No C#production code changed, no private payload/path logging.
No output rollback,source recycle/delete,UI/FIFO/password/risk/readmission expansion or dependency.

## Actual real red → green

New `ArchiveStreamConflictTests.ActualStandaloneConflicts` ran BEFORE production edits through
original runner/Job/currentworker and failed:
`standalone stream must enter existing AwaitingConflict: gzip_ask`.
red LIST1/RUN1/PASS0/SKIP0/FAIL1/not_run0,exit1. Apphost build0warnings/errors first, matchingworker
next. Same original native test is now green, not a mock or preconfirmed-overwrite substitute.

Seven independent small scenarios distribute policies rather than cross-product:

| Scenario | Actual assertion/result |
| --- | --- |
|Gzip Ask→Overwrite|Original ticket, unknown incoming size, held slot and zero currentpartial/write while waiting; typed reply -> exact276byte native publication, committed1/skipped0/Committed.|
|XZ AutoRename|Old payload retained; native payload (2) exactly equals independent276bytes, committed1/skipped0.|
|BZip2 Skip|Full decode succeeds without writes/partials/publication; old target retained; skipped1/committed0/warnings, members_skipped, recordingbackend calls0.|
|Zstd Overwrite|Existing overwrite strategy and native publication yield exact276bytes, committed1/skipped0.|
|Gzip waiting Cancel|Typed null answer wakes original wait; old target/foreignpartial retained, committed0, cancelled and actual cleanup.|
|Gzip Skip CRC damage|Only fixture CRC mutated; Skip still consumes payload/trailer and fails CorruptArchive,committed0/skipped1/no writes.|
|BZip2 Skip actual budget|Unknown-size member under1byte ordinary output quota; full sink path fails ResourceLimit,committed0/skipped1/no writes.|

All7assert source bytes unchanged, actual exited→DrainAsync/CleanupComplete/ActiveCount0/source
exclusive reopen, native FilesCommitted/FilesSkipped/category and Rust summary. Successful output
counts1/276; successful Skip complete0/0 is published-output coverage, not a fabricated test count.
Foreign .nxpart-foreign survives; no own partial leaks. Source qualification uses existing internal
observer plus a never-called backend, never Shell or actual source deletion.

Inputs are existing independent tiny fixtures: CPython gzip/lzma/bz2 UTF8 literal
`独立 gzip fixture\n` plus bytes0..255 (276bytes); Zstd checked.zst from independently documented
bsdtar/libzstd fixture+checksum. No new encoder/customformat or regenerated historicalfixture.
The exact literal oracle is independent of NxTools decoder/output. Existing fixture attribution,
checksums and producer details remain in stream/XZ/Zstd evidence and source notes.

## Final executed collection

All commands serial, machine Cargo lane checked; CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,
RUSTFLAGS=-C codegen-units=256, offline. C#isolatedDebug apphost first; matchingisolatedworker rebuilt
last after lint/fmt before native execution. No100k/largefile/longpath/password/risk/ZIP16matrix.

| Check | Actual final result |
| --- | --- |
|dotnet build PlatformHost.Tests --no-restore|exit0,0warnings/errors|
|Rust7smalltargets --features isolated-codecs --offline -- --test-threads=1|48PASS/0fail/ignored,exit0: interaction4,stream12,transaction5,worker_wire17,xz_read5,zstd_read1,zstd_safety4|
|StrictClippy isolated/default --tests --examples --offline -- -D warnings|both exit0|
|cargo fmt -p nxtools-archive -- --check /matchingisolatedworker build|both exit0|
|C#exact LIST→same-setRUN|selected5/run5/pass5/skip0/fail0/not_run0,exits0|
|spec-contracts/taskcontext/fixtureselector/gitdiff--check|PASS; source/document gates,not UI/release acceptance|

Rust worker_wire reuses existing shared capability test, changes outdated standalone rejection to
actual Skip observe→reply→summary/complete with0output, and retains no-policy capability omission.
Native exact catalog-order names:
1. `archive conflict fixture actual 7z RAR TAR decisions`
2. `archive conflict fixture actual standalone stream decisions`
3. `archive output fixture rechecked atomic path overwrite`
4. `archive overwrite fixture target changes before recheck`
5. `archive outcome fixture actual worker operation isolation`

Existing first/last3reuse numbered-targetrace, priorjob/retiredticket, native targetidentity-change,
publicationstate and isolation assertions. No old ZIP16scenario/fullpassword/risk matrix repeated;
no new asynchronous/failure framework. Explicit worker/ZIPfixtures/codecfixtures/Python/existing7Zip
oracle set before LIST/RUN; noSKIP counts asPASS and final run had noSKIP.

## Preservation / stop

Scratch `C:\Users\Administrator\.pi-desktop\scratch\949da288-1ef9-46b3-a698-90f543613cf5`
retains stream-conflict-opening-hashes/status, red/green logs, stream-conflict-verify.ps1,
command-results/native-results and per-step logs. Closing assertions validate128historical taskfiles
unchanged, prior-checkpoint byte snapshot, allowed changed paths, exact counts/names and no owned
archiveprocess/pipes/helper. Existing native-contracts35497byte context-length warning remains.
No optional review or independent acceptance claimed. No fullworkspace test/visual/release claim.

Current supported extraction formats now all connect existing conflict handling; only the tested
ordinary-file conflict conditions are closed. FullB2/PhaseA/A–F and other interaction/native/plan
requirements remain separate. No new framework/fallback/upgrade/system/UNC/sourceRecycle/delete,
UI/FIFO/PhaseB/performance/formalapp/package/install/sign/commit/push. Stop for scoped independent
acceptance; do not append defensive tests or another feature.
