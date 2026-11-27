# 产品实现：磨砂主题重做

预览已验收（Win10 方角，用户 11-23 决定）。本文件是产品阶段的实现契约。

## 主题风格

全局设置「主题风格」与预览相同：

| 值（配置/协议） | 标签 | 常驻窗口（38） | 瞬态界面（38） |
|---|---|---|---|
| `solid` | 实底 | 1 | 1 |
| `mica` | Mica | 2 | 3 |
| `mica_alt` | Mica Alt | 4 | 3 |
| `acrylic` | 亚克力 | 3 | 3 |

- 系统没有 DWM 系统背景（build < 22621）时，设置只列出「实底」「磨砂」（磨砂 = `acrylic`）；
  已保存的 `mica`/`mica_alt` 在这类系统上按「磨砂」显示与生效。
- 磨砂（非实底）风格统一叫 frosted：`MaterialStyle::is_frosted()`。
- 设置只保留一个滑块「内容层不透明度」，50%–100%，默认 50%（`MaterialTuning.content_percent`）。
  深色内容层 alpha = 滑块值 × 0.9。

## 协议（`window_material_v1`）

- `WindowMaterialStyle`：`solid | mica | mica_alt | acrylic`。
- `MaterialSurface`：`backdrop`（常驻窗口）、`transient`（瞬态界面）、`transparent`（自绘浮层，不变）。
- `SystemMaterialCapabilitiesChanged` 新增：
  - `system_backdrop: bool`：build ≥ 22621；
  - `rounded_corners: bool`：build ≥ 22000（DWM 属性 33/34 可用）。
- 回执 `effective`：frosted 风格生效为 `acrylic`（含 Win10 的 Accent 模糊），回退为 `acrylic_opaque`。
- `SetWindowBoundsCommand.corner_radius` 删除；不再有任何 GDI 窗口区域。

瞬态界面：速启面板、速启预览、剪切板弹窗、剪切板悬停预览、剪切板链接浮层、文件对话框搜索面板。
其余 `backdrop` 窗口均为常驻。

## PlatformHost（C#）

`WindowMaterialOwner` 对 `backdrop`/`transient` 窗口按以下顺序写入（每步前仍校验会话/修订/代数/HWND）：

1. `DWMWA_USE_IMMERSIVE_DARK_MODE`(20)。
2. 生效为 `acrylic` 且有系统背景：
   - WCA Accent 重置为 0；
   - `DwmExtendFrameIntoClientArea(0,0,0,32767)`；
   - 38 = 上表值。
3. 生效为 `acrylic` 且无系统背景：Accent 3（模糊，`AccentFlags`=0，`GradientColor`=`0x01000000`），
   帧扩展 (0,0,0,0)。
4. `solid` / `acrylic_opaque`：38=1（有系统背景时），Accent 0，帧扩展 (0,0,0,0)。
5. `rounded_corners` 时：33=2（ROUND），34=`0xFFFFFFFE`。Win10 不写 33/34。

`transparent` 窗口写入保持不变（20、38=1、Accent 0），不写帧扩展与 33/34。
任一步失败按现有规则回退 `acrylic_opaque` 并复位（38=1、Accent 0、帧扩展 0）。
诊断 `window.material.*` 增加 `backdrop_type`、`accent_state`、`corners` 字段。

## GPUI Windows（`vendor/gpui-pre-windows-0312/src/events.rs`，已批准补丁文件）

首帧遮蔽取代 90ms 实底保持：

- 隐藏窗口收到带 `SWP_SHOWWINDOW` 的 `WM_WINDOWPOSCHANGING` 且背景不是 `Opaque`：`DWMWA_CLOAK`=TRUE，
  `SetTimer(CLOAK_REVEAL_TIMER_ID, 34ms)`；`WM_TIMER` 解除遮蔽并 `KillTimer`。`SetWindowPlacement` /
  `SW_SHOWNORMAL` 的显示不发 `WM_SHOWWINDOW`，所以挂在 `WM_WINDOWPOSCHANGING` 上，覆盖所有显示路径。
