# GPUI 生态主题与亚克力材质调研结论

调研日期 2026-11-22。我们从 GitHub 检索了 378 个相关仓库，克隆了其中 25 个做源码对照（清单见 [00-repositories.md](00-repositories.md)），按方向整理成四份子报告：

- [01 框架层](01-framework-gpui-kit-nsh-zed-ely.md)：gpui-kit（官方组件库）、nsh（长桥 Native Shell）、Zed、Ely 组件库
- [02 Windows Mica 应用](02-windows-mica-apps.md)：Argus、Arke、rustyfiles、wu、termy、tty7
- [03 模糊窗口应用](03-blurred-apps.md)：Ghostex、quiet-md、nyaterm、Lattice、con-terminal
- [04 gpui-component 应用](04-gpui-component-apps.md)：FluxDown、zedis、navop、dbflux、omarchist、RTX-FG-Manager、ElyOffice、cyberapi

此外直接精读了 oxideterm（`AnalyseDeCircuit/oxideterm`）。它与 NxTools 一样固定使用 `gpui-pre =0.3.12`，并维护了打补丁的框架副本，是最接近的参照。

## 结论一：gpui-kit 本身不提供半透明主题

- 主题 schema 里与窗口有关的只有 `window.radius` 和 `window.border`，没有材质、透明度或模糊字段。
- `Root` 永远用不透明的 `tokens.background` 画底。
- 官方示例只在 Linux 上用过 `Transparent`，从未使用 `Blurred` 或 `Mica`，也没有任何 Win32 材质调用。

因此材质完全由应用层负责，NxTools 用 Root 插件接管底色的方向本身没有错。所有用到材质的项目都做了同类的应用层处理，没有哪个项目指望 gpui-kit 主题自动完成。

## 结论二：原生材质路径与生态做法相反

| 做法 | 谁在用 |
| --- | --- |
| Win11（build ≥ 22621）用 `DwmSetWindowAttribute(38, …)`：主窗口 Mica(2) / MicaAlt(4)，临时浮层 Acrylic(3 = `DWMSBT_TRANSIENTWINDOW`)；旧系统才回退 `Blurred`（WCA Accent 4） | oxideterm、tty7、rustyfiles、termy、nsh、con-terminal、Ghostex、Argus、Arke |
| 只用 WCA Accent 4（GPUI `Blurred`）且把 DWM 38 设为 None | NxTools 当前做法；quiet-md、Zed 的 `Blurred` 分支 |

- 微软的指引是：常驻窗口用 Mica，临时、轻量消失的表面用 Acrylic。生态项目基本照此划分，例如 nsh 让窗口用 Mica、侧栏用 MicaAlt、浮层和 HUD 用 Acrylic。
- NxTools 的主窗口、设置窗口全部使用 WCA Accent 4。这是未公开的旧接口，没有失焦回退、亮度层和噪点，拖动和缩放时容易拖影。
- 早期原生预览明确选择了 WCA Accent 路径（见 `11-21-desktop-motion-acrylic-style/native-proof.md`），这是当时的设计决定，不是 bug；本结论只说明它与生态和微软指引不一致。

## 结论三：表面分层太实，所以"看不出是亚克力"

生态里几乎一致的分层如下（数值来自源码）：

| 表面 | 生态常见值 | NxTools 当前（默认） |
| --- | --- | --- |
| 窗口底色 | 0.20–0.40 中性洗色（rustyfiles 浅 `#F3F3F3`@0.30、深 `#202020`@0.40；nyaterm 0.20），或直接 0 | 调色板色 @0.72 |
| 标题栏、侧栏、工具栏、标签栏 | **0**，直接露出材质（rustyfiles、Argus、wu、termy）；或 0.15–0.5（tty7 侧栏 0.15，Arke 0.35/0.5） | 组合覆盖 0.84 |
| 内容区（列表、表格、编辑器） | **一层** 0.45–0.82（rustyfiles 0.50/0.45，Arke 0.70/0.45，tty7 0.82，wu 编辑器 0.45） | 0.84 |
| 卡片、瓦片 | 墨色低透明度（白 0.05–0.07 深色 / 白 0.70 浅色），或不透明 | 约 0.90 |
| 菜单、对话框、输入框、全窗覆盖层 | 不透明，或 ≥0.86–0.98 | ≥0.97 |

- 生态的共同原则是"每个区域只染一次色"：根节点清空，各区域自己染一层，严禁叠加。nyaterm 有测试断言 tint 只画一次；oxideterm 在框架层加了 `with_surface_tint_limit` 限制子树的组合不透明度；Argus 提供 `on_card()` 把叠加的半透明压平成不透明色。
- NxTools 的组合覆盖率思路一致，但取值远比生态实：底层 0.72 之上再叠 0.84 的面板，导致整窗几乎不透光。

## 结论四：配色不是"调色板色加 alpha"

做得像 Windows 11 的项目（rustyfiles、Argus、Arke）采用 Fluent 式写法：

- 表面用**中性灰**洗色，不用调色板色相。rustyfiles 的注释说明了原因：红色壁纸会让整窗发红，要用中性色压住。
- 文字、边框、悬停、选中都用**墨色加透明度**：
  - 浅色：文字黑 0.90 / 0.62 / 0.45，边框黑 0.07，悬停 0.04，选中 0.06
  - 深色：文字白 0.94 / 0.70 / 0.46，边框白 0.08，悬停 0.06，选中 0.085
- 强调色取系统强调色，选中用"中性底 + 强调色小竖条"。
- **材质开和材质关是两套调色板**（Argus、Arke、rustyfiles），不是在同一套颜色上改 alpha。

