# Listary 启动器：搜索内容、排序与呈现

## 结论

本次源码核对时，NxTools.Next 与 Listary 的差异同时存在于候选语义、历史学习和结果呈现，不只是颜色或间距。

- Listary 区分应用候选与普通磁盘文件，并区分良好匹配和弱匹配；Next 对可启动扩展名统一增加 6500 分。
- Listary 的完整关键词历史、前缀关键词历史优先按最近使用时间排列；Next 只查完整关键词，并将累计次数放在最近使用时间之前。
- Listary 路径项按统一路径身份关联历史；Next 的应用目录项与索引项使用不同 ID，按路径去重时没有合并历史。
- Listary 应用行无副标题，文件行显示父路径；Next 普通结果强制双行，并以完整目标路径补齐副标题。
- Listary 启动器行高 52 DIP、图标 32 DIP、标题 15/16 DIP；Next 行本体 40 DIP、真实图标 21 DIP、标题和副标题均为 12 DIP。
- Listary 按内容调整窗口高度，列表最高 470 DIP；Next 非空查询展开为固定 620×410。

本记录是行为与模板参考，不改变当前产品契约，也不代表已完成交互或视觉验收。

当前实现见 [交互契约](../../.trellis/spec/nxtools-desktop/backend/ui-experience-contracts.md#scenario-quick-launch-continuous-refresh-and-default-selection) 和 [排名契约](../../.trellis/spec/nxtools-finder/backend/quality-guidelines.md#scenario-application-classes-and-merged-result-ranking)：新查询默认首项，同查询主动选择保留身份；旧行维持正常文字，暂不显示选中强调或快捷键，也不可执行；应用使用单标题、文件使用父目录副标题。下表中的 Next 差异是研究时快照，完成项见文末当前实现。

## 证据与复查入口

核对安装版本：`D:\Program Files\Listary\Listary.exe`，FileVersion / ProductVersion 均为 `6.3.5.94`。
固定托管反编译根目录为 `D:\Code\Listary.Decompiled\6.3.5.94\managed\Listary\`。下文的 `Core/`、`Main/`、`Views/` 均相对此根目录；编译器生成状态机另见同版本的 `expanded\Listary\`。资源导出模式下窗口代码为 `.xaml.cs`，旧行号应按类型和成员重新定位。

排序比较函数需启用编译器生成成员。使用 `D:\Tool\dnSpy-net-win64\dnSpy.Console.exe`：

```powershell
$dnSpy = 'D:\Tool\dnSpy-net-win64\dnSpy.Console.exe'
$listary = 'D:\Program Files\Listary\Listary.exe'
& $dnSpy --no-color --show-all --dont-anon-methods -t Listary.Core.Search.Class256 $listary
& $dnSpy --no-color --show-all --dont-anon-methods -t Listary.Core.Search.Class253 $listary
& $dnSpy --no-color --show-all --dont-anon-methods `
    -t Listary.Common.StringMatcher.StringMatchScoreRules `
    'D:\Program Files\Listary\Listary.Common.dll'
```

结果模板存在于 `Listary.g.resources` 的 BAML 中。导出资源时保留 BAML 转换：

```powershell
$exportRoot = Join-Path $env:PI_SCRATCH_DIR 'listary-xaml'
& $dnSpy --no-color --no-sln --threads 1 -o $exportRoot $listary
```

此次导出成功并读取了这些文件，路径相对 `$exportRoot/Listary/`：

| 导出入口 | 关键位置 |
| --- | --- |
| `Main/Views/LauncherSearchWindow.xaml` | 19：SizeToContent；29：普通模板；110：空副标题触发器；136：无结果；160：动作单行；231：组标题；409：570 DIP 内容列；448：输入框；478：结果区域；487：列表最大高度 |
| `Main/Views/SearchItemList.xaml` | 9：无边框及虚拟化；26：零外距/内距的行容器；38：可选择状态；61：选中背景 |
| `Views/searchwindowresources.xaml` | 12：另一套共享紧凑模板；并非 Launcher 的局部模板 |
| `Views/searchwindowstyles.xaml` | 5：结果滚动条模板，竖向宽 4 DIP |

导出物只用于本地研究；项目保存行为摘要和可复查入口，不把专有 C#/XAML 实现纳入产品源码。

## 结果从哪里来

`Core/Launcher/Class293.cs:214` 建立普通启动器 SearchMode，挂接多个 provider，再由 `Class256` 统一合并：

| 来源 | 入口与行为 |
| --- | --- |
| 开始菜单、桌面等指定目录的应用候选 | `Core/Search/Class253.cs:18` 配置目录；`:34` 查询限定目录、只匹配基础名、不搜索目录，候选上限 100；按 `FileName` 分组取首项，再构造 `Class241`，显示名去扩展名。隐藏委托 `Class253.<>c.method_0/1` 确认分组键为文件名，不是解析后的快捷方式目标。 |
| Windows 打包应用 | `Core/Launcher/Class291.cs:23` 枚举 `FOLDERID_AppsFolder`，读取 PackageFamilyName、AppUserModel_ID、显示名和 PIDL；`Core/Search/Class262.cs:23` 匹配显示名，构造 `Class261`。 |
| 索引文件/目录 | `Core/FileSearch/Class385.cs:25` 建立查询、取消、低优先级文件和优先目录条件；普通启动器给该来源 50 个候选。 |
| 历史路径候选 | `Core/Search/Class251.cs:27` 查询历史路径库，使用相同筛选语法与关键词 matcher；普通启动器给该来源 50 个候选。 |
| 命令、网址、即时结果、扩展 | `Core/Launcher/Class293.cs:226` 起依次挂接对应 provider；结果仍进入同一个合并器，而非按来源各画一个面板。 |
| 更多结果入口 | `Core/StandaloneSearchWindow/Class201.cs:13` 构造 `Class234`；`Core/Search/Class177.cs:23` 生成带查询词的更多结果行，`:48` 打开完整文件搜索窗口。 |

同路径合并入口 `Core/Search/Class257.cs:10`：以小写路径为键，在保留项上补充名称、关键词、索引类型、修改时间、匹配分与优先目录标记。它不是简单遇到重复就丢弃后项。

## 排序：类别优先，不是所有来源加一个统一常数

`Core/History/Class148.cs:41` 为每个结果查询三类时间：

1. 同类型、同 ID、同完整关键词的最近使用时间。
2. 没有完整关键词历史时，历史关键词以当前关键词开头的最近使用时间。例如以前用 `visual` 选择过某应用，现在输入 `vis`。
3. 同类型、同 ID 的任意关键词最近使用时间。

路径项的 `Core/Search/Class239.cs:93` 统一用小写路径作为历史 ID，并携带本次查询词。应用路径项 `Class241` 继承这个历史身份。

`Core/Search/Class256.cs:25` 的合并顺序是：

| 优先层 | 层内排序 |
| --- | --- |
| `Interface34` 扩展/即时委托结果 | 位于一般历史/匹配结果之前 |
| 完整关键词历史 | 最近使用在前；隐藏比较函数 `<>c.method_0` |
| 前缀关键词历史，以及带明确关键词的 `Interface14` 结果 | 最近关联在前；显式关键词无历史时使用最小时间占位；`<>c.method_1` |
| 有普通历史且不是弱匹配的结果 | 匹配分降序；`<>c.method_3` |
| 无上述历史、实现 `Interface16` 的良好应用匹配 | 匹配分降序 |
| 一般结果前 10 个 | 路径结果优先使用 `CombinedScore`，否则使用匹配分；`smethod_1`、`<>c.method_4` |
| 有普通历史但属于弱匹配的结果 | 最近使用在前；`<>c.method_5` |
| 弱应用匹配 | 匹配分降序 |
| 其余一般结果 | 延续一般结果排序 |

`Listary.Common.StringMatcher.StringMatchScoreRules.IsGoodMatch` 的托管判据为 `score > 1`，`BadMatchScore = 1`。这不意味着 Next 的独立评分也应使用这个数值。

合并器在结果数大于 9 时于索引 8 插入更多结果入口；超过设置上限时截取并在末尾追加入口。普通启动器构造上限为 50，不能把它理解成严格最多 50 个可见控件，也不能把前八条理解成只查询八条。

这些是内部排序层，不是要求 UI 为每一层显示分类标题。普通启动器仍是一条混合结果列表。

## 呈现：启动器与内嵌搜索不是同一套模板

### 启动器局部模板

`LauncherSearchWindow.xaml` 自己定义并绑定 `ItemViewSelector`，没有用共享紧凑模板替代它。

| 项目 | Listary 6.3.5.94 模板行为 |
| --- | --- |
| 普通结果行 | 高 52 DIP；左列 64 DIP；图标 32×32 DIP；图标外距 20/10/12/10；行容器没有额外间距与边框 |
| 文件/目录 | 标题 15 DIP，行高 20；副标题 12 DIP，设置行高 24；内容为父目录或上下文根相对目录，不重复显示完整文件名 |
| 应用 | `AppPathSearchItemViewModel.cs:20` 的 Subtitle 为 null；模板折叠副标题，标题改为 16 DIP、行高 32，使用单标题布局 |
| 标题高亮 | 绑定 `TitleMatchedPosition`，使用主题 `ThemeForegroundPrimaryAccentBrush`；`Views/HighlightTextBlock.cs:122` 按位图拆分文字片段，不给整个名称统一染色 |
| 长路径 | `PathTrimming=PathEllipsisPath`；`Views/PathTextBlock.cs:98` 从中间缩短，保留路径头尾；Tooltip 保留完整副标题 |
| 快捷键 | 右侧独立区域，15 DIP；由当前视口重新分配 Ctrl+1…9 |
| 无结果 | 同一列表内不可选择的专用行，高 35 DIP，居中文字 22 DIP |
| Shell 单行动作 | 高 36 DIP，图标 16 DIP，标题 16 DIP，可显示子菜单箭头 |
| 分组/分隔 | 列表内不可选项；组标题 18 DIP；分隔区高 10 DIP，线宽 1 DIP |
| 内容宽度 | 570 DIP，不含外部阴影留白和边框 |
| 高度 | Window `SizeToContent=WidthAndHeight`；结果列表只有 `MaxHeight=470`，少量结果自然收缩 |
| 搜索框 | 高 34 DIP，上下外距各 12 DIP，字号 26 DIP |
| 选择 | 同一列表选择；主题选中背景；无每行常驻卡片边框 |

`Views/searchwindowresources.xaml` 的共享默认模板则使用 16 DIP 图标、同一横行内的名称和右侧路径、另一种路径截断策略。不能用这套内嵌搜索布局来推断启动器。

### 刷新、选择与窗口生命周期

- `Core/ViewModels/SearchMode.cs:701`：普通启动器默认等待所有 provider，取消检查、排序、转换 ViewModel 后一次替换 Results。等待期间保留上次提交的列表。
- `SearchMode.cs:864`：空查询且未开启空查询搜索时取消搜索并清空 Results；`:509` 的 ShowResultList 依结果集合是否非空决定。
- `SearchMode.cs:745`：非空查询无命中时提交专用无结果项；新集合提交后选择第一个可选项。
- `Main/Views/SearchItemList.cs:39` 与 `SearchWindowBase.cs:425`：选中项滚入视口；不是每次移动都强制居中。
- `SearchItemList.cs:50`：同一行第二次且坐标发生变化的 MouseMove 才改变选择，静止指针不能夺走键盘选择。
- `Main/Views/LauncherSearchWindowFactory.cs:18` → `LauncherSearchWindow.cs:24` → `SearchWindowBase.cs:268`：独立窗口、ViewModel show/hide/activate/close 事件、前台与搜索输入焦点。`LauncherSearchWindow.cs:50` 在没有保存位置时放到光标所在屏幕的水平中央、工作区上方四分之一处。
- Launcher 拥有一个相邻 PreviewPopup。预览、选择、主窗口生命周期不能拆成多个竞争的控制器。
- `Main/Views/LauncherSearchWindow.xaml:459,469–495` binds `Query` on each text change, binds `ActiveMode.IsSearching` to a separate 32-DIP `ListaryLogoView`, and binds the old result list independently. `Views/ListaryLogoView.xaml.cs:33–67,110–113` starts rotation after the default 150 ms delay, then clears or settles it when searching stops. This is a presentation delay, not query debounce. `SearchBarViewModel.cs:1073–1106,1451–1454` dispatches Enter through the selected item and its `CanExecute`; that call path has no `IsSearching` guard. Next keeps its own stale-row execution guard.
- Next places the waiting cue in the existing Large Kit0.8.0 `Input::suffix`, with a stable-width slot and Small `Spinner` plus `Label`. Reduced motion uses the label alone. A generation-scoped 150 ms visual delay never delays dispatch or publication. Pending old rows keep their geometry but lose selection emphasis and shortcut hints; input focus, disabled stale activation, window placement and passive preview are unchanged.
- For Notepad and Settings specifically, Finder's existing `system_entries` → `visit_search_candidates` → merged application tier is sufficient. Use localized names and literal aliases for the two stable system targets; `quick_launch/actions.rs::platform_command_for_action` sends `SystemTool` to `Windows/ClipboardShellTarget.cs::TryCreate/Launch`, which uses the existing Shell dispatch. Exact `ms-settings:` is an ordinary protocol activation, not a filesystem or elevated target. No new AppsFolder enumeration, provider, result surface, focus, preview window, or native show/hide lifecycle is needed for these two targets.

## NxTools.Next 当前差异与修正方向

| 差异 | 当前入口 | 用户可见影响 / 修正方向 |
| --- | --- | --- |
| 可启动类型统一加 6500 | `rust/crates/nxtools-finder/src/lib.rs:1091`、`:1152`；`:35` 定义常数 | 任意索引 `.exe/.lnk/.cmd` 等也获得提升。独立评分中弱子序列约 2000、普通完整名称匹配 7000，弱可执行项可越过完整名称匹配的目录。应区分正式应用候选、普通可执行文件和匹配质量，不以一个大常数代替分类。 |
| 关键词历史次数先于时间 | `rust/crates/nxtools-finder/src/lib.rs:1078` | 同关键词下旧的高频项可压住刚刚选过的另一项。按明确的历史类别与最近选择处理，而不是继续叠加常数。 |
| 没有前缀关键词历史 | `rust/crates/nxtools-finder/src/lib.rs:633` | 完整查询学习无法自然作用于缩短后的查询。读取已有查询关联时补足前缀语义；不需要另一套历史库。 |
| 路径身份分裂、去重不合并历史 | `rust/apps/nxtools-desktop/src/quick_launch.rs:2364` 使用 `indexed:`；Finder `lib.rs:1066` 使用 `app:`；`:594` 按目标路径丢弃重复项 | 同一路径从索引候选变为应用目录候选时，已有使用关联不会自动继承。统一路径历史身份与去重信息，而不是按 provider 分割学习。 |
| 应用来源覆盖不足 | Finder `lib.rs:923`、`:952`、`:983`；PlatformHost/Rust 未检出 AppsFolder/AppUserModel 枚举入口 | 当前扫描开始菜单/桌面文件与静态系统项，不等于 Listary 的打包应用显示名/AUMID 通道。需要原生能力时由 C# 提供 Windows 枚举，Rust 继续拥有结果状态。 |
| 同名快捷方式保留多份 | Finder `lib.rs:1031` | 不同目录同名 `.lnk` 仅按完整路径去重；Listary 应用 provider 还按 FileName 折叠。应用候选应有明确的重复策略，不能据此推导所有同名普通文件都要合并。 |
| 强制双行与重复完整路径 | `rust/apps/nxtools-desktop/src/quick_launch/retained_text.rs:17`、`panel.rs:1239` | 应用名称被压小并附带无助于识别的快捷方式路径；文件名在标题和副标题重复。按结果语义保留可选副标题：应用单标题，文件父路径，命令/扩展保留自己的描述。 |
| 长路径只做尾部截断 | `panel.rs:1257` | 相同路径前缀占据空间，真正区分目标的后段丢失。父路径采用中间省略并可查看完整内容。 |
| 密度与主次关系不同 | `rust/apps/nxtools-desktop/src/ui/metrics.rs:28`、`panel.rs:1210` | 当前 40 DIP 行、21 DIP 图标、12/12 DIP 双行；不是 Listary 的 52/32/15–16/12。后续布局应明确选择目标密度和单/双行层级，不把历史尺寸约束当作参考验收。 |
| 少量结果也占满面板 | `rust/apps/nxtools-desktop/src/quick_launch/search.rs:57` | 非空查询固定展开 410 DIP，无结果也使用大块状态。参考是按内容增长、有最大高度的列表；几何修正需同时覆盖原生 bounds、滚动、预览停靠和焦点。 |
| 键盘移动使用居中滚动 | `panel.rs:505` | Listary 只保证可见。应避免仍在可见区域内上下移动时列表不必要地位移。 |

### 保留的当前契约

- `.trellis/spec/shared/ui-design.md` 的 Current-result amendment 明确要求当前本地/索引/扩展结果在所有 provider 结束前即可显示和执行。保留这个契约，不自动退回 Listary 的默认 Task.WhenAll 发布方式。
- 旧查询保留行必须保持不可执行，查询 generation、异步取消、IME Enter 和跨 entity 的 next-turn 边界不变。
- 保留 200 个结果上限、620 DIP 宽度和默认 4 DIP 可选间距；搜索头按参考扩大为 58 DIP，列表高度按查询提交结果自适应。默认结果混排不等于缺少分类标题，不额外发明 provider 分组面板。
- 继续使用当前 Kit 0.8.0 官方组件及公开内容槽，保留既有搜索/选择 owner。模板尺寸是参考证据，不授权替换框架或手绘普通控件。

## 回归边界

真实 Finder/Panel 回归覆盖同关键词最近选择、长关键词前缀学习、同路径跨来源历史、弱可执行匹配、应用去重与普通同名文件、应用单标题与文件父目录、长路径头尾、0/1/多结果窗口尺寸、滚动、旧结果不可执行及静止鼠标选择。测试结果记录在 [任务结果](../../.trellis/tasks/11-04-quick-launch-listary-alignment/outcomes.md)。

## 验证范围与限制

已完成托管调用链、隐藏比较函数和 BAML→XAML 模板核对，并对照 Rust/C# 源码实现本节列明的四项调整。参考模板与编译回归不等于真实机器上的视觉验收。

Listary 原生引擎的 CombinedScore 计算、索引内部排名与缓存格式仍未取得；已知的是托管层怎样使用这些输出。未打包、安装或运行现用产品进行物理键鼠、混合 DPI、原生 HWND 与用户视觉验收。

## 当前实现：四项对齐

- `rust/crates/nxtools-finder/src/ranking.rs`：应用身份来自应用来源，不按任意可执行扩展名加 6500。完整关键词、前缀关键词、良好历史/应用、一般结果与弱匹配分层；具体评分继续使用本项目独立的 FileQuery，不宣称复刻 Listary 原生分数。
- `FinderCatalog::load_history/load_query_history/record_activation`：`app:` 和 `indexed:` 路径历史统一读取，旧 SQLite 数据保留；最近关联优先，写入一次后重启不重复计数。先匹配再合并路径和同名应用快捷方式，普通同名文件及不同命令参数保留。
- `quick_launch/panel.rs::render_result`：使用 Kit0.8.0 官方 `Label` 配合 GPUI-pre0.3.9 的 `.truncate().text_ellipsis_middle()`。底层 `elements/text.rs` 按实际宽度调用 `TruncateFrom::Middle`；完整副标题、Tooltip、无障碍名称及动作目标不变。
- 启动器搜索头高 58 DIP，官方 Large 输入框高 44 DIP，文字 26 DIP、行高 34 DIP、搜索图标 24 DIP；与普通行高 52 DIP、结果图标 32 DIP 配合。应用单标题 16 DIP、文件标题/父路径 15/12 DIP。列表最高 470 DIP；默认分离行模式下一条结果窗口高 132 DIP、三条高 244 DIP，满列表高 530 DIP。空结果状态高 156 DIP，错误页脚额外预留 36 DIP。内嵌文件对话框搜索与紧凑动作行不变。
- `QuickLaunchPanel::resize_to_content` 在发布通知之后调度原有应用；`QuickLaunchApplication::resize_popup_content` 合并未完成的尺寸请求，复用 C# `SetWindowBounds` 的 `SwpNoActivate` 和客户区尺寸核对。关闭、迟到发布、已有预览和搜索焦点仍由原 owner 负责。
- 普通非空查询的尺寸提交与结果发布分离：已有结果区在输入合并、索引或脚本来源尚未完成时保持高度，完成后立即按最终结果调整；没有额外缩高计时器。首次查询的当前结果可展开折叠窗口，尚无结果则维持搜索栏。已可用结果仍提前显示并可操作，清空、隐藏、模式切换及错误状态直接调整。这保留了本项目结果可用性，同时避免中间批次反复驱动窗口伸缩。

## 2026-11-09 approved result motion

The approved [Quick Launch slice](../../.trellis/tasks/11-09-desktop-interaction-polish/quick-launch-slice.md) supersedes the count-driven sizing/settling policy above: ordinary typed search expands once to the existing 530 DIP viewport, independent of result counts and provider completion. Clearing input/hide/reopen retain the compact search-only lifecycle. Result identity/query generation owns one finite ease-out Tween for newly admitted visible rows, including delayed results; offscreen admissions and repaint/selection/icon/reorder/virtual scrolling do not replay it. None/reduced/unknown/disconnected motion finishes immediately. The original virtual list, input, selection, deferred native placement and passive preview owners remain.

Rechecked managed lifecycle: LauncherSearchWindowFactory.cs18 → LauncherSearchWindow.cs24 → SearchWindowBase.cs268 wires Show/Hide/Activate/Close; ShowWindow333–348 calls base.Show and focuses input, ActivateWindow323–329 forces native foreground, Hide274 remains base.Hide. LauncherSearchWindow.cs50–69 owns display positioning and PreviewPopup remains adjacent. This is managed-boundary evidence, not native-engine source. The secondary SwiftList QuickSearchWindowController.cs71–75/127–207/314–404 constructs original position/focus owners, prepares layout/position before Show191, resets preview before query reset and token-checks deferred Hide361/focus restoration367. Next preserves those lifecycle invariants while intentionally changing result geometry/motion.

## Stable indexed publication (2026-11-10)

The managed Listary `Core/ViewModels/SearchMode.cs:701–745` waits for all ordinary launcher providers before replacing results; `Launcher/Class293.cs:214–226` uses that ordinary mode. This is managed UI evidence only; its native search engine is unavailable. The approved Next behavior differs: an indexed query may publish at a bounded deadline before terminal completion, then append later ranked candidates without displacing published identities. Next keeps the original activating primary popup, selected-row/action owner, and one passive preview. `LauncherSearchWindowFactory.cs:18` → `LauncherSearchWindow.cs:24` → `SearchWindowBase.cs:268` binds show/hide/activation; `SearchWindowBase.cs:333–348` calls base.Show then focuses input, `:323–329` requests foreground, and `:274` calls base.Hide. Secondary SwiftList `QuickSearchWindowController.cs:71–75,127–207,314–404` creates the foreground/placement owners, stages position before Show, and retires preview/query before deferred Hide. The old `D:\Code\NxTools\Views`/`ViewModels` tree contains no Quick Launch entry point; `Views\ClipboardPreviewWindow.cs:29–40,93–95` is a different passive clipboard window, not a Quick Launch implementation.

Next's original `quick_launch/panel.rs::schedule_search/run_search` owns input debounce, generation, selection and retained-row execution gating; `quick_launch.rs::search_and_request_icons/handle_indexer_notification/refresh_indexed_results` owns index request, accumulated batches and merged publication. Keep recent-directory/action-mode layout, cancellation, current provider availability, default-first/explicit identity, and error/disconnect semantics. The first publication waits for terminal or ~150 ms; intermediate batches only accumulate. An indexed query has at most a deadline and terminal indexed publication. Stable rows remain actionable until input supersedes them; new results append, and normal-to-phonetic fallback cannot replace already shown rows. Timer callbacks must respect query generation/input/window session and cannot synchronously write into a leased GPUI panel.

## Installed black-box comparison entry points (2026-11-17)

Read-only recheck of the installed 6.3.5.94 managed boundary: `Core/Services/InternalNamedPipeService.cs:32–65,87–90` forwards `[url-schema]listary://show-launcher` through `ListaryInternalNamedPipe`; `UrlSchemaService.cs:63–66` reaches `Class292.cs:132–156` and its original `Show(true, false)` owner. `LauncherSearchWindowFactory.cs:18–20` constructs `LauncherSearchWindow.cs:24–32`; `SearchWindowBase.cs:268–278,323–348` binds the existing show/hide/activate/close and native foreground/input-focus lifecycle. This is the same local URI dispatch as the installed shell registration, not an index-engine API or an authorization bypass.

The installed launcher exposes an editable `ValuePattern` at `PART_SearchBox` and visible `ListItem` results. Titles and displayed parent paths are separate descendants; displayed parents may be truncated. The more-results `Class177` row is navigation, not a ranked file. Next exposes its unnamed `Zed::Window` by process/HWND, an editable `速启搜索` control and result buttons labelled `name，target` (`quick_launch/retained_text.rs:51`). Its existing PlatformHost message-only event window accepts the original quick-launch hotkey request; `WindowsPlatformAdapter.EventWindow.cs:73,509–510,2579–2640` retains summon policy, context capture and application ownership.

An external benchmark may set those real input values and read the visible accessibility results without executing an item, changing history/settings, replacing providers or restarting services. Clearing must retire the preceding rows before another sample; focus loss or operator input invalidates the sample. Measure input-to-accessible-result separately from physical keyboard/IME and paint timing. Normalizing both products to top five does not establish complete recall. Corpus membership must first be verified by literal lookup in both installed products. Existing history, index scope, working-tree/package correspondence and provider readiness remain separate evidence; Listary native engine source is unavailable.

Next's installed UIA value update is asynchronous: `ValuePattern.SetValue` may return before the input publishes its new value. Await the requested value after both query assignment and clearing without replaying the action. The benchmark clock starts before the initial call, so acknowledgement remains part of response time. The external runner and its evidence levels are documented in [the installed comparison method](../quick-launch-listary-benchmark.md).

## Windows application discovery (2026-11-20)

The two-target Notepad/Settings slice above does not cover Windows application discovery. Rechecked canonical `D:\Code\Listary.Decompiled\6.3.5.94\managed\Listary\Core\Launcher\Class291.cs:23-103,146-173`: FOLDERID_AppsFolder binds an item enumerator/property store, requires package family plus AUMID, reads the localized display name and releases COM objects. The refresh executes in an STA. This is managed application-boundary evidence, not the unavailable native search engine. Legacy NxTools has no Quick Launch implementation.

Next's supported boundary is a native installed-application snapshot merged by the original Rust Finder catalog. Actual installed membership is not a hardcoded app list. Known bilingual aliases are metadata only; absent packages must not become invented entries. Classic Calculator/Paint use their stable Windows dispatchers when no installed package supersedes them. Native failure retains last-good data; ordinary filesystem refresh preserves the installed snapshot. Query replacement refreshes only the current query; disable/disconnect/timeout retire pending ownership without reveal, focus or history mutation. Namespace activation remains ordinary Shell execution, not filesystem reveal/elevation/preview. Regression task: `.trellis/tasks/11-20-windows-app-discovery/`.

Current discovery path: `PlatformCommand::QueryInstalledApplications` → detached
`PlatformPipeClient.ExecutePlatformCommandAsync` → `WindowsPlatformAdapter.QueryInstalledApplications`
→ `InstalledApplicationQuery.ExecuteBounded` → `AppsFolderApplicationSource.Enumerate/ReadFacts`.
Native facts return in original correlated command-result data; `quick_launch/installed_applications.rs`
validates current connection/request ownership and calls Finder's atomic
`replace_installed_applications`, followed by the existing current-query refresh. Root refresh
preserves installed rows; native failure preserves last-good membership. No result activates,
shows or focuses a surface. AppsFolder Open uses original `ShellTarget.TryCreate/LaunchCore`;
file-only capabilities remain unavailable. `windows_apps_`, Finder `windows_applications` and the
C# `installed applications fixture` are synthetic boundary evidence, not installed-native acceptance.

## Packaged application icons (2026-11-20)

Rechecked the same managed 6.3.5.94 root before changing Next's icon path:

| Boundary | Verified source and invariant |
| --- | --- |
| Namespace identity | `Core/Launcher/Class291.cs:27–71,146–173` reads AppsFolder package-family/AUMID/display-name facts on an STA and retains `SHGetIDListFromObject` in `Class299.Class127_0`. `Core/Class126.cs:38–42` frees the owned PIDL with `FreeCoTaskMem`. |
| Result to view | `Core/Search/Class262.cs:20–54` creates `Class261`; `Core/ViewModels/Class179.cs:65–68` creates `Class176`. `DynamicIconSizeSearchItemViewModelBase.cs:30–49` reloads on positive physical-size changes. |
| Size and rendering | `Main/Views/LauncherSearchWindow.xaml:42–51` uses a 32-DIP IconViewer with physical width bound back to IconSize. `Views/CalculatePhysicalSizeBehavior.cs` owns the device-pixel conversion. |
| Native pixels | `Core/ViewModels/Class176.cs:47–57` passes PIDL plus requested size and ICONONLY=4 to `Core/Class141.cs:40–100`. `Class141:170` uses `Class124(2)`; `System/Threading/Tasks/Schedulers/Class124.cs:69–74` creates STA workers. `SHCreateItemFromIDList` requests IShellItemImageFactory (`bcc18b79-ba16-442f-80c4-8a59c30c463b`), then GetImage SIZE/flags. E_PENDING retries are bounded to five 200-ms waits; HBITMAP conversion always deletes the bitmap, and the async result is frozen (`Class141:425–438`). |
| Activation is separate | `Core/Launcher/Class299.cs:63–78` uses ApplicationActivationManager only for explicit execution, not icon loading. |

Next keeps its original `ExtractShellIcon { target, size }`, visible-range prefetch, resource coalescing, result-ID PNG binding, deferred cache-hit delivery, 96-entry Rust LRU, bounded native STA and shared IconCache. A valid exact `shell:AppsFolder\family!application` is an icon-capable namespace identity independently of file preview, Shell menus, path copy, reveal and elevation. C# resolves that parsing name directly to IShellItemImageFactory; it never treats the AUMID suffix as an extension or canonicalizes it with GetFullPath. Unknown installed applications use their own complete namespace identity, without an artwork/name allowlist. Failure keeps a static semantic application glyph, not a loading-shaped glyph.

The primary popup constructor/show/hide/activation and passive preview lifecycle remain the existing owners documented above; icon admission neither creates a window nor changes placement, focus, selection, cancellation or activation. Existing correlated platform dispatch/result diagnostics retain native HRESULTs and fixed failure categories without logging targets. This is managed Shell-boundary evidence, not Listary native search-engine source. Controlled factory/PNG fixtures and source tests do not establish installed AppsFolder, mixed-DPI or user visual acceptance.
