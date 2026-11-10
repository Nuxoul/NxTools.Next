# 第七批：键盘效率（U9 下载快捷键、U10 速启操作提示）

发送前确认：U10 默认使用方案 A（不改动结果行）。若要选中行改为显示“→”，把提示词中“采用方案 A”改为“采用方案 B”。

```text
你在 Windows 工作区 D:\Code\NxTools.Next-main 中工作（Rust + GPUI 桌面端，PowerShell）。任务：实现 U9（下载工作区键盘快捷键）和 U10（速启操作模式可发现性）。

## 通用规则
- 工作区里有他人未提交的改动，必须保留；不要回退、格式化或提交与本任务无关的文件。不要 commit/push，不要打包、安装或启动产品，不要访问真实网络和真实用户数据。
- 开始前阅读 AGENTS.md（Reference-driven migration：已有功能以旧实现为行为依据；GPUI 回调不得同步更新可能回写当前租用实体的实体，跨实体工作放到下一轮）、.trellis/workflow.md、.trellis/spec/shared/ui-component-selection.md、.trellis/spec/shared/ui-design.md（键盘、焦点与无障碍章节）、.trellis/spec/nxtools-desktop/backend/quality-guidelines.md 与 ui-experience-contracts.md、docs/reference-code/download.md、速启参考笔记，以及 .trellis/tasks/10-30-quick-results-ux/design.md（速启结果行契约）。方案依据：docs/ui/2026-10-31-ux-optimization-proposal.md 的 U9、U10。按 workflow 建立任务记录（建议 11-0x-keyboard-efficiency/）。
- 先写能在旧代码上失败的生产渲染回归测试（TestApp + simulate_keystrokes），再实现，并给出先红后绿的证据。
- Rust 命令在同一进程内设置 $env:CARGO_BUILD_JOBS='1'; $env:CARGO_INCREMENTAL='0'; $env:RUSTFLAGS='-C codegen-units=256'；桌面测试使用 --config 'profile.test.package.nxtools-desktop.debug=0' 并单线程；给足超时；超时后先检查残留进程，不要重复构建。
- 已知的无关失败（日期相关的日历测试、nxtools-archive 格式差异）不要修、不要压制，在报告中列出。行号以 2026-11-01 的工作区为准；如有偏移，按符号名定位。
- 不得声称做过物理键盘、IME、读屏或用户视觉验收；截图必须真正查看后再描述。

## 1. U9：下载工作区键盘快捷键
现状：
- 主窗口唯一的按键处理是热键录制（rust/apps/nxtools-desktop/src/main_window.rs 约 :531 的 on_key_down）；整个 desktop 只有 archive/progress.rs 和 ui/components/project_menu.rs 使用 KeyBinding。下载列表没有任何快捷键。
- 已有入口：新建下载 `open_new_download_window`；搜索框 `search_input`（main_window/download_page.rs 约 :1100 的 InputGroup）；多选状态与批量操作（第一批新增的 rust/apps/nxtools-desktop/src/downloads/batch.rs：异步执行，一次只允许一个批量操作；取消和删除文件的批量操作 ≥20 项时确认）；清除多选（download_batch_toolbar 中的 download-batch-clear）；关闭详情（download-detail-close）；双击行为（可配置）；模块切换 `shell.activate`，顺序为 rust/apps/nxtools-desktop/src/shell.rs:13 的 `MainModule::ALL`。

先做参考核对：在 AB Download Manager 和旧 NxTools（D:\Code\NxTools）中查找下载列表的键位定义，把入口文件和结论补记到 docs/reference-code/download.md。下表是提案，最终以参考核对的结果为准；有冲突时在报告中说明取舍：

| 键 | 行为 |
| --- | --- |
| Ctrl+N | 新建下载 |
| Ctrl+F | 聚焦下载搜索框 |
| Ctrl+A | 选中当前筛选结果中的全部任务（焦点在输入框时保持文本全选） |
| Delete | 对所选任务执行与右键菜单“删除”相同的路径（含现有确认规则） |
| Enter | 已完成任务：执行与双击相同的配置行为；未完成任务：打开任务窗口 |
| Esc | 先清除多选；无多选时关闭详情面板 |
| Ctrl+1…6 | 按 MainModule::ALL 的顺序切换侧栏模块（在整个主窗口生效） |

实现要求：
1. 使用 GPUI KeyBinding + key_context（例如下载工作区一个 context，主窗口一个 context），不要新增第二套按键分发器。焦点在 Input/Textarea/Editor、菜单、对话框、热键录制中时，这些控件原有的键盘行为优先。
2. 动作复用上面的现有入口，跨实体工作一律 cx.defer；Delete 和批量操作走第一批的异步批量路径和确认规则，不得在 GPUI 线程同步执行批量命令。
3. 可发现性：在对应按钮的 Tooltip 和右键菜单项上用官方 Kbd 显示键位。不要新增帮助面板或提示栏。
4. 诊断：只在快捷键被接受或拒绝时记录状态事件（与动作共用 correlation id），不记录按键流。

## 2. U10：速启操作模式可发现
现状：
- rust/apps/nxtools-desktop/src/quick_launch/panel.rs：进入操作模式只能用 →（光标在末尾时，约 :492）或右键（约 :1200）；← 或空输入时的 Backspace 返回（约 :482-488）。界面上没有任何提示。
- 结果行右侧的 48 DIP 提示列显示 Ctrl+1–9（约 :1136 `quick_launch_shortcut_hint`、:1284；search.rs:238）。
- 约束：.trellis/tasks/10-30-quick-results-ux/design.md 规定 Ctrl+1–9 固定在结果行右侧提示列，不加重复的行内按钮，窗口保持 620×44/410，结果行 40 DIP，图标列 26 DIP，提示列 48 DIP；完整名称和目标通过行的无障碍描述和辅助 Tooltip 提供。

采用方案 A（默认，遵守现有契约）：
- 结果行不变。
- 操作模式标题行（显示“操作 <名称>”的那一行）右侧用官方 Kbd 显示“← 返回”。
- 结果行的无障碍描述和辅助 Tooltip 末尾加入“→ 更多操作”。

方案 B（仅当用户明确选择时）：选中行的提示列从“Ctrl+N”改为“→”（Kbd），其他行仍显示 Ctrl+N。这会修改 10-30-quick-results-ux 的契约：先更新该 design.md 并注明用户批准日期，再实现；48 DIP 列宽内的实际排版必须用生产渲染边界测试验证。

两种方案都不改变窗口尺寸、行高、选择和预览逻辑，也不新增按钮。

## 3. 验收与验证
- U9 回归：每个键位的指针等价、焦点在搜索框时 Ctrl+A/Delete 不触发列表操作、多选 → Esc → 再按 Esc 关闭详情、Delete 在 ≥20 项时出现确认、Ctrl+N 打开唯一的新建窗口、Ctrl+1…6 切换模块且焦点落到正确位置。
- U10 回归：操作模式标题行出现“← 返回”，Kbd 在边界内；Tooltip 和无障碍描述包含新文字；现有 result_ux 和速启聚合测试保持通过。
- Light/Dark、默认 1060×650 和最小 920×480 下的生产渲染边界测试。可以生成截图的话，必须真正查看后再描述。
- cargo clippy -p nxtools-desktop --tests -- -D warnings；对改动文件做 rustfmt 检查；git diff --check；tests/architecture/download-ui-lifecycle.ps1；改动规范时运行 python tests/architecture/spec-contracts.py。

## 4. 范围外
侧栏分组和标题栏（第八批）、其他 UX 项、速启结果行布局（方案 B 除外）、新的帮助面板或命令面板、依赖和 vendor 修改。

## 5. 完成后的报告
参考核对结论（入口文件与最终键位表）；每个键位的实现位置；U10 采用的方案；先红后绿证据；验证命令的实际结果；未完成事项与无关的既有失败。
```
