# Encrypted extraction passwords — independent scoped acceptance

2026-10-21. **PASS for encrypted ZIP/7z/RAR4/5 extraction password interaction.** Reviewed
[evidence](extract-password-evidence.md), [sources](research/extract-password-sources.md),
worker_password coordination, private admission/capability and actual native fixture. No concrete
in-scope blocker found. Not full B2/Phase A/A–F, public Desktop/UI or deployment acceptance.

## Accepted boundary

- Extract creates a fresh read-only TEST request with original source/codepage/limits and secret,
  without conflict/output/delete/readmission options. Same finite password loop and InteractionPort
  reused; at most3preparation attempts. Only PasswordRequired/explicit WrongPassword retry;
  ambiguous authentication/data/corruption remains terminal, no guessing or secret persistence.
- Successful preparation invokes original extract EXACTLY ONCE using original control/native ID
  and original options. Every extraction result returns directly, including cancel/commit/error;
  no return to password loop, repeated publication, rollback or output merging.
- Source/Job/helper/slot remain original owners through waiting and extraction. Preparation uses
  new attemptIDs/shared cancellation; native output retains runner-accepted logical correlation.
- Success costs one complete no-output TEST plus one extraction payload decode. RAR retains its
  original metadata-only list; no third full decode is added. Incorrect attempts may decode data
  before typed failure under existing limits/timeout. This cost is explicit, not optimized away.
- Scope constrained to ZIP/7z/RAR4/5 by signature; extract requires additive password_extract_v1.
  Original open/test and password-free/read-only ArchiveRead contracts remain. No new framework,
  dependency, codec/conflict/native publication change or public permission was needed.
- Zero-publication preparation is not a new directory-rollback guarantee: original runner may
  prepare its output root. Test verifies existing contents/partials unchanged while awaiting input.

## Independent checks actually executed

One foreground serial batch; isolated C#apphost first, matchingisolatedworker rebuilt after lint
before C#execution. Cargo inherited jobs1/incremental0/RUSTFLAGS=-C codegen-units=256, offline;
fullmachine lane checked. No optional review or expanded format/password matrix.

| Check | Independent result |
| --- | --- |
| C#Debug build |exit0,0warnings/errors |
| Four selected Rust targets |27pass/0fail/ignored,exit0 |
| Exact existing AES classification/integrity test |1pass/4filtered,exit0 |
| Strict Clippy isolated/default tests/examples |both exit0,-Dwarnings |
| Archive scopedfmt/matchingworker build |both exit0 |
| Exact C#LIST/RUN/PASS |same5names;selected5/run5/pass5/skip0/fail0/not_run0,exits0 |
| Spec/context/fixtureselector/diff |all exit0 |
| Identity/sourcehash assertions |PASS;63currentbaselinefiles unchanged during checks |

New original runner/Job/actualworker/native fixture executes9small scenarios: ZIP,7z,RAR4,RAR5,
RAR5wrong_then_correct/staleticket,waitingCancel,ZIPAutoRename after preparation,correctpassword
with damagedRAR5 and cancellation after actualfirstZIPpublication. Asserts pending zero writes/
publication/new output entries, source/helper protection, one original worker, finite unique prepIDs,
zero/one extracting state, independent literal/hash outputs, original conflict options, truthful
commit/category, old/foreignfile retention, no secret in events/logs and actual cleanup/slot0/
source+helper exclusive reopening. Previously published output survives postcommit cancellation;
there is no second extraction/password restart. Other4fixtures preserve original open/test password,
productreadprivacy, codecerror and commitlifecycle behavior. No SKIP counted as PASS.

No workspace/global compile/test/format claim. Existing native-contracts context-size warning
remains; no unrelatedfile fixed. Current63file baseline verified, not a fresh independent claim
to reproduce implementer's entire132historicalfile comparison. Submitted evidence unchanged.

## Evidence / stop

Scratch C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df:
extract-password-independent-verify.ps1, commandledger/per-steplogs, baseline and assertionsJSON
retain commands/exits/counts/exactnames. Acceptance modified no implementation/tests; this report
only is added. Closing0archiveworkers/testhosts,0testpipes,fixedhelperabsent. No100k/largeinputs/
longpathmatrix/performance,UI/FIFO,sourceRecycle/delete,systemchange,formalapp,package/install/sign/
commit/push. **This extraction-password slice is accepted; stop. Full PhaseA/A–F remains incomplete.**
