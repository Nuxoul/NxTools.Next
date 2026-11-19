# Listary 文件对话框搜索条与 Auto Quick Switch 参考研究

> 研究日期：2026-07-24<br>
> 参考版本：本机 `D:\Program Files\Listary\Listary.exe`，文件版本 `6.3.5.94`<br>
> 主要证据：固定目录 `D:\Code\Listary.Decompiled\6.3.5.94\managed\Listary` 中的托管反编译边界；窗口代码对应 `.xaml.cs`，展开状态机见同版本 `expanded\Listary`<br>
> 次级证据：`D:\Code\SwiftList-main` 开源代码

## 结论

用户描述的行为确实是两个独立能力，不能归并为已经完成的“在速启结果动作里切换到此目录”：

1. **文件对话框内搜索 UI**：识别 Windows “打开/另存为”对话框后，为该宿主创建一条独立的固定搜索栏，停靠在宿主窗口底部；输入后在搜索栏附近展示结果。
2. **Auto Quick Switch / 目录同步**：文件对话框打开期间，用户先切到资源管理器浏览某目录，再切回原文件对话框时，文件对话框自动导航到资源管理器刚才的目录。

NxTools.Next 阶段 4 当前已经具备的“速启面板结果 → 切换到此目录”属于**显式动作**；它没有覆盖上面两项。两项都应作为阶段 4 的补充纵向切片完成，不应推迟到阶段 5 的“命令与扩展搜索源”。

## 术语与边界

| 能力 | 触发 | UI | 目录数据流 |
|---|---|---|---|
| 文件对话框内搜索 | 文件对话框被识别/在其中直接输入 | 对话框底部的独立固定搜索条 | 全局搜索；当前文件对话框目录只用于排序加权；选择结果后由对话框能力打开目录/文件 |
| Auto Quick Switch | `支持的目录来源 → 同一个文件对话框` 的前台切换 | 固定栏默认项可显示最近来源 | `OpenedFolderProvider.GetCurrentFolder()` → `FileDialog.OpenFolder(path)` |
| 已实现的显式“切换到此目录” | 双击 Ctrl 呼出速启，再选择动作 | 全局速启面板 | 当前速启会话冻结的宿主 HWND + 被选结果解析出的目标目录 |

三者可以复用同一套 Windows 宿主适配器，但生命周期、触发条件和 UI 不是一回事。

## Listary 托管调用链

### 一、文件对话框内搜索 UI

#### 1. 宿主发现与绑定

`Core/Class133.cs:79-113` 每 30 ms 取得当前前台窗口与其根窗口；只有 HWND 变化时才在 UI 调度器上发出对应事件。
`Core/HookedWindows/Class343.cs:61-70,98-151` 订阅这些事件，对新的根窗口依次调用已注册的文件应用插件 `BindFileWindow`，识别成功后由 `Class342.method_0` 创建宿主控制器。

标准文件对话框的内建识别入口为：

- `Core/HookedWindows/Implementations/Class368.cs:8-130`
- 反射还原后的显示名/ID：`File dialog` / `FileDialog`
- 顶层类必须是 `#32770`
- 控件 ID `1120` 必须存在且类名为 `ListBox`
- 控件 ID `1091` 的类名必须为 `Static`
- `Boolean_0 == false`：不是“已打开目录来源”
- `Boolean_1 == true`：是 Quick Switch 目标
- `Enum7_0 == const_0`：映射为 `SearchBarType.Fixed`

这说明 Listary 不会仅凭 `#32770` 就附着；它还检查文件对话框模板结构，避免把任意普通对话框误认为文件对话框。

#### 2. 创建固定搜索栏，而不是复用全局启动器

调用链：

```text
Class343.method_4
  -> Class274.Plugin.BindFileWindow(hwnd)
  -> Class342.method_0(...)
  -> plugin.SearchBarType == Fixed
  -> new Class335(...)
  -> Class165.imethod_0(Class334)
  -> FixedSearchBarViewModel
  -> Class335.method_15(viewModel, IDialogSearchWindowFactory)
  -> DialogSearchWindowFactory.CreateWindow
  -> new DialogSearchWindow(viewModel)
```

证据：

