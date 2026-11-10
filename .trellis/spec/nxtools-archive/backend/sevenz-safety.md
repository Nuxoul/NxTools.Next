# 7z extraction risk and finite output readmission

## 1. Scope / Trigger
The bounded Phase A safety remainder uses locked sevenz-rust2 0.23.0, the original InteractionPort,
ZipRisk and output_readmission coordinator. It does not add a backend, retry runtime, UI/CLI authority
or accept full B3/SAFE04/A–F. Current delivery status belongs only to the
[task checkpoint](../../../tasks/10-16-archive-manager/phase-a-checkpoint.md).
Read the [verified metadata reference](../../../../docs/reference-code/archive.md#7z-riskreadmission-exact-metadata-follow-up-2026-10-24)
and [product job contract](product-jobs.md) before changing these boundaries.
Evidence: `repo:rust/crates/nxtools-archive/src/worker_codecs.rs`,
`repo:rust/crates/nxtools-archive/src/worker_codecs/volumes.rs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVolumeSources.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs`.

## 2. Signatures
- `sevenz_member_packed(archive: &sevenz_rust2::Archive, index: usize, block_members: &[u64]) -> Option<u64>`.
- `SourceSet::open(request: &WorkerRequest, control: &OperationControl) -> Result<Self>` retains
  the original encoded-source limit; `open_sevenz(request, control) -> Result<Self>` selects the
  finite 7z-specific encoded limit before calling private `open_bounded(..., encoded_limit)`.
- `ArchiveVolumeSources.Open(string path, ulong maxMetadata, ulong maxEncoded,
  bool sevenzOutputPreflight = false)` acquires original leases; private `Add` returns the selected
  encoded bound. Original runner passes true only for extract with OutputReadmission present.
- Unchanged `output_readmission::execute(request, control, interaction, run)` owns at-most-one fresh
  attempt. The `run` callback must return `AttemptResult` only after actual native cleanup.

## 3. Contracts
**Member attribution:** file_block_index and block-first-pack indexes, not compressed_size or global
is_solid(), determine ownership. A streamed member has a known packed denominator only when its block
has exactly one streamed member and a uniquely attributed single packed stream in currently admitted
single-input coder chains. Checked subtraction must give one; checked sum must not overflow. Empty,
shared solid, ambiguous/orphan or unsupported multi-input spans return None. Zero packed bytes never
qualify for a ratio. No division or whole-archive/solid-block ratio is substituted for a member ratio.

**Risk:** strictly declared output >64MiB AND >1000 times reliable positive packed bytes, overflow-safe.
The original operation/ticket/generation and exact admitted entry ceiling bind approval. Risk cannot
raise any output/work/dictionary/metadata/Job budget. Decline/cancel stops subsequent output; prior
successfully closed facts survive. Read/TEST have no extraction Risk interaction; CLI cannot silently approve.

**Encoded source admission:** ONLY opt-in 7z extraction readmission uses
max(original max_output_bytes, existing default 100GiB) as its finite encoded-source ceiling. C# reads
six signature bytes only after the original source lease is held, and applies that ceiling only to
actual 7z. Rust uses the separate sevenz entry after format dispatch. Ordered family acquisition,
native identity/version and digest checks remain original. RAR and non-opt-in read/TEST behavior do
not gain this bound. This is not an output allowance or a Job/work/dictionary/metadata increase.

**Shortage:** original complete metadata/path/selection/format/source plan precedes preflight. Fully
known regular-member max/sum may produce OutputShortfall {current, required, source}; declarations
are admission information, not integrity proof. Original AttemptResult must match operation/current
limits,ResourceLimit,zero successfully closed files and actual cleanup_complete. A valid source binding and
explicit finite OutputBudget approval (entry <=32GiB,total <=2^50,covering required) permit one new
operation/control ID under the same logical ID/shared cancellation. Only entry/output limits change;
selection/codepage/password/conflict/MOTW and work/dictionary/metadata/Job limits are preserved.
Fresh normal admission verifies original native source identity/version and Rust digest again.
No third attempt,output merging,automatic approval or rollback of completed user files.

**Costs:** an unsuccessful declared preflight reads/hashes source and headers, then a fresh attempt
reads/verifies again. C# adds a six-byte held-source signature probe. No new complete payload decode
is used to estimate budgets. Existing password preparation can already perform a whole-archive
no-output TEST; preserve/disclose that separate cost. Solid selected extraction may decode unselected
bytes; those actual decoded bytes remain bounded, even when selected declared sizes fit the allowance.

**Test-only environment:** NXTOOLS_SEVENZ_EVIDENCE_ROOT, NXTOOLS_SEVENZ_FIXTURES,
NXTOOLS_SEVENZ_HOST and NXTOOLS_ARCHIVE_WORKER_FIXTURE select explicitly owned scratch fixtures.
NXTOOLS_SEVENZ_CHAIN_CASES is an optional test-only filter; unset it for the recorded full ten scenarios.
These keys add no product options. Existing DEBUG-only zip_risk_fixture may only lower thresholds;
Release omits that field under cfg(debug_assertions) and strict deny_unknown_fields rejects it.
No secrets go to argv, persisted requests or evidence; fixture audit records password presence only.

## 4. Validation & Error Matrix
| Condition | Required boundary result |
| --- | --- |
| Low output below encoded 7z length, opt-in extraction | Reach original plan; do not fail encoded-source acquisition solely at that low output limit |
| Ordinary member or shared/unknown packed | No fabricated Risk prompt; original actual byte limits remain active |
| Stale/cross-operation/duplicate/wrong-generation/larger Risk reply | Cannot authorize current output |
| Reliable entry/total declaration shortage before effects | ResourceLimit + matching bound facts; wait actual exited cleanup before OutputBudget prompt |
| Incomplete cleanup,wrong attempt/current limits,successfully closed output or invalid hard ceiling | No new attempt |
| Actual decode/output overflow or corrupt payload | Terminal original failure,no shortfall or budget prompt |
| Source identity/version/digest changes between attempts | source_changed/SourceChanged,no output and no further retry |
| Second attempt fails or closes output before cancellation | No third attempt; retain actual completion/journal/cleanup facts |
| Release receives DEBUG fixture field | Strict request rejection, not a relaxed production threshold |

## 5. Good/Base/Bad Cases
Good: tiny solid declared total6144 with total1 -> zero output and actual cleanup -> explicit finite
approval -> new source-bound attempt, unchanged memory and options. Base: tiny ordinary nonsolid
extract succeeds once without Risk. Bad: selected solid output4096 fits but full block decode6144
exceeds total4096; actual overflow is terminal, not a guessed declaration shortage.

## 6. Tests Required
- `repo:rust/crates/nxtools-archive/tests/sevenz_readmission_contract.rs`: actual worker with encoded
  length above total1 reaches original declaration shortfall before any output.
- `repo:rust/crates/nxtools-archive/src/worker_codecs.rs#packed_risk_tests`: orphan trailing packed
  stream cannot dilute member ratio; this structural metadata test is not a payload decoder oracle.
- `repo:rust/crates/nxtools-archive/tests/output_readmission_contract.rs`: malformed cleanup/completed-output/
  ID/limit facts,finite ceilings,ticket retirement and shared cancellation never create unsafe retry.
- `repo:rust/crates/nxtools-archive/tests/sevenz_readmission_chain.rs`: unchanged Rust coordinator ->
  original native runner/Job/output,held-cleanup barrier (active1/source+helper protected/no prompt),
  actual drain (active0/exclusive reopen),new IDs,preserved request/options/memory/source binding,
  selected actual overflow/no third attempt and completed journal truth.
- `repo:csharp/NxTools.PlatformHost.Tests/SevenzRiskReadmissionTests.cs` with
  `repo:csharp/NxTools.PlatformHost.Tests/SevenzReadmissionHost.cs`: actual worker/native output,
  Risk approve/decline/cancel/stale/larger replies,shared packed,ordinary and no-output TEST,
  solid skip/integrity and truthful close/journal. Assert exact LIST/RUN/PASS names/order and all
  skip/fail/not_run counts plus matching final worker hash, not just process exit zero.
- `repo:tests/integration/prepare-sevenz-safety-fixtures.py`: independently stock-produced tiny
  solid/nonsolid and owned corruption; list/test/x-so compared to fixed literal member bytes.
Implementation checks are not independent acceptance, real UI/Explorer or a format/method/encryption/
volume matrix. Dedicated Release runtime fixture rejection must be labeled separately from source proof.

## 7. Wrong vs Correct
Wrong: use entry.compressed_size, share/divide solid packed bytes, count orphan pack sizes, raise
original output limits to reach headers, or dispatch a retry on worker completion before native drain.
Correct: conservative unique attribution -> original Risk; separate finite encoded admission -> full
original declaration preflight -> actual zero-effect cleanup -> original finite explicit coordinator
-> one normally admitted fresh source-bound attempt. Keep unknown ratio and actual overflow distinct.
