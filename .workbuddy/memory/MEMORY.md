# NxTools.Next 项目长期笔记

## GPUI 固定尺寸窗口约定
- 新建下载对话框、链接识别面板等都是固定尺寸不可调窗口；高度预算必须从渲染结构逐项推导（标题栏/行高/padding/页脚），优先让渲染代码与预算函数共用同一组常量，避免再出现"内容被窗口裁掉/下方空白"类 bug。
- 错误行预留高度要叠加在当前实际渲染的布局高度上（紧凑 vs probe 确认布局高度不同），见 `new_download_error_window_size(confirmed)`。
- 确认态 body 必须带 `.min_h_0().overflow_hidden()`，保证固定窗口内页脚永远不会被 flex 溢出推出窗外。
- 测试注意：GPUI 测试环境 `window.resize()` 只改缓存 bounds 不改绘制表面；做元素几何断言前需 `window.simulate_resize(...)`，否则断言测量的是溢出布局（曾导致旧测试掩盖页脚被推出窗口的 bug）。

## 文件与工具约定
- **换行符必须 LF**：`.gitattributes` 是 `* text=auto eol=lf`，`.editorconfig` 是 `end_of_line = lf`。批量编辑脚本用 `read_text()/write_text(newline="")` 会把文件改成 CRLF，必须显式保持 LF（或直接二进制替换 `b"\r\n" -> b"\n"`）。
- 判定行尾别用 `grep -c $'\r'`：本机 Git Bash 下会误报（把所有行都算命中），要用 Python 数 `b"\r\n"` 与裸 `b"\n"` 的字节数。
- 主仓库 `target\` 偶发被锁（`hyper-util*.rcgu.o`，os error 5），主仓库全量构建约 27 分钟；超时/失败后先 `tasklist` 查残留 cargo/rustc，再决定是否重启构建，不要盲目重复启动。

## gpui-kit 0.6.1 迁移（并行目录 D:\Code\NxTools.Next-gpui-kit）
- 迁移不在主仓库内进行，落在与主仓库平级的 `D:\Code\NxTools.Next-gpui-kit`（无 git），**未提交未合并**。
- 依赖替换：`gpui = { package = "gpui-pre", version = "0.3.4" }` + `gpui-component = "0.6.1"` + `gpui-kit = "0.6.1"`（gpui-kit 0.6.1 底层要求 Longbridge 的 gpui-pre 分支，不是官方 crates.io 的 gpui）。
- 框架差异要点（迁移时反复踩到）：
  - `Bounds::contains` 在 gpui-pre 是**半开区间**，0.2.2 是闭区间；贴齐右/下边缘的元素判断要用 `ui::bounds_contains`。
  - `InputState::set_value` / `TextareaState::set_value` 不再发 `InputEvent::Change`，程序化写值必须走 `ui::set_input_value` / `ui::set_textarea_value`。
  - `Entity<V>` 只 impl `IntoElement`，`window.draw` 闭包返回值要 `.into_element()`。
  - 面板外框的 1px 边框在 gpui-pre 里画在盒内（内容内缩 1px），相关像素断言用 `ui::PANEL_FRAME_BORDER`。
  - taffy 0.9→0.13：taffy 0.9 会在 flex 列里把负空间摊到子项上、**无视子项 `min-height`**（曾把设置行从 42px 压到 38.5px）；0.13 遵守 min-height，因此内容真的会溢出并滚动。判断布局差异时先想这一条。

## 文档归属（重要，别再搞错）
- `docs/ui-remediation-plan.md` 是**上一个 UI 整改任务**的旧文档（含 DS-01..15 / SH-01..08 / DL / QL / CL / EX 各批次），**不是当前 gpui-kit 迁移任务的计划**。用户已明确纠正。不要拿它的"批次 1–6"当作当前迁移的阶段口径。
- 当前任务只有一个轴：**把 GPUI 全面换成 gpui-kit**（依赖栈 + 组件层 + 令牌层），并按用户提供的 GPUI Kit v2 设计规范重构前端。

## gpui-kit 迁移进度（对照上面的轴）
- **渲染/组件依赖栈：已完成**。`gpui`→`gpui-pre 0.3.4`、`gpui-component 0.5.1`→`0.6.1`、新增 `gpui-kit 0.6.1`；入口 `gpui_kit::application()` + `gpui_kit::init(cx)`。
- **令牌层：已完成**。`ui/typography.rs` 七级字号、`ui/metrics.rs` 4px 网格间距 + 圆角阶梯、`theme_palette.rs` 五套主题（one-dark/catppuccin-mocha/nord/zed-dark/github-light，默认 One Dark）经 `apply_semantic_config` 注入 gpui-component 的语义层。
- **组件层：未迁移，这是剩余主体**。`rust/apps/nxtools-desktop/src/ui/components/` 共 **3106 行 / 15 个模块**仍是自研包装：`custom_command_editor 534`、`quick_launch_diagnostics 335`、`list 274`、`path_list 266`、`settings 237`、`volume_scope 235`、`action 233`、`state 223`、`hotkey 214`、`titlebar 179`、`field 174`、`menu 83`、`dialog 58`、`popover 47`。另有 31 个文件直接使用 `gpui_component::`（这部分其实已在 gpui-kit 上），12 个文件使用自研 `crate::ui::components`。
- `gpui_kit::` 在源码里只出现 2 次（application / init）——因为 gpui-kit 本身是门面，`gpui_component::X` 即等价于 `gpui_kit::component::X`，**不必刻意改用 `gpui_kit::` 前缀**。
- **不要按行数判断"可替换"**：核对依赖后实际情况是——
  - **已是 gpui-component 薄包装（只需收敛、不需重写）**：`action.rs`（用 `button`+`tooltip`）、`field.rs`（用 `input`）、`menu.rs`（用 `menu`）。
  - **真正自绘、gpui-component 有原生等价物 → 应替换**：`settings.rs`（`setting::{SettingPage,SettingGroup,SettingItem}`）、`popover.rs`（`popover::Popover`）、`list.rs`（`list`/`radio`/`badge`/`tag`）、`state.rs`（`spinner`/`skeleton`/`alert`）、`titlebar.rs`（`title_bar::TitleBar`）。
  - **应用级布局原语，原生无对应，应保留**：`dialog.rs`（无边框辅助窗口的客户区骨架：标题栏 36 + body + footer 48；原生 `dialog` 是**窗口内模态**，不是一回事）、`settings.rs::settings_split_surface`、`titlebar.rs::ModuleHeader`。
  - **领域 UI，保留为薄组合**：`custom_command_editor`、`quick_launch_diagnostics`、`path_list`、`volume_scope`、`hotkey`。
- 替换顺序建议：从"原生等价物最明确、爆炸半径最小"的开始（`popover` → `state` → `list` → `titlebar` → `settings`），`settings` 收益最大但触及设置窗口与大量测试，放后面。
- 另有 `ui/metrics.rs` + `ui/typography.rs` 与 gpui-component 的 semantic spacing/typography tokens 并行；彻底迁移后应逐步退役。
- 用户已定：**5 套主题的切换入口放在全局设置窗口**，并持久化到 AppConfig（需同步 C#/IPC）。
- 替换自研组件会与 366 个已固化测试（像素契约 + 交互生命周期）冲突，必须**逐组件替换 + 逐个重校测试**，不能大爆炸式替换。

## 迁移任务文档（权威）
- **`docs/gpui-kit-refactor-plan.md`** 是当前迁移的任务文档：现状矩阵、遗留项 R-01..R-09（含文件:行证据）、任务 M-01..M-14、批次 P0..P4、验证矩阵、未决决策。**做下一步前先读它。**
- 执行结果与验证记录在 `docs/gpui-kit-migration.md`。
- 关键遗留：**浏览器扩展令牌 `popup-tokens.css` 已与桌面调色板脱钩**（扩展测试只校验自洽，跨边界无门禁，建议改为从共享调色板生成）；取色仍以应用自持的 `crate::theme::Theme` 为主（35 文件 vs `gpui_component::Theme` 2 文件）。
- `vendor/`（209 文件/2.6MB）与根 `Cargo.toml` 空 `[patch.crates-io]` 段已于 2026-09-12 清除（M-01 / M-02 完成）。

## 门禁现状（2026-09-12 收官：**源码门禁 15/15 全绿**）
- **已绿**：`quick-launch-lifecycle`（M-15）、`browser-native-messaging`、`download-ui-lifecycle`、`file-dialog-window-follow`、`clipboard-lifecycle`、`clipboard-ui-lifecycle`、`taskbar-overlay-lifecycle`、`desktop-shell-lifecycle`、`file-dialog-search-*`、`indexer-lifecycle`、`package-memory-policy`、`ui-*`。
- `desktop-product-icon` / `desktop-release-subsystem` 需 `-DesktopExecutable`（无构建产物时无法运行），是产物门禁不是源码门禁。
- 本轮清掉的三个红门禁**都红在同一模式**：门禁按"单文件 / 固定文件列表"读代码，而代码后来被拆分或搬迁。**遇到红门禁先怀疑读取面过期，别急着判代码回归。**

## 写/修架构门禁的坑（复用价值高）
- **C# 适配器是 partial 类**：`WindowsPlatformAdapter` 拆成 `.cs` / `.NativeMethods.cs` / `.EventWindow.cs`。
  门禁只 `Get-Content` 主文件会大量误报——QL 门禁 19 条必需模式里 13 条只在 partial 里。
  改拼接前先确认**反向断言**（`-match` / `Contains` 即失败）的模式不在被追加的文件里，否则引入假红。
- **扩文件列表不能按字母序 glob**：多处断言用"起点标记→终点标记"做区间扫描，字母序会打乱既有文件相对顺序，
  让终点标记跑到起点之前 → 区间失效或 IndexOf 返回 -1。要**追加在末尾**以保序。
  （实例：shutdown_window → settings_view 的区间。）
- **`IndexOf(x, $start)` 在 `$start = -1` 时直接抛异常**，会绕过后面 `-lt 0` 的正常报错。
  让第二次扫描在起点为负时直接返回 -1，让守卫去报错。
- **区间结束标记可能整个消失**（如 `#[allow(` 在搬迁后的新文件里一处都没有），需要换成真实存在的边界。
- **隔离标记不要依赖缩进**：`IndexOf('    fn xxx(')` 在模块拆到 `tests.rs` 后失配（函数落到 column 0）。
- **注入式红能力测试：替换名必须完全不含原串**。门禁是子串匹配，`WsExNoActivate` → `WsExNoActivate_TEMP` 门禁照样绿。
- 跑门禁前先确认它是否接受 `-RepositoryRoot`，部分门禁不接受，会被误读成失败。
- 各门禁文件行尾**不一致**（`download-ui-lifecycle` 是 CRLF，其它多是 LF），编辑后要逐文件核对没被改坏。