- `Core/HookedWindows/Class342.cs:35-54`：`Fixed` 宿主走 `Class335 + FixedSearchBarViewModel + IDialogSearchWindowFactory`，浮动文件管理器搜索走另一条 `Class336` 链路。
- `Core/ViewModels/Class165.cs:26-41`：构建固定搜索模型，并加入文件对话框默认项/Quick Switch 搜索提供者。
- `Core/HookedWindows/Class335.cs:45-58`：创建窗口后立即 `Show(false, false)`，并监听宿主边界和可见性事件。
- `Main/Views/DialogSearchWindowFactory.cs:7-13`：创建独立 `DialogSearchWindow`。
- `Main/Views/DialogSearchWindow.cs:21-31`：它继承通用 `SearchWindowBase`，但拥有自己的搜索框、结果 Popup 和附加信息 Popup。
- `Main/Views/SearchWindowBase.cs:26-33,333-349`：基类构造时先设置
  `Window.Topmost = true`；`ShowWindow(false, false)` 再设置 `ShowActivated = false`
  并调用 `Show()`。因此 Listary 的首次可见性依赖“一次性 Topmost + 非激活显示”，
  不是在每次位置更新时把窗口插到 `HWND_TOP`。

#### 3. 搜索栏停靠到宿主底边

`Class335.method_17`（`Core/HookedWindows/Class335.cs:126-130`）将宿主 HWND 的边界写入 `SearchBarViewModel.PlacementBounds`。宿主位置变化时，`Class335.Class317.method_1` 会重新计算边界。

`DialogSearchWindow.ah`（`Main/Views/DialogSearchWindow.cs:41-69`）执行：

```text
Top  = PlacementBounds.Bottom
Left = PlacementBounds.Left + (PlacementBounds.Width - SearchBar.Width) / 2
```

因此 Listary 的搜索条是一个独立窗口，视觉上居中贴在文件对话框底边，并不是把控件真正插入目标进程的窗口树。搜索结果使用独立 `Popup`；宿主缩放、DPI 或搜索栏尺寸改变后会重新定位。

补充核对 `DialogSearchWindow.cs:99-126` 的生成连接表后可以确认，固定搜索头只连接
`PART_SearchBox` 与一个 `RightSearchLogo` 组件；它没有三个可独立点击的
“收藏/历史/管理搜索”按钮。NxTools.Next 在这些动作尚未实现时不得仅凭参考截图添加
三个无行为图标。另一个关键点是 Listary 在 `SizeChanged` 后再次调用 `ah()`，并以
`ActualWidth` 计算中心；迁移时必须以最终可见窗口边界回读校正，不能只验证预设外层
HWND 的数学坐标。

#### 4. 搜索上下文与默认目录项

`Core/ViewModels/Class168.cs:70-87` 从当前宿主获取：

- `IGetFolder`
- `IOpenFolder`
- `IOpenFolderAndSelectFile`
- `IOpenFile`

并异步取得当前目录，构造宿主相关的 `SearchMode`。`Class168.method_3` 随后加入多种全局搜索提供者；当前目录并不是硬性的搜索范围。`Class249`（`Core/Search/Class249.cs:89-122`）只在排序时优先父目录等于文件对话框当前目录的文件项，其他索引结果仍然可见。

`Core/ViewModels/Class165.cs:26-41` 在创建固定搜索模型时还会加入默认项提供者。缺省实现 `Core/Search/Class228.cs`：

- 从 `Class345.Class334_0` 取得最近一次激活的 `IsOpenedFolderProvider`；
- 在显示时通过该来源的 `IGetFolder.GetCurrentFolder()` 重新读取目录；
- 读取 Shell 显示名，并生成 `<目录名> (<来源插件 DisplayName>)`；
- 将项目的 `PreferredHotkey` 设置为 `ListarySettings.QuickSwitchHotkey`，其默认值是 `Ctrl+G`；
- 点击或按 `Ctrl+G` 后，用目标文件对话框的 `IOpenFolder` 导航到该目录。

所以固定栏的空查询状态不是只有输入框：只要存在有效的最近目录来源，下方就保留一条可执行的 Quick Switch 目录项。

#### 5. 截图中的固定栏视觉结构

用户提供的 Listary 参考截图：

`C:\Users\Administrator\AppData\Local\NxTools\clipboard\images\2eeac9e1f93f04696ba26a607451c229e7964c8b46ef7acda867e647db5e18e7.png`

可测量的视觉结构约为：