NxTools 目前把 Catppuccin 等配色的 RGB 直接作为半透明底，再把文字往前景色推。这正是 rustyfiles 注释里描述的问题：配色色相加壁纸色相，叠出浑浊的灰。

## 结论五：GPUI 对非不透明窗口关闭 ClearType

`gpui-pre 0.3.12` 的 `should_use_subpixel_rendering`（`vendor/gpui-pre-0312/src/window.rs:5149`）规定：窗口背景外观不是 `Opaque` 时，文字一律改用灰度抗锯齿。

- NxTools 为了能随时切换材质，**所有窗口创建时都设为 `Transparent`，包括现有风格（Solid）**，因此全软件都失去了 ClearType。浅色主题下的文字发虚、偏淡与此有关。
- tty7 的做法是：填充不透明时向 GPUI 报告 `Opaque`，以保住 ClearType。Ghostex 则相反，先建 `Transparent`，换取运行时切换。两者是取舍关系。

## 结论六：Windows 上的窗口圆角

- 生态里 Windows 主窗口**保留系统 DWM 圆角**，没有项目在 Windows 上用 `SetWindowRgn` 画圆角（con-terminal 的形状遮罩只用于 Linux X11）。
- 需要自绘边框的弹窗采用 Ghostex 的组合：`DWMWA_WINDOW_CORNER_PREFERENCE(33) = ROUND` 加 `DWMWA_BORDER_COLOR(34) = 0xFFFFFFFE`（不画系统边框），由应用自己画 1px 边框。这样系统负责抗锯齿裁剪（材质也一起被裁成圆角），边框由应用设计。
- nsh 对无边框、无阴影的窗口改设 `DONOTROUND`，避免 DWM 再画一层外框。
- NxTools 当前用 GDI 区域裁剪，区域本身没有抗锯齿。用户这次反馈的圆角问题，根源就在这里。

## 结论七：首帧闪烁

- nsh 与 ElyOffice 用 `DWMWA_CLOAK` 先把窗口从合成中隐藏，等首帧画完再取消隐藏。nsh 的注释直接说明了目的：避免露出未绘制的内容。
- Argus 从首个可呈现帧开始做淡入；托盘弹窗从 alpha 0 开始。
- NxTools 目前用"先实底 90ms 再过渡"的折中办法，解决不了"窗口已显示、材质未生效"的根本时序问题。

## 结论八：用户设置与回退

- **可选项**：常见为 关闭 / 系统 / Mica / MicaAlt / Acrylic（oxideterm、tty7、rustyfiles、Arke）。
- **透明度滑块都有下限**：oxideterm 窗口透明度 0.5–1.0，navop 0.5–1.0，con-terminal 内部重映射后下限 0.72 / 0.84。
- **构建号门槛**：多数项目检查 build 22621（DWM 38 的最低版本）。Arke 错检 22000，导致 22000–22620 上请求了 Mica 却没生效，半透明调色板照样启用，界面发灰。这是需要避免的坑。
- **系统透明效果开关**：Ghostex、Lattice 会尊重系统"透明效果"开关和"减少透明度"；NxTools 已有这部分事实采集。
- **深色标题栏**：四个项目自行设置 `DWMWA_USE_IMMERSIVE_DARK_MODE(20)`，让外框跟随应用主题；NxTools 已做。

## 结论九：动效

被调研的 gpui-component 应用都没有模块切换动画，动效只用于脉冲、骨架屏、消息入场和启动画面。oxideterm 把动效做成可选档位（关闭 / 减少 / 正常 / 快速）。NxTools 的 120ms 模块淡入不是生态惯例，但不冲突。

## 对 NxTools 的改造方向（待确认，未实施）

1. **材质**：常驻窗口（主窗口、设置、下载等）改用 Mica 或 MicaAlt（DWM 38）；Quick Launch、剪贴板、菜单、通知这类临时表面用 Acrylic（`DWMSBT_TRANSIENTWINDOW`）。build < 22621 时回退 WCA Accent 4，再不行用实底。这会推翻早期原生预览"只用 WCA Accent"的决定，需要你拍板。
2. **调色板**：磨砂风格使用独立的 Fluent 式中性材质调色板（中性洗色加墨色 alpha 文字、边框），配色只决定强调色和选中色；现有风格保持原配色不变。
3. **分层**：标题栏、侧栏、工具栏 alpha 0；内容区一层，约 0.5（浅）/ 0.45（深）；卡片用墨色低 alpha；菜单、对话框、输入框不透明。滑块只调内容区一层，下限 0.5。
4. **圆角**：主窗口用系统 ROUND 加 `BORDER_COLOR = none`，由 GPUI 画 1px UnderBorder 边框；去掉 GDI 区域。这样系统抗锯齿裁剪与自绘边框兼得。需要实测 DWM 圆角半径在 125%/150% 下是否与 8 DIP 一致。
5. **ClearType**：现有风格的窗口创建为 `Opaque`，切到磨砂时再改 `Transparent`；磨砂下接受灰度抗锯齿，用更深的墨色文字补偿。
6. **首帧**：用 `DWMWA_CLOAK` 隐藏窗口，直到材质回执和首帧都就绪再取消，替代 90ms 实底过渡。

## 局限

- termy、tty7、Ghostex 依赖的 GPUI 分支没有包含在克隆里，这几个项目的材质落地细节只能根据应用侧代码判断。
- Zed 的稀疏检出不含把主题 `background.appearance` 应用到窗口的代码。
- 以上全部是源码阅读结论。没有运行任何项目，也没有做视觉或性能对比。
