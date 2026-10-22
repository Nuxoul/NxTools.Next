# ZIP output-budget readmission — independent scoped acceptance

2026-10-20. **PASS for declared ZIP output-budget preflight and one explicitly approved new
admission.** Reviewed [evidence](zip-output-readmission-evidence.md), [results](zip-output-readmission-results.json),
[source ledger](research/zip-output-readmission-sources.md), production Rust coordinator/worker,
InteractionPort and C#source/output/runner plus real-chain fixtures. No concrete in-scope blocker
found. This does not complete full B3/SAFE04/Phase A/A–F or Desktop/UI integration.

## Accepted boundary

- Production Rust output_readmission::execute retains request and calls an existing-runner adapter.
  It requires matching originalID/actualcleanup and zero-commit ResourceLimit shortfall/current
  limits before a separate OutputBudget prompt. Old Risk exact-original-limit semantics remain.
- Explicit finite approval must cover current/required limits and observed native ceilings.
  Only two output fields change on a fresh operation/control ID sharing original cancellation.
  At most one new attempt; actual-byte failure cannot trigger another prompt/retry. No work,
  metadata,dictionary,Job or accepted-request mutation and no output merging/rollback.
- Native output Prepare does not create missing destination; output_ready creates it only after
  original path/plan/local-header checks and new-source validation. Native held-source volume/
  fullID/size/mtime and Rustdigest reject changed input before output effects. Original stream
  counters/CRC/source/nativepublication remain authoritative.
- Coordinator is a callable narrow domain interface, NOT a newly integrated Desktop command or
  public ArchiveRead write path. Actual test adapter supplies original C#ForPackage runner/Job;
  adapter makes no approval decisions and waits actual exited then drain/source+helper reopening.
  The adapter's audit field named source_acquired is derived from accepted frame, not by itself
  proof of source acquisition; real source_stamp/heldidentity and native output protocol provide
  the applicable source-boundary evidence. No stronger inference from that audit label is made.

## Independent verification actually executed

One foreground serial batch, no delegate or broad review. C#isolated apphost built first;
matchingisolatedworker rebuilt after lint checks before C#execution. All Cargo inherited jobs1,
incremental0,RUSTFLAGS=-C codegen-units=256; machine lane checked before commands.

| Check | Actual independent result |
| --- | --- |
| C#Debug build |exit0,0warnings/errors |
| Eight selected Rust integration targets |36pass/0fail/ignored,exit0 |
| Real coordinator/native-chain test |1test,5scenarios: member,total,cancel,source_changed,actual |
| Existing ZIP risk domain tests |2pass/14filtered,exit0 |
| Strict Clippy isolated/default tests/examples |both exit0,-Dwarnings |
| Archive scoped fmt /matchingworker build |both exit0 |
| Same exact C#LIST/RUN/PASS |selected7/run7/pass7/skip0/fail0/not_run0,exits0 |
| Spec/context/fixtureselector/diff |all exit0 |
| Own identity/count/sourcehash checks |PASS;67reviewedfiles unchanged during verification |

Real tests use independent CPython tinyZIPs/lowordinaryquotas and execute productionRustcoordinator
->fixturetransport->originalrunner/Job->actualworker->nativepublication. They assert initial actual
cleanup before pending/newdispatch, no missing-root creation before approval, freshcontrol/request
IDs, options preserved, constantJobmemory, source/helperreleased, exact4096/6144outputbytes, cancel
oneattemptonly, stale/duplicateapproval ignored, changedsource refusal and actualoverflow with no
currentpublish/partial or thirdattempt. Contract tests cover incompletecleanup/commits/wrongfacts/
hardcaps/invalidapprovals/sharedcancel. C#7collection also checks original Risk/conflict/readprivacy,
Jobadmission/stuckslots/nativeZIP boundaries. No fakeworker/manualenlargedrequest substitute.

Specs106docs/14owners,context82implement/79checkentries; existingnative-contracts35497byte warning
retained. No globalworkspace compile/test/format PASS is claimed or required by this scoped run.
Current67file hashbaseline verified; no independent claim to reproduce implementer's entire116
historicalfile opening comparison. Submittedreports/results were not edited by acceptance.

## Evidence / stop

Acceptance scratch C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df:
readmission-independent-verify.ps1, commandledger/per-steplogs, baseline and assertionsJSON retain
actualcommands/exits/names/counts. Closing0archiveworkers/testhosts,0archiveproduct/resourcepipes,
fixedhelperabsent. No implementation/test changes, additionalreview,100k/longpathmatrix/1GiB/
largeZIP64/sampling,formalapp/UI,FIFO,systemconfiguration,sourceRecycle/delete,package/install/sign/
commit/push. **This preflight-only readmission slice is accepted; stop. Midstream upgrade/continuation,
other-format interaction and remaining canonical PhaseA/B3/SAFE04/A–F work are not accepted here.**