- DWM 只允许窗口所属进程设置 `DWMWA_CLOAK`（跨进程实测 `0x80070005`），因此遮蔽只能在 GPUI 进程里完成。
- `Opaque`（实底）窗口不遮蔽。

## Rust 桌面

- `theme/material.rs`：Fluent 中性调色板（`design.md` 表格），删除覆盖率求解与可读性推移；
  提供 `content(dark, tuning)`、`card_stroke(dark)`、`opaque_fallback(dark)`。
- 圆角：`rounded_corners` 时所有风格 `window_radius = 8`，否则 0。根元素一律自绘轮廓
  （`-1 DIP` 覆盖官方外框环，填充与 1px 描边同一元素）；`window_border.a = 0`。
- 根元素填充（`paint.rs`）：
  - `transient`：alpha 0（列表直接放在亚克力上）；
  - `backdrop`：内容层颜色 × 滑块 alpha；标记为分层窗口（主窗口）时 alpha 0，由内容区自己画内容层；
  - `acrylic_opaque`：不透明兜底色（浅 `#F3F3F3`，深 `#202020`）；
  - `transparent`：alpha 0，不画轮廓。
- 主窗口内容区：frosted 时内容层颜色、左上 8 DIP 圆角、上/左 1px 卡片描边。
- ClearType：根插件按「表面 + 风格」设置 GPUI 背景：实底 → `Opaque`，其余 → `Transparent`；
  切到 `Transparent` 会改写 Accent，因此随后使该窗口材质失效并重新下发。
- 删除 90ms 保持与缓入（`SETTLE_*`）。

## 验证

- 协议、C# 单测、Rust 单测（material / theme / settings / 受影响的展示所有者）。
- `python tests/architecture/gpui-release-contracts.py`、`spec-contracts.py`。
- 打包与安装不在本任务内；实机外观由用户验收。

## 用户反馈修订（2026-11-23）

本节取代上文的「主题风格」四选一、`BackdropType` 与中性调色板/透明 chrome 约定。

- 主题风格只有 `solid` / `acrylic`（Win10 标签「磨砂」）；旧配置的 `mica`/`mica_alt` 按 `acrylic` 读取。
  协议 `WindowMaterialStyle` 与 C# 契约同步收敛，Win11 所有带框磨砂表面写 38=3。
- 磨砂分层改为「配色方案着色」：`theme/material.rs` 保留方案的文字/强调/状态色，记录方案画布为 `Tint`；
  `Layers` 给出纱层（方案画布，Win11 系统亚克力上 0.50/0.55，Win10 未着色 Accent 模糊上 0.90/0.90）、
  内容层（提亮画布 × 滑块）与不透明兜底（画布）。
- Root 填充：瞬态表面＝纱层（不再透明）；常驻窗口＝内容层叠纱层；主窗口＝纱层、内容区画内容层；
  `acrylic_opaque`＝方案画布。
- 新增 NxTools 配色 `assets/themes/nxtools.json`：默认浅色/深色（石墨）、雾蓝、松石、暖砂各一对；
  默认浅/深色与「跟随系统」改用石墨，Catppuccin Latte/Frappe 作为独立选项保留。
- Win10 速启「完全透明」：本机（Win11 22631）无法复现 Win10 DWM 行为。纱层保证即使 Accent 模糊未生效，
  面板也有 0.88/0.84 方案底色；根因需 Win10 实机 `platform-host` 日志（`window.material.terminal` 的
  `accent_state`/`effective`）确认。

### 第二轮反馈（2026-11-23）

- 可读性：纱层加厚（见上），磨砂下次要/弱化/表头文字向正文色混合 35%；测试覆盖 Win10 模糊在黑、白、中灰与饱和壁纸色上正文 ≥7:1、次要与弱化 ≥4.5:1。
- Win10 速启日志（14:44–15:18）：全部 `effective=acrylic`、`accent_state=3`、`fallback_reason=none`、无失败，DWM 写入均成功；“完全透明”是旧版透明 Root 叠在未着色模糊上的观感，纱层后不再出现。
- Win10 二级窗口拖动卡顿：GPUI 每个 WM_MOVE 都 `bounds_changed` → 全窗刷新并重新呈现，Win10 DWM 每步重算 Accent 模糊。`gpui-pre-windows-0312` `events.rs`/`window.rs`（已批准补丁文件）在系统移动/缩放循环内只更新原点与显示器，退出循环时报告一次移动。
- Markdown 阅读器：磨砂下 `colors.table` 被清空，原 0.9 阅读底板把它补成近黑色；磨砂时页面透明、表头用 `muted`，由窗口 Root 的内容层与纱层显示材质。

