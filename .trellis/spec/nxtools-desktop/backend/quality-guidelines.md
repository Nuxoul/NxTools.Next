# Desktop quality and lifecycle scenarios

Evidence: `repo:rust/apps/nxtools-desktop/src/main_window/tests.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/tests.rs`, `repo:rust/apps/nxtools-desktop/src/quick_launch/tests.rs`, `repo:rust/apps/nxtools-desktop/src/ui/mod.rs#tests`.

Use production actions/bounds and worker/native seams. Toolbar/Progress lack automatic TestSupport: observe descendants/focus/scaled paint, never fabricate snapshots. No native a11y tree in TestWindow is not reader proof. DataTable external selection needs semantic/gesture gates; recipe-specific evidence is in queue03 outcomes. Source gates prove structure only; [UI authority](../../shared/ui-design.md) supersedes historical numeric intent, not behavior. No cross-owner reentry/fake timeout success/double resize. [Verification](../../shared/verification.md); retain all scenarios below.

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
- Ordinary collapsed task progress/completed share440x184. Same keyed HWND/generation changes content only; compare previous phase/details budget before issuing any native centered resize. The2026-11-13 progress layout uses expanded440x400 with separate connection/speed/completion TabBar pages; failed196 remains unchanged. Initial minimum is184, not a taller initial phase budget. Preserve official Medium32DIP speed Input, original draft/Enter/Blur/Escape commands, persisted completion intentions and bounded table viewport above the fixed footer. Same-attempt live range projection survives snapshot gaps; split plans replace it, task/attempt/source/status changes retire it. Gate actual production command queue plus footer/input/table bounds; native owned-HWND center evidence is separate from installed-pointer/DPI acceptance.
- Global owns independent Clipboard/Quick rule drafts and one atomic save; present-empty overrides legacy. Consumers apply next-turn, never save. Keep cancel/failure/retry, case/path/.EXE, busy/close and desired!=Windowsapplied. Queue09 uses Settings publicslots/Form/Checkbox requestedbool/SmallButtons, no row/childdispatcher;606tests/sourcegates are not native acceptance. [Queue09](../../../tasks/10-10-gpui-kit-component-visual-migration/ui-queue-outcomes.md).
- Evidence: `repo:rust/apps/nxtools-desktop/src/downloads/polish_tests.rs`, `repo:rust/apps/nxtools-desktop/src/network_settings.rs`, `repo:rust/crates/nxtools-core/src/lib.rs`; [dated follow-up](../../../../docs/audits/2026-10-09-completion-and-global-blacklist.md).
- Global layout requires aligned names/scope columns,32px original Inputs,Kit Small24px actions,center lines,default128px official Settings rail and bounded long/many/empty/busy states. Use released typography/tiers; retain720x480/input>=200/right<=700/table<=202/footer gates. Source/bounds tests are not aesthetic/native acceptance; exact adoption and scoped exceptions follow Queue09.

## Styled nullable media selection evidence
Released0.7.1 Radio checked on_change is inert. A nullable existing DASH audio row
uses ONE public StatefulInteractiveElement::on_click on the styled Radio, no parent
action/group on_change. Verify selected repeat-click and focused explicit KeyDown/
KeyUp once; simulate_keystrokes alone is not release evidence. Preserve original
typed media/draft/error owner. Calendar default-size availability does not certify
fixed popup fit: measure official tiers and record only proven narrow budget conflicts,
leaving alternate public scroll/date-picker adaptations unverified rather than rejected.

## Supporting download content adoption boundary
Schedule Switch/independentToggle retain one requested-value/local draft callback;
Small Form/Field labels use natural22.5px text_sm. User2026-10-12 authorizes official
H23/minute TimeField valid-time editing, superseding raw-invalid InputState retention.
Keep original Apply validation, independent enable/day/cross-midnight/Cancel/busy/error
owners. Test actual segmented editing separately from programmatic set_time/Apply.
Task aggregate XSmallProgress4px/DescriptionList/Toolbar/Checkbox project original
values/actions; domain segment dots remain domain content. Checkbox pointer prevents
focus movement by release design: keyboard test must actually Tab/focus it and send
both KeyDown/KeyUp, not add production keyboard compensation. Toolbar public content
preserves styled action variants; sized children would prepare buttons as ghost.
DialogContent/Footer/Alert belong INSIDE independently keyed native windows; Dialog/
AlertDialog modal hosts and DialogAction/Close must not introduce another dispatcher.
Verify live menu->delete request->view before claiming adoption. Preserve full risk,
once-only/cancel/disabled/pending/native-close and fixed long/error/footer bounds.
Evidence: repo:rust/apps/nxtools-desktop/src/downloads/polish_tests.rs and downloads/
tests.rs; owning queue05 outcome separates571tests/source gates from visual/native/
physical/DPI/reader/installed acceptance.

## Clipboard official Settings compatibility
Settings/SettingPage/Group/individualItem/Field own search/filter/selection/sidebar/
scroll; resettable(false). Use per-field public render and Form/Field for original
recorder/status and worker path, never entire old page in one custom item. Keep
validation/risk/runtime state/path visible; optional explanation actual Tooltip.
Do not invent editable numeric commit policy when original controls only saturate
integer +/- steps; exact NumberField editablef64Change is different. StyledSmall
Button public composition may retain original steps/section setters without a new
state/controller. Require tempSQLite worker/config real disk and typed command checks.
Sidebar observed nodes are release0-indexpaths, not guessed title label snapshots;
Tooltip current content requires actual700ms hover, not automatic role registration.
Keyboard test must verify actualfocusedButton and explicitKeyDownUp before claiming
one dispatch. Archive/storage fake picker tests return no path to avoid executing
data operations;busy backup/compact must be inert. NotificationCenter keeps existing
progress/error/cancel/details owner;no secondmanager. Verify minimumscrolltolastaction,
longpath/registrationerror,othersection/rootpreservation,recorderCancel and no search
save. Evidence: repo:rust/apps/nxtools-desktop/src/main_window/clipboard_page/kit_migration_tests.rs.
Controller/worker/native/preview files stay unchanged for presentation-only work;
tests/sourcegates never certify actual Clipboard/user/native/DPI/reader acceptance.

## Clipboard search/editor and navigation evidence
Search uses actual InputGroup/Addons/scopeInputGroupButton on original InputState;
Form/Field/defaultstyledTextarea/categoryInput owns ordinary editing frames. Preserve
IME/query intent/clear/generation, original save/Cancel/Escape/undo/focus/source set,
244px category occlusion and300x440/defaultuserresize; no mirrored worker/controller.
Test real tempworker text/category+source persistence and long/default/minimum/footer.
Worker startup resets legacy synthetic storage: initialize first, then seed fixtures.
External worker completion needs a bounded terminal-state wait, not run_until_parked
alone; never change reset/save policy for a fixture. Nativepreview owners remain intact.
User2026-10-12 authorizes continued realTab adaptation and Small24px search. BaseTab
stopsLeftMouseDown: place drag initiation and the sole pointer/key click on actualTab,
not an outerButton. Public track_focus uses original keyed handle; sole on_click restores
pointer focus then original selection. No compensating keyboard dispatcher. Test pointer,
focus survives repaint, Space down/up once, long wheel/context and original order/drop.
Earlier failed recipes are narrow historical evidence, not blanketAPI rejection.
Evidence: repo:rust/apps/nxtools-desktop/src/ui/kit_migration_tests.rs; post-gallery
outcome separates623tests from visual/physical/DPI/reader/installed acceptance.

## History/Link slots
ListItem slots keep variable-height/history owners; no competing ListState/query/scroll.
Toolbar keeps variants; Label/Alert String stays literal. Empty/error remains distinct.
Tools keeps one dispatcher/settings; Link420/28-24 with29px official StatusBar footer
keeps close-before-defer/route/timer/native owners (round2 supersedes historical22px).
TestPlatform is synthetic. Observe App.windows() for removal;
register timers before advancing clock; items_start measures natural height.
Evidence: repo:rust/apps/nxtools-desktop/src/ui/history_kit_migration_tests.rs;
[queue08 contracts/tests/limits](../../../tasks/10-10-gpui-kit-component-visual-migration/ui-queue-outcomes.md).

## Shared support
Loading copy is not percentage; Empty/Alert/Spinner keep distinct states and motion policy.
Current user-approved official notification integration supersedes the earlier private-completion/custom-shell exception: one released NotificationList owns card/stack/fixed5seconds/group remaining pause/close/transition; max3business metadata and3reusable official IDs keep admission bounded. No center expiry clock or GroupBox/Alert substitute. Use public content for at most2Small24actions, current-generation once-only deferred original-source callbacks. Verify actual official roles/paint/containment, pointer AND keyboard native grow/collapse, focused retirement/empty-reopen, stale close and near-expiry position change preserving lifetime. Native motion/error/disconnect must also reach GPUI reduce-motion. [Exact current evidence/limits](../../../tasks/10-16-menu-notification-refinement/official-notification-integration-outcomes.md); historical [Queue10](../../../tasks/10-10-gpui-kit-component-visual-migration/ui-queue-outcomes.md) is not current notification authority.

