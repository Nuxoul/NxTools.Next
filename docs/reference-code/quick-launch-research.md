# Quick Launch / Finder reference index

Last verified: 2026-07-24.

## Scope and use constraints

This note records user-visible behavior, module seams and call chains for NxTools.Next's “速启”
feature. `SwiftList-main` is MIT-licensed source. The installed Listary build is proprietary and is
used only as a clean-room behavioral/reference source. `dnSpy.Console.exe` was used read-only to
inspect managed type organization; no installed binary was modified, no authorization was bypassed
and no proprietary implementation is copied into NxTools.Next.

Reference priority is explicit: verify the desired product behavior against **Listary first**, then
use **SwiftList second** as an open-source implementation reference. When their behavior differs,
NxTools.Next follows Listary's user-facing interaction unless a project requirement says otherwise;
SwiftList is used to inform independently written architecture and algorithms, not to override that
behavior.

## Reference roots

| Root | Status |
| --- | --- |
| `D:\Code\SwiftList-main` | Source and first-party documentation inspected |
| `D:\Program Files\Listary` | Installed file layout inspected |
| `D:\Tool\dnSpy-net-win64\dnSpy.Console.exe` | Used to decompile `Listary.exe` managed metadata/code into local diagnostics only |
| `D:\Code\NxTools.Next-main\.diagnostics\listary-managed` | Temporary investigation output; not a production dependency |

> 2026-08-31 迁移说明:项目移植到新机器后,参考项目已恢复,Listary 托管反编译输出已用
> `D:\Tool\dnSpy-net-win64\dnSpy.Console.exe` 按原命令重新生成(抽查 22 处文档引用路径全部命中)。
> 当前 `D:\Code\SwiftList-main` 快照的目录结构与首次核对时不同:`InstantAnswers`、`Indexing`、
> `QuickNav`、`Presenter`、`Helpers`、`Persistence`、`WindowAdapters`、`Commands`、`DriveMonitoring`
> 等子目录为新增层级;本文记录的 SwiftList 路径已按当前快照更新,但原始核对的行号引用仍来自旧
> 快照,复用某条结论前需按当前文件重新定位。

## SwiftList verified behavior and entry points

### Process and index architecture

| File | Verified responsibility |
| --- | --- |
| `Site\zh-CN\dev-guide\architecture.md` | Documents the LocalSystem index service, user-session UI and isolated hook process; the UI queries the service through a named pipe. |
| `Core\SearchEngine.cs` | Owns index initialization/status, separate cancellation sources for global and directory searches, streaming results and idle cache trimming. |
| `Core\Services\SearchService.cs` | Client-side named-pipe module; searches local index and network sources concurrently, deduplicates paths, filters hidden/system items and forwards cancellation. |
| `Core\Services\UsnServicePipeServer.cs` | Service-side request dispatch and streamed search responses. |
| `Core\IndexV2\*` and `Core\Indexer\Usn\*` | Memory-mapped snapshot, delta overlay and USN/MFT implementation; detailed algorithm review remains a phase-2 task. |

### Phase-2 indexing lifecycle verified before implementation

| File | Verified responsibility / invariant |
| --- | --- |
| `Core\SearchEngine.cs` | Keeps independent cancellation sources for global and directory-scoped searches. Starting a newer search cancels the previous search in the same channel; index initialization and rebuild run away from the interactive caller. |
| `Core\Services\UsnServicePipeServer.cs` | Hosts a bounded process seam for streamed search and status. Search responses are pumped separately from control requests, while status subscribers receive an immediate snapshot followed by changes. |
| `Core\Indexer\Usn\JournalReader.cs` | Opens each selected volume, resolves its root file reference, queries the journal, and only then builds the initial NTFS snapshot from the raw MFT. Failure to establish those prerequisites rejects that drive instead of publishing a partial index. |
| `Core\Indexer\Mft\MftIndexScanner.cs` | Reads NTFS volume geometry and the `$MFT` run list, applies FILE-record fixups, emits one row per non-DOS `$FILE_NAME`, and publishes a drive snapshot only after the scan finishes. |
| `Core\Indexer\Mft\MftParser.cs` | Bounds-checks FILE attributes, decodes non-resident run lists, and uses runtime `$FILE_NAME` offsets instead of assuming a fixed trailing layout. |
| `Core\Indexer\Usn\JournalReaderHelper.cs` | Incremental catch-up validates both journal identity and the cached USN range. A changed journal ID or an expired USN cursor requires a full rebuild. |
| `Core\UsnMonitor.cs` + `Core\UsnRecordParser.cs` | Live monitoring reads bounded `FSCTL_READ_USN_JOURNAL` batches from the last committed cursor, parses records by their runtime layout, applies a whole batch atomically, backs off when no records arrive, and requests a rebuild when the volume/journal can no longer resume safely. |
| `Core\Indexer\Usn\SearchCoordinator.cs` | Snapshots the active per-drive indexes under a short outer lock, then searches drives in parallel with bounded concurrency and a shared cancellation token; callbacks re-check cancellation before publishing. |
| `Core\IndexV2\LiveIndex.cs` + `Snapshot.cs` | Treats the immutable snapshot and live delta as separate authorities; journal position advances in live metadata and becomes snapshot metadata only after compaction. |

### Windows service deployment verified before the privilege fix

The installed Listary service was checked through the Windows Service Control Manager on 2026-07-20.
`ListaryServiceV2` is an automatic, running service whose account is `LocalSystem` and whose binary is
`D:\Program Files\Listary\Listary.Service.exe`. The ordinary Listary UI is therefore not proof that
raw-volume indexing works at medium integrity: the privileged work is isolated in a separate service.

The read-only managed inspection also confirms an important path-building invariant: Listary's
`DiskSearch` index receives an explicit root file reference and seeds that root into its folder map
before attaching enumerated children. It does not require the root directory itself to appear in an
MFT/USN enumeration batch. NxTools.Next independently applies the equivalent NTFS invariant by
recognizing MFT record number 5 (while preserving its sequence bits) as the volume root.

SwiftList makes the same boundary explicit in source:

| File | Verified responsibility / invariant |
| --- | --- |
| `Service\Program.cs` | `--service` enters `ServiceBase.Run`; `--install` creates an automatic service, grants authenticated users query/start/stop rights, then starts it. |
| `App\Services\ServiceManagement\ServiceInstallManager.cs` | The user-session app performs a one-time `runas` install, validates that the registered image path matches the current build, and subsequently starts the existing service without elevating the UI. |
| `Core\Services\UsnServicePipeServer.cs` | Keeps the index in the service and exposes search/status through a named pipe; the desktop is a client rather than the service owner. |

NxTools.Next follows these invariants independently: `NxTools.Indexer.exe --service` owns MFT/USN
state under LocalSystem; the C# Windows adapter owns only SCM/UAC installation and start operations;
the normal Rust/GPUI desktop connects through a versioned local named pipe and reconnects after either
side restarts. Service installation is the only elevated transition. The main desktop process must
never request elevation, and C# must never receive or interpret MFT records.

The protected desktop, PlatformHost and indexer are one release unit. A successful connection to an
already-running service proves only protocol availability; it does not prove that the protected
binaries match the package that launched the desktop. On the first PlatformHost connection in each
desktop lifetime, the Rust desktop therefore requests one package/service verification even when
the indexer pipe is already connected. `IndexerServiceInstaller.IsInstalledAt` compares the
registered and protected hashes for all three package binaries before taking its no-UAC fast path;
only a mismatch enters the one-time elevated deployment. A manual index refresh remains a separate
operation and must not be used as the release-version check.

The protected directory is a single local release slot, not a side-by-side version store. Before an
elevated update replaces that slot, it stops the indexer service and terminates only processes whose
resolved executable paths exactly match the protected `NxTools.exe` or
`NxTools.PlatformHost.exe`. The installer process itself is excluded, process exit has a hard
timeout, and an unowned same-name process is never terminated. Only after those owned executable
images have exited may the installer replace and revalidate the three binaries, repair SCM/registry
state and restart the service. This avoids both a locked-file partial deployment and an unnecessary
version-directory migration.

NxTools.Next carries these lifecycle properties forward independently: volume discovery produces a
real status before MFT work starts; the process protocol is bounded and versioned; every search batch
carries the caller's generation; and the desktop rejects a batch whose generation is no longer the
active query. The first phase-2 tracer bullet establishes that process/status/generation seam before
the raw MFT reader is attached to it.

For the phase-2 incremental slice, the immutable MFT entry array remains shared across searches and
USN polling publishes a small per-volume overlay atomically. Searches exclude tombstoned/replaced base
rows and include overlay rows from the same published generation. A directory rename or move also
re-resolves visible descendants. The live cursor advances only with the published overlay; a journal
ID mismatch or a cursor outside `LowestValidUsn..NextUsn` schedules a full snapshot rebuild instead
of guessing from an incomplete journal. One maintenance turn reads at most four 256 KiB journal
buffers, so a long offline catch-up cannot monopolize the protocol coordinator or grow an unbounded
record vector. The polling deadline is independent of request traffic and backs off from one to eight
seconds while idle. A failing volume does not prevent healthy volumes from advancing. Finally, an
overlay larger than `max(4096, base_entries / 10)` requests a fresh MFT snapshot, bounding long-run
clone, lookup and memory costs instead of allowing the delta to grow indefinitely. Directory
creates, moves, renames, deletes, parent-chain resolution and descendant expansion share a per-batch
4096-unit work budget and one incrementally maintained parent-to-overlay-children index; exceeding
that budget rebuilds instead of monopolizing the coordinator. Initial MFT
build errors and later USN polling errors are tracked independently, so a healthy volume cannot hide
another volume that never produced a snapshot.

### Phase-2 initial MFT snapshot slice

Microsoft's documented `FSCTL_ENUM_USN_DATA` boundary enumerates NTFS master-file-table records in
bounded batches. The first call starts at file reference zero; every response begins with the next
file reference for the following call. `USN_RECORD_V2` is variable length, so record length, file-name
offset and file-name byte length must all be validated at runtime. Opening the `\\.\X:` volume handle
requires administrative access on supported Windows versions.

NxTools.Next's first usable file-search slice uses that documented enumeration boundary to build one
immutable per-volume snapshot away from GPUI. It reconstructs paths from file/parent references,
atomically publishes only completed drive snapshots, and reports a per-drive failure rather than
falling back to a foreground directory walk. This API exposes the primary MFT name for each record;
expanding every hard link from raw `$FILE_NAME` attributes is not claimed by this slice.

`SearchEngine.SearchStreaming` cancels the previous search in the matching search channel before
starting a new one. `SearchService.SearchStreamingAsync` requests more candidates than the final UI
limit, merges multiple sources through a case-insensitive path set and propagates a cancellation
token. These are lifecycle invariants, not code to copy.

### Query and ranking

| File | Verified responsibility |
| --- | --- |
| `Core\SearchIndex\SearchQueryParser.cs` | Detects path mode, normalizes separators, extracts a target drive and distinguishes a trailing directory separator. |
| `Core\SearchIndex\SearchQuerySortParser.cs` | Parses query sorting/bypass controls. |
| `Core\SearchIndex\Fzf\*` | FZF-style fuzzy/exact matching, scoring, top-N collection and radix ranking. |
| `Site\zh-CN\user-guide\search-syntax.md` | First-party contract for fuzzy terms, exclusions, exact/prefix/suffix/OR operators, drive targeting, path mode and parent-folder filters. |

`Core\SearchIndex\Fzf\FzfResultRank.cs` was additionally verified for the launcher-ranking
corrective slice. Its default rank compares the match start, match span, candidate length and score
as separate packed fields; it does not treat a provider's one scalar score as a complete cross-source
rank. NxTools.Next keeps its own scoring implementation, but must likewise preserve enough headroom
before the final 50-result merge for result type and usage history to affect ordering.

NxTools.Next should stage this syntax. Phase 1 needs ordinary fuzzy application matching only;
advanced file-query operators belong with the indexed-file slice.

### Summon gesture

SwiftList's first-party `Site\zh-CN\user-guide\settings\hotkeys-page.md` and
`Core\Settings\UserSettingsModels.cs` define a bare `Ctrl` binding as a double-tap gesture and make
it the default. `Core\Hook\InlineSearch\GlobalHotkeyDetector.cs` verifies the important behavioral
guards: the modifier must be released between taps, key-repeat is ignored, an intervening key resets
the sequence and both taps must fall inside a bounded time window. NxTools.Next implements those
observable invariants independently in the C# Windows adapter; the hook callback only posts a
message, while foreground capture and IPC remain on the adapter's message loop.

### Popup lifecycle and keyboard interaction

