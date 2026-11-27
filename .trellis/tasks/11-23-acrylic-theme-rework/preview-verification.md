# 原生预览实测（2026-11-22）

环境：Windows 11 build 22631，100% 缩放（96 DPI），系统深色、透明效果开启。
预览：`rust/apps/nxtools-desktop/examples/acrylic_rework_preview.rs`（debug 构建）。

## 原生回执

所有模式下 `DwmSetWindowAttribute` 38/20/33/34、`DwmExtendFrameIntoClientArea` 与
`DWMWA_CLOAK` 均返回 `S_OK`。运行时 F3 切换模式时每个窗口都重新写入并记录
`ui.acrylic_preview.material_applied`。

## 首帧遮蔽

| 窗口 | 解除遮蔽路径 | 打开到解除遮蔽 |
|---|---|---|
| 工作台（单独打开） | frame | 322–352 ms |
| 剪切板浮层 | frame | 356 ms |
| 搜索浮层 | frame | 367 ms |

21 次窗口打开全部由首帧回调解除遮蔽，250 ms 兜底计时器从未先到。耗时包含 debug 构建下的字体
注册与首帧布局。

## GPU 帧回读（premultiplied RGBA，F9 / `--capture-after-ms`）

| 采样点 | Mica 深色 | Mica 浅色 | 实底深色 |
|---|---|---|---|
| 标题栏 | 0,0,0,0 | 0,0,0,0 | 35,38,52,255 |
| 侧栏 | 0,0,0,0 | 0,0,0,0 | 35,38,52,255 |
| 内容层 | 26,26,26,115（`#3A3A3A`×0.45） | 127,127,127,127（白×0.50） | 35,38,52,255 |
| 左边缘中点 | 25,25,25,25（白×0.10 描边） | 0,0,0,25（黑×0.10 描边） | 62,66,85,255 |
| 像素 (0,0) | 0,0,0,0 | 0,0,0,0 | 255,255,255,255（Opaque 清屏色） |

标题栏与侧栏 alpha 为 0：官方客户端外框的阴影不进入窗口内部，系统背景原样透出。

## 屏幕合成（DWM 之后）

- 圆角：四角对角线上窗口像素 (0,0)、(1,1) 为窗口外背景，(2,2) 为描边抗锯齿，(3,3) 起为
  窗口内部。半径 8 圆弧在对角线上位于 2.34 px，DWM 裁剪与 GPUI 自绘圆弧重合；实底模式
  圆弧外的白色清屏像素被 DWM 裁掉，四角无白点。
- 实底模式正文出现 ClearType 彩色边缘（采样区 1068/11880 个彩色像素），Mica 模式同区域为 0，
  即只有 Opaque 窗口启用子像素抗锯齿。
- Mica 深色主体色随壁纸带暖色调（如 `#241F1A`、`#2C2C2B`），不是 `#202020` 纯色兜底。
- 最大化后右上角只有 Kit TitleBar 的按钮。

截图见 `preview/`：`mica-light-workbench.png`、`mica-dark-workbench.png`、
`acrylic-dark-clipboard.png`、`acrylic-light-search.png`、`solid-dark-workbench.png`。

## 未覆盖

- 125% / 150% 缩放下的圆角吻合需要切换系统缩放，留给用户验收时确认。
- 浅色壁纸 + 深色主题、深色壁纸 + 浅色主题的可读性以用户实机观感为准。