### 第三轮反馈（2026-11-23）

- Win10 0.90 纱层把模糊盖没了。没有亮度层的 Accent 模糊只能线性叠色，不可能整窗同时又透又满足 AA，所以按区域分：标题栏/侧栏纱层降到 0.70/0.77（普通桌面 AA、纯黑纯白 ≥3:1），内容区与瞬态面板用「内容层叠纱层」（任何桌面正文 ≥7:1、次要 ≥4.5:1）；Win10 次要文字向正文混合 50%，Win11 保持 0.35/0.50/0.55 不变。
- Win10 圆角：DWM 圆角属性 33 只在 build 22000+ 存在，与 Rust/GPUI 无关。自绘圆角的阻碍是 Accent 模糊层按窗口矩形绘制、DWM 阴影也是矩形；此前「窗口区域裁不掉 Accent」的结论来自 Win11 `--legacy-corners` 模拟，需在真实 Win10 上用预览重测后再定。

### Win10 圆角与拖动实验（2026-11-23）

- 用户反馈：二级窗口实底拖动不卡、磨砂卡；主窗口磨砂不卡；实体机。怀疑点：磨砂二级窗口拖过磨砂主窗口时 DWM 要叠两层模糊。
- 实验程序 `examples/win10_corner_lab.rs`（产物 `artifacts/win10-corner-lab/NxTools-Win10CornerLab.exe`，静态 CRT）：主窗口 + 二级窗口；F3 实底/磨砂，F7 圆角 ①方角 ②`SetWindowRgn` ③自绘圆角+自绘阴影（关 DWM 阴影、透明外圈作缩放边），F8 拖动时把 Accent 模糊临时换成不透明 Accent1 底色；日志 `%TEMP%\nxtools-win10-corner-lab\lab.log`（含每次拖动时长与 WM_MOVING 次数）。
- Win11 22631 上的 Accent3 初测：② 区域能裁掉模糊、四角干净但无阴影；③ 模糊铺满透明外圈，出现方形模糊光晕。Win10 结论待用户实测。
- 用户反馈：③ 自绘圆角可以，但磨砂时阴影区是一圈方形模糊（Accent 模糊铺满窗口矩形含透明外圈）。新增 ④：窗口仍用 `SetWindowRgn` 裁成圆角（模糊一起被裁），阴影由独立的点击穿透分层窗口（`WS_EX_LAYERED|TRANSPARENT|TOOLWINDOW|NOACTIVATE`，被主体窗口拥有，`UpdateLayeredWindow` 预乘位图）按圆角矩形距离场绘制，中间挖空，边缘加一圈抗锯齿描边盖住区域裁剪锯齿；跟随 `WM_WINDOWPOSCHANGED`，最大化/最小化/隐藏时隐藏。实底时主体保持 Opaque，ClearType 保留。
- 用户确认 ④ 外观可用，但磨砂拖动仍卡。实验程序加 F9「拖动时隐藏阴影窗口」（松手后在最终位置重新显示），与 F8「拖动时临时切实色」组合对比；拖动日志增加 `max_gap_ms`/`slow_gaps`（两次 WM_MOVING 间隔 >33ms 的次数），只反映窗口线程是否卡住，不反映 DWM 合成延迟。
- 用户反馈：开不开阴影、F8 强调色实色都卡，现象是窗口落后鼠标的「慢动作」。实验程序拖动方式扩为三档（F8：保持模糊 / 强调色实色 / 整窗实底——进入移动循环关 Accent，经 `async-channel` 通知 GPUI 在移动循环计时器内改 `Opaque` 并按实底重绘，退出时恢复），加「主窗口方式」SC_MOVE 拖动手柄（与产品 `BeginWindowMove` 同路径，日志 `via=sc_move`），以及 F6 移动节流（`WM_MOVING` 按显示器刷新周期合并，目标按光标位移计算）。用户实测：均仍不跟手。
- 结论：WCA Accent 模糊在 Win10 1903+ 上的拖动滞后是已知系统问题，切 Accent 或节流都绕不开。Windows Terminal 走 XAML `HostBackdrop`：窗口声明宿主背景（Win10 Accent 5 `ACCENT_ENABLE_HOSTBACKDROP`，Win11 `DWMWA_USE_HOSTBACKDROPBRUSH`=17），由 Windows.UI.Composition 的 `CreateHostBackdropBrush` 视觉在窗口底层绘制已模糊的背景。
- 实验程序新增 F5 磨砂来源「宿主背景（Terminal 方式）/ 强调色模糊（现状）」，默认宿主背景（`examples/win10_corner_lab/host.rs`：线程 DispatcherQueue + Compositor，`CreateDesktopWindowTarget(hwnd, false)`，单个 SpriteVisual 用 HostBackdropBrush，`RelativeSizeAdjustment`=1 随窗口尺寸）。为让底层目标可用，`gpui-pre-windows-0312/src/directx_renderer.rs`（已批准补丁文件）把 GPUI 的 DirectComposition 目标改为 topmost（GPUI 窗口不含子 HWND）。示例的 `windows`/`windows-numerics` 仅为 nxtools-desktop dev-dependency。
- Win11 22631 初测：宿主背景显示正常、GPUI 内容在其上，`host=shown`。Win10 是否跟手、失焦后是否保持磨砂待用户实测；通过后在 GPUI Windows 渲染层移植到产品（仅 Win10），圆角 ④ 与阴影窗口不变。

