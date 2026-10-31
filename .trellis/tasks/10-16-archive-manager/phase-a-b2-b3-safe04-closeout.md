# Phase A B2 / B3 / SAFE04 — bounded reconciliation and closeout

2026-10-24. **Bounded closeout completed; awaiting independent acceptance.** This report reconciles
original Phase A — Engine contracts and safety foundation requirements with current source owners
and later independent acceptances. It is NOT a new independent product acceptance, full B2/B3/SAFE04,
ENG03/ENG09/SAFE06/IPC01, Phase A/F or first-release PASS. Only the first current section of
[phase-a-checkpoint.md](phase-a-checkpoint.md) owns current status; this report is supporting evidence.

## Authority and classification

Read the original [Phase A work/exit and approved safety slices](implement.md), [PRD](prd.md),
[design](design.md), named [acceptance IDs](acceptance.md), applicable archive/native/shared owner
contracts and matching [reference note](../../../docs/reference-code/archive.md). The
[2026-10-20 exit ledger](phase-a-exit-ledger.md) remains immutable historical evidence. Its claims
that InteractionPort was not integrated and that B3 had no operation policy were true descriptions
of that checkpoint, not present defects. Later acceptance reports also retain their original
then-next-slice wording; the chronology below supersedes neither their facts nor their limitations.

Categories: **A** independently accepted subcondition, evidence reused; **C** correct implementation
with a confirmed necessary missing assertion; **B** reproducible product defect; **E** deferred or
unexecuted UI/native/Release evidence; **D** separate authorization/contract/cross-stage decision.
Source, compiled contracts and isolated native evidence are distinct; none establishes live-user or
Release acceptance. In this bounded comparison **no B or C item was confirmed**. No product/test
change, new test framework, extra negative fixture or compiled/native rerun is justified.

## Original requirement -> current owner -> independent evidence -> true remainder