Grouped download completion uses only runtime `BrowserBatchSubmit` correlation on newly accepted tasks and the nonserialized completion snapshot. The original coordinator freezes `(task,attempt)` membership before terminals, waits for surviving members to terminate or be removed/cancelled, keeps independent batch summaries and standalone400ms feedback separate, and never grants power authority from summaries. First dispatch (attempt0→1) retains admission membership; later retries emit standalone feedback. Already-open keyed task windows transform normally; only new passive grouped terminal fallbacks are suppressed. NotificationCenter remains the sole official presentation owner. Both grouped and standalone completion notices display result text without task-location or temporary-mute actions. Explicit folder actions, configured completion sound and saved download preferences remain unchanged. Evidence: `repo:rust/apps/nxtools-desktop/src/completion_actions.rs#register_completion_members`, `repo:rust/apps/nxtools-desktop/src/downloads/completion_shutdown_impl.rs#publish_terminal_notifications`, `repo:rust/apps/nxtools-desktop/src/downloads/task_windows_impl.rs#apply_task_window_directive`, `repo:rust/crates/nxtools-download/src/engine/browser.rs#submit_browser_batch_correlated`; compiled fixtures do not certify native/visual acceptance.

## Scenario: Installed-feedback theme, monitor and explicit native actions

### 1. Scope / Trigger
System theme, foreground download limiting, link action separation, monitor/confirmation and protected service coordination. Approved owner: `.trellis/tasks/10-12-installed-feedback-capabilities/`.

### 2. Signatures
`Theme::active(&App)`, `theme::observe_theme<T>(window, Context<T>)`; `SystemThemePreferenceChanged { correlation_id, dark, native_error_code }`; `ShellOpenAction::OpenWith` serializes `open_with`; `DownloadEngine::set_runtime_speed_cap(Option<u64>, Uuid)`; `global_speed_limit()` remains desired manual state, `effective_global_speed_limit()` is actual ceiling. `ProgressDisplaySampler::observe(Duration, bytes, total, transferring)` runs at snapshot replacement, never render.

### 3. Contracts
Persist SystemSettings.theme through atomic owned section; old config defaults Dark. Draft/Cancel never apply; Save defers shared palette update; System uses native snapshot/change/reconnect, error/unknown defaults dark. Weak keyed observers notify on next turn; preserve input/window/appearance identities. Background policy uses existing ForegroundChanged process ID, no polling; unknown/disconnect disables temporary cap. Effective global/task caps use minimum without overwriting persisted manual/task values. Monitor samples recent timestamped byte counters once per second, terminal/pause/rewind resets immediately; never smooth already-smoothed engine rate. Filename max two lines plus full Tooltip/name; segment unknown size is not completion. Ordinary246px confirmation retains fixed hidden-target handoff;200px schedule popup leaves trigger operable with124px scrollable calendar. Kit Root fills opaque even behind transparent content: refine only taskbar Root background after constructor lease.
Pending Shell command ID is installed before dispatch, caller correlation spans C#. Only matching success closes current originating HWND/generation. OpenWith cancellation is `success=false,error=null,data=cancelled`, clears pending without toast/close; human selection has no inactivity deadline, ordinary Shell retains its15second UI deadline. Reveal final→noncompleted temporary→existing save directory; click does not create directories. Link capture never opens/copies; explicit row-open and secondary Copy are distinct, Copy stops propagation. Protected indexer matching tuple reuses; healthy older tuple requires correlated UAC atomic upgrade; malformed/unapproved tuple fails closed. Rust handles both installed/upgraded through existing protected restart identity checks; no admission bypass.

### 4. Validation & Error Matrix
Theme native error/disconnect→System dark, explicit Light/Dark unaffected. Background zero/overflow enabled input→local validation; disabled retains saved rate. OpenWith directory/missing file→reject; cancel→quiet retained window; failure/timeout/disconnect→retryable retained window; late generation→no close. Damaged protected tuple→repair_required, cancelled UAC→authorization_cancelled, never Ready from service Running alone. Opaque Root scene→regression failure; scene alpha does not certify DWM.

### 5. Good/Base/Bad Cases
Good: Save Light repaints existing windows; other-app cap returns latest manual limit; cancelled chooser preserves completed view. Base: passive windows retain native bounds/focus. Bad: sync cross-owner observer, double averaging, click-open writes Clipboard, send-before-register, unconditional old-service reuse, transparent child claimed to erase Root.

