# Archive typed errors and name safety

Evidence: `repo:rust/crates/nxtools-archive/src/error.rs#ArchiveError`, `repo:rust/crates/nxtools-archive/src/path_policy.rs#annotate`, `repo:rust/crates/nxtools-archive/src/encoding.rs#decode`.

ArchiveError Display/Debug contains fixed categories, never raw backend messages/names/paths. Missing password is derived from read encryption flags, not matched error prose. Uncertain/ambiguous authentication must not become a guessed WrongPassword. Actual CRC/header/payload mismatch is CorruptArchive; finite admission/actual-byte overflow is ResourceLimit; stale source is SourceChanged; cooperative cancel is Cancelled, not empty success.

Private native results preserve explicit budget categories: `worker_output::PrivateOutput::call_payload` maps `resource_limit`, `write_limit`, `read_limit` and `session_entry_limit` to ResourceLimit. The original C# `ArchiveVolumeSources.CheckEncoded` can return `resource_limit` during `source_acquire`, before decoding or any output, including non-readmission7z with a deficient encoded allowance. Do not collapse that refusal into IoFailure or raise the allowance; `source_unavailable` and unclassified I/O failures retain IoFailure. `worker_extraction_contract::sevenz_native_source_admission_preserves_resource_limit_before_output` drives the real worker/native-reply boundary and asserts exact terminal, zero subsequent native calls and no shortfall/retry. `ArchiveExtractionWorkerTests.ActualSevenzToHandleSession` retains the actual low-budget ResourceLimit assertion plus later collision/corruption, sentinel and cleanup cases.

Browser-selected member output has a distinct fixed `SelectedOutputLimit` / `selected_output_limit` category at known-size selected admission and the original native cumulative output quota. Its general decoder entry/output work ceilings remain finite and independent; source acquisition, metadata, compound spool, solid work and working-memory exhaustion remain ResourceLimit. Preview maps only SelectedOutputLimit to its configured member-size message and keeps ResourceLimit as ReadLimit. Ordinary output retains its existing `write_limit` category. See [preview budget contract](../../nxtools-desktop/backend/archive-preview.md).

Keep unsafe/duplicate records visible; no sanitization/IndexMap swallowing. Authoritative UTF8/valid Unicode extra precedes explicit codepage; Auto uncertain names stay manual. Names are not implicitly NFC-normalized by native output; any requested normalization must rerun collision validation. Lexical policy is not junction safety: C# handle-relative no-reparse fixture and real ZIP output chain provide only their scoped evidence. WrongPassword is used only for native UnRAR's explicit code24; older RAR/7z ambiguous auth/data stays AuthenticationOrDataError. Native replacement32 is preserved as an in-scope failure, never pretend success or unsafe retry.

## Creation-verifier I/O provenance

Evidence: `repo:rust/crates/nxtools-archive/src/worker_sevenz_create.rs#VerificationInput`,
`repo:rust/crates/nxtools-archive/src/worker_sevenz_create/tests.rs`.
The owned7zverification spool tags/retains the first underlying Read/Seek failure before
signature/header preflight,ArchiveReader construction or payload decoder wrappers. Return that
typed failure even if a callback/parser later projects corruption. Raw storage Other/InvalidData
is IoFailure; OutOfMemory is ResourceLimit; existing ArchiveError is preserved. EOF/truncation,
format/CRC/hash/plan failure remains CorruptArchive. Interrupted remains retryable,not latched.
An untagged decoder Other is not proof of storage failure. Do not match error text, classify
every decoder failure as IoFailure or expand this repair to unrelated codecs. Tests must drive
actual creation/full-held-readback/verifier/abort,not just an isolated conversion helper.

