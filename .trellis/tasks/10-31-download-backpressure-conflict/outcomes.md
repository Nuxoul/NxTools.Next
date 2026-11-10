# Outcomes

## Result
H1/M2 and U1/U2 are implemented. Final scoped verification passes 505 tests and strict Clippy. The user approved numbered retry and Cancel as the final U2 action set on 2026-11-01. One unrelated date-dependent calendar test remains a known failure and is explicitly excluded from the final scoped rerun; existing Archive formatting differences remain.

## H1: Nonblocking bounded events
The primary 256-slot event stream previously used blocking state sends while GPUI owned its only consumer. A synchronous batch could fill that stream and wait for the same UI turn that had to drain it. Verified worker paths also emitted while holding the task mutex.

`rust/crates/nxtools-download/src/engine.rs::DownloadEngineInner::emit` now uses try_send for every primary event. A lost-state atomic requests a generation/sequence-aware SnapshotRequired before the next ordinary emission with capacity. Progress keeps its 192-slot watermark; secondary subscribers keep their disconnect-on-full behavior. Verified pause, progress, terminal, completion and failure emissions release the task mutex first, including the segmented runner. The existing desktop sequence-gap/SnapshotRequired path still refreshes the authoritative snapshot.

## U1: Asynchronous batch lifecycle
`engine/projection.rs::request_batch_background` runs the original correlated Batch command on the existing runtime blocking executor and returns one bounded result. `downloads/batch.rs` installs operation ID, action and deduplicated target IDs before dispatch; one operation is admitted at a time. Matching completion is deferred to another UI turn, refreshes the list and defers one NotificationCenter summary. Partial and whole-batch failures retain the exact failed IDs for the original MainWindow selection action.

`main_window/download_page.rs` uses official Button loading/disabled and PopupMenuItem disabled states. Separate toolbar/menu entries preserve list-only removal and expose RemoveWithArtifact when task_removal_deletes_file is enabled. Cancel and file-deleting batches of at least 20 deduplicated targets reuse DeleteDownloadView and its existing keyed confirmation-window lifecycle. The title names the quantity. Cancel, caption close and close-event simulation do not dispatch and release busy; list-only removal never confirms.
Opening a destructive batch confirmation closes every target task's existing single-item confirmation, including nonfirst targets. Unrelated single-item confirmations remain live. Actual Cancel and Confirm actions leave no target-bound stale window; cancellation preserves every task and confirmation removes only the selected targets.

Batch diagnostics share the original correlation ID across UI acceptance and engine start/aggregate completion, with target count, failure count and elapsed time. No per-target batch log or request payload is added.

## M2: No-replace publication and retained-output recovery
A one-time existence check followed by replacing rename could overwrite a destination created during transfer. Failed tasks also released names despite remaining retryable.

`http.rs::{move_final_no_replace,commit_prepared}` now uses Windows MoveFileExW with flags 0 inside spawn_blocking. The single unsafe function has a SAFETY explanation and local lint allowance. Existing-target errors become HttpDownloadError::DestinationExists; the destination and temporary source remain unchanged. `engine/browser.rs::import_blob_artifact` shares this boundary and retains create_new copying for its fallback.

`engine.rs::{target_reserved_by_tasks,retry_with_auto_rename}` reserves every uncommitted retryable destination, including Failed/Cancelled, and resolves numbered names through the existing organization policy. RetryWithAutoRename preserves task, checkpoint and temporary-file identity while updating the final name/path.
Numbered name, final path, cleared scheduled start, transfer state and queue admission use one existing SQLite transaction. Only successful commit updates memory and broadcasts TaskChanged. A rejected admission preserves the original name/path, prepared data, checkpoint and paused/eligible flags; repeated rejection does not consume numbered names. After admission commits, a queue-refill failure retains the accepted intent for later refill.

`storage.rs` persists prepared_size_bytes and the exact temporary path using the existing download_artifacts temporary row's prepared state. HTTP, segmented, HLS and DASH finalizers record readiness only after preparing and synchronizing the final output. The runner restores prepared output before probing and publishes it without remote access through the existing no-replace completion/verification/cleanup boundary. Incomplete/preallocated data never becomes prepared merely because its length matches the expected size; ordinary resume retains the checkpoint-to-temporary-file binding. No SQL schema migration is required.

