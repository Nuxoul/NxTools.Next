# Product archive jobs and finite output readmission

## 1. Scope / Trigger
Phase D typed create/extract/test dispatch over the original worker. Public ArchiveRead remains
read-only. This source contract does not certify live Desktop/Shell or full Phase A–F acceptance.
Evidence: `repo:rust/crates/nxtools-archive/src/product_job.rs`,
`repo:rust/crates/nxtools-archive/src/worker_job.rs`,
`repo:rust/crates/nxtools-archive/src/worker_output.rs`,
`repo:rust/apps/nxtools-desktop/src/archive/jobs.rs`,
`repo:csharp/NxTools.PlatformHost/Protocol/ArchiveJobSession.cs`.

## 2. Signatures
`product_job::execute_with_output_readmission(command, control, interaction, run)` adapts typed
jobs to original `output_readmission::execute`; run returns native terminal facts only after exited
cleanup. Logical ID, native attempt ID, transport receipt, session/generation and prompt ticket are
distinct. `PrivateOutput::preflight_output` preserves source binding and reports finite shortages;
`destination_preflight` carries optional known planned bytes to the original held-output owner.

## 3. Contracts
Desktop is admission/presentation, not codec execution: strict FIFO max2running/max32queued,
aggregate2GiB/per-worker1GiB, overlapping source/output serialization and cleanup-held slots.
All ordinary extraction formats enter the same finite coordinator; only fully known original planned
regular-file sizes can produce a shortfall. Preserve original selected ordinals, source digest/native
identity, conflict/MOTW/password and work/metadata limits. Retry is at most one fresh attempt and
changes only entry/output byte limits after matching zero-commit/actual-cleanup evidence. Unknown
stream overflow remains terminal ResourceLimit, never a guessed shortage. Compressed-TAR header
discovery uses the existing verified anonymous spool bounded by max(output,metadata); an overflow
before known inner sizes is terminal. Extraction uses the original target limits, not spool limits.
Ordinary known-destination extraction uses the [shared final-name output contract](../../../tasks/11-02-archive-workflow-optimization/ordinary-extraction-output.md): private `extraction_output_v1` inside bulk v1, held-relative `CREATE_NEW` or decided direct overwrite, <=32KiB original-pipe records without ACK, and metadata+drain+close completion. No ordinary partial/rename/fingerprint/reopen/durability flush. All formats retain codec/source integrity; cancel/failure deletes only incomplete owned output(s) and preserves successfully closed files/directories. Archive Create/Edit,virtual cache and HZ5 remain separate.
HZ5 [Stored ZIP reclamation](reclamation.md) is a separate destructive intent over the same original Job/worker/output owners. Start/Continue bypass ordinary finite readmission and batch retry; its consumed source is never restarted as ordinary extraction.

ZIP and meaningful independent-block7z/non-solid nonsplitRAR member ratios reuse ZipRisk and the
original InteractionPort: >64MiB AND >1000x known positive packed bytes. Shared solid/split/unknown
ratios stay unknown. Approval equals the existing admitted entry ceiling; it never raises budgets.
C# observes writability/free space from the held namespace/volume before directory effects. Missing
root/known low space requires a current ticket; unknown output remains unknown. No probe file,
elevation or automatic approval. Rust supplies layout/policy, C# native handles/API/results.
The [7z safety contract](sevenz-safety.md) specifies exact solid/unknown attribution and separate
opt-in encoded-source admission. A selected solid member may require extra block decode; actual
overflow is terminal even when the selected declared output fits, never another budget shortage.
The [RAR safety contract](rar-safety.md) records original UnRAR member/solid/split semantics and
RAR-only encoded admission. Password preparation TEST and Risk do not own output-budget retries.
The [TAR/stream contract](tar-stream-safety.md) separates encoded admission/spool/target output.
Private opt-in XZ/all-known Zstd declarations reuse existing scans; public read/TEST remain unknown.