| File | Verified responsibility |
| --- | --- |
| `App\Views\QuickSearchWindow\Helpers\QuickSearchWindowController.cs` | Captures the previous foreground HWND, toggles visibility, positions on the active display, requests foreground/topmost and waits for real focus before focusing input. |
| `App\Views\QuickSearchWindow\QuickSearchWindowInputHandler.cs` | Escape, Enter, arrows, action menu, selection completion and history navigation. It distinguishes WPF's `Key.ImeProcessed` Enter from an ordinary Enter and leaves the former to the IME instead of executing a result. |
| `App\Views\QuickSearchWindow\QuickSearchWindowResultExecutor.cs` | Records usage before executing results, hides the quick window and routes normal/admin execution. |
| `Site\zh-CN\user-guide\hotkeys.md` | First-party key behavior and quick-navigation contract. |
| `Site\zh-CN\user-guide\actions-and-preview.md` | Action menu and docked QuickLook behavior. |

Important invariants: a hotkey opens a focused standalone window; a stale foreground request cannot
silently eat immediate typing; selection remains visible; execution records history before hiding;
IME composition Enter must not execute the selected result.

## Listary managed implementation observations

The following facts were verified from the managed `Listary.exe` output produced by:

```powershell
dnSpy.Console.exe --no-sln --no-resources --no-resx --no-baml -o <diagnostics> Listary.exe
```

| Decompiled type/file | Verified observation |
| --- | --- |
| `Listary.Main.Views.LauncherSearchWindowFactory` | Creates a dedicated launcher search window from a `SearchBarViewModel`. |
| `Listary.Main.Views.LauncherSearchWindow` | Supports dragging with persisted X/Y; otherwise centers horizontally and places the launcher near the upper quarter of the cursor's display; owns search input, results and preview popup. |
| `Listary.Main.Views.SearchWindowBase` | Connects view-model show/hide/activate/close events, forces foreground, focuses input, rebuilds key bindings, scrolls selection into view and resets when deactivated if `HideWhenLosingFocus` is enabled. |
| `Listary.Main.Views.SearchWindowBase` + `Listary.Core.ViewModels.SearchBarViewModel` | A bare Right key at the end of the query and a result right-click both enter the same action-mode pipeline. The selected result is converted into a nested `SearchMode`; this is not a detached OS-style context popup. |
| `Listary.Core.Actions.Class465` + `Class468` + `Class472` | Builds the nested action search mode from first-class Listary commands and the target's Windows Shell commands. Actions remain searchable, Shell commands are grouped after the first-class actions, and submenu entries push another nested search mode. |
| `Listary.Core.ViewModels.SearchBarViewModel` | Action modes are kept on the search-mode stack. Escape or empty-query Backspace pops one mode and restores its parent query/selection context; execution uses the same selected-result pipeline as ordinary search results. |
| `Listary.Core.ViewModels.SearchMode` | A non-empty query starts a new cancellation-scoped search without first clearing the currently committed `Results`. The default non-incremental path awaits every provider with `Task.WhenAll`, re-checks cancellation, ranks the merged set and only then replaces `Results`. When the query becomes empty and `SearchWhenQueryIsEmpty` is false, the active search is cancelled and `Results` is set to `null`; `ShowResultList` is true only for a non-empty result collection. |
| `Listary.Core.Launcher.Class293` | The default launcher creates its multi-provider `SearchMode` without enabling incremental publication, so the last committed list remains visible until the next merged query completes. The separate web-suggestion mode explicitly enables incremental results with a 20 ms coalescing delay; that specialized setting is not the default launcher policy. |
| `Listary.Core.Launcher.Class293` | The default launcher constructs the final merge with a 50-result cap. Its indexed-file and history providers are also each configured for 50 results; the special prior-folder provider requests up to 100 candidates. Eight visible rows are a viewport choice, not the query/result cap. |
| `Listary.Main.Views.SearchItemList` | Selection changes call `ScrollIntoView`. Mouse movement selects a row only when the same row reports a second `MouseMove` at a different pointer position; entering a row or leaving the pointer stationary does not steal a keyboard selection. |
| `Listary.Core.Search.Class256` + `Class258` + `Listary.Core.History.HistoryKey` | Results from multiple providers are not merged by directly comparing provider-local scores. They are classified by history state and match quality, history is looked up through an item key that carries the used keyword, and only then are groups ordered and capped. The exact native/obfuscated comparer formula is not treated as available source. |
| `Listary.Main.Views.FileManagerSearchWindow` | Uses a separate surface for file-manager-integrated search. |
| `Listary.Main.Views.DialogSearchWindow` | Uses a separate surface/popup for file-dialog search rather than overloading the launcher window. |
| `Listary.MiniCore.IndexedSearchService` | Generated gRPC client exposes session open, search, result-detail retrieval and index status/rebuild operations. |
| `Listary.MiniCore.IndexedSearchSearchParams` | Query request includes session ID, query, max results, folder/file switches, priority/date filters, base-name matching and scoring mode. |

### Phase-3 preview lifecycle verified before implementation

| Reference entry | Verified responsibility / invariant |
| --- | --- |
| `Listary.Main.Views.LauncherSearchWindow` + `PreviewPopup` | The launcher owns one adjacent preview popup and nudges its horizontal offset when the launcher moves, so preview is part of the launcher session rather than a new window per result. |
| `Listary.Core.ViewModels.IFilePreviewViewModel` + `FilePreviewViewModel` | Preview exposes a lazily produced thumbnail plus name, path, size and timestamps; file-system metadata work is dispatched away from the UI callback. |
| `D:\Code\SwiftList-main\App\Services\QuickLookManager.cs` | Reuses one preview window, updates its target in place, docks on the right and flips left when needed, clamps the configured fixed size to the active monitor, and does not replay the entrance animation during target updates. |
| `D:\Code\SwiftList-main\App\Views\QuickLook\QuickLookWindow.xaml.cs` | A target change releases or re-points the current provider; hiding releases active content and stale target state. |
| `D:\Code\SwiftList-main\Plugins\CoreExtensions\Preview\DefaultPreviewProviders.cs` | Image and bounded text previews are separate lightweight providers with a metadata fallback. |

NxTools.Next intentionally differs in activation: this phase uses a bounded hover delay instead of
SwiftList's explicit `Alt+P` toggle. The invariants retained are one passive GPUI preview window,
background/bounded content loading (including encoded-file and decoded-pixel budgets for images),
single-read image validation to avoid path replacement races, explicit release of the previous GPUI
image asset on replacement/hide, metadata-only handling for animated GIF/WebP, generation
cancellation for stale hover work, docking inside the
active monitor, hiding while the result list scrolls, a short leave grace so the pointer can enter the
preview, and staging a reused window away from the display until its replacement frame is committed.
The preview never becomes a second search-state owner and never moves rendering into the C# adapter.

Creation and modification timestamps are read from the same background `std::fs::metadata` result
that supplies the preview's size and file-kind details. Each timestamp is optional: if Windows or
the backing file system cannot provide one value, that row is omitted without suppressing the other
metadata or failing the preview. Rust owns the metadata and local-time formatting; GPUI only renders
the resulting strings, and the C# platform adapter is not involved.

The installed Listary native engine remains opaque, so its on-disk layout is not claimed as reviewed.
The open SwiftList source provides the concrete secondary implementation reference for the same
lifecycle:

| SwiftList source | Verified observation |
| --- | --- |
| `Core/DriveMonitoring/DriveRecovery.cs` | Recovery opens a per-drive cache first, validates/catches up from its journal identity and cursor, and rebuilds only that drive when restore or USN catch-up fails. |
| `Core/Indexer/Usn/UsnIndexerCacheExtensions.cs` | A cache is accepted only when snapshot format and current volume identity match; an accepted snapshot becomes searchable before live monitoring starts. |
| `Core/IndexV2/Persistence/SnapshotFormat.cs` + `Snapshot.cs` | The base index is one versioned columnar file opened through a read-only memory map. Opening does not rebuild per-row strings or lookup dictionaries; sections page in on demand. |
| `Core/IndexV2/LiveIndex.cs` | The mapped immutable base and mutable delta have separate ownership. Empty deltas skip periodic compaction; a compaction atomically writes and swaps a fresh snapshot. |

This supports three architectural conclusions: launcher, file-manager and dialog search are distinct
surfaces; search uses a session-oriented process seam; and summary search/result-detail retrieval are
separate operations. Names that remain obfuscated are not treated as evidence of semantics.

## 阶段 5：网址即时结果纵向切片（2026-07-24 复核）

### 新核对的参考入口

| Reference entry | Verified responsibility / invariant |
| --- | --- |
| `.diagnostics/listary-managed/Listary/Core/Search/Class230.cs` | The default search mode recognizes absolute HTTP(S) URLs and a legacy-regex set of bare domains/IP addresses; bare targets are normalized to HTTPS and emitted as one browser-open result. |
| `.diagnostics/listary-managed/Listary/Core/Launcher/Class293.cs` | The URL provider is part of the ordinary merged launcher mode rather than a keyword-only nested web-search mode. |
| `.diagnostics/listary-managed/Listary/Core/ViewModels/Class178.cs` | URL/search results use a geography icon and execute away from the UI callback through the system URL opener. |
| `D:\Code\SwiftList-main\Plugins\CoreExtensions\Providers\InstantAnswers\WebUrlInstantProvider.cs` | The secondary open-source reference accepts trimmed explicit HTTP(S) URLs with a non-empty host, returns one typed instant result and opens it on Enter/click. |

NxTools.Next keeps Listary's useful bare-domain behavior but intentionally does not reproduce the
legacy regex's false positives. Rust performs bounded structured parsing, accepts only HTTP(S), and
normalizes a valid bare domain/IP to HTTPS; whitespace, credentials, unsupported schemes and invalid
host labels are rejected. The typed URL result participates in the ordinary merged launcher list,
sorts ahead of file results, and prevents the same URL query from reaching MFT/USN. GPUI renders the
result with a Material Symbols language icon. Activation reuses the existing validated
`OpenShellTarget` platform command, so C# remains only the Windows Shell adapter.

## 阶段 5：环境变量精确展开纵向切片（2026-07-25 复核）

### 新核对的参考入口

| Reference entry | Verified responsibility / invariant |
| --- | --- |
| `.diagnostics/listary-managed/Listary/Core/Commands/CustomCommandViewModel.cs` | The observable Listary managed boundary expands environment variables for configured command paths and arguments, but it does not expose a confirmable launcher environment-variable result provider. No native behavior is inferred from this boundary. |
| `D:\Code\SwiftList-main\Plugins\CoreExtensions\Providers\InstantAnswers\EnvironmentVariableInstantProvider.cs` | The secondary open-source reference recognizes `%NAME%`, resolves names case-insensitively, and returns typed instant actions without sending the query to file search. It also has a broader `%partial` enumeration mode, which is not part of this first NxTools.Next slice. |
| `D:\Code\SwiftList-main\Site\zh-CN\user-guide\instant-answers.md` | The documented public syntax distinguishes exact `%NAME%` expansion from partial-name enumeration and treats environment results as always-on instant answers. |

NxTools.Next's first environment-variable slice accepts exactly one complete, bounded `%NAME%`
token. Rust validates the token and reads the current process environment only after the token is
complete; ordinary queries do not enumerate environment variables. Following the reviewed SwiftList
behavior, Rust splits semicolon-delimited values into at most 64 deduplicated entries and classifies
each with one filesystem metadata read. Existing directories and files become typed `OpenTarget`
results; non-path and missing values remain typed `CopyText` fallbacks. GPUI uses the Google Material
Symbols `data_object` icon initially. Enter and click open valid paths through the ordinary Shell
target flow, while right-click exposes the applicable file actions; text fallbacks retain the sole
“复制变量值” action. C# only receives existing Shell-open or clipboard-write commands and has no
environment-variable parsing or UI responsibility. Unknown, empty, malformed, embedded and
incomplete tokens produce no instant result.
Any syntactically complete `%NAME%` query is reserved by the provider and does not reach MFT/USN,
even when the current process has no non-empty value for that name. A successfully resolved value
owns the Finder query and is not merged with coincidentally named file candidates.

## 阶段 5：计算器即时结果纵向切片（2026-07-24 复核）

### 新核对的参考入口

- Listary 托管边界
  `.diagnostics/listary-managed/Listary/Core/Extension/Implementations/Class411.cs` 把计算器注册为
  无需关键词且不显示预览的搜索扩展。它只在输入含数字、又不是一个可直接解析的纯数字时调用
  `Mages.Core.Engine.Interpret`；解释失败、结果不是有限 `double` 或表达式未完成时返回空集合，
  不把错误行混入普通结果。
