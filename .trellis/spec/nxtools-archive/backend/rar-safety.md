# RAR4/RAR5 extraction risk and finite output readmission

## 1. Scope / Trigger
Bounded Phase A safety remainder over locked unrar_sys0.5.8 / bundled UnRAR7.1. RAR creation remains
excluded. Reuse original metadata/pass/InteractionPort/ZipRisk/output_readmission/native output, not
an additional extraction/retry/security framework. Current status belongs only to the
[task checkpoint](../../../tasks/10-16-archive-manager/phase-a-checkpoint.md).
Read the [RAR reference](../../../../docs/reference-code/archive.md#rar-riskreadmission-source-admission-follow-up-2026-10-24)
and [general job contract](product-jobs.md). Evidence:
`repo:rust/crates/nxtools-archive/src/worker_codecs.rs`,
`repo:rust/crates/nxtools-archive/src/worker_codecs/rar.rs`,
`repo:rust/crates/nxtools-archive/src/worker_codecs/volumes.rs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVolumeSources.cs`.

## 2. Signatures
- Windows-only `SourceSet::open_rar(request: &WorkerRequest, control: &OperationControl) -> Result<Self>`
  enters existing `open_bounded(..., encoded_limit)`; run_rar alone selects it after actual signature
  dispatch. The existing sevenz entry is unchanged.
- `ArchiveVolumeSources.Open(string path, ulong maxMetadata, ulong maxEncoded,
  bool sevenzOutputPreflight = false, bool rarOutputPreflight = false)` retains ordered source leases.
  Private `Add` returns the selected encoded bound. Original runner passes rarOutputPreflight only
  for operation=extract with OutputReadmission present, never a caller-selected public permission.
- Unchanged `rar::execute(request, control, output, expected)` uses RAR_TEST/null destination and the
  packed DLL header ABI; no native filesystem extractor or RAR compressor.
- Unchanged `output_readmission::execute(request, control, interaction, run)` owns at most two total
  attempts. `run` returns original AttemptResult only after actual exited/drain/lease cleanup.

## 3. Contracts
**Metadata:** keep pack(1) PackedHeader and its x64 size/offset assertions; do not switch to upstream
repr(C) HeaderDataEx. Size and packed combine their high/low32 fields. INT64NDF is
0x7fffffff7fffffff, exposed as size=None, not zero/u64::MAX. PackSize of a split member is only the
current packed part, while UnpSize describes the member and must not be summed per volume. LIST
skips continuation headers; its packed field is not an aggregated full-member denominator.

**Ratio:** a reliable positive ordinary member packed value requires no RHDF_SPLITBEFORE/AFTER,
no archive ROADF_SOLID and no member RHDF_SOLID. Archive solid and member continuation differ: even
first member can have RHDF_SOLID=false in a solid archive. Shared/solid/split/unknown attribution is
None, never an average, shared block or archive-length substitute. A complete nonsolid nonsplit
member inside a volume family is not automatically split merely because ROADF_VOLUME is set.
Unknown ratio still obeys declared/actual byte and dictionary/work/metadata budgets.

**Risk:** original strictly >64MiB AND >1000x known positive packed, using overflow-safe arithmetic.
Original source/path/collision/selection/format/plan checks precede prompts,and all wanted Risk
confirmations precede payload/final-name begin. Exact current operation/prompt/generation approves
only its original admitted entry ceiling. Larger/stale/cross-operation/duplicate replies cannot
approve; same-operation stale transport acceptance is not Rust policy approval. Risk never increases
entry/output/work/dictionary/metadata/Job limits. Decline/cancel stops later output and retains prior
successfully closed truth. Open/TEST and noninteractive CLI gain no Risk UI or implicit approval.

**Encoded admission:** ONLY RAR extract+output_readmission uses max(original output allowance,
existing default100GiB) for encoded input. Rust obtains ResourceLimits::default; C# obtains the same
existing default from ArchiveWorkerBudget.Capture. Native RAR4/5 signature inspection happens AFTER
original source lease acquisition. Original family/name/sequence/missing/wrong-volume/native source
identity/version/digest checks remain; non-opt-in/open/TEST/other-format permissions stay unchanged.
This is not a decode/output/Job grant. No new constant or public budget field is introduced.

**Readmission:** original complete plan and fully known declared regular-file max/sum may report
OutputShortfall {current,required,source}; declarations are admission facts, not payload integrity.
Only matching ResourceLimit attempt/current limits,zero successfully closed files and actual cleanup
permit one explicit finite OutputBudget (entry <=32GiB,total <=2^50,covering required/current).
Fresh operation/control ID retains logical correlation/shared cancellation; normal native admission
rechecks original native source identity/version and Rust digest before effects. Only entry/output
limits change; selection/password/codepage/conflict/MOTW/work/dictionary/metadata/Job remain original.
Actual overflow/corruption/source change/completed output/incomplete cleanup never finances another
attempt,output merge or completed-user-file rollback. A second failure is terminal,no third prompt/attempt.

**Password / cost:** Required and explicit native BAD_PASSWORD24 stay in original finite Rust
worker_password no-output TEST preparation. Ambiguous auth/data is terminal, not guessed WrongPassword.
C# only forwards RespondPassword, never owns another retry loop. Preparation may decode full payload
before one original extraction; it can prepare an empty root,not close files. This is distinct
from cheap metadata preflight. RAR list uses original metadata-only RAR_SKIP; extraction retains its
metadata pass plus full RAR_TEST callbacks/integrity, all decoded bytes charged even on skip. A fresh
admission rereads/hashes headers/source, and password interaction may repeat original preparation.
No complete pre-extraction is added solely to estimate budgets. C# adds an eight-byte held-signature
probe on opt-in source acquisition; finite encoded scanning can now reach the existing default bound.

**Test environment:** NXTOOLS_RAR_EVIDENCE_ROOT / NXTOOLS_RAR_FIXTURES / NXTOOLS_RAR_HOST and
NXTOOLS_ARCHIVE_WORKER_FIXTURE select owned isolated evidence only; NXTOOLS_RAR_CHAIN_CASES is an
optional fixture filter and must be unset for the final13 scenarios. No public request keys added.
Existing DEBUG fixture thresholds only lower policy; Release cfg/strict unknown-field rejection is
source evidence unless separately runtime-tested. Never save secret-bearing requests or passwords.

## 4. Validation & Error Matrix
| Condition | Original required result |
| --- | --- |
| Opt-in RAR encoded length > deficient output allowance | Reach original metadata/declaration under separate finite encoded bound |
| Non-opt-in/open/TEST or non-RAR signature | No RAR encoded admission expansion |
| Shared/solid/split/unknown packed | No fabricated ratio; original declared/actual budgets still enforce |
| Wrong operation/generation/stale/larger Risk reply | Cannot approve current member; no output while pending |
| Known entry/total shortage, zero effects | ResourceLimit+source-bound shortfall; wait actual cleanup before OutputBudget |
| Cleanup held/incomplete,successfully closed output,mismatched ID/current limits or hard ceiling | No prompt/new attempt; native slot/leases remain owned |
| Actual UCM_PROCESSDATA overflow | ResourceLimit without shortfall; no invalid close/no third attempt |
| Corruption or source identity/version/digest change | Original typed failure,no retry/output authorization |
| Successful close then cancel | cancelled_with_committed_output and original completion journal/retained bytes |
| PasswordRequired / explicit WrongPassword | Original bounded no-output preparation, never extraction retry after effects |

## 5. Good/Base/Bad Cases
Good: tiny RAR5 total1 -> original member declaration7 -> actual cleanup -> explicit finite approval
-> fresh source-bound admission, exact7byte output. Base: ordinary RAR4/5 succeeds with no Risk at
production thresholds. Bad: count every part's UnpSize, use first solid member's packed value as an
independent ratio, release a cleanup-held slot or convert callback overflow into a declaration.

## 6. Tests Required
- `repo:rust/crates/nxtools-archive/tests/rar_readmission_contract.rs`: actual original worker RAR4/5
  total1 must reach bound declaration11/7 before output. Native counterpart below protects C# owner.
- `repo:rust/crates/nxtools-archive/tests/output_readmission_contract.rs`: incomplete cleanup/completed-output/
  ID/current-limit/hard-ceiling guards,explicit finite approvals and shared cancellation.
- `repo:rust/crates/nxtools-archive/tests/rar_readmission_chain.rs`: unchanged coordinator -> original
  native runner/Job/output; cleanupheld active1/sourcehelper protected/no prompt,then actual drain/
  active0/exclusive reopen; fresh IDs/options/source binding; selected/encrypted/solid declarations;
  actual overflow,second actual failure/no third attempt,completed-output cancel retained.
- `repo:csharp/NxTools.PlatformHost.Tests/RarRiskReadmissionTests.cs` and
  `repo:csharp/NxTools.PlatformHost.Tests/RarReadmissionHost.cs`: original RAR4/5 metadata/TEST/lowtotal,
  skip/overwrite,Required/WrongPassword preparation,cancel after prior close and exact native journal.
- `repo:csharp/NxTools.PlatformHost.Tests/RarRiskInteractionTests.cs`: genuinely compressed tiny
  RAR4/5 positive Risk at lowered DEBUG(1,1), production no false prompt, stale/larger/cancel replies;
  real RAR5 solid skip/full integrity/corrupt refusal; exact genuine volume declarations/unknown packed.
- `repo:tests/integration/prepare-rar-safety-fixtures.py` copies hash-attributed source data/uu/license
  and checks independent decoded literals/hashes. Controlled unknown-size header mutations are negative
  copies only, NOT independent RAR production or broad unknown-size/solid compatibility evidence.
Final matching isolated worker must follow strict/features checks; exact LIST=RUN=PASS names/order and
skip/fail/not_run + before/after worker hashes are required. No helper-only or source gate substitutes
for native lifecycle, independent acceptance, real UI/Explorer, all-method/encryption/volume matrices.

## 7. Wrong vs Correct
Wrong: clone 7z rules/signature blindly, raise current output limit to list RAR, invent a shared packed
ratio, or manually enlarge a second request and call it readmission.
Correct: actual RAR signature under original lease -> separate finite encoded admission -> original
metadata/plan/source-bound declaration -> actual zero-effect cleanup -> original explicit finite
coordinator -> one fresh normal admission; original RAR_TEST callbacks/integrity and ordinary
metadata+drain+close still own actual bytes and irreversible completed-output facts.
