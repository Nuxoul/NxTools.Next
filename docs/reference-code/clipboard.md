# Clipboard reference implementation index

Last verified: 2026-07-18.

Additional references inspected2026-10-19: [ClipKeep and TieZ source index](clipboard-clipkeep-tiez.md), with [current comparison and proposed candidates](../audits/2026-10-19-clipboard-reference-comparison.md). These supplement legacy behavior; no UI/feature/lifecycle change or phase reorder is approved by the research.

## Behavioral contract

- The clipboard is a shortcut-opened popup, not a page in the main window.
- Capture preserves text, image, file-drop and useful native formats before selecting a primary
  history kind.
- Original paste restores the richest compatible representation; plain paste maps images and files
  to paths.
- The popup targets the window that was active before it opened. The old NxTools popup hides before
  target-focus restoration and paste input; NxTools.Next must keep the popup available when the
  confirmed Windows action fails so the user can retry.
- Pure images must expose PNG/DIB for image-aware applications. `CF_HDROP` is added only for
  allowlisted file/terminal targets that need a path fallback; rich editors such as Zed must not
  receive the same image a second time as a file attachment.

## Old NxTools: behavioral source of truth

Root: `D:\Code\NxTools`

| File | Verified responsibility |
| --- | --- |
| `Services\ClipboardCaptureService.cs` | Observes Windows clipboard content and constructs captured history input. |
| `Services\ClipboardNativeFormatService.cs` | Reads/writes native Windows clipboard representations and selects target-compatible image behavior. |
| `Services\ClipboardPasteService.cs` | Writes the selected item, restores the target window and performs the paste fallback sequence. |
| `Services\ClipboardPasteActionService.cs` | Converts special actions to payloads; image plain-text action returns `ImagePath`, files return their paths. |
| `Views\MainWindow.axaml.cs` | Owns `ClipboardPasteActionKind` and dispatches Ctrl/Alt/Shift modifier actions into paste, copy-only, plain-text/path and line-break flows. |
| `Services\ClipboardHistoryService.cs` | History-facing orchestration. |
| `Services\ClipboardRepository.cs` | Clipboard persistence, duplicate revival, usage ordering and retention cleanup. |
| `Services\ClipboardHotKeyParser.cs` | Parses Ctrl/Alt/Shift/Win plus a non-modifier key into Win32 modifier and virtual-key values. |
| `Services\ClipboardHotKeyService.cs` | Re-registers the configured global hotkey, applies the 350ms debounce and reports registration failure without installing a permanent low-level hook. |
| `ViewModels\MainWindowClipboardViewModel.cs` | Applies capture exclusions case-insensitively after splitting process names by semicolon, comma or line break. |
| `Models\AppState.cs` | Defaults to 1000 items, 30 cleanup days and password-manager process exclusions. |
| `Views\MainWindow.axaml` (clipboard settings) | Uses separate Ctrl/Alt/Shift selectors; ordinary click remains original-format paste. |

Verified call chain:

```text
clipboard change
  -> ClipboardCaptureService
  -> ClipboardNativeFormatService/native representations
  -> ClipboardHistoryService + ClipboardRepository

panel action
  -> `Views\MainWindow.axaml.cs` / `ClipboardPasteActionKind`
  -> `ClipboardPasteActionService` (original or path/text transformation)
  -> `ClipboardPasteService`
  -> `ClipboardNativeFormatService` + target activation/input

```
## Verified popup and hotkey lifecycle

Reconciled against the actual old NxTools sources:

- `Views\ClipboardHistoryPopupWindow.axaml.cs:127-167,206-260` creates one reusable popup,
  focuses the search box on show, hides rather than destroys it, and has no idle auto-hide timer.
  `Deactivated` hides after the 180ms opening suppression unless a context menu is open.
- `Views\ClipboardHistoryPopupWindow.axaml.cs:430-507` runs a separate 45ms outside-pointer monitor;
  a primary/secondary press outside the popup hides it, while preview exit only affects the preview.
- `Views\ClipboardPreviewWindow.cs` is a passive auxiliary window; its lifetime is independent from
  the main popup. `Services\ClipboardPasteService.cs` receives a `beforeRestore` callback so the
  popup is hidden before target focus restoration and paste input.
- `Services\ClipboardHotKeyService.cs:25,111-123` uses `RegisterHotKey`, applies a 350ms trigger
  debounce, and installs `WH_KEYBOARD_LL` only during shortcut recording. A registration failure
  does not become a permanent low-level hook fallback.
- The old action mapping is `ClipboardPasteActionKind` in `Views\MainWindow.axaml.cs`, not a
  standalone `Services\ClipboardActivationResolver.cs`; that file is absent from the reference tree.
  The note does not claim a verified multiple-modifier rejection rule.

NxTools.Next keeps these boundaries in Rust/C# form: `PopupLifecycle` owns the current reusable
popup, `PreviewSession` owns one independent preview, C# owns native hotkeys and Windows focus APIs,
and worker-confirmed actions update usage only after success. The current C# hotkey boundary also
- The old ViewModel marks ordinary paste usage before the Windows paste result is known. NxTools.Next
  deliberately does not copy that failure-prone ordering: usage is persisted only after a successful
  platform result, and failed actions must not reorder history.
uses a tested 350ms debouncer in `ClipboardHotKeyDebouncer.cs`.
The old NxTools `ClipboardHistoryService.CleanupOrphanedImageFiles` scans only active recent rows
after soft deletion; that can delete files still reachable through a recoverable deleted row. NxTools.Next
must keep the stronger remaining-database-reference check instead of copying that cleanup shortcut.


Verified retention rules in `ClipboardRepository.Cleanup`:

- age and overflow cleanup soft-delete only unpinned, non-favorite items;
- maximum item count is clamped to 100–100000;
- cleanup days use a minimum of one day;
- deleted rows are permanently purged only after `max(7, cleanupDays)` days; the old default of 30
  days makes the common case 30, but 30 is not the hard minimum.

Verified history identity and ordering rules:

- `ClipboardRepository.Save` resolves duplicates by the complete content hash across the whole
  active/deleted history, not only against the immediately preceding row; recopying content revives
  and moves the existing record instead of creating another visible copy;
- `ClipboardRepository.MarkUsed` updates `last_used_at`, increments `use_count` and moves an
  activated entry to the front of its non-pinned order;
- repository recent ordering uses pinned state, `order_value`, last-use and capture recency;
  the popup query itself matches visible text, file paths and image paths, while source metadata is
  available through the broader recent/search paths rather than every popup query.

NxTools.Next must preserve distinct native representations of equal visible text, so its global
duplicate match is the typed payload plus the complete native-format set. A successful paste or
copy-only command marks the item used only after Windows confirms the action; failed commands must
not change ordering.

Managed clipboard assets are content-addressed and may be shared by multiple rows. Retention may
delete a file only after the database transaction confirms that no remaining image, source-icon or
native-format row references it. The separate direct-paste fallback directory also needs bounded
age cleanup while protecting the path just placed on the Windows clipboard.

Verified settings defaults: `Alt+C`, 1000 items, 30 days, and `KeePass.exe`,
`1Password.exe`, `Bitwarden.exe` excluded from capture.

Reverified on 2026-09-18: clipboard capture limits are user-configurable in `ClipboardSettings` and are carried through `ConfigureClipboardCommand` to `WindowsPlatformAdapter.EventWindow`. Text is bounded by UTF-8 byte size; images are bounded by encoded output bytes and pixel count. Over-limit content is skipped without truncating or logging payload data, while independent file/native representations may remain capturable. Source process icons use a content-addressed staging cache with a bounded in-memory process index.

Verified modifier-action contract:

- Ctrl, Alt and Shift remain the only configurable modifier slots;
- each slot can choose original paste, copy only, plain-text/path paste, line-break removal or disabled;
- multiple modifiers are unsupported;
- line-break removal remains text-only at execution time.
- Modifier actions that rewrite the Windows clipboard arm a Rust-side capture suppression window
  before the platform command is sent. PlatformHost already suppresses the first native clipboard
  notification; Rust also drops the next matching-time capture even when the target application
  exposes mixed formats (for example image plus path text), then removes its staging files. This
  prevents Alt/Shift/Ctrl transformed output from creating a second history row without assuming
  that the target returns only one clipboard format.
- The category editor is an in-panel modal surface, not a second native window. Its root consumes
  left-button down and click propagation before history rows can receive the event; focus remains
  owned by the category-name input until save or cancel.

The persisted Rust hotkey model uses semantic modifiers plus a key label. The C# adapter maps those
values to `MOD_*` and virtual-key constants immediately before `RegisterHotKey`; legacy development
configs containing numeric Win32 values are migrated during Rust config deserialization.

## Main-window visual references

Clean-room source inspection for this settings pass:

- Zed `crates/theme/src/default_colors.rs` separates dark `background`, `surface_background`,
  `element_background`, hover/active states and focused borders; accent is not used as a large
  selected-navigation fill.
- AB Download Manager `DefaultThemes.kt` uses `#1E1F22` background, `#2A2B2F` surface,
  `#D6D6D6`/`#E0E0E0` foregrounds and blue `#4791BF` as the primary action color.
- ABDM `MyColors.kt` derives borders from `onSurface` at low opacity instead of drawing strong
  outlines around every settings group.

NxTools.Next therefore uses a graphite background/surface ladder, blue focus/action accents,
low-contrast separators, and one continuous settings surface rather than symmetric bordered cards.

Reverified on 2026-08-24 before the CL-01 main-workspace settings migration:

- Old NxTools `Views/MainWindow.axaml:1458-1564` keeps calling/hotkey and modifier-click actions
  separate from storage and retention policy. It confirms ownership and behavior, but its stacked
  card layout, Avalonia measurements and turquoise palette are not copied into NxTools.Next.
- NxTools.Next already placed those responsibilities in one two-column surface: the flexible
  `呼出与快捷操作` column owns the hotkey plus Ctrl/Alt/Shift actions, while the fixed
  `保留与隐私` column owns item/day limits, exclusions, archives, database maintenance and storage.