## Scenario: ZIP Auto candidates and safe reprojection
### 1. Scope / Trigger
ZIP names without authoritative Unicode, or a changed manual page. Other formats are unchanged.
### 2. Signatures
`ArchiveEngine::reproject(&ArchiveSnapshot, NameEncoding, &OperationControl) -> Result<ArchiveSnapshot>`;
`ExtractionPlan::build` then `ArchiveEngine::extract`. Background/worker calls only, like open.
### 3. Contracts
UTF8 flag/valid CRC-bound Unicode extra precede explicit page, then ZIP Auto. Auto considers
only strict UTF8/GBK/ShiftJIS; at least four non-ASCII letters, script checks, and one plausible
candidate. ShiftJIS needs two full-width kana to select; all-Han/half-width-only alternatives
can still veto another candidate. Short/multiple candidates retain UncertainCp437/manual choice.
Heuristic(page) retains UncertainEncoding, never Unicode authority or numeric confidence.
Worker/product existing provenance/codepage fields admit Heuristic only65001/936/932; C# only
for Auto. No DTO shape, version, helper trust or product limit change. Candidate scratch is
bounded before candidate decode; no archive-wide voting or retained candidate collection.
Reprojection checks original source revision before/after directory reads and preserves session/
ordinal/raw bytes. Existing annotation and plan reject new unsafe/colliding paths. Plans share
their immutable entries Arc; extraction rejects a different projection before output calls.
### 4. Validation & Error Matrix
Changed source/stale projection plan -> SourceChanged; unsafe -> UnsafePath; case/file-prefix
collision -> OutputCollision; finite allocation -> ResourceLimit; cancellation -> Cancelled.
### 5. Good/Base/Bad Cases
Good: unique script evidence or honest ambiguity. Base: manual pages and authoritative Unicode.
Bad: decode-success-only detection, locale priority, sanitization, or old-plan reuse after switch.
### 6. Tests Required
`repo:rust/crates/nxtools-archive/tests/encoding_contract.rs` checks independent byte literals,
identity, Unicode/manual precedence, ambiguity, original checks and zero output for stale plans.
`repo:rust/crates/nxtools-archive/tests/product_read_chain.rs` checks actual candidate metadata;
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveEncodingTests.cs` checks original native bytes/names
and zero writes on refusal. Exact C# LIST/RUN, no SKIP; no whole-language matrix required.
### 7. Wrong vs Correct
Wrong: same session means an old plan is valid. Correct: preserve entry IDs, rebuild the plan
for the immutable new projection, and refuse mismatched projection at extraction admission.

## Scenario: Original worker password interaction (B2 password portion)
### 1. Scope / Trigger
Opt-in private schema2 open/test and ZIP/7z/RAR4/5 extract after one original source acquisition.
Extract uses no-output TEST preparation,not extraction retries. No ArchiveRead password/write/UI.
### 2. Signatures
`WorkerRequest.password_interaction: Option<bool>` (absent by default, only true allowed);
`InteractionPort::request_notified(Password, original_control, notify)` installs pending first.
C# `ArchiveWorkerRequest.PasswordInteraction` and `RespondPassword(id, ArchivePasswordTicket,
string?)` forward private `password_reply {ticket,password}`; null is user Cancel.
### 3. Contracts
Rust owns at most3actual read/preparation attempts (initial+2manual retries), fresh attempt
OperationControl ID sharing original cancel/lifetime. Frames retain original logical operation_id
plus password_state attempt_id/number. Open/test semantics unchanged. For extract, create fresh
WorkerRequest::open as operation=test with same path/codepage/limits and only private password;
no conflict/MOTW/delete/readmission/write options enter preparation. Original extract request
retains ALL supported options. Success emits extracting then calls original execute_attempt ONCE
with ORIGINAL control/native operation_id. Its every result is terminal,never restarts password
preparation,including Required/WrongPassword/corrupt/conflict/cancel/committed outcomes.
Only PasswordRequired/explicit WrongPassword request another password. Existing secret type,
private input and zeroed buffers; no argv/config/diagnostic/metadata/terminal secrets. C# only
transports; original runner/Job/source/image/slots/timeout and budgets retain lifecycle ownership.
Negotiate password_interaction_v1; absent option leaves old requests/terminals unchanged.
Extract additionally negotiates password_extract_v1; open/test/legacy capability behavior stays.
ZIP open cannot verify payload key;7z/RAR open may only validate headers. Existing full TEST is
the no-output boundary for all encrypted extraction: same authentication/CRC/size/budgets/source
checks,zero partials/publications. Success cost is one full test plus one extraction payload decode;
RAR extraction keeps existing metadata-only first pass. No third full verification decode added.
ZIP ZipCrypto/AES failures remain AuthenticationOrDataError;7z MaybeBadPassword stays ambiguous;
RAR explicit ERAR_BAD_PASSWORD24 alone maps WrongPassword,encrypted BAD_DATA stays ambiguous.
### 4. Validation & Error Matrix
WrongPassword -> awaiting_password retry; AuthenticationOrDataError -> terminal, no loop.
Third password failure -> same typed failure, no fourth attempt. Cancel/EOF -> original cleanup,
no later attempt; old/duplicate ticket -> ignored by existing port, not applied to new prompt.
Option on schema1/ArchiveRead/create -> reject; extract on nonZIP/7z/RAR signature -> UnsupportedOperation.
Password<=4096UTF8bytes,noNUL; original typed Cancel/EOF cleans resources without extraction.
### 5. Good/Base/Bad Cases
Good: RAR5missing->pending->manualwrong->explicitWrongPassword->manualcorrect->full test.
Base: unencrypted or old explicit-password path unchanged. Bad: Test remapped to Open, guessed
password error, retry after cancel, or publishing files then silently overwriting on retry.
### 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/ArchivePasswordTests.cs` uses original runner/Job/actual
worker: pending, correct open/test, WrongPassword, stale ticket, both cancel routes, finite cap,
ambiguous damage, secret-free frames/logs, source/image held while pending/released after exit.
`repo:rust/crates/nxtools-archive/tests/worker_wire.rs` preserves old/private-read boundaries;
existing InteractionPort/ZIP AES tests retain ticket/cancel and ambiguous classification.
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveExtractPasswordTests.cs` additionally drives real
ZIP/7z/RAR4/5 missing->pending->correct->nativebytes,explicitRAR5correction/staleticket,waitCancel,
post-password conflict option,correctpassword+damage terminal and nativepostcommitcancel. Assert
zero output while preparing,oneextracting/worker,originalnativecorrelation,counts/cleanup/privacy.
Await actual exited event before asserting events: original runner removes active slot before
publishing exited, so DrainAsync alone is not evidence that terminal has reached the observer.
### 7. Wrong vs Correct
Wrong: allow extract in the retry loop,then republish earlier outputs after a password failure.
Correct: existing finite InteractionPort loop runs read-only preparation; one original-ID extraction
returns its native outcome without any password retry or new runtime.

## Scenario: Extraction ordinary-target conflicts and direct final-name output
### 1. Scope / Trigger
Opt-in ZIP,7z,RAR4/5,TAR and supported compressed TAR,plus standalone Gzip/XZ/BZip2/Zstd
ordinary extraction after original source/path/plan preflight. This includes known-destination
button/dialog/context,Smart/Here/Name/Desktop/custom and destination-directory drag. Internal
duplicate/case/file-prefix/unsafe entries reject BEFORE native begin or prompts. Virtual cache,
archive Create/Edit,HZ5 and ArchiveRead retain separate contracts.
### 2. Signatures
Private schema2 `WorkerRequest.conflict_policy: Option<ConflictPolicy>` remains ask/overwrite/skip/
auto_rename,absent by default. Shared ExtractionConflict uses the existing InteractionPort/ticket/
conflict_prompt/conflict_reply; C# RespondConflict forwards typed input only.
`ArchiveOutput::begin_extracted_file -> Option<token>` starts final-name `CREATE_NEW`.
An existing target returns `success:false,error_code:target_exists,existing_bytes` without a token.
`begin_overwrite` directly opens/truncates the same relative path. Ordinary `commit` carries
`OutputMetadata` and closes. Require `extraction_conflict_v1` plus private
`extraction_output_v1` inside bulk v1; retain the old ZIP compatibility marker only as wire history.
### 3. Contracts
Rust owns strategy,current-attempt ApplyAll,numbered-name selection,skipped count,metadata and
format integrity. C# owns held no-reparse parents/components,final file handles,writes,metadata APIs,
close/delete and native codes. Initial begin uses `CREATE_NEW` on the final validated component.
There is no sibling `.nxpart`,rename/replacement,`observe_target`,target signature/CAS,per-write hash,
output content/file-ID fingerprint,per-file VerifyDirectory,published reopen or FlushFileBuffers.

The target-exists response supplies existing bytes for the prompt. Overwrite consumes the current
decision and directly truncates that same observed relative path; a path-kind/reparse/access failure
is the owning native result,not a blind retry or replacement candidate. AutoRename tries bounded
`name (N).ext` final names with `CREATE_NEW`; target_exists advances to the next candidate and never overwrites.
Skip never begins a handle and makes source deletion/recycling ineligible. Stale/duplicate tickets,
retired attempts and cross-job ApplyAll cannot act.

Rust sends <=32KiB coalesced records through the original anonymous pipe without per-record ACK;
the single Host stdout pump consumes sequentially and retains the first native write failure/code.
The next synchronous commit drains prior records,returns that sticky failure if present,applies
mtime/readonly/hidden/MOTW from Rust and closes. This is an ordering barrier,not durability flush.
An uncontended new file has exactly begin+commit synchronous JSON calls.

7z still decodes skipped members through bounded per-file counters/CRC/size; RAR still uses
RAR_TEST/null destinations and callback counters when token=None. Never RAR_SKIP: solid/sequential
state and integrity remain valid. TAR copies skipped payload to sink under the same limits; compound
TAR still verifies its full outer spool. Standalone streams use the same begin/strategy/summary and
fully decode Skip for end/checksum/actual-byte truth.

Only outputs whose required codec/source integrity is complete may commit/close. ZIP/independent
members can complete at their member boundary; solid RAR/7z or delayed format integrity keeps a
bounded set of owned final-name handles incomplete. First failure/cancel closes and deletes only
those incomplete owned outputs. Earlier successfully closed files and finalized directories remain;
there is no fingerprint scan or whole-job rollback. Native journal/summary counts actual closed,
skipped and deleted-incomplete outcomes,not decoded discarded members.
### 4. Validation & Error Matrix
schema1/nonextract/read-contract option or missing capability -> reject before output effects.
Unsafe/reparse/invalid name -> original typed refusal. Existing target -> target_exists+bytes,
then one Rust decision. Overwrite access/kind failure -> exact native code; no target observation retry.
AutoRename target_exists -> next bounded candidate; exhaustion -> OutputCollision. Cancel before close -> delete current incomplete owned output(s);
cancel after prior closes -> cancelled_with_committed_output/current completed journal preserved.
Sticky write failure -> reported at commit with first native code; later cancellation cannot replace it.
Successful Skip returns0closedfiles/bytes,files_skipped1/all_members_writtenfalse,
succeeded_with_warnings and members_skipped. Unknown expanded size remains None in prompt.
### 5. Good/Base/Bad Cases
Good: Ask receives target_exists bytes,ApplyAll remains current attempt,and chosen overwrite directly
truncates the same safe relative path. Base: absent target uses CREATE_NEW then metadata+close.
Bad: C# chooses ApplyAll,Rust dictates a Windows target signature,per-file observe+CAS returns,
or cancel hashes/deletes earlier closed output.
### 6. Tests Required
Original runner/Job/worker/native tests must cover all shared formats,Ask/Overwrite/Skip/AutoRename,
all ApplyAll choices,task/ticket isolation,target bytes,direct truncate,raced no-replace,wait cancel,
internal collision refusal,standalone Skip integrity and exact journal/source-delete qualification.
New regressions must assert final names/no `.nxpart` or rename,no observe_target/fingerprint/reopen/
durability flush,two JSON calls for uncontended files,<=32KiB records,zero write-ACK frames,sticky
native failure at close,commit metadata and incomplete-only cleanup while prior closed outputs remain.
Use exact LIST/RUN with no SKIP when native execution is authorized; existing Create/Edit,HZ5,cache,
readmission,path/source and operation-isolation regressions remain.
### 7. Wrong vs Correct
Wrong: `observe target -> verified partial -> flush -> rename -> reopen/hash` for ordinary extraction.
Correct: held safe parent -> final-name CREATE_NEW/direct decided overwrite -> bounded pipe writes ->
codec/source integrity -> metadata+drain+close -> delete only incomplete owned output on failure.

Phase D amendment: the two ZIP-only scenarios below retain their original acceptance scope.
Current product jobs additionally reuse these SAME owners for other known formats and selected
plans as specified by [product jobs](product-jobs.md). Shared solid/unknown packed ratios remain
unknown. General jobs delay `output_ready` directory effects until explicit held-destination
confirmation; legacy private ZIP callers keep the original behavior. ArchiveRead stays read-only.

## Scenario: ZIP high-expansion confirmation (B3 ZIP portion)
### 1. Scope / Trigger
ZIP extract member declared expanded size>64MiB AND expanded>1000×known positive packed size.
Strict integer/u128 comparison; unknown/zero packed size has no fabricated ratio. No threshold
configuration, permanent trust or other-format/UI/ArchiveRead extension.
### 2. Signatures
ZIP calls `ArchiveOutput::confirm_zip_entry(entry,limits)` before current-member decode/partial;
PrivateOutput uses original InteractionPort Risk ticket via `ZipRisk`. Private risk_prompt carries
ordinal/declared/packed/original entry+output limits; risk_reply carries ticket/approved_limit.
C# `RespondRisk(id,ticket,ulong?)` forwards only; null rejects. risk_confirmed records a fixed state.
### 3. Contracts
Path/internal-collision and hard declared-budget checks precede risk confirmation. Approval is
member/task/ticket scoped: approved_limit must equal ORIGINAL admitted entry limit and cover the
declared member. Smaller/larger, stale or duplicate answers do not approve. No request/Job limit
mutation, new attempt, automatic retry or expanded-budget support in this slice. Original actual
entry/total counters, codec dictionary/work limits, CRC/size/end/source/native checks remain live.
No current-member file begin before approval. Cancel/reject stops later members; original cleanup
deletes only incomplete owned final-name output(s); prior successfully closed counts remain,no rollback.
Original source/worker/Job/timeout remain owners. Correlated awaiting_risk/risk_confirmed/terminal
events log fixed state/count/elapsed only, never names/paths/payload/tickets as prose.
### 4. Validation & Error Matrix
Risk+no interaction adapter -> ResourceLimit, not implicit approval/unsupported format. Correct
finite answer -> decode under SAME limits; actual limit overflow -> ResourceLimit/no current
successful close. Corrupt/unsafe/collision categories unchanged. Cancel -> cancelled or real
cancelled_with_committed_output. Larger-budget approval is rejected,not silently granted.
### 5. Good/Base/Bad Cases
Good: legal high ratio confirms per member and verifies full data before close. Base:
ordinary ZIP no prompt. Bad: unknown packed treated as0,floating ratio,trusted declaration,
answer disables counters,or enlarged budget changes an already admitted Job.
### 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveRiskTests.cs` uses tiny .NET ZIPs and actual original
runner/Job/worker/native output: production-threshold declaration pending,confirmation,
reject/cancel,current-member zero begins,old targets/foreign files,prior closes,stale scope,
larger-answer refusal, actual member/total overflow and corruption/path/collision refusal.
`repo:rust/crates/nxtools-archive/src/zip_risk.rs` tests exact production boundaries and unknown/
zero packed values. DEBUG-only private fixture thresholds may ONLY lower positive warning
thresholds, exact test-apphost guarded in C#; Release serde rejects that field. No public bypass.
worker_wire and interaction_contract assert finite confirmation and unchanged read boundaries.
### 7. Wrong vs Correct
Wrong: Risk always means budget escalation or cross-task trust. Correct: this slice confirms
only the current ZIP member inside original immutable admission. Separate preflight readmission follows below.

