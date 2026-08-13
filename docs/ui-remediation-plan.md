# NxTools.Next UI 整体整改计划

> 状态：P0 实施中
> 建立日期：2026-08-13
> 视觉目标：以 Zed 为主参考，以当前剪贴板工作区为产品内基准，保持高级、简约、极客、紧凑。
> 适用范围：桌面 GPUI、浏览器扩展 Popup、网页媒体悬浮入口与拖放目标。

## 1. 文档目的

下载、速启和剪贴板的主要功能已经基本具备，但视觉实现尚未形成统一的组件层和布局规则。
本计划用于统一三个模块及其辅助窗口的字体、颜色、控件尺寸、层级、布局、状态与动效，避免继续
按单个截图对局部控件反复打补丁。

本轮结论来自源码级扫描，没有启动桌面 GUI，因此以下内容分为：

- **源码已确认问题**：能从现有布局、常量、组件和资产映射直接确认；
- **实机验收项**：字体清晰度、DPI、窗口裁剪、动效、焦点和实际视觉差异，必须在 Release 包中
  使用固定截图矩阵验证，不能只靠源码或几何测试宣称完成。

本计划不改变 Rust 业务核心、C# Windows 适配和 GPUI 渲染的职责边界。

## 2. 扫描范围

### 2.1 桌面主壳层

- 主标题栏、窗口控制按钮、拖动区；
- 左侧模块导航；
- 下载、速启、剪贴板三个主工作区；
- 全局设置入口与全局设置窗口；
- 通知浮层、空状态、错误状态和加载状态。

### 2.2 下载

- 主任务表、过滤条、搜索、排序、批量操作和详情 dock；
- 新建下载、HTTP 探测、HLS/DASH 轨道选择和分类弹层；
- 下载设置五个页面及分类编辑器；
- 计划设置、任务进度/失败/完成窗口、删除确认和关机倒计时；
- 任务栏内部下载覆盖条。

### 2.3 速启

- 主搜索弹窗、结果列表、动作模式和右键菜单；
- 文件、应用、命令、计算、网址等结果类型；
- 预览窗口；
- 文件对话框附着搜索；
- 主窗口中的热键、索引、上下文、历史、自定义命令和诊断设置。

### 2.4 剪贴板

- 主窗口设置页；
- 独立剪贴板面板、分类栏、搜索、历史列表和条目操作；
- 文本、图片和文件预览；
- 编辑、分类和来源菜单。

### 2.5 浏览器扩展

- 扩展 Popup 的任务、捕获、工具和设置页面；
- 网页媒体悬浮下载入口与资源选择面板；
- 网页拖放下载目标。

## 3. 当前总体问题

| 领域 | 源码现状 | 影响 | 目标 |
| --- | --- | --- | --- |
| 设计令牌 | `theme.rs` 主要定义颜色和少量弹窗指标，没有完整的字号、控件、间距和窗口层级 | 页面继续直接写像素值 | 所有页面只消费统一的语义令牌 |
| 字体 | 主窗口和下载分别出现十余种字号，包含大量 8.5、9.5、10.5 px 中文 | 中文发虚、字重和行高不一致 | 固定少量整数级字号，中文正文不低于 11 px |
| 控件高度 | 22–48 px 的按钮和输入在同一页面混用 | 同级操作看起来没有关联 | 使用明确的 Zed 式控制尺寸层级 |
| 圆角 | 2–38 px 混用 | 卡片、按钮、状态胶囊层级混乱 | 控件 4 px、浮层 6 px，胶囊只用于状态 |
| 组件复用 | 色彩基本集中，但按钮、设置行、标题栏、弹层和 footer 大量手写 | 每次局部优化都会再次漂移 | 建立共享 GPUI 组件层 |
| 模块结构 | `downloads.rs` 约 1.4 万行，`main_window.rs` 约 7100 行 | 布局和交互难以单独维护与测试 | 按 surface/controller/component 拆分 |
| 图标 | `Settings`、`Terminal` 已由 `MaterialIcon` 引用，但没有登记到 `NxToolsAssets` | 可能显示空图标或错误占位 | 所有枚举图标必须通过资产完整性门禁 |
| 浏览器扩展 | 使用另一套深黑、青绿色、渐变、网格和毛玻璃语言 | 与桌面完全割裂 | 与桌面共享语义颜色和控件层级 |
| 视觉回归 | 下载有较多几何测试，速启与扩展仍有源码字符串断言 | 可防部分裁剪，无法防视觉品质回退 | 建立真实窗口、交互和截图矩阵 |