- CL-01 extracts only that outer grammar into the shared settings component: one square boundary,
  12 px column insets, one 1 px seam and the existing 410 px secondary rail. It does not change row
  density, labels, controls, persistence, picker flows, hotkey registration or clipboard lifecycle.

## Clipboard setting descriptions

Verified before the compact settings-row migration:

- Old NxTools `Views/MainWindow.axaml:1460-1495,1524-1538` groups clipboard calling/hotkey,
  storage and retention controls with concise section-level help; these controls do not use a
  fixed-size per-row tooltip.
- NxTools.Next `main_window/clipboard_page.rs:809-893` supplies explicit descriptions to the shared
  `main_window/quick_launch_page.rs::compact_setting_row`; that row is also used by Quick Launch.
  The current implementation truncates descriptions in-row and builds a fixed 360×170 tooltip.
- Preserve the existing setting text and control behavior, but keep descriptions readable across
  the row and size the hover tooltip to its text, bounded by a maximum width rather than a fixed
  width/height.

## Link detector floating panel

Verified current NxTools.Next call chain for the link-detector tool settings and panel placement:

- `rust/apps/nxtools-desktop/src/clipboard.rs::handle_link_detector_capture` extracts detected
  links, while `show_link_panel` owns one reusable GPUI popup and crosses the native
  `SetWindowPosition`/`SetWindowBounds` boundary.
- `rust/apps/nxtools-desktop/src/tools/link_detector.rs` owns link extraction and
  `rust/apps/nxtools-desktop/src/tools/link_panel.rs` owns the panel content, expansion and
  auto-hide lifecycle. Neither module owns persisted settings.
- `ClipboardSettings::link_panel_position` is persisted with `center` as the compatibility default;
  `ClipboardApplication` is the sole writer through `set_link_panel_position`, and the dedicated
  `ToolSettingsWindowBody` defers that cross-entity write after a dropdown selection.
- Position math is Rust-owned and uses the selected display bounds plus the fixed panel size:
  top-left and top-right use a 24 px inset, center uses both display axes, and all results are
  clamped to the selected display. Reusing the panel reapplies both position and size after every
  capture and keeps expansion growing downward from the selected top edge.
- Exhaustive inspection of the old NxTools tree found no equivalent link-detector popup or persisted
  three-position setting. This is a new NxTools.Next preference, not a legacy behavior migration.

Verified 2026-10-02 local-target link-panel route contract:

- `DetectedLinkKind` carries `Web`, `LocalFile` or `LocalDirectory`; detector acceptance remains limited to known Web targets and existing drive/UNC paths, with file-vs-directory determined by the filesystem.
- `link_panel_route` honors the persisted global copy-only preference first (default `true`), so no kind launches while copy-only is enabled. When copy-only is disabled, local files map to `OpenShellTarget(SelectInExplorer)`, local directories map to `OpenShellTarget(Open)`, and only Web targets consult the opt-in browser-auto-open setting.
- `link_panel_shell_command` is shared by production dispatch and tests. It preserves the original local target and rejects a Web kind on the shell branch; local targets are never synthesized into `https://` URLs.
- The row removes its panel window before recording the copied echo, dispatching the selected route, and writing the original target back to the clipboard. Diagnostics use kind/length/correlation/result only, never the path or URL payload.

## GPUI settings and popup ownership invariant

The main-window hotkey editor owns the persisted modifier-action settings. A clipboard popup receives
a value snapshot when it is created and renders only from that snapshot. `ClipboardPanel::render`
must never synchronously read or update its parent `ClipboardApplication` entity: platform-command
confirmation already leases that application, and notifying a still-alive hidden popup can otherwise
re-enter the parent and trigger GPUI `double_lease_panic`.

Verified failure chain from the 2026-07-18 Windows crash dump:

```text
configure_clipboard succeeds
  -> ClipboardApplication notification schedules hidden popup render
  -> ClipboardPanel::render reads ClipboardApplication again
  -> gpui::entity_map::double_lease_panic
  -> panic crosses the Windows callback boundary
  -> FAST_FAIL_FATAL_APP_EXIT (0xc0000409)
```

The 2026-08-08 packaged browser-takeover acceptance run exposed the same invariant at a second
boundary. `paste_native_formats` completed successfully, then `ClipboardApplication` forwarded its
command result into the still-open panel while the application entity remained leased. The panel
synchronously called `mark_item_used` through the same application entity, producing
`cannot update ClipboardApplication while it is already being updated`; the panic crossed the
Windows callback boundary and terminated the protected desktop with `0xc0000409`. The browser
handoff arrived roughly two seconds later and was accepted by the independent DownloadHost, so the
browser extension was not the crash origin. Usage persistence must be deferred to the next
application turn, and the copy-only list refresh must be deferred once more until that application
update has released its lease.

Hotkey recording still suspends `RegisterHotKey` through C# before accepting a key. Cancel/focus-out
restores the existing registration, while a confirmed replacement registers the new binding.

## Hover-preview surface invariant

The old NxTools implementation keeps one passive `ClipboardPreviewWindow`, updates its content in
place and only disposes the previous bitmap after the replacement has been assigned. NxTools.Next
keeps the same single-window lifecycle, but GPUI image decoding and DirectComposition frame
submission are asynchronous. A native window hidden through `ShowWindow(SW_HIDE)` does not produce
the GPUI frame callback needed to reveal refreshed content. Reusing a preview window must therefore
follow this order:

```text
move the native preview surface outside every display
  -> show it off-screen if it was hidden
  -> update the GPUI preview entity
  -> render one frame for the requested item (a neutral loading surface is valid)
  -> move it to the requested position only if hover generation and item are still current
```

Showing the HWND before the replacement frame is submitted exposes the window's previous swap-chain
buffer and briefly displays the previous image. A delayed show callback must also be generation
guarded so scrolling or a newer hover cannot resurrect stale content.

The clipboard panel and preview use transparent native windows, but their outer GPUI shells are
deliberately rectangular and fill the complete native client bounds. A rounded transparent root
reveals the rectangular HWND at its corners on Windows 10, so rounded styling is reserved for inner
cards and controls. Generic GPUI drop shadows are also not used on the root shells because they
render into transparent Windows surfaces as visible rectangular blocks; compact borders provide the
surface separation instead. C# owns topmost, position, visibility, move and resize Win32 calls only;
it does not own visual shape policy or apply a native window region.
Reverified popup creation on 2026-09-18: a hidden GPUI clipboard window can expose a native HWND with the monitor-sized default rectangle even when `WindowOptions.window_bounds` and the Rust open log both report `300×440`. The clipboard creation path must therefore send `SetWindowBoundsCommand` with the final popup rectangle before the topmost confirmation can reveal the HWND. The command is handled by `WindowsPlatformAdapter.SetWindowBounds`/`SetWindowPos`; this native resize is part of popup lifecycle correctness, not a visual-only adjustment.

Preview appearance motion follows the Zed/GPUI one-shot animation model. Each requested item update
increments an appearance generation, and that generation is part of the animation element id so a
reused preview can animate again without restarting during ordinary image-decoder redraws. The
animation is a short (130 ms) fade plus four-pixel settle, requests frames only while appearing and
never repeats. Hiding, steady-state rendering and pointer tracking must schedule no animation frames.

## Ditto: native format reference

Root: `D:\Code\Ditto-master`

| File | Verified responsibility |
| --- | --- |
| `src\ClipboardSaveRestore.cpp` | Captures/restores global-memory clipboard formats and explicitly retains `CF_HDROP`. |
| `src\CF_HDropAggregator.cpp` | Produces `CF_HDROP` buffers and path-as-text representations. |
| `src\OleClipSource.cpp` | Restores delayed/native formats, aggregates file drops, and can create `CF_HDROP` when required. |

Important conclusion: `CF_HDROP` is a first-class compatibility representation, not UI text. Keep
it beside the image formats so the receiving program chooses the representation it supports.

## Windows Terminal and Codex CLI compatibility finding

Verified against the locally inspected Windows Terminal source and Codex CLI source:

```text
Ctrl+V in Windows Terminal
  -> Terminal.PasteFromClipboard
  -> clipboard::read()
  -> CF_UNICODETEXT first, then CF_HDROP
  -> pure PNG/DIB produces no terminal paste
```

Codex CLI can accept an image from its clipboard image reader or from a pasted filesystem image
path. Therefore a pure-image clipboard bundle containing PNG/DIB plus `CF_HDROP` handles both paths
without adding Unicode text that would override image paste in ordinary applications.

## NxTools.Next implementation map

| Module | Responsibility |
| --- | --- |
| `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.cs` | Capture/write orchestration on the STA Windows thread. |
| `csharp/NxTools.PlatformHost/Windows/ClipboardHotKeyDebouncer.cs` | Applies the old 350ms monotonic trigger debounce to the clipboard `WM_HOTKEY` boundary. |
| `csharp/NxTools.PlatformHost/Windows/NativeClipboardFormats.cs` | Native format capture, bounded staging, restoration and image fallback injection. |
| `csharp/NxTools.PlatformHost/Windows/ClipboardFileDropPayload.cs` | Encodes Unicode `DROPFILES` memory for `CF_HDROP`. |
| `csharp/NxTools.PlatformHost/Windows/ClipboardImageDataObject.cs` | Exposes history images as PNG/Bitmap and conditionally adds FileDrop. |
| `csharp/NxTools.PlatformHost/Windows/ClipboardImageFallbackStore.cs` | Keeps a content-addressed image file alive for direct external Ctrl+V. |
| `csharp/NxTools.PlatformHost/Windows/WindowsPasteCoordinator.cs` | Clipboard write, target activation, modifier release and input diagnostics. |
| `rust/crates/nxtools-protocol/src/lib.rs` | Versioned paste command, including optional current image-path fallback. |
| `rust/apps/nxtools-desktop/src/clipboard.rs` | Chooses typed actions, supplies the persisted image path and owns popup lifecycle. |
| `rust/crates/nxtools-clipboard/src/lib.rs` | SQLite history, native format metadata and clipboard domain operations. |
| `csharp/NxTools.PlatformHost/Windows/ClipboardShellTarget.cs` | Validates typed quick-jump targets and launches registry, Explorer, URL or allowlisted Windows tools. |