- 总宽约 500 px，水平居中贴在文件对话框可见边界的正下方；
- 搜索头部约 36 px 高，左侧是搜索/产品图标，右侧是紧凑工具图标；
- 默认目录项约 35–36 px 高，文件夹图标、`目录名 (来源)`、截断路径和右侧 `Ctrl+G` 在同一行；
- 空查询且存在来源时总高约 72 px；没有来源时只显示搜索头部；
- 输入查询后使用全局索引结果，文件对话框当前目录只影响优先级，不把搜索限制在该目录。

截图中的 `ds (Directory Opus)` 同时证明来源并不限于 Windows Explorer。`Core/HookedWindows/Class334.cs` / `Class345.cs` 的共享状态保存的是通用 `IsOpenedFolderProvider` 控制器；Explorer、Directory Opus 或其他实现该能力的插件都能成为来源。

这条默认目录项属于固定搜索能力本身，不应依赖“自动切换”开关是否开启；关闭 Auto
Quick Switch 只能禁止自动导航，不能让最近目录来源从固定搜索条中消失。

Directory Opus 等双栏文件管理器的来源必须冻结真实活动面板。前台切换瞬间若焦点 HWND
尚未建立，可以在短时预算内重新读取焦点；不能因为当前只有一个可见容器就退化为
“取第一个面板”，否则双栏、标签切换和延迟焦点场景会跳到错误目录。

### 二、Auto Quick Switch / Explorer 到文件对话框目录同步

#### 1. Explorer 是“目录来源”，文件对话框是“同步目标”

Windows Explorer 的内建入口是：

- `Core/HookedWindows/Implementations/Class370.cs`
- 反射还原后的显示名/ID：`File Explorer` / `WindowsExplorer`
- 识别 `ShellTabWindowClass`，其父窗口必须为 `CabinetWClass` 或 `ExploreWClass`
- 创建 `Class329(WND_EXPLORER)`
- `Boolean_0 == true`，经 `Class276.IsOpenedFolderProvider` 映射为“已打开目录来源”
- `Boolean_1 == false`，它本身不是 Auto Quick Switch 目标

标准文件对话框 `Class368` 的两个标志正好相反：不是来源，但可以作为目标。

#### 2. 共享状态是“来源 + 一次性待同步标记”

`Core/HookedWindows/Class345.cs` 只有两个状态：

- `Class334_0`：最近激活的目录来源控制器
- `Boolean_0`：来源更新后是否还有一次待消费的同步

给 `Class334_0` 赋值会同时把 `Boolean_0` 设为 `true`。

每个已绑定宿主在 `Class334.method_2`（`Core/HookedWindows/Class334.cs:183-200`）订阅根前台窗口变化：

- 当前宿主若是 `IsOpenedFolderProvider`，就将自身写入 `Class345.Class334_0`。
- 当前宿主不是来源，则先清掉待同步标记，等待以后由来源重新置位。

#### 3. 切回文件对话框时执行一次同步

`Class334.Class337.method_0`（`Core/HookedWindows/Class334.cs:482-503`）是核心状态机：

```text
if activated_root != this_host_root:
    return

if this_host.IsOpenedFolderProvider:
    shared.Source = this_host       // 同时 Dirty = true
else if shared.Dirty:
    if shared.Source != null:
        this_host.AutoSwitchFrom(shared.Source)
    shared.Dirty = false
```

真正的数据流在 `Class334.method_13`（`Core/HookedWindows/Class334.cs:362-393`）：

```text
source.smethod_0() -> IGetFolder
await source.GetCurrentFolder()
target.smethod_1() -> IOpenFolder
await target.OpenFolder(path)
Track(AutoQuickSwitchTrackingEvent)
```

因此用户描述的可观察行为与代码吻合：

1. 打开文件对话框；
2. 切到 Explorer；
3. Explorer 成为新的 `OpenedFolderProvider`，保存其控制器并将状态标脏；
4. 再切回原文件对话框；
5. 文件对话框消费这一次待同步状态，读取 Explorer 当前目录并在原对话框内导航；
6. 清掉 dirty 标记，同一份来源不会因为对话框内部焦点切换而反复同步。

这不是“聚焦时永远取最后一个缓存字符串”的简单实现。Listary 保存的是能够现查当前目录的来源控制器；真正切回目标时才异步读取路径，减少用户在 Explorer 内继续浏览造成的过期路径。