## 4. 视觉基准与硬规则

### 4.1 产品内基准

当前剪贴板工作区作为 NxTools.Next 的内部视觉基准：

- 连续工作面，不使用对称卡片墙；
- 低对比边界，选中态使用中性填充和窄强调标记；
- 26–28 px 普通操作、稳定的行高和文字基线；
- 主文字、说明、元数据拥有明确层级；
- 紧凑来自稳定的布局和较少空白，而不是使用 8–9 px 中文。

剪贴板的业务布局和默认面板尺寸应尽量保持，不因组件统一而被重新设计。

### 4.2 Zed 对齐规则

- 背景使用石墨色连续层级：Background、Surface、Elevated、Element；
- 大多数按钮使用 subtle/transparent，只有当前主操作使用 filled/accent；
- Accent 只表示焦点、选择和主操作，不铺满大面积导航或卡片；
- 普通列表行不画独立卡片，选中行使用中性背景；
- 不使用无意义的大阴影、大圆角、渐变、玻璃和装饰性胶囊；
- 窗口、菜单、Popover、列表和设置行采用稳定的尺寸体系；
- 动效短促、一次性，空闲时不持续申请动画帧。

## 5. P0：设计系统基础

这些项目必须先完成，之后才能开始逐模块迁移。

| ID | 整改项 | 具体动作 | 完成标准 |
| --- | --- | --- | --- |
| DS-01 | 字体角色 | 建立 UI/CJK/Mono 字体角色；清除页面内硬编码的 `Microsoft YaHei UI`、`Consolas` 等名称 | 字体名称只出现在主题/字体模块 |
| DS-02 | 字号层级 | 建议固定为 Title 14、Body 12、Label 11、Meta 10；中文普通信息不得小于 11 px | 生产 UI 不再出现任意小数中文字号 |
| DS-03 | 行高与字重 | 为四级字号定义固定 line-height；普通正文使用 Regular，标题最多 Medium/Semibold | 同级文字基线和视觉重量一致 |
| DS-04 | 控件尺寸 | 建立 18 px compact、22 px default、28 px form action、32 px input 四级尺寸 | 同级按钮、输入和图标按钮高度一致 |
| DS-05 | 间距尺度 | 只保留 2/4/6/8/12/16/24 px 间距 | 页面不再添加一次性 magic spacing |
| DS-06 | 圆角尺度 | 控件 4 px、浮层 6 px；窗口根和连续列表不额外圆角 | 删除无语义的大圆角和伪胶囊 |
| DS-07 | 语义颜色 | 补齐 hover、pressed、disabled、focus、selected、warning 等语义角色 | 业务页面不得直接定义新的 RGB/Hex 颜色 |
| DS-08 | 操作组件 | 新建 ActionButton、IconButton、SegmentedControl 和 Toggle | 下载、速启、剪贴板不再分别实现按钮状态 |
| DS-09 | 表单组件 | 新建 Field、SettingRow、Section、PathPicker、NumericField、InlineError | 设置页使用同一标签宽度、输入高度和说明层级 |
| DS-10 | 列表组件 | 新建 CompactListRow、ResultRow、EmptyState、StatusBadge | 下载任务、速启结果、分类和命令列表共享交互语法 |
| DS-11 | 窗口组件 | 新建 ClientTitlebar、DialogShell、SettingsShell、DialogFooter | 所有辅助窗口具有相同标题栏和 footer |
| DS-12 | 浮层组件 | 新建 Popover/ListBox/Menu/Tooltip 视觉包装 | 分类、动作、来源和筛选弹层统一 |
| DS-13 | 动效令牌 | 定义 100–140 ms 进入/退出与 140–180 ms 尺寸变化 | 空闲态无循环动画，支持 reduced motion |
| DS-14 | 图标完整性 | 把 `settings.svg`、`terminal.svg` 等所有 `MaterialIcon` 加入 `NxToolsAssets`，枚举驱动测试 | 任意枚举值都能实际加载对应资源 |
| DS-15 | 品牌资产 | 主标题栏与浏览器扩展使用真实 NxTools 图标，不使用文字/CSS 图形模拟 | 所有品牌位置使用同一正式资产 |