Current image compatibility chain:

```text
direct copy of pure image
  -> C# captures the source PNG/DIB/native formats
  -> content-addressed fallback PNG is persisted
  -> C# publishes a canonical PNG + CF_DIBV5 + CF_DIB core and retains safe non-private native formats
  -> CF_HDROP is retained as the explicit path fallback for terminal/file-manager compatibility
  -> PixPin/OLE-private source formats are not replayed into the active clipboard

click image history item
  -> Rust classifies the captured target process
  -> rich editor: send PNG/DIB only and remove stale CF_HDROP
  -> terminal/file manager: send PNG/DIB + persisted image_path_fallback
  -> C# follows the typed include_file_drop_fallback flag
```

The canonical DIB path is important for screenshot applications such as PixPin: copying the
source's `DataObject`, `PixPinData`, or `Ole Private Data` registrations as if they were ordinary
global-memory formats can make the clipboard look populated while rich editors and CLI paste
surfaces reject it. The compatibility rewrite keeps the PNG bytes, rebuilds a top-down 32bpp
`CF_DIBV5`/`CF_DIB` pair, and preserves the bounded file-path fallback without logging payload data.
Browser right-click `Copy image` is covered by the same rewrite boundary. Chromium can publish an
image together with `HTML Format`, `text/html`, URL-list formats, or `Chromium internal source ...`
and `Chromium Web Custom MIME Data Format` registrations. Those metadata formats trigger the
canonical rewrite but are not replayed into the active clipboard; the image remains available
through PNG/DIB and the bounded `CF_HDROP` fallback.

## Quick-jump target resolution

The original NxTools open-target service only handled URLs and existing filesystem paths. The new
resolver keeps those behaviors and adds two typed, defense-in-depth actions:

```text
计算机\HKEY_LOCAL_MACHINE\SYSTEM\...
  -> Rust canonicalizes the registry root
  -> protocol action = registry_key
  -> C# validates an allowed root, sets Regedit LastKey, launches regedit.exe -m

msconfig
  -> Rust exact-line allowlist -> msconfig.exe
  -> protocol action = system_tool
  -> C# repeats the allowlist check and launches through the Windows shell
```

Arguments and arbitrary commands are intentionally rejected (`cmd /c ...`, PowerShell expressions,
or unknown executables). The card reuses the compact Material `open_in_new` action; a single target
opens directly and multiple targets produce a typed selection menu.

## Search scope and history loading

The popup search box defaults to content-only matching. Its trailing Material search-scope button
enables extended metadata matching for the current popup session:

```text
scope off -> clipboard text only
scope on  -> text + image name + file paths + source process + source window title
```

The toggle belongs to the GPUI panel state; Rust history owns both query definitions. Opening a new
panel resets to the predictable content-only default. Large histories must be fetched in bounded
pages rather than raising the popup query limit to the configured retention maximum.

## Async worker-owned history queries

Reverified for the worker ownership/query migration:

- `ClipboardHistory` is opened and retained only by `clipboard_worker`; the desktop runtime and
  GPUI entities receive a cloneable worker handle rather than a database connection.
- Popup creation waits for one worker snapshot containing the first bounded page, categories and
  custom-category sources. The hidden popup then follows the existing native-bounds, topmost and
  reveal confirmation lifecycle.
- Search and pagination replies are accepted only when generation, query, category and search scope
  still match the panel state. Stale replies are discarded without changing rows, selection or
  loading/error state.
- Worker replies are awaited from GPUI async tasks. `ClipboardApplication` and
  `ClipboardPanel::render` do not synchronously wait on SQLite; entity-to-entity UI updates are
  deferred to a later application turn.
- Loading, worker-error and genuinely empty results remain distinct presentation states; a query
  failure must not be rendered as an empty history page.


## Bounded query and history-window invariant

The Next implementation keeps the filtered total independent from the loaded payload window. The
first search/category page establishes the total; forward pages use the stable usage-order cursor
(`is_pinned`, `last_used_at_unix_ms`, `captured_at_unix_ms`, `id`) and fetch one extra row to retain
`has_more` semantics. The legacy offset path remains for compatibility and backward page loads.

The GPUI panel retains at most four history pages (320 rows) at a time. Moving forward evicts the
oldest rows; moving backward prepends a page and evicts the newest tail. `ListState::splice` owns the
logical index adjustment so the visible anchor survives each eviction. This bounds retained payload
rows without changing the database's total count or the worker ownership boundary.

## Portable archive management

Verified old NxTools behavior in `ClipboardHistoryService.ExportJson` / `ImportJson`:

- export writes a JSON list containing each item plus its saved formats;
- import reads that JSON and merges each item through the normal repository `Save`/deduplication path;
- the old project has no verified `.nxtclip` ZIP archive or `ExportArchive`/`ImportArchive` call chain.

NxTools.Next separately provides a portable archive format with managed assets and native-format files;
that is a Next behavior, not an old NxTools reference behavior.

## Database maintenance and storage migration

Verified old NxTools behavior in `ClipboardRepository.CreateBackup`, `ClipboardRepository.Vacuum`,
`ClipboardHistoryService.Configure` and the storage controls in `Views\MainWindow.axaml`:

- a live SQLite database is backed up with SQLite's online backup API, not by copying the open file;
- `VACUUM` runs outside a transaction and can require exclusive access;
- the old storage-directory setting switches repositories after creating the new database and asset
  folders, but does not move the previous history;
- storage maintenance serializes mutations and creates a timestamped backup before destructive
  legacy cleanup.

NxTools.Next deliberately strengthens the directory change into a real migration. It queues incoming
captures while the operation is active, stages an online database backup and the managed asset tree
beside the destination, rewrites managed absolute paths in the staged database, validates the result,
and only then switches the persisted directory and live history connection. Cancellation or failure
must leave both the configured source and its data unchanged. Database compaction always creates a
recoverable backup first.

## Verification and diagnostics

- C# regression suite: `csharp/NxTools.PlatformHost.Tests/Program.cs`
  - Unicode `DROPFILES` header and double-null path list;
  - pure image bundle gains PNG and `CF_HDROP`;
  - history image exposes PNG/Bitmap and conditionally includes FileDrop;
  - rich-image targets can omit the duplicate FileDrop representation;
  - protocol preserves `image_path_fallback` and `include_file_drop_fallback`.
- Rust regression suites:
  - `nxtools-protocol`: native format command round-trip;
  - `nxtools-core`: target classification keeps Zed on native-image-only paste;
  - `nxtools-desktop`: terminal targets receive the persisted fallback path while Zed does not.
- Runtime logs: `%LOCALAPPDATA%\NxTools\NxTools.Next\data\logs`.

## 2026-07-27 UI 质感收尾

当前运行证据：
`docs/audits/launcher-clipboard-ui-2026-07-27/02-clipboard-mixed-preview.png`。
图片自适应、分类、搜索、悬停动作和相邻预览已经成立；首轮视觉收尾只统一 GPUI 表面语法：

1. 保持条目高度、图片比例、面板尺寸和交互不变。
2. 默认条目边框透明，悬停只使用中性边框；图片承托面不再叠加第二层描边。
3. 时间、次数和字符/项目数量继续保留；时间使用无底色的弱化文本，次数和内容数量使用
   3 px 小圆角中性元素底色，避免一排同型胶囊。
4. 分类名与总数合并为可截断的标题摘要，搜索输入占满可用行宽，也避免状态提示出现时标题溢出。
5. 剪贴板预览使用与速启一致的中性窗口边框；不再用强调色暗示选中状态。
6. 视觉层只属于 GPUI；不得借 UI 优化修改剪贴板领域状态、持久化、Windows 适配或预览生命周期。

本轮进一步统一速启/剪贴板的共享字号、4 px 紧凑圆角、语义选中态和中性边框。历史列表渲染只
克隆 `Rc<Vec<ClipboardItem>>` 句柄，不再在每次 GPUI 重绘时深拷贝整页条目；图片和来源图标使用
96 项 LRU 资源门禁，离开热集合的路径主动从 GPUI 图片资产缓存移除。性能脚本可通过
`-ExerciseScroll -HoverFirstItem` 同时记录 CPU、工作集和私有内存；交互式基线仍需在 Release 包
上由人工运行，自动测试不启动桌面窗口。

### 第二轮：窗口安全边距

用户标注截图
`C:\Users\Administrator\AppData\Roaming\NxTools\NxTools.Next\data\clipboard-images\c79c423d8bf265099556cf998ba68ffcf10e4e5198b88d4cdc063d9abf2d23ff.png`
显示标题、分类、搜索和历史条目几乎贴住 1 px 窗口边框。旧 NxTools
`Views/ClipboardHistoryPopupWindow.axaml:94-101` 在外壳边框内使用统一 `Padding="8"` 和
`RowSpacing="6"`；这是已经验证过的紧凑桌面间距，不需要重新发明。

GPUI 迁移规则：

1. 标题、分类、搜索、历史视口统一使用 8 px 左右安全边距，区段间距统一为 6 px。
2. 顶部保留 6 px 呼吸区，历史视口底部保留 8 px；右下角原生缩放热区仍贴窗口边缘。
3. 列表和图片可用宽度必须从同一 8 px 边距常量推导，不能继续只在图片算法中假设不存在的
   6 px 边距。
4. 不改变 300 × 440 默认窗口尺寸、条目高度、预览位置、分页或输入交互。

## Open items

- Interaction still requires user verification in Windows Terminal/Codex CLI because automated tests
  do not inject into the user's live terminal.
- Portable archive import/export, exact database backup, backup-first compaction and storage-root
  migration are available from the main window. Live interaction with the Windows file and folder
  dialogs still requires user verification; popup search and history paging no longer depend on
  loading the configured retention maximum.

- Clipboard action pending state must be installed in the GPUI panel before the command can be
  enqueued: allocate one operation/command ID at action start, store `(command_id, action, item_id)`
  and its watchdog token, then pass that ID through the IPC envelope. Rust results use the envelope ID.
  This prevents a fast Windows result from arriving before the panel has a matching pending action.
