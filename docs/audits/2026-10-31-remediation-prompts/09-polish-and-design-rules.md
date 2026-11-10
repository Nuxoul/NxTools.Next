# 第九批：细节打磨与设计规则治理

```text
你在 Windows 工作区 D:\Code\NxTools.Next-main 中工作（Rust + GPUI 桌面端，PowerShell）。任务：完成 UX 方案中的 P2 局部改进，并整理 UI 设计规则文档。

## 通用规则
- 工作区里有他人未提交的改动，必须保留；不要回退、格式化或提交与本任务无关的文件。不要 commit/push，不要打包、安装或启动产品。
- 开始前阅读 AGENTS.md、.trellis/workflow.md、.trellis/spec/shared/ui-component-selection.md、.trellis/spec/shared/ui-design.md、.trellis/spec/nxtools-desktop/backend/ui-experience-contracts.md；方案依据：docs/ui/2026-10-31-ux-optimization-proposal.md 的 P2 与“设计规则治理”两节。压缩部分另读 .trellis/tasks/10-30-archive-flow-ux/ 下的 design.md、continuation-design.md、outcomes.md 和 docs/reference-code/archive.md。按 workflow 建立任务记录（建议 11-0x-polish-and-design-rules/）。
- 先写能在旧代码上失败的生产渲染回归测试，再修改，并给出先红后绿的证据（纯文档改动除外）。
- Rust 命令在同一进程内设置 $env:CARGO_BUILD_JOBS='1'; $env:CARGO_INCREMENTAL='0'; $env:RUSTFLAGS='-C codegen-units=256'；桌面测试使用 --config 'profile.test.package.nxtools-desktop.debug=0' 并单线程；给足超时；超时后先检查残留进程，不要重复构建。
- 已知的无关失败（日期相关的日历测试、nxtools-archive 格式差异）不要修、不要压制，在报告中列出。行号以 2026-11-01 的工作区为准；如有偏移，按符号名定位。
- 不得声称做过用户视觉验收；截图必须真正查看后再描述。

## 1. 下载页
1. 下载计划按钮的图标：rust/apps/nxtools-desktop/src/main_window/download_page.rs 约 :1151 `icon_action("download-schedule", MaterialIcon::History, "下载计划…")`，History 的语义是“历史”。改为已注册的 `MaterialIcon::CalendarMonth`（icons.rs）。只改这个入口；同文件其他使用 History 的地方（约 :2072、:2112、:2120、:2382、:2427、:2435、:3057）先逐个确认语义，属于“计划”含义的一并修改，其余保持不变，并在报告中列出。
2. 空状态文案：约 :1302 “点击右上角新建下载，提交真实 HTTP 或 HTTPS 地址；也可以从浏览器直接发起下载。”改为“点击右上角新建下载，粘贴 HTTP 或 HTTPS 链接；也可以从浏览器直接发起下载。”同步更新断言这段文案的测试。

## 2. 速启结果图标占位
- 2026-10-22 的合成截图（docs/audits/2026-10-22-ui-experience-captures/dark-continuous-results.png）中，第一行“Windows 终端”的图标是一个转圈的样式。先查清这是图标加载中的占位动画、合成夹具专用图标，还是真实图标。
- 如果是加载占位动画：在图标异步加载期间改为静态的类型图标占位，避免列表中出现持续动画；遵守 reduced-motion 规则，不改变 26 DIP 图标列、行高、选择和预览逻辑。如果不是加载占位，只在报告中说明，不做修改。

## 3. 压缩：继续空间回收解压流程（归入已有任务 10-30-archive-flow-ux）
现状（截图 .trellis/tasks/10-30-archive-flow-ux/assets/continuation-completed-light.png、continuation-confirmation.png、continuation-pending-dark.png）：
- “已完成”状态下，表单只剩一个禁用的“继续空间回收解压”主按钮和“取消”。outcomes.md 规定：已完成的记录只报告记录中的完成状态，不代表当前输出有效；不提供继续动作；表单禁止提交，不显示后续操作的警告和偏好。
- 表单正文中的不可撤销警告（Alert）与独立确认对话框的说明内容重复。

要求：
1. 已完成状态：不再显示禁用的主按钮。主动作改为“打开目标文件夹”（复用现有的打开目标文件夹或 Shell reveal 入口，不新增原生所有者），次动作改为“关闭”。目标文件夹缺失或不可用时，主动作禁用并给出原因。仍然不得提供继续解压的路径；不得把“已完成”表述为“当前输出有效”。
2. 警告去重：表单正文的 Alert 保留一句摘要（例如“继续后会永久释放已核验的原压缩包数据，不能撤销”），完整说明只放在独立确认对话框中。确认对话框的内容、按钮和独立确认流程保持不变，安全与不可逆说明仍然出现在决策点。
3. 在 10-30-archive-flow-ux 的 continuation-design.md 中记录这两项修改，以及用户批准日期（来自 docs/ui/2026-10-31-ux-optimization-proposal.md）。
4. 回归：已完成状态没有禁用的主按钮，“打开目标文件夹”只发出一次正确的命令；pending、failed、missing-target、disconnected 状态的行为保持现有测试；确认对话框文案不变；Light/Dark 和最小尺寸下底部按钮在边界内。

## 4. 设计规则治理（仅文档）
现状：
- .interface-design/system.md 仍写着 208px 侧栏、18/22px 控件高度、项目自定义配色等已被取代的数值（现行为 176px 侧栏、Kit 默认尺寸、官方主题），文件中没有标注已过时。
- .trellis/spec/shared/ui-design.md 有 300 多行，按日期累积修订；要知道哪条现行有效，必须读完整个修订流水。

要求：
1. 在 .interface-design/system.md 顶部加一段说明：这是历史文档，现行规则见 .trellis/spec/shared/ui-design.md 与 ui-component-selection.md；正文不删。
2. 在 .trellis/spec/shared/ui-design.md 开头加一张“现行规则速查表”，每一行都链接到正文中确立该规则的修订段落，不复制大段内容。至少包括：组件与主题（Kit 0.8.0、官方主题与当前默认主题）、主窗口与侧栏尺寸、各弹出面板与窗口的原生预算（速启 620×44/410、剪贴板 300×440、新建下载与任务窗口系列、通知）、通知规则、焦点与动效规则、已被用户否决的方案清单。表中的每个数值都必须从正文或源码核实，不得凭记忆填写。
3. 同时检查 docs/ui/layout-quality/*.md 中被后续修订取代的数值，在相应位置标注“已被取代，见……”，不要删除历史内容。
4. 运行 python tests/architecture/spec-contracts.py；如果改动了组件目录相关文档，再运行 python tests/architecture/gpui-component-catalog.py。

## 5. 验证
- 桌面：过滤运行下载页、速启结果、压缩继续流程的受影响测试
- cargo clippy -p nxtools-desktop --tests -- -D warnings；对改动文件做 rustfmt 检查；git diff --check
- 文档检查见第 4 节
- 可以生成截图的话（下载空状态、继续解压已完成状态），必须真正查看后再描述。

## 6. 范围外
其他审查问题、导航与快捷键（第七、八批）、配色和主题调整、压缩流程中除上述两项以外的修改、依赖和 vendor 修改、打包。

## 7. 完成后的报告
每项修改的文件与符号；History 图标逐处的语义判断；速启图标占位的调查结论；规则速查表内容及每个数值的来源；先红后绿证据；验证命令的实际结果；截图查看结论或无法查看的说明；未完成事项与无关的既有失败。
```