### Win10 速启搜索框与磨砂质感（2026-11-25）

- Win10 磨砂（Accent3）只有模糊，没有系统亚克力自带的噪点；纯色纱层叠在平滑模糊上像着色塑料。确认生效为 `acrylic` 的带框窗口由 `MaterialRoot` 插件覆盖层按设备像素 1:1 平铺固定种子的 256px 颗粒（`BLUR_GRAIN` 浅 0.025 / 深 0.03，黑白对称，不改变整体明暗，无命中区域）；Win11、`acrylic_opaque`、实底与透明浮层不画。
- Win10 速启是方角磨砂面板，内部再画一个圆角输入框边框会形成「框中框」：Win10 磨砂下官方 Large `Input` 用 `appearance(false)`，面板本身就是搜索框，内边距、字号与焦点不变；Win11 磨砂与实底保留原输入框边框。
- 验证：GPU 回读真实速启面板（Win10/Win11 × 浅/深，叠在模糊壁纸上）确认外观；`material_paint_grains_only_the_live_windows_10_blur`、`material_paint_grain_tile_is_fine_balanced_and_bounded`、`corner_surface_quick_search_is_the_panel_itself_on_the_windows_10_blur`。Win10 实机观感待用户验收。

### Win10 宿主背景与圆角 ④ 进产品（2026-11-26）

- 用户在 Win10 实体机验收实验程序：宿主背景（Terminal 方式）+ 圆角 ④ + 拖动保持磨砂 + 阴影跟随 + 不节流，拖动不再卡顿。本节取代上文 Accent3 与 Win10 方角的约定。
- PlatformHost：无系统亚克力时磨砂写 Accent 5（`ACCENT_ENABLE_HOSTBACKDROP`）、帧扩展 0；build 22000–22620 另写 DWM17=1（复位写 0）。Accent 3 不再可达。
- GPUI 进程 `ui/material/win10.rs`（Desktop 唯一允许 `unsafe_code` 的模块，AGENTS.md 已登记）：Root 插件在 render 里按窗口发布 `Wanted { round, host, dark }`，经注册消息在子类化窗口过程里执行（`SetWindowRgn` 会同步发位置消息，不能在 GPUI 更新中调用）。区域圆角 8 DIP，最大化移除；阴影窗口照搬实验程序 ④，`WM_WINDOWPOSCHANGED` 同步跟随，最大化/最小化/隐藏/遮蔽时隐藏；宿主背景视觉在底层 `DesktopWindowTarget`，GPUI 内容 topmost（`directx_renderer.rs`）；`WM_NCDESTROY` 销毁阴影、释放合成目标。实底窗口同样圆角。
- `rounded_corners` 改为「事实已知即圆角」，Win10 `window_radius` 也是 8 DIP；`region_corners` 表示走窗口区域。`windows`/`windows-numerics` 由 dev-dependency 改为正式依赖，Cargo.lock 不变。
- 验证：C# window material 13 项通过；Desktop clippy（tests 与 bins）通过、相关 204 项测试通过，新增 `material_paint_windows_10_rounds_framed_windows_and_hosts_only_live_frosting`；本机隐藏窗口原生冒烟确认区域生效、阴影显示、宿主背景创建成功、关闭后区域移除阴影隐藏、窗口销毁后阴影窗口与记录释放（临时代码已删除）。Win10 产品实机观感与拖动待用户打包验收。