### 6. Tests Required
Theme draft/Cancel/Save/reopen and component/custom palette plus auxiliary paint/entity/generation checks; Core legacy/section merge; foreground/manual/disable/disconnect engine state; runtime_speed_cap_contract persistence/concurrency/diagnostic8tests; production completed window cancel/failure/timeout/late success; category bounds and schedule trigger non-overlap; taskbar production Root quads preserve progress paint. C# exact installed native/integration fixtures and native Quick/Clipboard message/UIA. Full Desktop650 tests, strict production/tests lint and workspace/fmt are source checkpoint, not physical/installed/visual acceptance.
Open With wait regression advances the actual DownloadApplication/TestApp clock beyond601seconds before returning cancellation, checks the same pending ID and originating window survive, and rejects a late success after cancellation. Retain ordinary Reveal timeout/disconnect/stale-generation controls. IPC human-operation exemptions follow [the original router contract](../../nxtools-ipc/backend/quality-guidelines.md#scenario-human-driven-shell-authorization-and-native-drag); native Shell execution waits for its terminal/cancel while menu enumeration remains bounded.

### 7. Wrong vs Correct
Wrong: `.bg(transparent)` content under opaque Kit Root; send Shell then register; average engine rate again; upgrade result treated as initial-only success.
Correct: per-instance deferred Root background refinement; register caller-ID before dispatch; timestamp byte-delta sampler; installed/upgraded share approved protected tuple restart policy.

Evidence: `repo:rust/apps/nxtools-desktop/src/theme.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/progress_display.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/tests.rs`, `repo:rust/apps/nxtools-desktop/src/network_settings/kit_migration_tests.rs`, `repo:csharp/NxTools.PlatformHost.Tests/SystemThemePreferenceTests.cs`. Actual visual inspection remains blocked by unavailable Browser panel; current captures are not visual approval.

## Installed feedback round2 source contract
Clipboard selected-only rows never reveal management rail; hover or actual keyboard focus may reveal. Persistent pin/favorite flags paint independently outside the hidden time/action slot, nonshrinking16px identifiers; test actual descendant paint, distinct SVG alpha and minimum-width no overlap, not cached bounds alone. Root/key-release retains one action/paste/menu owner. Window-keyed use_keyed_state is render-only; never create same-name focus outside render in a regression fixture.
Conflicting ForegroundChanged while GPUI panel is still active is rechecked next turn on the same weak entity/window/session; still-active snapshot discarded, actual inactive window uses original hide. External press/real focus loss/Escape behavior remains. Preview reuse offscreen -> acknowledged SetWindowBounds(client readback) -> update existing entity -> committed next frame -> generation-scoped reposition; no competing GPUI native resize. Test fail/timeout/cancel/oldHWND, active-vs-inactive close callback and same-window size change. Actual font wrapping fixture passes; no first-preview latency claim.
Completed list speed uses reliable optional durable DownloadTransferStatistics, never stale live tick/timestamps; legacy/crash unknown staysdash. Reveal list error copy describes target operation, not nonexistent completion window. Success data target_confirmed_foreground_not_confirmed closes only current originating task window after correlated native result; cancellation/failure/timeout/stale/disconnect unchanged. Link actual ListItem focus/click/tooltip/public default padding replaces custom outer2px box/inner shrink; StatusBar default grammar fits29px (single67/collapsed91/expanded123), row28/copy20/expander24 and original timer/session retained. Boundary tests and full674Desktop pass are source evidence, not installed/native/DPI/reader/visual approval.
Evidence: `repo:rust/apps/nxtools-desktop/src/ui/history_feedback_tests.rs`, `repo:rust/apps/nxtools-desktop/src/clipboard_preview_presentation_tests.rs`, `repo:rust/apps/nxtools-desktop/src/main_window/download_average_tests.rs`; task `.trellis/tasks/10-13-installed-feedback-round2/`.

## Quick Launch search-extension execution contract
Approved W2–W5 retain existing InputState/IME/native window/preview and final Finder rank. Finish local ordinary sources before deciding explicit indexed homophone permission; normal indexed files remain enabled. Launcher recent mode is explicit/default-off; file-dialog same-prefix requests retain ordinary syntax and consume current-generation terminal batches. Pending new search rejects retained-row execution, never queues automatic activation. Every deferred primary action rechecks open/generation/input/target; action-mode execution also rechecks mode/level/parent query/selected typed action including Shell identity. Close/reopen/input/action replacement must emit no stale command. Keep cross-owner work next-turn and correlated native success-only close.
Evidence: `repo:rust/apps/nxtools-desktop/src/quick_launch/panel.rs#activate_selected_action`, `repo:rust/apps/nxtools-desktop/src/quick_launch/tests.rs`, `repo:rust/apps/nxtools-desktop/src/quick_launch/file_dialog_search.rs`. Actual-entry red/green regressions and Desktop684/684 are source evidence, not installed pointer/IME/DPI/visual acceptance. Exact seven-section contract and data-migration/native limits: `.trellis/tasks/10-14-winshun-w2-w5/code-spec.md` / outcomes.md.

## Manual extension feedback and Clipboard result navigation
Per-run pending correlation precedes dispatch; cancellation requests do not drain ownership or claim cleanup/rollback. Bounded session activity contains manual invocation metadata, not automatic-event/search telemetry. Host-confirmed derived IDs are observed at the real serialized commit boundary with package/handler/content/correlation identity, never accepted from script completion. A write admitted before cancellation may finish later; preserve bounded observation through its actual reply and publish only state transitions, no payload telemetry. Explicit View Result uses weak originating window/panel/session and current query generation, preserves original history ordering/paging and never copies, pastes, marks used, opens a new panel or replays the script. Plugin-provided explanation is labelled and bounded, not host evidence. Released Dialog Escape may close before global keystroke observers run: preserve pre-dispatch dialog ownership so one key cannot also hide Clipboard. Production actual hit-target cancellation, Escape, stale/closed query and temporary worker/SQLite tests are required. Evidence: `repo:rust/apps/nxtools-desktop/src/extensions/manual_runs.rs`, `repo:rust/apps/nxtools-desktop/src/extensions/manual_actions.rs`, `repo:rust/apps/nxtools-desktop/src/ui/extension_feedback.rs`, `repo:rust/apps/nxtools-desktop/src/ui/kit_migration_tests.rs`; `.trellis/tasks/10-15-extension-action-feedback/outcomes.md` separates compiled, visual and installed evidence.

## Scenario: Passive Clipboard Markdown and code presentation
### 1. Scope / Trigger
Changes to full-item hover-preview classification, safe Markdown projection, readonly syntax display or preview mode controls.
### 2. Signatures
`preview_text::classify(&str) -> TextKind::{Plain,Markdown,Code(language)}`; `safe_markdown(&str) -> Option<String>`; `ClipboardPreview::{new,set_item,on_measured_layout,set_measured_layout}`. `PreparedText` owns per-item official TextViewState/EditorState; `ViewingMode::{Auto,Source}` is presentation-only. Screenshot2026-10-20 removes the language/Markdown menu; auto language detection/grammars remain.
Evidence: `repo:rust/apps/nxtools-desktop/src/ui/preview_text.rs`, `repo:rust/apps/nxtools-desktop/src/ui/preview_rich.rs`, `repo:rust/apps/nxtools-desktop/src/ui/preview.rs`.
### 3. Contracts
Preview uses full hydrated text, never400character summary. Original item bytes/action/native formats remain. Auto defaults prose to Markdown and distinctive code to readonly highlighting, no manual language control. Rich input/output<=64KiB/1200lines/noNUL,AST<=32depth/8192nodes/pathological quote preflight; images become unloaded labels,HTML escaped,fences allowlisted; verify safe projection before official TextView. Source toggle only affects current view, callbacks check appearance generation. History uses mounted cached400character safe TextView, natural content height at actual available width, compact minimum only; fixed24/40/96buckets and max_lines4 are superseded by user rejection. Read measured ListState bounds for page stride. Preview uses actual official non-scroll rich layout measurement in GPUI layout phase, then existing native client confirmation; source/code use bounded real-font measurement and over-budget original is a bounded scroll viewport. Explicitly sized/nonshrinking body with size_full TextView and separate nonshrinking footer prevents masking. First hidden/reuse offscreen content measurement completes before native final bounds,first final-size frame is explicitly drawn after ACK before show; keep same passive HWND/noactivate/generation/focus/cancel/native API owners. Never infer ready from rendered_text emptiness: structural Markdown can have no copy text; use parsed source readiness. No implicit network/resources,execution,formatting or payload rewrite.
### 4. Validation & Error Matrix
Ambiguous language -> default Markdown prose; unknown fence language -> ordinary code block/no raw name in highlighter log. Over budget/deep/failed safe projection -> original view plus explicit preview fallback label/Tooltip. Missing grammar/style is a regression, not imported-type proof. Stale control callback/item replacement -> no new-mode mutation. Markdown resource must never reach HTTP/file loader. History link click invokes original item action, never URL navigation or a dead click region. No blanket warning suppression or dependency-source patch.
### 5. Good/Base/Bad Cases
Good: copied Rust preview highlights; Markdown table/fences/italic/list render immediately; original paste bytes unchanged. Base: ordinary prose uses default TextView without a manual toggle. Bad: use Editor as Markdown renderer, infer language for x=1, load untrusted image URLs, or equate a wrapper selector/parse tree with rendered text/nonempty syntax styles.
### 6. Tests Required
Actual production rendered_text/readonly/styles/resourceprivacy; source toggle pointer/current entity; variable paragraph count and longlink/CJK280/460width natural row height/complete bounds; actual official rich layout vs body allocation, single source-control/no language menu, scroll-to-end must notify/redraw and assert last block bounds inside viewport, not just scroll index. Typed native bounds equals measured size before ordered noactivate/bounds/draw/topmost/show; short replacement/structural emptytext/latecancel/oldHWND/timeout boundaries. Retain existing preview lifecycle/fullDesktop/strict/workspace/source gates. TSX/C++/C# public grammar supplements remain. Generated/headless geometry does not certify installed/physical/mixedDPI/reader/user visual acceptance.
### 7. Wrong vs Correct
Wrong: clipboard text -> TextView::markdown directly, syntax tree exists -> claim highlighter works. Correct: bounded classify -> sanitized verified projection or readonlyEditor -> actual rendered text/styles regression; original item remains independent.

## Scenario: Opt-in clipboard currency reference conversion
### 1. Scope / Trigger
Currency tool parsing, HTTP/cache lifecycle, clipboard notification or compact settings changes.
### 2. Signatures
`tools::currency::{parse_catalog,parse_pair,parse_amounts_in}`; `RateSnapshot::{catalog,validate,refresh_due,stale,convert,message}`; `fetch_rates(NetworkOperationSnapshot,now,id)`; `ClipboardApplication::{currency_catalog,set_currency_settings,refresh_currency_rates,handle_currency_capture}`. Evidence: `repo:rust/apps/nxtools-desktop/src/tools/currency.rs`, `repo:rust/apps/nxtools-desktop/src/clipboard/currency_tool.rs`, `repo:rust/apps/nxtools-desktop/src/main_window/tools_page/currency_settings.rs`.
### 3. Contracts
Default off/CNY; strict whole-copy<=256bytes/8amounts, case-insensitive catalog codes and longest marker matching; explicit code overrides generic symbol, $/刀/美刀/美元/美金=USD and ₩/￦=KRW. No prose extraction or clipboard writeback. User2026-10-20 replaces the former31-code list/ExchangeRate-API/Frankfurter fetch with fawazahmed0 complete code/name catalog plus fixed CNY-base bulk rates. Use jsDelivr /v1/currencies.min.json and /v1/currencies/cny.min.json; documented Cloudflare fallback fetches the same complete pair. Names are not rates. One immutable NetworkPolicy snapshot/client, singleflight,8srequest/3sconnect/7ssource/16swhole sequence, no redirects;128KiB per response,512KiB cache,<=4096 entries,2..16ASCII-alnum codes containing a letter,<=256char control-free names, finite positive rates<=1e12, CNY1 and valid nonfuture date. Catalog/rate key sets must match before publishing. Empty names fall back to code. Bundled CC0 catalog seeds offline choices only; no bundled runtime rates. Version2 names/rates persist atomically as currency-rates-v2.json; read-onlyv1 fallback stays searchable/usable and requests refresh. Refresh24h; stale also marks reference date>=4days. Local cross-rate amount/rate[source]*rate[target],2decimalreference-only output. Existing pause/exclusion/writeback/capture guards and generation/capture/pause notice tokens remain; list->one unique-invocation shared notice. Existing180x160tile/460x320featureForm/32pxSelect/SmalloutlineUpdate preserved; callbacks next-turn guarded.
### 4. Validation & Error Matrix
Malformed/prose/unknown code/overbudget copy->no effect; missing rate->explicit no-result,never inventedconversion. Invalid/partial/mismatched pair or refresh/cache-commit failure->lastgood retained and retry/error visible. Invalid cache->reject,try legacy fallback; unknown persisted but syntactically valid target remains selected/unavailable,not silently CNY. Core validates code syntax only; Desktop owns live membership and section saves. Stale ID/token->no newer state change or retired amount notice. Failed config save retains enabled/target/disk. Disabled default never automatically fetches.
### 5. Good/Base/Bad Cases
Good: `₩1000 5美刀 2刀` converts locally in one notice; `5CAD 2.5cad 32$ €43` remains supported. Base: code/name catalog339 and rates339 captured together validate without network. Bad: assume all codes are three letters, confuse names with rates, send copied amounts remotely, overwrite clipboard/full stale config, or enable a Select search box on plain Vec and claim actual filtering.
### 6. Tests Required
Retain original clipboard/privacy/stale/daily/singleflight/failed-save tests. Add actual production won/dollar-alias notice, all bundled codes/nonISO/digit-leading/metals/new runtime codes, Core dynamic-code normalize/roundtrip, paired schema/count/date/rate/mismatch/budget validation, version1/2 cache/restart/corruption/real atomic rename denial, bounded localHTTP paired fallback/status/redirect/timeout/requestprivacy and captured complete provider fixtures. Search uses exact released SelectState<SearchableVec<CurrencyChoice>>::searchable(true), not plain Vec (its default perform_search does not filter). Test actual keyboard code/name/Chinesealias filtering, no-match Enter no-write, Escape no-save, statusnotify preserves search, catalogrefresh preserves committedselection and adds new choices, originalwindowreuse/busy/error/settingssave. Only realcatalogrevision updates delegate next-turn; unchangedstatus must not clear query. Core/Network/strict/workspace/fullDesktop/source/spec/license remain required. No native/physical/DPI/reader/visual/installed or24hsoak claim.
### 7. Wrong vs Correct
Wrong: fixedISOallowlist or catalog-onlyfetch treated as rates; search UI with unfiltered Vec; raw copied text in HTTP. Correct: bounded validated paired catalog/rates -> atomic lastgood -> catalog-driven exact local parser -> current-token shared notice; official SearchableVec owns filtering on original Select/session.

## Scenario: IME Enter ownership ends with its key gesture
### 1. Scope / Trigger
Quick Launch InputState composition/commit/cancel and Enter dispatch changes.
### 2. Signatures
`ImeEnterGate::{observe_composition,observe_change,consume_enter,reset}`; production panel InputEvent::PressEnter, input-focused on_key_up and activate_selected. Evidence: `repo:rust/apps/nxtools-desktop/src/quick_launch/panel.rs`, `repo:rust/apps/nxtools-desktop/src/quick_launch/tests.rs`.
### 3. Contracts
Native IME can consume confirming Enter without PressEnter. Reject live marked text; consume bubbled same-gesture confirmation; when input-focused KeyUp observes no marked text, retire confirmation/composition latch. Never carry that latch into next ordinary Enter. Keep original current-result/querygeneration/searchpending/deferred identity/native action/close policy; do not bypass stale-row safety to hide a no-op.
### 4. Validation & Error Matrix
Active preedit or same-gesture confirm->no native action; next ordinary Enter after commit/cancel->one current action. New-query pending/retired target->no old action. IME events outside fixture/native access->report unverified, not installed pass.
### 5. Good/Base/Bad Cases
Good: IME commit consumed natively, release, ordinary Enter opens once. Base: ordinary Unicode input uses existing dispatch. Bad: sticky pending_confirmation_enter consumes a later gesture; any platform-command assertion passes on icon traffic.
### 6. Tests Required
Actual production InputState replace_and_mark/replace_text/unmark plus real KeyDown/KeyUp, no-bubbled confirm and cancel regressions; active preedit/same-gesture confirmation safety; assert pending action and typed OpenShellTarget, not arbitrary queued command. Retain original IME and w5 pending-old-row gate, fullDesktop. Reference Listary managed first then SwiftList current Helpers input boundary; proprietary native engine unavailable. Real Windows IME/physical/installed acceptance separate.
### 7. Wrong vs Correct
Wrong: every composition sets a future Enter debt with no gesture end. Correct: same-gesture safety + live marked-text guard + KeyUp retirement; immutable result dispatch remains unchanged.

## Scenario: Natural notification sizing and configured official timeout
### 1. Scope / Trigger
Changes to notification text layout, native height allocation, dwell setting or the narrow local Component patch. User2026-10-20 supersedes the historical54pxtextcap/fixed5s/read-onlysettings contract, not official presentation/lifecycle ownership.
### 2. Signatures
`NotificationCenter::{set_duration_seconds(u32,cx),upsert(AppNotification)}` snapshots 1–60/default 6 into new timed records; `Notification::duration(Duration)` is the approved local Component builder, carried to current0.10.0. `OfficialPresenter::content_measured` reports actual public-content height to existing ScreenHost::measured; original PrepareNotificationWindow/readback/draw/reveal is unchanged. Evidence: `repo:rust/apps/nxtools-desktop/src/notifications/presentation.rs`, `repo:rust/apps/nxtools-desktop/src/notifications/expiry_tests.rs`, `repo:rust/apps/nxtools-desktop/src/network_settings/notification_tests.rs`, `repo:vendor/gpui-component-0100/NXTOOLS-PATCH.md`.
### 3. Contracts
Retain official Notification/NotificationList/ToastStack/card/clock/whole-group hover/focus remaining-time pause/close/animation and bounded3business records/once-only source actions. No second clock/custom card or registry edit. The licensed local Component0.10.0 patch keeps optional duration/defaultNone/public builder forwarding into original ToastOptions, plus optional header_action in the same notification.rs; unspecified upstream remains5,autohidefalse persistent,system delivery unchanged. Config/startup publish dwell only for newly admitted records; existing entity/generation/countdown must not reset. GlobalSettings original atomicApplyCancel,32pxNumberInput/168pxlane/stableInputState,1–60 inlinevalidation; set_step(None) prevents competing defaultstep before project clamped Step callback.
Text grows naturally, not clipped to54px before measurement. Native336card/360window/400height budget stays; per-retained-card share subtracts24edge/30officialpadding/14peek. Window-local compatibility content additionally subtracts top inset from real viewport. Nonshrinking natural body scrolls only at budget; officialSmall24actions remain reachable outside textscroll. Resize current native surface with keep_visible, never hide/reveal or inflate transparent catcher.
### 4. Validation & Error Matrix
Ordinary multiline->all text initially inside viewport; huge body->bounded scroll-to-true-end with actions reachable;3expandedlongcards->all inside actual native client. Invalid/empty/0/61/noninteger setting->inline error,no durable/live update. Failed save->prior config/live duration retained. Cancel->no writes. Setting/position change->no mounted timer reset; stale close->no newer removal; persistent progress->no deadline. Actual nativeworkarea clamp/DPI/physical acceptance remains separate.
### 5. Good/Base/Bad Cases
Good: currency amount/reference/source all visible without initial scroll; user types8 thenCancel leaves17 on disk; Apply9 changes future notices only. Base: upstream raw Notification still expires5s; product default6. Bad: cap text54px then sizehost from clipped bounds; combine defaultstep and projectstep; widen timers by repushing every mounted card after settings notify.
### 6. Tests Required
Red-capable actual textbody/copy/card bounds and real production screen-host fake-native acknowledgement; short->multiline naturalgrowth/100line scrollend/3longcards/actions/smallviewporttop+bottom. Official1/6/60expiry,unconfigured5,hover remainingpause,setting/position noreset,replacement/progress; productionInput pointer/keyboardstep/range/busy/Cancel/Apply/reopen/savefailure. Preserve existing action clicks/generation/native hiddenframe/readback/timeout/disconnect tests. FullDesktop/scopedstrict/workspace/fmt/source/spec/license and exact vendoredrelease diff. Test snapshots/staticgates do not certify screenshots,physical/DPI/reader/installed acceptance.
### 7. Wrong vs Correct
Wrong: `max_h(54)` hides content; private/parallel clock or lifecycle replacement to change5s. Correct: natural publiccontent bounded by actual stack capacity -> existing native measured resize; duration snapshot -> minimal publicbuilder -> unchanged officialToastOptions.

## Scenario: Clipboard usable shortcuts and one capture toggle
### 1. Scope / Trigger
User-rejected shortcuts, pause/search focus and accidental preview behavior. Evidence: `repo:rust/apps/nxtools-desktop/src/ui/efficiency.rs`, `repo:rust/apps/nxtools-desktop/src/ui/search_input.rs`, `repo:rust/apps/nxtools-desktop/src/ui/capture_controls.rs`, `repo:rust/apps/nxtools-desktop/src/ui/efficiency_tests.rs`, `repo:rust/apps/nxtools-desktop/src/ui/history.rs`.
### 2. Signatures
`ClipboardPanel::{history_keyboard_ready,number_key_down,number_key_up,move_page,activate_entry_correlated,search_input_ready}`; `SearchInputFallback` implements public GPUI InputHandler by delegating to ElementInputHandler<InputState>. `render_capture_controls` -> deferred `ClipboardApplication::set_capture_pause(paused,false,cx)`; input actions -> deferred original `dismiss_preview_for_interaction` -> dismiss_preview.
### 3. Contracts
Page/Ctrl+number do not depend solely on search focus or any focused descendant. Ctrl is required on digit down; unmodified1..9/0 are search text even when the query is empty. Same-window interceptor repairs absent focus before element dispatch; down captures immutable ID/querygeneration/correlation, matching up executes once; Ctrl-first release is not cancellation. Restore original search on summon/activation/no-focus; focused noninput controls use a paint-installed platform text/IME forwarding handler with original search input_bounds, never key-label text synthesis. Real focused Input/Textarea/Editor/OTP, category/text editor/dialog/menu retain their owner. Preserve pending/querygeneration/held-repeat/IME guards and official control Enter/Space. Accepted number/pause/item action/search change retires old preview candidate and native pending stage next-turn; no stale reveal or second controller. Keep one Small ghost Pause/Play header icon, pending/nativeACK/errorretry/privacy/reconnect/no-backfill unchanged. Shortcut accepted/rejected(reason)/cancelled/dispatched plus original action/IPC/terminal share correlation; repeated rejection rate-limited1second, no held-repeat/general text/keys/query/payload logging. Focus logs only restoration transitions; preview cancellation only when active.
### 4. Validation & Error Matrix
No focus+Ctrl2 -> exact immutable index1/full PasteText original target, not selection-only or preview. Oldquery/deleted/pending/IME/editor/menu -> no stale paste and semantic rejection/cancel reason. Typing outside input -> same search state with first ASCII/Unicode/IME commit once, no popup hide; real editor stays untouched. Pause/number with queued or visible preview -> original controller candidate/pending generation invalidated before late show. Native failure -> no false success, retryable same intent. Markdown links retain row action, no resource navigation.
### 5. Good/Base/Bad Cases
Good: blur all GPUI focus then Ctrl2 still pastes full item, type from a header control -> search, pause cancels queued preview. Base: full-by-ID text and native success-only action close survive. Bad: force fixture search focus and infer installed shortcuts work; require Ctrl held at release; only assert arbitrary IPC; reconstruct IME from key labels; let a header action inherit stale hover preview.
### 6. Tests Required
Production events from no focus/search/control, actual ModifiersChanged/Ctrl-first/repeateddown/keyupcharacter forms, exact typed payload/target/correlation, plain digits with empty/nonempty search and query/editor/dialog/menu/IME guards. Initial popup focus must pass without a fixture focus repair, and ordinary text must reach the original search InputState. Header-control ASCII firstchar/no double insertion, Unicode mark/commit/caret using originalInputState, activationrestore/editorownership; queuedcandidate and normal pointerhover->Pause/number cancel original preview and reject late native lifecycle. Preserve top-right/blankheaderdrag/nativeACK/retry, original row/link actions/summary/paging and full passive presentation regressions. Relevant Desktop tests, strict affected-package lint, lifecycle/spec/scopedfmt are required; source/compiled tests are not installed/physical/IME/DPI/reader/user visual acceptance.
### 7. Wrong vs Correct
Wrong: helper passes -> claim keyboard works; rendered link returns no-op. Correct: production events -> current immutable item -> correlated existing action; safe cached TextView -> original row/link action owner.
Header icon pointer regression must observe the actual header MouseDown before the native handle early return: icon press yields0drag calls, blank-header press1. The layout slot stops only left MouseDown bubbling; keep official Button keyboard/click/disabled behavior. Official TextView on_link_click is Send+Sync; capture a weak row-state handle, clone original action outside its lease, then invoke it. Do not capture the row entity strongly inside its own render state or change the whole Rc action graph merely to fit a presentation callback.

## Scenario: Rendered Clipboard content owns its height
### 1. Scope / Trigger
User screenshot rejects masked rich text/history and redundant Markdown button. Evidence: `repo:rust/apps/nxtools-desktop/src/ui/history.rs`, `repo:rust/apps/nxtools-desktop/src/ui/preview.rs`, `repo:rust/apps/nxtools-desktop/src/clipboard_preview_presentation.rs`.
### 2. Signatures
Natural ListItem/TextView rows bounded by `TEXT_CARD_MAX_HEIGHT=210.0`; text_content receives remaining body maximum. `ClipboardPreview::measure_rendered_layout` -> `on_measured_layout` next-turn -> `complete_preview_content_measurement` -> original typed native bounds. `PreviewLayout.maximum_height` carries the separate preview work-area cap.
### 3. Contracts
Use actual rendered Markdown dimensions below the user's text-card210DIPmaximum, not raw source/newline counts or fixed multiline buckets.210includes metadata/border/padding and any inline feedback, excludes external8DIPgap; reserve feedback allowance before capping summary body, keep header/feedback nonshrinking and no nested scroll owner. Short content remains natural; over-budget list summary may clip, never original data or preview. Images retain190DIPordinarycard/files62DIP. Preview measurement occurs during request-layout/render only, cached by content/mode/width; native callback next-turn/current-token/HWND and timeout bounded. Initial hidden/reuse offscreen prevents exposing stale geometry. Explicit draw clears its arena token and releases leases before application continuation. Source only control, no language menu; no fade concealing short clipped text.400char list projection/safe parser bounds remain independent from full action payload.
### 4. Validation & Error Matrix
More paragraphs/narrower width -> greater actual natural row height until whole-card210DIP; complete body fits below budget, over-budget body clips only within bounded summary. Short text not stretched to maximum; feedback actions stay inside cap. Preview beyond its own work-area maximum -> internal scroll with final block reachable/footer outside. Cancel/oldtoken/oldHWND -> no native show/resize; structural Markdown with empty rendered copy still completes. Oversize source -> bounded original scroll/no unbounded measurement.
### 5. Good/Base/Bad Cases
Good: long links wrap to taller rows, header/footer stay readable. Base: source toggle retains original entity/bytes. Bad: four-line clip plus fixed96px, compare only wrapper height or scroll index, manually render second action labelled Markdown, or use feathering to hide a layout error.
### 6. Tests Required
Production official ListItem card<=210/slot<=218 at narrow/wide Light/Dark, short natural/under-budget fullcontent/longlinkreflowuntilcap/header24DIP and external8DIPgap; actual image/file190/62 retained, clipped-summary click pastes complete original. Inline feedback running/cancelling/success all controls inside210DIPcard; restore short fixture before unrelated reorder/filter tests so offscreen gestures are not mistaken for product defects. Original full preview last-block/footer/native-stage tests plus full Desktop/lint/workspace/lifecycle/spec/catalog/scopedfmt. User screenshot/source bounds are not installed/physical/DPI verification.
### 7. Wrong vs Correct
Wrong: stored newlines decide rich height. Correct: official renderer layout at real width -> natural row or bounded measured native viewport -> actual last block/footer containment.

## Scenario: Clipboard cells remain separate and icon tint fades
### 1. Scope / Trigger
User-requested history cell separation and localized source-application icon fade, explicitly rejecting content-type colors. Evidence: `repo:rust/apps/nxtools-desktop/src/ui/history.rs`, `repo:rust/apps/nxtools-desktop/src/ui/efficiency_tests.rs`, `repo:rust/apps/nxtools-desktop/src/clipboard_worker/icon_colors.rs`, `repo:rust/apps/nxtools-desktop/src/ui/icon_colors.rs`.
### 2. Signatures
Existing ListItem public Styled/child; `SourceIconTint` receives `Option<[u8;3]>` and uses public GPUI `linear_gradient`/`linear_color_stop`. `ClipboardCaptureWorkerHandle::source_icon_colors_async(Vec<PathBuf>) -> Result<SourceIconColors>` resolves at most4paths on the original serialized worker; `SourceIconColors = Vec<(PathBuf, Option<[u8;3]>)>`. No new owner or upstream patch.
### 3. Contracts
8DIPgap outside click/focus target,1DIPofficialcardborder/theme radius/idle surface. Border adds2DIPchrome without reducing text/image body; natural Markdown rows retained. Source24DIPrail is a background, not overlay:90degreeleft->right sameRGBalpha>0to0. Hue comes only from actual source-{64lowerhex}.png pixels, never ClipboardKind/name/hash/copied content; identical icon means identical hue for Text/Image/Files. Keep original sharp icon, kind fallback glyph only when absent, and theme-neutral hue when unavailable. Worker validates managed-root regular nonreparse PNG<=1MiB/512x512/4MiBdecode, samples<=64x64 without enlarging smaller icons, ignores alpha<32 and alpha-weights colors. Meaningful chromatic coverage>=5% uses dominant neighboring hue bins to group shaded logo pixels over neutral backdrop; otherwise retain neutral palette. Cache<=128including unavailable results, clears on root change; panel retains only active list keys, one pending batch, revalidates open/querygeneration/activekeys on reply. Palette jobs never supersede history queries. No render IO/decode, implicit resources, timer/frame diagnostics; existing Kit owns selection/hover/keyboard. Bounded batch diagnostics carry correlation/count/cachehits/unavailable/elapsed only, not paths/icons/RGB/payloads.
### 4. Validation & Error Matrix
Gap click -> no paste; icon/header/body click -> original exact immutable item/native action once; management -> no row paste. Width/theme/selected/hover preserve separation/natural body/no action overlap. Missing/corrupt/oversize/outside-root icon -> cached unavailable/theme-neutral fallback, not a type color. Closed/oldquery/retiredkey reply -> no metadata resurrection; root change -> invalidate worker cache. Null source icon uses kind glyph, not image for all rows.
### 5. Good/Base/Bad Cases
Good: green application icon gives green fade across Text/Image/Files, switching source icon to blue changes hue; cards remain separate and tint clears before body. Base: absent source uses neutral Light/Dark tint; original preview/native/data callbacks unchanged. Bad: FilesYellow/ImageBlue/TextNeutral, resizing a32px icon upward before palette extraction, render-time decode, whole-history neon, overlay hitbox, fixed text height/fake control, or painted quads treated as visual approval.
### 6. Tests Required
Production panel adjacent slots/official ListItem painted border and8DIPgap; painted gradient90degree/stops0..1/fromalpha>0/toalpha0/theme/selected distinction; actual PNG->worker->painted same-icon equality across3kinds and changed-icon hue; neutral type-independent fallback. Worker transparent/white-background/shaded-logo/gray/empty fixtures, trusted-root/byte/dimension/corruption rejection,128cache/reuse/root invalidation,<=4batch/noquerygenerationchange; panel late reply/pruned keys. Original typedPasteText/fulltail/target, emptygap no action, hover/focus/management/longMarkdown reflow. Relevant/fullDesktop/workspace/scopedstrict/fmt/source/spec gates; report unrelated blockers and visual/native limits separately.
### 7. Wrong vs Correct
Wrong: extra padding inside one colored hit region, or Files -> yellow regardless of app icon. Correct: actual source-icon pixels -> bounded cached RGB -> localized typed gradient inside real external gap/official boundary -> unchanged interaction owner.

## Scenario: Coherent GPUI Kit release upgrade with narrow carried patches
### 1. Scope / Trigger
Explicit user authorization upgrades the coherent stack to Kit0.10.0/defaultpre0.3.12; historical pins are superseded, not product layout/native ownership. Evidence: `repo:Cargo.toml`, `repo:Cargo.lock`, `repo:tests/architecture/gpui-release-contracts.py`, `repo:docs/reference-code/gpui-kit-0100.md`.
### 2. Signatures
`gpui-kit = "=0.10.0"`; `[patch.crates-io]` selects `vendor/gpui-component-0100`, `vendor/gpui-base-0100`, `vendor/gpui-pre-0312` and `vendor/gpui-pre-windows-0312`. `Notification::duration(Duration)` forwards to official `ToastOptions.timeout`; optional `header_action` remains in notification.rs. `WM_MOUSEACTIVATE` reads live `GWL_EXSTYLE`/`WS_EX_NOACTIVATE`. Base `LeafGeometry::table_cell_bounds()` exposes actual column slack/dividers and row frames; `table_cell_content_bounds()` resolves content boxes from final Div padding/border. Text position/index already reflect alignment. editor_host::parse_markdown uses NodeContext preserve_editor_whitespace=true. Gate CLI: `python tests/architecture/gpui-release-contracts.py [--upstream <extracted-root>] [--active-tree <clean-cargo-tree-log>]`.
Released Input frame query: `TestWindowExt::find(("input", state.entity_id()))` or Base `test_support::find(window, scope, &id)`; component-ID resolution keeps frame geometry and inherits native editor semantics. `snapshots(window)` enumerates raw native registrations and is not an interchangeable frame query.
### 3. Contracts
Kit/Base/Component/Assets/macros0.10.0 and default pre stack0.3.12 stay coherent. Fresh published sources retain license and original per-file provenance. The approved Component/Base/core/Windows union is1/5/2/4: Component notification.rs; Base src/text/editor_host.rs, src/text/editor_seam.rs, src/text/format/markdown.rs, src/text/node.rs, src/text/state.rs in that order; core scene.rs/style.rs; Windows directx_renderer.rs/events.rs/shaders.hlsl/window.rs. state.rs adds only the mechanical preserve_editor_whitespace:false field to its complete NodeContext initializer. Only editor-host whitespace fidelity and real table grid/content geometry are added in Base; ordinary TextView retains upstream0.10.0 behavior. The retained material amendment preserves UnderBorder, source-over alpha and actual client/frame/restored-cache consistency. No registry edits, old-vendor rewrite, Fast backend or editor replacement. Fixed editor revision/model and all original fixtures/assertions remain. Exact reversible frame/element adaptations and prior passive/lowbit/serializer/API exceptions are enumerated in [editor provenance](../../../../docs/reference-code/markdown-editor.md). Optional lock entries/downloads are not activation; verify clean enabled Windows tree. Primary HWND activates, passive HWND uses live noactivate; deferred callbacks, full payload, bounded layout and diagnostic correlation survive.
Medium Input retains its official32DIP full-width frame and original InputState. A source Textarea with an explicit viewport height uses its retained non-auto-growing TextareaState and internal scrolling; native editor and official frame both stay above Save/Cancel. Default Sheet ingress150ms and Dialog SlideDown250ms use the test clock; business/geometry fixtures render the mounted overlay, advance250ms, then draw its settled frame without changing production motion.
### 4. Validation & Error Matrix
Mixed version/wrong patch path/unapproved vendor file or extra -> source gate fails. Enabled Fast -> active-tree gate fails. Disabled semantics become Some(true), Settings dropdown label becomes `title: selectedlabel` -> update exact semantic fixture expectations, retain actual no-dispatch/persistence assertions. Missing native/physical evidence -> report unperformed, never infer from fixture success.
Raw Input role/path-descendant lookup returns its inner editor -> select the component ID for frame assertions, not a padding correction. Native textarea exceeds a fixed frame -> remove the conflicting auto-growth configuration, not only outer clipping. Overlay target lies outside its viewport before its virtual transition finishes -> establish the settled frame before pointer/paint assertions, never relax reachability.
### 5. Good/Base/Bad Cases
Good: verify independent published provenance, carry the narrow approved1/5/2/4 union, preserve existing owners and run focused plus full compiled regressions. Base: unconfigured notification remains5s; ordinary Windows activation and TextView behavior remain unchanged. Bad: transplant old vendor wholesale, disable semantics to preserve an obsolete snapshot, treat downloaded Fast as enabled, or rebrand historical79-page evidence as a current audit.
### 6. Tests Required
Release provenance/active-tree gate; catalog historicalJSON/table/viewer equality and currentauthority; spec navigation. Production Clipboard no-focus Ctrl-digit/type-anywhere/IME bridge/preview cancellation, text210/image190/file62/gap8, passive staged ACK/frame/generation; Notification expiry/settings/no-reset; menus and Settings semantic interactions. Strict Desktop test Clippy, workspace --tests, full Desktop suite, affected lifecycle gates and scoped rustfmt. Every direct Cargo command inherits CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256. Installed/native/physical WindowsIME/DPI/reader/user visual acceptance stays separate.
Retain the four recorded editor semantic failures as regression evidence and run their original assertions unchanged: whitespace round-trip plus aligned-cell clicks, uneven/wrapped table grips/hits and resize/scroll geometry. Source/provenance success does not close compiled behavior failures.
Editor frame.rs uses actual grid boxes, content-box horizontal bounds and text-box vertical bounds without row_shift; element.rs removes only shape-aligns and padding/right inference arguments from that call. Six readable exact upstream/compatible fragments in tests/architecture/markdown-editor-frame-0100.json are individually reversed to the original frame.rs digest; the element call reverses to the original passive baseline. Isolated upgrade editor/Base outcomes remain dated evidence, not main-workspace compiled acceptance.
`custom_command_input_has_one_full_width_kit_frame` retains exact initial/repaint geometry and entity identity. `queue07_category_input_has_no_duplicate_inset_frame` asserts the resolved frame's role/label/slot containment and exact width. `queue07_long_text_and_resize_keep_form_footer_and_cancel_identity` checks native box/footer containment at three sizes, real Ctrl+End/end-line input/internal scrolling, original entity, cancel readback and restored search focus. Extensions pointer/keyboard and Light/Dark outlined-action assertions retain original events and minimum viewport. Evidence: `repo:rust/apps/nxtools-desktop/src/ui/kit_migration_tests.rs`, `repo:rust/apps/nxtools-desktop/src/main_window/extensions_settings.rs`, `repo:rust/apps/nxtools-desktop/src/main_window/extensions_page/trust_confirmation.rs`.
### 7. Wrong vs Correct
Wrong: `disabled() == None` and a value-only label remain mandatory after upstream exposes richer semantics; weaken pointer/persistence tests or patch upstream back. Correct: assert `Some(true)` and the full `title: value` accessible name, preserving the original disabled click/no-dispatch and actual settings round-trip checks.
Wrong: treat a native text-box snapshot as a styled frame, hide an oversized auto-grown editor with clipping, or disable a failing pointer/paint assertion. Correct: query the declared frame identity, give the original source editor one bounded scrolling viewport, and advance the official virtual transition before measuring its settled geometry.

## Archive password fixture compatibility (2026-10-25)
Evidence: `repo:rust/apps/nxtools-desktop/src/archive/application_tests.rs`, `repo:rust/apps/nxtools-desktop/src/archive/browser.rs#install`.
Use actual read-batch nullable encrypted metadata, never the synthetic archive filename, to establish the current unlock entry. production_password_unlock_visibility_uses_actual_read_encryption_metadata traverses original Accepted/Negotiated/Batch/credit/Terminal/deferred install for false/unknown/true; onlytrue exposes the actual named official Button and none sends authorization/OLE merely on render. Password Dialog/ticket/deferred-grant tests must seed true and keep real pointer/private reply/current identity/no-auto-start assertions. Original plain helper stays false. Current approved icon-over-label action is official Medium outline with64DIP public height adaptation; historical32DIP leaf assertion does not override the later approved browser UI. Never make production visibility unconditional or drop lifecycle assertions to repair a stale fixture. Test/native/user acceptance remain separate.

## Launcher Indexer query retirement

Evidence: `repo:rust/apps/nxtools-desktop/src/quick_launch.rs#cancel_index_search`,
`repo:rust/apps/nxtools-desktop/src/quick_launch/panel.rs#close`,
`repo:rust/apps/nxtools-desktop/src/runtime.rs#IndexerBridge`,
`repo:rust/apps/nxtools-desktop/src/quick_launch/tests.rs`.

Store the original Search envelope ID and generation before enqueue. Empty input begins a new
local query generation and sends an exact cancellation; hiding invalidates the generation before
native hide dispatch. A query that does not request Indexer or whose new Search cannot enqueue
also retires its previous provider work. Ordinary admitted Search already supersedes the previous
job, so do not send a redundant cancel for every replacement. Matching terminal/failure and
disconnect clear pending identity; stale batches cannot clear a newer request.

Cancel waits asynchronously for the original bounded IndexerBridge queue, never blocks GPUI or
silently drops on Full. Capture the retired request identity before waiting; late delivery cannot
target the current query. The original deferred panel close checks panel identity, closed state
and query-dispatch generation before application hide, so reopening cannot cancel the new session.
Retain original InputState/IME, layout, native placement/focus and extension-provider owners.
Diagnostics carry request correlation, generation, fixed reason, queued state and elapsed time only.
The search_retirement_ TestApp cases cover clear/hide wire frames, full-queue delivery and old-close
reopen isolation; service retirement and protocol contracts are owned by Indexer/Protocol below.
No installed-service or physical-input acceptance follows from these isolated tests.

## Indexer event-driven command dispatch

Evidence: `repo:rust/apps/nxtools-desktop/src/runtime.rs#run_indexer_service_at`,
`repo:rust/apps/nxtools-desktop/src/runtime/indexer_dispatch_tests.rs`,
`repo:docs/reference-code/quick-launch-research.md#fixed-export-normal-query-dispatch-and-completion-2026-11-17`,
`repo:docs/audits/2026-11-17-indexer-dispatch-wakeup.md`.
The existing supervisor thread owns one current-thread Tokio runtime. Await the original bounded
command receiver alongside stop and reader completion; never gate command delivery behind a periodic
stop-channel timeout or replace it with short-interval polling. Stop and reader completion precede
commands when simultaneously ready. Reader sender closure signals completion on return/unwind.
Preserve FIFO envelopes, exact cancellation correlation/generation, disconnected admission and stale
queue draining. Idle EOF wakes reconnect; the existing one-second backoff remains stop-interruptible.
Stopping detaches the process-lifetime synchronous reader; reconnect joins its completed predecessor.
Transport scheduling is independent of the normal launcher's completion-owned publication;
follow [current result availability](ui-experience-contracts.md#scenario-current-quick-launch-result-availability). Reuse existing correlated search/cancel lifecycle diagnostics; do not
log query payloads or add per-poll/per-frame transport telemetry. Runtime/reader-start failures log
fixed transport events and native error codes at their owning boundary.
Required real random-named-pipe regressions: search/cancel dispatch under50ms without a following
command, FIFO request identity, partial/terminal receive while idle, EOF/reconnect without a query,
explicit stop, stop-sender closure and stop during retry. Retain bounded admission, persistent-reader
shutdown and Quick Launch generation/publication regressions. Compiled fixture latency is not
installed UI performance; package/protected-tuple deployment and A/B need separate authorization.

## Quick Launch foreground activation terminal state

Evidence: `repo:rust/apps/nxtools-desktop/src/quick_launch/actions.rs`,
`repo:rust/apps/nxtools-desktop/src/quick_launch.rs#handle_window_activation_result`,
`repo:rust/apps/nxtools-desktop/src/quick_launch/tests.rs`.

The bounded sequence remains PreTopmost -> Activate -> ConfirmTopmost. ConfirmTopmost is reachable
only after a successful Activate result; exhausting Activate retries is a real terminal failure,
not permission to treat a later topmost success as activation. Search focus is granted only by a
successful current-session Activate result. The existing Panel notification path reports a retryable
activation failure and keeps the popup searchable by click; no new controller, window or native API.
Pending-map regressions must deliver real command results through QuickLaunchApplication, assert no
post-failure ConfirmTopmost command or false success/focus, retain the successful chain, and prove an
old result cannot consume the replacement activation. TestApp evidence does not certify the physical
double-Control gesture, elevated foreground policy or installed Windows acceptance.

## Extension executor exit ownership
Evidence: `repo:rust/apps/nxtools-desktop/src/extensions/service.rs`, `repo:rust/apps/nxtools-desktop/src/extensions/service_storage_tests.rs`, `repo:rust/apps/nxtools-desktop/src/main.rs`, `repo:rust/apps/nxtools-desktop/src/runtime.rs`.
ExecutionLease retains the original executor JoinHandle and one shared terminal result. ExtensionService::shutdown_and_wait aborts event subscriptions, signals once and joins even when resolver/service clones remain. The original executor drives ExtensionRuntime::shutdown_and_wait while Tokio still runs, so cancelled feature futures and actual Blob IO can release their backing before physical cleanup joins. Final Drop only signals: it may run inside an executor task and must not self-join. The existing native grace/executor timeout never substitutes for Blob deletion completion.
GUI joins after Application::run returns, outside GPUI callbacks, before Clipboard worker shutdown; both shutdown operations are attempted even if one reports failure. Headless host joins during original idle drain before marking stopped/returning. No component/layout/focus/window lifecycle changes. Synthetic GUI/headless service tests retain clones, observe suspended task destruction and immediate real storage-owner release after explicit join; keep the final-Drop compatibility regression. Do not run script/native fixtures merely to verify this owner seam.
Source wiring gate: `repo:tests/architecture/extension-blob-lifecycle.py` rejects missing GUI post-GPUI join, skipped Clipboard flush, missing headless idle-drain join and draining outside the original live executor.

## Floating assistant Drop release ownership
`tools/floating_assistant/application.rs` captures the original PlatformBridge connection before Prepare. Release, native lifecycle and destructor use that connection under the router lock, not later is_connected/current identity. One pending release/retire survives full bounded command admission in the original connection; its consumer progresses it independently of a live GPUI controller. Retire covers same-session release and waits for original native result/EOF, not generic5second UI/10second IPC timeout. Repeated Retry preserves that pending identity. Only its real terminal advances session/generation and prepares a replacement.

`actions.rs::PendingAction` captures session AND generation. Cancellation, native completion and deadline release only that still-current identity; old action completion never rebuilds ReleaseDrop from a new session. Deferred/copy cancellation and normalized-image ownership remain unchanged. C# owns physical image staging, retains its sole process-wide16MiB Drop slot through deletion failure/Stop, and retries on the original STA. Logical release success is a transfer of cleanup responsibility, not physical completion.

Required production regressions in `repo:rust/apps/nxtools-desktop/src/tools/floating_assistant/application/tests.rs`: real bounded queue full -> exact release after recovery, cancellation, retry/destructor without controller, replacement pipe before UI sees EOF, stale action generation and60second late Retire result -> next-generation Prepare. IPC and self-owned C# locked-file fixtures protect the receiving/deletion boundaries. Preserve hidden-frame/passive focus/routing/component composition; no real Shell or Desktop product launch is implied by TestApp evidence.

## Environment-rooted link detection

`repo:rust/apps/nxtools-desktop/src/tools/link_detector.rs#looks_like_local_path` is the
shared drive/UNC/environment-root predicate for whole-line, wrapped, prose and typed
classification boundaries. Percent-delimited environment roots must resolve to an
absolute existing path before local file/directory classification. Keep original display
text and resolved Copy/Open targets; do not expand HTTP(S) percent escapes. Undefined
variables or missing whole paths must not produce a match or an existing-prefix fallback.
Regression tests exercise the production detector with the LOCALAPPDATA logs example,
actual TEMP files/directories, spaces/quotes/Markdown, case-insensitive variable names,
undefined variables and URL preservation. See [reference](../../../../docs/reference-code/link-detector.md).

## Scenario: Blacklist process window selection

### 1. Scope / Trigger
Original global Application Blacklist editor gains a window picker beside manual input. Rust owns its draft and session; GPUI draws one passive highlight; C# owns HWND facts, native styles and input. Evidence: `repo:rust/apps/nxtools-desktop/src/network_settings/window_picker.rs`, `repo:csharp/NxTools.PlatformHost/Windows/ProcessWindowPickController.cs`.

### 2. Signatures
`PlatformCommand::ProcessWindowPick(ProcessWindowPickCommand)` uses `process_window_pick`; `PlatformEvent::ProcessWindowPickChanged` uses `process_window_pick_changed`. Actions are `start {session_id,source_window_handle,overlay_window_handle}`, `show_overlay {session_id}`, `cancel {session_id}`. UUID session is operation correlation; command IDs remain individual receipts.

### 3. Contracts
Events carry `session_id`, `state: hover|selected|cancelled|failed`, optional `viewport {left:i32,top:i32,width:u32,height:u32}`, `candidate {window_handle:i64,process_id:u32,process_name,left,top,width,height}` and `native_error_code:i32`. Hover requires viewport; absent candidate clears highlight. Selected requires candidate. Coordinates are native device pixels including negative origins; GPUI subtracts viewport origin and divides by actual overlay scale. Outer protocol1 and original result envelopes remain.

Register pending session before Start; bind original weak view, source window and bridge connection. Source/overlay HWNDs must be distinct and belong to the connected Desktop. Overlay starts hidden, becomes transparent/topmost/click-through/noactivate with exact virtual-desktop bounds, and is shown only by session-qualified Show after a GPUI frame. After overlay preparation/hook admission, native HideSourceWindows(source,overlay,peer) captures and hides currently visible top-level windows of that exact Desktop process, excluding overlay/foreign/child/already-hidden HWNDs. Retain the source process handle; intentional hiding must not fail liveness. RestoreSourceWindows runs on every terminal/rollback before publication, revalidates surviving same-process HWNDs and uses visibility-only noactivate/noorder/noresize/nomove flags. Original normal/minimized/maximized state is retained. Continue other restores if one fails and report its native code. Update source editor/focus on a later application turn; selection fills process input only, retains scopes/rules and never writes config. Editor has only Input, picker and Add/Update; scope Checkboxes remain on rule rows. Row scope changes synchronize the selected edit's copied flags; original new-rule defaults and Apply/Cancel remain authoritative. Initial source gestures pass through; target confirming down/up and Escape down/up are consumed as pairs. Hook callbacks do not resolve processes, log or publish IPC. Coalesce hover facts. Native120s and Rust125s deadlines retire work; cancel/page leave/close/disconnect/owner loss prevent late writes or show.

### 4. Validation & Error Matrix
- Missing native handles/disconnected bridge/overlay creation or Start failure -> release local session, retain manual value and show retryable error.
- Wrong session/connection/source/view -> ignore or retire; no editor mutation or activation.
- No eligible hover target -> empty highlight; source/overlay/desktop/invisible/cloaked windows are excluded.
- Process lookup/Show failure -> terminal error; no fabricated process name.
- Cancel/close/page leave -> retain manual value; release native hooks after intercepted releases drain.
- Partial hide/start failure or terminal -> attempt restore of every captured survivor before acknowledgement/publication; restore rejection -> failed outcome/native code, not false selection success.
- Diagnostic events carry session/state/fixed reason/elapsed/native code only, never names, paths, titles or hover streams.

### 5. Good/Base/Bad Cases
Good: hover a window on a negative-origin monitor, left-click, then explicitly Add and Apply. Base: manual process entry remains available without PlatformHost. Bad: an old selection adds a rule automatically, changes scopes, writes config or resurrects a closed highlight.

### 6. Tests Required
`network_settings::window_picker_tests` asserts actual official Button pointer/keyboard flow, manual/editor bounds, deferred local refill, unchanged rules/scopes/config, old-session and connection rejection, cancellation/page/close cleanup and scaled negative origins. Protocol roundtrip uses real command/event envelopes and rejects wrong action shape. Injected C# fixtures assert actual parser/controller/native API seams, hidden placement/ownership, fresh clicked HWND, matching-release suppression, cancel-before-Show, startup failure and disposal. Real HWND/input/mixed-DPI/user acceptance remains separate.

### 7. Wrong vs Correct
Wrong: picker callback synchronously updates the leased settings view and persists a new blacklist rule. Correct: retire the qualified session, defer the original source-window/view update, fill InputState and leave Add/Apply to the user.

Picker rendering intersects native target bounds with the virtual desktop before scale conversion;
maximized/off-edge native borders must not put the outline or caption outside that surface.
The caption retains original native dimensions and no target HWND geometry is mutated.
Layered overlay preparation calls checked SetLayeredWindowAttributes(alpha255,LWA_ALPHA) while
hidden, after style assignment and before exact outer/client placement; failure releases admission
without installing hooks or revealing the window.

## Scenario: Modeless close-only About
### 1. Scope / Trigger
About caption actions and unnecessary parent-window state changes. Evidence: `repo:rust/apps/nxtools-desktop/src/main_window/about.rs`, `repo:rust/apps/nxtools-desktop/src/ui/components/titlebar.rs`, `repo:vendor/gpui-pre-windows-039/src/window.rs`, `repo:vendor/gpui-pre-windows-039/src/events.rs`.
### 2. Signatures
`about_window_options(&App) -> WindowOptions` is consumed by the original open_about call. It returns Normal, fixed420x480, nonresizable/nonminimizable, original client caption options. `About::close_window(&mut Window)` retires update ownership then calls original GPUI remove_window. Existing native should-close retains retirement.
### 3. Contracts
Read-only About is modeless; framework Dialog is reserved for a true parent-blocking decision. Locked WindowsWindow::new disables the active parent only for Dialog and handle_destroy_msg re-enables/force-activates that parent. About must not enter that path. Reuse one34DIP caption_action official Button/Small icon beside a native drag-only region in the transparent caption slot; no native Close hit region, Minimize or Maximize. Keep dimensions, Root, copy/license/update and repeat-open owners. Do not suppress all refreshes/animations or edit vendor code to hide an unmeasured flash. Diagnostics keep original About correlation/close lifetime and bounded caption dispatch events, not frame logging.
### 4. Validation & Error Matrix
Stale open handle -> original new-window path; repeated valid open -> original activation reuse. Client Close -> cancel pending update before one removal, no late download. Native Close -> existing retirement callback. Other modal dialogs remain unchanged. Native frame stability is not established by these source/controlled assertions.
### 5. Good/Base/Bad Cases
Good: About leaves the primary window enabled while open and avoids framework parent-enable/forced-activation teardown. Base: transparent compact close-only caption and existing footer remain. Bad: select Dialog merely to obtain a small auxiliary window, keep ignored min/max buttons, or add a second close dispatcher.
### 6. Tests Required
Production About pointer Close asserts one WindowClosed and retired update, caption snapshots assert only Close, and the constructor-policy test asserts Normal/fixed capabilities. `python tests/architecture/about-window-policy.py` binds the actual factory/call site to the inspected native modal branch. Existing About layout/update, settings transactions and caption tests remain. Shared settled entrance must not replay after ordinary refresh; use the released animation's actual clock. Real parent-window frame capture and physical/DPI interaction remain separate acceptance.
### 7. Wrong vs Correct
Wrong: assume is_minimizable/is_resizable options remove private TitleBar controls or that a caption change proves all UI flashing resolved. Correct: use the existing client close component and explicit modeless creation policy, protect the actual lifecycle with regressions, and report native/visual evidence separately.

## Scenario: Stable Clipboard header during feature apply
### 1. Scope / Trigger
Main Clipboard enable Switch, background statistics refresh and native configuration receipts.
### 2. Signatures
ClipboardOverview.has_snapshot/metadata; MainWindow.request_clipboard_overview; original ClipboardApplication.set_feature_enabled/feature_configuration_status. Evidence: `repo:rust/apps/nxtools-desktop/src/main_window/clipboard_page.rs` and its kit_migration_tests.rs.
### 3. Contracts
Only successful counts establish a snapshot, including valid zero. Keep last-good numeric metadata while refreshing; initial load/unavailable remains explicit. A real failure retains counts with failure copy and is not cleared until success. Missing worker does not fabricate zero. Preserve request generation/coalescing and original worker/subscription/feature/pause owners. Status uses a stable elastic lane, not label-dependent width; pending native apply and confirmed/error values remain distinct. No blanket motion disable or per-frame logging.
### 4. Validation & Error Matrix
Loaded+loading -> retain counts. Initial failure -> unavailable, not zero. Loaded failure -> last counts plus visible failure. Pending ACK -> original pending status, no false success. Retired generation -> no stale replacement. Normal enable/disable does not change history.
### 5. Good/Base/Bad Cases
Good: toggle with held counts keeps header/status layout and correct native states. Base: new history changes update counts through the original worker. Bad: every application notify replaces valid counts with loading copy or resets status layout/entrance animation.
### 6. Tests Required
Actual Root/MainWindow/official Switch with temporary worker and fake configuration ACK; test-only bounded worker barrier inspects pending/terminal/settled frames, normal/reduced motion, valid empty snapshot, entity identity, one command and closed-popup no-presentation changes. Retain keyboard/persistence/motion and snapshot error tests. Frame bounds/model text are separate from native glyph-opacity/Win10/DPI/physical/user acceptance.
### 7. Wrong vs Correct
Wrong: overview.loading -> replace all numbers with a loading label after every feature-state notification.
Correct: successful snapshot presence -> numeric projection during refresh; stable status lane -> original pending/applied/error semantics.
