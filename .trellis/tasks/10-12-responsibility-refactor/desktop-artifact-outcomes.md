# Desktop artifact lifecycle extraction

## Scope and pre-change invariants

Authorized behavior-preserving responsibility refactor only: `downloads.rs` and new private `downloads/artifact_actions.rs`. Keep one `DownloadApplication` entity, its pending map, keyed windows and engine; no manager, trait, dispatcher, dependency or new policy. Settings, FFmpeg, overlay, general task commands and task-window rendering/lifecycle stay in their existing owners. Existing dirty feature changes are the baseline, not HEAD.

Baseline: 4197 lines; SHA-256 of UTF-8/LF text `a96f4ef6b222e460ba0174c906c98fc340126a453a2fe7eaa1666fee3907ac58`. Snapshot is in session scratch `downloads-artifact-before.rs` for function-preservation comparison.

Blocking reference review completed before changes:
- `docs/reference-code/download.md:107-163, 177-223, 1097-1140` records completion, terminal Shell actions, identity-scoped close and still-pressed OLE boundaries.
- ABDM first: `D:\Code\ab-download-manager-master\desktop\app\src\main\kotlin\com\abdownloadmanager\desktop\pages\singleDownloadPage\CompletedDownloadPage.kt:87-147` (open/chooser/reveal/OS drag/close).
- Legacy `D:\Code\NxTools\Views\DownloadCompletedWindow.axaml.cs:17-35, 51-98, 119-173`: constructors attach animation, Avalonia Window base, open/reveal intent closes, pressed-pointer drag enters native service first.
- Legacy `Views\DownloadProgressWindow.axaml.cs:26-59, 84-135`: constructor/subscription cleanup, in-place completion, center preservation and topmost/focus.
- Legacy `Views\MainWindow.axaml.cs:2145-2226, 2663-2666, 2844-2884`: keyed registry, Closed cleanup, passive completion, actual `window.Show()` boundary.
- Legacy `Services\WindowAnimationService.cs:13-48`: Closing cancellation runs exit then actual `window.Close()`; `Services\WindowsFileDragService.cs:29-61, 338-347`: OLE entry and escape/button-release termination.
- Legacy `ViewModels\MainWindowViewModel.cs:1583-1616, 2466-2500`: existing file/directory reveal/save fallback and final/part/working cleanup. Legacy creates a missing directory; current explicitly authorized behavior does NOT, and this refactor must NOT reintroduce it.
- Current `downloads/task_windows_impl.rs:147-168` remains the sole HWND+generation guarded success close; `downloads/task_window.rs:367` remains the direct pressed-pointer dispatcher.

Required preservation:
- Resolve existing final file/directory; reveal may use temporary path only before completion, then existing save-directory fallback; chooser accepts files only. Never create directories.
- Coalesce pending per task; caller correlation ID installed in pending before dispatch. Chooser timeout ten minutes, ordinary action fifteen seconds.
- Queue reject/timeout/disconnect/native failure retain retryable task window; exact per-action Chinese title/message and notification key/timing unchanged. Logs keep events, IDs, elapsed and native codes, never introduce paths.
- Successful result closes only originating current HWND+generation, never main-list or replacement window. Cancelled chooser with no error is quiet and retires pending; stale results cannot close anything.
- Delete-disabled setting remains list-only removal; confirmation reuse/placement/native close is unchanged, deferred close cleanup compares window identity, confirmation closes only after accepted remove. Engine owns final/temp deletion; general `remove` command stays untouched.
- Native drag stays synchronous from the still-pressed callback, carrying source HWND and final path. C# retains capture release/button check and dedicated STA OLE ownership; no second GPUI drag loop.
- Preserve notification updates in the same turn and same sequence relative to pending removal, logging, other disconnect cleanup and keyed window close. No new cross-entity update path.

Existing red-capable regression boundary (parent owns execution): `downloads/tests.rs:3465` `production_task_window_reuses_one_skeleton_and_generation_across_phases` includes completed reveal timeout, failure, success/reopen, pending-before-result and quiet chooser cancel; target resolution at line503 and delete confirmation at line3119. No tests will be moved or renamed.

## Verification handoff

No Cargo/rustfmt/native/service/user-data/package operations authorized for this subagent. Parent owns full baseline/post desktop fixture tests, strict check and source gates. After extraction, record exact function hashes and any source-location-only gate updates below.