## 设计规范（2026-09-14，来自 D:\Tencent\nxtools-gpui-kit-ui_ux-优化设计方案.zip）
- 该 zip 是 **React/TypeScript 交互原型**（Vite + React），非 Rust 代码。规范里的
  `src/views/`、`src/components/` 是**原型内的路径**，Rust 侧实际是
  `ui/`、`downloads/`、`main_window/`、`quick_launch/`、`tools/`，需做路径映射。
- 原型 `src/components/CodeView.tsx` 给出了**目标令牌的权威 Rust 定义**（`NxTheme`，8 个槽）：
  `bg_base #0f1115` / `bg_surface #15181e` / `bg_overlay #1b1f27` / `bg_element #242a35` /
  `border_subtle rgba(255,255,255,0.07)` / `accent_primary #3b82f6` / `text_primary #f1f5f9` /
  `text_muted #64748b`；用 `cx.global::<NxTheme>()` 注入。
- **与项目现状的冲突（必须决策）**：设计是 **8 令牌 + 单一深色**；项目是
  **`crate::theme::Theme` 27 字段 + 5 套主题**（One Dark 默认 / Catppuccin Mocha / Nord / Zed Dark / GitHub Light，
  含浅色），取色靠 `theme: Theme` 参数传递（而非 global）。直接照搬设计会**丢掉多主题能力**。