- 同一 Listary 类型把计算结果包装成一个带数学图标的 `ISearchItem`；激活委托只把格式化后的结果
  写入剪贴板。字符串资源仍被混淆，因此本记录不推断其精确中文标题或小数格式。
- SwiftList 的 MIT 源码
  `Plugins/CoreExtensions/Providers/InstantAnswers/CalculatorInstantProvider.cs` 与
  `Plugins/CoreExtensions/Providers/InstantAnswers/ScientificMathParser.cs` 独立确认了相同的产品边界：即时结果先做严格数学字符/常量门禁，普通
  纯字母搜索不触发；支持括号、常用算术/科学函数和显式进制转换；无效、`NaN`、无穷大及“结果
  与输入相同”的查询不产生结果。
- SwiftList 一方文档 `Site/zh-CN/user-guide/instant-answers.md` 明确说明计算结果实时出现，
  `Enter` 把最终值复制到剪贴板。`App/Services/Plugin/PluginActionExecutor.cs` 进一步确认即时结果执行
  通过类型化 `Copy` 动作，而不是伪装成文件路径或 Shell 启动目标。

### 本纵向切片必须保持的不变量

1. **触发门禁**：用户输入必须以 `=` 显式进入计算模式；空白、纯数字、普通单词和未完成/非法
   表达式不生成计算项。引导符后的合法表达式至少包含一个数字或受支持常量，并且计算后的展示值
   不能与原输入等价。无 `=` 的数学字符仍按普通搜索处理，计算器不得抢占 `excel`、`calculator`
   等普通应用/文件查询。
2. **安全与有界**：Rust 使用无脚本能力的有界递归下降解析器，只接受明确的数字、运算符、括号、
   常量和函数集合；限制输入长度、嵌套深度和函数参数数量。不得执行代码、读取环境变量、访问
   文件系统或调用 C#。
3. **结果模型**：计算项是 Rust 搜索源返回的类型化即时结果。一个表达式可有序派生计算值、
   十六进制、二进制、中文数字和中文金额，每项携带独立副标题与可复制值；不适用的表示必须省略。
   结果不能伪装成 `.exe`、路径或任意 Shell 目标，并在有效表达式下排在普通文件结果之前。
4. **查询成本**：确认是完整合法表达式后不向 MFT/USN 文件索引发送同一查询；计算发生在内存中，
   不触发图标提取、预览读取、SQLite 或后台目录扫描。无效数学输入继续走普通统一搜索。
5. **交互**：计算行使用 Material Symbols 数学图标，主行显示当前表示值，次行显示
   `计算结果`、`十六进制`、`二进制`、`中文数字` 或 `中文金额`。普通 `Enter` 和单击复制当前选中行；
   成功后关闭面板，不记录为应用/文件启动历史。右键动作模式只暴露适用于即时结果的“复制结果”，
   不查询 Windows Shell 菜单。
6. **职责边界**：`nxtools-finder` 拥有识别、解析、格式化和类型化即时结果；GPUI 只负责展示、
   选择和把类型化动作交给应用；现有 C# 适配器只复用 Windows 剪贴板写入命令，不参与计算和
   UI 绘制。

## Explicitly not confirmed

- The native implementation inside `listary_engine.dll` and `listary-core.exe` was not decompiled or
  disassembled; its MFT/USN layout, ranking formula and cache format are unknown.
- Listary's exact global-hook implementation, licensing code and native injection behavior are out of
  scope.
- Decompiled string-resource identifiers are obfuscated, so hidden copy and precise labels were not
  inferred from numeric resource lookups.
- No claim is made that Listary's process protocol should be reproduced; only the observed separation
  of search sessions, summaries and details informs our own interface.

## NxTools.Next invariants carried forward

1. The actual search experience is an on-demand GPUI popup; the main window owns settings/status.
2. Rust owns catalog state, parsing, ranking, history and indexing; C# owns Windows hotkeys,
   foreground/Shell/COM adaptation; GPUI owns rendering.
3. Every query has a generation/cancellation identity; late results never replace a newer query.
4. Popup focus acquisition is verified against Windows state, not assumed after one scheduled frame.
5. Losing focus to an external window hides the popup, while owned action/preview surfaces do not.
6. Foreground and global-pointer notifications arrive while `QuickLaunchApplication` is leased.
   A nested `QuickLaunchPanel` update may only return a dismissal decision; it must never update
   the application again from that callback. The application performs the actual hide after the
   panel lease has ended.
7. Input and keystroke callbacks arrive while `QuickLaunchPanel` is leased. Actions that can write
   back into the panel (launch status and popup hiding) are deferred to the next GPUI update cycle.
8. Transient launch and error status is rendered as an absolute overlay. It never participates in
   the popup's column flow or steals height from the result list after an item is activated.
9. Result hover updates the same selected index used by Up/Down and Enter. Enter opens,
   Ctrl+Enter reveals a shortcut, and Ctrl+Shift+Enter emits a `run_elevated` intent; only the
   Windows adapter applies the `runas` verb and validates eligible targets.
10. Application launch is delivered as the first usable slice; MFT/USN, preview and file-dialog
   integration are later independent slices.
11. Opening the launcher shows the current in-memory catalog immediately and rescans application
   roots on the background executor. A completed snapshot replaces catalog entries atomically and
   refreshes only the current query, so filesystem traversal never blocks popup activation/input.
   A snapshot containing traversal errors is rejected, the last good entries stay searchable, and
   the error is surfaced in both the popup and the shared notification center.
12. Right-click and bare Right at the query end enter an in-window, searchable action mode for the
   selected result. First-class actions precede a grouped Windows Shell command section; nested Shell
   submenus push another action mode. Escape or empty-query Backspace returns to the parent search
   mode without losing its query/selection. A generic detached `ContextMenu` is not equivalent.

The phase-1 corrective slice implements the state transition and the three already-supported
first-class actions (`Open`, `Reveal`, `RunElevated`). Enumerating the grouped Windows Shell command
section and nested Shell submenus remains phase 3 work; phase 1 must not fake that behavior with a
    detached generic menu.
13. A newly summoned launcher has an empty query, no result collection and only the compact search
    surface. Typing a non-whitespace query expands the result surface; clearing it collapses the
    launcher again. Search-provider scores are normalized into one Rust-owned rank that gives an
    explicit-query history match priority, followed by textual match quality and source/type
    priority. Raw MFT and application-provider scores must never be compared as one scale.
14. Index-provider readiness is part of the active search lifecycle. If the user types while the
    service is reconnecting or rebuilding, the application catalog may answer immediately, but the
    transition back to `Ready` must resubmit the query currently visible in the popup with a fresh
    generation. Internal session invalidation on disconnect must not strand that visible query or
    require another keystroke before files and folders reappear.
15. A successful index-provider handshake means the connection owns a real search session. A
    reconnect must wait for the previous session boundary to release; it must never receive a false
    `HelloAck`/`Ready` pair followed by an immediate close. This is the local-pipe equivalent of
    Listary's persistent session IDs and per-query cancellation: session ownership and query
    generations are explicit, so a command cannot be accepted by a disposable busy response and
    silently lost.
16. Index freshness and index availability are separate states. Listary's search session remains
    usable while its index service performs maintenance; NxTools.Next must likewise keep the last
    good snapshot online during USN recovery, volume rediscovery and background MFT rebuilds.
    `VolumesReady` only blocks file queries when no snapshot has ever been published. Rebuilding a
    newer snapshot must atomically replace the old one after completion rather than clearing search
    availability at rebuild start.
17. The launcher keeps up to 50 merged results behind a scrollable viewport. Up/Down always reveals
    the selected row. Mouse input may take over selection only after real pointer movement within a
    row; a stationary pointer must not undo keyboard navigation after layout or scrolling.
18. A bounded USN read batch is normal incremental work, even when it contains more than 4096 direct
    file changes. Direct records are applied atomically to the live overlay. The bounded-expansion
    guard is reserved for operations that recursively visit a large directory subtree; a resulting
    background compaction is maintenance, not a partial index failure, and leaves the previous
    searchable snapshot online until replacement.
19. Service restart and desktop reconnect are restore operations, not rebuild triggers. A versioned,
    atomically-written, per-volume snapshot is validated against the current volume and journal,
    opened without materializing millions of path strings, and caught up from its committed USN
    cursor. Only a missing, incompatible, replaced-volume or expired-journal snapshot schedules a
    rebuild, and that rebuild is scoped to the affected volume.
20. Release verification for the indexer includes restart-to-first-client CPU and working-set probes.
    Protocol `Ready` and successful search are insufficient: a stable idle service must consume near
    zero CPU, and loading an existing snapshot must not repeat full MFT enumeration.
21. A non-empty query never publishes an empty pending result set over the last committed rows.
    The current generation may cancel older provider work, but its merged results replace the visible
    list only after publication. Clearing the query is the explicit operation that collapses and
    clears the result surface.
22. Query-to-query result replacement does not replay entrance motion. The launcher uses Zed's
    semantic dark roles rather than one accent color for every surface: the window border is quiet,
    ordinary rows are unboxed, the selected row uses a neutral element-selection fill, and real
    Shell icons are not placed inside a decorative colored tile. Motion is reserved for the single
    collapsed-to-expanded transition.
23. The phase-2 query contract follows the verified SwiftList syntax surface: whitespace terms are
    ANDed, `|` creates alternatives, `!` excludes, apostrophe-delimited phrases require word
    boundaries while an unclosed apostrophe requests exact substring text, and `^`/`$`
    anchor the file name, drive/path/trailing-separator modes constrain location, and `::` constrains
    the parent. A leading `*` bypasses user exclusions for one query but never bypasses the selected
    volume scope. Parsing and scoring remain one Rust-owned module shared by indexed files and the
    merged launcher catalog.
24. Index settings are durable desktop configuration and an explicit indexer protocol command.
    Receiving the same settings on reconnect is a no-op for build scheduling; a build is scheduled
    only when an enabled volume lacks a searchable snapshot. Excluded paths are enforced at the
    service search boundary, not only hidden by GPUI. The LocalSystem service persists its own copy
    before the next restart so disabled volumes are not scanned during the desktop reconnect race.
25. Overlay size is a maintenance trigger, not a failed journal transaction. The newest USN batch is
    published first, a worker folds replacements/deletions into a new immutable base, and journal
    polling pauses until that worker finishes. Search continues against the published snapshot, and
    a compacted base is accepted only when its journal ID and cursor still match. One poll schedules
    every oversized volume; one volume can never overwrite another volume's maintenance request.
26. File search workers partition results into bounded ordered batches. Only the last batch is marked
    complete; the desktop accumulates matching generations and publishes the merged application/file
    list when the complete batch arrives, so partial transport does not reintroduce result flicker.
27. Incremental compaction retries are isolated per volume with bounded exponential backoff. The
    retry timer never postpones the shared USN poll; successful rebuild/compaction or new USN
    progress clears that volume's compaction fault.
28. Launcher merging keeps query-specific history first, then gives strong launchable candidates a
    bounded advantage over ordinary indexed paths without promoting weak incidental app matches over
    every strong file match. After user testing showed the 50-row presentation cap hid too many
    otherwise valid matches, the virtualized product list retains up to 200 merged rows; final
    cross-source ranking and truncation happen only in `nxtools-finder`.
29. The result viewport and Up/Down navigation share the same GPUI `ScrollHandle`. Pointer movement
    may change selection after the movement gate opens, but it never rewinds the viewport to the top.
30. The observed Listary `gram` result keeps the launchable `AyuGram.exe` and its clean, shallow
    `D:\Tools\AyuGram` parent ahead of exact-name hits inside generated cache/dependency trees.
    NxTools.Next therefore applies a bounded generated-path penalty after textual scoring instead
    of globally weakening exact matches. The visible result name emphasizes the same contiguous or
    fuzzy characters that admitted it; ordinary and selected rows keep one semantic selection fill.
31. Windows `ERROR_JOURNAL_ENTRY_DELETED` (`1181`) means the saved USN cursor cannot be read any
    longer. It enters the existing affected-volume rebuild path instead of becoming a permanent
    generic polling error; the last searchable snapshot remains available until replacement.
32. Index scope is edited as state, not as semicolon-delimited text. GPUI renders discovered NTFS
    volume toggles and removable exclusion entries, Rust persists the resulting vectors, and the
    C# adapter owns only the native folder picker. An empty volume vector continues to mean
    “automatically include every discovered NTFS volume.”