## U2: Destination-conflict presentation
`downloads/{conflict.rs,task_window.rs,task_windows_impl.rs}` recognize the distinct DestinationExists fault. The existing failed task window explicitly shows the same-name reason and bounded filename, with full filename semantics. Its official primary action is automatic numbered retry; Cancel remains outlined. Ordinary nonconflict failure actions remain. The original 440x196 failed client, keyed generation and native placement/activation/close owners are unchanged.
The user approved this two-action design on 2026-11-01; the approved action set is recorded in the task design and UX proposal.

Detected, choice and choice-result diagnostics use task-derived correlation identity, and the task command carries it. An accepted retry is queue admission, not a claim that publication has finished. Logs exclude request URL, headers, cookies, filename and path payloads.

## Reference conclusions
Inspection order was AB Download Manager, then legacy NxTools. Entry files and lifecycle evidence are recorded in docs/reference-code/download.md, section Batch feedback and final destination conflicts.

- ABDM: shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/home/AbstractDownloadActions.kt and BaseHomeComponent.kt; desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/HomePage.kt; downloader/core/src/commonMain/kotlin/ir/amirab/downloader/DownloadManager.kt; desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/single/AddDownloadPage.kt; downloader/core/src/commonMain/kotlin/ir/amirab/downloader/destination/SimpleDownloadDestination.kt.
- Legacy NxTools: ViewModels/MainWindowViewModel.cs, Services/DownloadManagerService.cs, Views/DeleteConfirmationWindow.axaml.cs, Views/DownloadProgressWindow.axaml.cs and Views/MainWindow.axaml.cs, through ShowIndependentSecondaryWindow and Window.Show. WindowAnimationService, WindowChromeService and WindowActivationService establish application-level close/activation ownership. External Avalonia native source was not inspected.
- Both references support numbered admission and explicit overwrite, but neither supplies final-publication numbered recovery or accurate batch partial-result feedback. This flow offers numbered retry and Cancel and never replaces an existing destination. Existing confirmation and window ownership are retained.

## Compatibility
Download command protocol 8 -> 9 adds DestinationExists and RetryWithAutoRename. IPC handshake 1 -> 2 explicitly rejects a prior-version Hello before commands. The stable pipe name preserves discovery and an explicit incompatibility response. CLI and BrowserHost consume shared version constants and generic fault handling; no client-specific match changes are required. Both clients compile with tests. PlatformHost DTOs and SQL schema remain unchanged.

## Behavior-red / green evidence
Each command used the single-job, nonincremental, codegen-units=256, offline environment. Logs are in the session scratch directory.

| Boundary | Command filter | Behavior-red | Green evidence |
| --- | --- | --- | --- |
| H1 | cargo test -p nxtools-download --lib batch_remove_without_event_consumer_returns_and_requests_snapshot -- --test-threads=1 | 0 passed / 1 failed, deadline | Included in the final 127-test library suite; h1-red.log, h1-green.log |
| U1 | cargo test -p nxtools-desktop --bin nxtools-desktop batch_feedback_real_store_wait_keeps_next_ui_turn_responsive -- --test-threads=1 | 0 passed / 1 failed, next UI turn blocked on real SQLite | Passed in final desktop/download-page filters; u1-red.log, followup-4-downloads.log, followup-5-download-page.log |
| M2 | Three final-commit, Blob-conflict and failed-name commands below | Each 0 passed / 1 failed | 12 focused checks passed; covered by final library/IPC suites; download-m2-red.log, download-m2-green.log, download-m2-extra-green.log |
| Prepared recovery | cargo test -p nxtools-download --lib completed_transfer_conflict_auto_renames_without_refetching_offline_source -- --test-threads=1 | 0 passed / 1 failed, Failed instead of Completed | 1 passed; prepared/incomplete/real ranged-checkpoint and HLS/DASH contracts passed; download-prepared-red.log, download-prepared-green.log, download-checkpoint-green.log |
| U2 | cargo test -p nxtools-desktop --bin nxtools-desktop destination_conflict_task_window_keeps_named_actions_inside_failed_budget -- --test-threads=1 | 0 passed / 1 failed, numbered retry absent | destination_conflict_ 3/3, including real retry/cancel clicks; u2-red.log, verified-conflict-focused.log |
| Atomic numbered admission | cargo test -p nxtools-download --lib auto_rename_retry_admission_failure_rolls_back_name_transfer_and_events -- --test-threads=1 | 0 passed / 1 failed, rejected command changed name/path | 1 passed, ordinary and scheduled tasks, repeat rejection, exact durable state and event assertions; followup-atomic-red.log, followup-atomic-green.log |
| Batch confirmation ownership | batch_feedback_retires_every_target_confirmation_but_keeps_unrelated_window in the desktop command below | 0 passed / 1 failed, 3 windows instead of 2 | 1 passed, actual Cancel/Confirm, unrelated window retained; followup-confirmation-red.log, followup-confirmation-green.log |