- 建议折中：**保留 5 套主题，把令牌名统一到 NxTheme 的语义槽**（每套主题供这 8+ 个值），
  语义色（success/danger/warning）作附加槽。既满足"统一令牌、无硬编码"，又不丢功能。
- 规范明令禁止硬编码颜色；已查实并修掉唯一违规：`main_window/tools_page.rs` 曾直接
  `rgb(0xa1a1aa)` 等 4 个字面量（现改为读 `theme.*` 令牌）。`theme.rs` / `theme_palette.rs` 里的
  `rgb(0x...)` 是调色板定义，属合法。
- 原型自身渲染用的色值并不统一（#161821/#171a23/#151720 等各出现十余次），**以 CodeView.tsx 的
  8 令牌为准**，不要抄原型里的 Tailwind 近似色。

## NxTheme 已落地（2026-09-14）
- 位置：`rust/apps/nxtools-desktop/src/theme.rs`（**新增，与 legacy `Theme` 并存，未覆盖**）。
  15 字段 = 5 级景深 + 2 边框 + 5 语义状态 + 3 文本阶，全部 `Hsla`，`impl Global` + `Default`。
- 注册：`main.rs` 的 `gpui_kit::application().run(|cx: &mut App| { ... })` 里，
  `gpui_kit::init(cx);` 之后 `cx.set_global(NxTheme::default());`。