- After asynchronous native-format preparation, re-check the operation token before enqueueing. A
  timeout or disconnect must clear both pending fields; a stale preparation result must not send a
  command or restore a completed pending action. PlatformHost disconnect invalidates ordinary
  pending commands by ID, not by individual `CommandResult`; the current popup therefore explicitly
  abandons its pending paste and remains available for retry.
- The caller-supplied IPC command-ID seam rejects an ID already pending on the active connection;
  command IDs remain registered only for the existing bounded IPC timeout/result lifecycle.
- Reverified in 2026-10: the panel now installs the action's operation/command ID and watchdog
  before asynchronous native-format preparation begins, and `PlatformBridge::send_command_with_id`
  uses that exact ID in the IPC envelope. Before enqueue, the async continuation rechecks the panel
  operation token; after timeout or disconnect, stale preparation cannot still send a paste command.
  The panel clears both its command pending value and operation/watchdog value on failure,
  timeout, reconnect or disconnect; late results no longer mutate the abandoned action.
- `ui::tests::clipboard_panel_registers_action_before_platform_command_and_clears_it_on_result`
  exercises the production GPUI panel and bounded PlatformBridge endpoint: pending identity exists
  before command receive, the emitted envelope has that ID, failed results unlock the panel, and
  disconnect plus late success leave it in a retryable error state. This does not replace the
  separate end-to-end native visibility/topmost/focus-order gap below.

Migration gap: old NxTools hides the popup before target-focus restoration and paste input, while
`rust/apps/nxtools-desktop/src/ui/mod.rs` currently sends the paste action first and hides the popup
after a successful Windows result. A true hide-before-paste migration needs an ordered native hide
confirmation and a failure path that reveals the popup again; this lifecycle was not changed here.
- Historical first-preview gap (resolved by the explicitly approved2026-10-10 repair): visible creation/topmost-before-policy has been replaced with hidden creation and acknowledged noactivate→final bounds→topmost→show in the existing ClipboardApplication. Pending caller ID precedes dispatch; initial and reuse-policy steps have3s deadlines. Saved HWND and generation/cancel guards reject stale success/failure/timeout without hiding a replacement HWND. Controller8/8 and native96DPI All2/2 pass; actual physical pointer/frame/installed acceptance remains separate.
- Migration gap: no full cross-boundary fake-platform test currently drives popup visibility/topmost
  failure, stale visibility results, target/focus restore parameters and success/failure action
  ordering. The current gates protect source invariants and worker MarkUsed ordering, but do not
  replace that lifecycle test.
- Compatibility gap: direct image fallback cleanup is currently exercised through its persistence
  trigger; add an independent cleanup boundary test so stale fallback files are reclaimed even when
  no new image capture occurs, while preserving the path currently placed on the Windows clipboard.
- Fixed in the current implementation: `GlobalPointerPressedEvent` carries
  `sampled_at_unix_ms` and a monotonic `event_sequence`, `ClipboardPanelRequest` carries the summon
  sequence, and the Rust popup session rejects presses sampled before that session. Zero-valued
  fields remain a backward-compatible fallback for older PlatformHost peers; current package parts
  must ship together for the strict race protection.

## 2026-10-08 SOL remediation: storage and import authority

Reverified references: legacy `ClipboardRepository.Cleanup` permanently purges deleted rows after `max(7, cleanupDays)` and preserves favorite/pinned soft-delete exclusions; `ClipboardHistoryService.ImportJson` merges normal repository rows (no legacy portable ZIP implementation); `ClipboardPasteService.CopyItemAsync` does not restore the target, while paste uses the beforeRestore hide callback; `MainWindow.axaml.cs` dispatches copy/paste separately. Next keeps one session, stale-result rejection and deferred cross-entity updates.

- Next purge is `max(7, retention_days)`, not a hard thirty-day minimum. The default thirty-day preference remains unchanged.
- `archive.rs::import_archive` preflights only importable rows. Owned-file cleanup removes newly created resources/temp files after preflight failure, never existing/shared files. A late database error returns explicit partial inserted/updated/skipped counts. Already committed rows remain; cleanup deletes only newly owned assets proven unreferenced by `ClipboardHistory::referenced_asset_paths`. If that query fails, retain conservatively instead of deleting uncertain ownership.
- `clipboard_worker::migrate_storage` serializes staging, opening replacement history and locked durable config commit before publishing the new live history/paths. Failure retains source authority; deferred captures remain serialized with the worker.
- `ClipboardApplication::save_clipboard_section` does not own `storage_directory`: inside the disk-rereading locked merge it preserves the latest disk root. Only the worker migration transaction changes that field. `refresh_committed_migration_config` reloads latest disk state, not a stale pre-callback snapshot.
- The decisive race test holds the UI at source S after the worker commits destination D, runs an ordinary settings save, then completes migration. Disk and live worker must remain at D; this regression was red before the storage-field ownership fix and is now green.

Synthetic ZIP, SQLite-trigger, worker/config and GPUI regressions pass; real user archive/migration and native clipboard acceptance were not performed. See [the remediation evidence and limits](../audits/2026-10-08-sol-review-remediation.md).

## 2026-10-09 Clipboard + Tools surface review (before polish)

- Old `D:\Code\NxTools\Views\ClipboardHistoryPopupWindow.axaml:67-119` supplies an 8px safe inset, compact category/search/list hierarchy and separate selected-item state. `ClipboardHistoryPopupWindow.axaml.cs:61-167,206-259` constructs the popup, restores search focus when opened/activated, hides rather than destroys it, and hides its preview before hiding the popup. `:749-857,897-905` cancels superseded hover work, checks visible popup plus item identity before showing, creates at most one `ClipboardPreviewWindow`, updates/places it in place, and hides it on exit.
- Old `D:\Code\NxTools\Views\ClipboardPreviewWindow.cs:29-95,97-159,200-237` constructs a separate non-activating, topmost tool window, changes content before disposing its prior bitmap, places it within the display, and reapplies passive native style on open/topmost. `MainWindow.axaml.cs:971-1067` keeps copy separate from paste and supplies a hide-before-focus/paste callback. Next's stronger retry-on-failure and staged-frame contracts at lines 239-273 above remain unchanged.
- Zed `D:\Code\zed-main\crates\theme\src\default_colors.rs:205-225` distinguishes neutral hover/active/selected surfaces from focused blue borders; `crates\acp_tools\src\acp_tools.rs:708,738-752` tracks focus and routes interactions through click callbacks. These are visual/GPUI precedents, not clipboard lifecycle authority. Next's own `ui/components/action.rs:12-69` handles focused Enter/Space on KeyUp; do not attach a second key activation callback.
- Next presentation chain: `ui/mod.rs:1882-2493` composes category strip, search, loading/error/empty and bounded list; `ui/history.rs:80-417` renders action/selection/hover; `ui/preview.rs:58-196` renders the passive view without owning show/hide. `clipboard.rs:1673-1840` schedules generation-guarded preview and native hide/staging; C# is the native visibility/bounds owner. Cosmetic changes must keep fixed 300×440 default client, panel/editor focus, history selection distinct from hover, one preview, generation guards, frame-before-reveal, offscreen staging, native no-activate and deferred cross-owner updates. Current first-show/native acceptance gaps remain at lines 526-537 above.

## Keyboard refinement source boundary (2026-10-09)

- Rechecked `D:\Code\NxTools\Controls\ClipboardHistoryListView.cs:472-491`: left release selects and invokes `ItemClicked`, right release selects and invokes context menu. `Views\ClipboardHistoryPopupWindow.axaml.cs:321-369,418-427` keeps search focus, routes Enter to selected history, Escape to popup hide, and Ctrl+F back to search; `:376-385` treats the native resize grip as drag-only, not a keyboard action. `:845-905` reuses and hides a single preview; `ClipboardPreviewWindow.cs:29-95,200-237` creates it non-activating/topmost and applies native no-activate style. No popup resize/focus/native reveal ordering changes belong to this refinement.
- Next actionable custom GPUI entry points: `ui/mod.rs:2110-2216,2410-2505,2690-2825,3110-3288` (category add/tabs, search scope, back-to-top, undo, source toggles and editor buttons), `ui/history.rs:134-249,518-549` (row and management actions), `main_window/clipboard_page.rs:254-439,599-642` (two steppers and hotkey recorder). `ui/preview.rs:168-204` is intentionally passive; header move and resize grip are pointer gestures, not tab stops. The owning search/selected-item keyboard path and deferred cross-owner operations remain intact. For pointer-only actions, reuse GPUI keyed focus and the existing single KeyUp-generated click; do not synthesize a second Enter/Space listener. Category editor keeps its input focus until save/cancel; no row activation should leak through its occluding backdrop.

## Global per-application blacklist amendment (2026-10-10)

Reverified `D:\Code\NxTools\ViewModels\MainWindowClipboardViewModel.cs:595-619,956-965`
and `Models\AppState.cs:229-245`: legacy capture splits exclusions and compares case-insensitively;
KeePass, 1Password and Bitwarden default to clipboard-only exclusions. The requested global
per-app Quick Launch/clipboard rule table is an intentional product amendment, not legacy UI parity.
Canonical rules must distinguish absent legacy data from an explicitly empty table: migrate once,
derive native clipboard exclusions only from clipboard-enabled rows, never resurrect deleted or
unchecked rules from that compatibility projection. Basename/case/.EXE duplicates merge scopes.
Draft add/edit/remove is local; cancel publishes nothing. One locked Core transaction owns the global
network/system/rules save; only durable success defers weak live consumer updates to a later GPUI turn.
Consumers must not save the same rules again. Diagnostics use UUID, counts, booleans, fixed failure
stage and elapsed time, never application names/paths or clipboard payload. No clipboard history,
paste, hotkey, preview, retention or native protocol policy changes are part of this amendment.