33. Result actions stay inside the launcher's searchable nested mode. Listary's managed boundary
    places first-class actions before grouped Windows Shell commands and includes copy-path as a
    built-in action; SwiftList independently exposes open, locate, copy-path and elevated-open as
    separate actions. NxTools.Next keeps action availability and completion policy in Rust, uses
    the existing copy-only clipboard command for copy-path, and leaves C# responsible only for the
    Windows clipboard/Shell operation. Copy-path keeps the launcher available and reports its
    completion in the active popup; launch-like actions record activation and close only after a
    correlated successful platform result. Any failed action preserves the nested mode for retry.
34. The action mode is not a fixed built-in list. Listary creates one `IContextMenu` session for the
    selected filesystem target. `Class472` asks `Class477.method_1` for only the root `HMENU`;
    `Class477.Class478` sends `WM_INITMENUPOPUP`, reads separators/disabled state, converts a nonzero
    `MENUITEMINFO.hbmpItem` to the row icon, and records the submenu handle plus its parent index
    without recursively enumerating it. Only after the user enters that row does `Class473` call the
    same method for that submenu, and `ContextMenuSearchItemViewModel` pushes the returned level as a
    nested searchable mode. Invocation continues through the same live session. NxTools.Next mirrors
    this boundary independently: C# owns the STA COM session and converts Shell-owned `HBITMAP`
    values through the same WPF `Imaging.CreateBitmapSourceFromHBitmap` boundary used by Listary and
    SwiftList (rather than `System.Drawing.Image.FromHbitmap`, which discards menu-icon alpha), stages the bitmap, and
    exposes opaque submenu ids; Rust owns per-level loading generations, filtering and correlated
    execution; GPUI renders the merged rows and real icons. Shell enumeration is lazy per entered
    level and never becomes an idle polling loop. The implementation details above are observations
    of Listary's managed boundary, not a claim that its native search engine source was available.
35. Entering the root action mode does not incrementally expose NxTools built-ins before the Shell
    provider completes. Listary's action `SearchMode` registers both first-class and context-menu
    providers, and its normal non-incremental search path awaits providers before replacing the
    committed result collection. NxTools.Next therefore renders a bounded loading state, then
    atomically publishes built-ins plus the completed root Shell level; a failed Shell query falls
    back to the built-ins. Nested Shell levels follow the same pending/commit rule.
36. File-result icon loading is tied to presentation, not selection. Listary's
    `DynamicIconSizeSearchItemViewModelBase.IconSize` setter installs a default icon and starts the
    asynchronous real-icon load when a result receives its presented icon size; pointer hover is
    not the trigger. NxTools.Next initially warms the first viewport and, after scrolling, requests
    the current visible result range with bounded overscan. Hover remains only a retry fallback.
37. A missing path still receives a stable file-type icon. Listary's `Class141.method_2` asks
    `SHGetFileInfo` for the path extension with `FILE_ATTRIBUTE_NORMAL` (or the folder type with
    `FILE_ATTRIBUTE_DIRECTORY`) instead of requiring the indexed path to still exist. NxTools.Next
    likewise prefers an existing target or resolved shortcut, then falls back through
    `SHGFI_USEFILEATTRIBUTES`; stale versioned paths therefore keep their `.png`, `.exe`, and other
    Windows type icons without relying on pointer hover.
38. Query work is cancelled/coalesced before publication. Listary's normal launcher replaces its
    committed collection only after its providers and ranking stage complete, while each new query
    cancels the previous search scope. NxTools.Next preserves committed rows during its 65 ms input
    coalescing window and rejects older index generations. A non-empty one-character query still
    has full file/folder semantics; its cost must be controlled by the index data layout rather than
    by silently skipping the indexed-file provider.
39. The indexed-search boundary is session-oriented and pageable. In the inspected managed
    Listary boundary, `Class387.method_23` searches into a stable session and returns a
    `SearchResultsSummary` with `TotalCount`; `Class387.GetSearchResults` then requests an explicit
    `(offset, length)` range. The launcher may cap its initial merged set, but the engine does not
    confuse viewport size, initial result count and total match count.
40. SwiftList's MIT-licensed `IndexV2` supplies the inspectable hot-path reference: `Snapshot`
    separates hot name tables from cold paths/metadata, assigns rows to unique UTF-8 names and
    stores precomputed ASCII/name masks; `SearchMatcher` evaluates unique names (not every full
    path), applies the mask/ASCII fast path, cancels between bounded chunks and reuses worker,
    scratch and hit buffers. Only ranked hits reach `ResultBuilder`, where cold paths are materialized.
41. The GPUI presentation boundary is the virtual list's rendered range. Zed's completion list uses
    `uniform_list` with `cx.processor`, records the supplied `Range<usize>`, and navigates through the
    matching uniform-list scroll handle. NxTools.Next must request icons for that rendered range
    (with small overscan) for initial layout, wheel scrolling, scrollbar dragging and keyboard
    navigation; pointer hover is never required to make an icon appear.
42. Reuse happens at the resource and batch boundaries, not by recycling stateful GPUI row entities.
    Ordinary indexed files share one cached Shell resource per extension, directories share the
    folder resource, target-specific executables/shortcuts are coalesced while in flight, and only
    the virtual list's presented rows receive entry-to-resource bindings. Search workers rent and
    return bounded lightweight name-candidate vectors per Rayon thread; complete path strings are
    not retained in that pool.
43. The persisted name mask, name offset and UTF-8 name sections form one contiguous search hot
    range before the record and path sections. Windows receives a best-effort prefetch hint for only
    that range when a cached snapshot opens, while the service working set remains reclaimable. A
    name-only query scores this hot data first, keeps bounded ranked headroom, and materializes cold
    records/paths only for those candidates.
44. Result-row pointer callbacks execute while `QuickLaunchPanel` is leased by GPUI. They may update
    selection immediately, but every application-level icon request is scheduled onto the next GPUI
    application turn. A synchronous `QuickLaunchApplication::request_icons` can find a cached icon
    and write it back through `popup.panel.update`, re-entering the same panel lease and terminating
    the process with `double_lease_panic`. The presentation-range loader and hover retry therefore
    share the same deferred application boundary.

## Phase-1 implementation mapping

| NxTools.Next file | Preserved invariant |
| --- | --- |
| `rust/crates/nxtools-finder/src/lib.rs` | Catalog, fuzzy score and activation weights remain Rust-owned and memory-backed on the query path. |
| `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.cs` | Double-Ctrl/combination-key Windows adaptation captures the pre-popup context; activation can be suspended while editing. |
| `csharp/NxTools.PlatformHost/Windows/ShellIconExtractor.cs` | Shell icon extraction remains a bounded Windows adapter fact, not a ranking input. |
| `rust/apps/nxtools-desktop/src/platform_events.rs` | One receiver broadcasts platform facts to independent application modules. |
| `rust/apps/nxtools-desktop/src/quick_launch.rs` | One standalone popup HWND is shown, hidden and reused; GPUI owns layout and input state. |

The phase-1 query is synchronous over a small in-memory application catalog, so there is no late
asynchronous batch capable of replacing a newer query. The generation/cancellation contract becomes
active when phase 2 introduces the separate MFT/USN index process.

The staged implementation plan is in [`../quick-launch-plan.md`](../quick-launch-plan.md).

## 阶段 4：资源管理器上下文导航（2026-07-23 复核）

### 新核对的参考入口

- Listary 托管边界：`.diagnostics/listary-managed/Listary/Core/HookedWindows/Implementations/Class370.cs`
  将匹配的宿主分类为 `WND_EXPLORER`。其本地导航引擎仍属于不可见的专有边界，因此这里只记录
  可观察的宿主分类和产品行为，不声称掌握 Listary 原生实现。
- SwiftList `App/Services/ExplorerLocateHelper.cs`：通过 Shell Windows COM 集合按精确 `HWND`
  找到既有资源管理器窗口；目录使用该窗口自己的 `Navigate2`，不会启动第二个 Explorer。
- SwiftList `App/Services/ShellMenu/QuickNav/QuickNavigationNavigator.cs`：目标 HWND 必须在弹出辅助 UI 前捕获；
  执行时重新读取前台窗口会被辅助窗口自身的焦点变化污染。
- SwiftList `App/Views/InlineSearchWindow/Helpers/InlineSearchNavigator.cs`：索引结果的目录类型是动作
  决策依据，但执行前仍重新确认路径存在；成功导航后才隐藏搜索面板。
- Listary `.diagnostics/listary-managed/Listary/Core/Actions/Class465.cs`：动作模式仅要求结果实现
  `IPathItem` 且 `GetPath()` 返回非空路径，不按“应用、快捷方式、文件、文件夹”等搜索来源做白名单。
  `.diagnostics/listary-managed/Listary/Core/Search/Class241.cs` 进一步确认开始菜单/桌面快捷方式
  结果同样继承 `IPathItem`；其路径能力会进入与普通索引文件相同的动作管线。
- Listary `.diagnostics/listary-managed/Listary/Core/Commands/Class424.cs` 与
  `Core/HookedWindows/Class342.cs`：`IOpenFolder`、`IOpenFolderAndSelectFile` 和 `IOpenFile`
  是彼此独立的宿主能力。对非文件夹结果提供“切换到此目录”不等于执行或提交该结果。
- SwiftList `App/Services/ShellMenu/QuickNav/QuickNavigationNavigator.cs`：文件对话框收到非目录结果时，
  先用 `Path.GetDirectoryName` 转为父目录，再发送 `NavigateDialog`；不会把完整文件路径提交给
  打开/保存按钮。
- SwiftList `Core/Hook/ExplorerIntegration/ExplorerTracker.cs` 与
  `Site/zh-CN/dev-guide/sdk/system-adapters.md`：宿主 HWND、焦点子 HWND 和当前路径是同一份上下文，
  不能在不同时间分别采集。
- Windows SDK `OleCtl.h` 将 `DISPID_HWND` 定义为 `-515`，`ExDisp.h` 则确认 Explorer Shell
  窗口实现 `IWebBrowser2`（IID `D30C1661-CDAF-11D0-8A3E-00C04FC9E26E`）及 `Navigate2`。
  在当前 Windows 11 Explorer 上，.NET 8 `dynamic.HWND` 的名称查找会稳定返回
  `0x8000FFFF`；平台适配器因此使用带固定 `DispId(-515/500)` 的托管 COM 接口，不能退回
  `dynamic` 名称分派。

### 本纵向切片必须保持的不变量

1. **布局**：普通搜索结果布局不变；只要呼出上下文支持目录导航，且结果能够安全解析为本地目录，
   动作菜单就增加“切换到此目录”。文件夹使用自身；应用、EXE 和普通文件使用父目录；快捷方式
   先解析真实目标，真实目标是目录时使用自身，是文件时使用父目录。来源类型不能成为动作白名单。
2. **生命周期**：双击 Ctrl 时用原生窗口类冻结资源管理器 HWND，面板复用时覆盖旧上下文；本切片
   不在热键路径同步读取 Shell COM/磁盘，当前目录留给后续路径缓存切片。执行成功后才按普通启动
   动作隐藏，失败则保留面板并显示可诊断错误。
3. **取消**：关闭面板或开始新一次呼出不会让旧上下文泄漏到新会话；该动作没有后台搜索代次。
4. **位置**：导航发生在呼出时的同一个 Explorer HWND，不创建新窗口，也不退化为“打开目录”。
5. **焦点**：执行动作时不得重新读取当前前台窗口；GPUI 面板已经成为前台窗口是正常状态。
   `Navigate2` 成功后，C# 适配器必须在同一条平台命令返回前恢复呼出时捕获的 Explorer HWND
   为前台窗口。否则面板隐藏后焦点会回到非 Explorer 窗口，下一次呼出无法再次获得
   `directory_navigation_context`，表现为只能切换一次。
6. **失败状态**：目标 HWND 消失、不是 Shell Explorer 窗口、目录已删除或 COM 导航失败，都返回相关
   命令失败；不得静默导航另一个窗口。
7. **职责边界**：Rust 保存带宿主标签的会话上下文并编排统一 `NavigateDirectory` 动作；C# 用
   Win32/COM 选择并执行 Explorer 宿主适配器；GPUI 只渲染上下文动作。文件对话框及后续第三方
   文件管理器复用同一协议，不为每个 Windows 宿主扩张 Rust 命令集。

### 阶段 4：原生打开/保存对话框纵向切片（2026-07-23 复核）

#### 新核对的参考入口

- 旧 `D:\Code\NxTools` 未找到速启、文件对话框宿主识别或目录导航入口；本切片没有可迁移的
  旧行为实现。
- Listary 托管边界 `.diagnostics/listary-managed/Listary/Main/Views/DialogSearchWindow.cs` 与
  `DialogSearchWindowFactory.cs` 表明文件对话框使用独立的搜索窗口形态，并按宿主提供的放置边界
  定位；它不是普通启动器结果窗口。