- 取用：`let theme = cx.global::<NxTheme>();`（返回 `&NxTheme`，字段 `Hsla` 是 Copy）。
- **gpui-pre 0.3.4 的 API 差异**：`Global` / `App::set_global` / `App::global::<G>()` 都在；
  `App` 用 `app.rs:683`；`AppContext` 在 main.rs 里可见（是 trait 不是 struct）。
  `Global` trait 定义在 `global.rs:22`。
- 首个消费者：`main_window/tools_page.rs::ToolSettingsWindowBody`，
  原来写死 `Theme::zed_dark()` + 4 个硬编码 `rgb()`，现改为 `cx.global::<NxTheme>()`。
  字段映射（legacy → NxTheme）：`canvas→bg_base`、`surface→bg_surface`、
  `surface_hover→bg_element_hover`、`accent→accent_primary`、`success→status_success`、
  `text→text_primary`（`text_secondary`/`text_muted`/`border_subtle` 同名不变）。
- 注意 tools_page.rs 里**有两个 theme 来源**：`window.theme`（legacy，另一个函数，别动）和
  `cx.global::<NxTheme>()`（新）。做字段映射时要限定行范围，否则会误改 legacy 那一处。
- 未接入字段用 `#[allow(dead_code)]` 标注（同 `Theme` 的既有风格），不要掩盖成别的东西。

## 下载行 Hover 操作条（2026-09-14）
- 真实位置：动作在 `main_window/download_page.rs` 的 `download_task_row`（不是 `src/views/download_row.rs`——
  那是原型里的路径）。行高 `DOWNLOAD_TASK_ROW_HEIGHT = 46.0` 被测试断言，行内 12 个
  `download-task-*-selected` selector 也必须保住。
- **⚠️ 关键坑：不能在行渲染时读 `MainWindow` 实体**。行是在 `MainWindow::render_downloads`
  里渲染的，此时实体正在被 update，`main_window.read(cx)` 会 panic：
  `cannot read MainWindow while it is already being updated`。
  → **指针状态必须放 `Global`（非实体，不受该借用规则约束）**，已加
  `DownloadRowHoverState { hovered, expanded }` + `impl Global`。
- 读用 `cx.try_global::<T>()`（不 panic，测试环境没跑 main.rs 时读到 None）；
  写用 `cx.default_global::<T>()`（首次访问自动插入默认值）。**不要用 `global()`/`global_mut()`**（会 panic）。
- **⚠️ 第二个坑：`cx: &mut Context<Self>` 不能 move 进虚拟列表 processor 闭包**（要求 'static，报 E0521）。
  但闭包自带第 4 个参数（`_window`/`_cx`），**用闭包自己的**那个即可。
- 动作 API（都在 `DownloadApplication` 上）：
  `open_task_artifact(task_id, ShellOpenAction::{Open,SelectInExplorer}, cx)`、
  `batch_action(vec![id], DownloadBatchAction::{Pause,Resume,Cancel,Remove}, cx)`、
  `request_delete_confirmation(id, cx)`。
  不要试图复用上下文菜单里的 `artifact_item`/`batch_item`——它们是函数内的局部闭包。
- **架构门禁冲突**：`tests.rs::download_workspace_is_one_filter_strip_...` 里有
  `assert!(!row.contains("Button::new"))`（行内禁按钮，操作走菜单/批量栏）。Hover 操作条与之冲突。
  已按新设计更新门禁：**把 `row` 的提取范围收窄到 `download_task_row`→`download_row_action_bar`**，
  行主体仍禁按钮；新增独立的 `action_bar` 片段，断言它**确实**有 `Button::new` / `open_task_artifact` /
  `request_delete_confirmation`，并断言行有 `on_hover` + `try_global::<DownloadRowHoverState>()`。
## 行内 Chunk Map（2026-09-14）
- **数据源就在行里**：`DownloadProgress` 自带 `segments: Vec<DownloadSegmentProgress>`
  （`nxtools-download/src/message.rs:397`），所以行内画分片**不需要额外加载** `DownloadTaskDetail`
  （详情视图用的是 `DownloadTaskDetail.segments`，两者是不同类型：`DownloadSegmentProgress` vs `DownloadSegment`）。
