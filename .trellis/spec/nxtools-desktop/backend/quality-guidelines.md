# Desktop quality and lifecycle scenarios

Evidence: `repo:rust/apps/nxtools-desktop/src/main_window/tests.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/tests.rs`, `repo:rust/apps/nxtools-desktop/src/quick_launch/tests.rs`, `repo:rust/apps/nxtools-desktop/src/ui/mod.rs#tests`.

Use production GPUI actions/bounds and worker/native result seams. Source gates only prove structure. [Approved UI/current matrix](../../shared/ui-design.md) supersedes historical numeric intent, not behavioral invariants. No cross-owner re-entry, fake timeout success or double native resize. [Verification](../../shared/verification.md); visual/DPI/accessibility is separate. Preserve every scenario below.

## Scenario: Generation-scoped clipboard window

### 1. Scope / Trigger
- Trigger: changes to `ClipboardPanel` query dispatch, scroll prefetch, selection, or worker reply handling.
- Purpose: prevent stale query replies and unbounded payload retention across GPUI turns.

### 2. Signatures
- `request_entries(query, generation, offset, request, known_total, cursor, cx)` owns replace/append/prepend transitions.
- `ClipboardPageRequest` is `Replace`, `Append`, or `Prepend`.
- `ClipboardCaptureWorkerHandle::query_category_page_after_async` is used for forward pages.

### 3. Contracts
- `QuerySuperseded` is an expected cancellation result: clear pending state without clearing committed rows or showing an error.
- Scroll prefetch is disabled while `search_pending` is true; a replacement query owns the window until it commits.
- The loaded window is capped at four 80-row pages. `ListState::splice` is the only logical-index adjustment for front/tail eviction.
- Total count, window start offset, next cursor, and selected visible index are separate state; no cross-entity synchronous update may be introduced in render/callback paths.

### 4. Validation & Error Matrix
- Generation/query/category/scope mismatch -> discard without changing rows, selection, loading, or error state.
- Append/prepend offset no longer matches the current window -> discard as stale.
- Worker/database failure -> preserve committed rows for append/prepend; replace shows a typed query error.
- Native callback failure -> record bounded diagnostics and keep the platform event thread alive where possible.

### 5. Good/Base/Bad Cases
- Good: search replacement blocks scroll paging, then commits a fresh first page and resets the window.
- Base: moving forward evicts the front; moving backward prepends and evicts the tail.
- Bad: append a new-query page using the previous cursor, or treat `QuerySuperseded` as an empty/error result.

### 6. Tests Required
- Worker test proves queued obsolete queries are skipped before SQLite.
- UI test covers empty-selection keyboard behavior, source contract for bounded append/prepend, and generation/category/scope rejection.
- Desktop full suite plus packaged scroll/search/delete soak must measure retained rows, queue depth, and private memory.

### 7. Wrong vs Correct
#### Wrong
```rust
if panel.has_more_entries && !panel.page_request_pending {
    panel.load_next_page(cx);
}
```
#### Correct
```rust
if panel.has_more_entries && !panel.page_request_pending && !panel.search_pending {
    panel.load_next_page(cx);
}
```

## Scenario: Foreground-independent Directory Opus source recognition

### 1. Scope / Trigger

- Trigger: changes to PlatformHost Directory Opus/third-party source recognition, file-dialog foreground handling, or Auto Quick Switch dirty-state consumption.
- Purpose: prevent a delayed source capture from being replaced by the file dialog's foreground HWND and prevent a stale source from causing duplicate navigation.

### 2. Signatures

- `FileDialogRecognitionRetry.TryBeginAttempt(foregroundWindowHandle, out windowHandle)` remains foreground-gated for file-dialog recognition.
- `FileDialogRecognitionRetry.TryBeginPendingAttempt(out windowHandle)` is reserved for passive retries against a previously saved source HWND.
- `WindowsPlatformAdapter.EventWindow.RetryDirectorySourceRecognition()` captures the saved HWND, not the current foreground HWND.

### 3. Contracts