建议新增目录：

```text
rust/apps/nxtools-desktop/src/ui/
  components/
    action.rs
    field.rs
    list.rs
    popover.rs
    settings.rs
    titlebar.rs
  metrics.rs
  typography.rs
```

## 6. P1：全局壳层

| ID | 表面 | 当前问题 | 整改动作 | 验收 |
| --- | --- | --- | --- | --- |
| SH-01 | 主标题栏 | 手写 `NX` 标志，字体和按钮仍有独立尺寸 | 使用正式图标、统一 36 px 标题栏和 28–36 px hit target | 拖动、最小化、最大化、关闭功能不回归 |
| SH-02 | 全局设置入口 | 与标题栏其他操作缺少统一层级 | 使用共享 subtle action；保持顶栏右侧固定位置 | 窗口缩放后不挤压标题和窗口按钮 |
| SH-03 | 主侧栏 | 图标、标题、说明基线尚未完全受令牌约束 | 保留 208 px 结构，统一 42 px 行和文字层级 | 三模块切换时侧栏不跳动 |
| SH-04 | 模块页头 | 三个工作区的标题、摘要、搜索和主操作结构不同 | 建立统一 ModuleHeader | 同尺寸窗口下三页页头高度一致 |
| SH-05 | 全局设置窗口 | 复用下载 helper，仍是另一套窗口语法 | 迁移到共享 SettingsShell | 标题栏、Section、Toggle、footer 与下载设置一致 |
| SH-06 | 通知 | 368 px 阴影卡片偏厚重，内部动作尺寸较小 | 改为轻量 elevated surface、4 px 圆角、单侧语义标记 | 三条通知堆叠不遮挡窗口按钮和主操作 |
| SH-07 | 空/错/加载状态 | 各模块分别构造 | 统一 EmptyState、InlineError、ProgressState | 同类状态文案、图标、间距一致 |
| SH-08 | 托盘 | 原生菜单与桌面视觉职责容易混淆 | 保持 Windows 原生菜单，只统一正式图标和文案 | 不引入自绘托盘菜单 |

## 7. P1：下载模块

### 7.1 主任务页

| ID | 整改项 | 具体动作 | 完成标准 |
| --- | --- | --- | --- |
| DL-01 | 任务表 | 统一为连续表面和稳定列宽，名称/路径为主列 | 长名称不会推走状态和操作列 |
| DL-02 | 任务行 | 固定约 46–48 px 行高和两级文字；清理行内任意字号 | 中文名称、路径和错误信息清晰可读 |
| DL-03 | 选中态 | 中性整行填充加左侧 2 px accent | 不使用亮色卡片边框 |
| DL-04 | 进度信息 | 统一进度条高度、速度、已下载和剩余时间布局 | 所有任务数值垂直对齐 |
| DL-05 | 失败信息 | 错误为辅助行，可展开查看完整详情 | 错误文本不压缩主列或污染按钮文案 |
| DL-06 | 过滤与排序 | 统一 chip/segmented control，减少胶囊数量 | 全部、下载中、等待等状态宽度稳定 |
| DL-07 | 详情 dock | 固定标签列、数值列、Tab 和滚动区 | 未选中任务时完全收起；选中后不浮成独立卡片 |
| DL-08 | 批量操作 | 只在多选时出现，使用同一 toolbar | 单选和无选择时不保留空操作区 |