## Feature-first workbench layout migration
Rechecked legacy MainWindow.axaml calling/storage/archive/retention controls, MainWindow.axaml.cs original/copy/plain/hide-before-target flow, ClipboardHistoryPopupWindow constructor/open/show/search/hide/one preview and ClipboardPreviewWindow passive style/clamp. Zed list_item stable start/end slots is visual precedent only. New main settings projection keeps every action/limit/modifier/hotkey/storage/archive callback in a non-shrinking scrollable group stack; no new controller/transaction. History source/name/time/management share stable slots, original actions unchanged; compact300x440/default/user resize and passive preview owner retained. Management Button intentionally prevents pointer focus; real Tab/KeyUp path tested rather than a second activation handler. Actual native owned scroll-to-storage reveals export/import/backup/compact/migrate; no real data operation performed. Full installed/DPI/native-preview acceptance separate.

### Compact editor layout ledger synchronization (2026-10-09)
Pre-edit advanced ledger rechecked legacy ClipboardHistoryPopupWindow.axaml.cs:61–167 (constructor/Open/Activated search focus/deactivation/close guards) and206–259 (reusable preparation → HidePopup → HidePreviewWindow → native/base Hide). Avalonia native base is unavailable; no lifecycle rewrite authorized. Current category editor remains244px at top104 inside actual300×440 default client. Readable12/18 help borrows source-list space rather than enlarging the panel;32px input,28px source/action/footer slots remain mounted. Full source names use tooltips. Long-list regression exposed unconstrained scrollbar-wrapper bounds; the identified source viewport now uses explicit GPUI overflow_y_scroll with flexible remaining height and nonshrinking input/footer. Original source KeyUp/focus/Save/Cancel/Escape and text IME/generation/paging/passive-preview owners remain. [Remaining evidence](../audits/2026-10-09-remaining-ui-layout-migration.md).

## Kit0.7.1 project menu boundary (2026-10-10)
User approves the project-owned Kit menu seam after executed300px submenu overflow. Reverified old NxTools Views/ClipboardHistoryPopupWindow.axaml.cs ShowItemContextMenu/CloseItemContextMenu: one menu, close predecessor before opening, clear identity on Closed, HidePopup closes menu before preview/native hide. Controls/ClipboardHistoryListView.cs right release selects the item before ItemRightClicked; no legacy source-category submenu exists there. NxTools.Next history.rs preserves its additional edit/category actions and checked/source snapshots. Former vendor popup_menu.rs resolve_submenu_x/update_submenu_menu_anchor/absolute submenu_position are behavior evidence only:4px client edges/gap, prefer available left space, clamp fallback, align hovered row y; flat border/no shadow preserved. No upstream source is modified.

Project menu invariants: one active window-local menu; entity/trigger lifetime cannot resurrect stale item callbacks; owned row snapshots, bounded scroll/layout within actual viewport, submenu follows actual row bounds; arrows/Home/End skip disabled/separators, right opens child/left or Escape returns to parent, outer Escape/outside/Tab dismiss, then previous focus restored. Pointer and one key-release dispatcher share eligibility; held/modifier gestures never double-dispatch. Dismiss before original callback, defer callback across menu/panel leases; hide closes menu without stealing focus. Open/terminal diagnostics use correlation/count/reason/elapsed only, never item/source/category/payload strings. Native popup HWND policy remains unchanged.

Implemented Kit menu reference follow-up: history.rs replaces only history context/submenu with project_menu.rs; shared open-target/category-tab dropdowns remain unchanged. One Base Root plugin owns active session; trigger prepaint liveness handles keyed-state caching, end-frame cleanup is deferred/session-checked and no synchronous Root notification during render/prepare. Fixed marker/label/arrow rails andneutral states independently follow ShellRS comparison, noGPL code. Actual300px submenu regression passes;220×140 long-category scroll retains24px parent edge. Mouse-entered child owns subsequent arrows,leaf Right preserves selection,child Escape returns to parent;one armed release/no modifiers/held duplicates. Original focus excludes predecessor menu handle andretired trigger restores external focus;hide stays non-restoring until failure handler explicitly recovers search focus. Real history callbacks retain four clipboard transforms,four management actions andtwo category checked add/remove actions. Full desktop510/510/strict lint/workspace/fmt pass;actual HWND/DPI/reader acceptance still separate.

## Authorized passive-preview repair (2026-10-10)
Previously verified old ClipboardPreviewWindow.cs constructor/Open/topmost/noactivate lifecycle remains authority: one passive auxiliary,reuse/update in place,hide when inactive,no primary activation-policy change. Source entries: ClipboardApplication::show_preview→clipboard_preview_presentation.rs acknowledged initial chain→C# SetWindowNoActivate/SetWindowBounds/SetWindowTopmost/SetWindowVisibility→GPUI backend HWND message owner. User-approved local Windows0.3.8 WM_MOUSEACTIVATE patch honors live WS_EX_NOACTIVATE; no registry modification/injection/focus stealing. Released branch response1 was real red; current owned Quick/Clipboard HWND response3,primary/disabled-policy1,re-enabled3 are green. Reuse reconfirms passive policy before existing offscreen/render staging,so an earlier noactivate failure cannot be bypassed. Controller8/8 covers ordered acknowledgements,each-step failure,cancel/stale show,initial/reuse timeout,old-HWND success/failure and timeout isolation. Full desktop519/519,0 ignored. Native synthetic client/reuse/UIA checks pass at96DPI; physical/mixed-DPI/reader/full product/installed lifecycle remains unverified.

## Queue06 Settings prerequisite — 4. Clipboard + Tools
Reviewed old MainWindow.axaml clipboard calling/storage/retention controls ->
MainWindowClipboardViewModel BeginClipboardHotKeyCapture/RestoreClipboardState/
CreateClipboardSettingsState/ConfigureClipboardStorage/On...Changed -> dirty section
save. ClipboardHistoryService Configure creates repository+asset directories without
moving old history; ExportJson/ImportJson carries formats and merges via repository
Save. ClipboardRepository Cleanup keeps pinned/favorite and max(7,days) purge;
Vacuum is outside transaction. This reference snapshot's online backup method was
not located in ClipboardRepository; prior CreateBackup note is not fresh evidence.
Next existing portable .nxtclip/online backup/backup-first compact/worker migration
remain product behavior, not invented legacy equivalents.
ClipboardHotKeyService Configure -> RegisterHotKey/no permanent hook fallback and
BeginCapture/CancelCapture inspected; Next acknowledged suspension/restoration is
stronger and unchanged. Popup constructor -> Opened/Activated focus -> PrepareForPopupShow
-> HidePopup/HidePreviewWindow -> base Hide; one PreviewWindow constructor ShowActivated
false -> Open/EnsureTopmost -> ApplyPassiveWindowStyle native WS_EX_NOACTIVATE and
SetWindowPos(SWP_NOACTIVATE). Avalonia native base unavailable.
Keep Next MainWindow original recorder/hotkey_focus/KeyDown/Escape/focus-out handlers;
per-control cx.defer -> original section setters, raw saturating steps/Core normalization,
worker retention/cleanup and source-root-preserving locked save. Global app exclusions
stay global-owned. Storage/archive actions retain original picker/worker/NotificationCenter
busy/progress/error/cancel/results and directory details; no duplicate status manager.
Official Settings search/filter/sidebar/scroll allowed, resettable(false), individual
SettingItem/SettingField public slots/Form for complex fields, not whole old page slot.
Preserve main1060x650/min920x480, existing fields/actions and single passive preview.
No actual user config/clipboard/history/archive/migration operations authorized.

Queue06 adoption result: releasedSettings/3Page/3Group/13individualItem/publicField
render,recorder+path actualFormField;officialsearch/selection/filter/sidebarresize/
scroll,resetfalse. Defaultstyledrecorder/modifierButton+officialmenu/Smallstepand
managementButtons retain original immediate section/deferredcallback semantics.
NumberField reviewed but its editablef64Change is not same as existing noneditable
saturatingstep/save policy;publicstyledButtoncomposition keeps original owner.
All fields/actions/inlinevalidations/risk/status/workerpath/optionalTooltip and original
NotificationCenter progress/error/cancel/results retained.11keyfunctionbodies equal
dirty06baseline;application/worker/MainWindow filesbytes equal.9new tempSQLite/config/
fakepicker/scroll/longerror/path/KeyDownUp/Tooltip regressions;580/580/strictchecks/
7sourcegates0. No newcomponenttest archive/backup/compact/migration execution;no
realuserdata/Clipboard/history/standalonebuild/start/Release/service/nativeacceptance.
Exact evidence/limits: owning ui-queue-outcomes.md06;whole4/finalgallery pending.

## Queue07 search/editor/navigation prerequisite
Rechecked legacy ClipboardHistoryPopupWindow constructor/Open/Activated -> search
focus, PrepareForPopupShow -> reused state, HidePopup -> close menu/hide one preview
-> public base Hide; PreviewWindow ShowActivatedfalse/EnsureTopmost -> real native
WS_EX_NOACTIVATE/SetWindowPos(SWP_NOACTIVATE). Avalonia native base unavailable.
PopupViewModel OnSearchTextChanged -> cancelled debounce/Refresh; SelectedCategory
-> reset selection/Refresh. Popup UpdateCategoryButtonStates selects five builtin
categories. Old MainWindowClipboardViewModel SaveClipboardItemMetadata -> HistoryService
UpdateText/normal repository persistence. Next custom categories/source assignment,
drag order and extended search are existing Next behavior, not verified old equivalents.
Keep Next search InputState/Change query intent/100ms generation/scope/page cache;
text TextareaState and category InputState silent hydration/actual edits/worker-save/
Cancel/Escape/focus restoration;300x440/defaultuser resize,244px categoryeditor at104,
source list bounded/full help,backdrop+editor occlusion/click consumption/previewoff.
Official0.7.1 InputGroup/Addons/Button/Form/Field/Textarea reviewed;Tabs Tab public
InteractiveElement/drag/drop/context and TabBar original ScrollHandle candidate;
ToggleGroup multi-bool/child callback override is not automatically exclusive nav.
Actual compatibility/budget/keyboard tests required before navigation adoption;
no worker/native/controller/library transplant or genuine user data operation.