- When a third-party source capture is delayed, clear the previous Auto Quick Switch dirty source before scheduling the new bounded attempt.
- A passive source retry may read the saved lister while the file dialog is foreground, but must not activate, focus, navigate, send `WM_COPYDATA`, or send Rust IPC during recognition.
- On source success, publish one source and immediately re-consume the currently known file dialog if it is still foreground.
- On destroy, supersession, timer failure, or budget exhaustion, clear the pending source and never publish the retired HWND.
- Recognition diagnostics may include HWND, attempt count, elapsed time, terminal reason, and native error code; never include paths, titles, or payload data.
- Directory Opus friendly paths are localized display strings, not parsing paths. Alias construction must include `SHGetFileInfo(SHGFI_DISPLAYNAME)` names and a fallback for the parent of `Environment.SpecialFolder.UserProfile` so localized `C:\Users` segments resolve before any `Directory.Exists` or navigation check.
- Directory Opus owns activation around its `WM_COPYDATA` `Go` command. After a successful navigation, the session registry must not re-activate the captured pane/focus state, because that stale focus can restore the pre-navigation tab or directory.
- `lib://<KnownFolder>` targets are valid directory-navigation inputs. Rust action eligibility must preserve the URI, and Windows must resolve known library names before the filesystem existence gate; unknown library names remain a bounded failure.
- `navigate_directory` and `query_shell_actions` diagnostics may log target kind, root, library name, length, segment count, trailing-separator state, and resolved navigation metadata, but never the raw target path.

### 4. Validation & Error Matrix

- Current foreground differs from pending source HWND -> probe the pending HWND through `TryBeginPendingAttempt`; do not replace the target.
- Pending source HWND is destroyed -> terminate with `target_destroyed` and clear dirty state.
- Capture remains unavailable until the retry budget expires -> terminate with `attempt_budget_exhausted` and clear dirty state.
- File dialog returns before source success -> source success must yield one candidate for that same dialog, not zero or two candidates.
- HWND `0` -> never report a pending retry.

### 5. Good / Base / Bad Cases

- Good: DOpus capture settles after the file dialog returns; the saved lister is read passively and the dialog navigates once.
- Base: DOpus capture succeeds while DOpus remains foreground; the existing source-to-dialog path is unchanged.
- Bad: return to the dialog consumes an old source, then the delayed DOpus source is consumed again.

### 6. Tests Required

- Regression asserts a known dialog plus delayed source capture produces exactly one candidate after the pending HWND is probed in the background.
- Retry test asserts supersession, budget exhaustion, and `HasPendingAttempt(0) == false`.
- Architecture gate asserts the EventWindow caller uses `TryBeginPendingAttempt`, saved-HWND capture, stale-source clearing, dialog re-consumption, and destroy termination.
- Full PlatformHost suite and interactive DOpus/file-dialog soak are required when the environment provides DOpus.
- The real DOpus boundary test covers a valid localized `C:\` source, one `C:\Windows` navigation, readback, and restoration; a raw `WM_COPYDATA` success alone is insufficient evidence.
- Library URI navigation and nested filesystem target tests must assert the original target is preserved until the Windows resolver, and diagnostic metadata distinguishes `D:\Qingfeng` from `D:\`.

### 7. Wrong vs Correct

#### Wrong

```csharp
var foreground = _foregroundContextProvider.Capture();
_directorySourceRecognitionRetry.TryBeginAttempt(
    foreground.WindowHandle,
    out var windowHandle);