| Original requirement | Current owner | Existing independent evidence (reused, NOT rerun) | Closure / true remainder |
| --- | --- | --- | --- |
| **B2: real operation password interaction**, not a standalone port mock | Rust `worker_password::execute` -> original `execute_attempt` / InteractionPort; C# original runner/source/Job/helper | [Password open/TEST](password-operation-acceptance.md): actual 7-scenario fixture; [extract preparation](extract-password-acceptance.md): 9 scenarios across encrypted ZIP/7z/RAR4/5, stale/cancel/options/postcommit | **A:** original-operation finite preparation, typed-only retry, source lifetime and single extraction invocation close the historical missing integration. **E:** physical password/focus/IME/UI acceptance remains deferred, not full B2. |
| **B2: conflict choices / task-local apply-all / stale and cancel** | Rust `worker_conflict::ExtractionConflict`, PrivateOutput, original ticket owner; C# target observation / ArchiveOutputSession | [ZIP](zip-conflict-acceptance.md): 16-scenario fixture; [7z/RAR/TAR](format-conflict-acceptance.md): 13 scenarios; [standalone streams](stream-conflict-acceptance.md): 7 scenarios | **A:** all currently supported extraction families use the existing conflict strategy; Skip retains decoder/integrity/budget/source checks, no member output. TGZ is the conflict native representative for shared compressed-TAR wrappers, not every wrapper x policy. **E:** real dialog/user acceptance. No confirmed unconnected format or necessary per-format copy of shared async tests. |
| **B3: ratio warning and finite original-admission confirmation** | Rust `zip_risk`, InteractionPort, worker/isolated `extraction_preflight`, format confirm hooks | [ZIP Risk](zip-risk-acceptance.md), [7z](sevenz-risk-readmission-acceptance.md), [RAR4/5](rar-risk-readmission-acceptance.md), [TAR/streams](tar-stream-risk-readmission-acceptance.md) | **A:** strict >64MiB AND >1000x reliable positive packed bytes, exact original ceiling/current ticket, no current-member partial/decode/write/publication before Risk. Shared/solid/split/unknown remains unknown. **E:** dedicated Release fixture-field rejection runtime assertion NOT RUN; production-size benign expansion/resource observation is not supplied by DEBUG positives. |
| **B3: one explicit finite output-budget readmission**, never midstream budget mutation | Rust `output_readmission::execute` -> `product_job` -> Desktop `readmission.rs`; original C# Job/runner exited cleanup/source/output | [ZIP readmission](zip-output-readmission-acceptance.md): 1 test/5 scenarios; later 7z 1/10, RAR 1/13, TAR/stream 1/17 actual coordinator/native chains in their linked acceptances | **A:** reliable complete original-plan declaration, zero commits/publication, matching limits/attempt, actual cleanup before prompt; one fresh source-bound attempt with original options/cancel and unchanged non-output budgets. Actual overflow/corruption/source change/commits/unclean facts cannot finance retry. TAR/stream second actual ResourceLimit no-third assertion is **coordinator fact injection only**, not native overflow evidence and not a task to manufacture. |
| **SAFE04: metadata / declared and actual output / dictionary / native process bounds; benign finite approval** | Rust engine/format scans, sevenz budget and output counters; C# ArchiveWorkerBudget/Job and original source/output owners | Historical [ledger K/Z/H references](phase-a-exit-ledger.md), [resource review](resource-acceptance-review.md), later [tool repair](resource-tool-repair-acceptance.md) + [summary repair](resource-summary-p2-acceptance.md), [finite100k](product-100k-post-status-acceptance.md), plus four safety acceptances above | **A:** existing bounded metadata/work/Job protections and named small real actual-overflow / finite approval subconditions; later format hooks no longer a missing feature. **E/D:** full original SAFE04 resource/native/Release disposition remains unaccepted; reservations and sampled Debug peaks are not per-allocation fences, absolute RSS or broad resource proof. No unbounded bomb, large/performance matrix or arbitrary-call interruption is implied. No new concrete source/assertion gap found. |
| **ENG03: certain password typing vs damaged authentication/data, cancellation, selected subtree/custom destination/options** | Rust password preparation + original immutable request/selection/source bind; native extraction is invoked once after TEST | Password/extract acceptances above; original crypto/error evidence in ledger; [Phase D](phase-d-operation-acceptance.md) selected-source/native seam and later format selection/readmission chains | **A:** typed PasswordRequired/explicit WrongPassword only, ambiguous AuthenticationOrDataError terminal, original intent and commit truth retained; no secret argv/log/persistence. Representative seams compose without demanding every format x password x selection x destination case. **E:** real UI and full original ENG03 acceptance not asserted. |
| **ENG09: paused/chunk/prompt/finalization cancellation; truthful busy and cleanup** | OperationControl / InteractionPort; original runner/Job/resource disposal; Desktop cleanup-held FIFO | [Worker cancellation](worker-cancel-acceptance.md), password/conflict waiting-cancel cases, four readmission chains; [Phase D](phase-d-operation-acceptance.md) and existing Desktop job-owner tests | **A:** named small cooperative receive/observe/credit/pause, real operation prompt cancel, held slots until actual cleanup and postpublication journal preservation. **D/E:** original literal 1GiB versus bounded cooperative timing/cross-stage signoff remains a disposition, not a newly demonstrated source bug; <=200ms is not arbitrary blocked OS/codec interruption. No fresh timing run. |
| **SAFE06: overwrite/skip/rename/ask-all, target replaced/edited/missing/appeared/type/reparse/busy** | Original C# held-parent/observation/fullID/version/final recheck/atomic path publication; Rust task policy | Original overwrite/native outcome evidence retained by ledger and later ZIP/format/stream conflict and Phase D acceptances | **A:** named native checks/refusals, task-local apply-all, race-safe no-replace rename and cancellation facts close historical B2 conflict gap. Approved recheck-close -> atomic-path TOCTOU and native32/5 no-generic-fallback remain; no target-ID CAS is asserted. **E:** live filesystem/Shell/user matrix remains unaccepted; strict source recycling is not this output contract. |
| **IPC01: bounded symmetric touched payloads; invalid enum/count/path/NUL/capability/version** | Rust worker/private wire, `nxtools-protocol::archive_job`, IPC generation/receipt; C# ArchiveJobWire/Session and runner capability checks | Password/conflict/Risk/readmission strict-wire acceptance unions; Phase D independent Core/IPC/Protocol and 13 native representatives | **A:** existing touched B2/B3 payload integration/typed validation/read-only separation has evidence, not merely future DTO plans. No new payload in this round, so no new IPC round-trip obligation. **E:** dedicated Release DEBUG-field runtime rejection remains unexecuted; later unrelated protocol/workspace changes are not accepted wholesale. |

B1/delivered C4 and C1/C2/C3/C5, ordinary CLI and CLI lifecycle remain independently accepted and
closed; they were not reimplemented, built or rerun. The historical Phase D 973/6 full-suite failure
is retained; the later [six-fixture Root acceptance](phase-d-fixture-root-acceptance.md) closed that
specific blocker with a distinct 979/979 run. Neither is a fresh current-workspace result.

## Current source trace and retained invariants

