# 设计：磨砂主题重做与原生预览

## 预览形态

独立 example `rust/apps/nxtools-desktop/examples/acrylic_rework_preview.rs`，由
`scripts/preview-acrylic-rework.ps1` 构建并启动。它走工作区已有的 `[patch.crates-io]`
（与产品同一套 gpui-pre 0.3.12 / Kit 0.10.0），不改 vendor、`Cargo.toml` 依赖与 `Cargo.lock`。

预览里 DWM 调用直接在 example 内通过 `dwmapi`/`user32` 外部声明完成，便于单进程观察；
产品阶段这些调用归 C# PlatformHost 所有（Rust 管状态、C# 管 Windows API、GPUI 管渲染）。

### 窗口

| 窗口 | 种类 | 尺寸 | 材质（磨砂模式） |
|---|---|---|---|
| 工作台 | Normal，可调整大小，自绘标题栏 | 960×640 | Mica / MicaAlt / 亚克力（按当前模式） |
| 剪切板 | PopUp | 480×540 | 亚克力（38=3） |
| 搜索 | PopUp | 620×392（输入行 + 结果列表） | 亚克力（38=3） |

### 模式（F3 循环，全局生效）

| 模式 | GPUI 背景 | WCA Accent | 帧扩展（l,r,t,b） | DWM 38 | 调色板 |
|---|---|---|---|---|---|
| 实底 | Opaque | 0 | 0,0,0,0 | 1 (None) | Catppuccin 原样 |
| Mica | Transparent | 重置为 0 | 0,0,0,32767 | 2 | 中性材质 |
| MicaAlt | Transparent | 重置为 0 | 0,0,0,32767 | 4 | 中性材质 |
| 亚克力 | Transparent | 重置为 0 | 0,0,0,32767 | 3 | 中性材质 |
| Accent 回退（Win10 为「磨砂」） | Blurred | 3（F6 切 4） | 0,0,0,0 | 1 | 中性材质 |

瞬态浮层在 Mica/MicaAlt/亚克力 三个模式下都用 38=3；Accent 回退模式下用同一 Accent。
所有模式都写 20（深浅色跟随预览主题）、33=2、34=`0xFFFFFFFE`。

其他按键：F2 切换深浅色，F4 打开剪切板，F5 打开搜索，F9 GPU 帧回读，Esc 关闭当前窗口。

### 系统背景在 GPUI 窗口上生效的条件

GPUI 窗口是 `WS_EX_NOREDIRECTIONBITMAP` + DirectComposition，`WM_NCCALCSIZE` 把整个窗口
交给客户区。系统背景（38）只画在 DWM 帧下面，因此：

1. **重置 WCA Accent**：GPUI 的 `Transparent` 写的是透明渐变 Accent（state 2），它让窗口
   直接透出后面的窗口，系统背景不出现。设 38 前把 Accent 重置为 0。
2. **只向下扩展帧**：`DwmExtendFrameIntoClientArea(0,0,0,32767)` 让背景覆盖整个客户区。
   顶边保持 0：顶边一旦扩展（含 -1 全玻璃），DWM 会在右上角画自己的最小化/最大化/关闭
   按钮并给标题区着色，与 Kit TitleBar 的按钮重叠。最大化后同样没有系统按钮。
3. **非客户区渲染保持开启**：关闭 `DWMWA_NCRENDERING_POLICY` 后背景退化为纯色兜底
   （深色实测恰为 `#202020`，不带壁纸色调），窗口阴影也随之消失。
4. 写入顺序：GPUI `set_background_appearance` → Accent 重置 → 帧扩展 → 38/20/33/34。

窗口失去焦点时 Mica/亚克力按系统规则变为纯色兜底，这是 Windows 的标准行为。

### 首帧遮蔽

1. `open_window(show: false)`；
2. 构建回调里拿到 HWND：`DWMWA_CLOAK`(13)=TRUE，写入材质与 33/34；
3. `activate_window()` 应用初始位置并显示（仍被遮蔽）；
4. 首次 render 注册 `on_next_frame`，它在下一帧开始时运行，此时第一帧已提交给
   DirectComposition，回调里解除遮蔽；