### 7.2 下载设置

| ID | 整改项 | 具体动作 | 完成标准 |
| --- | --- | --- | --- |
| DL-09 | 设置壳层 | 保留左导航＋内容列＋固定 footer，迁移到共享 SettingsShell | 五页切换不改变窗口几何和草稿 |
| DL-10 | 字体/控件 | 所有中文标签至少 11 px；输入 32 px，普通操作 28 px | 不再混用 xsmall/small/自绘高度 |
| DL-11 | 分类列表 | 左侧使用标准 master list，名称和目录两级展示 | 选中、禁用、hover 清晰但不过度强调 |
| DL-12 | 分类编辑器 | 名称、保存目录、扩展名、URL 使用统一 Field | 标签基线和左右列宽保持一致 |
| DL-13 | 分类动作 | 启用放标题行；上下移动、删除放工具组 | 删除只有危险 hover，不永久高亮 |
| DL-14 | 请求上下文 | Header、Cookie、证书使用标准 form section | 长 Header 编辑器不挤压 footer |
| DL-15 | 传输设置 | 连接、分段、限速采用对齐的两列 SettingRow | 两列只细分父宽，不形成独立卡片墙 |
| DL-16 | FFmpeg | 路径、自动安装、来源链接使用全宽顺序行 | 自动下载和手动选择层级明确，无大片空白 |
| DL-17 | 窗口/进度设置 | 任务窗和任务栏设置拆为稳定小节 | Toggle、数值和模式选择对齐 |
| DL-18 | 诊断页 | 配置、日志、数据库分别占满内容列 | 路径可复制/打开，不能被半宽裁切 |

### 7.3 新建与辅助窗口

| ID | 表面 | 整改动作 | 完成标准 |
| --- | --- | --- | --- |
| DL-19 | 新建下载 | 统一地址、目录、文件名、分类的标签和输入网格 | 文件名与分类标签、输入框位于同一基线 |
| DL-20 | 探测摘要 | 使用右侧窄 inspector 或字段尾部状态，不使用独立大卡片 | 探测前后窗口只按内容需要变化 |
| DL-21 | 分类弹层 | 用有界可滚动 ListBox 替代四项分页；支持键盘搜索 | 32 类全部可达，无需连续翻七页 |
| DL-22 | 分类信息 | 每项名称＋目录两行，完整目录放 Tooltip | 长路径可辨认，不靠裁切猜测 |
| DL-23 | HLS/DASH | 轨道采用标准 radio list；主行显示清晰度，副行显示编码/码率 | 自动选择与具体轨道不再视觉相同 |
| DL-24 | Footer | 稍后下载、添加、下载、取消统一 28 px 操作层级 | 主操作唯一，次操作不抢强调色 |
| DL-25 | 任务窗 | Progress/Failed/Completed 共享同一骨架 | 状态切换保持窗口中心且不出现双窗口 |
| DL-26 | 完成窗 | 打开文件为主操作，目录/拖放为次操作 | 删除不进入完成态主路径 |
| DL-27 | 计划/删除/关机 | 统一 DialogShell 和危险操作语义 | 原生关闭、取消和确认生命周期保持现有安全契约 |
| DL-28 | 任务栏覆盖条 | 提升数字清晰度，编辑态只增加边框和拖动柄 | 正常/编辑切换不改变信息位置 |

## 8. P1：速启模块