The comparison inspected these production owners rather than inferring implementation from report
headings or the cumulative dirty HEAD diff:

- [worker_password.rs](../../../rust/crates/nxtools-archive/src/worker_password.rs): fresh no-output
  TEST preparation, at most 3 typed-password attempts, original source/options/shared cancellation;
  successful preparation invokes the original extraction once and immediately returns its result.
  Readmission preparation retains the existing bounded preparation allowance; it does not grant
  extraction output or C# policy authority. Full TEST plus extraction and failed-attempt decode
  prefixes remain explicit existing costs. Rust private buffers are zeroized; C# managed strings
  are not claimed locked or GC-zeroized.
- [interaction.rs](../../../rust/crates/nxtools-archive/src/interaction.rs) and
  [worker_conflict.rs](../../../rust/crates/nxtools-archive/src/worker_conflict.rs): exact current
  operation/prompt/generation matching, one response, cancel wakes the original wait, local-only
  apply-all and bounded no-replace numbering. C# forwards transport facts, not a second policy loop.
- [zip_risk.rs](../../../rust/crates/nxtools-archive/src/zip_risk.rs),
  [worker.rs](../../../rust/crates/nxtools-archive/src/worker.rs),
  [worker_codecs.rs](../../../rust/crates/nxtools-archive/src/worker_codecs.rs) and
  [PrivateOutput](../../../rust/crates/nxtools-archive/src/worker_output.rs): overflow-safe u128 ratio,
  immutable original ceiling, path/selection/source/plan before current-member Risk; unknown
  declarations never create a guessed shortage. ZIP/XZ/Zstd confirm hooks and original isolated
  7z/RAR preflight retain actual decoder/counter/integrity checks. TAR packed size stays None and
  has no payload checksum. Compound TAR retains its existing full-integrity finite outer spool.
- [output_readmission.rs](../../../rust/crates/nxtools-archive/src/output_readmission.rs): first
  result must match operation, actual cleanup, zero commits, ResourceLimit, current limits and
  source binding; required/approved output stays within 32GiB/file and 2^50 total. Only entry/output
  fields and required source bookkeeping change on a new control ID. Second result returns directly.
  [product_job.rs](../../../rust/crates/nxtools-archive/src/product_job.rs) clones host intent and
  projects actual native terminals, not private complete/ACK. Source native identity/version and
  Rust digest are distinct checks, not substituted by the logical correlation ID.
- Desktop [readmission.rs](../../../rust/apps/nxtools-desktop/src/archive/readmission.rs),
  [jobs.rs](../../../rust/apps/nxtools-desktop/src/archive/jobs.rs) and existing forms/application
  defer boundaries connect the original coordinator, bounded FIFO, tickets and terminal facts;
  cleanup=false cannot free a slot. The GPUI entity/render ownership is unchanged. This inspection
  is not physical UI acceptance and introduces no component/layout changes.
- C# [runner cleanup](../../../csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs)
  waits for process/Job/pumps, disposes source/output/helper resources, then emits exited with
  actual cleanup truth; stuck resources remain rooted. [ArchiveJobSession](../../../csharp/NxTools.PlatformHost/Protocol/ArchiveJobSession.cs)
  does not finish on private complete. Native publication/unknown/committed facts outrank late
  cancellation; file counts, bytes and journal survive. No rollback/deletion of replaced user files.

Source/test navigation additionally checked the existing `output_readmission_contract.rs`
(unclean/commit/mismatch/hard cap, actual failure, finite approval/generation/shared cancellation),
`interaction_contract.rs`, `worker_wire.rs`, `job_execution_contract.rs`, Desktop `job_tests.rs`,
`ArchiveJobTests.cs` and the separately labeled `tar_stream_readmission_guard.rs`. Existing
independent execution remains the runtime evidence; this round does not relabel source reading as
execution or compile every concurrently dirty file.

Encoded-source permission remains format-specific: original extract+readmission opt-in only;
7z/RAR use max(original output, existing default100GiB) after held actual-signature admission;
TAR/streams use max(original output, original metadata), not 100GiB. Compound spool and actual target
output are separate bounds. No output/work/dictionary/metadata/Job enlargement is implied by encoded
admission, Risk or password preparation. Shared solid/split/ambiguous attribution is never divided
into fabricated member ratios.

Risk protects the CURRENT MEMBER, not a new global directory rollback policy. The TAR/stream
independent report's two direct non-Job probes observed a missing root prepared and retained EMPTY
before Risk decline, with zero partial/write/publication and actual cleanup. That inherited behavior
remains unchanged; Job destination confirmation retains its original owner. Earlier committed
members also remain, rather than being rolled back by a later decline/cancel.