- Listary 托管边界 `.diagnostics/listary-managed/Listary/Core/HookedWindows/Class342.cs` 与
  `Core/Plugins/FileAppPlugin/Class278.cs` 把文件宿主能力拆成 `IGetFolder`、`IOpenFolder`、
  `IOpenFolderAndSelectFile` 和 `IOpenFile`。本记录只确认托管职责边界，不声称掌握其原生对话框
  导航实现。
- SwiftList `PluginSdk/Abstractions/Plugins/WindowAdapters/IFileDialogAdapter.cs` 将宿主识别、当前路径、目录导航、
  停靠边界和焦点恢复定义为同一适配器能力；`FileDialogAdapterRegistry.cs` 按宿主选择适配器。
- SwiftList `Plugins/CoreExtensions/FileDialog/StandardFileDialogAdapter.cs` 使用顶层 `#32770` 加
  `Breadcrumb Parent` 区分现代 Windows 文件对话框；`ClassicFileDialogAdapter.cs` 对同类名额外要求
  标准文件名控件和类型下拉框，避免把注册表搜索等普通 `#32770` 弹窗误识别为文件对话框。
- 2026-07-23 在本机真实 `OpenFileDialog` 上用发布版
  `Win32FileDialogWindowOperations` 复现：宿主识别成功，但向文件名 `Edit` 写入目录后提交会稳定超时，
  当前目录仍停留在桌面；先激活原对话框、发送 `Ctrl+L`、取得该对话框线程新获得焦点的地址栏
  `Edit`，再写入目录并提交则可稳定切到目标目录。现代打开/保存对话框因此必须走地址栏导航，
  不能把文件名输入框当成通用目录导航接口；经典对话框保留文件名控件回退。
- SwiftList `App/Services/ShellMenu/QuickNav/QuickNavigationNavigator.cs` 与
  `App/Views/InlineSearchWindow/Helpers/InlineSearchNavigator.cs` 在辅助 UI 出现前冻结对话框 HWND；
  目录结果导航到该目录，文件结果只允许导航到父目录，绝不替用户提交“打开/保存”。

#### 本纵向切片必须保持的不变量

1. **宿主识别**：只接受具备文件对话框结构标记的顶层 `#32770`；类名相同但缺少结构标记的普通
   对话框不得获得导航上下文。
2. **上下文冻结**：热键请求携带宿主种类、顶层 HWND 与可选当前目录；动作执行不得重新读取当前
   前台窗口。Explorer 与文件对话框上下文互斥并由同一 Rust 会话保存。
3. **动作范围**：只要呼出上下文支持目录导航，目录结果和能够安全解析到本地文件系统的非目录
   结果都增加“切换到此目录”。目录使用自身路径；应用、EXE、普通文件使用目标父目录；`.lnk`
   必须先由 C# 的 Windows Shell/COM 边界解析真实目标，真实目标是目录时使用自身，是文件时使用
   父目录，不能跳到开始菜单中 `.lnk` 自身所在目录。无法解析为本地文件系统路径的 URL、
   AppUserModelId 或命令标识不显示该
   动作。绝不把完整文件路径写进打开/保存对话框，避免替用户自动确认文件。C# 在动作执行边界
   重新验证最终目录仍存在；解析只发生在选中结果进入动作模式时，不在结果列表逐项渲染热路径访问
   磁盘。
4. **原窗导航**：C# 仅向捕获的文件对话框控件写入已验证存在的目录并提交目录导航；不启动新的
   Explorer，不向另一个 `#32770` 重定向。现代对话框先恢复原 HWND，再通过 `Ctrl+L` 打开其
   地址栏，并校验最终焦点仍是该对话框内部的非文件名 `Edit`；这条链路不得改写“文件名”内容。
   经典对话框才使用标准文件名控件作为兼容回退。
5. **焦点与连续性**：导航提交前恢复捕获的对话框及其路径/文件名编辑控件焦点；成功后该对话框
   成为前台，从而下一次呼出仍能捕获同一宿主并连续切换。
6. **失败状态**：目标目录消失、HWND 失效、宿主结构已变化、控件写入或导航消息失败时保留速启
   面板并返回可诊断错误。Enter 必须进入目标对话框自己的消息队列；C# 随后通过面包屑或
   `CDM_GETFOLDERPATH` 在有界时间内确认当前目录，不能把 `PostMessage` 仅成功入队当成导航成功。
   失败结果与 Windows 前台事件来自不同生产者，Rust 会在重新激活面板期间短暂抑制该目标 HWND
   的迟到前台事件，避免面板刚显示错误又被误隐藏。
7. **职责边界**：Rust 定义通用目录导航上下文、会话和动作，并只接受 C# 返回的最终目录；
   C# 解析快捷方式/系统可执行文件并分派 Explorer/文件对话框适配器，拥有全部 Win32/COM 操作；
   GPUI 只渲染同一个“切换到此目录”动作。正常“打开”仍使用原始 `.lnk`/应用目标，不能因目录
   导航解析而改变启动语义。

### 阶段 4：桌面上下文与最近目录纵向切片（2026-07-23 复核）

#### 新核对的参考入口

- 旧 `D:\Code\NxTools` 未找到速启、桌面宿主识别或最近目录入口；本切片没有可迁移的旧实现。
- Listary 托管边界
  `.diagnostics/listary-managed/Listary/Core/HookedWindows/Implementations/Class352.cs` 只接受两种
  Shell 桌面窗口类并创建 `Class328`；`Class328.cs` 将宿主明确标为 `WND_DESKTOP`，再沿桌面宿主
  查找列表控件并使用鼠标所在显示器的工作区。专有本地导航引擎不可见，因此这里只记录宿主分类和
  可观察行为，不声称掌握其原生实现。
- SwiftList `Core/Hook/ExplorerIntegration/ExplorerNativeHooks.cs` 将 `GetShellWindow()` 和
  `Progman` 识别为桌面；`WorkerW` 只有包含 `SHELLDLL_DefView` 子窗口时才算桌面，避免把普通
  WorkerW 误识别。`ExplorerWindowClassifier.cs` 在同一次前台分类中冻结 HWND 与 `isDesktop`。
- SwiftList `Plugins/CoreExtensions/Providers/Indexing/ExplorerPathCollector.cs` 对桌面返回
  `Environment.SpecialFolder.Desktop`，不向 Shell COM 查询一个虚假的 Explorer 当前目录。
  `ExplorerInlineSearchAdapter.cs` 在桌面执行目录结果时通过 Shell 打开目标；只有普通
  `CabinetWClass` 才在原窗口内 `Navigate2`。
- SwiftList `App/Views/QuickSearchWindow/Helpers/QuickSearchWindowController.cs` 在 `ShowWindow` 时保存
  `_lastActiveHwnd`，普通 `HideWindow(true)` 隐藏后立即用 `SetForegroundWindow` 恢复该窗口；
  `QuickSearchWindowInputHandler.cs` 的 Escape 走这条恢复链路。只有即将打开另一个产品窗口或
  执行会接管前台的动作才调用 `HideWindowNoRestore`。这使 Escape 后的下一次全局热键仍能从真实
  宿主采集上下文，而不是从刚隐藏的辅助窗口采集。
- Listary `Core/Menus/Class284.cs`、`Class285.cs` 把历史作为快速导航菜单的独立分组，并与当前
  已打开目录分开。其托管历史存储细节不能完整确认。
- SwiftList `Core/SearchHistoryStore.cs` 对同一路径去重，成功执行时更新最近时间，并从持久化
  JSON 建立内存排序快照；`Plugins/FolderCascader/Navigation/MenuBuilderContentExtensions.cs` 只从该快照取最近 30 项，
  `MenuBuilder.cs` 延迟到用户展开“历史”层级时才生成菜单，不在普通结果逐行渲染时读磁盘。

#### 本纵向切片必须保持的不变量

1. **桌面识别**：仅接受 Shell 自身的 `Progman`，或带 `SHELLDLL_DefView` 的 `WorkerW`；
   `GetShellWindow()` 可作为额外强证据。不能仅凭任意 `WorkerW` 类名建立导航上下文。
2. **桌面语义**：桌面上下文使用用户的真实 Desktop 目录和呼出时的桌面 HWND。桌面不是可被
   `Navigate2` 改变目录的 Explorer 窗口，因此“切换到此目录”在此宿主中交给 Windows Shell
   打开目标目录；参考实现不返回可靠的新窗口 HWND，由 Shell 决定激活或复用窗口，不能加入脆弱
   的前台轮询，也不能把桌面错误分派给 Explorer COM 导航。
3. **上下文冻结**：桌面、Explorer、文件对话框三种宿主互斥，均在热键请求进入平台事件前冻结；
   弹窗出现后不得重新读取前台窗口。每次重新呼出覆盖旧宿主和 HWND。
4. **最近目录来源**：只记录已确认成功的目录打开/切换，以及由受支持宿主明确捕获到的有效当前
   目录；失败动作、相对路径、文件路径和不存在目录不得进入历史。
5. **持久化与排序**：Rust 对规范化后的 Windows 目录路径做大小写不敏感去重，保留展示路径、
   访问次数和最近访问时间，按最近时间优先、次数作为稳定次序，并设置有界容量。C# 不保存产品
   历史；GPUI 不拥有历史状态。
6. **热路径**：SQLite 只在记录或首次加载时访问；目录存在性校验和记录事务进入后台任务，完成后
   只把聚合记录合并回 Rust 内存快照。弹窗会话变化只能阻止旧面板刷新，不能丢弃已经捕获的有效
   历史。普通搜索、结果渲染、滚动和图标复用路径不得逐项调用 `Directory.Exists` 或查询数据库。
7. **交互**：默认呼出仍只显示紧凑搜索框，不因最近目录恢复而自动展开结果。最近目录通过搜索栏
   右侧的独立历史入口进入同一个虚拟化列表；列表支持过滤、键盘选择、滚动和直接导航，退出后恢复
   普通搜索。没有目录导航上下文或历史为空时不显示该入口。
8. **职责边界**：`nxtools-finder` 提供记录和读取最近目录的深模块接口；Rust 应用在平台命令成功
   回执后提交记录；C# 只识别桌面和执行宿主动作；GPUI 只渲染历史入口及列表。
9. **显式关闭连续性**：Escape 和再次按呼出热键属于显式取消，必须在隐藏/取消置顶后恢复本会话
   捕获的顶层宿主 HWND；成功执行结果、点击外部窗口或系统前台已切换时不得抢回旧焦点。恢复动作
   由 Rust 根据关闭原因编排，C# 只执行 HWND 激活。这样第二次呼出会重新采集同一个 Explorer、
   桌面或文件对话框上下文，目录动作不会消失。

### 阶段 4：第三方文件管理器纵向切片（2026-07-23 复核）

#### 新核对的参考入口

- 旧 `D:\Code\NxTools` 未找到速启或第三方文件管理器导航入口；本切片没有可迁移的旧实现。
- Listary 安装目录的 `FileAppPlugins` 证明其文件管理器集成是按宿主能力拆分的适配器体系；
  `OneCommander/ListaryPluginManifest.json` 明确声明独立的 OneCommander 文件应用插件。Listary
  专有插件的原生导航细节不可见，因此这里只记录可观察的宿主隔离边界，不复制其实现。
- SwiftList `PluginSdk/Abstractions/Plugins/WindowAdapters/IInlineSearchAdapter.cs` 将宿主识别、当前目录读取和结果
  执行定义成适配器能力；`Core/Hook/Commands/InlineAdapterCommandHandler.cs` 在宿主同权限的 Hook 进程内
  执行这些操作，并要求 COM/UI Automation 适配器在 STA 上运行。
- SwiftList `Plugins/TotalCommander/TotalCommanderInlineSearchAdapter.cs` 与
  `Win32/Win32Helper.cs` 只接受 `totalcmd*` 进程的 `TTOTAL_CMD` 顶层窗口，使用 Total Commander
  已公开的 Unicode `WM_COPYDATA` 查询取得活动源面板路径，再用 `CD` 命令及 `S` 标记只切换活动
  面板；所有跨进程消息都必须有界，不能因宿主卡死而阻塞速启。
- SwiftList `Plugins/DirectoryOpus/DirectoryOpusPathCollector.cs` 与
  `DirectoryOpusInlineSearchAdapter.cs` 只接受 `dopus` 进程的 `dopus.lister`，从可见
  `dopus.filedisplaycontainer`/`dopus.ctl.treepath` 识别活动面板，并通过 Directory Opus 的
  `DOpus.ParentWindow` 命令通道执行 `Go`。2026-07-23 在本机正在运行的 Directory Opus 上只读
  复核了这些真实窗口类；可见 `dopus.filedisplaycontainer` 的标题是当前目录。
