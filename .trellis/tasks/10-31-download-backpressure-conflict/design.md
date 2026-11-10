# Design

## Boundary
The defects live in DownloadEngine's primary event dispatch and final artifact publication, not in transport or rendering. Engine state/store remain authoritative; GPUI owns existing views and selection; C# remains the existing native window adapter.

## Event recovery
Keep the bounded 256-slot primary stream and 192-slot progress watermark. A pending-state-loss atomic records failed state try_send. On a later emission, attempt SnapshotRequired before the new event, using the existing sequencer and session generation. Do not spawn backlog tasks. Secondary subscribers retain their existing bounded disconnect-on-full behavior. Release tasks before the verified state/progress emits where local cloning suffices.

## Batch lifecycle
Install operation ID/action/deduplicated target IDs before dispatch. Run the synchronous engine command off GPUI on a real blocking executor, then defer result application to a later UI turn. Busy state spans actual execution and rejects duplicates. Reuse the delete-confirmation view/window lifecycle with quantity/action metadata. Completion publishes one notification; failed-only action changes the original MainWindow selection on a deferred turn, including every original target when the whole batch is rejected. Native close/cancel does not submit. List-only removal never confirms; the separate file-deleting toolbar/menu entry follows task_removal_deletes_file.
Before opening a batch confirmation, retire every existing single-item confirmation belonging to its target IDs, including nonfirst targets. Keep unrelated single-item confirmations alive. Cancellation and completion leave no target-bound single-item confirmation behind.

## Conflict lifecycle
Use one no-replace filesystem move at the final writer/Blob publication boundary. Windows MoveFileExW has zero replacement flags and narrowly scoped unsafe; async writers await spawn_blocking. Retain source on error. Resolve numbered names through existing organization policy, preserve task/checkpoint/temporary identity, and keep final_committed as the artifact ownership fact. Fully prepared HTTP/segmented/HLS/DASH output persists prepared_size_bytes in the existing temporary artifact state before final publication. Queue recovery publishes that output without remote access, retaining original completion/verification/cleanup. Incomplete/preallocated bytes require the original transfer path; file length alone never grants prepared status.
Numbered destination, cleared scheduled start, transfer state and queue admission commit in one save_task_with_transfer_and_admission transaction under the scheduler/task locks. Only a successful commit updates memory and broadcasts TaskChanged. Rejection retains the original destination, checkpoint, prepared marker and admission; subsequent queue refill failure retains the accepted intent.

## Presentation invariants
Existing keyed task windows and confirmation owners remain. No new banner or modal framework, native size inflation, activation/focus changes or synchronous cross-entity writes. Failed client remains 440x196; filename uses bounded truncation and actions remain visible. Official Kit0.8.0 Button loading/disabled and DialogContent/DialogFooter/NotificationCenter are reused. Batch ToolbarGroup.content retains direct official Small icon Buttons (24 DIP), avoiding Toolbar.child variant adaptation. Confirmation uses existing Small outlined Cancel/danger action. Conflict retry is official Small primary with Cancel outline, inside original DialogFooter.

## Out of scope
No overwrite command; no IPC queue redesign, unbounded channels, retry framework, vendor edits or unrelated cleanup.

## Resolved wire contract
DestinationExists is a distinct fault and RetryWithAutoRename is a dedicated task action. Download command protocol9 and IPC handshake2 explicitly reject older clients before commands. The stable pipe name and shared CLI/BrowserHost constants retain deterministic host discovery. C# DTOs and SQL schema are unchanged. The user-facing conflict cycle uses task-derived correlation identity across presentation, choice, command and result; the engine retains its original per-attempt diagnostics.

## Approved conflict action scope (2026-11-01)
The user approved automatic numbered retry and Cancel as the final U2 action set. This scope supersedes the proposal's three-action recommendation. Existing destination bytes remain untouched; the fixed failed-window budget and deferred original window lifecycle are retained.
