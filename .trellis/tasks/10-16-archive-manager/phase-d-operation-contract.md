# Phase D — End-to-end operations and desktop launch intents: implementation boundary

2026-10-21 authorized source/isolated implementation, not earlier-phase or live acceptance.
Read implement.md's dependency amendment and archive reference Phase D follow-up first.

## Gap and owners

Current product browser uses original read client and entry materialization. General verified
create/extract/test are private-worker capabilities without forms/FIFO/business projection.
ArchiveApplication will own immutable drafts, max2running/max32queued FIFO, memory and conflicting
source/output admission, presentation and once-only completion. C# translates typed execution into
original ArchiveWorkerRunner/Job/source/output; no second executor. Public ArchiveRead stays
read-only and its budgets/credit/compatibility unchanged. Startup intents are consumed in Rust,
not native commands bounced through C#. Source deletion remains unavailable/default-off.

## Interface agreement

Add protocol `archive_job.rs`, re-export through `pub mod archive_job`.
- `ArchiveJobCommand {version:u32, operation_id:Uuid, session_id:Uuid, generation:u64,
  action:ArchiveJobAction, request:Option<ArchiveJobRequest>, reply:Option<ArchiveJobReply>}`.
- Actions Start/Cancel/Pause/Resume/Reply, snake_case, version1, strict bounds and unknown-field
  rejection. Private creation password uses existing redacted/clearing PrivatePassword, never Debug.
- `ArchiveJobRequest {kind:ArchiveJobKind, paths:Vec<String>, destination:Option<String>,
  codepage:u32, limits:ArchiveJobLimits, extract:Option<ArchiveJobExtract>,
  create:Option<ArchiveJobCreate>}`. Kinds Extract/Create/Test. One source for extract/test;
 1..32creation roots (launch paths<=256 must never silently truncate). Destination is native-owned
 output base, required for writes; no helper/executable/arbitrary command field.
- Limits expose max_entries/max_metadata_bytes/max_working_bytes/max_entry_bytes/max_output_bytes;
  defaults original1M/256MiB/512MiB/32GiB/100GiB, native memory/entry ceilings still authoritative.
- Extract contains strategy Here/Name/Smart, single_file_folder, conflict Ask/Overwrite/Skip/
  AutoRename, copy_motw, selection optional original browser context+bounded ordinals. All is None;
  selected IDs must bind the source revision/digest before new worker ordinal remap.
- Create contains format Zip/SevenZ/Tar/Tgz/Txz/Gz/Xz, level optional Store/Fast/Normal/Maximum,
  method optional Store/Deflate, password optional PrivatePassword, solid/encrypt_header optional
  bools, strip_single_root bool, exclude_patterns Vec<String>, test_after_create bool,
  name String. Validation rejects inapplicable hidden fields; TAR no level/secret, streams single
  regular source, ZIP Store level/method agree. Mandatory precommit verification never disabled.
- Events `ArchiveJobEvent {version,operation_id,session_id,generation,kind,state?,progress?,prompt?,
  terminal?}`; Accepted/Started/State/Progress/Prompt/Terminal. Terminal exposes outcome/code,
  files_committed/bytes_committed/files_verified/bytes_verified/files_skipped, cleanup_complete,
  commit_state, elapsed_ms/native_error_code and bounded native journal facts when available.
  Actual exited cleanup, not receipt/private complete, owns terminal. Warning/link skips retained.
- Prompt is typed Password/Conflict/Risk/OutputBudget/Destination/CreateOverwrite as actually
  required, exact ticket+generation. UI response returns same ticket, original operation/context.
  Preserve original finite InteractionPort and output_readmission coordinator, no UI retry loop.
- PlatformCommand::ArchiveJob(command), PlatformEvent::ArchiveJob(event). C# strict symmetric
  contract/session. Deferred Desktop receives ordinary PlatformNotification::Event; source reads
  keep their original dedicated receiver. Cancel must remain admitted under ordinary queue pressure.
- Add `archive_picker.rs` DTO if necessary for owned files/folders/destination/save purposes,
  bounded paths and exact source HWND; existing PickArchivePath remains compatible.

## Startup interface

Protocol `desktop_archive_intent.rs`: typed explicit/bare-path request/receipt, max32pending,
max256paths, bounded replay, real peer identity. Intent actions Open/Extract/Compress/Test with
extract mode Smart/Here/Name/Dialog. Parse args_os before ordinary summon return, preserve argv
through protected redirect and all existing special modes. Cold archive startup main show/focus
false. Owner readiness and native readiness are distinct; bounded wait has typed failure.
Expose one deferred archive owner entry for intent handling; Quick Find ordinary file Open only
routes there using a completion receipt, retaining usage/accepted-close/failure owners.

## Layout and lifecycle invariants

Official locked Kit0.7.1 Form::vertical/Field for labels/validation and stable Input/Select entities;
create680/extract560 starting widths, bounded scroll body and nonshrinking Medium action footer.
Dialog public content/header/footer/action/close for per-job prompts; one Root/native GPUI window.
Button outline/default32 via Toolbar.content when applicable; no Toolbar.child appearance coercion.
Progress::new/value(0..100)/loading for actual known/unknown snapshot, reduced motion owned by Kit.
Existing CJK family/DefaultLight/Dark, no vendor/palette changes. Constructor→Kit open_window/
Root→native HWND show/hide and close-request hooks must be traced and tested. Closing drafts cancels
only draft work; progress close/background never cancels the independent job. Stop explicitly
cancels original worker; stale window/generation/snapshot/ticket cannot submit or revive a job.
All cross-entity callbacks, including immediate/cache-hit results, defer to the next application turn.

## Required red-capable checks and stop

Production form validation/defaults→request; FIFO/duplicate/queued cancellation/cleanup slots;
original prompt identity/cancel/stale; actual native commit truth/once terminal/notifications/taskbar;
authenticated cold/running intent and Quick Find routing. Use representative tiny fixtures and
exact C# LIST→sameRUN counts. Final feature check then matching isolated worker; one final Desktop
suite after source converges. All Cargo lane checks and memory environment mandatory.
Low-noise accepted/start/state/terminal/elapsed/native diagnostics; no paths/members/secrets/ticks.
No100k/large/performance/full-codec matrix. Record JOB01–07/IPC01–05/DIAG01 separately; JOB07/SAFE08
cannot fully pass while real original-object recycle remains unaccepted. GUI/Shell/DPI/UIA/user
acceptance NOT RUN, no launch workaround. No E/F/package/install/system mutation/commit/push.