Queue07 actual result: ui/mod.rs adopts released0.7.1 InputGroup/Search addon/InlineEnd
scopeInputGroupButton on originalInputState;SmallFormField/defaultstyledTextarea and
categoryInput,sourceSmallCheckbox/styledSaveCancel/undoButtons. No duplicate ordinary
control frame/callerappearance suppression. Original query/IME/scope/generation/worker
paging/cache/edit/category-save/Cancel/focus/occlusion/passivepreview retained;27key
bodies+constructor equal07dirtybaseline,Application/worker/oldtests bytes equal.
300x440/userresize/244categoryeditor and long80line default/minimum/footer bounds pass.
Worker fixtures seed AFTER initialization's existing legacy reset; bounded real worker
terminal wait, not production reset/save changes.10new syntheticproduction/APItests;
focused10/10/full590/590/strictchecks/8sourcegates0,notnative orvisual acceptance.
Navigation narrowly retained: releasedC:tab/{tab,tab_bar}.rs directXSmallunderlineTab
pointeronce/<=28px works; directkeyboard and simplepublictrack_focus+tab_stop recipe
does not expose original target/action. C:button/toggle.rs checkedSmallToggleGroup
actualfocusedSpace down/up invokesgroupcallback0. Otherpublicfocus/action/child and
fullscroll/drag/drop/context adapters remainunverified,not universally unsupported.
Originalcategory_tab/order/drag/drop/wheel/rightmenu/selection/onefocusable_content/
ScrollHandle unchanged;8longcategory actualwheel/rightmenu regression passes.
Exact recommendations/options/remainingdecision: owning ui-queue-outcomes.md07.
No07actualHWND/physical/DPI/reader/installed/PNG/buildstart/Release/service/realdata/
delegation/commit;wholeClipboardTools/history/sharedstates/finalgallery remains open.

## Queue08 history/content prerequisite
Rechecked old Controls/ClipboardHistoryListView.cs::DrawRow/DrawPreviewText/OnPointerReleased/
OnPointerWheelChanged: stable item identity, literal text/image rendering, left action/right
context selection and wheel-before-preview cancellation. Popup constructor/Open/Activated/
PrepareForPopupShow/HidePopup -> close predecessor menu/hide preview/base Hide;
ShowItemContextMenu/CloseItemContextMenu retains one session and original paste/copy/plain/
linebreak/pin/favorite/delete. PreviewWindow constructor/Open/EnsureTopmost -> native
WS_EX_NOACTIVATE/SetWindowPos(SWP_NOACTIVATE) unchanged; native Avalonia base unavailable.
Next richer category/edit/worker cursor/cache and failure-retry policies remain existing
Next behavior. Queue08 evaluates releasedList/ListDelegate (same-height section rows and
owned query/selection/scroll), statelessListItem children/suffix, Toolbar publiccontent,
Label literaltext/Tag/DescriptionList metadata and Empty/Alert content. Prefer actual
stateless presentation without replacing variable-height GPUI ListState/paging/cache.
No automatic Markdown/HTML interpretation, second menu/native/preview/controller owner.

Queue08 result: history realstatelessListItem/public children/selected+aria projection,
XSmallToolbar/publiccontent/20pxstyledactionButtons/literalLabel source/content/metadata;
original variable60+4text/files/cappedimage144+44+4 heights, payloadID/cache/controller
and one focusable row preserved. Ordinarychrome/oldmicrometadata overrides removed;
zero-callcompact_result_row removed,livecompact_list_row unchanged. PopupactualEmpty/
HeaderMediaTitleDescription and XSmallerrorAlert/boundedscroll retain originalstate
conditions/input/focus/query/paging. AlertString is plainText::String;noMarkdownHTML.
OriginalsingleRootprojectmenu/sourcechecked/actions and passivepreview unchanged.
46originalbodies and originalmenuprojection equal08baseline;Applicationworker/menu/
preview bytes equal.8new syntheticproductiontests/full598/598/strictchecks/8gates0,
notnative/visualacceptance. Exact owningoutcomes08;no realClipboard/history/PNG/buildstart/
Release/service/delegationcommit. Sharedstates10/finalgallery12 remain open.

## Post-gallery user amendment — 2026-10-12
User asks continued actualTabs adaptation and a shorter searchInputGroup, plus targeted
layoutbalance corrections after rejecting overlyloose/unbalanced visuals. Original
popupconstructor/Open/searchfocus/category/HidePopup/onepassivepreview chain above
rechecked;nativebaseunavailable. Preserve300x440/defaultresize/244editor/source/queryIME/
generation/worker/controller/order/dragdrop/wheel/context/singlefocusaction/deferred
crossowner boundaries. Test officialcompactsearchtier and realTab within28pxnav lane;
no dependencypatch/secondselectiondispatcher. Prior failed directfocus recipes are
narrow evidence,not blanketTabsrejection. New realproduction redgreen required.

Released0.7.1 `B:tabs.rs::Tab::render` stops Left MouseDown propagation, so an outer
Button cannot own a child Tab's pointer activation or drag initiation. Public styled
`C:tab/tab.rs::Tab` forwards InteractiveElement/track_focus/on_drag and its single
on_click to that same Base node. `G:elements/div.rs` synthesizes a focused Enter/Space
click on release at the tracked node, guarded by focus generation and modifiers.
Adapt the real Tab with the original keyed focus and sole selection callback;
move drag initiation to that node, retain original strip scroll/drop/context owners.
Production pointer/focused-release/order/drag regressions remain required.
Productionredgreen confirms childTab pointer selection and focus:BaseTab's stopdown
also prevents an earlier caller MouseDown focus listener. Solepublicon_click therefore
focuses originalkeyedhandle then calls originalselect_category. Actualfocus remains
acrossrepaint;Spacedown noaction/up generation+1 passes. No duplicatekeylistener.
Longwheel/rightmenu passes/full623green. CompleteTabtraversal/held/modifiers/actualdrag
matrix remainsunverified;new30PNG uninspected due Browser sessionviewblocked.

## Installed resize / first-preview diagnosis (2026-10-13)
Reverified old popup OnPropertyChanged:298-318 emits size only while visible; MainWindow.axaml.cs:878,889-913 subscribes persistence/restores clamped size before Place/Prepare/Show/Activate. Native Avalonia base source remains unavailable. Old QueueVisibleImagePreviewWarmup:552-605 warms at most8images after120ms, while hover:749-905 preserves configurable default800ms/item/cancel/onepassivewindow.
Next now observes actual confirmed-visible logical viewport in ClipboardPanel::observe_panel_bounds, defers original-owner size intent, and coalesces panel-only locked config merge in ClipboardApplication::observe_panel_size. Hidden startup/default,stale session and scale-multiplied persistence are rejected; ordinary settings saves preserve independently owned size. Real owned fixture goes red300x440 vs512x688 without observer; restored final658Desktop suite passes.
First preview remains800ms plus hidden creation and4acknowledged native steps; no safe prewarm optimization established. Bounded correlated stage timing and cancellation fixtures added; injected25ms milestones are not installed measurements. Full evidence/limits: [installed lifecycle outcomes](../../.trellis/tasks/10-13-installed-lifecycle-feedback/outcomes.md). No new package/native/visual acceptance.

## R2 history action/status visibility prerequisite (2026-10-13)
Reverified `D:\Code\NxTools\Controls\ClipboardHistoryListView.cs:79-99,249-299`:
constructor binds pointer/wheel once; `DrawRow` treats selection/hover as row styling,
but favorite edge/icon (`271-280`) and pin icon (`289-293`) depend only on their
independent item flags. Selected/hovered rows do not remove those identifiers.
Legacy has no inline management toolbar in this DrawRow; Next's existing toolbar
is retained under the explicitly approved R2 hover/keyboard-only visibility rule.
Popup constructors/event bindings `ClipboardHistoryPopupWindow.axaml.cs:61-167`
and preparation/hide `206-259` keep search focus, one reusable popup, menu-close ->
preview-hide -> base Hide. `ClipboardPreviewWindow.cs:29-95,200-237` remains one
passive topmost window with WS_EX_NOACTIVATE/SWP_NOACTIVATE; Avalonia native base
source remains unavailable. This presentation slice changes no native lifecycle.
Released Kit0.7.1 ListItem children/selection/public interaction, Toolbar content,
Button keyed focus/one release-click and Icon path -> Svg -> paint_svg were read
before adding real production paint/focus regressions. Exact APIs and pending-red
handoff are recorded in the owning round2 `history-outcomes.md`; no fix or compiled,
native, installed, physical/DPI/reader acceptance is claimed at this checkpoint.

## Round2 integrated preview/history correction (2026-10-13)
The prerequisite checkpoint above is superseded by `.trellis/tasks/10-13-installed-feedback-round2/outcomes.md`: selected-only action rail hidden, pin/favorite independent paint outside hidden time slot, actual SVG/hover/keyboard/minimum no-overlap tests pass. Contradictory ForegroundChanged while GPUI remains active is same-session next-turn rechecked; genuine blur/press/Escape retained. Reused preview now offscreen -> acknowledged C# client bounds -> existing entity update -> fresh frame -> generation-scoped reposition, no competing GPUI native resize. NativeMethods.SetWindowBounds final client readback verified; timeout/failure/cancel/oldHWND regression retained. Font-wrap fixture passed before change (no whitespace-collapse fix claim); actual installed large blank area and sporadic hover-close still need current-package interaction evidence. Full674Desktop and download/native-safe owner checks are source evidence, no live clipboard/native/visual/package acceptance.

## Installed summon-hotkey alignment correction (2026-10-14)

Rechecked legacy MainWindow.axaml1460–1495: label/editor share one horizontal row,
Windows registration status remains separate visible state. Current Next Settings slot0
keeps the existing recorded-hotkey Button and acknowledged suspend/cancel/registration
owner; replaces its vertical Form presentation with shared aligned_setting_row in the
official SettingItem render slot. Title-adjacent help and right-aligned editor share center,
registration state remains below. Production red measured24.5px lower stacked editor;
actual920/1060width center/right-edge/firstrow/statusbounds regression is green. No
native hotkey policy, user data, InputState or popup lifecycle change is part of this fix.
Evidence: .trellis/tasks/10-14-installed-feedback-actions/ui-outcomes.md.