- `downloads::task_window::segment_dot_ratio` **不可跨模块访问**（模块私有，且 `SEGMENT_DOT_LIMIT` 无 pub）。
  已在 `download_page.rs` 本地定义等价的 `chunk_ratio()`（downloaded/total clamp 0..1）。
  类型需 `use nxtools_download::DownloadSegmentProgress;`
- **行高几何（已踩坑，别再试错）**：
  - `border_b_1` 占 1px → 列条高度必须是 `DOWNLOAD_TASK_ROW_HEIGHT - 1.0`（常量 `ROW_STRIP_HEIGHT`），
    否则行变成 47 而契约要 46。
  - **行容器必须保持 block**（不要加 `.flex().flex_col()`）——加了之后列宽会被拉伸（实测 created 列 984 vs 契约 697）。
    block 布局下子项自然垂直堆叠，列宽不受影响。
  - 展开高度用固定 `ROW_CHUNK_HEIGHT = 14.0`，且分片条**单行 + `overflow_hidden`**（不要 `flex_wrap`，
    否则换行导致高度不可控、行高断言失效）。
## 设置窗口与任务栏仿真（2026-09-14）
- **设置窗口已有一级导航**：`DownloadSettingsPage { General, Request, Transfer, Window, Diagnostics }`
  （`settings_view.rs:295`），共 5 页——与"扁平化单层导航"的 5 项需求**几乎一一对应**（常规/请求/传输/窗口/诊断）。
  真正还存在的二级嵌套只有 General 页里的 `selected_category` + `category_list`（分类主从列表）。
- **改设置布局的破坏面：126 个 `download-settings-*` selector 契约**（分布在 `downloads/tests.rs` 与
  `main_window/tests.rs`）。动布局前必须知道这个数字。
- 任务栏覆盖配置（`nxtools-core/src/settings.rs:261` `DownloadTaskbarProgressOverlaySettings`）：
  `enabled` / `anchor: DownloadTaskbarOverlayAnchor {Left, Right}` / `offset_x: i32` / `offset_y: i32` /
  `width: u32` / `height: u32` / `progress_mode` / `show_progress` / `show_speed` / `show_remaining_time` /
  `show_active_tasks` 等。设置 view 里这四个数值是 `Entity<InputState>`（**字符串输入**，即"裸数字"问题）。
- **真实悬浮窗已支持拖拽调节**：`DownloadTaskbarOverlayView` 有 `edit_reference: TaskbarEditGeometryReference`
  + `edit_gesture_active`（`download_taskbar_overlay.rs:313`）。缺的是设置窗口里的预览，不是拖拽能力。
- 已新建 `downloads/taskbar_preview.rs`：1:1 仿真画布 `taskbar_preview_canvas(theme, TaskbarPreviewGeometry)`，
  含居中任务栏、5 个图标、托盘时钟、按 anchor+offset 定位的指示器。已接入 Window 页顶部。
  ⚠️ 写新模块要 `use gpui::prelude::*;`，否则 `id()` / `when()` / `gap()` 等扩展方法找不到。
- **诊断中枢没有后端**：全仓无 db 体积 / 内存占用 / 日志体积 / VACUUM / 日志打包的任何 API。
  需求 3 是功能开发（要新写后端），不是 UI 重构，不能混在 UI 任务里做。
## 任务栏滑块（2026-09-14 完成）
- `SliderState`（`gpui_base::slider`，经 `gpui_component::slider` 重导出）API：
  `new().min(f32).max(f32).step(f32).default_value(impl Into<SliderValue>)`、
  `value() -> SliderValue`（`Single(f32)` / `Range(f32,f32)`）、`set_value()`。
  事件 `SliderEvent::Change(SliderValue)` / `Release(SliderValue)`，需 `cx.subscribe_in` 订阅。
- 渲染：`Slider::new(&Entity<SliderState>)`（不是值滑块，必须持有状态实体）。
- **双向同步防循环的关键**：文本字段仍是唯一提交源，滑块只"写穿"文本并走既有
  `sync_taskbar_overlay_edit` 提交路径。写入前比较文本是否已相同，相同就 return——
  否则"滑块→文本→订阅→滑块"会无限往返。
- 订阅句柄存进 view 的 `_subscriptions: vec![...]`，**不需要单独字段**。
- ⚠️ 脚本往 `impl` 块里插辅助函数会变成关联函数，调用要写 `Self::xxx(...)`。

