# Phase A — Engine contracts and safety foundation: partial checkpoint

Date:2026-10-20. **Phase A in progress, NOT passed. Only this file owns current state.**
Canonical names/order stay in [implement.md](implement.md). No Phase B/A–F completion.

## Current checkpoint — B3 ZIP preflight output-budget readmission

**仅完成“预检阶段输出预算重新准入”；scoped checks PASS; independent acceptance pending.
NOT full B3,SAFE04,Phase A or A–F completion.**
[Evidence](zip-output-readmission-evidence.md), [actual results](zip-output-readmission-results.json),
[verified sources/lifecycle](research/zip-output-readmission-sources.md).

- Rust narrow output_readmission::execute retains original request. Original ZIP plan/local records
  produce explicit current/required member+total shortage BEFORE target-directory/partial/publication.
  C#uses existing Prepare; only output_ready creates output. No generic approval/scheduler/runtime.
- Only matching zero-commit ResourceLimit and ACTUAL exited cleanup can request a separate finite
  OutputBudget approval. Original Risk remains exact-original-budget confirmation, never escalation.
  Cancel/stale/duplicate/invalid answers cannot dispatch extra operations. No permanent approval.
- Explicit larger finite quotas respect existing32GiB/file +2^50total native bounds. One fresh ID/
  attempt control links logical original and shares cancel; original admission/runner/Job/source
  acquisition repeats. Native source identity/revision plus Rust SHA256 must match before output.
  Path/codepage/supported conflict/MOTW/private and host destination options retained; worker selection
  stays all entries. Work/dictionary/metadata/Job limits unchanged. No ArchiveRead write or UI/FIFO.
- Actual counters/CRC/size/path/source/publication still apply. Actual overflow/new-attempt failure
  is terminal, no third attempt, cross-attempt merge, committed-output upgrade or rollback.
- Real preflight red→green; production Rust coordinator→original C#runner/Job→actual matchingworker→
  nativepublication5tiny scenarios PASS. Eight small Rust targets36PASS +oldRiskdomain2PASS; exact
  C#LIST=RUN=PASS7,skip/fail/not_run0. C#apphost0warnings/errors first; matchingisolatedworker last.
  StrictClippyisolated/default,archivefmt,spec/context/selector/diff PASS. No fullworkspace claim.

## Accepted prior scope / remaining work

- [B1/deliveredC4 independent PASS](zip-encoding-acceptance.md).
- [B2 password-only independent PASS](password-operation-acceptance.md).
- [B2 ZIP conflict independent PASS](zip-conflict-acceptance.md).
- [B3 original-admitted-budget ZIP risk independent PASS](zip-risk-acceptance.md).
- Other-format interaction/risk, fullB2/B3/SAFE04 and canonical cross-stage/native dependencies
  remain unfinished. No exit-ledger reclassification, phase renumbering or stage amendment.
  NativeRecycle remains native_protection_unproven/NotAttempted.
- Sequence16 is unrelated-feature acceptance, NOT archive completion/deployment proof.

## Preserved history and stop

Prior whole checkpoint is byte-identical in
[checkpoint before output readmission](phase-a-checkpoint-before-output-readmission.md), SHA256
C0AA4D0BB0B03C0479CDB7FBB3BF70351C586099AF1614E2445F24ECA25166AC.
116pre-existing taskfiles other than currentcheckpoint match opening hashes. All prior reports/
results/acceptances,Failed/Stopped facts and original ledger remain unchanged and historical.
Only this file owns current state. Existing native-contracts context-length warning remains.

No100k/longpathmatrix/largefile/performance,sourceRecycle/delete/systemconfiguration,formalapp,
UI/FIFO/PhaseB,dependencyupgrade,package/install/sign/commit/push. Stop for independent scoped
acceptance; no next feature. No current implementation/check blocker; no full-phase acceptance.