## Scenario: ZIP declared-output preflight readmission (B3 portion)
### 1. Scope / Trigger
Only ZIP extract declared member/total output shortages. No actual-overflow retry, committed-output
upgrade, work/dictionary/metadata/Job escalation, UI/FIFO, other formats or ArchiveRead authority.
### 2. Signatures
`output_readmission::execute(WorkerRequest, &OperationControl, &InteractionPort, run) -> Result<AttemptResult>`
retains the original request. Callback `run(&WorkerRequest, &OperationControl)` uses original native
runner and returns ONLY after actual exited cleanup. Separate `Prompt::OutputBudget {current,required}`
accepts `PromptResponse::OutputBudget {approved}` or Cancel; Risk matching stays unchanged.
### 3. Contracts
Private schema2 extract opts in with `output_readmission {logical_id,expected_source?}`; negotiate
`zip_output_readmission_v1`. `output_budget_shortfall` carries current/required entry_bytes+output_bytes
and source binding. Original plan/path/local-record checks precede report. Native output uses existing
Prepare; only `output_ready` creates target root. Shortage has zero target directories/file begins/completed files.
First attempt must have matching ID/current budgets,ResourceLimit,zero successfully closed files and actual
cleanup_complete. Larger finite approval covers both current and required, is positive and within
existing32GiB/file and2^50total native bounds. Only two output fields change; fresh attempt control/ID
shares cancellation, logical_id links original. Preserve supported path/codepage/password/conflict/
MOTW/private options and host-owned destination; current worker selection remains all entries.
New callback re-enters original admission/runner/Job/source acquisition. Expected native volume/full
fileID/size/mtime and Rust SHA256 snapshot digest are verified before output_ready. Actual counters,
CRC/size/path/source and ordinary metadata+drain+close checks remain unchanged. At most one approved
new attempt; second failure is terminal,never another prompt. Correlated fixed lifecycle/elapsed diagnostics only.
### 4. Validation & Error Matrix
schema1/nonextract/read/delete option -> reject; nonZIP -> UnsupportedOperation. Over native hard
bound -> ResourceLimit without approval. Stale/duplicate/under-required/unlimited answer -> ignored.
Cancel -> no new attempt. Incomplete cleanup,wrong ID/current or completed-output shortage -> IoFailure,
no prompt/new attempt. Changed native identity/revision or digest -> SourceChanged,no target effects.
Actual overflow -> ResourceLimit,no current successful close or third attempt; prior closes remain.
### 5. Good/Base/Bad Cases
Good: tiny ZIP low quota -> cleaned shortage -> explicit finite approval -> new native operation.
Base: no shortage completes once. Bad: mutate admitted request/Job, larger Risk answer, auto-retry,
assume private complete/ActiveCount0 proves exited, or treat old approval as authority for changed source.
### 6. Tests Required
`repo:rust/crates/nxtools-archive/tests/output_readmission_chain.rs` drives real Rust coordinator,
original C#runner/Job, actual worker and native bytes with tiny independent ZIPs: member/total success,
cancel, stale/duplicate, source replacement, actual overflow, changed IDs/options/memory and actual
cleanup before prompt. `output_readmission_contract.rs` rejects unsafe attempt facts/invalid approvals;
`worker_wire.rs` preserves strict opt-in/read-only boundaries. C# `ArchiveReadmissionTests` provides
red-capable zero-target preflight. Build apphost first, matchingisolatedworker next; exact LIST/RUN,
no SKIP. No large/resource matrices required.
### 7. Wrong vs Correct
Wrong: codec raises limits inside an accepted Job. Correct: Rust retains original logical request,
waits for actual native cleanup, gets explicit output-only approval and dispatches one fresh admission.
