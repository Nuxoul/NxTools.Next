# TAR / stream risk and finite output readmission — bounded Phase A slice

2026-10-24 implementation authorization; exact original phase is **Phase A — Engine contracts and
safety foundation**. No new/reordered stage/full B3/SAFE04/A/F/first-release acceptance. ZIP/7z/RAR/
CLI independent acceptances remain closed; UI/Explorer deferred/no user operation; package27 unchanged.
Only [checkpoint](phase-a-checkpoint.md) owns current status.

## Original owners / candidate gaps before code

Group by actual boundaries, not a full cross-product:
1. Plain TAR: original fixed-header/PAX metadata/selection/plan, unknown member packed, known regular
   sizes, original preflight_output and staged token publication. TAR has no payload checksum.
2. TGZ/TBZ2/TXZ/TZST: original full-integrity outer anonymous disk spool before inner TAR metadata;
   existing worker spool_limits=max(output,metadata), original inner actual output/work/Job unchanged.
   No new full budget-estimation decode or index. Disclose existing open/extract spool/decode cost.
3. GZ/BZ2/XZ/ZST: one logical output stream. Gzip ISIZE is per-member modulo2^32, not reliable total;
   BZip2 offers no full output declaration. Original XZ structure/index/LZMA2 extents and Zstd all-frame
   content-size scans may provide declared admission facts without payload decode, not integrity proof.
   Mixed/unknown/skippable semantics must be checked; no inferred ratio from format name.

Current worker already uses original output_readmission/PrivateOutput/Risk. Candidate1: native encoded
source acquisition uses output allowance and may reject before TAR declaration/actual stream counters.
If real native red proves it, a TAR/stream-only extract+readmission signature opt-in may use original
finite max(output,metadata) source reservation, never RAR/7z100GiB or other-format/open/TEST expansion.
Candidate2: current XZ/Zstd metadata preflight can reject declared output before original shortfall,
while public listed size remains unknown. If actual worker red proves it, expose only original scans'
complete declaration to opt-in extraction preflight, preserve open/TEST behavior, all structural/work/
dictionary/metadata checks and actual extraction limits. No decoder/data-source framework or public wire.
Correct existing paths get tests/evidence only. Expected repair files if red: formats/xz.rs/zstd.rs,
worker.rs and original native source owners; no edits to accepted ZIP/7z/RAR policies/CLI/dependencies.

## Policy / lifecycle invariants

Reliable positive logical-member packed plus declaration strictly >64MiB AND >1000x, overflow-safe,
uses original exact-current-ceiling Risk ticket; no threshold disable/raise or enlarged admitted Job.
Plain/compound TAR packed stays None: outer compressed size cannot be attributed to every member.
Standalone known complete XZ/Zstd declaration may use its one encoded stream's length; unknown totals
cannot produce a guessed ratio/shortfall. Unknown actual bytes still bounded. Original path/collision/
selection/plan/source checks precede prompt/output; read/TEST/noninteractiveCLI cannot autoapprove.

Only complete reliable matching plan declaration, zero commits/publication and actual exited/drained
worker/Job/source/helper/output cleanup may prompt original OutputBudget. Cleanup-held owns slot/leases,
no prompt/retry. Explicit finite approval under original32GiB/file/2^50total ceilings allows at most one
fresh control/operationID, same logical correlation/sharedcancel, normal native/source identity/version+
Rustdigest validation. Preserve codepage/password(if applicable)/selection/conflict/MOTW/work/dictionary/
metadata/Job. Only two output scalars and necessary source bookkeeping change. Actual overflow/corrupt/
sourcechange/commits/incompletecleanup cannot authorize retries/merging/rollback/deletion of replacements.
Second failure terminal,no third attempt. Original low-noise correlation/lifecycle diagnostics only.

## User-agreed regression seams / execution

Original actual worker/private protocol, original Rust output_readmission::execute -> fixture-only
transport -> original native runner/Job/source/output. No decoder helper or manual enlarged requests
as substitute. Use tiny independent Python TAR/gzip/bz2/lzma and attributed Zstd decoder/producer
fixtures. Structural mutated negative data labeled separately. Prove native lowquota and XZ/Zstd
candidate red before product changes; no invented defect if original path already correct.

Minimal representative coverage: nine-format ordinary metadata/noRisk; TAR/plain+compound declaration
entry/total before effects; all compound metadata decode boundaries; known XZ/all-knownZstd vs unknown
GZ/BZ/mixedZstd; cleanup-held/newIDs/options/source/unchangedJob; decline/sharedcancel/sourcechange;
actual unknown overflow/no-upgrade and second actual failure/no-third; native postpublishcanceljournal.
No full password/conflict/method/resource matrix or all-format repeated lifecycle cases.

Initial isolated C#host -> targeted contracts/strict -> finalfeatures -> matchingisolatedworker ->
original generator scratchidentities -> finalhost/tests -> exactLIST/RUN/PASS+unfilteredcoordinatorchain.
EveryCargo checkedsharedlane/jobs1/incremental0/codegen256; raw percommandlogs/failures one scratchroot.
Relevant fmt/spec/task/diff/preservation audits only. No fullDesktop/CLI/accepted-format/100k/large/
longpath/performance. At most one necessary bounded review, no repeated research or review loops.

## Delivery / stop

Add outcomes and source-backed seven-section owner spec; preserve historical checkpoint body and all
reports/ledger/concurrentchanges. No productlaunch/package/install/sign/sequence/service/bootstrap/
Registry/COM/UNC/recycle/P209/dependency/vendor/elevation/commit/push. Explain metadata scans, hash rereads,
existing compound full spools and any repeated decode; no whole archive Vec/new full estimator decode.
Stop **TAR／流格式风险与有限预算再准入切片待独立验收**; independent/native/UI/release levels remain distinct.

## Implemented bounded disposition

Actualworker03RED confirmed missing XZ/completeZstd declarations;04GREEN reused original scans via
crate-private engineopen/extract opt-in,publicopen/TEST unchanged. Native06RED collected allfiveTAR
source_unavailable beforemetadata;08GREEN only adds held-signature extract+readmission finite
max(output,metadata) encodedcredit,not100GiB. PlainTAR/compoundspool/Gzip/BZip2/coordinator policies
unchanged. Finalcontracts3+2+1,publicXZ/Zstdregressions5+1+4,nativeLIST5=RUN5=PASS5 andoriginalchain
1test/17scenariosPASS. FalseFCS negative returnedCorruptArchive,notactualResourceLimit; secondactual
ResourceLimit no-third evidence is separately labeled coordinatorfactinjection,notnativeproof.
See [outcomes](tar-stream-risk-readmission-outcomes.md) and [owner contract](../../spec/nxtools-archive/backend/tar-stream-safety.md).
Stop unchanged,pendingindependentacceptance;no wholeB3/SAFE04/A/F claim.
