# Archive edit entry and conflict interaction

## 1. Scope / Trigger
Step4 of the archive edit sequence unifies qualified ZIP/7z/TAR/TGZ/TXZ editing in the original Browser/ModifyDraft/OperationForm/Job/worker owners. It is not a codec or native transaction rewrite, a new controller, or product/user acceptance. Read the format contracts first: [ZIP/volumes](zip-volumes.md), [7z](sevenz-edit.md), [TAR](tar-edit.md). Evidence: `repo:rust/apps/nxtools-desktop/src/archive/jobs.rs`, `repo:rust/apps/nxtools-desktop/src/archive/browser.rs`, `repo:rust/crates/nxtools-archive/src/worker_zip_edit.rs`, `repo:csharp/NxTools.PlatformHost/Protocol/ArchiveJobSession.cs`.

## 2. Signatures
- Original `modify_browser(origin:(Uuid,u64,Uuid), action, ordinals, (value,source_prefix), codepage, cx)` opens the original Modify form. Both this entry and `submit_form_admitted` call `ArchiveBrowser::edit_members_current(action,ordinals,value,source_prefix)` against the immutable current snapshot, not sorted row positions/current mutable selection.
- `worker_conflict::edit_display(incoming,existing)` fills the existing optional `ArchiveJobPromptDisplay { incoming_member, target }`. No new public protocol field. Each name is nonempty, <=32767 UTF-8 bytes and has no control characters; invalid presentation falls back to absent context, never native authorization.
- `ArchiveBrowser::prepare_edit_refresh(intent,next_generation)` retains one private `(generation,folder,selected_paths)` hint. `install` consumes it only for the matching new generation and remaps names to NEW RowIds. Retirement clears it.

## 3. Contracts
Qualified ZIP supports Add/Delete/RenameFile/explicit-directory Rename/NewDirectory/Comment, not ReplaceContent. Qualified7z/TAR/TGZ/TXZ supports Add/Delete/one-regular ReplaceContent/file+explicit/implicit subtree Rename/NewDirectory, not Comment. Missing worker facts/stale/loading/source failure/split/special structures refuse edits precisely; read/extract/preview remain independent.

Every Modify final submission binds archive path, original window generation/browser/snapshot/source and operation capability. Missing/warning original ordinals or wrong rename/replacement kind refuse before queue/IPC; worker still independently validates native source identity, revision, immutable ordinal and actual kind. Implicit subtree intent requires slash-ended original prefix plus a real member under that prefix. Frozen draft validation must not rebind to a later selected row.

Add names the local protected sources and internal target directory; current policy is Ask with per-prompt Overwrite/Skip/AutoRename/Cancel and current-job/attempt-only ApplyAll, not a new persisted policy preference. The explicit incoming-dialog choice “添加到当前压缩包” submits the original Add request directly; deliberate editing and Paste retain the original form. Drop freezes archive/window/session/generation/snapshot/source/read connection, internal folder and the complete source vector; recheck both the deferred choice and queued dispatch before native Start. Protected picker cancellation cannot submit/save or acquire source IO. ReplaceContent retains original name/metadata rules and uses a distinct typed action.

ZIP adapter resolves Ask before source reads/candidate generation; overwrite composes original Delete IDs and Add in ONE candidate, never two commits or delete-old-first. Reserve archive names AND all planned incoming names before AutoRename so a numbered choice cannot consume a later source. Directory-directory merges; file/directory conflicts refuse. All skipped closes sources and returns zero publication plus skip warning. Finite metadata/spool/output/cancellation bounds and explicit finish/flush/revalidation remain mandatory. ZIP record editor and original native publication are unchanged.

Conflict presentation labels existing archive member, incoming object and truthful known/unknown sizes; dates without facts stay unknown. Original ticket binds job/attempt/prompt/generation; stale/duplicate/wrong-attempt responses cannot act. Member/path context is private UI, Debug-redacted and never log/native authority. Original Root/Dialog/DescriptionList/Checkbox and outlined Medium32 decision buttons retain bounded scroll body/reachable footer/focus/password clearing. Browser uses `Toolbar.content` outlined Medium32 actions; `Toolbar.child` would erase those boundaries.

Successful warning/no-op is terminal, not a retryable draft. Only actual committed files with completed cleanup trigger origin-browser refresh; precommit failure retains input, unknown/published results block blind retry. Refresh transforms renamed/deleted paths and finds nearest surviving folder only after ready new snapshot; no old ordinal crosses snapshots. Other tabs/generations stay isolated. Original same-archive FIFO/cleanup-held slots, five real progress phases/clock and correlated accepted/start/state/terminal elapsed/native-code diagnostics remain; no payload/chunk/pointer logging.

## 4. Validation & Error Matrix
- Missing/stale origin, changed path/source/snapshot or invalid member/kind -> refuse before queue/IPC; retain draft for explicit fresh recovery.
- Missing qualification/unsupported operation -> original precise capability refusal, no suffix/read/create inference.
- Internal type/rename collision -> OutputCollision/refusal; no promised directory replacement or delete-before-rename.
- Skip-all -> succeeded_with_warnings, zero commit; Cancel -> no publication/original retained.
- Actual native5/64/late cancellation -> retain original commit/cleanup/journal truth; unknown cannot retry.
- Mismatched refresh generation -> consume/drop hint, never restore old authority.