M2 behavior-red commands:

```powershell
cargo test -p nxtools-download --lib final_commit_never_replaces_an_existing_destination -- --test-threads=1
cargo test -p nxtools-download --lib blob_publication_conflict_keeps_destination_and_staged_bytes -- --test-threads=1
cargo test -p nxtools-download --lib failed_uncommitted_task_reserves_name_for_numbered_and_reject_submissions -- --test-threads=1
```

## Final verification
Final Cargo verification sets CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256 and CARGO_NET_OFFLINE=true in the same process. Commands run sequentially without timeout. Desktop tests use the command-line single-package setting `--config 'profile.test.package.nxtools-desktop.debug=0'`; dependency profiles, product configuration and features are unchanged. Final logs use the followup- prefix in session scratch.

| Command | Final result |
| --- | --- |
| cargo test -p nxtools-download --lib | 127 passed, 0 failed |
| cargo test -p nxtools-download --test engine_contract -- --test-threads=1 | 104 passed, 0 failed |
| cargo test -p nxtools-download-ipc | 7 passed, 0 failed; 0 doc-tests |
| cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop --bin nxtools-desktop downloads:: -- --test-threads=1 --skip polished_calendar_eligible_keyup_and_disabled_past_preserve_draft_and_bounds | 227 passed, 0 failed; 1 known calendar case excluded |
| cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop --bin nxtools-desktop main_window::download_page -- --test-threads=1 | 32 passed, 0 failed |
| cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop --bin nxtools-desktop production_download_batch_toolbar_appears_for_multi_select_and_dispatches_one_batch -- --test-threads=1 | 1 passed, 0 failed |
| cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop --bin nxtools-desktop main_window::command_naming_tests -- --test-threads=1 | 7 passed, 0 failed |
| cargo clippy -p nxtools-download -p nxtools-desktop --tests -- -D warnings | Passed |
| cargo check -p nxtools-cli -p nxtools-browser-host --tests | Previous pass; closure changes no client/wire contract |
| cargo fmt --all -- --check | Previous full check has existing Archive differences; final scoped rustfmt check passes |
| git diff --check | Passed |
| python tests/architecture/spec-contracts.py | Passed: 120 documents / 14 owners |
| python tests/architecture/gpui-release-contracts.py | Passed |
| python tests/architecture/markdown-editor-contracts.py | Passed |
| tests/architecture/download-ui-lifecycle.ps1 | Passed |

The seven nonoverlapping final test commands execute 505 passing tests, 0 failures. The single known calendar failure is excluded rather than counted as passed. Focused red/green repetitions are not added to that count. Desktop cases cover real 300-task SQLite contention/next-window-turn responsiveness, duplicate suppression, exact feedback/selection, 19/20 confirmation, cancel/close cleanup, first/nonfirst target-confirmation retirement, unrelated-window preservation, production file-deleting toolbar/menu and Light/Dark 440x196 action bounds.

## Limitations and known issues
- downloads::polish_tests::polished_calendar_eligible_keyup_and_disabled_past_preserve_draft_and_bounds fails at polish_tests.rs:301 (past hit bounds). On 2026-11-01, the Sunday-start grid begins on today and contains no previous-day cell; the existing fixture assumes one. That calendar code is outside this slice.
- Workspace formatting differences remain in nxtools-archive/src/{output_readmission.rs,product_job.rs,worker.rs,worker_output.rs} and nxtools-archive/tests/{metadata_wire_contract.rs,output_readmission_chain.rs,rar_readmission_chain.rs,sevenz_readmission_chain.rs,tar_stream_readmission_chain.rs,worker_wire.rs}. They are not changed by this task.
- Filesystem publication and SQLite Completed persistence remain separate operations. If the move succeeds but the database commit fails, the prepared temporary path may no longer exist. Recovery does not claim the final file as owned in that state.
- Compiled/TestApp checks are not screenshots, physical keyboard/pointer, real HWND/DPI, installation or user visual acceptance. No screenshots or native/product acceptance were performed.
- No package, installation, product launch, external-network/user-data access, commit or push was performed. Pre-existing dirty work was retained; changes are limited to this download slice and its records.