Native publication facts survive cancellation/error and post-create test failure. Post-create Test
is a fresh original attempt under the same logical slot, with one final result/notification. Actual
cleanup owns retirement; disconnection is not zero-output proof. Pending state precedes dispatch;
old attempts/receipts/tickets cannot affect current work. Cross-entity callbacks defer, including
immediate results. Progress<=10Hz, bounded lossless-or-fail-stop prompt/terminal, existing taskbar/
NotificationCenter owners. Source deletion is visibly unavailable; strict recycling remains unaccepted.

## 4. Validation & Error Matrix
Invalid hidden format options/single-stream multiple roots -> reject before execution. Declared
shortage -> ResourceLimit+bound facts before target effects; native failure -> original code, not
shortage. Changed source/target -> typed refusal/no unsafe retry. Actual overflow -> terminal and
owned-incomplete-output cleanup. Completed-file/unclean/mismatched shortfall -> no new attempt. Stale reply ->
ignored. Cancel preserves native completed journal; cleanup unknown retains the running slot.

## 5. Good/Base/Bad Cases
Good: tiny known TAR shortage -> actual exited cleanup -> explicit finite approval -> one bound
native attempt. Base: ordinary ZIP/no shortage completes once. Bad: suffix proves capability,
solid block copied into every member ratio, UI retry loop, private complete releases a Job slot,
late cancellation zeroes commits, or source path deletion substitutes for strict recycling.