| ID | 表面 | 整改动作 | 完成标准 |
| --- | --- | --- | --- |
| QL-01 | 主弹窗 | 保留 620×410 和 42 px 搜索栏，迁移到统一字体与颜色令牌 | 空查询仅显示搜索框，非空稳定展开 |
| QL-02 | 结果行 | 应用、文件、命令、计算、网址统一图标槽、标题、副信息和快捷提示 | 不同结果类型切换不跳列 |
| QL-03 | 选择态 | 使用 neutral selected surface，不给真实 Shell 图标加装饰色块 | 键盘和鼠标选中完全一致 |
| QL-04 | 动作模式 | 复用 ResultRow，只增加轻量模式提示 | 不表现成第二套弹窗 |
| QL-05 | 预览 | 与剪贴板共享 PreviewShell 和媒体承托面 | 预览复用时只有一次 100–140 ms 动效 |
| QL-06 | 文件对话框搜索 | 统一搜索输入、结果行、边界和阴影 | 100/125/150% DPI 均不越出宿主对话框 |
| QL-07 | 主窗口设置 | 拆成常规、索引、上下文、命令、诊断五页 | 不再在一个长页面堆叠所有设置 |
| QL-08 | 热键设置 | 使用统一 HotkeyField 和录制态 | 录制、等待、失败和已应用状态清晰 |
| QL-09 | 索引卷 | 用复选列表替代大量 chip | 全部/指定卷的语义明确 |
| QL-10 | 排除目录 | 使用标准路径列表与尾部操作 | 长路径 Tooltip、删除 hit target 和键盘操作可用 |
| QL-11 | 自定义命令 | 使用 master-detail：左列表、右编辑器 | 新建、启用、编辑、删除不挤在一行 |
| QL-12 | 性能与日志 | 移入诊断页，等宽展示数值 | 普通用户路径不被性能指标干扰 |

## 9. P2：剪贴板模块

剪贴板只做“共享组件迁移＋视觉保护”，不进行结构性重做。

| ID | 表面 | 整改动作 | 保护项 |
| --- | --- | --- | --- |
| CL-01 | 主窗口设置 | 抽取连续双栏设置面为共享布局参考 | 不改当前密度和主次层级 |
| CL-02 | 剪贴板弹窗 | 使用统一字体、图标、菜单和状态组件 | 保持 300×440 默认尺寸和 8 px 安全边距 |
| CL-03 | 历史行 | 抽成共享 CompactResultRow | 保持图片比例、元数据和 hover 操作 |
| CL-04 | 分类栏 | 统一 Tooltip、菜单和焦点态 | 保持横向滚动和拖动排序 |
| CL-05 | 预览窗 | 与速启共享 PreviewShell | 保持现有预览位置、尺寸与资源上限 |
| CL-06 | 回归基准 | 迁移前后建立固定截图对照 | 不允许以“统一”为由降低当前舒适度 |

## 10. P1：浏览器扩展

### 10.1 Popup

| ID | 整改项 | 当前问题 | 整改动作 |
| --- | --- | --- | --- |
| EX-01 | 色彩 | 独立使用 `#0d0f11 + #53c7d8` | 与桌面共享石墨背景和 `#4791bf` accent 语义 |
| EX-02 | 背景 | 径向渐变、网格和半透明层过多 | 改为纯色连续 surface 和细分隔线 |
| EX-03 | 字体 | 大量 8.5–10.5 px 文本 | 中文正文提升到 11–12 px，元数据才使用 9–10 px |
| EX-04 | 品牌 | 使用三根 CSS 柱模拟标志 | 使用正式扩展图标资产 |
| EX-05 | Tab | 活动 Tab、计数和强调色较重 | 活动 Tab 仅使用文字＋底部 2 px accent |
| EX-06 | 资源列表 | meta chip 数量过多 | 稳定展示类型、清晰度、大小，其他内容放详情/Tooltip |
| EX-07 | 工具页 | Feature card 偏卡片墙 | 改为普通 command/settings rows |
| EX-08 | 设置页 | 与桌面设置语法不同 | 使用相同 SettingRow、Toggle 和 footer 层级 |
| EX-09 | 状态栏 | 捕获扫光持续运行 | 状态变化时一次性 pulse，稳定后停止动画 |
| EX-10 | 测试 | 当前测试锁死青绿色和 signal sweep | 改为共享 token、字体下限、focus 和 reduced-motion 门禁 |

### 10.2 网页注入界面