#### 4. 文件对话框实际导航边界

标准文件对话框的托管适配器链为：

```text
Class368 -> Class332 -> Class330 -> Class329 -> Class327 -> Class319/Class307
```

- `Class327.imethod_0`（`Class327.cs:120-155`）通过 Shell PIDL 读取当前目录。
- `Class329.imethod_1`（`Class329.cs:21-29,45-56`）将路径解析为 Shell 项并调用 `Class307.method_11` 导航原 HWND。
- `Class332` 额外实现 `IOpenFile`；它取控件 ID `1148` 的文件名输入框，先恢复目标为前台，再交给 `Class307.method_16` 输入/提交文件。

这里能确认托管职责和调用方向，但 `Class307` 最终依赖 Listary 的 Hook/Interop 边界；`D:\Program Files\Listary\Hooks\ListaryHook64-6.1.10.1.dll`、`ListaryHook32-6.1.10.1.dll` 以及部分 Shell 辅助实现不是本次托管反编译输出中的可审计源码。因此：

- 可以确认它在**原文件对话框 HWND** 上执行目录导航；
- 可以确认 Get/OpenFolder/OpenFile 是分离能力；
- 不应声称已经掌握所有消息、注入、跨位数桥接和失败恢复细节。

同样，`ListarySettings.EnableAutoQuickSwitch` 默认是 `true`，但在当前托管反编译输出中没有找到完整的运行时开关接线；不能据此虚构其所有设置传播细节。

## SwiftList 次级开源参照

SwiftList 不是第一行为标准，但其开源结构印证了上面的能力拆分。

### 文件对话框适配器

`PluginSdk/Abstractions/Plugins/WindowAdapters/IFileDialogAdapter.cs` 将以下能力放入同一宿主适配器：

- `CanHandle`
- `GetCurrentPath`
- `NavigateTo`
- `GetDockBounds`
- `RestoreFocus`

`Plugins/CoreExtensions/FileDialog/StandardFileDialogAdapter.cs` 以顶层 `#32770` 加 `Breadcrumb Parent` 识别现代文件对话框；`ClassicFileDialogAdapter.cs` 则排除带 Breadcrumb 的窗口，并额外要求标准文件名控件和 ComboBox，避免误识别普通 `#32770`。

### Auto Quick Switch

`Core/Hook/ExplorerIntegration/ExplorerWindowClassifier.cs:36-41,78-119,203-234`：

- 识别 Explorer/文件对话框；
- Explorer 激活时更新最近目录；
- 文件对话框激活时把 `previousWasPathProvider` 交给 `FileDialogNavigationTracker`；
- 再用适配器读取文件对话框自身目录，刷新搜索范围。

`Core/Hook/ExplorerIntegration/FileDialogNavigationTracker.cs:19-55`：

- 为每个对话框 HWND 保存首次发现时间；
- 新对话框第一次出现时不自动导航；
- 旧对话框重新激活，且之前是路径来源或 Explorer 路径在其首次出现后更新过，才调用 `adapter.NavigateTo(dialogHwnd, lastExplorerPath)`；
- 后台投递导航，避免阻塞前台事件处理。

该实现与 Listary 的可观察行为一致，但内部状态模型不同：SwiftList 以 HWND/时间戳和路径快照为主，Listary 以“来源控制器 + 一次性 dirty”状态为主。NxTools 应借鉴行为不变量，不必照搬 SwiftList 的具体时间戳实现。

### 对话框底部搜索条

`App/Services/InlineSearch/InlineSearchManager.cs:63-107` 在识别到文件对话框时创建/刷新独立 `InlineSearchWindow`；`144-208` 用当前对话框路径作为搜索范围，并在不抢走文件名编辑框焦点的情况下显示窗口。

`App/Views/InlineSearchWindow/Helpers/InlineSearchWindowPositioner.cs` 使用适配器提供的宿主边界居中贴近底部；文件对话框与普通 Explorer 的垂直放置模式分开处理。它进一步印证“外置独立窗口 + 宿主边界停靠”，而不是跨进程注入 WPF 控件。

## NxTools.Next 的实现不变量

### 文件对话框内搜索 UI

