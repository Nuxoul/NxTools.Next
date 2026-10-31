# 7z risk / finite output readmission — independent bounded acceptance

2026-10-24. **PASS for this bounded Phase A safety slice.** Not full B3/SAFE04/Phase A/Phase F or
first-release acceptance. UI/Explorer unified acceptance remains deferred. Current status belongs
to [the sole checkpoint](phase-a-checkpoint.md). [Implementation outcomes](sevenz-risk-readmission-outcomes.md),
[slice contract](sevenz-risk-readmission-contract.md), historical ledger/matrix and ZIP acceptances
remain unchanged. [Owner contract](../../spec/nxtools-archive/backend/sevenz-safety.md) describes limits.

## Review and conclusion

Parent inspected the four production files, original extraction preflight, packed attribution,
original coordinator/transport and native test boundaries. No new confirmed actionable defect found;
no additional delegate review/second review cycle or implementation change in this acceptance.

- Rust SourceSet::open_sevenz separates encoded source admission only for opt-in7z readmission;
  C# ArchiveVolumeSources probes actual6byte7z signature AFTER original lease acquisition, selected
  only by extract+OutputReadmission in original runner. Finite bound is max(original output,existing
  default100GiB), not new output/work/metadata/dictionary/Job authority. Nonopt-in/other formats do not
  gain that bound. Source set/native identity/version/digest protections remain.
- sevenz_member_packed requires one streamed member and exactly one attributed packed input span.
  Shared solid/no stream/orphan or ambiguous attribution stays None; compressed_size/globalis_solid
  is not used to invent a per-member denominator. Original >64MiB AND >1000x predicate remains.
- Original path/selection/plan precedes declaration/preflight/Risk, wanted-member confirmations before
  decode/partial/publication. Risk confirms only original entry limit; unknown ratio still obeys
  actual byte limits. Read/TEST receives no extraction Risk permission.
- Original output_readmission owns at most one explicitly approved fresh attempt. A matching zero-
  commit ResourceLimit declaration plus actual cleanup permits finite entry/output changes only,
  with fresh controlID/same logical correlation/shared cancel and normal source readmission. Actual
  overflow/corruption/commits/incompletecleanup never finances another attempt. Solid selected decode
  may charge unselected block bytes and fail actual quota; no declaration-estimation fulldecode added.

## Independent execution

Evidence:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\sevenz-independent-package-20261024-141028`.
Runner `sevenz-independent-verify.ps1` in session scratch; commands.jsonl/raw logs,
acceptance-results.json and worker-meta.json retain actual exits/names/counts/hashes. Host first,
strict/features then matching worker, original generator scratch approvals and finalhost/tests.
All Cargo serial with checked shared lane,jobs1/incremental0/codegen256. No product/source edit.

| Independent check | Actual result |
| --- | --- |
| Isolated initial/final host then Tests builds |PASS,0warnings/errors|
| output_readmission_contract + sevenz_readmission_contract |3+1PASS,0FAIL/ignored/filtered;03-contracts.log|
| Exact orphan-packed attribution unit |1PASS,21filtered;04-packed.log|
| Archive strict Clippy --tests with isolated/default features |bothPASS,-Dwarnings;05/06logs|
| Feature tree/matching isolated worker |PASS;07/08logs|
| Exact C# native set including TEST |LIST5=RUN5=PASS5,skip0/fail0/not_run0,names/order equal;12/13logs|
| Original coordinator -> real native runner chain |1test/10scenariosPASS,0FAIL/ignored/filtered;14-chain.log|
| Worker before/after native+chain |92C85AAF4253F34668F9155F816A4D72E2CF25A332490FA501765D741AF31159,identical|
| Four affected Rust files rustfmt/spec/task/diff |PASS;spec112documents/14owners;source/docs not native acceptance|

Exact native names: packed metadata; risk and solid semantics; publication cancellation skip;
low total preflight; TEST no risk interaction (each prefixed `archive 7z safety fixture `).
Actual ten chain scenario order:entry,total,solid_total,cancel,shared_cancel,source_changed,
selected_success,actual,second_actual,committed. No NXTOOLS_SEVENZ_CHAIN_CASES filter. These are ten
scenarios in ONEtest,not ten Rusttestcases or a sum of old successful evidence directories.

Stock7-Zip26.00 fixture producer SHA4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f
created new tiny solid/nonsolid archives; independent l/t/x-so checks compare fixed4096/2048byte
literals. Corruption only modifies own packedbyte35. Original native chain asserts cleanupheld/active1/
source+helperprotection/noBudgetprompt, then actualexited/draintrue/active0/exclusive reopen before
AttemptResult/newapproval. NewIDs/options/source identity and unchanged Job memory verified. Actual
solid overflow yields no shortfall/retry; second failure no third attempt; publication cancellation
preserves one2048byte file/Committed/native journal. Risk stale/larger/crossoperation replies refuse,
solid skip/full integrity and corrupt refusal retain existing/foreign bytes.

First oracle invocation failed BEFORE fixture creation due to PS5 UTF8-without-BOM Chinese-path
interpretation in the scratch runner. Originaltool exists; pass its exact path via environment and
continue only unexecuted steps.02failure retained,02bproducerPASS. No product test failure/reverted
source or overwritten log. No CLI/fullDesktop/workspace execution/formatmatrix/100k/large/performance.
Dedicated Release fixture-field runtime rejection not part of this acceptance; original strict
cfg/deny_unknown_fields source proof only, distinguish subsequent Release packaging checks.

## Preservation / packaging boundary

Before this report writes,3181/3181baselinefilesunchanged,missing0;84/84sequence26deliveryfilesunchanged,
sixprotectedbinaryhashesmatchpackaged26,noownprocessunderthisroot. No installer/key/ledger/package/
service/Registry/COM/UI/source-recycle operation performed by acceptance. Historical policies,
coordinator/ZIPreports/dependencies/vendor retained. No evidence counts inferred from cumulativeHEADdiff.

User separately requests packaging AFTER acceptance. A following signed sequence27 may replace the
writable artifacts with originalpublisherkey/sequence/headroom controls and singlepayload installer;
this does not authorize installation or liveUI/Shell/service changes. Current observed pass is not a
claim about that future build. CLI ordinary/lifecycle acceptance remains closed,no repeat required.
RAR/TAR/stream risk/readmission,UNC/recycle/earlierA-F/native/user gates remain open; UI/Explorer
continues deferred. Stop accepted7zfinite slice,not broad Phase A/SAFE04/F/firstrelease completion.