- SwiftList `Plugins/OneCommander/OneCommanderPathCollector.cs`、
  `OneCommanderInlineSearchAdapter.cs` 与 `Automation/UiaPathAccessor.cs` 以进程名
  `OneCommander` 为强身份，使用 UI Automation 的 `CurrentPathGet`/`CurrentPathSet` ValuePattern
  读取和设置当前面板。呼出前必须保存焦点元素作为活动面板锚点；弹窗取得焦点后再全窗盲找会误中
  多面板中的第一个控件。

#### 本纵向切片必须保持的不变量

1. **统一能力**：Rust 协议只增加一个 `third_party_file_manager` 宿主能力，不认识 Directory Opus、
   Total Commander 或 OneCommander 的私有细节；C# 注册表根据冻结 HWND 决定具体适配器。
2. **严格识别**：进程名与顶层窗口类/结构必须同时满足。不能仅凭标题或某个通用 WPF 类建立导航
   上下文；OneCommander 必须同时确认进程身份和 UIA 路径控件。
3. **活动面板**：双栏/多标签宿主必须在热键采集时用焦点子 HWND 或 UIA 焦点元素冻结活动面板，
   并保存当时的当前目录；弹窗显示后不得重新把“当前前台窗口”当作目标。
4. **原窗导航**：文件夹切换到自身，文件、EXE、快捷方式仍由既有 Rust/C# 动作解析到父目录；
   适配器只把最终目录切换到捕获的同一窗口和活动面板，不启动 Explorer 或另一个文件管理器实例。
5. **连续性**：适配器确认导航成功后恢复捕获宿主为前台，使下一次双击 Ctrl 能重新采集同一宿主；
   Escape/重复热键仍走通用显式关闭恢复链路。
6. **有界失败**：HWND 消失、宿主身份变化、消息超时、UIA 控件失效或导航后目录未变化时返回相关
   失败并保留速启面板；不得把“消息已发送”当作成功。跨进程消息和验证等待均设置短超时。
7. **性能与职责**：第三方识别只在呼出时运行一次，导航只在用户执行动作时运行；普通查询、结果
   渲染和空闲状态不得轮询宿主或调用 UIA。Rust 保存会话与历史，C# 只做 Windows 适配，GPUI
   继续复用同一个“切换到此目录”动作和错误通知。
8. **请求级面板身份**：每次成功采集都创建新的不透明 `host_context_id`，绑定具体适配器、顶层
   HWND 和活动面板状态。Rust 只能原样带回；C# 必须拒绝缺失、过期、跨 HWND 或已成功消费的
   令牌。不能用“每个 HWND 一个当前面板”的全局缓存，因为同一窗口的后一次呼出会覆盖仍在
   执行的前一次请求。
9. **高权限宿主边界**：SwiftList 不是让普通 UI 对高权限窗口直接发送消息，而是由 LocalSystem
   服务在请求用户的会话中启动同权限 Hook 进程，再经受限 IPC 执行适配器。NxTools.Next 不能用
   `runas` 每次弹 UAC、不能整体提升 Rust/GPUI 桌面，也不能声称中完整性进程已覆盖高完整性宿主；
   该能力必须通过索引服务代理启动、同会话身份校验和独立高权限 C# 适配器通道实现。

#### NxTools.Next 同会话高权限实现（2026-07-23）

- 中完整性 `NxTools.PlatformHost.exe` 创建随机 `CurrentUserOnly` 命名管道和 256 位随机令牌，
  只向 Rust 发布 rendezvous 数据，不发布任意可执行文件路径。
- Elevated installer 从锁定且拒绝重解析点的源句柄读取 `NxTools.Indexer.exe`、broker
  `NxTools.PlatformHost.exe` 和 `NxTools.exe`，逐级取得 ProgramData 产品父目录所有权、替换
  精确 DACL，再原子部署到 `%ProgramData%\NxTools\NxTools.Next\Service`；只保留
  SYSTEM/Administrators 写入与 Users 读取/执行。SCM 不注册、特权服务也不认证用户可写的
  Release artifact；UAC 授权的 PlatformHost 入口锁定自身映像，并用构建时嵌入的
  Desktop/Indexer SHA-256 清单校验两个 Rust 源句柄；安装成功后 Release 启动器切换到字节一致
  的受保护桌面。
- LocalSystem `NxTools.Indexer.exe` 从已接受的桌面管道句柄读取真实客户端 PID 与会话 ID，验证
  客户端路径和内容与管理员登记的 `NxTools.exe` 一致。服务只允许启动受保护目录中固定的
  `NxTools.PlatformHost.exe --launch-elevated-directory-helper`，并在每次启动前核对管理员安装
  时写入服务注册表项的桌面及 PlatformHost SHA-256；等待时间有硬上限。
- C# broker 使用 `WTSQueryUserToken` 获取请求会话令牌，要求存在关联高权限令牌，并以
  `CreateProcessAsUser` 在 `winsta0\default` 启动辅助模式；不提升 Rust/GPUI，也不在每次动作时
  使用 `runas`。
- 主机验证辅助进程的随机令牌、实际命名管道客户端 PID、同一 Windows 会话、精确程序路径和
  `TokenElevation`；
  辅助进程随后在高权限侧重新采集请求 HWND，避免信任中完整性进程无法读取的焦点控件。
- 高权限侧为实际宿主采集结果再创建有界、过期、成功后消费的 `elevated:` 外层令牌，并在执行时
  还原内部第三方活动面板令牌。只有 `elevated:` 上下文进入辅助通道，普通资源管理器、文件对话框
  和第三方文件管理器仍走中完整性适配器。连接、帧长度、STA/UIA 与导航都必须有界并失败关闭。
- 热键处理先发布无高权限上下文的面板请求，再在工作线程请求 helper；回填事件带原始目标 HWND
  与请求时间，C# 发布前和 Rust 应用前都检查一秒时效，且 Rust 只接受仍处于同一召唤代次的结果。
  OneCommander 使用同完整性进程持续缓存的最后焦点 UIA 元素，并保留呼出时的焦点 HWND，避免
  GPUI 抢焦点后退化为第一个窗格。helper 启动状态也带 pipe generation，
  普通 USN/索引状态不能重复启动高权限进程，旧状态不能清除新请求。
- 管道 hello、请求和响应都在读取过程中执行字符上限，错误连接会换新 pipe/token 按有限次数
  退避重试；成功消费和过期清理同时移除一次性令牌及顺序元数据，不能形成隐性无界队列。
- OneCommander 焦点跟踪器在两个 PlatformHost 模式入口预热，订阅后立即采样当前 UIA 焦点，
  后续只保存同一窗口最后焦点元素。Explorer/OneCommander 的非协作导航超时通过 Fatal 响应让
  helper 退出，同时使普通客户端立刻作废旧连接并预建下一代 helper。同名 UIA 工作在旧线程
  真正退出前持有唯一租约，重复呼出只快速失败而不继续创建可能阻塞的 STA 线程。

### 阶段 5：自定义命令纵向切片（2026-07-25 复核）

#### 新核对的参考入口

- 旧 `D:\Code\NxTools` 没有速启自定义命令实现，本切片没有可迁移的旧行为。
- Listary 托管边界 `.diagnostics/listary-managed/Listary/Core/Search/Class263.cs` 表明启用且关键词
  非空的命令有两种匹配：查询仍是关键词前缀时显示提示；查询以完整关键词加一个空格开头时，
  空格后的原始文本作为参数传给命令。大小写不敏感，关键词后没有空格的更长单词不会误匹配。
- Listary `.diagnostics/listary-managed/Listary/Core/Commands/Class265.cs` 为每次命中记录实际使用的
  关键词和参数；`CustomCommand.cs` 的稳定配置字段是标题、程序路径、参数、工作目录、管理员运行和
  静默运行；`CustomCommandViewModel.cs` 执行前展开程序与参数中的环境变量，优先读取程序的 Shell
  图标，并使用 `ProcessStartInfo`、`UseShellExecute = true`、可选 `runas` 和隐藏窗口。其若干占位符
  字符串已被混淆，不能据此臆测语法。
- Listary `.diagnostics/listary-managed/Listary/Core/OptionsWindow/CommandOptionsViewModel.cs` 使用一组
  可新增、删除、启用和编辑的命令配置，不为每条命令创建独立插件进程。
- Listary 同一 `CommandOptionsViewModel.cs` 还把可观察命令集合、当前选中命令、新增后选中新项、删除
  当前项和最终 Apply 持久化放在同一主从状态模型中；本轮只把它作为选择与动作归属证据，不复制其
  托管视图结构。
- SwiftList `Plugins/CustomCommands/CustomCommandsInstantProvider.cs`、`ArgQuoting.cs` 与
  `CustomCommandsPlugin.cs` 提供可审计的具体模板规则：第一个空格前是大小写不敏感的精确关键词；
  `%s`/`{}` 引用全部后续输入，`%s1`/`{1}` 引用按单双引号分词后的第 N 个参数，越界替换为空；
  模板没有占位符时不会隐式追加输入。启动字段还包括工作目录、静默与管理员标记。
- NxTools.Next 的程序路径选择沿用现有类型化选择器边界：
  `PlatformCommand::PickProgram` → `PlatformCommandParser.ParsePickProgram` →
  `Windows/ProgramPicker.cs`。Rust 保留未保存编辑状态，C# 只打开系统 `OpenFileDialog` 并返回路径。
- SwiftList `App/Views/Settings/Plugins/Templates.xaml`、`FieldRowTemplate.xaml` 与
  `PluginConfigArrayItemViewModel.cs` 的对象数组设置使用真实 master-detail：左侧连续选择列表、下方
  新增入口、右侧由分隔线隔开的纵向字段编辑器。其列表尾部逐行删除仅作反例；QL-11 明确将启用和
  删除归属到当前详情，避免选择、状态和破坏性动作挤在同一行。

#### 本纵向切片必须保持的不变量

1. **匹配与综合结果**：启用命令支持关键词前缀提示和 `<完整关键词> <参数>` 执行项；完整关键词
   后紧跟非空格字符不能命中。命令项参加统一排序但不能排他，查询仍提交 MFT/USN，应用、文件与
   文件夹结果在同一代完整批次中合并，不能被命令项或异步文件批次互相覆盖。禁用或非法命令不参与匹配。
2. **模板**：首版只承诺 `%s`、`%sN`，并兼容 SwiftList 的 `{}`、`{N}`；位置参数使用单双引号
   分词，展开为类型化参数数组。模板没有占位符时不追加用户输入，越界位置参数为空。未闭合引号、
   `%s0`、空占位符编号等非法模板在保存或执行前返回明确错误，不能静默拼接整条命令行。
3. **配置**：Rust 持久化 `enabled`、`keyword`、`title`、`program`、`argument_template`、
   `working_directory`、`run_as_admin`、`run_silently`。规范化只修剪外围空白和统一关键词比较，不能
   改写用户参数内容。启用命令的关键词必须非空且大小写不敏感唯一，程序必须非空。
4. **执行**：Rust 在执行时展开程序、参数和工作目录中的 Windows 环境变量，生成独立的 executable、
   arguments、working directory、UAC 和隐藏标记；C# 只把该类型化请求映射为 `ProcessStartInfo`。
   工作目录未配置时使用程序父目录；Shell 执行、UAC 取消、路径无效或启动失败都返回相关错误。
5. **布局与交互**：主界面使用一个连续 master-detail：左侧 42 px 身份列表只承载 Body 标题与
   Label 关键词/状态，新增归属 master 标题；右侧唯一详情编辑器承载启用、删除和保存。字段使用
   32 px shell，相关字段等宽并保持 8 px gutter，标准 Checkbox 取代手绘小控件。切换选择只替换
   当前草稿，不触发隐式保存；新增、启用、删除保持既有显式即时持久化，字段编辑只在保存时提交。
   程序输入右侧通过类型化命令调用系统文件选择器，选择结果只写回当前未保存编辑器；取消或 stale
   结果不修改草稿，诊断保留 correlation/outcome/elapsed/native code 且不记录路径。速启结果显示
   命令标题和将执行的摘要。Enter/单击执行并在成功回执后隐藏；右键只提供“运行命令”和“复制命令”，不请求 Windows
   文件 Shell 菜单。图标优先请求程序 Shell 图标，失败使用 Material Symbols 命令图标。
6. **生命周期**：平台启动成功后才记录历史并隐藏面板；失败时面板保持可见并显示错误。命令执行
   不重新读取当前前台窗口，也不把 GPUI 自己的 HWND 当工作目录上下文。
