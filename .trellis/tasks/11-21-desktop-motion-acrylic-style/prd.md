# Desktop motion and acrylic material themes

## Goal

为 NxTools.Next 设计覆盖全软件的磨砂亚克力主题和克制、由操作触发的动效。现有主题与亚克力主题是两套完整视觉体系，配色独立选择。保留现有功能、官方组件和字体选择，并验证真实 Windows 合成成本。

## Confirmed requirements

- 用户认可独立 GPUI 示例的 UnderBorder 圆角效果，并要求将这个方向用于剪切板面板。当前仅示例通过视觉确认，产品尚未集成。
- 全软件动效以反馈、空间关系和状态连续性为目的，不添加装饰性常驻动画、弹跳或逐项排队入场。
- 使用 stageshow 表达设计；演示不替代实际 GPUI、原生窗口、交互生命周期或资源测试。
- 亚克力是一套全软件主题，与现有主题并列；配色独立于主题。主题统一定义材质、透光、表面层级、边界、选中层、阴影及回退表现。
- 参考图为 [reference-acrylic.png](reference-acrylic.png)。图中可见低饱和蓝灰/紫灰底、柔化的背景色、细边界、较实的选中层。静态图片不能确定原应用采用了哪一种原生材质 API。
- 评估 CPU、内存、GPU、DWM、帧时间及功耗影响，不提供未经测量的资源增量。

## Constraints

- 固定 Kit/Base/Component/Assets 0.8.0 + gpui-pre0.3.9；没有依赖升级授权。
- Rust 管理状态/布局，GPUI 渲染，C# 管理新增产品 Windows 事实和原生适配。现有框架内部合成实现是研究依据，不授权添加第二个 HWND 材质控制器。
- 复用原主题/设置保存者、InputState、会话代数、预览控制器和原生 show/hide 边界。
- 原生窗口先获得最终位置/尺寸，再展示；不得用逐帧 HWND bounds 实现动效。
- 减少动态效果时直接进入最终视觉状态；保留行为性 hover dwell、取消、超时和 ACK。
- 保留 Sarasa UI SC Regular、明确保存的字体/配色选择及 Consolas。
- 规划阶段不改产品代码，不启动常规 runtime、Cargo、打包、安装或服务操作。

## Theme scope

- 主题：现有主题 / 磨砂亚克力主题；一次选择，全软件统一应用。
- 配色：独立保留现有颜色方案及深浅色选择，不与亚克力主题绑定。
- 字体：保留原独立设置及保存策略。
- 覆盖主窗口、各模块、设置、工具窗口、快捷启动、剪切板、菜单、Sheet/Dialog、预览、通知及 About，包含已打开和之后创建的窗口。
- 全覆盖指所有表面遵循同一主题，不是每个子控件重复进行实时模糊。正文、输入、选中项等实度由亚克力主题的统一层级规则决定。
- 代表性样例用于验证实现；全软件主题范围已确定。资源测试用于优化合成方式、层级和回退，不将主题缩减为局部效果。

动效、材质设计见 [design.md](design.md)，源码、系统资料及资源测试方案见 [research-and-validation.md](research-and-validation.md)。

## Acceptance

### Planning

- [x] 确认最新参考图和已认可的圆角方向。
- [x] 盘点现有动效、原生约束及实际减少动态效果缺口。
- [x] 区分当前 GPUI Blurred、系统 Acrylic 和 Mica 的 API/兼容性。
- [x] 记录 stageshow 分镜及资源对比方案。
- [x] 确认亚克力为全软件主题，配色独立。
- [x] 制作整套主题总览、三个 stageshow 演示及独立外观控件：[预览](../../../docs/ui/theme-preview/preview.html)。
- [ ] 用户确认主题外观和动效节奏。
- [ ] 根据视觉确认与真实 GPUI 验证结果完成产品实施计划。

### Product adoption

- [ ] 真实 GPUI 材质样例验证透明边缘、遮罩、阴影、输入、焦点及被动预览。
- [ ] 圆角补偿覆盖目标剪切板外壳；半透明填充独立验证。
- [ ] 动效一次触发、静止停止、减少动态效果正确，重开/过期会话不能复活动画。
- [ ] 两套完整主题按相同场景对比进程/DWM/GPU/帧时间，覆盖常驻大窗口与多窗口组合。
- [ ] 支持的 Windows/DPI/系统透明偏好/回退及物理交互验收。

## Theme acceptance

- [ ] 切换主题时所有已打开的窗口及之后新建的窗口一致更新，保留原焦点、输入、会话和业务状态。
- [ ] 配色、深浅色和字体选择独立保留，两套主题均可使用。
- [ ] 系统不支持透明时仍呈现亚克力主题自己的回退层级，不切回现有主题。

## Native proof evidence

Actual GPUI/Windows Acrylic, fixed-client frame alignment and correlated native dark synchronization are verified in the isolated preview. [Native verification](native-verification.md) separates compositor pixels, owned-window boundary checks and input limitations from the pending production adoption gates above.

## Isolated resource evidence

The30-run paired Solid/Acrylic Debug comparison is complete: [results and limits](performance-results.md), [structured measurements](performance-results.json), [portable raw evidence](performance-data.zip). It covers idle, changing background, finite input-state updates, real1000-row virtualized scrolling and three windows in one process. This does not complete the full-product/Release/60Hz/power acceptance gates above or authorize production integration.