## 6. Tests Required
`repo:rust/crates/nxtools-archive/tests/worker_extraction_contract.rs` drives original workers for
non-ZIP shortage/source/risk/unknown overflow. `output_readmission_contract.rs` and
`job_execution_contract.rs` guard finite admission and projection. Production Desktop regression:
`repo:rust/apps/nxtools-desktop/src/archive/job_tests.rs`. Actual native boundaries:
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveJobTests.cs`,
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveCreateOverwriteTests.cs`,
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveDestinationPreflightTests.cs`.
Exact LIST->sameRUN with matching isolated worker is required; TestWindow/native fixtures are not
live GUI/user acceptance. Keep full-suite failures and earlier-phase obligations separately visible.

## 7. Wrong vs Correct
Wrong: publish form submission as success or enlarge an already admitted Job. Correct: immutable
validated draft -> original bounded FIFO/worker/native transaction -> actual exited cleanup ->
truthful once-only terminal; explicit finite output readmission is a separate fresh admission.

## Scenario: HZ2 presentation freshness, correlated controls and directory-only output
### 1. Scope / Trigger
No-event stalls, pause/resume admission failure and completed empty-directory extraction.
Evidence: `repo:rust/apps/nxtools-desktop/src/archive/progress.rs`,
`repo:rust/apps/nxtools-desktop/src/archive/progress_metrics.rs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs`.
### 2. Signatures
`RateSamples::expire(now_ms)` prunes original samples; `ProgressSnapshot::progress_elapsed_ms`
is the original accepted Progress event's monotonic observation, not UI refresh time.
`control_job` retains receipt/action/logical-job/attempt/generation/connection in the original
JobRecord; `job_control_result` observes acceptance, `finish_job_control` retires only that UI wait.
`ArchiveOutputSession.ResultLocation()` reads committed file OR completed held directory facts.
### 3. Contracts
Visible active progress alone owns a <=10Hz task using the existing executor's monotonic clock;
hide/occlusion/background/terminal/retirement stops it, no idle global timer or synthetic samples.
Elapsed advances from the last real snapshot. Five-second/64-sample/one-second-history limits stay.
UI-only refresh does not renew sample freshness. Controls register before original IPC dispatch:
not-submitted/refused/receipt-timeout/state-timeout clears the matching wait and permits explicit
retry, never automatic replay or native-slot release. Accepted is not Paused/Running; those require
real current attempt State events. Cancel remains possible while Pause is pending and retains the
original IPC reserved cancellation lane/returned deduplicated receipt. Native directory Completed
is marked only after successful EnsureDirectory plus held-handle verification; preparation roots,
wrappers and unfinished file ancestry alone do not count. Native location is not a success flag.
### 4. Validation & Error Matrix
Old receipt/attempt/generation cannot clear a current request. Disconnect/terminal retire UI waits,
not unconfirmed native cleanup. Failed/cancelled/cleanup-unknown output never auto-locates. Zero
file commits plus actual completed directories may locate; genuinely zero output remains absent.
### 5. Good/Base/Bad Cases
Good: 0B then1MB/1s, no events6.1s -> no obsolete speed/ETA; queue Busy -> visible retry.
Base: original ZIP/file result and actual pause State semantics remain. Bad: ACK means paused,
UI refresh resamples old bytes, timer fabricates progress or prepared base means successful output.
### 6. Tests Required
`repo:rust/apps/nxtools-desktop/src/archive/progress_feedback_tests.rs` exercises no-event clock,
visibility/terminal/background retirement and long/minimum footer. `job_tests.rs` drives actual
buttons/PlatformBridge pressure/refusal/timeout/old receipt/cancel and consumes real worker terminal
transcripts. `repo:csharp/NxTools.PlatformHost.Tests/ArchiveJobTests.cs` proves13 directory shapes,
six zero outputs and actual guarded terminals. Optional fixture-only export key
`NXTOOLS_ARCHIVE_DIRECTORY_TERMINALS` points to an existing explicit evidence directory; default
Desktop regression consumes the checked-in actual transcript, never silently skips without it.
### 7. Wrong vs Correct
Wrong: files_committed>0 is the only output fact, or send error is discarded while UI stays pending.
Correct: original native held completed outputs -> typed terminal -> existing success/cleanup policy;
original correlated control owner distinguishes enqueue/ACK/observed state and bounded failure recovery.

## Scenario: Per-operation modification foundation with separately qualified 7z writer
### 1. Scope / Trigger
Capability projection and future edit intent over the original record-preserving ZIP chain.
### 2. Signatures
`ArchiveOperation` distinguishes Browse/Extract/Preview/Add/Delete/ReplaceContent/RenameFile/
RenameDirectorySubtree plus original NewDirectory/Comment. `ArchiveCapabilities::admit(operation)`
returns typed `ArchiveModificationRefusal`; `ArchiveJobModifyAction::operation(value)` retains
the existing wire action grammar. `ArchiveEngine::inspect_modification_source` verifies the
original source revision before the asynchronous browser eligibility result is published.
### 3. Contracts
Capabilities derive from the opened actual source/backend, never suffix or create support. Reads
remain independent of write refusal. The later approved [7z edit slice](sevenz-edit.md) adds qualified
ordinary7z operations and a distinct replacement wire; TAR/RAR/volumes and ZIP replacement remain
unavailable. No C++ ABI, native authority, queue or commit manager is added.
ZIP immutable IDs/revision, full candidate validation, encrypted-record retention and C# source
handoff/atomic publication remain. Unknown metadata blocks retaining/rewriting those records but
must not remove the original ability to delete them; Delete still enters the original complete
candidate validation and can fail if unsupported kept records remain. Pending/unbound snapshot
never inherits this deletion exception. Browser command/menu/shortcut/drop and application/worker
admission share typed operation semantics and current source/snapshot checks.
### 4. Validation & Error Matrix
Missing backend/snapshot/source/volumes/application container/unsupported structure or encryption/
corrupt/unverified metadata has a typed reason only when established. Unknown detection is not an
invented SFX diagnosis. Unsupported actual source cannot fall through to ZIP mutation/output.
### 5. Good/Base/Bad Cases
Good: healthy7z browses/previews without global error; missing eligibility refuses only edits.
Base: ordinary ZIP keeps original actions. Bad: can-create implies can-edit or global read_only
flag replaces operation capabilities; all-record metadata rejection disables legal Delete.
### 6. Tests Required
`repo:rust/crates/nxtools-archive/tests/zip_edit_contract.rs` covers matrix/unknown/stale source,
delete-unsupported compatibility and original encrypted records. `worker_wire.rs` proves actual
unsupported source rejection/no output frames; browser tests verify original entry eligibility;
Protocol tests preserve typed rename distinctions with unchanged wire shape. Native ZIP handoff
fixtures remain mandatory. The qualified7z writer follows the linked contract; TAR writing, HZ3–HZ5
and real user/Explorer acceptance stay separate.
### 7. Wrong vs Correct
Wrong: enable Add for TAR because a TAR encoder exists. Correct: backend missing -> typed refusal;
future backend must connect to the original typed intent/candidate/verification/native commit seam.

## Scenario: Negotiated large-count ordinary extraction
### 1. Scope / Trigger
Ordinary product Extract with `archive_job_large_count_v1` in the current Desktop/CLI HelloAck.
Reference and native evidence: [large-count ZIP](../../../../docs/reference-code/archive-large-count.md).
### 2. Signatures
Private `WorkerRequest.job_large_count: Option<bool>` permits only true/schema2/product Extract/
JobExtract without reclamation, preview or source deletion; accepted requires `job_large_count_v1`.
`ArchiveOutputSession.ConfigureJournal(ArchiveAdmittedLimits)` retains bounded final-name result facts;
optional `ArchiveJobTerminal.journal_summary` contains total_records/detail_complete/outcomes.
### 3. Contracts
Rust admits the complete original plan under entries/catalog/work/output and native-history path
credit before output. Native cumulative records use admitted entry/metadata credit; active ordinary
final-name tokens/streams retain their separate4096ceiling and close as soon as their required
format integrity and metadata succeed. Journal records retain bounded result path,created count and
completion/cleanup state,not ordinary output hashes or file IDs. Only negotiated ordinary product
extraction may use no fixed work-duration deadline; cancellation,native errors,Job memory,cleanup
and retained-slot ownership remain active. Other paths are unchanged. The native history owns
complete `(state,completed,cleanup)` grouped file/byte totals. Public detail is at most256records,
nonordinary outcomes first,original ordinal retained; detail_complete is exact.
Rust validates group/ordinal uniqueness,checked totals,completed totals and detail membership.
No global frame increase, unrelated jobs or silent detail completeness. Legacy sessions omit the
summary and retain their old capacity. Desktop safe Retry checks full summary outcomes as well as
legacy details, never infers zero output from an incomplete detail page. Success logs are cumulative
milestones; failure/terminal diagnostics preserve original categories and native boundaries.
### 4. Validation & Error Matrix
No capability/old worker -> no large-count effects. Catalog/history/output shortage -> ResourceLimit
before final-name begin. Cancel/error after prior closes -> exact completed totals and cleanup,no global
rollback. Unknown close/delete or failed cleanup remains explicit in complete groups even when details
are partial.
### 5. Good/Base/Bad Cases
Good:70,000tiny ZIP members close in one Job with a bounded terminal. Base: legacy small jobs retain
wire and resource semantics. Bad: raise4096globally,expand1MiBframes or total only the detail page.
### 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveLargeCountTests.cs` drives actual supplied70k/native
bytes, cancel4,200, late CRC failure and legacy/metadata refusal. Explicit native-transcript gate:
`repo:rust/crates/nxtools-protocol/tests/archive_large_count_native.rs`. Protocol malformed summary,
IPC capability, original output/cleanup and Desktop retry regressions remain required. Capacity does
not certify all staged codec workloads or Release performance.
### 7. Wrong vs Correct
Wrong: cumulative completed transactions are simultaneous open files, or256details mean256outputs.
Correct: admitted cumulative native facts + independent active resources + complete bounded aggregates.
Evidence: `repo:rust/crates/nxtools-archive/src/worker_job.rs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.Journal.cs`,
`repo:rust/apps/nxtools-desktop/src/archive/batch_jobs.rs`.