## 设置窗口契约分布（2026-09-14 精确统计，比预估乐观）
- **唯一 selector 精确数 119**（此前口头说的 126 含 `download-settings-` 空后缀等噪声）。
- 按区域：`category` 36 / `path` 15 / `taskbar` 15 / `ffmpeg` 10 / `nav` 7 / `pick` 5，其余各 1-4。
- **关键：36 个 category selector 几乎全集中在同一个测试函数**
  `category_settings_use_a_readable_list_and_a_labeled_full_width_directo...`。
  → 改分类列表布局只会集中打挂**那一个测试**，不是散落各处。难度远低于原先估计。
- `nav` 只有 7 个 → **改一级导航布局的代价很小**。
## 需求1 进度
- **导航文案已改（零风险）**：5 项改为 `常规与分类规则` / `请求与代理凭据` / `传输与并发配置` /
  `窗口与任务栏进度` / `系统健康与诊断`。**测试不断言导航文案**（grep 无命中），只改显示串、
  selector（`download-settings-nav-*`）与页序未动 → 366 测试全过。
- **真正待做的"双层嵌套"结构**（`settings_view.rs:~1537` `category_management`，高 310px）：
  ```
  category_management
    ├── category_list  w=228 selector "download-settings-category-master"  ← 第一层主列表
    │     └── 行点击 → this.selected_category = Some(index)
    └── 右侧详情面板（name/directory/extensions/url 编辑字段）            ← 第二层
  ```
  平铺 = 删掉 228px 主列表，每个分类渲染成一张含全部编辑字段的卡片 + 行内上移/下移/删除/启用。
  `selected_category` 只保留用于滚动定位。会重写测试 `category_settings_use_a_readable_list_and_a_labeled_full_width_directo...`
  （36 个 category selector 几乎全在该函数内，`-master`/`-marker`/cell/field 都会变）。
## 需求1 分类平铺：代码已完成，测试重校未完成（2026-09-15）
- **生产代码改造已完成**，`cargo check --all-targets` **0 error / 0 warning**：
  - `settings_view.rs`：`category_editor` 从 `selected_category.and_then(单张)` 改为
    `self.categories.iter().enumerate().map(所有分类)` + `collect::<Vec<_>>()`；
    `category_management` 从「228px 主列表 + 右侧详情」改为 `flex_col` 纵向卡片流；
    删除 99 行 `category_rows`/`category_list`（含主列表滚动容器）。
  - 卡片根加 `.debug_selector(move || format!("download-settings-category-{index}").into())`
    → 每张卡可独立寻址（`category-0`/`category-1`…）。
  - 卡片标题从硬编码"分类规则"改为 `presentation.name`；副标题加解析后的
    `presentation.directory`（平铺后每卡自证去向）。`DownloadCategoryPresentation` 删了没人用的 `enabled` 字段。
  - **"+ 新增"按钮随主列表一起被删了** —— 必须重新挂到 `category_management` 末尾
    （`download-settings-category-add` / `-add-button`），否则丢功能。
- **根因已定位（不是断言过时，是真实可用性问题）**：平铺全展开后 6 张卡 × 205px ≈ 1230px，
  远超设置窗口高度 → **分类区下方的控件（重复策略单选等）被推出视口，`simulate_click` 打空**。
  表现为 `duplicate_strategy` 断言拿到 `AddNumbered` 而非期望的 `Reject`。
  - 已验证：把 add 从卡片流末尾移到**上方**后，add 的断言就过了（6→7 成功）——证明确是"视口外点不到"。
- **结论：分类卡片必须默认折叠**（只显示标题行+目录，点击展开字段），否则平铺=不可用。
  已用已有的 `selected_category` 当"当前展开项"，页面高度回到可控。
- **需求1 已收官：366/366 全绿，0 error / 0 warning。**
  最后一轮逐个清掉的重名/过时契约（都是同一个模式，一遍一遍改）：
  1. `category-heading` / `category-heading-title` 静态名 → 改 `format!("…{index}-heading[-title]")`；
     测试同步改为 `-0-heading` / `-0-heading-title`。
  2. 测试找 `category-0-title` / `category-0-detail`（原主列表行）→ 卡片副标题补上
     `debug_selector(move || format!("download-settings-category-{index}-detail"))`，
     测试的 title 指向 `-0-heading-title`。
  3. `category-1-marker`（主列表 2px 选中边）→ 平铺后"选中"=展开该卡，
     改为断言「展开的卡片比折叠的高」。
  4. tooltip 断言：路径现在直接印在卡片副标题上（比藏 hover 里好），
     而 `presentation.directory` 的解析行为本身另有断言覆盖 → 移除 hover 那段。