| ID | 表面 | 当前问题 | 整改动作 |
| --- | --- | --- | --- |
| EX-11 | 媒体入口 | 青绿渐变圆球、强阴影、毛玻璃 | 28 px 石墨色 IconButton，使用真实下载图标 |
| EX-12 | 媒体选择面板 | 9 px 圆角、渐变和大量胶囊 | 使用 6 px Popover 和标准两级资源行 |
| EX-13 | 拖放目标 | 大圆角、虚线青绿、文字箭头 | 使用正式图标、低对比 surface；进入时只增强边框和背景 |
| EX-14 | 页面隔离 | 样式内嵌在 content script 中 | 提取为可测试的 Shadow DOM 样式模块和共享 token 文件 |
| EX-15 | 可访问性 | 视觉操作主要依赖标题和鼠标 | 补齐 aria、键盘导航、Escape 关闭和 focus return |

## 11. 代码结构整改

| ID | 文件/模块 | 当前风险 | 调整方向 |
| --- | --- | --- | --- |
| AR-01 | `downloads.rs` | 八个以上 View 和多个窗口集中在约 1.4 万行文件 | 按 settings/new-task/task-window/aux-dialog 拆分 |
| AR-02 | `main_window.rs` | 壳层、三个模块设置、通知和大量 helper 混合 | 保留 shell，模块页面移入各自 `ui` 子模块 |
| AR-03 | `network_settings.rs` | 视觉上依赖下载设置 helper | 改依赖通用 SettingsShell，不依赖下载模块命名 |
| AR-04 | `quick_launch.rs` | 应用编排、弹窗、预览和渲染集中 | 抽 quick-launch panel、preview、row presentation |
| AR-05 | 浏览器 `content-script.js` | 功能逻辑和大段 Shadow DOM CSS 混合 | 抽 overlay/drop-target view 与样式模块 |
| AR-06 | 重复像素 | 多处手写 28/30/32 px、圆角和 padding | 只能通过共享组件和 metrics 使用 |

拆分只允许移动 GPUI 展示和本地交互状态，不得把业务策略移出 Rust 核心或移入 C#。

## 12. 实施批次

### 批次 1：设计系统与资产

- [ ] 完成 DS-01 至 DS-15；
- [x] 补齐 Settings/Terminal 等图标加载；
- [x] 为首批共享 Action/SettingRow/Section/Toggle 建立生产窗口几何和点击回归；
- [ ] 捕获剪贴板当前基准截图，作为后续对照。

首个 P0 纵向切片（2026-08-13）：

- 已建立单一 UI/CJK/Mono 字体入口、Title/Body/Label/Meta 字号与行高、
  18/22/28/32 px 控件、4/6 px 圆角、间距和短动效令牌；尚未将所有旧页面硬编码迁完，
  因此 DS-01 至 DS-13 不提前勾为整体完成。
- 已建立共享 ActionButton、SettingRow、Section、Toggle 和主操作 variant，并先迁移
  全局设置窗口；该窗口不再依赖下载模块私有 helper。
- 主标题栏已改用已有正式 NxTools PNG 资产；MaterialIcon 的资产门禁改为枚举驱动，
  Settings/Terminal 缺图将直接使测试失败。
- 真实 GPUI 门禁覆盖 36 px 主标题栏、18 px 品牌图标、28 px 全局设置操作、
  36 px 设置标题栏、同宽 Section、40×22 px Toggle，以及原有标题栏/Apply 点击生命周期。
- 本切片没有启动 GUI，也没有冒领 100/125/150% DPI 和 Release 截图人工验收。
- 自动化验证：desktop 281 项、targeted Clippy、资产/标题栏/设置交互门禁通过；workspace
  全量曾完整通过一次。最终复核时既有 HLS 带宽暂停时序用例出现 `HLS task did not start`
  瞬断，随后两次 exact 复跑为一次通过、一次同样瞬断；本切片未修改下载引擎，未把该结果
  误报为全绿。

第二个 P0 纵向切片（2026-08-13）：

