# TAR / stream risk and finite output readmission — independent bounded acceptance

2026-10-24. **PASS for the bounded Phase A source/native slice.** Not full B3/SAFE04/Phase A/F or
first-release acceptance. UI/Explorer remains DEFERRED / NOT RUN. Current status belongs only to
[the checkpoint](phase-a-checkpoint.md). [Implementation outcomes](tar-stream-risk-readmission-outcomes.md),
[pre-code contract](tar-stream-risk-readmission-contract.md), [owner contract](../../spec/nxtools-archive/backend/tar-stream-safety.md)
and all earlier ZIP/7z/RAR/CLI acceptances remain retained. User separately authorizes packaging AFTER
this acceptance, not installation or live interaction/service acceptance.

## Source review and accepted behavior

Parent read the source/reference/owner boundaries and original tests. One bounded read-only review
covered the six production files; no repeated review loop or product/test modification by acceptance.

- Original generic worker/plan/preflight/InteractionPort/output-readmission chain remains the owner.
  Private extract+OutputReadmission admission reuses complete XZ Index/extent scan declarations and
  Zstd FCS only when ALL standard frames are known. Any unknown standard frame makes the whole output
  unknown; skippable contributes zero. Public open/read/TEST metadata remains unknown and retains its
  original preventive limits. Actual decode still checks original budgets/end/integrity before commit.
- Native source acquisition probes at most512bytes AFTER the original lease. Only opt-in extract uses
  max(original output,original metadata reservation) for recognized TAR/stream encoded input; not the
  RAR/7z100GiB bound. Original source/family/native identity/version/digest and actual output/Job stay.
- TAR regular payload declarations exclude framing; member packed remains None. Compound TAR retains
  its original complete verified anonymous finite spool, then original inner plan/output. Outer length
  is not a per-member ratio. Gzip ISIZE/BZh do not supply full reliable output declarations. TAR has no
  payload checksum; structure success is not payload integrity proof.
- Complete standalone XZ/Zstd logical streams can use their own encoded length for original Risk,
  before CURRENT MEMBER partial/decode/write. Production >64MiB AND >1000x/exact-current-ceiling tickets
  remain. Risk is not budget permission. Read/TEST/CLI cannot silently authorize extraction Risk.
- Original coordinator still requires reliable matching declaration, zero commits/publication and
  actual cleanup before one explicit finite fresh admission. New control ID, same logical cancellation,
  original options/source revalidation and unchanged working/dictionary/metadata/Job limits remain.
  Actual overflow/corrupt/source change/incomplete cleanup/commits do not fund another attempt or merge.