## 速启浮窗（需求 B）2026-09-15 进度
- **已完成并验证（366/366，0 error / 0 warning）**：
  1. 窗口 `PANEL_WIDTH` 620→760、`PANEL_HEIGHT` 410→460（`quick_launch.rs:81` 附近）。
     测试断言的是常量本身，改常量即可（`quick_launch/tests.rs::popup_dimensions_are_compact...`
     里的 620/410 要同步改）。
  2. 毛玻璃：`WindowBackgroundAppearance::Opaque` → `Blurred`（gpui-pre 另有 `MicaBackdrop` /
     `MicaAltBackdrop` 供 Win11）。窗口透明后**面板背景必须半透明才透得出来**：
     `panel.rs` 根容器改为 `.bg(Rgba { a: 0.82, ..theme.canvas })`（需 `use gpui::Rgba;`）。
  3. 圆角 + 描边：根容器加 `.rounded(px(POPUP_CORNER_RADIUS))`（16.0，已有 `overflow_hidden` 可裁切）；
     **"外框生硬暗黄"的元凶是 `.border_color(theme.window_border)`** → 改为 `theme.border_subtle`。
- **未做**：动作盘归整。现有 `QuickLaunchActionMode`（`actions.rs:394`）是
  `levels: Vec<QuickLaunchActionLevel>` 的**多级**结构，与需求的"3 个扁平正交动作"不是一回事，
  改造需独立一轮。
- 右键菜单现状（`actions.rs:173-188`）：打开 / 打开文件所在位置 / 复制路径 / 以管理员身份运行
  （文件类）、切换到此目录（目录类）、运行命令等（命令类）。**"复制路径"并不与其他项重复**，
  是否移除属于产品取舍，别为凑"3 个"而删有用功能。
- **彻底修法的两种形态（已验证）**：
  1. `debug_selector(|| "...".into())` → 直接改 `format!("...{index}")` ✅
  2. `&'static str` 参数（`settings_toggle(id,…)`、`labeled_field(…,id)`）→ 不能传动态串；
     解决法是包一层 `div().debug_selector(move || format!(…).into()).child(原控件)`。
     ⚠️ `settings_toggle` 返回 `AnyElement`，**不能再链式** `.debug_selector()`，只能外包 div。
- **剩余 2 红**：
  1. `category_settings_...` 5133：断言 `download-settings-category-master`（按设计已删）→ 需删这些断言。
  2. `settings_window_routes_picker_...` 5971：视口外点不到 → 需折叠卡片后才可能过。
- ⚠️ 环境：测试配置下 rustc 可能 `STATUS_STACK_BUFFER_OVERRUN`（栈溢出）。设 `RUST_MIN_STACK=33554432` 即可。
- **平铺踩坑**：卡片内 selector 原本全是静态名（当时只渲染选中项所以唯一），平铺后 N 张卡重名 →
  `debug_bounds` 取不到。想加 `-{index}` 后缀会撞类型错误（那些参数是 `&str` 不是 `String`），
  最终方案是**只给卡片根加 index 后缀**，内部保持静态名。
- ⚠️ 环境：测试配置下 rustc 可能 `STATUS_STACK_BUFFER_OVERRUN`（栈溢出）。设 `RUST_MIN_STACK=33554432` 即可。