## Precise remaining conditions — no automatic next task

1. **Source/assertions:** no confirmed necessary missing behavior or boundary assertion in this
   authorized B2/B3/SAFE04 comparison. Close those historical missing-integration subconditions,
   not the entire original acceptance IDs. This round awaits independent acceptance of the closeout.
2. **UI/live native (E):** forms/password/conflict/Risk/budget prompts, keyboard/focus/IME/DPI/UIA,
   Explorer/foreign COM and applicable real process-window/user acceptance remain DEFERRED / NOT RUN.
   No gesture/sample or GUI action is requested; no launcher workaround is authorized.
3. **Evidence limitations (E):** dedicated Release test-field rejection remains NOT RUN; cfg plus
   strict parser source and a Release build are not that runtime assertion. TAR/stream native
   second actual ResourceLimit remains NOT PROVEN; native false-FCS produced CorruptArchive, while
   no-third-on-second-ResourceLimit is a coordinator injected-facts guard. Preserve the failed old
   expectation; do not fabricate more native negatives to turn that into PASS. Full resource/native/
   Release SAFE04 and original timing/first-paint disposition are not accepted by small format slices.
4. **Decisions (D):** trusted UNC requires a separately decided capability/network fixture, not only
   another test; strict original-object source recycling remains `native_protection_unproven` /
   `NotAttempted`, unavailable/default-off, with no permanent/path-delete or output-TOCTOU substitute.
   JOB07/SAFE08 and literal cross-stage Phase A exits remain visible. No safety reduction or phase
   reorder is inferred. P206/P207 live gates and P209 remain outside this round;
   `ProductApplyAuthorized=false` stays unchanged.
5. **Previously closed/excluded:** no automatic reopening of B1/C1-C5, CLI/lifecycle, accepted format
   matrices, package28 or other debts. Package28 is inherited historical context only: no running
   product, installation, service, updater, protected directory or release-ledger observation here.

## This round: changes, verification and preservation

Only this additive report and a new top section in the sole checkpoint are authorized workspace
writes. Product/tests/dependencies/vendor/reference/spec/task registration/history stay untouched.
One evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\phase-a-b2-b3-safe04-closeout-20261024-220134`.
Opening baseline is 709 scoped source/document files, not artifacts/protected/runtime state.
Immediately before documentation writes: **709/709 unchanged, missing0**, recorded in
`prewrite-preservation.json`. The initial PowerShell ConvertFrom-Json array-wrap comparison counted
1 row incorrectly; its raw `prewrite-powershell-array-error.json` is retained, not file-drift evidence.
The corrected explicit Python per-file SHA256 comparison supplies the 709 result.

Executed documentation-only checks (`verified-*` raw logs / `commands.jsonl`):

| Check | Actual result |
| --- | --- |
| `python tests/architecture/spec-contracts.py` | PASS,114documents/14workspace owners; source/navigation only |
| `python .trellis/scripts/task.py validate .trellis/tasks/10-16-archive-manager` | PASS,82implement/79check entries;8context-size warnings across4existing large documents retained |
| Scoped `git diff --check` + new-document whitespace/conflict/EOF checks | PASS; no product verification implied |
| Requirement table / local file links / current-state owner | 9rows covering all7requested IDs,40local links valid,exactly1current checkpoint |
| Opening SHA256 and history comparison | 708/709unchanged;only authorized checkpoint changed;new report additional;missing0,all historical checkpoint body bytes identical after its retained-title relabel |

No concurrent drift was found within the709-file measured scope; this is not a frozen whole-workspace
claim or acceptance of unrelated dirty work. `verified-preservation-links.json` records exact paths
and hashes. The final post-record pass is retained separately under `final-*` with the same checks;
it does not rerun product/native evidence. Initial console-encoding interruption and rejected Edit
payloads are preserved in `tool-errors.txt`; initial spec/task exits were0, but no later check was
credited until the corrected UTF-8 static run completed. No historical failure was rewritten.
No optional delegate review was needed. No Cargo/dotnet/native/full Desktop/CLI/accepted format/
100k/large/long-path/performance execution, package/sign/install/sequence reservation, product launch,
process/service/updater/protected-state audit, Registry/COM/UNC/recycling/elevation/commit/push.

**Stop: Phase A B2／B3／SAFE04有界收口完成，等待独立验收。**
No whole-phase/first-release approval and no automatic UI resumption, debt progression or package.
