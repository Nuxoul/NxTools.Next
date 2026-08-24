# Clipboard reference implementation index

Last verified: 2026-07-18.

## Behavioral contract

- The clipboard is a shortcut-opened popup, not a page in the main window.
- Capture preserves text, image, file-drop and useful native formats before selecting a primary
  history kind.
- Original paste restores the richest compatible representation; plain paste maps images and files
  to paths.
- The popup targets the window that was active before it opened, hides after confirmed paste, and
  stays available when a platform action fails.
- Pure images must expose PNG/DIB for image-aware applications. `CF_HDROP` is added only for
  allowlisted file/terminal targets that need a path fallback; rich editors such as Zed must not
  receive the same image a second time as a file attachment.

## Old NxTools: behavioral source of truth

Root: `D:\RiderProject\NxTools`

| File | Verified responsibility |
| --- | --- |
| `Services\ClipboardCaptureService.cs` | Observes Windows clipboard content and constructs captured history input. |
| `Services\ClipboardNativeFormatService.cs` | Reads/writes native Windows clipboard representations and selects target-compatible image behavior. |
| `Services\ClipboardPasteService.cs` | Writes the selected item, restores the target window and performs the paste fallback sequence. |
| `Services\ClipboardPasteActionService.cs` | Converts special actions to payloads; image plain-text action returns `ImagePath`, files return their paths. |
| `Services\ClipboardActivationResolver.cs` | Maps click/modifier actions; names image plain paste `粘贴图片路径` and file plain paste `粘贴文件路径`. |
| `Services\ClipboardHistoryService.cs` | History-facing orchestration. |
| `Services\ClipboardRepository.cs` | Clipboard persistence. |
| `Services\ClipboardHotKeyParser.cs` | Parses Ctrl/Alt/Shift/Win plus a non-modifier key into Win32 modifier and virtual-key values. |
| `Services\ClipboardHotKeyService.cs` | Re-registers the configured global hotkey and reports registration failure without installing a permanent low-level hook. |
| `Services\ClipboardActivationResolver.cs` | Defines five configurable modifier-click actions and rejects multiple simultaneous modifiers. |
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
  -> ClipboardActivationResolver
  -> ClipboardPasteActionService (original or path/text transformation)
  -> ClipboardPasteService
  -> ClipboardNativeFormatService + target activation/input
```

Verified retention rules in `ClipboardRepository.Cleanup`:

- age and overflow cleanup soft-delete only unpinned, non-favorite items;
- maximum item count is clamped to 100–100000;
- cleanup days use a minimum of one day;
- deleted rows are permanently purged only after at least 30 days.

Verified history identity and ordering rules:

- `ClipboardRepository.Save` resolves duplicates by the complete content hash across the whole
  active/deleted history, not only against the immediately preceding row; recopying content revives
  and moves the existing record instead of creating another visible copy;
- `ClipboardRepository.MarkUsed` updates `last_used_at`, increments `use_count` and moves an
  activated entry to the front of its non-pinned order;
- popup ordering uses pinned state first and last-use/capture recency second;
- the old search path includes visible content and source application metadata.

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

Verified modifier-action contract:

- Ctrl, Alt and Shift remain the only configurable modifier slots;
- each slot can choose original paste, copy only, plain-text/path paste, line-break removal or disabled;
- multiple modifiers are unsupported;
- line-break removal remains text-only at execution time.

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
  -> C# captures PNG/DIB
  -> content-addressed fallback PNG is persisted
  -> C# rewrites clipboard with PNG/DIB + CF_HDROP

click image history item
  -> Rust classifies the captured target process
  -> rich editor: send PNG/DIB only and remove stale CF_HDROP
  -> terminal/file manager: send PNG/DIB + persisted image_path_fallback
  -> C# follows the typed include_file_drop_fallback flag
```

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

## Portable archive management

Verified old NxTools behavior in `ClipboardHistoryService.ExportArchive` / `ImportArchive`:

- `.nxtclip` is a ZIP archive with `manifest.json`;
- export includes active history and image bytes instead of retaining absolute image paths;
- import writes assets into the target storage root and merges through the normal deduplication path;
- SQLite maintenance creates a database backup before destructive migration and uses `VACUUM` to
  reclaim space.

NxTools.Next keeps that user-visible format concept, while also carrying native clipboard-format
files and source icons. Archive entry names are generated by Rust, imports reject unsafe or oversized
entries, and imported assets are content-addressed inside the configured clipboard asset directory.

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