1. 只有严格识别成功的打开/保存对话框才显示；任意普通 `#32770` 不得附着。
2. 它是 GPUI 创建的独立辅助窗口，不向第三方进程注入 UI；C# 只提供 HWND、宿主分类、边界、路径和导航能力。
3. 搜索条在对话框底部居中停靠；宿主移动、缩放、DPI/显示器变化时更新位置。
4. 空查询显示紧凑搜索条；存在有效的最近目录来源时，下方还显示一条可点击并可由 `Ctrl+G` 执行的默认目录项。
5. 搜索使用全局索引；当前文件对话框目录仅作为排序加权，不能变成硬性范围。路径改变后更新排序上下文并使旧搜索代次失效。
6. 面板不应抢走文件名输入框的正常输入。需要接收搜索输入时，必须定义明确的激活/焦点策略，并在关闭或执行后恢复目标对话框焦点。
7. 结果动作使用宿主能力：目录结果导航目录；文件结果按“打开/选择”语义交给对话框，不启动第二个 Explorer。
8. 文件对话框创建期间，顶层 HWND 可能已成为前台，但模板子控件和最终窗口边界仍未就绪。识别重试必须在有界预算内保留目标 HWND；一次瞬时的前台/根窗口不一致不能直接丢弃该目标，窗口销毁或预算耗尽才终止。
9. 宿主拖动链路必须与结构识别链路分离。`Class133` 只在根前台 HWND 改变时触发绑定，`Class335.Class317.method_1` 则只对已绑定宿主重新计算位置；不能在每个子控件 `LOCATIONCHANGE` 上重新执行完整模板扫描、路径读取和绑定。
10. 高频根窗口位移应在 Windows 适配器内通过单个待处理窗口消息合并，并复用 Rust 已给出的可见尺寸/结果重叠量完成原生 HWND 跟随。不能增加固定 16 ms 定时等待，因为宿主自身可能按更高刷新率移动，Windows 定时器粒度还会把一帧延迟继续放大。Rust 仍拥有布局尺寸，C# 只执行实时 Win32 定位；普通移动不应往返发布宿主事件再重复设置 owner 与 visibility。
11. 已严格识别且当前仍可见的同一个文件对话框收到重复前台/结构唤醒时，必须复用已绑定控制器，不得重新扫描模板控件。2026-07-24 的 NxTools 实测拖动跟踪在 2 秒内重复识别 14 次，峰值 11 次/秒，单次完整扫描 66.27 ms；这与 `Class133` 的“根 HWND 未变则不重新发出绑定事件”不变量相反，也是拖动掉帧而非定位 IPC 的直接证据。
12. 固定栏的移动/缩放不得改变前台窗口或 Z 序。Listary 的 `Class335.method_15`
    只创建并 `Show(false, false)` 一次，`Class317.method_1 -> method_17` 在宿主边界变化时
    只更新 `PlacementBounds`；`DialogSearchWindow.ah` 只写 `Top/Left`。因此 Win32 跟随必须
    保留 `SWP_NOZORDER | SWP_NOACTIVATE`，不能用 `HWND_TOP` 掩盖首次显示时序问题。
13. 固定栏首次创建与后续跟随是两个阶段。GPUI 以隐藏状态创建原生窗口时，owner、
    最终设备像素边界和可见性只能在该窗口的首次提交完成后应用；不能在
    `open_window` 的同一个应用更新中先 `ShowWindow`，再让 GPUI 的初始 `show: false`
    提交把它重新隐藏。后续宿主边界事件只复用已经绑定的窗口并定位，不重新创建。
14. 固定栏显隐由文件对话框宿主的可见生命周期统一驱动。Listary `Class335` 监听宿主
    `Event_2` 后调用 `method_16`，不会再让另一个通用前台处理器独立隐藏同一窗口。
    NxTools 的 C# 宿主识别器已经发布 `visible=true/false` 后，Rust 不得再对同一条
    `ForegroundChanged` 重复发送隐藏命令；NxTools 自身辅助 HWND 也不能被当成宿主离开。
15. 高频拖动只允许读取一次宿主可见边界并执行一次非激活定位。初次绑定可以校准
    GPUI HWND 的非客户区边距；校准结果应随绑定会话复用。普通跟随不得每帧重新读取
    搜索窗外框/可见框、二次校正、再次刷新宿主几何并拼接诊断字符串。