- 原先过宽的单文件 `ui/foundation.rs` 已拆为 `typography.rs`、`metrics.rs`、`motion.rs`
  与 `ui/components/*`，避免令牌、窗口骨架和业务控件继续堆在同一模块。
- 已建立并实际使用 Action/IconAction、SegmentedControl、Field、InlineError、
  ClientTitlebar、SettingsShell、SettingsSection、SettingsRow、Toggle 与 DialogFooter；
  CompactListRow、EmptyState、StatusBadge、Popover/ListBox 先建立受限 API，待首个真实业务表面
  迁移和门禁完成后再把 DS-10/DS-12 勾为完成。
- 全局设置窗口已迁移到共享 SettingsShell；代理地址/端口输入、代理模式、系统设置 Toggle、
  错误提示、Apply/Cancel 与原生关闭继续复用原 persistence/IPC 生命周期，没有把业务策略
  搬进组件层。
- 真实生产窗口门禁覆盖 titlebar/content/fixed-footer 几何、同宽 section、28 px segmented
  option、32 px field、40×22 px toggle、inline error 位于 footer 上方，以及共享关闭按钮销毁
  原窗口实体。
- NxTools.Next 当前固定的 GPUI 0.2.2 / gpui-component 0.5.1 尚未暴露 Zed 新版使用的
  accessibility role/label builder；IconAction 已提供可见 Tooltip，但 DS-08 不会在这个缺口
  关闭前提前勾完。
- 自动化验证：全局设置 7 项、desktop 284 项和 targeted Clippy 全部通过。workspace 全量
  仅既有 HLS 重启恢复 loopback 用例出现一次本地请求瞬断；该 exact 用例随即 1/1 通过，
  因此如实记录为全量非全绿、精确门禁通过。本切片未启动 GUI、未打包、未冒领 DPI/截图
  人工验收。
- 最终 Standards/Spec 双轴复审的阻塞项均为 0；复审中发现的 Field 仅撑外壳、
  SegmentedControl 单选/单外边框不自证、Cancel 未走生产点击门禁等问题已在交付前关闭。

### 批次 2：主壳层与全局设置

- [ ] 完成 SH-01 至 SH-08；
- [ ] 主标题栏、侧栏、模块页头和通知统一；
- [x] 全局设置迁移到 SettingsShell。

### 批次 3：下载全套界面

- [ ] 完成 DL-01 至 DL-28；
- [ ] 先主任务页，再设置，再新建/媒体弹窗，最后辅助窗口和覆盖条；
- [ ] 每个子切片保持原有下载生命周期和协议行为。

### 批次 4：速启

- [ ] 完成 QL-01 至 QL-12；
- [ ] 保留现有 popup 尺寸和性能门禁；
- [ ] 主窗口设置拆页后再统一自定义命令编辑器。

### 批次 5：剪贴板保护性迁移

- [ ] 完成 CL-01 至 CL-06；
- [ ] 每次迁移都与批次 1 的剪贴板基准截图对照；
- [ ] 不改面板、预览和粘贴生命周期。

### 批次 6：浏览器扩展

- [ ] 完成 EX-01 至 EX-15；
- [ ] Popup 和网页注入层使用同一份浏览器 UI token；
- [ ] 更新现有 Popup contract tests，不再锁死旧青绿色和循环扫光。

## 13. 自动化验收矩阵

### 13.1 设计令牌门禁

- [ ] 除主题模块外，生产 UI 不直接写字体家族；
- [ ] 除确有语义的特殊展示外，生产 UI 不直接写任意字号、圆角和控件高度；
- [ ] 所有 MaterialIcon 都能从 `NxToolsAssets` 加载；
- [ ] 浏览器扩展颜色变量与桌面语义色有明确映射；
- [ ] 空闲状态没有持续动画。

### 13.2 GPUI 真实窗口门禁