## 组件替换可行性核验（2026-09-12，**重要：P1 计划前提大部分不成立**）
- R-08 把组件分成"应替换/保留"，依据是**有没有原生等价物**；但真正决定能不能换的是
  **原生等价物的几何与行为能否满足已固化契约**。逐个核验证伪了 3/5：
  - **M-04 `status_badge`：全仓引用数 0，是死代码** → 已删除（不是替换）。
  - **M-05 `settings_toggle` → Switch：不可行**。原生 `Switch` 只有 28×16 / 36×20 两档，
    尺寸由 `Size` 枚举**固定、无法覆盖成任意 px**；应用是 40×22 且被 `network_settings.rs:1045` 断言。
  - **M-06 `state.rs`：分类本身就错**。里面没有 spinner/skeleton，是共享的"图标+标题+描述"工作态面板
    （empty 14 / progress 8 / error 3 处引用），`MaterialIcon::ProgressActivity` 是静态图标。
    契约：最小高 72、图标 18×18、三态左对齐（`state.rs:210-221`）。
  - **M-07 `settings` 外壳：不可行**。原生 `setting::*` 用弹性 gap，无 42 行高/48 footer；
    而 42/48 在 `downloads/tests.rs`、`main_window/tests.rs` 被大量断言。
  - **M-08 `client_titlebar`：经查不应替换（有源码证据）**。原生 `TitleBar` 渲染时**无条件**拼
    `WindowControls`（无开关），按钮可见性来自 `window.window_controls()`，而该结构描述的是**平台能力**
    （"Whether this platform supports maximize"）；Windows 用默认实现 = 全 `true`。
    → 换过去会给 5 个固定尺寸对话框（`is_resizable:false` + `is_minimizable:false`）都加上
    最小化+最大化按钮 = 功能回归。且 `WindowControls`/`ControlIcon` 是私有类型，模块只导出 `TitleBar`。
- **教训：判定"可替换"前，先比原生组件的实际几何常量与已固化断言**，不要只看 API 名字是否对应。
  gpui-component 的尺寸常由 `Size` 枚举固定，无法用 `Styled` 覆盖。
- **`setting::*` 不能只换骨架（2026-09-13 核查）**：`SettingItem` **无 `RenderOnce`/`IntoElement`**，
  `SettingItem::render_item` / `SettingGroup::render` 是 `pub(crate)`、`SettingPage::render` 是 `pub(super)`
  → **唯一公开入口是 `Settings` 容器**（侧栏+分页+搜索+重置，且自管选中态）。用 `setting::*` 等于
  把两个设置窗口（`settings_view.rs` 2983 行 + `network_settings.rs` 1379 行）整体改成 pages 模型，
  **是重写不是替换**，须单独立项。

## 用户决策：组件替换的取舍（2026-09-12，重要）
- **用户明确要"不要旧 UI 组件"**：当原生组件几何与既有契约冲突时，**按原生组件的几何重校布局与测试**，
  而不是保留自绘。我曾把这类"是否接受视觉变化"当成需批准的设计变更去询问，被纠正——**这属于执行范围，直接做**。
- 因此 M-05 已从"不应替换"改为**已执行**（18 处开关换原生 Switch，断言 40×22 → 36×20）。
- **M-09 不要"原地改取色"，要"继续换组件"**（2026-09-13 用户指出）：原生组件自带取色，
  把自绘控件换成原生组件后，那些 `theme.xxx` 取色点**连同自绘代码一起消失**，不需要逐项做颜色映射。
  实证：`dialog_button` 换原生 `Button` = 消除 6 处取色 + 30 行自绘。
  → **M-09 的正确执行方式是继续扩大组件替换**，映射表（`docs/m09-token-mapping.md`）只用于判断
  "哪些取色点必须等组件替换才能消掉"。
- **普查自绘控件要按"手绘模式"扫，不能只按函数名**：`div()` 链 + `on_click`/`on_mouse_down`
  + (`rounded` + `border`/`bg`) 且段内不含原生组件名。全仓命中 17 处/12 文件。

## gpui-component 尺寸可调性（**修正过一次，别再记错**）
- `Size` 枚举**有**自定义变体 `Size::Size(Pixels)` 且有 `impl From<Pixels> for Size`；
  `icon.rs:182`、`button.rs:598`、`avatar`、`attachment`、`otp_input`、`number_input` 等都实现了。
- **但 `Switch` 没实现**：render 里 `match self.size { XSmall|Small => (28,16), _ => (36,20) }`，
  `Size::Size(px)` 落进 `_` 被丢弃。
- → 判断"该组件能否调尺寸"必须**读它 render 里的 match 分支**，不能只看是否 impl `Sizable`。

## 本机构建/验证经验
- 后台 `cargo check --workspace --all-targets`（`CARGO_BUILD_JOBS=1`）**约 10 分钟被回收**；日志没有 `EXITCODE` 行即是被截断，**不是**编译失败。续跑用 `Out-File -Append` 累积日志（cargo 有缓存接着跑），重跑前先 `tasklist` 确认无残留 cargo/rustc。
- 迁移目录 `D:\Code\NxTools.Next-gpui-kit` **无 git**：任何删除前先确认主仓库同名文件被 git 跟踪（`git ls-files | wc -l` 核对数量）。