16. 固定栏创建完成后的首次原生提交必须复现 `SearchWindowBase` 的窗口层级语义：
    对隐藏 HWND 一次性设置 `TOPMOST`，但保持非激活，然后设置 owner、最终边界并用
    `SW_SHOWNOACTIVATE` 显示。`TOPMOST` 只属于首次初始化；后续位置跟随仍使用
    `SWP_NOZORDER | SWP_NOACTIVATE`。宿主离开前台时必须隐藏该窗口，不能让 Topmost
    辅助栏悬浮在其他应用之上。
17. 固定栏查询与速启面板走同一条搜索管线：本地目录源（应用、自定义命令、计算器、环境变量、
    声明式扩展）与索引请求使用相同的准入、`retain_kinds_apart`、同音回退和 150 ms 首次合并期限，
    合并结果由 Finder 统一排序并保持已发布行的位置（`unified_catalog_results`、
    `stable_indexed_results`）。对话框差异只保留四点：当前目录作为 `preferred_directory`，并在
    统一排序后把父目录为当前目录的索引行排到其它索引行之前，应用等非索引行位置不变；`recent:`
    保持普通文本；文件夹/文件结果交给原对话框导航/打开，其它结果走速启动作执行；生成号带
    `FILE_DIALOG_SEARCH_GENERATION_MASK` 路由。

### Auto Quick Switch

1. 第一次创建/第一次看到文件对话框时不应无条件跳到历史 Explorer 目录。
2. 只在明确发生 `支持的目录来源 → 同一个仍有效的文件对话框` 转换后同步；来源包括 Explorer 和已适配的第三方文件管理器。
3. 来源目录必须在来源激活/路径改变时刷新，或在目标激活时从冻结的来源控制器再读取；不能从已经成为前台的辅助搜索窗口重新推断来源。
4. 一次来源更新只触发一次目标同步；对话框内部子控件焦点变化不能重复导航。
5. 目标 HWND 必须仍有效且仍匹配原适配器；不能把目录发给另一个同类对话框。
6. 路径为空、不存在、非本地文件系统或导航失败时保持原对话框目录，不自动打开 Explorer。
7. 同步不替用户按“打开/保存”，也不覆盖文件名字段。
8. 导航完成后保持/恢复原文件对话框为前台；用户应能继续操作它。
9. 只有已识别的宿主激活才改变同步状态：目录来源激活会替换/置位，文件对话框激活会消费，新对话框首次出现会清除。任务栏、Alt+Tab 切换器、对话框所属程序主窗口等无关前台既不消费也不清除待同步标记，也不作废已冻结的 Explorer 来源（对应 `Class337.method_0` 的 `activated_root != this_host_root → return`）。
10. 最近来源是 Directory Opus 窗口时，该窗口再次激活但当前面板暂时无法识别（焦点在标题栏、工具栏或文件夹树），保留来源并重新置位待同步；切回对话框时由 `dialog_return` 刷新重新读取活动面板，只有该刷新失败才放弃本次同步。

### 职责边界

- **Rust 核心**：定义宿主会话、路径来源/目标状态机、一次性同步代次、搜索范围和取消语义。
- **C# Windows 适配器**：识别文件对话框/Explorer，取得窗口边界和当前路径，在原 HWND 上导航并恢复焦点。
- **GPUI**：绘制底部搜索条、结果层、动画和交互；不直接做 Win32/COM/UI Automation。

## 建议验收矩阵

1. 打开对话框首次出现：有底部搜索条，但目录不被历史状态擅自改写。
2. 对话框 → Explorer A → 原对话框：自动跳到 A。
3. 对话框 → Explorer A → Explorer B → 原对话框：只跳到 B。
4. 已同步后在对话框内部切换文件列表/文件名框：不重复跳转。
5. 两个文件对话框同时存在：返回哪个 HWND，只同步哪个 HWND。
6. 普通 `#32770`（查找、设置、确认框）：不显示搜索条、不获得目录同步。
7. 对话框移动、缩放、跨显示器/DPI：搜索条保持贴底居中。
8. 搜索目录并执行：在原对话框导航；搜索文件并执行：由原对话框选择/打开，不能另起 Explorer。
9. Explorer 来源在同步前关闭，或目录已删除：不崩溃、不错误导航，记录可诊断失败。
10. 对话框关闭后：辅助搜索条和结果层立即销毁，旧 HWND/路径不得泄漏到下一次会话。