7. **性能与职责**：普通查询只遍历内存中的小型命令快照；参数解析和环境变量展开不访问索引或
   SQLite。Rust 拥有配置、匹配和模板，GPUI 拥有渲染与编辑，C# 仅拥有 Windows 进程启动适配。
   本切片不实现网页搜索，也不建立二进制插件 ABI。

### 阶段 5：设置、诊断与验收收尾（2026-07-25 复核）

#### 新核对的参考入口

- 用户明确将速启设置补齐、第三方文件管理器覆盖和最终验收/质感收尾排在网页搜索、非 NTFS
  索引与环境变量名称枚举之前；这是经批准的阶段 5 内部优先级调整。
- Listary 托管边界
  `.diagnostics/listary-managed/Listary/Core/OptionsWindow/IndexOptionsViewModel.cs`
  将索引路径、标题、最近更新时间、条目数和重建命令作为设置状态，而不是放进搜索结果列表。
- Listary
  `.diagnostics/listary-managed/Listary/Core/OptionsWindow/FileSearchIndexDebugViewModel.cs`
  和 `Main/Options/FileSearchIndexDebugDialog.cs` 提供独立诊断命令、诊断文本和复制入口；
  `MiniCore/IndexerDiagnostics.cs` 通过类型化索引服务消息承载诊断内容。托管边界没有暴露
  native 搜索器内部采样公式，因此不推断其实现。
- 已核对的 Listary 诊断调用链是
  `Main/Options/FileSearchOptionsView.cs::cy` 创建 `FileSearchIndexDebugDialog` →
  `Core/OptionsWindow/FileSearchOptionsViewModel::CreateFileSearchIndexDebugViewModel` 注入
  `Core/FileSearch/Class387` → `FileSearchIndexDebugViewModel::DoLog` 执行 `Class387::method_5` →
  索引服务接口 `Class65::imethod_3` 返回诊断文本 → 对话框 `LogTextBox` 展示并由
  `FileSearchIndexDebugDialog::cu` 复制。这个链路只证明诊断入口和服务边界，不证明 native
  引擎内部如何生成文本。
- NxTools.Next 已有 `FinderCatalog` 内存排序历史、SQLite 查询关联和最近目录三类状态；
  `IndexSearchBatch.elapsed_us` 提供索引服务查询耗时；`AppPaths.log_dir` 是桌面与平台主机日志的
  稳定目录。设置页只读取这些现有 Rust 状态，打开日志目录复用类型化 `OpenShellTarget`。

#### 本纵向切片必须保持的不变量

1. **历史一致性**：清空操作必须同时移除排序学习、关键词关联和最近目录的 SQLite 与内存状态；
   重启后不能恢复已清理内容。应用/文件索引本体和用户配置不能被删除。
2. **性能热路径**：查询耗时只在已有本地合并与索引批次边界记录，不增加采样线程、空闲轮询、
   逐项计时或额外 SQLite 查询。设置页只读取内存快照。
3. **诊断入口**：设置页显示真实日志目录，并通过现有类型化 Windows Shell 动作打开；Rust 不直接
   调用 Win32，C# 不拥有诊断状态或 GPUI 布局。
4. **布局**：历史、最近查询和日志入口使用一张紧凑设置卡片，继续复用主设置页滚动容器；
   不新增独立大面板或持续动画。
5. **反馈**：清理成功或失败使用全局通知，不把状态文本永久拼接在设置项后面。

### 阶段 5：Files 第三方文件管理器适配（2026-07-27 复核）

#### 新核对的 Listary 入口与调用链

- Listary 安装目录
  `D:\Program Files\Listary\FileAppPlugins\Files\ListaryPluginManifest.json`
  声明 `Files` 为 `FileAppPlugin`，并指向公开项目
  `https://github.com/files-community/Listary.FileAppPlugin.Files`。本次只核对已经安装的托管
  插件行为，不复制其源码。
- 经用户授权使用 dnSpy 读取
  `Listary.FileAppPlugin.Files.dll` 的托管边界后，确认调用链为：
  `FilesPlugin::BindFileWindow` 先要求目标 HWND 的进程映像文件名精确为 `Files.exe`，再要求
  UIA 根名称等于 `Files` 或包含 `- Files`，随后构造 `FilesWindow`；
  `FilesWindow::GetCurrentTab` 构造 `FilesTab`；
  `FilesTab` 在根元素的直接子元素中查找类名
  `Microsoft.UI.Content.DesktopChildSiteBridge`，再在该桥接元素的直接子元素中取得 Automation ID
  `CurrentPathGet` 与 `CurrentPathSet`；
  `FilesTab::GetCurrentFolder` 读取前者的文本，
  `FilesTab::OpenFolder` 向后者写入目标路径。
- Listary 将 Files 标记为已打开目录提供者，使用浮动搜索栏，但未标记为 Quick Switch
  目标。这意味着 Files 可以向文件对话框快速切换链路提供当前目录，但不是自动跳转的目标。
  NxTools.Next 另有用户明确要求的手动“切换到此目录”，因此只复用可验证的窗口识别和目录控件
  协议，导航成功判定继续采用 NxTools.Next 更严格的回读确认。

#### 本纵向切片必须保持的不变量

1. **强身份**：只接受进程名 `Files`、仍有效的原始顶层 HWND，以及符合 Listary 证据的 UIA
   根名称；仅凭标题或存在同名 Automation ID 不能绑定其他程序。
2. **事件时冻结**：手动导航只在双击 Ctrl 呼出时冻结同一 Files HWND 的当前目录和 UIA
   宿主状态；既有自动快速切换只在明确的 Files 前台来源转换中采集当前目录，并且目标仍只能是
   已识别的文件对话框。GPUI 面板获得焦点后不得重新寻找当前前台窗口，也不得切换到另一个
   Files 窗口。
3. **有界 UIA**：识别、读取、写入和回读都在 C# Windows 适配层的有界 STA 操作中完成；
   UIA 卡死、控件消失或取消时返回 Fatal 失败，不能阻塞 Rust/GPUI 线程；取消令牌在写入和
   每次回读前检查，超时后不得重试或发起第二次写入。已经进入 Windows UIA COM 调用的单次写入
   无法回滚，因此不能把超时响应描述成 OS 级事务撤销；通用会话注册表收到 Fatal 导航失败时
   立即作废本次令牌，用户必须重新呼出并重新捕获后才能再次执行。
4. **路径验证**：捕获值必须是有根目录；执行前继续由通用会话注册表验证目标目录真实存在。
5. **导航确认**：向 `CurrentPathSet` 写入后必须轮询 `CurrentPathGet`，只有规范化目录与目标一致
   才算成功；仅完成 `SetValue` 不能消费会话令牌。
6. **窗口连续性**：只有写入并回读成功后才恢复捕获的同一 Files HWND；窗口身份变化、原 HWND
   失效或回读超时时不得预先激活 Files，激活失败保留相关错误。成功消费、过期、跨 HWND 拒绝
   仍复用通用会话生命周期。
7. **职责与性能**：Rust 继续只认识通用 `third_party_file_manager` 上下文和不透明令牌；
   C# 拥有 Files/Win32/UIA 细节；GPUI 不增加 Files 专属分支。除手动呼出/执行和自动快速切换的
   明确前台来源转换外，空闲和普通查询期间不得轮询 UIA。

### 阶段 5：最终验收与质感静态收尾（2026-07-27 复核）

Reverified on 2026-08-22 before the QL-01 main-popup token migration:

- Listary's managed chain remains `LauncherSearchWindowFactory` → `LauncherSearchWindow` →
  `SearchWindowBase`. The launcher owns one search box and one result list; the base owns show,
  activation and focus, while `SearchMode::Query` cancels and clears results for an empty query when
  `SearchWhenQueryIsEmpty` is false. `ShowResultList` is true only for a non-empty committed result
  collection, so a fresh summon is a compact search surface rather than an empty permanent panel.
- `Class293` still creates the ordinary launcher as one merged multi-provider mode, and its normal
  `SearchMode` awaits provider completion before replacing the committed collection. QL-01 therefore
  preserves the existing single collapsed-to-expanded transition and does not replay entrance motion
  when one non-empty query replaces another.
- SwiftList's `QuickSearchWindowController::ShowWindow` independently confirms the same ownership:
  one reused quick-search HWND is positioned near the active display's upper quarter, shown, verified
  as foreground and then given search focus. It does not justify a second shell or a permanently
  expanded empty-results region.
- Old NxTools still has no Quick Launch surface to migrate. NxTools.Next remains the product geometry
  authority: 620 x 410 expanded, a 42 px search header and a 44 px collapsed native window. The
  QL-01 animated-wrapper repair restores the intended 40 px production row; its regression stays
  with the main-popup stable-expansion contract rather than redesigning
  result-row content ahead of QL-02.

Reverified on 2026-08-22 before the QL-02 result-row migration:

- Listary's ordinary heterogeneous results still converge on
  `SearchItemViewSelector::DefaultDataTemplate`. `SearchItemViewModelBase` supplies the shared
  `Title`, `Subtitle`, `Icon`, `PreferredHotkey` and assigned `Hotkey` contract; path results place
  the parent/root-relative path in `Subtitle`, while application results may intentionally leave it
  empty. Special templates remain limited to no-results, separators, group titles and explicit
  one-line rows, so application, file, command, calculator and URL sources do not justify separate
  column geometries.
- Listary's `SearchMode::UpdateHotkeys` assigns the visible selectable results Ctrl+1 through
  Ctrl+9 unless a result supplies a preferred key. SwiftList independently confirms the same
  behavior in `QuickSearchWindowLayoutManager::UpdateShortcutHints`: its shared
  `SearchResultTemplate` is exactly icon / flexible name-plus-parent-path / trailing shortcut, and
  only the currently visible first nine ordinary rows receive a hint. NxTools.Next therefore keeps
  one fixed-width trailing rail and implements the displayed Ctrl+1…Ctrl+9 routes; it does not show
  a non-functional hint.
- QL-02 preserves the existing 40 px virtual row and result-selection lifecycle. The visual
  migration is limited to a transparent fixed icon slot, a two-line shared copy rail and the fixed
  shortcut rail. It uses Body 12/18 for the title and Label 11/16 for subtitle and shortcut, without
  importing SwiftList's badge treatment or advancing the QL-03 selection redesign.

Reverified on 2026-08-24 before the QL-03 selection-state migration:

- Listary `SearchItemList::OnSelectionChanged` scrolls the one selected item into view. Its row-level
  `MouseMove` handler changes that same `ListBoxItem::IsSelected` only after a second movement at a
  different pointer position; ordinary pointer entry therefore cannot steal a keyboard selection.
  WPF `ListBoxItem` still applies normal press selection, so a direct click cannot execute a row
  while leaving a different row visually selected.
- SwiftList `QuickSearchWindow` binds `ResultsControl.SelectedItem` two-way to the single
  `SelectedResult`, while `QuickSearchWindowInputHandler::MoveResultSelection` changes the same
  `LstResults.SelectedIndex` and scrolls it into view. Enter reads that selected item; pointer and
  keyboard activation do not own parallel selection models.
- NxTools.Next QL-03 therefore keeps one Rust-owned `selected` index and one neutral
  `Theme::element_selected` fill. A left pointer press must commit the row to that index before the
  existing click activation is scheduled; the movement gate and shared virtual-list scroll handle
  remain unchanged. Real Shell images stay unboxed and are never recolored by the selection surface.

Reverified on 2026-08-24 before the QL-04 action-mode migration:

- Listary `SearchBarViewModel::method_9` pushes the child `SearchMode`, while `method_10` disposes only
  that child and reactivates the same parent mode object. The parent therefore retains its committed
  results, selected item and query rather than rebuilding a visually new list before restoration.
- SwiftList `App/Services/ShellMenu/Presenter/ShellMenuPresenter.cs::EnterActionsMode` hides the existing result
  grid without replacing it. `ExitActionsMode` makes that same grid visible, restores the saved query
  while action-mode filtering is still active (so result search cannot restart), and only then calls
  `ScrollIntoView` for the already-selected result. Its source comment explicitly protects the exact
  result and scroll position from being wiped by query restoration.
- NxTools.Next QL-04 must likewise treat the parent result viewport as frozen action-mode context.
  Escape, empty-filter Backspace and root-level Left must reveal the retained result list at its
  previous offset on the first returned frame. Resetting the result scroll to zero and centering the
  selected row in a later query/result callback is a visible top-to-context jump and is not equivalent.

Reverified on 2026-08-24 before the QL-05 preview migration:

- The old `D:/Code/NxTools` tree has no Quick Launch product surface, so it supplies no
  launcher-preview behavior to preserve. Its clipboard preview remains relevant only as the product's
  existing preview-content reference; QL-05 does not merge clipboard and launcher window lifecycles.