## 5. Good/Base/Bad Cases
Good: TAR F2 -> original form -> one typed IPC, source stale at submit -> zero jobs; ZIP Ask AutoRename reserves future `same (2).txt`; refresh maps old71 by transformed name to new1. Base: existing7z/ZIP commands, prompts, queue and native output owners remain. Bad: current selection becomes frozen-draft identity; conflict target becomes a disk path; Skip-all enables retry; refresh assigns old ordinal to new snapshot.

## 6. Tests Required
`repo:rust/apps/nxtools-desktop/src/archive/application_tests.rs`: real rendered pointer/menu/F2 -> original form/IPC once, all-format stale refusal, final member/kind guard, drop cancel/frozen-target guard, actual read IPC ready-new-snapshot restoration. `browser_tests.rs` covers explicit/implicit selection and one-shot directory/generation hints; `forms_tests.rs`/`job_tests.rs` cover original protected picker/keyboard/focus/masked clearing/tickets/retry/queue/warnings/minimum layout. These synthetic GPUI tests are not physical external drag/DPI/IME/UIA acceptance.

`repo:rust/crates/nxtools-archive/tests/edit_interaction_worker_contract.rs` + independent CPython zipfile checks original Ask tickets, exact member bytes/names/source hash, one candidate/publication or zero for Skip/Cancel and future-name reservation. Original7z/TAR conflict/candidate/raw-header gates remain. `repo:csharp/NxTools.PlatformHost.Tests/EditInteractionTests.cs` requires explicit `NXTOOLS_ARCHIVE_WORKER_FIXTURE` and matching `NXTOOLS_EDIT_INTERACTION_WORKER_SHA256`; exact effects -> LIST -> identical RUN uses original Job/browser/native publish and independent .NET decode. TAR/7z/ZIP lease/control/preview compatibility must use that same current frozen worker. Skips are not passes; exact results/history/remaining native cancellation instability belong in the sole task checkpoint.

## 7. Wrong vs Correct
Wrong: enable all operations by format/read support -> submit stale ordinal -> separately commit Delete then Add -> replay on unknown. Correct: per-operation qualification -> frozen draft/current origin+member recheck -> original typed Ask/one complete verified candidate -> original held-reader-close/FILE-only one-shot publish -> actual terminal and generation-scoped presentation refresh.

## Scenario: EIC directory merge and native cancellation reply
### 1. Scope / Trigger
Step4 EIC-AC01/AC02 deterministic acceptance counterexamples, not new format/transaction policy.
### 2. Signatures
Original `worker_zip_edit::additions` plans against the original ArchiveSnapshot; original
`worker_output::PrivateOutput::call_payload` maps `NativeReply { success:false,error_code }`.
No public DTO, native lease, output authority or Job precedence signature changes.
### 3. Contracts
An incoming directory merges when the ORIGINAL ZIP has an explicit same directory OR any
slash-boundary descendant under that case-insensitive directory. Future incoming/reserved names
are not original-directory evidence. Do not materialize a redundant directory record; if all files
Skip and directories only merge, zero source-file opens/candidate begin/publication, one skip
warning and exact original bytes. Truly new empty directories/non-skipped files still publish once.
Map exact native error_code `cancelled` to `ArchiveError::Cancelled`, even without a cancel control
frame. Never infer cancellation just from a request; existing native publication/unknown/source-
changed/cleanup/timeout authority remains stronger. Preserve adjacent native error mappings.
### 4. Validation & Error Matrix
Implicit/explicit folder+allSkip -> complete/zero candidate/skip summary; new empty directory ->
one verified candidate; folder versus folder-other -> distinct; exact file/directory conflict ->
OutputCollision. Failed commit reply cancelled -> worker failure/Cancelled -> Job cancelled;
source_changed/target_exists/write_limit/native errors keep prior typed/native facts.
### 5. Good/Base/Bad Cases
Good: folder/same.txt alone establishes folder, adding folder/same.txt+Skip changes no archive.
Base: new-empty/ survives. Bad: add folder/ record after allSkip, or native cancelled -> IoFailure.
### 6. Tests Required
`repo:rust/crates/nxtools-archive/tests/eic_repair_worker_contract.rs` and independent fixture
`tests/fixtures/eic-edit-repair.py` drive actual ZIP/TAR/7z workers, readback/reader-close then one
reply-only native commit failure, no control frame; independent zipfile exact manifests/sourcehash.
`repo:csharp/NxTools.PlatformHost.Tests/EditInteractionTests.cs` EicAc01DirectoryMergeZeroPublication
and EicAc02NativeCancelledReplyOnly require matching frozen interaction worker/hash, explicit
Python/stock7Zip oracles, existing owned Job/native commit probes and exact effects/LIST/RUN.
Keep original actual control cancellation, native5/64/latecancel/sourcechange/unknown fixtures.
### 7. Wrong vs Correct
Wrong: exact directory record only; cancel request globally overrides errors. Correct: original
logical directory merge/no-op -> zero candidate; exact native cancellation reply preserves its
semantic through worker/Job while native publication and cleanup facts remain authoritative.