5. 250ms 兜底计时器：若帧回调未到，计时器解除遮蔽。日志 `ui.acrylic_preview.uncloaked`
   记录 `path=frame|timeout` 与耗时。

### 圆角

根元素 `rounded(8)` + `border_1`，背景与描边同一元素；DWM 33=ROUND 做窗口裁剪，34 关闭系统描边。
客户端装饰窗口的官方外框保留 1 DIP 边环，根元素用 -1 DIP 绝对定位覆盖它，填充和描边一直到
窗口边缘仍是一个元素。最大化时根元素去掉圆角与描边。

实底模式 Opaque 的清屏色是白色，圆弧外的像素由 DWM 圆角裁掉。

Windows 10（build < 22000）没有属性 33/34，DWM 不裁圆角，所有窗口（实底与磨砂）都保持方角，
根元素保留 1px 描边。窗口区域（`SetWindowRgn`）只裁 GPUI 内容、不裁 WCA Accent 模糊层，
`DwmEnableBlurBehindWindow` 加圆角区域同样裁不掉，因此不使用任何 GDI 区域。

- **拖动卡顿**：Win10 1903 起，Accent4（亚克力）窗口拖动时落后于鼠标；Accent3（模糊）不受影响。
  磨砂回退用 Accent3，F6 可切回 Accent4 对比。
- **设置项**：Win10 上 Mica / Mica Alt / 亚克力 都退化为同一个 Accent 模糊，只提供「实底」「磨砂」。

`--legacy-corners` 在 Windows 11 上模拟这条路径（33=DONOTROUND，只列出实底/磨砂），用于本机对照。
诊断同时写入 `%TEMP%\nxtools-acrylic-preview\preview.log`。

## 中性材质调色板（磨砂模式）

数值取自 Fluent（WinUI `Common_themeresources_any.xaml`）与调研中的 rustyfiles 实测，
配色方案只提供 `primary`/`ring`/`selection` 等强调色。

| 角色 | 浅色 | 深色 |
|---|---|---|
| 窗口/标题栏/侧栏 | 透明 | 透明 |
| 内容层 | `#FFFFFF` × 滑块值（默认 0.50） | `#3A3A3A` × 滑块值×0.9（默认 0.45） |
| 卡片 | `#FFFFFF` 0.70 | `#FFFFFF` 0.05 |
| 卡片描边 | `#000000` 0.06 | `#000000` 0.10 |
| 正文 | `#000000` 0.89 | `#FFFFFF` 1.00 |
| 次要文字 | `#000000` 0.62 | `#FFFFFF` 0.79 |
| 三级文字 | `#000000` 0.45 | `#FFFFFF` 0.55 |
| 分隔线/描边 | `#000000` 0.08 | `#FFFFFF` 0.08 |
| 悬停 | `#000000` 0.04 | `#FFFFFF` 0.06 |
| 按下 | `#000000` 0.02 | `#FFFFFF` 0.04 |
| 选中 | 强调色 0.14 | 强调色 0.20 |
| 输入框 | `#FFFFFF` 不透明 | `#2D2D2D` 不透明 |
| 菜单/对话框 | `#F9F9F9` 不透明 | `#2C2C2C` 不透明 |
| 窗口描边 | `#000000` 0.10 | `#FFFFFF` 0.10 |

瞬态浮层没有内容层：列表直接放在系统亚克力上，只有输入框不透明。搜索浮层的输入行本身就是
浮层表面，因此不另加填充。

## 产品阶段

实现契约见 [implement.md](implement.md)：主题风格 实底 / Mica / Mica Alt / 亚克力（Win10 实底 / 磨砂），
瞬态界面固定亚克力；C# `WindowMaterialOwner` 按本文配方写入 Accent 重置、帧扩展、38 与 33/34，
Win10 写 Accent3；`SetWindowBounds` 不再有圆角字段；Rust 中性调色板、单一内容层与 Opaque 实底；
首帧遮蔽放在 GPUI Windows `events.rs`（DWM 只允许窗口所属进程设置 `DWMWA_CLOAK`）。
