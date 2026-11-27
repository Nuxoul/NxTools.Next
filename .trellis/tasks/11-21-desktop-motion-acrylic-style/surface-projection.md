# 全软件表面投影

## 公开组件与形态

范围沿用 `design.md` 的全软件表面矩阵：每个普通 HWND 一层材质底色，正文、输入、选中和浮层保持可读层级；稀疏宿主的内容外区域保持透明。布局、控件尺寸、原保存/取消/焦点/滚动及业务实体不变。

固定 Kit/Base/Component/Assets0.10.0、gpui-pre0.3.12。使用 Component `Theme::update`、Base `Root::register_plugin` 和 `RootPlugin::style`；现有 Settings、Form、InputGroup、Input/Textarea、ListItem、Dialog/Sheet、Notification 保留自己的公开组件、变体及尺寸。无新增普通控件或框架修改。Toolbar.child 的 ghost/compact 适配不改变；独立有边界操作继续使用原 outline Button。

已核对的公开链路：

- `vendor/gpui-base-0100/src/root.rs`：Root 每次渲染按注册顺序执行插件 prepare/style/decorate，实例样式在插件 style 后覆盖；插件 render 输出是内容之上的覆盖层。style/prepare 不得 notify。
- `vendor/gpui-component-0100/src/root.rs::WindowState::style`：官方插件使用 tokens.background 绘制 Root 背景并设置文字、字体及圆角；decorate 保留官方 window_border。
- `vendor/gpui-component-0100/src/theme/mod.rs::Theme::update`：公开更新同步组件 tokens 与 Base 语义投影。颜色从已选择的主题重新载入后再映射实度，Solid 恢复原值，不在旧 alpha 上累乘。
- `rust/apps/nxtools-desktop/src/theme.rs::Theme::active`：现有业务角色投影来自同一公开颜色集。
- `docs/reference-code/desktop-materials.md`：C# 应用回执、Preparation 和原窗口 prepare/ACK/frame/reveal 所有权不变。

## 有效材质与首帧

原生意图、能力事实、成功回执和像素证据分别记录。渲染层只读取原 Preparation 已验证结果的派生投影，不持有请求、不发命令、不拥有窗口计时或显示行为。

投影必须在原准备所有者继续隐藏帧/首显之前可用，不能只依赖首显后的 runtime 移交。当前会话、事实版本、风格、深浅色及原生外观代数任一失效时，旧投影不得授权透明底层；未准备的普通窗口使用实底。关闭移除相应窗口投影。跨实体刷新仍在原延迟应用轮进行，缓存命中也不能同步重入窗口实体。

| 有效状态 | 普通 Root 底层 | 内容外稀疏区域 |
| --- | --- | --- |
| Solid | 原配色实底 | 透明 |
| Acrylic | 原配色 RGB 的 tint，露出同一 HWND 原生材质 | 透明 |
| AcrylicOpaque | 同一 Acrylic 角色体系的实底，不切换主题身份 | 透明 |
| Transparent | 透明，不启用普通 Acrylic | 透明 |
| 无当前回执 | 普通窗口实底，首显仍受原准备屏障控制 | 保留显式透明宿主策略 |

官方正文/表格/编辑器、输入和浮层采用统一的较实角色；文字、图标和状态色不降低整窗 opacity。保留选定配色 RGB、UI 字体及 Consolas/monospace 大小。不对子控件增加 blur。

当前覆盖值由用户可调的 `MaterialTuning` 生成，且每个像素只有一层半透明：Root tint = 窗口底色不透明度（默认72%）；侧栏/标题/状态/标签栏/列表/表格这些"面板"按组合覆盖率对齐内容层不透明度（默认84%，不低于窗口底色），分组卡片/折叠为面板之上40%的剩余空间，列表头/表头/表尾30%，输入60%，浮层85%且不低于97%。各角色的 alpha 由"组合覆盖率相对窗口底色"反解，嵌套不会把页面叠成不透明。Markdown 阅读面不低于0.9。正文、次要文字与分割线沿调色板自身前景色加深，使其在10%与90%灰度桌面上分别达到4.5:1/3.5:1/1.35:1，分组卡片相对底层提亮。背景内容层使用零填充，底层仅由Root绘制；回退Root为1.0。Solid重新载入原主题值，不应用上述覆盖。Dialog/Sheet外层通过公开Styled槽读取统一浮层背景，Solid保留原tokens.background完整填充。

## 生命周期与回归边界

- 同一实体在 Solid/Acrylic、配色及系统事实更新后继续存在；不重建输入、草稿、标签或任务。
- 首显前、运行时移交、复用、事实变化、回退、失败及关闭都从同一准备记录派生渲染状态。
- About 与 Markdown 从公共主题角色投影画布、阅读层、文字、边界及代码色；原字号、行距、文章宽度、纲要和编辑状态保留。
- Quick Launch/Clipboard 保留认可圆角和原生命中边界；透明通知、菜单、悬浮助手、任务栏、选择叠层不铺普通窗口底色。
- Markdown 标题高亮减少动态效果分支不建立展示帧循环；定位、纲要 dwell、取消和 ACK 不改变。

新增回归应检测实际 Root/官方控件的颜色投影、未准备/回退下的 alpha、首显前可见性、原实体身份和减少动态效果帧请求。合成测试与源门禁不代替实际 GPUI 像素、原生交互和用户视觉验收。

## 表面矩阵

完整的表面/覆盖率/可读性/动效说明见 [surface-matrix.md](surface-matrix.md)。

## 状态

统一角色、Root有效状态、About/Reader、剪切板渐隐、选择器强调色、官方弹层、标题高亮及目标外壳圆角已接入。当前材质119项、外壳4项、全局设置66项、Reader38项、标题动效4项、官方弹层2项与Clipboard缩放/激活8项通过。下载整组359通过/1失败，严格lint和目录导航保留已知阻塞；阶段四整组验收未完成。实际GPUI像素、物理交互和用户视觉验收未完成。
