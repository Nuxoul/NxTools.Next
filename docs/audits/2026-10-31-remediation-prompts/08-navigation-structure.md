# 第八批：导航结构（U6 侧栏分组、U7 打开面板入口、U8 标题栏）

## 发送前需要确认的决策

下面是推荐的默认选择，已写进提示词。如需修改，直接改提示词里“已批准的决策”一节。

| 决策 | 默认选择 | 备选 |
| --- | --- | --- |
| D1 侧栏分组 | 两组：“工作区”（下载、压缩）和“快捷工具”（速启、剪贴板、扩展、其他工具） | 三组：扩展单独成组；或保持一组不变 |
| D2 模块顺序 | 下载、压缩、速启、剪贴板、扩展、其他工具（扩展仍在剪贴板之后、其他工具之前，符合 2026-10-15 的要求） | 保持现有顺序，只加分组标题（压缩会落在“快捷工具”中间，不推荐） |
| D3 标题栏 | 去掉“· 模块名”，只保留产品名 | 保留 |
| D4 打开面板按钮 | 速启和剪贴板设置页头部各加一个 outline 按钮，文案带当前快捷键 | 不加 |

```text
你在 Windows 工作区 D:\Code\NxTools.Next-main 中工作（Rust + GPUI 桌面端，PowerShell）。任务：实现 U6（侧栏分组）、U7（弹出工具设置页的“打开面板”入口）、U8（去掉标题栏重复的模块名）。

## 已批准的决策
- D1：侧栏分两组，“工作区”（下载、压缩）和“快捷工具”（速启、剪贴板、扩展、其他工具）。
- D2：模块顺序为 下载、压缩、速启、剪贴板、扩展、其他工具。
- D3：标题栏去掉“· 模块名”，只保留产品名。
- D4：速启和剪贴板设置页头部各加一个“打开面板”按钮，文案包含当前实际绑定的快捷键。

## 通用规则
- 工作区里有他人未提交的改动，必须保留；不要回退、格式化或提交与本任务无关的文件。不要 commit/push，不要打包、安装或启动产品。
- 开始前阅读 AGENTS.md、.trellis/workflow.md、.trellis/spec/shared/ui-component-selection.md（必读：组件优先；页头 outline 按钮必须用 Toolbar.content 保留边框，Toolbar.child 会把 Button 变成 ghost().compact()）、docs/reference-code/gpui-kit-080.md 与 gpui-kit-component-index.md、.trellis/spec/shared/ui-design.md（尤其 “Extensions workspace correction (2026-10-15)”：扩展必须位于剪贴板之后、其他工具之前，176 rail 与主窗口 1060×650、最小 920×480 不变）、.trellis/spec/nxtools-desktop/backend/ui-experience-contracts.md。方案依据：docs/ui/2026-10-31-ux-optimization-proposal.md 的 U6–U8。在 .trellis/spec/shared/ui-design.md 中记录本次已批准的修订（注明批准日期），然后再实现。按 workflow 建立任务记录（建议 11-0x-navigation-structure/），并按组件选择规则记录选用的组件、精确 API、变体、尺寸和父级适配。
- 先写能在旧代码上失败的生产渲染回归测试，再实现，并给出先红后绿的证据。
- Rust 命令在同一进程内设置 $env:CARGO_BUILD_JOBS='1'; $env:CARGO_INCREMENTAL='0'; $env:RUSTFLAGS='-C codegen-units=256'；桌面测试使用 --config 'profile.test.package.nxtools-desktop.debug=0' 并单线程；给足超时；超时后先检查残留进程，不要重复构建。
- 已知的无关失败（日期相关的日历测试、nxtools-archive 格式差异）不要修、不要压制，在报告中列出。行号以 2026-11-01 的工作区为准。
- 不得声称做过用户视觉验收；截图必须真正查看后再描述。

## 1. U6：侧栏分组与顺序
现状：
- rust/apps/nxtools-desktop/src/shell.rs:13 `MainModule::ALL` 的顺序为 Downloads、QuickFind、Clipboard、Extensions、Archive、Tools。
- rust/apps/nxtools-desktop/src/main_window/shell.rs:246 `render_sidebar` 用一个 `SidebarGroup::new("工作区")` 包住自定义的 `FocusedModuleMenu`（:301、:311-338），每个模块是 `FocusedModuleButton`。焦点句柄 `module_focus[index]` 按 ALL 的下标索引。Kit 的 SidebarMenuItem 没有焦点句柄和 Tab 绑定，这是现有自定义焦点按钮的原因（:307-309 注释），必须保留这个方案。

要求：
1. 按 D1、D2 拆成两个官方 SidebarGroup，组标题使用 Kit 的组标签样式。顺序变化要同步到 MainModule::ALL（或单独的展示顺序，二选一并说明理由）、module_focus 的索引，以及第七批的 Ctrl+1…6 映射（如已实现）和对应的 Kbd 提示。
2. 保留现有的 Tab/Shift+Tab 焦点顺序、选中态、切换模块时的清理（退出扩展页时 retire_import_preview、离开下载页时 retire_download_detail_scope）和 blur/focus 逻辑。
3. 检查所有依赖模块顺序的地方：测试（例如 main_window/tests.rs 约 :599 的模块循环）、上次打开模块的持久化（如有）、visual_preview 画廊、托盘或命令入口。

## 2. U7：速启和剪贴板设置页的“打开面板”入口
要求：
1. 在速启（main_window/quick_launch_page.rs）和剪贴板（main_window/clipboard_page.rs）的模块头部右侧，各加一个官方 Button，使用 outline、Medium 尺寸，通过 Toolbar.content 放置以保留边框。文案为“打开剪贴板面板（<当前快捷键>）”/“打开速启（<当前触发方式>）”，从当前配置读取，热键修改后文案同步；按钮宽度不足时，文案只保留“打开面板”，完整说明放入 Tooltip。
2. 点击后调用与热键唤起相同的入口（剪贴板面板和速启弹窗现有的打开函数），以主窗口作为前台来源；不得新建第二个窗口或控制器；跨实体工作用 cx.defer。
3. 热键不可用或面板正在打开时，按钮的禁用或 loading 状态要有原因说明。

## 3. U8：去掉标题栏重复的模块名
- main_window/shell.rs 约 :149-157 的 `main-active-module` 元素显示“· 模块名”，删除它；拖动区域、品牌图标和标题的几何位置保持现有测试的约束（最小 920 宽下，工具按钮不与拖动区重叠）。

## 4. 验收与验证
- 生产渲染回归：两个组标题与模块顺序；点击和 Tab 切换后焦点与选中态正确；切换模块时的清理逻辑仍然执行；“打开面板”按钮在 Light/Dark 下有可识别的边框（按规则检查实际绘制的边框，而不只是布局矩形），点击只调用一次唤起入口，热键变更后文案更新；标题栏不再出现模块名；默认 1060×650 与最小 920×480 下没有溢出。
- cargo clippy -p nxtools-desktop --tests -- -D warnings；对改动文件做 rustfmt 检查；git diff --check
- python tests/architecture/spec-contracts.py、python tests/architecture/gpui-component-catalog.py、python tests/architecture/gpui-release-contracts.py
- 可以生成截图的话，提供 Light/Dark 主窗口截图并真正查看；最终视觉以用户确认为准。

## 5. 范围外
各模块页面内容的重新布局、配色和主题、新增模块、命令面板、其他 UX 项、依赖和 vendor 修改。

## 6. 完成后的报告
组件选择记录；顺序变更涉及的所有位置；规范修订内容；先红后绿证据；验证命令的实际结果；截图查看结论或无法查看的说明；未完成事项与无关的既有失败。
```
