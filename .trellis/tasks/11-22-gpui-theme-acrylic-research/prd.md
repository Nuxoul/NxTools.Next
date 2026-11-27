# GPUI 生态主题与亚克力材质调研

## Goal

调研 GitHub 上基于 GPUI / gpui-component（GPUI-Kit）的真实应用怎样设计主题、半透明表面和 Windows 亚克力 / Mica 材质；克隆源码对照，产出针对 NxTools 磨砂主题的设计结论与改造方向。

## Requirements

- 广泛检索并克隆多个项目做源码级对照，覆盖框架层、Windows 材质应用、模糊窗口应用和 gpui-component 业务应用。
- 每条结论都要有文件路径、行号和实际数值作依据，不凭印象。
- 对照 NxTools 当前实现，指出差异与根因。
- 只调研，不改产品代码。

## Acceptance Criteria

- [x] 克隆并阅读 25 个仓库，清单见 [research/00-repositories.md](research/00-repositories.md)。
- [x] 四份带路径与数值的子报告，见 `research/01`–`04`。
- [x] 汇总结论与改造方向，见 [research/README.md](research/README.md)。
- [ ] 用户确认改造方向（材质路径、独立材质调色板、分层取值、圆角方案、ClearType、首帧遮蔽）后，另立实施任务。

## Notes

- 克隆的源码位于会话临时目录，不进入仓库。
- 改造方向第 1 条会推翻 `11-21-desktop-motion-acrylic-style` 原生预览"只用 WCA Accent"的决定，必须由用户拍板。
