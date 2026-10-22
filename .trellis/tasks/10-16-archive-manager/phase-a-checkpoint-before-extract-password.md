# Phase A — Engine contracts and safety foundation: partial checkpoint

Date:2026-10-21. **Phase A in progress, NOT passed. Only this file owns current state.**
Canonical names/order stay in [implement.md](implement.md). No Phase B/A–F completion.

## Current checkpoint — standalone stream extraction conflicts

**Standalone Gzip/XZ/BZip2/Zstd now reuse existing conflict handling; scoped checks PASS;
independent acceptance pending. Current extraction-format conflict connections are filled,
NOT full B2/Phase A/A–F completion.** [Evidence](stream-conflict-evidence.md),
[verified seam](research/stream-conflict-sources.md),
[latest format-conflict independent PASS](format-conflict-acceptance.md).

## Conflict coverage — all currently supported extraction formats

All rows connect SAME Ask/Overwrite/Skip/AutoRename/typedCancel,existing task-local apply-all,
InteractionPort/native observation and publication. Capability is not everypolicy×format×failure
runtime evidence; exact verification level is stated below. No public ArchiveRead write/UI added.

| Supported extraction format | Connection / actual evidence |
| --- | --- |
|ZIP incl.supported methods/crypto and ZIP aliases|Integrated; [ZIP conflict independent PASS](zip-conflict-acceptance.md),original policy/native rules retained,no16matrix rerun.|
|7z incl.solid|Integrated; [format conflict independent PASS](format-conflict-acceptance.md): full skipped-member decode,apply-all once/following member,cancel/native counts/cleanup.|
|RAR4 / RAR5|Both integrated; same independentPASS: each AskOverwrite+Skip/nativebytes/counts/eligibility; RAR_TEST consumes skipped payload.|
|TAR|Integrated; same independentPASS: AutoRename/Skip/raced no-replace/internalpath refusal. TAR has no payloadchecksum.|
|TAR.GZ / TAR.BZ2 / TAR.XZ / TAR.Zstd and supported aliases|Integrated through shared TAR inner-output path after full outer-integrity spool; independentPASS uses TGZ native representative. BZ2/XZ/Zstd connection is source/compiled shared-path evidence,not fresh per-wrapper×policy native matrix.|
|Standalone Gzip|Integrated this slice; actual Ask→typedOverwrite/native276bytes,waitCancel and SkipCRCfailure PASS.|
|Standalone XZ|Integrated this slice; actual AutoRename/nativebytes/oldtarget preservation PASS.|
|Standalone BZip2|Integrated this slice; actual Skip/warnings/members_skipped and skipped actual-budget failure PASS.|
|Standalone Zstd|Integrated this slice; actual Overwrite/nativebytes PASS.|

- Four stream extract admissions use existing begin_extracted_file/optional token; Skip still
  consumes entire original stream to sink with actual limits/end/applicable CRC/checksum/source
  checks. No token means no partial/write/commit. SuccessfulSkip0publishedfiles/bytes,skipped1/
  warnings/members_skipped; corrupt/overbudget Skip fails,not verified-success fiction.
- Existing filename/path/metadata/publication and no-policy behavior retained. Only known-format
  restriction widened; existing capability/wire/strategy/summary/C#production unchanged. No new
  StreamConflict/framework/transaction or password/risk/budgetreadmission extension.
- RealGzipred→green; new actual runner/Job/worker/native7scenario fixture PASS. Rust7smalltargets
  48PASS; C#exact LIST=RUN=PASS5,skip/fail/not_run0,including reused numberedrace/tickets/target
  change/nativepublication/isolation. C#apphost0warnings/errors first,matchingisolatedworker last.
  StrictClippyisolated/default,archivefmt,spec/context/selector/diff PASS. No fullworkspace claim.

## Accepted prior scope / remaining work

- [B1/deliveredC4 independent PASS](zip-encoding-acceptance.md).
- [B2 password-only independent PASS](password-operation-acceptance.md).
- [ZIP conflicts independent PASS](zip-conflict-acceptance.md).
- [7z/RAR/TAR conflicts independent PASS](format-conflict-acceptance.md); prior pending superseded.
- [B3 original-budget ZIP risk independent PASS](zip-risk-acceptance.md).
- [ZIP preflight output-budget readmission independent PASS](zip-output-readmission-acceptance.md).
- [C1/C2/C3/C5 independent PASS](phase-a-small-interop-acceptance.md).
- Ordinary-file conflict connection now covers all current extraction formats as tabled; no
  residual generic “other-format conflicts unimplemented” claim. Other-format password/risk/
  readmission and remaining fullB2/B3/SAFE04/canonical cross-stage/native decisions are NOT closed.
  NativeRecycle remains native_protection_unproven/NotAttempted; no source deletion performed.

## Preserved history and stop

[Prior checkpoint byte-identical snapshot](phase-a-checkpoint-before-stream-conflicts.md), SHA256
1F11DAE861C4A956E4A351D61364A61B8DA1752DB9DE43EC610EE7B2F639432B.
128pre-existing historical taskfiles match openinghashes. Historical reports/results/acceptances/
ledger and Failed/Stopped facts unchanged; no renumbering/exit amendment/dependency waiver.
Existingnative-contracts35497byte context warning retained. Only this file owns current state.

No100k/largefile/longpath/passwordriskZIPmatrix/performance,UI/FIFO/PhaseB/UNC,newframework/
dependency/system/sourceRecycle/delete,formalapp,package/install/sign/commit/push. Stop for
independent scoped acceptance; no next feature. No current implementation/check blocker.
