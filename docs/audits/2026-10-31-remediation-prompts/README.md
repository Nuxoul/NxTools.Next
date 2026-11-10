# 修复批次提示词索引

来源：[代码质量审查问题记录](../2026-10-31-code-quality-review.md)、[UI/UX 优化方案](../../ui/2026-10-31-ux-optimization-proposal.md)。

每个批次文件都是一份完整的 Codex 提示词，整份复制到新对话即可。批次之间有共享文件，必须**按顺序**执行，并在上一批提交后再开始下一批。

## 批次状态

| 批次 | 内容 | 状态 |
| --- | --- | --- |
| 1 | 下载引擎 H1、M2 + 批量反馈 U1、同名冲突 U2 | 已完成（`.trellis/tasks/10-31-download-backpressure-conflict/`），截至 2026-11-01 尚未提交 |
| 2 | PlatformHost H5、键盘钩子 H4 + 热键状态 U3、扩展合成事件 M1 + U5 | 进行中（`.trellis/tasks/11-01-platformhost-input-and-extension-trust/`） |
| 3 | [索引服务正确性与资源](03-indexer-correctness.md)：H2、H3、M3、M4、U4，附 M8 测量、L11 | 待执行 |
| 4 | [跨进程通道与浏览器链路](04-ipc-and-browser-chain.md)：M7、L5、L6、L7、L9 | 待执行 |
| 5 | [下载、压缩、扩展遗留问题](05-download-archive-extension-leftovers.md)：L1、L2、L3、M5、L10 | 待执行 |
| 6 | [剪贴板维护路径](06-clipboard-maintenance.md)：M6、L4、L8 | 待执行，必须在第 2 批之后（共用 EventWindow.cs） |
| 7 | [键盘效率](07-keyboard-efficiency.md)：U9 下载快捷键、U10 速启操作提示 | 待执行，U10 有一项需你确认 |
| 8 | [导航结构](08-navigation-structure.md)：U6 侧栏分组、U7 打开面板入口、U8 标题栏 | 待执行，**发送前需确认文件顶部的决策** |
| 9 | [细节打磨与设计规则治理](09-polish-and-design-rules.md) | 待执行 |

## 依赖与冲突

- 第 3、4 批都会改动索引服务的进程通信（第 4 批的 M7 影响索引服务的管道读取），因此都受 AGENTS.md 的 Indexer release gate 约束。真正的服务验收需要打包并重启服务，必须单独授权，提示词中已要求 Codex 把它列为待执行项，不要自行进行。
- 第 6 批的 L8 与第 2 批同在 `WindowsPlatformAdapter.EventWindow.cs`。
- 第 7 批的 Ctrl+1…6 模块切换按 `MainModule::ALL` 的顺序映射；第 8 批若调整侧栏顺序，必须同步更新映射与提示。
- 第 9 批中压缩对话框的两点，归入已有任务 `10-30-archive-flow-ux` 的继续解压流程。

## 需要你决定的事项

1. 第 7 批 U10：默认方案不改动结果行（遵守 `10-30-quick-results-ux` 契约：右侧固定显示 Ctrl+1–9），只在操作模式标题行加“← 返回”，并在 Tooltip 和无障碍描述中加入“→ 更多操作”。若你希望选中行改为显示“→”，要先修改该契约，请在提示词中把对应选项改为 B。
2. 第 8 批：侧栏分组方式与顺序、是否去掉标题栏的“· 模块名”，见该文件顶部。