- SwiftList `App/Services/QuickLookManager.cs::ShowOrUpdate` retains one `QuickLookWindow`, calls
  `SetTarget` for selection changes and animates only the transition from hidden to visible. Reposition
  and visible target updates do not replay the window entrance. `PositionWindow` also keeps a fixed
  preview size independent of the owner's current result count and clamps it to the active monitor.
- SwiftList `App/Views/QuickLook/QuickLookWindow.xaml(.cs)` keeps one stable header/content/footer
  skeleton. A provider may reuse its content in place for a new target; replacement releases the old
  preview before mounting the next. Media remains inside a dedicated content area rather than changing
  outer-window chrome. Listary's managed `PreviewPopup`/`LauncherSearchWindow` confirms a separately
  anchored preview popup, but its decompiled managed boundary does not expose enough template detail to
  treat its visual internals as a source.
- NxTools.Next QL-05 therefore retains the existing 400 x 410 non-activating, fixed-size auxiliary
  window, generation/correlation guards, offscreen replacement staging, content limits and focus
  restoration. The launcher and clipboard share only the opaque PreviewShell and neutral media carrier.
  A reused visible launcher preview must keep its current shell until the replacement is ready and run
  one 130 ms generation-scoped replacement entrance; it must not play an exit followed by another enter.

Reverified on 2026-08-24 before the QL-06 file-dialog-search migration:

- The old `D:/Code/NxTools` tree still has no Quick Launch or attached file-dialog search
  surface, so there is no older product rendering to preserve. The already-verified
  `file_dialog_search.rs` host recognition, activation, navigation, cancellation and placement
  lifecycle remains the behavioral baseline for this visual slice.
- Listary `DialogSearchWindowFactory::CreateWindow` creates a dedicated `DialogSearchWindow` rather
  than reusing its launcher. `DialogSearchWindow::ah` converts the host `PlacementBounds` from device
  pixels, centers the final `ActualWidth` on the host and places the search header at the host bottom;
  an open result popup is repositioned with it. `SearchWindowBase::ShowWindow` keeps the auxiliary
  surface Topmost but non-activating. QL-06 must therefore retain one owner-bound auxiliary HWND and
  may not turn the attached search into another global launcher or inject controls into the dialog.
- Listary's generated connection table exposes one `PART_SearchBox`, one result popup and one compact
  right-side logo region; it does not support inventing decorative action buttons. Its reference
  screenshot measures approximately 500 px wide with a 36 px search rail and a 35–36 px default
  Quick Switch row. The results expand toward the host while the search rail remains at the bottom.
- SwiftList `InlineSearchManager::EnsureWindowCreated` likewise freezes the active dialog HWND,
  creates one independent `InlineSearchWindow`, sets the current host path as search context and
  restores the dialog's edit focus instead of stealing it. `InlineSearchWindowPositioner` separately
  positions dialog mode against the host/working area and clamps the visible surface. This confirms
  host-relative placement and focus continuity, but its alternate WPF card styling is not a product
  visual source for NxTools.Next.
- NxTools.Next QL-06 therefore keeps the existing 500 logical px width, 36 px search rail, 40 px
  result rows, 264 px result overlap, 16 px host-side safety inset and 80 ms first-show motion. It
  migrates only presentation: Body 12/18 search/title, Label 11/16 path and shortcut, an 8 px rail
  rhythm, a transparent 26 px icon slot and one semantic window boundary. The client-decorated native
  HWND, rather than a clipped shadow inside the exact-size opaque root, owns exterior elevation. At
  96/120/144 DPI the visible width and the result overlap must remain within the bound host while the
  search header stays immediately below its bottom edge; native placement, owner, Topmost, focus,
  debounce, generation and activation behavior do not change.

Reverified on 2026-08-24 before the QL-07 main-workspace settings split:

- The old `D:/Code/NxTools` tree has no Quick Launch settings surface, so it contributes no
  legacy all-in-one layout or page lifecycle that NxTools.Next must preserve.
- Listary `OptionsWindowViewModel` owns one stable `Pages` collection plus one `SelectedPage` and its
  `Navigate` method changes the selected page before forwarding a nested target. Its separate Apply,
  OK and Cancel commands remain owned by the window rather than by each navigation row.
  `FeaturesOptionsViewModel` repeats that pattern for nested feature pages. This is the primary
  reference for stable navigation and page-only replacement; the decompiled tree does not expose a
  trustworthy visual template, so NxTools.Next does not copy its styling.
- SwiftList `SettingsWindow.xaml` independently keeps a fixed titlebar, sidebar and footer while a
  single content grid swaps named settings pages. `SettingsSearchIndex` can activate a page and its
  nested tab before revealing a setting, confirming that navigation is presentation state rather
  than a setting mutation. Its rounded WPF shell and larger page catalogue are not visual sources.
- NxTools.Next QL-07 therefore replaces only the main module's long settings scroll with a stable
  160 px local rail and five mutually exclusive pages: 常规、索引、上下文、命令、诊断. The module
  header, existing setting actions and application lifecycles stay intact; each page owns the same
  bounded scroll viewport, which returns to the top on page change. QL-08 through QL-12 retain
  ownership of the hotkey field, index-volume list, exclusion editor, command master-detail and
  diagnostics-detail redesigns.

Reverified on 2026-08-24 before the QL-08 HotkeyField migration:

- The old `D:/Code/NxTools` tree still has no Quick Launch surface. Its clipboard settings
  provide only the existing NxTools recording lifecycle reference: a read-only value, an explicit
  recording prompt, Escape cancellation, an in-flight registration message and the native success
  or failure result. QL-08 does not copy the old Avalonia card styling or merge the clipboard and
  launcher owners.
- Listary `Views/HotkeyView.cs` uses one focused, read-only field for display and capture. Focus
  changes the cue, valid key input replaces the value in place, Backspace clears it and a small
  trailing action remains inside the same field. `HotkeyOptionsViewModel` keeps application at the
  options boundary. The managed inspection is behavioral evidence only; its template styling is
  not copied.
- SwiftList `HotkeyRecorderControl.xaml(.cs)` independently confirms a fixed recorder field with
  focused key capture, clear/restore affordances and explicit modifier policy. Its first-party
  hotkey guide and `GlobalHotkeyDetector` keep bare Ctrl as a double-tap summon gesture with release,
  repeat, intervening-key and bounded-time guards. NxTools.Next retains its existing typed
  `SetQuickLaunchHotKeyRecording` then `ConfigureQuickLaunch` protocol and C# gesture adapter; this
  visual slice does not replace that native boundary.
- NxTools.Next QL-08 therefore presents the current or pending chord on one stable Mono rail and
  maps the existing correlated lifecycle to applied, waiting, recording, applying and failed states.
  A rejected native result keeps the last applied setting; only a correlation-matched success may
  replace and persist it.

Reverified on 2026-08-24 before the QL-09 index-volume migration:

- The old `D:/Code/NxTools` tree still has no Quick Launch or local-file-index scope UI, so
  it supplies no visual source for this slice. NxTools.Next retains its established typed settings
  boundary: Rust persists selected mount points and synchronizes them to the indexer; the service
  continues reporting every discovered NTFS volume even when that volume is outside the enabled
  set, so a disabled row remains available to re-enable.
- Listary discovers NTFS volumes as named mount roots and persists disabled roots. Its managed
  options model keeps each discovered volume as a stable list item and toggles exclusion/restoration
  without removing the item from discovery. This is behavioral evidence for persistent per-volume
  rows; its proprietary template and delete/restore presentation are not copied.
- SwiftList's first-party Index guide, `LocalTab.xaml`, `Templates.xaml` and
  `LocalDriveSettingsViewModel.cs` independently expose one local-drive row per discovered drive with
  an enable checkbox, drive name, file-system/state and indexed-item metadata. A missing or disabled
  drive remains a row whose state changes rather than becoming a chip or disappearing.
- NxTools.Next QL-09 therefore uses one standard-checkbox list. Its explicit automatic row makes the
  existing empty-vector contract visible: all present and future NTFS volumes are included until the
  user clears one; an explicit set is stored only while the scope is narrower than all discovered
  volumes. The final currently selected volume cannot be cleared because the empty vector is reserved
  for automatic-all; selecting all currently discovered volumes canonicalizes back to automatic even
  when an old unavailable mount remains in persisted input.

Reverified on 2026-08-24 before the QL-10 excluded-directory migration:

- The old `D:/Code/NxTools` tree still has no Quick Launch or local-index exclusion UI, so it
  supplies no visual or interaction source for this slice. NxTools.Next keeps the existing typed
  boundary: Rust owns the ordered, case-insensitive path set and persistence; PlatformHost owns the
  correlated Windows directory picker; the indexer receives the same `IndexSettings.excluded_paths`.
- SwiftList's first-party `ExclusionsTab.xaml`, `Templates.xaml`, `Styles.xaml` and
  `ExclusionSettingsViewModel.cs` use one stable rule row per path, trim long values, keep a dedicated
  trailing remove action with a tooltip, reject blank/duplicate rules case-insensitively and persist
  the resulting collection. This is behavioral evidence for a path list with bounded actions; its
  cards, nested tabs, edit/bulk-text features and WPF styling are not copied into NxTools.Next.
- Listary's managed options boundary was rechecked, but no equivalent user-owned excluded-directory
  list was verified there; its native search engine source is not available and is not claimed as a
  QL-10 reference.
- NxTools.Next QL-10 therefore keeps one full-width continuous path list. Rows expose the complete
  path through a tooltip, reserve a stable trailing delete hit target and support selection with
  Up/Down plus Delete/Backspace. Pointer or keyboard deletion freezes the selected path, defers the
  cross-entity mutation and emits at most one settings/indexer update. The existing picker correlation,
  duplicate suppression, cancellation and failure notification remain unchanged; diagnostics record
  correlation/outcome/elapsed/native code but never the selected path.

Reverified on 2026-08-24 before the QL-12 diagnostics-page closure:

- The old `D:/Code/NxTools` tree still has no Quick Launch settings, history-learning or
  query-performance surface, so it provides no legacy diagnostic layout to preserve.
- The previously verified Listary boundary keeps index metadata and commands in settings, while its
  `FileSearchIndexDebugViewModel` opens a separate diagnostic text/copy flow through the typed index
  service. This supports separating operational diagnostics from ordinary settings, but neither its
  WPF dialog nor its opaque native timing implementation is a visual or sampling source.
- NxTools.Next already owns the complete required state: `FinderCatalog::history_summary` exposes
  ranking, activation, query-association and recent-directory counts; the local merge and
  `IndexSearchBatch.elapsed_us` boundaries retain the latest timing in memory; `AppPaths.log_dir`
  identifies the shared log directory. QL-12 only presents these snapshots and does not add polling,
  database reads or per-result instrumentation.
- QL-12 therefore uses one continuous diagnostic surface rather than metric cards: four equal
  history cells, two equal timing cells and one exact-path log row. Clearing keeps the established
  transactional SQLite plus in-memory reset and global notification; opening logs keeps the typed
  `OpenShellTarget` boundary. Both UI callbacks defer cross-entity work, and the performance rails
  remain absent from every ordinary settings page.

- Listary 的默认启动器合并应用、文件、文件夹和即时结果；NxTools.Next 的主速启和文件对话框
  附着搜索也使用同一类多来源结果语义。因此空结果提示必须使用中性的“没有匹配项”，不能仍写成
  仅针对应用的“没有匹配的应用”。
- 主速启统一为 42 px 搜索头、40 px 双行结果、6 px 列表内边距和语义主题色。2026-07-27
  当前运行截图
  `docs/audits/launcher-clipboard-ui-2026-07-27/01-launcher-results-preview.png`
  证明八行结果、整行选中和相邻预览的基本密度已经成立，本轮不再次任意缩放。
- QL-01 至 QL-06 已将主搜索、普通/动作结果、预览和文件对话框附着搜索迁移到共享 Body 12/18
  与 Label 11/16。各表面继续使用语义选中态、中性窗口边界和各自已验证的短动效；详情使用次要
  文本色，时间使用弱化文本色，不把速启改成高饱和强调色界面。4 px 圆角只保留在规范明确要求的
  控件或媒体承托面，连续结果面和精确尺寸附着根节点保持方正。
- 预览窗口保持 400 × 410 px 固定尺寸。该窗口会跨条目复用，且为保护键盘焦点与避免闪烁，切换
  内容时不会调用 GPUI/Windows resize；因此元数据、文本和图片不能采用不同窗口高度，否则先出现
  的小窗口会裁剪后续文本/图片。元数据只在固定窗口内部使用紧凑内容块。主速启与附着搜索只共享
  视觉标尺，不合并两个独立窗口的布局状态或生命周期。