- [ ] 主窗口：标题栏拖动、三枚窗口按钮、全局设置、三模块切换；
- [ ] 下载：任务选择、详情显隐、设置五页、新建下载、分类弹层、HLS/DASH、进度到完成；
- [ ] 速启：折叠/展开、结果选择、动作模式、预览、文件对话框附着搜索；
- [ ] 剪贴板：分类、搜索、历史 hover、预览、编辑和关闭；
- [ ] 所有设置页：Apply、Cancel、原生关闭和草稿保留/丢弃行为。

### 13.3 浏览器扩展门禁

- [ ] Popup 四个 Tab 的键盘导航和 focus return；
- [ ] 空任务、离线、捕获中、资源列表、批量选择、保存失败状态；
- [ ] 页面媒体入口打开、资源选择、Escape 关闭；
- [ ] 拖放目标进入、离开、接受和失败状态；
- [ ] reduced-motion 下无循环或位移动画。

## 14. Release 人工截图矩阵

每次视觉批次完成后，使用同一 Release 构建、同一主题和稳定测试数据截图。

### 14.1 DPI

- [ ] 100%；
- [ ] 125%；
- [ ] 150%。

### 14.2 主窗口

- [ ] 1060×500 默认窗口；
- [ ] 920×480 最小窗口；
- [ ] 最大化窗口；
- [ ] 下载、速启、剪贴板三个工作区；
- [ ] 全局设置、下载设置每个页面。

### 14.3 状态

- [ ] 空；
- [ ] 加载；
- [ ] 正常数据；
- [ ] hover；
- [ ] 键盘 focus；
- [ ] selected；
- [ ] disabled；
- [ ] error；
- [ ] Popover/Menu 打开；
- [ ] 长中文、长路径和长文件名。

### 14.4 独立窗口

- [ ] 新建下载普通/HLS/DASH；
- [ ] 下载中、暂停、失败、完成；
- [ ] 删除确认、计划、关机倒计时；
- [ ] 速启折叠/展开、动作模式、预览；
- [ ] 剪贴板 300×440 与调整大小后的布局；
- [ ] 浏览器 Popup 和网页注入界面。

## 15. 完成定义

只有同时满足以下条件，才可以声明整体 UI 整改完成：

1. 三个模块和浏览器扩展使用同一套语义色、字体、控件尺寸和交互状态；
2. 下载、速启、剪贴板不再分别手写同类基础组件；
3. 中文在 100%、125%、150% DPI 下清晰，普通正文没有依赖 8–10 px 小字；
4. 主工作区不出现卡片墙、无意义大留白、装饰性渐变和过量胶囊；
5. 所有可见操作拥有 hover、pressed、focus、disabled 和 Tooltip/accessible name；
6. 所有窗口在默认、最小和最大化尺寸下无裁剪、错位和不可达操作；
7. 自动化交互、几何门禁与 Release 截图矩阵全部通过；
8. 剪贴板当前舒适度没有因共享组件迁移而下降；
9. 未改变 Rust/C#/GPUI 职责边界，也未破坏下载、速启或剪贴板生命周期。

## 16. 相关源码入口

- `rust/apps/nxtools-desktop/src/theme.rs`
- `rust/apps/nxtools-desktop/src/main_window.rs`
- `rust/apps/nxtools-desktop/src/downloads.rs`
- `rust/apps/nxtools-desktop/src/network_settings.rs`
- `rust/apps/nxtools-desktop/src/quick_launch.rs`
- `rust/apps/nxtools-desktop/src/quick_launch/file_dialog_search.rs`
- `rust/apps/nxtools-desktop/src/clipboard.rs`
- `rust/apps/nxtools-desktop/src/ui/mod.rs`
- `rust/apps/nxtools-desktop/src/ui/history.rs`
- `rust/apps/nxtools-desktop/src/ui/preview.rs`
- `rust/apps/nxtools-desktop/src/download_taskbar_overlay.rs`
- `browser_extension/nxtools-interceptor/popup.html`
- `browser_extension/nxtools-interceptor/popup.css`
- `browser_extension/nxtools-interceptor/popup.js`
- `browser_extension/nxtools-interceptor/content-script.js`