var attempt = DirectoryNavigationAdapter.TryCaptureLocalAttempt(foreground);
```

#### Correct

```csharp
var foreground = _foregroundContextProvider.Capture();
_directorySourceRecognitionRetry.TryBeginPendingAttempt(out var windowHandle);
var target = _foregroundContextProvider.Capture(new IntPtr(windowHandle));
var attempt = DirectoryNavigationAdapter.TryCaptureLocalAttempt(target);
```
## Scenario: Clipboard action terminal state

### 1. Scope / Trigger
- Trigger: changes to `ClipboardPanel::activate_entry`, PlatformHost command dispatch/result routing, or popup reconnect/disconnect cleanup.
- Purpose: prevent a paste/copy action from remaining busy when a result races panel setup, arrives late, or is invalidated by disconnect.

### 2. Signatures
- `ClipboardPanel::activate_entry(entry, action, window, cx)` creates one operation ID and installs `(command_id, action, item_id)` plus its start time before spawning format preparation.
- `ClipboardApplication::send_clipboard_action_with_native_formats(..., command_id)` uses `PlatformBridge::send_command_with_id` with that exact ID.
- `ClipboardPanel::handle_command_result(command_id, result, window, cx)` only consumes the matching command ID.

### 3. Contracts
- The action operation ID is also the IPC envelope ID; the panel is pending before any command can be observed by PlatformHost.
- After async native-format preparation, re-check the operation token before dispatch. Timeout, reconnect or disconnect invalidates the preparation; stale work must not enqueue a command or restore pending state.
- Every terminal path clears both `pending_action` and `pending_action_started_at`. Do not combine `.take()` calls with short-circuit `||`; evaluate both fields independently.
- Only a matching successful paste closes the popup; copy-only success keeps it open; a failed native result leaves it open and retryable.
- Diagnostics carry session/operation/command IDs, action, terminal reason and elapsed time, but not clipboard text, paths, URLs or raw native error messages.

### 4. Validation & Error Matrix
- Matching success -> mark usage asynchronously; paste closes, copy-only shows confirmation and remains open.
- Matching native failure -> clear both pending fields, show retryable error, keep popup open; usage order is unchanged.
- Preparation/queue failure -> clear both pending fields and report a payload-free failure event.
- Timeout/reconnect/disconnect -> clear both pending fields; late preparation/result cannot send, close, or reorder a newer action.
- Unmatched result -> ignore; it cannot mutate the current action.

### 5. Good/Base/Bad Cases
- Good: panel stores the ID before native-format worker work; the IPC envelope and result use that same ID.
- Base: a fast matching result arrives immediately after enqueue and is consumed by the already-pending panel.
- Bad: install `pending_action` only after async send returns, or clear it on reconnect but leave the operation watchdog token behind.

### 6. Tests Required
- GPUI `clipboard_panel_registers_action_before_platform_command_and_clears_it_on_result` asserts pending ID exists before the envelope is received, exact envelope ID, failed result cleanup, disconnect cleanup and late-result suppression.
- The same test verifies stale native-format preparation after disconnect enqueues no command.
- IPC caller-ID tests verify result routing and duplicate-ID rejection; desktop full suite and clipboard architecture gates remain required.

### 7. Wrong vs Correct
#### Wrong
```rust
let command_id = prepare_formats_and_send().await?;
panel.pending_action = Some(command_id);
```

#### Correct
```rust
let command_id = Uuid::new_v4();
panel.pending_action = Some(command_id);
prepare_formats().await?;
ensure_operation_is_current(command_id)?;
bridge.send_command_with_id(command, command_id)?;
```

## Scenario: Probe confirmation through custom-client native resize

- Rust owns logical target layout; after HWND creation send one native centered
  resize request. GPUI `window.resize` is another asynchronous SetWindowPos,
  not a logical-only viewport update, and must not compete with PlatformHost.
- Prefixed P/Invoke declarations specify the actual Win32 `EntryPoint`. A
  custom-client mismatch regression must execute GetWindowRect/GetClientRect,
  ClientToScreen and frame-changed SetWindowPos; compilation proves no exports.
- Perform visible-center correction before final client correction/readback.
  Recovery may retire only when the final client matches the requested layout
  after all operations, outside a move/size loop, within the bounded budget.
- Confirmation metadata may be staged while the viewport is compact. Render
  and submit gates use the actual WM_SIZE-derived viewport; a timeout never
  releases a clipped form. It yields an explicit retryable presentation error.
- Regression evidence includes a real custom-chrome HWND (WM_NCCALCSIZE),
  compact restoration on a position boundary, final client dimensions/center,
  pending retirement, and a production GPUI form/inspector/footer gate. A system
  dialog or source-string check alone is insufficient.
- Carry correlation across native resize diagnostics; success reason is
  client_size_settled with final dimensions. Real pointer/probe/DPI smoke is
  recorded separately and never inferred from compilation or screenshots.

### 1. Scope / Trigger
Probe confirmation through real custom-client HWND; preserved invariants above apply.
### 2. Signatures
SetWindowBoundsCentered client device-pixel width/height; prefixed native ClientResize APIs use actual Win32 EntryPoint.
### 3. Contracts
One native resize owner; center correction precedes final client readback; actual WM_SIZE viewport authorizes form/submission.
### 4. Validation & Error Matrix
Final mismatch -> pending bounded recovery; timeout -> retryable error, no clipped confirmation release; stale request -> no apply/retire.
### 5. Good/Base/Bad Cases
Good: moved center and exact client settle. Base: staged metadata waits for viewport. Bad: GPUI competing native resize or timer-only success.
### 6. Tests Required
Real custom chrome HWND mismatch/readback/retire and production GPUI form/inspector/footer; live pointer/DPI separately pending.
### 7. Wrong vs Correct
Wrong: SetWindowPos BOOL or timeout equals layout-ready. Correct: final native client and WM_SIZE-derived viewport gate presentation.

### 2026-10-09 native/viewport recurrence gate
- Final readback must occur at pending retirement after every applied correction/commit interleaving. Carry target/readback dimensions in the immutable replay result; terminal publication cannot report success when current client changed.
- Never PostMessage a requested-size WM_SIZE: GPUI0.2.2 trusts its lParam, and a queued278 message can arrive after real client returned176. Refresh the live client on its HWND owner via fixed native workers; retain pointer-bearing parameters until processing finishes, not only until a send timeout. Same-size refresh must actually produce current WM_SIZE, not merely no stale messages. A hung HWND retains its fixed worker; logical timeout does not interrupt Windows or justify replacement threads.
- Add real HWND grow278/shrink176/queued-message and readback-before-retire restoration tests. Same-size notification count and newer-generation CAS tests are required. No new retry budget/unbounded worker or focus/Z-order change.
- GPUI Frame.clear in this pinned version does not clear debug_bounds. After selector disappearance use current gate/guidance/footer/real hit geometry, not cached selector absence; do not patch the registry source or weaken visible bounds tests.
- Safe C# filters are direct PowerShell argument arrays with preselected and actual RUN names compared. Zero matches/broader names are failure; exclude both real Directory Opus tests. Never embed quoted name filters inside cmd strings.

## Scenario: Generation-owned native resize and deferred migration publication

### 1. Scope / Trigger
- Changes to probe generation, error/media layout, task details, native centered resize/replay, clipboard migration completion, or ordinary settings save.

### 2. Signatures
- `PlatformCommand::SetWindowBoundsCentered` carries native client pixel dimensions and correlation; `CenteredResizeCoordinator.TrySchedule(IntPtr root, Action operation)` returns admission only.
- `ClipboardApplication::save_clipboard_section(&mut self) -> anyhow::Result<()>`; `refresh_committed_migration_config(&mut self, committed: AppConfig)`; worker `migrate_storage(destination, config_path)` owns storage-root commit.

### 3. Contracts
- Two native workers, 64 HWND slots, one in-flight/coalesced next per HWND; accepted/applied=false is not settled. Never create a replacement worker for a stuck call.
- Pending transactions carry immutable generation; replay/writeback/retire require the same current generation, even when command IDs repeat. Correct client after center adjustment, then read back. GPUI uses WM_SIZE, not a competing real-HWND resize.
- Retire probe generation before validation. Compact error replaces guidance; actual viewport gates confirmation/submission. Ordinary 176/278 heights remain; media/error targets share one content budget.
- Local callback updates stay local; cross-entity work uses `cx.defer` or a later turn, including export/cache-hit paths.
- Migration worker commits disk before changing live history. Ordinary clipboard saves preserve latest disk storage_directory inside AppConfig::update_section; delayed UI completion reloads latest disk instead of overwriting newer settings.
- Native/browser registration mutation follows capture-all, register attempted-write before Write, reverse compensation and explicit aggregate rollback failure. Installer service stop requires exact owned path/protected tuple, never name alone. OLE cancellation is non-error and distinct from drop.

### 4. Validation & Error Matrix
- Stale native generation -> no overwrite/retirement/terminal ownership; full admission -> bounded rejection.
- Insufficient viewport/timeout -> withheld form or compact retryable error, never clipped release.
- Config commit failure -> source worker/root remains authoritative; delayed completion -> latest disk wins.
- Native mutation failure -> compensate all attempted steps; compensation failure -> explicit aggregate. No payload-bearing diagnostics.

### 5. Good/Base/Bad Cases
- Good: worker commits D, ordinary save while UI still S preserves D, completion loads newest settings.
- Base: native resize settles with final requested client and live moved center.
- Bad: synchronous cross-entity re-entry; accepted equals applied; timeout creates an overtaking worker; whole cached clipboard settings restore S.

### 6. Tests Required
- Invalid-probe generation, compact-error visible row, media error/footer bounds and task-detail queued native target tests.
- Real custom-client HWND final size/center; A/B replay/writeback/retire barriers, Guid.Empty reuse, bounded worker and delayed HWND responsiveness.
- `settings_save_during_migration_result_delay_preserves_committed_destination`: assert disk/root D after ordinary save and delayed completion.
- Synthetic browser-store mutation-then-throw at every step; normal cancelled vs dropped status; exact-owner uninstall predicate. Live pointer/DPI, registry and installer/service checks stay separate.

### 7. Wrong vs Correct
```rust
// Wrong: a cached S root overwrites the worker's durable D root.
config.clipboard = cached_clipboard;
// Correct: inside the locked disk-rereading section update.
cached_clipboard.storage_directory = config.clipboard.storage_directory.clone();
config.clipboard = cached_clipboard;
```

Evidence/limitations: `docs/audits/2026-10-08-sol-review-remediation.md`. Automated checks do not authorize packaging, product launch, real DOpus navigation or service mutation.

## Scenario: Foundation navigation and shared palette
### 1. Scope / Trigger
Shared theme/component mapping, navigation focus and representative Tools work surface.
### 2. Signatures
`theme::apply_component_theme(&mut App)`; `focused_action(id, focus, theme, disabled, window, on_click)`; MainWindow module_focus/tool_focus.
### 3. Contracts
Approved palette has one product/component mapping. Selection and keyboard highlight remain distinct; primary hover/pressed are distinct. Focus border has stable geometry. GPUI0.2.2 generates Enter/Space click on KeyUp; reuse one on_click, no parallel activation handler. Render-order tab stops, not unscoped global indices. Module switch clears orphaned menu focus and restores its trigger. Preserve fixed-client content budgets; shared visual tokens do not authorize native/window height expansion.
### 4. Validation & Error Matrix
Disabled/busy -> no pointer/keyboard dispatch; modified activation -> none; held keydown -> no duplicate; focus change -> no layout shift; old page menu -> no retained orphan focus. Native screenshot/DPI/provider unavailable -> explicit pending, not source-test certification.
### 5. Good/Base/Bad Cases
Good: Downloads->QuickFind and reverse focus before page controls, one release/one dispatch. Base: pointer and keyboard open/reuse one tool settings window. Bad: global tab_index interleaves content, simulate KeyDown-only and claim Enter activation, global row inflation clips fixed footer.
### 6. Tests Required
Foundation palette/component/density; actual MainWindow Tab/Shift+Tab and KeyUp selection;920/1060 minimum geometry, Tools settings reuse; shared action held/modifier/disabled single dispatch; full desktop existing fixed media/network footer contracts. Real screenshot/pointer/DPI remains separate.
### 7. Wrong vs Correct
```text
Wrong: Button leaf has no key listener -> add another Enter handler -> duplicate GPUI click.
Correct: inspect GPUI KeyUp click -> track actual focus -> one on_click -> assert real release and tab order.
```

## Scenario: Fixed probe-to-confirmation replacement
### 1. Scope / Trigger
User2026-10-09 amendment replaces the successful new-download same-HWND resize/watchdog above with fixed compact176→new confirmation278 (or existing media budget). Older resize contracts still govern other resize consumers, not this new success path.
### 2. Signatures
`DownloadApplication::create_fixed_confirmation`/`finish_new_download_handoff`; `PresentFixedWindowOverAnchorCommand { window_handle, anchor_window_handle, width, height }`.
### 3. Contracts
One pending slot installed before caller-ID dispatch; source window/view/generation/current URL must remain current. Create fresh window-bound inputs from owned draft, preserve request/probe/schedule/idempotency/media; hidden target is not natively final until one C# fixed apply. Bind source display; freeze URL/paste while pending. Result success commits/closes old once and defers local input focus; native activation only, no GPUI hidden initial_placement replay. Failure/disconnect/one IPC deadline closes target but retains successful probe for presentation-only retry, never re-probe because pixels timed out. No dimension polling/replay loop.
### 4. Validation & Error Matrix
Closed/stale/URL-mismatched source or late ID -> no resurrection. Create/apply/disconnect/timeout failure -> old owner/data retained and retry reachable. Cross-DPI target -> source-display/native monitor preparation before fixed apply. Disabled pending Input/Button -> no URL mutation.
### 5. Good/Base/Bad Cases
Good: real local probe creates new278, native apply/readback, original HWND destroyed, footer fits. Base: same-display ordinary fixed flow. Bad: test HWND-less fallback/direct278 screenshot and certify real transition, clear probe on viewport timeout, activate hidden GPUI target and replay stale placement.
### 6. Tests Required
Production localhost begin_probe/window identity/footer; failure/disconnect/timeout/retry preserves data and fresh input identity; close/stale/late result; actual pending URL/paste red-green. Owned native GPUI hidden-target placement + localhost→production command→C# apply→result/old HWND retirement. Mixed-DPI/held-pointer/installed runtime acceptance separate; TestAppContext native_window_handle=None is not native-gate evidence.
### 7. Wrong vs Correct
Wrong: metadata success→resize old HWND→viewport deadline→erase probe. Correct: retain probe→new fixed HWND→one correlated native apply→identity-guarded handoff; failed presentation retains data.

## Completion geometry and global rule ownership follow-up
- Ordinary collapsed task progress/completed share440x184. Same keyed HWND/generation changes content only; compare previous phase/details budget before issuing any native centered resize. Expanded280/failed196 remain genuine different-size transitions. Initial minimum is184, not a taller initial phase budget. Gate actual production command queue plus footer bounds; native owned-HWND center evidence is separate from installed-pointer/DPI acceptance.
- Global settings owns per-application Clipboard/Quick Launch rule drafts and one atomic save. Canonical present-empty rules override legacy exclusions; consumers apply committed rules on the next GPUI turn and never save the same settings themselves. Preserve cancel/failure draft, case/path/.EXE matching, independent scopes, busy action/close guard, desired-vs-applied shell semantics and no unchanged startup/tray dispatch. Test actual keyboard/pointer actions with existing focusable_content, no nested checkbox Tab stops or duplicate activation.
- Evidence: `repo:rust/apps/nxtools-desktop/src/downloads/polish_tests.rs`, `repo:rust/apps/nxtools-desktop/src/network_settings.rs`, `repo:rust/crates/nxtools-core/src/lib.rs`; [dated follow-up](../../../../docs/audits/2026-10-09-completion-and-global-blacklist.md).
- Global settings visual quality requires actual header/row/editor geometry, not merely source token use or no overflow: same application left edge, feature-header/checkbox/editor centers within1 logical pixel,32px input/action heights and center line,128px navigation rail, long-name fixed-column containment, bounded many/empty rows and busy/keyboard behavior. Default small component typography and centered label-buttons must not silently override the explicit12/18 text hierarchy. Palette mapping alone is not accepted UI polish.