## Scenario: OPT-02 complete Add namespace
### 1. Scope / Trigger
All qualified ZIP/ordinary single-volume7z/TAR/TGZ/TXZ Add operations use the original
protected source, interaction, format adapter and native publication owners.
### 2. Signatures
`creation::IncomingPlan::build(source, limits, control)` retains `Vec<PlannedSource>` with
every protected ID/requested relative path/type. Ordinary `CreationPlan::build` and
ReplaceContent keep strict collision admission before directory traversal or payload reads.
`worker_conflict::add::resolve` returns `(Vec<ResolvedAddition>, skipped)`;
`ResolvedAddition { input: usize, name: String, replaces: Option<usize> }` binds the selected
input index to the original member index without granting filesystem mutation authority.
Evidence: `repo:rust/crates/nxtools-archive/src/creation.rs`,
`repo:rust/crates/nxtools-archive/src/worker_conflict/add.rs`.
### 3. Contracts
Reserve original explicit/implicit types, ALL requested incoming paths and generated paths
before AutoRename. Keep this reservation map separate from original/accepted occupancy.
Use original case keys, path policy, numbered-name helper and InteractionPort tickets.
Normalize admitted existing-member separators with `old.name.replace('\\', "/")` BEFORE
trimming the terminal directory slash for Add reservation/occupancy. Preserve original member
indices and encoded ZIP record spelling; this is comparison-space normalization, not input renaming.
An existing `folder\` directory is valid; embedded empty components/traversal still fail the
original `path_policy::validate_relative`. Windows Python `ZipInfo.orig_filename`, not its
normalized `filename`, is the raw-name evidence when comparing separator encodings.
Resolve all incoming conflicts before external payload opens; overwrite selects a source
index and retains the old replacement binding/metadata, never deletes an external source.
Logical directory merge keeps accepted parent spelling/old metadata. No selected addition
means zero candidate/publication, including merge-only without any skipped file. New empty
directories are changes. ApplyAll is local to this resolution attempt.
ZIP new files stay plain Deflate; checked size plus worst-case expansion reserves ZIP64
local headers before encoding. This does not change existing encoded-record preservation.
The existing ZIP password transition `password_state.state="modifying"` is accepted by
`ArchiveWorkerRunner.Password.cs::DiagnosticPassword` after successful preparation; it is
not an unknown frame. PasswordRequired prompts; ZIP/7z ambiguous authentication remains
terminal AuthenticationOrDataError, with no guessed WrongPassword or blind retry.
### 4. Validation & Error Matrix
- Same-type file collision -> original Ask/Overwrite/Skip/AutoRename/Cancel.
- File/directory or ancestor type mismatch -> OutputCollision, no payload/candidate.
- Source/stale identity or corrupt readback -> original typed failure, no publication.
- Merge-only/allSkip -> exact original, zero candidate; accepted empty directory -> one.
- Password cancel/authentication failure -> no candidate/publication, source retained.
- Native unknown/late cancel -> original journal and cleanup facts, never automatic retry.
### 5. Good/Base/Bad Cases
Good: old `same.txt` + requested `same.txt`,`same (2).txt` assigns generated `same (3).txt`
to the first input. Base: two roots named `folder` merge and resolve child conflicts.
Bad: a future reservation is treated as an existing directory and drops a new empty folder.
### 6. Tests Required
`repo:rust/crates/nxtools-archive/tests/add_namespace_worker_contract.rs` runs the actual
five-format worker, observes source-open/candidate/commit boundaries and independently
checks exact name/content binding. `creation_plan_contract.rs` retains ordinary Create's
early duplicate refusal; `worker_zip_edit::tests` checks ZIP64 local header reservation.
`repo:csharp/NxTools.PlatformHost.Tests/EditInteractionTests.Opt02.cs` uses exact effects/LIST/RUN,
hash-matched frozen worker, original browser/Job/source/publication and independent readers.
Check same-batch decisions, future/reversed/case names, merge/no-op/new-empty/type refusal,
ZIP/AES/header7z, wrong password/cancel, exclusive source release, journal and one publication.
Original source-change/native5/64/latecancel/read-close/metadata gates remain mandatory.
`repo:csharp/NxTools.PlatformHost.Tests/EditInteractionTests.Incoming.cs` additionally drives
the actual Desktop drop button into the original typed command/Job/native worker, root and
existing subdirectory, slash/backslash-ended directory records, Unicode/space file+folder+empty
directory, one measured publication, independent bytes and actual cleanup. Preserve cancellation,
Skip and original encoded names; successful compilation alone is not this boundary's acceptance.
### 7. Wrong vs Correct
Wrong: collapse Add into ordinary CreationPlan's unique-name map, or reserve only names
already visited. Correct: retain all IDs -> reserve complete namespace -> original decisions
-> one format-owned verified candidate -> original one-shot native publication.