## Trusted-extension clipboard seam (source reverified 2026-10-14)

- `D:\Code\NxTools\Services\ClipboardCaptureService.cs:15-28,31-125` injects save
  delegates into the single capture owner; ignore markers/time suppression precede saving.
- `Services\ClipboardHistoryService.cs:124-145,158-177,190-229,237-269` returns the
  repository-saved item before raising history-change notification. Image assets are written
  before repository save; this is not evidence for an old script runtime.
- `Services\ClipboardRepository.cs:271-345` commits native formats and duplicate revival
  in the same transaction, then returns the stable existing/new row ID.
- `Services\ClipboardPasteService.cs:13-33,35-52` resolves full content through the existing
  history delegates; copy does not activate a target or invoke the paste hide callback.

Invariants for the narrow Next seam: one serialized worker/history and one native capture
listener; notify only after required asset promotion and history commit; failed persistence
and empty capture publish nothing. Duplicate revival retains its row ID and is distinguishable
from insertion. Derived text/files go through the existing capture owner, never edit/delete
their source. Native replacement copy reuses the existing correlated action/suppression seam;
admission is not Windows success. Immutable bounded snapshots expose no GPUI entity/HWND/native
formats. Cross-owner effect dispatch and completion callbacks run on a later application turn,
never inside capture callbacks or database transactions. This adds no popup/preview placement,
focus, show/hide or script-runtime ownership; parent composition still owns trust, cancellation,
event opt-in and causality routing.

## Manual extension feedback recheck (2026-10-15)
User-approved `.trellis/tasks/10-15-extension-action-feedback/` adds usage and local one-shot feedback, not a new native popup owner. Re-read old `Views/ClipboardHistoryPopupWindow.axaml.cs` constructor/Open/Activated/Deactivated, PrepareForPopupShow and HidePopup through Avalonia Window.Show/Hide; old `Services/ClipboardRepository.cs` Save/GetPopup ordering/duplicate revival. Preserve explicit show/search focus, original hide/deactivate/outside-pointer and separate passive preview boundaries. There is no equivalent cross-feature plugin feedback in that old chain; new behavior is explicitly requested. Next `ui/history.rs` -> original `ClipboardItemAction` callback -> deferred manual chooser -> per-run application/runtime -> original adapter/worker commit observation -> explicit `ui/extension_feedback.rs` exact-ID location. `nxtools-clipboard/src/query.rs::page_containing_item` uses the same store/order and bounded SQLite snapshot. Dialog close does not cancel/replay; one Escape belongs to the owning transient layer. No reference binary, live clipboard or user script executed.

## User-authorized outside-HWND context menu correction (2026-10-15)
Reread old `D:/Code/NxTools/Views/ClipboardHistoryPopupWindow.axaml.cs:61–167` constructor/Open/Activated/Deactivated guards, `:248–259` HidePopup -> CloseItemContextMenu -> HidePreviewWindow -> inherited Window.Hide, and `:967–1002` ShowItemContextMenu -> close predecessor -> new Avalonia ContextMenu -> Closed clears identity -> menu.Open(ItemsList). Avalonia native Popup implementation source is not present locally, so no invented lower native call is claimed. Locked Kit0.7.1 `src/native_menu/mod.rs` explicitly documents OS NativeMenu can exceed HWND while GPUI PopupMenu is clipped; `menu/context_menu.rs` and `popup_menu.rs` use anchored/deferred within the window. User now explicitly supersedes earlier narrow-window clamp exception: menus must extend beyond Clipboard HWND. Windows API ownership stays C# via bounded context-menu coordinator, Rust retains IDs/tree/action eligibility/session callbacks; official PopupMenu remains for larger Downloads surface. No upstream/registry/dependency edits. Invariants: one session, pending before dispatch, menu guards panel deactivate/outside until terminal, hide cancels exact menu, Escape keeps source, outside dismissal closes source, stale/disabled/unknown leaf cannot dispatch, callback after menu terminal and cross-owner defer. Actual owned native overflow/placement/cleanup and source-session regressions are required; not implied by this source review.

## Item-local extension feedback authorization
User rejects the panel-top compact feedback and requests progress/outcome WITHIN the clicked history item. Reverified legacy Controls/ClipboardHistoryListView.cs472–491 (right release selects actual cached item then ItemRightClicked) and ClipboardHistoryPopupWindow.axaml.cs127–153/248–259/967–1002 (focus/deactivate guard, hide closes menu first, callback captures exact item). There is no legacy plugin-progress row; this is an explicit new presentation behavior, not a claimed native migration. Preserve current native menu/preview/window lifecycle. Next ClipboardOrigin carries source itemID, original ListItem gains a lower feedback slot, original ListState remeasures changed rows while preserving scroll. Runtime/effects/worker exact result owner unchanged; repeating official Spinner pending/visible only, static under reduced/unknown motion. [Current amendment](../../.trellis/tasks/10-15-extension-action-feedback/item-feedback.md).

## Official styled outside-HWND menu amendment (2026-10-16)
The user rejects white unstyled Windows HMENU in dark Clipboard. This supersedes only the renderer choice in the2026-10-15 correction, not its outside-source-HWND requirement. Reverified old ClipboardHistoryPopupWindow.axaml.cs127–167,206–259,967–1002 and ClipboardHistoryListView.cs472–491: cached exact item before right-menu event, one predecessor-close owner, deactivation guard, hide closes menu before preview/inherited Window.Hide; Avalonia native popup source remains unavailable. Locked Kit0.7.1 PopupMenu::build/item plus PopupMenuItem::submenu/element/separator/checked/disabled/icon/on_click, Focusable and DismissEvent now render on a separate GPUI transparent popup Root. Default26px official rows/text_sm13px, popover grammar, check rails and vendor cascades remain styled; destructive label/icon uses public content slot and existing semantic danger. No palette/dependency fork.

Verified Next call chain: project_menu::MenuTrigger → deferred native_context_menu::open → pending source HWND/session/weak trigger + immutable eligible callback map → styled_menu_surface::open(hidden independent HWND/one Root) → typed ShowContextMenu.popup_window_handle → NativeContextMenuCoordinator.PrepareStyledSurface (same-process/source-session validation, ClientToScreen/MonitorFromPoint/GetMonitorInfoW, WS_POPUP/tool owner, final work-area client bounds/readback; no TrackPopupMenuEx on this branch) → deferred official menu placement → acknowledged visibility/activation phases → first-wins terminal → remove popup → deferred original source callback. The auxiliary viewport, not the unchanged300×440 source HWND, contains cascades; no production clipped fallback. Unique phase IDs reject stale/duplicate native replies; one session diagnostic ID owns acceptance/ready/terminal/elapsed/native errors. Existing source outside/deactivate guards and hide cancellation retain their helper entry points. Menu failure/outside helpers now match source session as well as source window, preventing an old dismissal from closing a replacement session. All cross-entity callback effects are deferred.

Regression/source evidence and pending compiled/native/Light-Dark/DPI/user acceptance are tracked in [menu slice ledger](../../.trellis/tasks/10-16-menu-notification-refinement/menu.md). Static gates and .NET semantic fixtures are not paint or interaction acceptance; no package/install/service/plugin execution is implied.

Current correction closeout: official menu model placement now waits for a freshly submitted GPUI frame before native reveal/activation, and first inactive preparation state is ignored until actual activation. Source/session/trigger remains checked at final callback. Original-item pending feedback remains; the user's later notification request moves terminal/start-failure results to the shared exact-window notification host with explicit host-confirmed ViewResult/Details, no automatic report/copy/paste/reopen. [Current source/compiled/native/render evidence and limits](../../.trellis/tasks/10-16-menu-notification-refinement/outcomes.md) supersede the previous pending-compilation checkpoint, not physical/mixed-DPI/installed/user acceptance.

## Approved unified Markdown and image modes (2026-10-22 prerequisite)
User supersedes TextView-only/edit-only scope: list summary, full passive textpreview and explicit editor adopt fixed external editor with narrowly approved passive element mode. Read [editor chain and invariants](markdown-editor.md) before implementation. Original full-by-ID/nativeformats/paste targets,400charsummary/210DIPtextcard/190image/62files/8gap, original category/search/IME/action ownership unchanged. Image modes defaultAuto/Fit/Panorama/Light800/Mini500/Compact300 use same passive HWND and true visible workarea; rawpixels/DPI/axis scroll separate from list geometry. Legacy ClipboardPreviewWindow constructor/update/PlaceNear/ApplyPassiveWindowStyle rechecked; fixed440x340 is intentionally superseded only for imagepreview. New settings use originalsectionmerge, failure retains live priorvalue/storage root. No productlaunch/package/install implied.

## Approved density follow-up (2026-10-22)
User supersedes fixed210DIP with whole-textcard setting120–320/default160; same400charsummary/fullpayload/nativeowner, image190/file62/gap8 unchanged. Currenthistory.rs caps body/slot/card at147/300/320; originalListState mustremeasureonnext-turnpresentationupdate,notreplacequery. Official0.8Settingsnumberfield owns bounds/step32DIP; deferred original section setter only. Conditional16DIPfeather allowed only asonepaintgradientusingexistinggeometry, noadditionalparse/layout/frameclock/blur orhitinterception. OriginalClipboardpopupconstructors61–125,PrepareForPopupShow206–239,HidePopup248–259 were reread; noinput/native lifecycle rewrite. Task10-22-clipboard-density-resource-names ownsred/green andacceptance.

