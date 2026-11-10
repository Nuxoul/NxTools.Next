# TAR / stream risk and finite output readmission

## 1. Scope / Trigger
Phase A bounded TAR/TGZ/TBZ2/TXZ/TZST and standalone GZ/BZ2/XZ/ZST safety slice. Reuse original
engine/plan/InteractionPort/ZipRisk/output_readmission and native Job/source/output. No new framework,
index/backend/dependency, ArchiveRead/CLI write authority or UI acceptance. Status belongs only to
[the checkpoint](../../../tasks/10-16-archive-manager/phase-a-checkpoint.md).
Read [reference follow-up](../../../../docs/reference-code/archive.md#tar--stream-declaration-follow-up-2026-10-24)
and [general job contract](product-jobs.md). Evidence:
`repo:rust/crates/nxtools-archive/src/worker.rs`, `repo:rust/crates/nxtools-archive/src/engine.rs`,
`repo:rust/crates/nxtools-archive/src/formats/xz.rs`, `repo:rust/crates/nxtools-archive/src/formats/zstd.rs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVolumeSources.cs`.

## 2. Signatures
- Unchanged public `ArchiveEngine::open(OpenRequest, &OperationControl)` calls private open_impl
  with stream_declarations=false. Crate-private `open_for_output_readmission(...)` selects true;
  original worker invokes it only for extract with output_readmission present.
- XZ/Zstd `list_for_output_preflight(file, path, limits, control, declared_output: bool)` reuses
  original scans. Original list passes false; ordinary open/TEST remain unknown listed size.
- XZ private `scan(..., enforce_output: bool) -> Result<u64>` returns complete checked declaration.
  Existing preflight retains Result<()> and enforces original output limits for normal use/decoding.
- Zstd private `scan(..., enforce_output: bool) -> Result<(u32, Option<u64>)>` returns window log plus
  complete declaration only if all standard frames provide FCS. Existing preflight returns original
  window log and enforces limits; stream decoder retains that original path.
- Native `ArchiveVolumeSources.Open(..., sevenzOutputPreflight=false, rarOutputPreflight=false,
  tarStreamOutputPreflight=false)` forwards original maxMetadata into private Add only when opted in.
  Original runner selects true only for extract+OutputReadmission, not a public command field.

## 3. Contracts
**Three real groups:** plain TAR has known supported non-sparse regular payload sizes (PAX local,
global, then fixed header), packed=None. Size excludes headers/padding/extensions/tail. Compound TAR
uses the same inner metadata after complete outer stream verification into the existing anonymous
spool. Outer compressed length is shared and must never become every member's packed size. TAR
structure/checksum/bounds success does not imply payload checksum; sparse remains unsupported.

Standalone Gzip/BZip2 retain size=None, one logical output with encoded source length as packed.
Gzip ISIZE is per-member modulo2^32, not a reliable complete total; BZh is block/work level, not
output length. No full predecode is added just to estimate budgets. XZ original bounded structure/
all-index/block/LZMA2 chunk scan returns checked total declaration without payload decode. Zstd
scans all standard/skippable frames: skip contributes zero, any standard frame missing FCS makes
the entire logical output unknown. Never use a partial known sum as a complete declaration.

**Opt-in declaration, not integrity:** only private extract-readmission snapshots use XZ complete
declaration or all-known Zstd FCS as size. Original structure/extent/metadata/count/dictionary/window
checks still run to completion. Only early output-quota rejection is deferred to original complete
plan/preflight; actual decode/entry/output limits stay original. Actual stream decoder still enforces
payload end/size/checksums when supplied. False declarations/corruption cannot authorize a successful close.
Public list/read/TEST remain original unknown metadata and do not approve extraction Risk.

**Risk:** XZ/Zstd extract calls original confirm_zip_entry before current final-name begin/decode. With a
complete single logical stream declaration, its encoded stream length is a reliable denominator;
TAR shared outer lengths are never used. Predicate stays strictly >64MiB AND >1000x positive packed,
overflow-safe. Original current-operation/prompt/generation ticket approves exact admitted ceiling,
never increases output/work/dictionary/metadata/Job. Larger/stale/cross-operation replies cannot
approve. No silent CLI/read/TEST approval; no new UI. Unknown ratio still has actual limits.

**Encoded source, spool and target are distinct:** under original held source lease, native bounded
signature probe recognizes Gzip/BZip2/XZ/Zstd standard/skippable or ustar/zero TAR header. Only opt-in
extract-readmission uses max(original output, original metadata reservation) as encoded input bound.
No100GiB copy, native format-validity claim, non-opt-in/open/TEST/other-format permission expansion.
Original family/lease/identity checks remain. Legacy extension-routed TAR without this signature is
not claimed to gain the opt-in bound. Original worker compound spool_limits also uses max(output,
metadata), then tgz decode uses that as outer stream limit; inner extraction restores original target
limits. Spool overflow before inner complete declarations is terminal, never a guessed shortage.

**Readmission:** original immutable plan/source verification and fully known max/sum are required.
Original AttemptResult must match operation/current limits,ResourceLimit,zero successfully closed files
and actual cleanup. Cleanup-held owns slot/source/helper/output/Job,no prompt or retry. Only after
exited/drain/active0/exclusive lease release may original coordinator request explicit OutputBudget
under32GiB/file/2^50total hard ceilings. At most one new normal admission; fresh operation/control ID,
same logical correlation/shared cancellation/source native identity/version+digest; preserve selection,
codepage/password(if applicable)/conflict/MOTW/work/dictionary/metadata/Job. Only two output limits and
necessary source bookkeeping change. Actual overflow/corrupt/sourcechange/completed output/incomplete
cleanup never finance upgrades,merges,completed-file rollback or third attempts.

**Costs and diagnostics:** public source revision already hashes full encoded input. TAR header
parser seeks over ordinary payload; extraction rereads payload and source. Existing compound open
fully decodes/checks outer stream into anonymous spool; extraction repeats original spool decode,
and readmission adds fresh source/header admission. No new full budget-estimation decode/whole archive
Vec/index. XZ/Zstd declaration uses original structure scan, then extraction's original preventive
scan/full decoder remains. Native opt-in adds at most512-byte signature probe. Original low-noise
correlated lifecycle/native codes remain; no payload/password/path production diagnostics.

Test-only NXTOOLS_TAR_STREAM_EVIDENCE_ROOT/FIXTURES/HOST and NXTOOLS_ARCHIVE_WORKER_FIXTURE select
owned scratch fixtures. NXTOOLS_TAR_STREAM_CHAIN_CASES is optional test filter; unset for final17.
Existing DEBUG lowered thresholds remain strictly bounded; Release test-field rejection is source
proof unless a dedicated runtime check was actually executed. No product env/DTO capability added.

## 4. Validation & Error Matrix
| Condition | Required original boundary result |
| --- | --- |
| Opt-in encoded source exceeds low output but fits metadata reservation | Reach original format/plan preflight; no actual output or memory grant |
| Known XZ / all-known Zstd exceeds output | Original source-bound declaration shortfall before output |
| Gzip/BZip2/Zstd unknown or mixed frames | No guessed shortfall or ratio; actual original counters remain |
| Invalid structure/window/dictionary/metadata/count | Original typed refusal,no shortfall/output authorization |
| Compound spool over finite reservation before known TAR | ResourceLimit,zero target effects,no retry |
| Cleanup incomplete/held,wrong ID/limits,prior completed output | No new approval/admission; native slot/leases remain owned |
| Source change on new attempt | Original native/digest SourceChanged,no effects |
| Actual overflow or second corruption/false declaration | Terminal,incomplete-owned-output cleanup,no third attempt |
| Completed close then cancel | Retain actual files/completion journal; no rollback or rerun |

## 5. Good/Base/Bad Cases
Good: tiny TAR total1 -> declared max4096/total6144 -> actual cleanup -> explicit finite approval ->
fresh original source-bound native attempt. Base: nine formats ordinary read/test/extract no false
Risk; public XZ/Zstd size remains unknown. Bad: gzip last ISIZE as total, partial Zstd FCS sum,
outer-TAR packed duplicated per member, spool overflow called a target declaration or ACK called cleanup.

## 6. Tests Required
- `repo:rust/crates/nxtools-archive/tests/tar_stream_readmission_contract.rs`: actual original worker
  XZ/knownZstd/all-known multi-frame declarations, unknown actual failures/no shortfall.
- `repo:rust/crates/nxtools-archive/tests/tar_stream_readmission_chain.rs`: original coordinator ->
  native cleanupheld/no prompt, real exited/drain/reopen, IDs/options/source/constant Job, representative
  TAR/compound/known streams,actual unknown failures,second corruption and completed-output cancel.
- `repo:rust/crates/nxtools-archive/tests/tar_stream_readmission_guard.rs`: injected original
  coordinator second ResourceLimit facts/no third attempt; NOT native decoder overflow evidence.
- `repo:csharp/NxTools.PlatformHost.Tests/TarStreamSafetyTests.cs`,
  `repo:csharp/NxTools.PlatformHost.Tests/TarStreamRiskTests.cs`,
  `repo:csharp/NxTools.PlatformHost.Tests/TarStreamReadmissionHost.cs`: original runner/Job/output,
  nine-format metadata and exact bytes, five TAR lowtotal declarations, unknown counters/spool/corrupt,
  completed-output journal and known-stream original Risk tickets. Exact LIST/RUN/PASS/skip/fail/not_run/hash.
- `repo:rust/crates/nxtools-archive/tests/xz_read_contract.rs`, `repo:rust/crates/nxtools-archive/tests/zstd_read_contract.rs`,
  `repo:rust/crates/nxtools-archive/tests/zstd_safety_contract.rs`: original public unknown semantics,
  structural/actual limits/source/cancel/corruption, not just new private snapshots.
- `repo:tests/integration/prepare-tar-stream-safety-fixtures.py`: tiny independent CPython/libarchive
  payloads; correct-FCS variant independently decoded. Corruption/false-FCS negatives labeled separately.
No broad method/encryption/resource matrix, UI/native-user acceptance or full safety-phase claim.

## 7. Wrong vs Correct
Wrong: copy RAR100GiB fix, decode fully to obtain stream size, expose partial FCS as total, manually
raise a second request, or label decoder CorruptArchive as actual ResourceLimit for test coverage.
Correct: reuse original bounded scans and complete declarations only at private extraction admission
-> original plan/source preflight -> actual zero-effect cleanup -> explicit finite original coordinator
-> one fresh normal admission. Report native actual counters,corrupt terminal and injected guards
as distinct evidence levels; preserve all completed-output and historical failure facts.