### Win10 产品拖动与透明度滑块（2026-11-27）

- 用户反馈：实验程序拖动跟手，产品仍略不跟手；Win10 上调透明度几乎没有效果。
- 滑块：Win10 内容层叠在 0.70/0.77 纱层上，沿用 Win11 的 alpha 时整体只在约 0.85–1.0 之间变化。改为 `host_content_alpha`：50–100 映射为 0–1，文字表面从与标题栏/侧栏同样通透到完全不透明。最透明一档文字对比度按纱层标准（常见桌面 AA，纯黑/白 ≥3:1），最不透明一档按原 7:1/4.5:1。默认值 50 因此比之前更通透。
- 拖动：产品与实验程序的差异之一是阴影跟随每一步都查询 DWM 遮蔽状态（跨进程），GPUI 移动循环 10ms 计时器每次也会触发一次。改为只在阴影尚未放置时查询；已放置的阴影只用本窗口调用跟随。
- 新增每次拖动一条诊断 `product.material.win10_shape.drag`（位置步数、最大间隔、超过 33ms 的间隔次数，以及 GPUI 移动循环计时器的次数、总耗时与最长一次）。若仍不跟手，用它区分「窗口线程被应用工作卡住」与「合成延迟」。

### Win10 宿主背景加 Fluent 亮度层（2026-11-27）

- 用户：拖动已不卡；Win10 透明效果仍只有一点点；设置里滑块数值「100%」折行。
- 原因：宿主背景只有模糊，没有系统亚克力的亮度层；半透明底色与壁纸是线性混合，要让任意壁纸上文字可读，底色必须 ≥70%，透明感自然很弱。
- 做法（用户选 A）：宿主背景视觉的画刷改为 XAML `AcrylicBrush` 同款效果图（`IGraphicsEffect`/`IGraphicsEffectD2D1Interop`，`#[implement]`）：D2D Blend COLOR(22)（背景=宿主背景，前景=方案底色 × 亮度不透明度）→ Blend LUMINOSITY(23)（前景=方案底色 × 色调不透明度）。本机 DWM 实测：第一步把背景亮度拉向底色、保留色相（深色 L85：白→#414141、黑→#1B1B1B、红仍是红），第二步在亮度不变下着色。取值 `HOST_ACRYLIC` 浅 60/0、深 70/15（XAML 为 85/96，几乎只剩底色）。
- 因为有了亮度层，Win10 改用与 Win11 相同的纱层 0.50/0.55、内容层与滑块映射、次要文字混合 35%，删除 0.70/0.77 纱层与 Win10 专用滑块映射。测试改为：黑到白任意灰度桌面经亮度层后、只有纱层时正文 ≥7:1、次要/弱化 ≥4.5:1。
- 滑块数值列固定宽 44 改为最小 56 且不换行。
- `#[implement]` 需要 `windows-core` 直接依赖（0.62.2，已在依赖树中）：Cargo.lock 只在 nxtools-desktop 的依赖列表里多一行，没有新包。
- 用户要求 Win11 速启搜索框与 Win10 一致：所有磨砂面板（Win11、Win10）搜索框都用 `appearance(false)`，面板本身即搜索框；实底保留原输入框边框。