## Approved clipped-text finish and repeated input symptom (2026-10-22)
User cancels hoverednotificationCtrlC entirely; noframeworknotificationpatch is authorized. User approves~14DIPstaticbody-onlyfeather and asks forverylightcardshadow plusreportsCtrl+digits/type-anywherestillfail andpreviewappears. KeepListItem/InputGroup/currentInputState,fullpayload,120..320/default160wholecard,gap8 andonepassivepreview. InspectoldClipboardHistoryPopupWindow.axaml.cs61–167constructor/Open/Activated,206–259Prepare/searchfocus/Hide,321–428tunnelnumber/TextInput. OriginalAvalonia base nativeShow/Hide isexternal/unavailable; existingexplicitprimaryactivationintent isclear, not proof ofnewnativebehavior.
NewlyverifiedNext chain: clipboard.rs::open_panel creates hidden show:false/focus:true GPUI HWND -> nativebounds/topmostACK -> SetWindowVisibility(true)ACK -> GPUIsearchfocusonly. C# WindowsPlatformAdapter.NativeMethods.cs::SetWindowVisibility544–554 usesSW_SHOWNOACTIVATE; GPUIWindowswindow.rs578–588 defersinitialplacementwhenhidden,sofocus:truealonehasnotshown/activatedit. NoClipboardActivateWindowcommandexists; QuickLaunch has explicitnativeactivation. Addrealproductioncommand-chainredbeforefix: revealACK->current-sessionActivateWindow->activationACK->deferredsearchfocus,stale/hidden/disconnect/failurecannotactivate/revive. Do notcallGPUIactivate_windowonhiddenpreparedHWND (canreplayinitialplacement); C#existingnativeActivateWindowownsfocusAPI. No changes to passivepreviewpolicy/nativewriter, no realclipboard/globalinputtest.
Feather usesexistingrenderedbodybounds,notnewparse/layout/timer; publicListItemgroup/group_hover canstylepaint-onlyoverlaywithoutsecondhoverstate. Matchactualnormal/hover/selectedcompositionincludingalpha beforeclaimingvisualcorrectness. Lightstaticshadow<=oneboundedshadowpermountedcard,notwholewindowblur/repeatinganimation; realpaint/clip/nohit/gap/payloadtests andboundedcostcomparisonrequired. Performanceclaim islimitedtomeasuredCPU/scene/idlework,notGPU/installedacceptance.

## Stationary-pointer preview admission diagnosis (2026-10-22)
User reports a preview appearing on summon without moving. Rechecked old `D:/Code/NxTools/Controls/ClipboardHistoryListView.cs` constructor79–85, ResetScroll107–114 and OnPointerMoved/UpdateHoveredPreviewItem434–460: pointer movement establishes the hovered index; preview entry is emitted only when that cached item changes; exit clears it. Popup constructor/Open/Activated61–167, Prepare/Hide206–259 and ShowPreviewWindowAfterDelay749–905 retain800ms, cancellation/item/visible checks and one passive auxiliary. Avalonia native base remains unavailable; Next's acknowledged C# presentation chain is not changed by this repair.
Executed real-panel regression `idle_preview_mount_under_stationary_pointer_does_not_schedule`: empty mounted list -> pointer(45,155) -> restore rows -> draw three times without another move. Actual failure: painting a row underneath a stationary cursor arms the original preview timer (1failed/0passed). New fixture excludes old-session residue; no native show/activation event is needed. Exact published pre0.3.9 `elements/div.rs:3133–3184` schedules on_hover from paint as well as mouse events; `window.rs:3740` refreshes hit testing before paint. Thus hover presence is not pointer intent.
Preserve requested300x440/default layout, released Component0.8.0 ListItem selected/hover styling and all existing size/parent adaptations. Public `ListItem::on_mouse_enter`132–137 maps to real on_mouse_move213–215, including moves inside an already-hovered row. Use that public slot for entry, reject unchanged panel-retained screen coordinates (seeded at creation, refreshed on native bounds changes and actual panel moves) and pressed/drag events, keep on_hover(false) for original leave cancellation. Deduplicate current-item entry at the original PreviewSession so movement does not restart800ms. Keep deferred current-panel readiness/session/query checks and original full-ID hydration, scroll suppression, generation/nativeACK/frame/reveal/one passive HWND owners. No timer increase, polling, global input, vendor, C#/protocol, style or notification changes; no pointer/payload logs. Physical/installed acceptance remains separate.

## Session-only panel pin and menu feedback (2026-10-24)
The newly approved task `10-24-runtime-interaction-feedback` intentionally changes auto-dismiss policy only while pinned for the current opening; successful paste remains visible and native target focus restoration is preserved. Explicit hide/reopen resets pin without persistence. Rechecked legacy `Views/ClipboardHistoryPopupWindow.axaml.cs:127-167,206-259`: Opened/Activated restore search, Deactivated skips hide while `_isContextMenuOpen`, HidePopup cancels preview/menu then calls inherited Avalonia Window.Hide (framework internals not available in checkout). Existing Next `ui/components/native_context_menu.rs::handle_notification -> finish -> clipboard.rs::context_menu_outside -> ClipboardPanel::close_from_native_menu` currently merges source-panel and external clicks. Distinguish these without replacing the menu owner; opening must retire pending/visible preview, new/delayed/cache reveal stays blocked for the menu lifetime. Preserve one preview HWND,800ms deliberate movement, query/session/generation checks and deferred cross-owner effects. This note records reference/approved invariants, not completed implementation or physical acceptance.

## Audit #10 retained history focus identity prerequisite (2026-10-25)
Canonical phase remains Clipboard + Tools. Rechecked legacy Controls/ClipboardHistoryListView.cs
constructor79–99/PointerReleased472–491: coordinates resolve cached actual item and pass
that item to selected/ItemClicked/ItemRightClicked. Popup constructor61–167 binds actions,
Opened/Activated search focus, PrepareForPopupShow206–239 and HidePopup248–259 retain one
reused popup/menu-before-preview-hide -> inherited Window.Hide; Preview remains independently
passive. Avalonia native base unavailable, not new show/hide proof. No native owner change.
Exact pre0.3.9 elements/list.rs::List has id=None and invokes render_item/layout_as_root/
prepaint without adding an index element namespace. Window::use_keyed_state -> with_global_id
uses the current full element stack. Component0.8.0 Button::render and project unchanged
FocusableContent use that keyed state for actual FocusHandle; gpui div owns one release click.
Before repair history slot/wrapper/ListItem/Toolbar/open and management Button IDs use index,
so Prepend/front splice or reorder can retain focus/armed state on a different domain record.
Required boundary: entry.id+action-kind throughout actual ancestry and production focus lookup,
existing view/window namespace isolates panels; index only positions/debug hints/paging.
Keep original bounded320rows/ListState.splice/query-generation/selection/full-by-ID worker
actions/native menu/one preview/search InputState. Removed row/action retires focus/gesture;
release must never invoke occupant of the old index. Original Ctrl-digit already arms itemID,
so preserve its generation checks rather than add a second dispatcher. No shared-helper,
vendor/native/protocol/theme/layout redesign. Real production focus/page/action red-green,
exact own-deltas and separate physical/native/DPI/user limits belong to single-item task
`.trellis/tasks/10-25-clipboard-stable-focus/`; no other audit item is authorized.
Additional verified removal boundary: pre0.3.9 app/context.rs::Context::on_focus_lost
retains a weak view subscription; window.rs::draw dispatches it after the rendered tree swap
only when a previous nonempty focus path becomes empty. Window::focused alone can still
return a strongly held orphan. Window::focus_lost_restore_target resolves only a still-mounted
ancestor during that callback. A non-Tab panel focus scope can therefore qualify the original
search fallback without a row cache, index lookup, global dispatcher, or cross-panel focus steal.
Changing focus increments the original GPUI focus_generation, cancelling an old armed release.

## Audit #8 command-result naming follow-up (2026-10-26)
Actual Settings Buttons in main_window/clipboard_page.rs defer to ClipboardApplication:
request_clipboard_export/import -> request_clipboard_archive_path -> typed
PickClipboardArchivePath; request_clipboard_storage_migration -> PickClipboardStorageDirectory.
The original matching CommandResult handler retires pending state and starts no worker operation
when success carries no nonempty path. These picker commands use one U+2026 in visible labels;
the original Button label also supplies its semantic name. Backup/Compact call
start_clipboard_database_operation directly, with the original serialized worker and fixed
database-parent/backups destination; they do not open a picker and retain no ellipsis.
Existing history Delete is soft_delete_async with original undo/restore owner, not file deletion.
No changed import/export/retention/privacy/storage policy or new confirmation is authorized.
Production Button-label and synthetic pointer/Tab/Space/IPC-cancel regressions are recorded in
.trellis/tasks/10-26-command-naming; no actual archive/data/native picker operation was performed.
Strict/broad shared-tree failures and separate physical/reader/visual/user acceptance remain
explicit there; these source entries do not declare global audit or release completion.

## Approved summon-hotkey reset addition (2026-10-28)
User asks for reset immediately left of a changed global shortcut. Rechecked old MainWindow.axaml.cs
751–818 Configure/BeginCapture/Escape/captured callback -> ClipboardNativeService.cs41–65 hidden
message-window/start/configure -> ClipboardHotKeyService.cs42–81 RegisterHotKey and125–129dispose.
Only explicit settings default restoration is new; popup show/hide/focus and350msdebounce unchanged.
Next existing set_clipboard_hotkey requires acknowledged recording: do not fake that state for reset.
Admit fixed Coredefault through same pending ConfigureClipboard/CommandResult/section-save owner,
block concurrent recording/configuration and retain old binding on native rejection. Official
Medium32outlined Button in original Settings slot; no native framework/transport/provider change.

## Ctrl-digit-only search input correction (2026-10-30)
User explicitly requires Ctrl+number for indexed paste; unmodified digits are ordinary search text,
including an empty query. This supersedes earlier legacy empty-query digit shortcuts. Rechecked old
ClipboardHistoryPopupWindow constructor/Open/Activated127–167 and Prepare/Focus206–247: showing
and activation focus the existing SearchBox; Hide248–260 clears menu/preview before inherited Hide.
Next ClipboardPanel constructor/InputState -> original Root -> native reveal/ActivateWindow ACK ->
focus_search_for_session retains one current session/input owner. Production regression without
fixture focus repair confirms the input FocusHandle is focused, but plain numeric text is intercepted
by efficiency.rs::number_shortcut_index's legacy empty-query branch. Require Control when arming;
keep original item ID/query generation, Ctrl-first release, IME/editor/menu ownership and native
success-only paste lifecycle. Ordinary numbers must reach the existing InputState without a paste
command; do not add another dispatcher or global hook.