## Independent execution

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\tar-stream-independent-package-20261024-174738`.
Session scratch runner `tar-stream-independent.ps1`; raw logs/commands.jsonl, worker-meta.json,
20-independent-audit.json and fixture manifest retain actual exits/counts/hashes. Every Cargo command
checked one shared lane and inherited jobs1/incremental0/codegen256. Initial isolated C# host came
first, then strict/features and matching worker, original scratch identity generator, finalhost/tests.

| Check | Independent result |
| --- | --- |
| Initial/final isolated host and final Tests builds |PASS,0warnings/errors;01/10/11logs|
| Original readmission + TAR/stream contract + injected guard |3+2+1PASS,failed/ignored/filtered0;03log|
| Affected public XZ/Zstd regressions |5+1+4PASS,failed/ignored/filtered0;04log|
| Archive strict Clippy --tests, isolated and default |bothPASS,-Dwarnings;05/06logs|
| Final feature tree / matching isolated worker |PASS;07/08logs|
| Exact native fixture selection |LIST5=RUN5=PASS5,same names/order,skip/fail/not_run0;12/13logs|
| Original coordinator -> actual native runner/Job/output |1test/17unfilteredscenariosPASS,failed/ignored/filtered0;14log|
| Worker before/after native+chain |45C9C1DCEA5E853EC4F6596191E66DA4D42627809871B3ADA83571116B780CE3,identical|
| Seven affected Rust files rustfmt / spec / task / diff |PASS;15–18logs,spec114documents/14owners|

Exact native suffixes, prefix `archive TAR stream safety fixture `: low total preflight;
metadata ordinary no risk; unknown corruption finite spool; postcommit cancellation;
known stream risk tickets. Five test cases, not five archive operations.

One test `original_tar_stream_coordinator_readmits_only_after_actual_cleanup` executed17scenarios:
tar_entry,tar_total,tgz_total,txz_total,xz_total,zstd_total,second_corrupt,second_false_declaration,
cancel,shared_cancel,source_changed,selected,actual_gzip,actual_bzip,actual_zstd,spool_limit,committed.
NXTOOLS_TAR_STREAM_CHAIN_CASES was removed. All17 summaries and each attempt were audited for actual
exited/cleanup/active0/source+helper exclusive release. Cleanup-held checks retain original slot and
no Budget prompt until actual release. Normal Job memory stays138412032bytes; spool representative
uses its separately bounded smaller metadata reservation. Options/codepage936/auto_rename/MOTWfalse,
new IDs/source identity and original selected ordinals remain checked. Commit-bearing cancellation
preserves one independently validated TAR member and Committed/journal, not assumed archive order.

Unknown first-attempt actual counters are real native ResourceLimit evidence. second_corrupt and
second_false_declaration are real second-attempt CorruptArchive, NOT second actual ResourceLimit.
The latter no-third-attempt rule is separately proven ONLY by original-coordinator fact injection;
no native second-overflow PASS is claimed. Historical false-FCS failed expectation remains retained.
Historical worker03/native06 red logs were independently read, not rerun on reverted source.

Fresh independent tiny CPython tarfile/gzip/bz2/lzma and Windows libarchive raw Zstd samples were
regenerated under this evidence root. Independent stock7-Zip decode compares literal data for Zstd;
CPython verifies Gzip/Bzip2/XZ bytes. Correct-FCS Zstd structural variants are re-decoded, not claimed
as unmodified producer output. CRC/payload/false-FCS negatives remain explicitly invalid copies.
Producer/oracle hashes and commands are in fixtures/manifest.json. No NxTools self-roundtrip oracle,
new tool installation, executable launch, large/bomb/performance test or full predecode estimator.

## Review observation: prepared empty root is not member output

The reviewer suspected `output_ready` creates a missing target before Risk, and interpreted the
retained empty root as zero-effect-denial violation. Parent checked original ZIP Risk evidence:
confirmation precedes current-member decode and partial, not all output-root preparation. Original
ArchiveOutputSession.CreatePreparedRoot deliberately retains created directories on failure and only
cleans exact owned unpublished partials; no path-based directory rollback. That owner is unchanged.

A separate scratch-only probe links original fixture classes and references the tested host DLL,
using the SAME matched worker. Missing targets for known XZ/Zstd each reach Risk, then decline:
root_at_prompt=true/root_after=true, EMPTY; writes0/publications0/partials0, cancelled/cleanuptrue,
actual source/helper release. Two probes passed (19-risk-root-probe.json,19b-run log). Therefore this
is an observed inherited root-preparation behavior, not a confirmed new defect or a reason to change
original output policy. PASS does NOT imply decline leaves a previously missing root absent.
Public JobExecution still retains its original destination-confirmation owner; this probe is the
non-Job direct fixture path. No extra review or production fix was performed.

The scratch probe first used an incorrect Debug DLL path (build failure, no execution); corrected
actual debug_win-x64 path built with1assembly-resolution warning/0errors and ran2probes. This is
separate from final original host/tests0warnings. First background verification starter vanished
before producing output; no child/build remained, its empty log retained, then foreground long-timeout
runner completed once. First audit parser captured CRLF in names; corrected parser passed without
rerunning product tests. Failures are retained, not product reds or missing cases relabeled PASS.

## Preservation and release boundary

Before report/checkpoint writes3215/3215opening files unchanged,missing0;84/84deliveryfiles unchanged.
All13monitored protected Service-directory files unchanged, including the previously observed six
protected binaries. This is file observation, not service restart/runtime/ledger/installation acceptance.
Concurrent changes present at opening were preserved, not reset or accepted wholesale. No frozen
workspace claim beyond the measured interval. No current-evidence-root residual product process was
needed or launched outside controlled fixtures. Historical reports/checkpoint bodies are retained.

Acceptance writes only this additive report, checkpoint and scratch. No product/test/dependency/vendor
edit, install/service/bootstrap/Registry/COM/UNC/recycling/P209/commit/push. No full Desktop/CLI/workspace
or already accepted format matrix,100k/large/longpath/performance rerun. Dedicated Release fixture-field
runtime rejection remains NOT RUN; cfg/parser source and a later Release build are not that assertion.

Existing compound open/extract fully decodes outer spool and fresh readmission repeats source/header/
hash/admission work. XZ/Zstd declaration reuses structure scans; actual extract retains original scan
and full decoder. These costs are retained, not a new full budget-estimation decode/index/whole Vec.

**Stop: bounded TAR/stream source/native slice independently accepted; packaging now separately
user-authorized.** Packaging result belongs to its own outcomes/checkpoint update. UI/Explorer,
foreignCOM/DPI/UIA/P206/P207live, trustedUNC, strict recycling/default-off NotAttempted,
JOB07/SAFE08/resource and earlier A–F obligations remain unaccepted. ProductApplyAuthorized=false;
no full B3/SAFE04/Phase A/F/first-release conclusion.
