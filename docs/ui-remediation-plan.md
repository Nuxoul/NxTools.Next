# NxTools.Next UI 整体整改计划

> 状态：P0/P1 主体切片完成；批次 5（CL）由用户授权的自动化管线推进（2026-09-01）
> 建立日期：2026-08-13
> 视觉目标：以 Zed 为主参考，以当前剪贴板工作区为产品内基准，保持高级、简约、极客、紧凑。
> 适用范围：桌面 GPUI、浏览器扩展 Popup、网页媒体悬浮入口与拖放目标。

## 1. 文档目的

下载、速启和剪贴板的主要功能已经基本具备，但视觉实现尚未形成统一的组件层和布局规则。
本计划用于统一三个模块及其辅助窗口的字体、颜色、控件尺寸、层级、布局、状态与动效，避免继续
按单个截图对局部控件反复打补丁。

初始结论来自源码级扫描；后续切片同时补充真实 GPUI 窗口门禁与 Release 包截图，因此以下内容分为：

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
| QL-06 | 文件对话框搜索 | 统一搜索输入、结果行、边界和阴影 | 100/125/150% DPI 结果面不越出宿主，搜索头按参考贴住宿主底边 |
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

第三个 P0 纵向切片（2026-08-13）：

- 新建下载的分类选择已成为共享 Popover/ListBox 的首个生产消费者：移除四项分页器，
  改为一个 320×240 px 有界浮层、32 px 搜索框和 188 px 真实滚动列表。DL-21 的实现与
  生产门禁均已落地；DS-12 仍需等其他动作、来源和筛选浮层迁移后再整体勾完。
- 搜索只在本地匹配分类名称、保存目录和扩展名；Up/Down 移动高亮，Enter 选择，Escape
  关闭。鼠标和键盘复用同一 `selected_category_id`，关闭后焦点回到文件名字段；完整目录
  继续由 Tooltip 呈现，搜索词和目录不进入诊断日志。
- 共享 ListBox 由调用方提供唯一 `ScrollHandle`，内容跟踪和可见滚动条复用同一句柄；
  生产窗口门禁验证 32 类产生非零滚动范围、最后一类可经搜索与键盘选择、旧分页控件消失，
  同时保留鼠标选择和生产 Cancel 关闭窗口的原生命周期。
- 自动化验证：分类过滤、真实生产分类浮层、鼠标/键盘确认和焦点回归均已在后续同日
  desktop 288 项串行门禁中通过。外部点击测试点也改为先证明位于生产 Popover 外，避免
  宽度调整后出现假失败。本切片未启动 GUI、未打包、未冒领 DPI/截图人工验收。

第四个 P0 纵向切片（2026-08-13）：

- “稍后下载”日期时间浮层成为共享 Popover 的第二个生产消费者：保留既有本地草稿、
  中文日历和延迟提交语义，迁移到 304×240 px 有界 6 px 浮层；清除/确定统一为共享
  28 px 操作，普通中文日期提升为 11–12 px，10 px 仅保留星期元数据。
- Popover 打开、月份切换、日期选择、清除和确定都在下一 application turn 更新父 View，
  避免组件回调同步重入。外部关闭继续只丢弃未确认草稿，不改变上次已提交时间。
- 新增门禁通过真实 `open_browser_download_dialog` 生产窗口点击日期、确认、外部关闭、
  清除和 Cancel，并验证浮层边界及操作高度；后续同日 desktop 288 项串行门禁已实际覆盖
  该用例。外部点击点同样从生产 bounds 派生并先证明位于 Popover 外。未启动 GUI、未
  打包、未冒领 DPI/截图人工验收。

第五个 P0 纵向切片（2026-08-13）：

- 下载任务右键菜单成为共享 Menu 的首个生产消费者：移除 `cfg(test)` 专用条目渲染和
  `min_w(0)`，改为 192–240 px 有界壳层、28 px 行、固定 14 px 图标槽、12 px 正文与
  normal/danger 语义。删除文件和从列表移除只使用危险文字/图标，不使用永久危险填充。
- 保留旧版/ABDM 验证后的选择和能力拓扑：右击未选中行先选中，已有多选继续作为批量
  目标；不可执行动作直接省略，队列边缘动作由权威顺序决定。所有 DownloadApplication
  跨实体操作统一 defer 到下一 application turn。
- 继续复用 `gpui-component` PopupMenu 的 Up/Down/Enter/Escape、外部关闭与 focus 生命周期；
  生产任务行回归验证五个终态动作同高同宽、不可用动作缺席、Escape 后焦点返回工作区，
  且旧菜单坐标不再能触发平台打开文件命令。
- 自动化验证：menu 过滤门禁 5/5、desktop 串行门禁 288/288、production desktop check、
  `clippy -D warnings` 与 `git diff --check` 通过。DS-12 仍需迁移剩余来源/筛选浮层，因而不
  提前勾为整体完成。本切片未启动 GUI、未打包、未冒领 DPI/Release 截图人工验收。

第六个 P0 纵向切片（2026-08-13）：

- DL-01 至 DL-05 的主任务表收口为连续 46 px 表面：文件名使用 12 px Body，路径/错误使用
  11 px Label；中性整行选中和左侧 2 px accent 保持不变，不恢复卡片或行内操作列。
- 原单一进度列改为 180 px 固定传输区，按已下载/总量、实时速度、剩余时间三个稳定子列
  对齐，并在下方共享 3 px 进度轨。数值复用 `DownloadApplication` 已有的有界进度快照，
  终态任务回到 durable task 值，避免过期实时采样把“已完成”显示成未满。
- 为保护紧凑窗口中的名称主列，已下载/总量只显示一次单位；缺失速度或剩余时间显示破折号，
  不改变列拓扑；长剩余时间收敛为 `1h02m`/`99h+`，高速值跨 1024 单位晋级，避免固定列
  裁切。失败文本只替换辅助行，完整错误继续由 320×144 px 有界可滚动内容视口（可见浮层
  另含共享 tooltip 的 padding/边框，整体上限 340×170 px）和显式选中后的详情 dock 提供，
  不进入诊断日志。
- 真实 720 px 生产下载工作区门禁验证名称列仍至少 180 px，表头与三项传输子列、状态列严格
  对齐，辅助行达到 16 px 可读高度，进度轨保持 3 px。DL-06 过滤、DL-07 详情 dock 和
  DL-08 多选工具栏留在后续独立切片，本次不提前勾完下载批次。

第七个 P0 纵向切片（2026-08-14）：

- DL-06 将六个独立状态胶囊收敛为一个共享 28 px segmented control：六项等宽、共用一个
  外边界和一个语义选中态，实时数量改为单元格内文本，不再嵌套独立 badge。筛选仍只改变
  `MainWindow` 本地呈现状态和稳定 ID 可见集，不发送引擎命令或改变队列顺序。
- 搜索框统一为 28 px editor-style 控制，非空时只出现一个 18 px 清除命中区；筛选/搜索结果
  摘要移到模块页头元数据。38 px 控制条因此只保留状态分段和一个紧凑排序入口，720 px
  工作区也不会挤压表格列。
- 队列、大小和添加时间继续通过紧凑排序菜单选择；名称、进度和状态继续由真实表头切换方向。
  两个入口复用同一 `DownloadSortState`，搜索、筛选和排序诊断仍只记录有界状态，不记录查询、
  URL、路径或文件名。
- 真实生产 GPUI 门禁在 720 px 工作区验证六个状态单元连续且仅有像素舍入差，点击等待筛选、
  写入并清除搜索，以及用键盘从真实排序菜单选择名称排序；表头排序门禁继续通过。desktop
  串行门禁 289/289、production desktop check、workspace `clippy -D warnings` 与
  `git diff --check` 通过。DL-07 详情 dock 和 DL-08 多选工具栏仍留在后续独立切片；未启动
  GUI、未打包、未冒领 DPI/Release 截图人工验收。

第八个 P0 纵向切片（2026-08-14）：

- DL-07 将已选任务详情收口为固定高度的底部 inspector：30 px Tab 列保持稳定，正文统一
  使用 11/12 px typography token，概览采用固定标签列与可伸缩值列，所有长内容留在同一个
  纵向滚动区；无选择时不渲染也不预留详情高度。
- DL-08 在真实多选时于筛选条和表头之间显示一条紧凑工具栏。动作完全复用右键菜单的能力
  快照与目标子集；单选/空选择时不渲染工具栏，不保留空操作带。
- 生产 720 px 下载窗口已证明详情 Tab/字段列/滚动区几何稳定；多选回归使用 Failed +
  Downloading 混合状态，验证 Resume/Pause/Cancel 的能力子集、四个队列动作和 Remove 的省略，
  并从真实 Cancel 命中区只取消可取消任务，再清除选择、回收完整 36 px 工具栏高度。
- 自动化验证：DL-07/DL-08 两条生产 GPUI 定向门禁、desktop 串行门禁 290/290、workspace
  check、workspace `clippy -D warnings`、`cargo fmt --check` 与 `git diff --check` 通过；Standards
  与 Spec 双轴复审剩余阻塞均为 0。未启动 GUI、未冒领 DPI/Release 截图人工验收；Release
  包由本切片提交后统一生成。

第九个 P0 纠偏切片（2026-08-14）：

- Release 截图暴露出共享令牌并未自动保证视觉分布：下载模块页头的三个真实按钮仍只有
  24 px，排序入口被弹性空白推到筛选条最右端；全局设置则把少量内容塞进 560 px 窄列，
  同时保留 430 px 固定高度，形成明显无效留白。
- 全局设置生产窗口改为 720×368；常规与代理 section 获得足够横向说明空间，最后一条
  策略说明只保留 12–14 px 到固定 footer 的正常内边距。校验失败在同一状态行原位替换
  策略说明，不重新撑高窗口，也不会被 footer 遮挡。
- 下载设置与计划改为带 Tooltip 的共享 28 px 图标操作，新建下载保留唯一带文字的 28 px
  主操作，三者使用 4 px 稳定组间距。排序改为共享 28 px 有边界操作，并固定在状态分段
  右侧 8 px，不再成为孤立的远端文字目标。
- 真实生产门禁先分别以 560×430 和 24 px 命中红灯，修复后验证全局设置原生尺寸、内容
  尾部空间、错误状态、三个页头操作 containment 及筛选/排序邻接；desktop 串行门禁
  290/290、workspace `clippy -D warnings` 与 `git diff --check` 通过。本切片未启动 GUI、
  未打包，也未冒领 Release 截图人工验收。

第十个 P0 主壳层切片（2026-08-14）：

- 使用 interface-design、make-interfaces-feel-better、design-system、windows-desktop-e2e 与
  screenshot 五项工作流重新检查主窗口。视觉方向固定为 Zed 式石墨工作台：连续表面、低对比
  边界、稳定密度与最少强调色；设计决策记录在 `.interface-design/system.md`，Rust 的
  `theme.rs`、`ui/metrics.rs` 和 `ui/typography.rs` 仍是唯一生产令牌来源。
- 新增共享 `ModuleHeader`，下载、速启和剪贴板统一使用 48 px 页头、14/20 标题与 11/16
  元数据；剪贴板计数保留 Mono，下载搜索、设置、计划与新建操作收进同一 4 px trailing group，
  不改变三个模块各自的信息拓扑和业务回调。
- 主侧栏继续保持 208 px 与 42 px 行，但标题/说明改为共享 Body/Label 字体角色，并为三个真实
  模块行增加稳定调试选择器。三模块切换不再出现速启 68 px、剪贴板 15/10 px 与下载 48 px
  三套页头节奏。
- 新增真实生产 `MainWindow` GPUI 回归：在 1060×500 窗口中依次点击下载、速启、剪贴板侧栏，
  断言三个页头和标题行几何一致；既有标题栏设置/拖动/关闭与 720 px 下载操作区回归继续通过。
  Windows 原生生命周期继续使用 typed PlatformHost/真实 HWND 门禁；GPUI 自绘控件不冒充完整
  UIA 控件树。
- 本切片已通过 desktop `cargo check --tests`、三模块页头门禁、3 项标题栏门禁与下载紧凑工作区
  门禁。首次默认 test profile 链接因系统提交内存不足失败，改用同一源码与依赖、仅关闭 test
  debuginfo 的低内存 profile 后精确门禁通过；尚未生成新的 Release 包，也未用旧包截图冒充改后
  效果。SH-03/SH-04 的当前生产入口已收敛，但主壳层批次仍需完成 SH-01/02/06/07/08 与完整
  DPI 截图矩阵后才能整体勾选。

第十一个 P0 主标题栏切片（2026-08-14）：

- SH-01/SH-02 收敛到一条 36 px Zed 式 chrome：正式 18 px NxTools 资产、Body 产品名、
  Label 当前模块和右侧 utility group 使用共享字体与间距令牌，不再保留 9 px 一次性 gap。
- 全局设置保持一个 28 px subtle action；它与连续的三枚 36 px Windows 控制按钮之间固定 4 px。
  最小化、最大化/还原、关闭到托盘仍走原 typed PlatformHost 命令，零 HWND 回退语义不变。
- 四种 caption 状态使用独立的 12 px 内嵌 SVG，不再手绘边框或依赖字体 `×`；资产注册门禁
  覆盖四个文件与 `currentColor`，窗口状态选择门禁锁定 Maximize/Restore 的不同 ID 与图形；
  四个 icon-only 命中区均提供稳定中文 Tooltip，真实 hover 门禁覆盖最小化入口。
- 真实生产 `MainWindow` 在 920×480 最小尺寸下验证品牌/模块文字行高、utility containment、
  拖动区不覆盖操作区、三个窗口按钮连续排列，并实际点击拖动、最小化、最大化/还原和关闭。
  全局设置真实点击与窗口所有权门禁、desktop shell architecture gate 均继续通过。
- 本切片只关闭 SH-01/SH-02 的生产入口与自动化门禁；SH-06 通知、SH-07 状态和 SH-08
  托盘视觉文案仍留在后续切片，新的 Release/DPI 截图也仍需统一打包后人工验收。

第十二个 P0 通知层切片（2026-08-14）：

- SH-06 不再把 368 px 阴影卡片贴在标题栏下方；最多三条通知迁移到窗口右下角的独立浮层，
  在 920×480 生产最小尺寸中保持 12 px client inset，并完整避开标题栏和模块主操作区。
- 单条通知收敛为 336 px、最高 112 px 的轻量 raised surface：4 px 圆角、subtle border、small
  shadow 与 3 px 单侧语义标记；进度/信息/成功/失败只通过标记和 14 px 状态图标表达，不再用
  大面积语义色或厚重卡片层级。
- 标题、正文、详情和操作分别复用 Body/Label/Meta/Mono 与共享 28 px action tier；原有最多三条、
  generation-scoped 自动退出、结构化详情和操作语义保持不变。操作/关闭后的 NotificationCenter
  更新延迟到下一 application turn，且冻结渲染时的 `(id, generation)`，避免跨实体同步重入或旧卡
  误关同 ID 的新通知。
- 新增真实生产 `MainWindow` 门禁，覆盖 progress/error/detailed-success 三种通知，验证三卡边界、
  语义标记、12/11 px 字体与 18/16 px 行高、28 px action/dismiss，并实际点击操作与关闭进入退出态。首次低内存
  精确 Cargo 门禁在加载 Rust 标准库元数据时受 Windows pagefile 1455 阻断；源码/格式门禁继续
  完成，待系统提交空间恢复后重跑，不以本轮源码结果冒领 Release/DPI 截图验收。

第十三个 P0 工作状态切片（2026-08-14）：

- SH-07 新增共享 `EmptyState`、`ProgressState` 与工作区 `InlineError` 呈现：三者保留所属
  内容边界，统一 18 px 图标槽、Body/Label 两级文字、8 px 内容间距与 12 px inset；不新增
  独立卡片、阴影或循环加载动画。空态为中性，加载仅使用 accent 图标，错误仅通过 danger
  图标/标题和窄语义边表达。
- 下载任务空列表、搜索/过滤空结果、分段与日志详情的空/加载/错误，速启结果与 Windows
  操作的空/加载，以及剪贴板历史空/搜索无结果均迁移到同一组件；设置字段错误也复用已有
  共享 `InlineError`，不再保留下载私有的 9.5 px 错误文字。
- 真实 GPUI 门禁已写入生产下载 `MainWindow`、速启弹窗和剪贴板面板，并补充共享组件三态
  几何回归；静态 `ui-state-lifecycle.ps1`、原下载 UI 生命周期门禁、rustfmt 与 diff check 已通过。
  当前 Windows FreeVirtual 约 0.53 GiB，未重复启动此前会在 Rust 元数据 mmap 阶段触发 1455
  的 Cargo 门禁；精确 GPUI/Cargo 回归与 Release/DPI 截图继续保留为统一打包前验收，不冒领
  本轮环境未提供的运行证据。
- 本切片结束时 SH-08 托盘仍保持下一切片；在完成原生图标/文案核对与生命周期门禁前，
  批次 2 不整体勾选。

第十四个 P0 原生托盘切片（2026-08-14）：

- SH-08 保持 Windows 原生 `NotifyIcon + ContextMenuStrip`，没有引入 GPUI 自绘或 owner-drawn
  菜单。Tooltip 固定为 `NxTools`，菜单固定为 `打开 NxTools`、一条原生分隔线和 `退出`；左键、
  Open、Exit 继续复用既有 correlated typed event 与同一主窗口生命周期。
- 桌面构建从仓库内同一组 16/32/48/128 px NxTools 品牌 PNG 生成多尺寸 ICO 并嵌入
  `NxTools.exe`，使 EXE、任务栏、GPUI 标题栏和托盘不再出现两套品牌。PlatformHost 只提取
  EXE 的关联图标；缺失正式图标时返回 typed failure，不再静默退回 Windows 通用应用图标。
- C# 真实托盘门禁补充精确 Tooltip、菜单顺序、分隔线、关联图标存在，以及真实无图标 PE
  不得退回通用图标的 fail-safe；
  Release 打包新增像素级产品图标门禁，防止后续构建丢失 PE 图标资源。ICO 编码器另有独立
  目录/偏移/拒绝非法 PNG 单测。
- 静态 PowerShell AST、rustfmt 与 diff 门禁已写入；当前 Windows FreeVirtual 从约 0.15 GiB
  继续降至约 0.07 GiB，小型 `rustc --test` 尚未启动即由 PowerShell 发生 stack overflow。
  FreeVirtual 短暂恢复到约 0.33 GiB 后，以单进程且禁用 shared compilation 重试 PlatformHost
  Release 门禁，Roslyn `csc.dll` 仍以同一 `-1073741571` stack overflow 退出且没有源码诊断。
  因此本切片不冒领 ICO 单测、PlatformHost 编译测试、Release 产品图标门禁或 DPI 截图通过；
  它们保留到系统提交空间恢复后的统一打包验收。

第十五个 P0 下载设置壳层与控件切片（2026-08-20）：

- DL-09 将下载设置接入共享 `SettingsShell`：保留原五页左导航和单一滚动内容列，页脚统一为
  固定 48 px。页面切换仍只改变本地页面与滚动位置，设置草稿、原生操作、Apply/Cancel 和
  taskbar edit cleanup 生命周期不变。
- DL-10 将普通中文设置文案收敛到 Body 12/18 与 Label 11/16，输入统一为 32 px，目录与
  FFmpeg picker 改为带 Tooltip 的 28 px 方形操作；分类、FFmpeg、任务栏预览和页脚操作统一
  复用共享 28 px action tier，不再混用 `xsmall`、`compact` 与调用点自绘高度。
- 真实生产下载设置窗口门禁覆盖 984×650 外壳、共享 36 px titlebar、五页切换时固定 footer、
  单一滚动所有者、同一内容宽度、两列传输网格、诊断全宽行，以及修改本地草稿后切遍五页
  仍保持原值。12/18 与 11/16 基线、共享 32 px 输入、28 px 普通 action、22 px toggle 和目录/
  FFmpeg picker 的真实悬停 Tooltip 均有生产窗口断言；分类排序、删除、picker、FFmpeg 和
  Apply/Cancel 的既有点击门禁保持不变。
- Windows FreeVirtual 恢复到约 3.9 GiB 后，desktop `cargo check --tests` 首次抵达编译并暴露
  前三批因 pagefile 阻断而未发现的 SH-06/SH-07 类型门禁缺口；已只修复 GPUI 类型/测试坐标
  seam，并实际通过通知、共享工作状态和速启状态生产门禁。DL-09/DL-10 的较早生产窗口门禁、
  完整 picker/跨页/Apply/Cancel 与 FFmpeg 取消路径曾各取得 `1 passed`。最终又收敛共享
  segmented control、field shell、titlebar、单滚动层、跨五页草稿和 Tooltip hover 门禁后，
  desktop `cargo check --tests` 与 diff check 继续通过；此时 FreeVirtual 仅约 0.78 GiB，未再次
  启动会重链 GPUI 测试二进制的 exact run。更新后 exact 门禁、Release 100/125/150% DPI 截图
  和打包继续待提交空间恢复；本切片未启动 GUI，也不冒领最终运行或人工视觉验收。

第十六个 P0 下载分类主从编辑切片（2026-08-20）：

- DL-11 将分类入口收敛为一个连续的 228 px master list：42 px 行使用 Body 分类名和 Label
  解析后保存目录，选中态为中性底色加不改变布局的 2 px 左边，禁用分类仍可读；长目录由
  完整 Tooltip 补充。列表拥有单一滚动层，新增入口固定在同一边界底部。
- DL-12 将选中分类编辑器收敛到共享 `labeled_field + field_shell`：名称/保存目录与扩展名/
  URL 各组成一行等宽两列，Label 基线对齐，全部输入保持 32 px。切换分类只替换当前草稿
  entity，既有启停、排序、删除、新增和 Apply/Cancel 持久化边界不变。
- 真实生产下载设置窗口门禁已扩展为：断言 42 px master 行、2 px 选中边、点击后选中边转移、
  两组等宽字段、四个 32 px 输入，以及原排序/删除/启停/保存回归。desktop
  `cargo check -p nxtools-desktop --tests` 已在规定的单 job/禁增量/高 codegen-unit 配置下通过，
  rustfmt、diff check 和 `download-ui-lifecycle.ps1` 通过。复审后新增默认目录/分类名称/分类目录
  的父视图订阅，确保编辑时 master 名称、解析路径和完整 Tooltip 实时更新；该版本 desktop
  `cargo check -p nxtools-desktop --tests` 通过。复审随后把 picker 结果与测试输入更新移到父视图
  租约之外，关闭订阅写回导致的 GPUI 重入风险；最终增量的 rustfmt、diff check 与静态生命周期
  门禁通过，但 Cargo check 在 FreeVirtual 约 0.58 GiB 时由工具宿主直接关闭输出且无存活进程，
  因而该最后一次增量不冒领 compiled gate。定向 Clippy 先被 Rust 1.96 在未改动
  `build.rs` 中新增的 lint 阻断；仅放行该既有 lint 后继续编译依赖，Windows FreeVirtual 又降至
  约 0.74 GiB，rustc 因 501,760-byte allocation failed 退出且没有源码诊断。FreeVirtual 短暂
  恢复后启动一次 exact 生产 GPUI 门禁，但在进入测试逻辑前因 pagefile 1455 无法 mmap Rust
  `libtest` 元数据；未重复链接。该生产窗口门禁、完整 Clippy
  与 Release 100/125/150% DPI 截图继续保留为提交空间恢复后的打包前验收；本切片未启动 GUI，
  也不冒领未取得的运行证据。

第十七个 P0 下载分类动作切片（2026-08-20）：

- DL-13 将启用状态从独立输入卡片移入选中分类的 32 px 标题行，标题、状态说明与共享 toggle
  形成同一编辑器语义边界；禁用只降低说明强调，不把整个分类编辑器处理成不可读的禁用卡片。
- 上移/下移收敛为一个 4 px 间距的中性排序工具组，首尾边界继续使用真实 disabled 状态；删除
  与排序组保持至少 12 px 的空间分离，静止时使用中性 raised surface，只在 hover/press 使用
  `danger_soft`，不再以永久红色抢占主操作层级。所有动作仍只修改本地分类草稿，Apply 才持久化。
- 真实生产下载设置窗口门禁新增标题行与 toggle containment、排序组/工具栏 containment、三项
  28 px 命中区、删除空间分离和 hover 前后几何稳定断言；实际点击首项禁用上移证明无副作用，
  且其回调计数保持不变，并继续点击下移、上移和删除验证草稿顺序与数量变化。门禁在 Apply
  前重载配置证明动作没有提前落盘，点击真实 Apply 后再验证启用、顺序与删除结果精确持久化；
  共享危险动作调色板另锁定中性静止态和 `danger_soft` hover/press。desktop `cargo check -p
  nxtools-desktop --tests`、rustfmt、diff check 与 `download-ui-lifecycle.ps1` 已通过。
- 当前 Windows FreeVirtual 约 2.06 GiB，未重复启动此前会在 GPUI 链接阶段触发 pagefile 1455
  的 exact 测试；危险 hover 的最终色彩、Release 100/125/150% DPI 截图和打包仍留待统一人工
  验收。本切片未启动 GUI，也不冒领未取得的运行或截图证据。
- DL-14 请求上下文是下一切片；在 Header、Cookie、证书表单层级和长 Header 不挤压 footer
  的生产门禁完成前不提前勾选。

第十八个 P0 下载请求上下文切片（2026-08-20）：

- DL-14 将请求页收敛为两个连续 form section：`请求上下文` 以 Header 为主编辑区，Cookie 为
  同宽次级字段；`证书校验` 独立保留严格校验默认值与显式例外 toggle。未改变 Header/Cookie
  合并、证书策略、设置草稿或 Apply/Cancel 的业务边界。
- 共享 Field 组件新增同一标签/说明骨架和可指定高度的 multiline shell。Header 使用固定
  120 px、全宽、内部滚动的 inset editor，Cookie 使用同一 x/宽度的 32 px field；两者继续
  使用 Body/Label、4 px 圆角、subtle border 与 canvas input surface，没有新卡片或危险色。
- 真实生产下载设置窗口门禁扩展为：请求/证书两节共享内容宽度；Header/Cookie 标签与字段
  对齐；Header 120 px、Cookie 32 px；写入 48 行有界 Header 后 editor bounds 与固定 footer
  完全不变，完整草稿仍可读取，证书 section 始终位于 footer 之上且不移动；同一生产窗口
  真实点击证书 toggle、遍历五页后点击 Apply，并重载配置验证 Header、Cookie 与证书策略。
- desktop `cargo check -p nxtools-desktop --tests`、rustfmt、diff check 与
  `download-ui-lifecycle.ps1` 已通过。当前 FreeVirtual 约 2.21 GiB，未启动会重新链接大型
  GPUI test binary 的 exact 门禁；该生产窗口用例、Release 100/125/150% DPI 截图和人工视觉
  验收仍保留到统一打包前。本切片未启动 GUI，也不冒领未取得的运行或截图证据。
- DL-15 传输设置是下一切片；连接、分段、限速两列 SettingRow 的对齐与父宽门禁完成前不
  提前勾选。

第十九个 P0 下载传输设置切片（2026-08-20）：

- DL-15 将六项传输参数收敛为同一 `传输` section 内的三条双列 SettingRow：连接上限/同时
  下载、动态分段/分段上限、最小长尾/全局限速。矩阵只细分现有内容宽度；所有单元格共享
  一个外框、一个标题行和 1 px 中轴，不增加独立背景、圆角、阴影或子级最小宽度。
- 参数值、toggle、校验范围和本地设置草稿保持原实体与回调；切页仍只改变呈现，Apply/Cancel
  仍是唯一持久化边界。本轮没有新增下载命令、协议字段、诊断事件或高频日志。
- 真实生产 `984 x 650` 设置窗口门禁扩展为：三行左右单元格共享 y/高度，左右等宽且严格位于
  父 section 内；每行只有连续的 1 px 中轴；三行共享同一 x/宽度；连接/动态分段/最小长尾与
  同时下载/分段上限/全局限速分别落在两条稳定尾部控件轨道。旧自由换行实现缺少这些行级
  selector，因此该门禁可对布局回归变红。
- rustfmt、`git diff --check` 与 `download-ui-lifecycle.ps1` 已通过。desktop Cargo check 在
  FreeVirtual 约 0.96 GiB 时于 desktop test crate 检查阶段发生 24 KiB allocation failure，
  rustc 以 `0xc0000409` 退出且没有源码诊断；确认没有 Cargo/rustc 残留后未重复启动构建。
  desktop check、精确生产窗口门禁、Release 100/125/150% DPI 截图与人工视觉验收继续保留到
  commit/pagefile 空间恢复后；本切片未启动 GUI，也不冒领这些证据。
- DL-16 FFmpeg 是下一切片；路径、自动安装和公开来源整理为全宽顺序行并完成真实交互门禁前
  不提前勾选。

第二十个 P0 下载 FFmpeg 设置流切片（2026-08-20）：

- DL-16 将媒体工具收敛为同一 section 内的三条全宽顺序行：`FFmpeg 可执行文件` 先展示候选
  路径和 picker，`自动安装` 保留唯一 filled action 与同轨取消，`公开下载来源` 最后提供两个
  outline fallback。短按钮不再被推到远离说明文字的最右侧，也不再形成大片无意义空洞。
- 路径控制删除 350 px 子级最小宽，三条控制轨道均使用 `w_full + min_w_0` 服从设置内容列；
  每行沿用 Body/Label、72 px 密度、32 px 输入和 28 px action。没有新增卡片背景、阴影、颜色
  或本地尺寸方言。
- 原路径输入、picker、自动安装、取消、Gyan.dev/BtbN 打开动作和所有 application-owned 状态
  保持不变；跨实体回调继续 defer。成功托管安装仍使用既有即时配置事务，手动路径仍由设置
  Apply/Cancel 管理，安装状态只替换说明或显示同轨 Cancel。
- 真实生产 `984 x 650` 设置窗口门禁要求三行同 x/同宽、上下连续且不高于 72 px；路径、安装
  与来源拥有同一全宽左控制轨道；主安装按钮和第一个 fallback 都从该轨道起点开始。旧的两条
  尾部控件行缺少顺序行/轨道 selector 且按钮位于右侧，因此门禁可对原布局和回归变红。既有
  生产 picker Tooltip、自动下载/取消、两个公开来源和 Apply/Cancel 点击门禁继续保留；同一
  生产窗口另注入 Installing 状态，确认 quiet Cancel 出现后，三行及固定 footer 的 bounds 完全
  不变，且 Cancel 仍位于自动安装行内。
- rustfmt、`git diff --check` 与 `download-ui-lifecycle.ps1` 已通过。当前 Windows FreeVirtual
  约 1.1 GiB，未重复启动上一切片已因 commit 枯竭失败的 desktop Cargo 检查或大型 GPUI
  test link；desktop check、精确生产窗口门禁、Release 100/125/150% DPI 截图与人工视觉验收
  继续保留到提交空间恢复后。本切片未启动 GUI，也不冒领这些证据。
- DL-17 窗口/进度设置实现已落地、自动验收待提交空间恢复：窗口页不再使用带 `350 px`
  子项最小宽的自由换行网格，改为
  `任务窗口` 两行矩阵和 `任务栏进度` 七行矩阵。两节均只有一个外框与安静的 `1 px`
  中缝；总开关/预览、锚点/模式、X/Y、宽/高以及信息开关按语义配对，Toggle、数值与
  分段模式落在稳定的左右尾轨。生产 `984×650` 窗口门禁覆盖九行等宽/同行/中缝/父节
  containment 及四条控件尾轨；既有 Apply/Cancel 和任务栏编辑生命周期不变。
- rustfmt、`git diff --check` 与 `download-ui-lifecycle.ps1` 已通过。当前 Windows FreeVirtual
  约 0.66 GiB，未重复启动此前已因 commit 枯竭失败的 desktop Cargo/GPUI 链接；desktop
  check、精确生产窗口门禁、Release 100/125/150% DPI 截图与人工视觉验收仍待提交空间恢复。
  本切片未启动 GUI，也未打包。
- DL-17 自动门禁已于 2026-08-20 在恢复提交空间后通过：
  `cargo check -p nxtools-desktop --tests`、精确生产 GPUI 设置窗口测试
  `settings_sections_share_one_content_width_and_diagnostics_are_not_half_width`、
  `download-ui-lifecycle.ps1` 与 `git diff --check` 均为绿色。首次执行同时暴露并修正了门禁自身的
  三处问题：裸 Button ID 不是稳定调试边界、等宽公式漏算两个外框和一个中缝、持久化场景缺少
  初始配置夹具；修正后的测试仍从生产 `DownloadApplication -> DownloadSettingsView` 边界验证
  同一布局、草稿和 Apply 生命周期。DL-17 因此完成；Release 100/125/150% DPI 截图与人工视觉
  验收仍属于最终发布门禁，不冒充本切片证据。
- 下一切片为 DL-18 诊断页；三条路径的复制/打开操作、完整路径 Tooltip、有界操作诊断与全宽
  裁切门禁完成前不提前勾选。
- DL-18 诊断页实现已落地：配置文件、日志目录和下载数据库使用三条连续全宽 `42 px` 行；路径
  文本占据剩余主列，以 Label/Consolas 呈现并在截断时通过有界 Tooltip 暴露完整值。每行右侧
  紧邻一个 Copy 与一个 Open/Reveal 动作，均复用共享 `28 px` 方形图标层和中文 Tooltip，未
  增加卡片、阴影、颜色或局部控件尺寸。
- 两类动作继续经 deferred GPUI callback 进入 application owner。Copy 使用现有 copy-only
  `PasteText`，日志目录使用 `OpenShellTarget::Open`，配置/数据库文件使用
  `SelectInExplorer`；C# 仍是唯一 Windows Clipboard/Shell adapter。每次操作在 15 秒内以
  started -> completed/failed 收口，超时释放 desktop pending identity，迟到结果被忽略；已经被
  PlatformHost 接受的 Windows 命令不可撤回，超时提示因此明确说明操作仍可能完成。日志只含
  固定 path/action/failure kind、elapsed 与 native code，不含路径或剪贴板文本。
- 红—绿生产门禁先在缺少配置路径值节点处失败，随后从真实
  `DownloadApplication -> DownloadSettingsView -> PlatformBridge` 验证三行全宽、长路径与
  动作 Tooltip、六个 `28 x 28` hit target、三条 copy-only 命令、目录打开/文件 reveal、成功/
  失败反馈、15 秒超时和迟到结果抑制。当前 `cargo check -p nxtools-desktop --tests`、该精确
  GPUI 测试、`download-ui-lifecycle.ps1`、rustfmt 与 `git diff --check` 均通过。
- 严格 `cargo clippy -p nxtools-desktop --tests -- -D warnings` 在未修改的
  `rust/apps/nxtools-desktop/build.rs:59` 被既有 `needless_borrows_for_generic_args` 阻断；仅放行
  这一条基线 lint、其余 warnings 继续 deny 后全量 Clippy 通过。该基线未混入 DL-18 修复。
- 一次完整 `cargo test -p nxtools-desktop` 运行得到 295/300，DL-18 精确门禁在整包中仍通过；
  其余五项在隔离复跑中稳定失败，分别属于既有分类裸 Button selector、网络设置尾距、Quick
  Launch action mode、剪贴板空态宽度和下载详情 progress selector，其中四个文件本切片未
  修改。该既有基线已记录到 `.learnings/ERRORS.md`，不把整包冒充为绿色，也不在 DL-18 中
  扩张修复范围。Release 100/125/150% DPI 截图和人工视觉验收仍保留为最终发布门禁。
- 双轴复审修正了两处实质问题：可见标签从“数据”改为规范中的“数据库”；PlatformHost 的四类
  paste 操作从命令 ID 改为优先继承 envelope correlation，使 copy-only 路径从 Rust started 到
  C# `clipboard.copy.completed` 使用同一诊断身份。C# 聚焦 correlation 测试通过；完整
  PlatformHost.Tests 已打印通过所有直接相关的协议、Shell、Clipboard、Paste 和诊断测试，但
  在后段集成用例超过四分钟未终止，已只清理本轮测试子进程并记录为非绿色全套结果。

第二十一个 P0 新建下载表单网格切片（2026-08-22）：

- DL-19 将确认态的四个核心字段收敛到共享 Field 语法：下载地址保持全宽主行，保存目录保持
  全宽路径行并内含 28 px picker，文件名与分类组成等宽双列。四项统一使用 Label 11/16 与
  32 px 控件层级、4 px 控件圆角和共享边界，不再保留私有 9.5 px 标签、30 px 输入壳或分类
  单独手写的标签节奏。
- 两阶段探测、浏览器预填确认高度、目录 picker 请求、分类 Popover 搜索/滚动/键盘选择、关闭后
  焦点返回、QueueOnly/StartNow 和固定 footer 均保持原 application/view owner 与 deferred
  GPUI 生命周期；本切片没有改变下载请求、协议、持久化或诊断载荷。
- 红—绿生产窗口门禁先在旧实现缺少共享 URL 标签边界处失败，修复后从真实
  `DownloadApplication -> NewDownloadView` 断言 URL 与目录行的全宽边界、四项 Label 行高、四个
  32 px 真实控件、28 px picker，以及文件名/分类可见标签和控件同基线；同一用例继续通过 32
  分类 Popover、键盘选中、焦点返回与 Cancel 原生关闭。`new_download` 4 项、desktop
  `cargo check -p nxtools-desktop
  --tests`、`download-ui-lifecycle.ps1`、rustfmt 与 `git diff --check` 均通过。
- DL-20 探测摘要是下一切片；在右侧窄 inspector/字段尾部状态、探测前后有界几何和无独立大卡片
  的生产门禁完成前不提前勾选。

第二十二个 P0 探测摘要切片（2026-08-22）：

- DL-20 将未确认态提示收敛为 URL 下方的共享 Label 字段尾部状态；确认态摘要收敛为右侧固定
  `120 x 112 px` inspector，以左侧 hairline 与目录/文件名字段分隔，不增加背景、圆角或独立
  全宽卡片。状态、大小、能力分别使用 Label、Body、Meta 字体层级，长 MIME/状态受 inspector
  裁切，不再依赖私有 9/9.5/11 px 字号。
- 探测、浏览器交接、文件名回填、目录与分类编辑、普通/HLS/DASH 提交流程均保持原 owner；
  HLS/DASH 仍由 `new_download_window_size_for_probe` 按真实选择行扩展，本切片不改协议、引擎、
  请求或诊断载荷。
- 红—绿生产门禁先在真实 560 x 176 未确认窗口缺少 guidance 边界处失败，修复后证明确认前不
  预留 inspector；注入普通探测结果并走同一尺寸 helper 后，窗口仅增至 560 x 278、宽度不变，
  inspector 小于表单宽度四分之一并跨越目标字段行，状态/大小/能力三行全部位于其内且不侵入
  固定 footer。精确门禁、既有 `new_download` 4 项、真实 DL-19 分类/Cancel 门禁、desktop
  `cargo check --tests`、仅放行既有 build-script lint 的严格 Clippy、生命周期架构门禁、rustfmt
  与 `git diff --check` 均通过。desktop 全套为 298/302，DL-20 门禁在整包中通过；其余分类按钮
  selector、网络尾距、剪贴板空态宽度和下载详情 selector 四项为已记录的切片外既有基线。
- DL-21 分类弹层已由 2026-08-13 的第三个 P0 纵向切片完成；下一项是 DL-22 分类信息，在名称/
  目录双行和完整目录 Tooltip 获得稳定生产边界前不提前勾选。

第二十三个 P0 分类信息切片（2026-08-22）：

- DL-22 保留已完成 DL-21 的 `320 x 240 px` Popover、搜索和单一可滚动 ListBox，只整改每个
  分类项的信息层级：名称使用共享 Body 12/18，目录使用 Label 11/16；两行各自占满可用宽度并
  仅在列表内截断。悬停行使用稳定 selector 的 wrapped monospace Tooltip 显示完整目录，不再
  依赖无法从生产门禁观察的默认 Tooltip。
- 自动匹配、已选与键盘高亮、目录/扩展名搜索、同一 ScrollHandle、鼠标/Enter 选择、Escape/
  overlay 关闭及文件名焦点回归均保持原 view owner；本切片不改变分类规则、目录应用、下载请求、
  诊断或 Popover 生命周期。
- 红—绿门禁先在真实 32 分类窗口缺少名称行边界处失败；修复后断言名称 18 px、目录 16 px 且
  在同一 38 px 选项内按两行排列，长目录悬停 600 ms 后出现完整路径 Tooltip。同一用例继续通过
  真实滚动范围、无分页器、最后一类键盘搜索/选择、焦点回归和 Cancel 原生关闭。desktop
  `cargo check --tests`、仅放行既有 build-script lint 的严格 Clippy、生命周期架构门禁、rustfmt
  与 `git diff --check` 均通过；全套保持 298/302，四项失败与 DL-20 已记录基线完全相同。
- DL-23 HLS/DASH 轨道是下一切片；在标准 radio list、主副行信息层级和自动/具体轨道视觉区分的
  生产边界完成前不提前勾选。

第二十四个 P0 HLS/DASH 轨道切片（2026-08-22）：

- DL-23 新增共享 `radio_list_row`，HLS 清晰度与 DASH 视频/音频轨统一为 42 px 连续列表行；普通行
  不再绘制独立圆角卡片和整行强调边框，选中态只使用中性 `element_selected` 表面、14 px radio
  外圈与 6 px accent 圆点，hover 使用共享语义色。主行使用 Body 12/18，副行使用 Label 11/16，
  原 8–9.5 px 私有字号全部从这两个媒体选择面移除。
- HLS 主行只显示实际分辨率；无分辨率时根据有界 codec 明确显示“视频流”“音频流”或“未标注
  分辨率”，不再把缺失元数据表现为“自动”。编码、平均/声明码率与帧率统一进入副行。DASH 主行
  保留视频清晰度或音轨语言，编码与码率统一进入副行；视频单选、音频再次点击取消、typed media
  构造、QueueOnly/StartNow、FFmpeg 封装和协议/引擎 owner 均未改变。
- HLS/DASH scroller 仍最多显示三行，但按共享 42 px 行高连续计算；一视频/一音频 DASH 窗口为
  560 x 333 px，五项 HLS 窗口为 560 x 402 px，不保留最大高度空白。红—绿 GPUI 门禁先在旧实现
  缺少标准 radio selector 处失败，修复后覆盖选中/未选中圆点、主副行边界、点击切换与 typed
  media 提交；无分辨率 HLS、媒体窗口高度和既有 4 条 `new_download` 回归均通过。
- `cargo check -p nxtools-desktop --tests`、仅放行既有 build-script lint 的严格 Clippy、
  `download-ui-lifecycle.ps1`、rustfmt 与 `git diff --check` 均通过。desktop 全套保持 298/302；分类
  按钮 selector、网络尾距、剪贴板空态宽度和下载详情 progress selector 四项失败与 DL-20/DL-22
  已记录基线完全相同。DL-24 Footer 是下一切片，在四项 28 px 操作层级和唯一主操作获得真实窗口
  门禁前不提前完成。

第二十五个 P0 新建下载 Footer 切片（2026-08-22）：

- DL-24 将确认态的稍后下载、添加、下载与取消全部迁移到共享 `action_button`；刷新迁移到共享
  `icon_action_button`。五项都由 `CONTROL_HEIGHT_ACTION` 固定为 28 px，圆角和水平内距不再由
  `NewDownloadView::render` 分别手写，Footer 继续保持 44 px 固定高度和原有左右分组。
- “下载”是确认态唯一使用共享 `primary_button_variant` 的 accent-filled 主操作；“稍后下载”和
  “添加”保持 outline secondary，“取消”保持 ghost secondary，刷新保持 28 x 28 px outline utility。
  未确认态“继续”仍是该阶段唯一主操作。QueueOnly/StartNow、计划 Popover 草稿、重新探测、原生
  Cancel 关闭和延迟任务窗口调度均保留原 view/application owner，本切片不改请求、协议或引擎。
- 红—绿真实 GPUI 门禁先在旧实现缺少 footer 角色 selector 处失败；修复后精确断言四项可见操作
  均为 28 px、刷新为 28 x 28 px，并要求每项 primary/secondary 角色边界与实际 hit target 完全
  重合；动作枚举同时拥有 Button ID、图标、角色 selector 和 emphasis，纯映射门禁要求 StartNow 是
  唯一 Primary，避免角色标签与实际 variant 分离。同一真实窗口门禁继续验证 schedule Popover 不
  改变窗口/footer 几何、确认/外部关闭的草稿语义；既有 4 条 `new_download`、HLS QueueOnly 与
  DASH QueueOnly 提交回归均通过。
- `cargo check -p nxtools-desktop --tests`、仅放行既有 build-script lint 的严格 Clippy、
  `download-ui-lifecycle.ps1`、rustfmt 与 `git diff --check` 均通过。desktop 全套为 299/303；分类
  按钮 selector、网络尾距、剪贴板空态宽度和下载详情 progress selector 四项失败与前序基线完全
  相同。DL-25 任务窗是下一切片，在 Progress/Failed/Completed 共享骨架及原位状态转换获得真实
  窗口门禁前不提前完成。

第二十六个 P0 任务窗共享骨架切片（2026-08-22）：

- DL-25 将 Progress、Failed、Completed 三态迁入同一个 `download_task_window_shell`。任务窗现在
  固定复用 `shell -> 34 px titlebar -> flex body -> 44 px footer` 骨架，各状态只提供正文和动作；
  Progress 原先独立的 42 px footer 已收敛到共享 44 px 轨道，三态不再分别手写根背景、边界和尺寸。
- 生命周期仍由 `DownloadWindowLifecycle` 和每任务一个 `task_windows` keyed entry 管理。状态转换继续
  通过 `DownloadTaskWindowView::replace_snapshot` 更新同一 view；既有 resize 换算提取为纯函数，
  Progress→Failed/Completed 在 150% DPI 下的物理中心保持门禁已覆盖，原生窗口 handle 和 generation
  不发生替换。Completed 的打开/目录/拖拽层级属于 DL-26，本切片未提前调整。
- 红—绿真实 GPUI 门禁先在旧实现缺少共享 shell selector 处失败；修复后在一个生产任务窗内依次
  渲染 Progress、Failed、Completed，断言三态共享相同 titlebar/body/footer 几何、注册表始终只有
  一个窗口且 generation 不变；失败态转换穿过生产 lifecycle/application 链，并在有界平台传输端
  精确收到同一 handle 的中心保持 `SetWindowBounds`。8 条生命周期状态机测试、
  `cargo check -p nxtools-desktop --tests`、
  仅放行既有 build-script lint 的严格 Clippy、`download-ui-lifecycle.ps1`、rustfmt 与
  `git diff --check` 均通过。desktop 全套为 301/305；分类按钮 selector、网络尾距、剪贴板空态宽度
  和下载详情 progress selector 四项失败与前序基线完全相同。DL-26 完成窗是下一切片，在主/次动作
  层级和拖放行为获得生产窗口门禁前不提前完成。

第二十七个 P0 完成窗动作层级切片（2026-08-22）：

- DL-26 新增 `CompletedTaskWindowAction`，统一拥有 Drag、Reveal、Open 三项的 Button ID、图标、
  label、role selector 和 emphasis。三项全部复用共享 28 px action primitive：“打开文件”是唯一
  accent-filled Primary；“打开所在目录”是 labelled outline secondary；拖放是带 Tooltip 的
  28 x 28 px outline icon secondary。原私有 labelled drag `div` 和 Reveal ghost 层级已移除。
- Rust 仍选择已完成任务的最终路径，`OpenShellTarget::{Open,SelectInExplorer}` 与
  `BeginFileDrag` 仍穿过原 PlatformHost typed boundary。拖放继续直接绑定真实 Button 的
  `on_mouse_down(MouseButton::Left)`，使用建窗时解析并注入 View 的 source HWND，同步进入现有 C# OLE 协调器；没有改成
  click、`cx.defer` 或 GPUI 内部 drag loop。完成动作模型不包含 Delete，删除仍留在独立确认路径。
- 红—绿生产 GPUI 门禁先在旧实现缺少 drag secondary role 处失败；修复后从三项真实 role hit
  target 断言 28 px 几何、唯一主操作和顺序，悬停 600 ms 后要求“拖拽文件”Tooltip 真实出现，
  并实际触发 SelectInExplorer、Open、BeginFileDrag。三条命令都携带同一最终路径，拖放命令另断言
  非零 source HWND（测试值 42）。纯映射门禁要求 Open 是唯一 Primary；升级后的生命周期架构门禁
  同时锁定共享 action primitive、Tooltip、无 Delete 和同步 pointer-down/source-HWND 链。8 条任务窗生命周期
  测试、`cargo check -p nxtools-desktop --tests`、仅放行既有 build-script lint 的严格 Clippy、
  rustfmt 与 `git diff --check` 均通过。desktop 全套为 302/306；分类按钮 selector、网络尾距、
  剪贴板空态宽度和下载详情 progress selector 四项失败与前序基线完全相同。DL-27 计划/删除/关机
  DialogShell 是下一切片，在三类辅助窗口共享安全动作语义与原生关闭生命周期门禁前不提前完成。

第二十八个 P0 辅助 DialogShell 切片（2026-08-22）：

- DL-27 新增共享 `DialogShell`，以 `DialogShellIds` 统一 36 px `ClientTitlebar`、有界 body、
  48 px `DialogFooter`、UI 字体与连续 canvas/surface 边界。每日计划、删除本地文件、自动关机三个
  生产窗口不再各自维护 34 px 标题栏、44/46/48 px footer 和私有按钮高度，全部动作回到共享
  28 px action tier。
- 每日计划的“应用”保持唯一 accent Primary，“取消”为 outlined secondary；“删除文件”和
  “立即关机”统一使用 neutral-at-rest、danger-on-interaction 变体，取消仍是 outlined secondary。
  关机执行中继续禁用两项动作，既有 correlation ID、typed `ShutdownSystem` 平台确认与成功后关闭
  契约不变。
- 计划与删除窗口新增显式 owned-close 路径：Cancel/标题栏关闭通过下一 application turn 清理 keyed
  registration 并关闭窗口；原生关闭只同步判定是否允许，注册清理由下一 application turn 按
  window/correlation identity 防陈旧执行。计划草稿在取消或原生关闭后丢弃，Apply 仅在校验及
  持久化成功后关闭；删除 Cancel/原生关闭不移除任务，确认删除仅在 engine removal 成功后关闭。
  三个 View 的跨实体动作回调均由 `cx.defer` 隔离。
- 红—绿生产 GPUI 门禁覆盖三套 shell role、36/48 px 几何、28 px 动作与顺序；计划窗实际验证
  Cancel、修改草稿后 native close、重开及 Apply，删除窗验证 Cancel、native close、重开及成功
  Confirm，关机窗继续验证 Cancel、native close、立即关机 typed boundary 和平台成功后关闭。
  `cargo check -p nxtools-desktop --tests`、仅放行既有 build-script lint 的严格 Clippy、rustfmt、
  diff check 与升级后的生命周期架构门禁均通过。desktop 全套为 303/307；分类按钮 selector、
  网络尾距、剪贴板空态宽度和下载详情 progress selector 四项失败与前序基线完全相同。DL-28
  任务栏覆盖条是下一切片，在正常/编辑态信息位置和数字清晰度获得生产窗口门禁前不提前完成。

第二十九个 P0 任务栏覆盖条切片（2026-08-22）：

- DL-28 将可选速度、ETA、任务/连接数和传输字节从私有 9/10 px 比例行迁移到共享 Label 11/16；
  数字 value 使用 `UI_MONO_FONT_FAMILY` 形成稳定节奏，中文标签继续使用 UI 字体和 Label regular。
  进度 lane 独占 accent，普通态仍为透明、无卡片和无 pill 的任务栏内 telemetry rail。
- 正常态与编辑态现在共用同一 content frame、metric row、progress stack 和 lane 容量。编辑态不再
  扣除 10 px lane 宽度、不再预留 2 px 容量或添加 5/1 px 内边距，也不再切换 raised 背景；它只在
  内容之上增加 absolute accent border 和既有右下 resize handle，因此切换时信息坐标完全不变。
- Rust 聚合、GPUI 渲染、C# taskbar-DPI placement，以及 no-activate/topmost/click-through 普通态和
  typed move/resize 编辑态边界保持不变。顺带将 move/resize 结果登记和 native close registration
  清理延后到下一 application turn；原生关闭同时校验当前 window identity 与 edit correlation，防止
  陈旧窗口或旧编辑手势的回调取消新窗口编辑。
- 红—绿门禁先命中旧编辑态把 3 条 lane 压到 2 条以及缺失稳定 content role；修复后 320 x 32
  生产 GPUI 窗口精确断言 normal/edit 的 content、metrics、progress bounds 相等、metric row 为
  16 px，且 border/handle 只在编辑态出现。19 条 taskbar-overlay 回归（含真实陈旧窗口/关联 ID
  生命周期回归）、升级后的生命周期架构门禁、
  `cargo check -p nxtools-desktop --tests`、仅放行既有 build-script lint 的严格 Clippy、rustfmt 与
  `git diff --check` 均通过。desktop 全套为 305/309；分类按钮 selector、网络尾距、剪贴板空态宽度
  和下载详情 progress selector 四项失败与前序基线完全相同。下载 DL-01 至 DL-28 至此完成，下一
  批次从 QL-01 主弹窗开始。

下载主列表 Release 复核修正（2026-08-22）：

- 包级实机复核发现 46 px 任务行仍把所有文件画成 raised surface 内的通用 Download glyph，并把
  `DownloadFault.message` 的英文协议详情和任务 UUID 直接放进辅助行；这不改变 DL-01 至 DL-28 的
  功能完成状态，但不满足文件身份扫描和失败信息层级。
- 主列表现复用既有 typed `ExtractShellIcon` PlatformHost 通道：未完成任务按扩展名共享 24 px Windows
  文件类型图标，已完成任务按实际目标路径请求 Shell 图标；Rust 以 resource key 合并请求、以任务 ID
  和当前 resource assignment 防护陈旧结果。图标直接进入 26 px 透明槽，不再有 raised tile；加载或
  失败期间才显示中性 fallback。
- 失败辅助行根据完整 `DownloadFaultCode` 映射短中文摘要，并对资源变化、服务器忽略 Range/返回 200
  两个高频协议场景给出可执行提示。完整原始信息仍保留在有界 Tooltip 和底部详情 dock。名称恢复 Body
  regular，路径与数字提升至 secondary contrast，行、进度与固定列几何保持不变。
- 包级复核后补回旧版已有但整改时误删的“添加时间”：原生最小 920 px 与默认 1060 px 窗口都显示
  68 px 固定列，表头可直接切换 Created 排序；当天显示 `HH:mm`，更早任务显示 `MM-dd`，完整本地时间
  进入 Tooltip。已有紧凑排序菜单仍保留同一排序入口。两条生产 GPUI 门禁分别锁定 920 px 表头可见与
  排序行为，以及 1060 px 表头/行值的同宽同坐标；全套仍为 314/318，四项失败与既有基线相同。
  [100% DPI 添加时间实机截图](audits/download-main-remediation-2026-08-22/02-download-list-created-column-100pct.png)
  已确认默认 Release 窗口中的表头、降序指示、当天 `HH:mm` 与历史 `MM-dd` 行值均实际可见。
- 2 条失败摘要单测、typed Shell 请求/同扩展名合并/任务回填回归、真实 GPUI Shell 图标透明槽回归和
  共享 PNG 校验共 5 条定向测试通过；`cargo check -p nxtools-desktop --tests`、下载/UI state/桌面 shell
  三项架构门禁、package memory policy、rustfmt 与 diff check 通过；desktop 全套为 314/318，分类按钮
  selector、网络尾距、剪贴板空态宽度和下载详情 progress selector 四项失败与前序基线完全相同。
  PlatformHost 运行输出确认缺失文件按 Windows 文件类型取图标、
  不同快捷方式保留不同应用图标；Release 内存预检、Rust/C# 打包、PE subsystem 与产品图标门禁通过。
  [100% DPI 实机截图](audits/download-main-remediation-2026-08-22/01-download-list-100pct.png)为
  1076 x 508（含原生阴影，对应 1060 x 492 客户区），当前样本均为 `.bin`，因此 Windows 正确返回相同
  文件类型图标；125%/150% 仍归 Release 人工截图矩阵统一收口。

第三十个 P1 速启主弹窗切片（2026-08-22）：

- QL-01 保留一个复用的 620 x 410 graphite 工作面与 42 px 搜索栏；普通空查询时原生窗口收起为
  620 x 44，只保留搜索框且不挂载结果 surface，非空查询在同一窗口、同一宽度和同一搜索栏坐标下
  稳定展开到 410 px。查询间替换结果不重播入场动画，既有激活、焦点、provider 和执行生命周期不变。
- 主搜索从私有 13.5 px 迁移到共享 Body 12/18，并继承 `UI_FONT_FAMILY`。背景、正文、占位、边框、
  搜索图标、光标和选择态继续由共享 `Theme` / component theme 提供；搜索栏水平留白及间距统一为
  8 px，未引入搜索卡片、accent 填充或新的装饰层。
- 修复动画结果 wrapper 缺少纵向 flex 约束导致 40 px 结果行在展开后塌缩的问题；结果 surface 现在
  填满搜索栏下方的连续区域，既有行密度、selection、action、preview 和滚动职责留给后续 QL 切片。
- 红—绿生产 GPUI 门禁实际验证空查询 620 x 44、非空 620 x 410、搜索栏坐标不漂移、结果 surface
  连续填充以及动画 surface 内 40 px 行高。90 条 Quick Launch 定向回归、升级后的生命周期架构门禁、
  `cargo check -p nxtools-desktop --tests`、包级仅放行既有 build-script lint 的严格 Clippy、rustfmt、
  `git diff --check` 与 PlatformHost `dotnet build` 均通过。`cargo test --workspace` 中 desktop 为
  307/311；分类按钮 selector、网络尾距、剪贴板空态宽度和下载详情 progress selector 四项失败与
  前序基线完全相同。原样 workspace Clippy 还命中同一 build-script lint 及下载模块三个既有未使用项，
  本切片不跨域修改。100% DPI 当前真实 GPUI [收起态](audits/quick-launch-ql01-2026-08-22/01-collapsed-100pct.png)
  与[展开结果态](audits/quick-launch-ql01-2026-08-22/02-results-100pct.png)已人工检查；125%/150% 继续由
  Release 截图矩阵在速启视觉批次收口时验收。QL-01 完成，下一切片为 QL-02 结果列表。

第三十一个 P1 速启结果行切片（2026-08-22）：

- QL-02 将应用、文件、命令、计算和网址收敛到同一 40 px 三栏结果行：26 px 透明图标槽、可伸缩的
  双行文案栏和 48 px 快捷提示栏。真实 Shell 图标与语义 fallback 直接位于透明槽中，不增加色块；
  标题、父路径/结果语义和快捷提示切换来源时保持同一横坐标，长文案只在自身栏内截断。
- 标题从私有 13 px medium 迁移到共享 Body 12/18 regular，副信息与快捷提示使用 Label 11/16，
  双行间距由任意 1 px 改为 `SPACING_2`，栏间继续使用 8 px 节奏。选中、hover、动作模式和预览
  生命周期未在本切片提前重做。
- 参考实现确认 Listary 的普通异构结果共享 `Title/Subtitle/Icon/Hotkey` 模板，SwiftList 对当前可见
  前九行分配快捷提示。NxTools.Next 因此为可见普通结果显示并实现 Ctrl+1 至 Ctrl+9；滚动后按键
  依据当前可见范围定位，范围外保留固定栏但不显示不可执行提示。快捷键、回车、鼠标、动作模式和
  最近目录执行均记录触发来源；Windows 命令从 accepted 到 terminal 复用同一 correlation ID，终态
  同时记录耗时、成功/失败和 native error code，且不写入查询、目标路径等敏感载荷。
- 红—绿生产 GPUI 门禁使用应用、文件、命令、计算、网址五类真实 `SearchResult`，逐栏比较 icon、
  copy、title、subtitle、shortcut bounds，并通过连接的内存平台端点验证实际 `ctrl-2` 键击发出第二
  行的类型化命令且 pending action 复用相同 correlation ID。首轮图标补验只证明兜底图标有像素，且
  截图脚本用假 PlatformHost 把同一个桌面 EXE 路径冒充所有 `extract_shell_icon` 结果；因此方格/快捷方式
  符号不构成 QL-02 的真实 Shell 图标验收。纠正后的包级流程保留真实 PlatformHost，通过其原生消息窗
  呼出面板，等待 Shell 请求完成后截图，并要求九行至少出现四种不同图标轮廓；Windows 侧另以两个真实
  快捷方式锁定不同应用不得坍缩为一个通用图标。PNG 可解码检查与失败 fallback 仍保留，但只有应用自身
  Shell PNG 实际出现在列表中才视为通过。104 条 Quick Launch
  定向回归、生命周期架构门禁、`cargo check -p nxtools-desktop --tests`、放行四项已记录基线后的包级
  严格 Clippy、rustfmt、`git diff --check` 与 PlatformHost `dotnet build` 均通过。workspace 全套中
  desktop 为 310/314；分类按钮 selector、网络尾距、剪贴板空态宽度和下载详情 progress selector
  四项失败与前序基线完全相同；原样 workspace Clippy 仍先命中同一 build-script needless-borrow。
  96 DPI 当前真实 GPUI [结果行验收图](audits/quick-launch-ql02-2026-08-22/01-result-rails-100pct.png)
  已由真实 PlatformHost 重新捕获并人工确认 Quicker、QQ、Quick Assist、Metasequoia 等应用自身图标、
  九行列坐标与截断行为；125%/150% 继续由 Release 截图矩阵在速启
  视觉批次收口时验收。
  QL-02 完成，下一切片为 QL-03 选择态。

第三十二个 P1 速启选择态切片（2026-08-24）：

- QL-03 保留一个 Rust-owned `selected` 索引与一层中性 `Theme::element_selected` 整行填充；真实
  Shell PNG 继续直接位于透明图标槽中，选择不增加 accent 卡片、装饰色块、描边或独立动效。新的
  96 DPI Release [选择态实机图](audits/quick-launch-ql03-2026-08-24/01-selection-unified-100pct.png)
  为 652×434（含 8 px 原生阴影），使用真实 PlatformHost、Shell 图标和索引结果；指针扫过后第八行
  仍只显示一层克制的整行中性选择面。125%/150% 继续由 Release 截图矩阵统一收口。
- 参考复核确认 Listary 的 guarded `MouseMove` 只防止静止指针抢走键盘选择，普通 WPF
  `ListBoxItem` 仍在按下时提交同一个 selected item；SwiftList 的 Up/Down、Enter 与 pointer 也共享
  一个 `SelectedResult`。NxTools.Next 原实现虽让移动后的 hover 和键盘共用索引，但直接按下另一行
  会在 click 执行前继续显示旧键盘项，造成“可见选择”与“将执行项”短暂分裂。
- 红—绿生产 GPUI 门禁先用真实 Down 键把第二行选中，再验证第一次 pointer move 不接管、第二次
  不同坐标 move 选中第三行、Up 回到第二行，最后直接左键按下第三行；旧实现停在第二行，修复后
  四条路径都写入同一个索引。复审补强后同一测试继续发送真实 mouse-up，并从真实内存 IPC 端点
  断言最终执行的是第三行 `third.lnk`，pending action 的 `entry_id` 与触发源也分别为 `third` / Pointer，
  因而“可见选择”与“实际执行项”由同一生产链端到端锁定。Ctrl+1…9、滚动、预览延时和跨实体调度链
  保持不变。
- Quick Launch 定向回归为 105/105；升级后的生命周期架构门禁、`cargo check -p
  nxtools-desktop --tests`、无允许项的严格 desktop Clippy、rustfmt 与 `git diff --check` 均通过。
  Desktop 全套为 315/319，分类按钮 selector、网络尾距、剪贴板空态宽度和下载详情 progress
  selector 四项失败与前序基线完全相同。标准 Release 包装在完成架构、fmt 与严格 Clippy 后仍按
  规则停在这四项既有 workspace 测试失败；在上述验证结果已独立留证后，使用官方包装脚本的
  `-SkipVerification` 模式避免重复验证，仍保留 6 GiB 安全门槛并以 7.62 GiB headroom 完成 Release
  编译、可信二进制清单、PlatformHost publish、PE subsystem 与产品图标门禁，产物位于
  `artifacts/win-x64`。受保护运行副本与新包的 desktop / PlatformHost SHA-256 均逐一相等后捕获上图。
  QL-03 行为、生产窗口、包级 96 DPI 与 Standards/Spec 双轴验收完成；下一切片为 QL-04 动作模式。

第三十三个 P1 速启动作模式切片（2026-08-24）：

- 修复“下滚结果、进入二级动作、返回时先闪到顶部再跳回原位置”的可用性缺陷。红测试在真实
  620×410 GPUI 窗口中把第 25 项滚到视口内，记录父列表实际偏移 `-474 px`，经 Right 进入动作模式、
  Escape 返回后旧实现首帧偏移为 `0 px`；修复后查询、选择身份与同一个 `ScrollHandle` 均冻结，返回
  首帧直接保持原偏移。恢复查询发生在 action mode 仍拥有 input-change 处理权时，因此不会启动一轮
  替换搜索再异步居中。Escape、根级 Left 与空过滤 Backspace 共用这一退出路径；后台应用目录刷新
  也会在 action mode 期间只提交 catalog snapshot，不替换或居中被冻结的父列表。
- 参考复核确认 Listary 只弹出并销毁子 `SearchMode`，重新激活同一个父 mode；SwiftList 也先在动作
  模式内恢复保存查询，再显示原 result grid 并滚回已选项。NxTools.Next 删除了退出时清零结果滚动、
  重挂 presented results 和延后 selection 恢复的旧链路，遵守“返回哪儿就是哪儿”的首帧契约。
- QL-04 将普通结果与动作项收敛到同一个生产 row shell：40 px 行高、透明 26 px 图标轨、灵活
  Body 12/18 文本轨与固定 48 px 尾轨；动作模式只增加一条 24 px、Label 11/16 的目标上下文，不增加
  卡片填充、描边、阴影或第二套入场动效。真实 GPUI 几何测试逐一比较普通行与动作行的 row、icon、
  copy、title 和 trailing bounds；三种退出键均在 `window.refresh()` 后确认第 25 项仍出现在第一棵
  返回渲染树，生命周期架构门禁同时禁止 36 px 私有动作行、Medium 字重、catalog refresh 越过冻结
  边界与返回时触碰父列表 base scroll handle。
- Quick Launch 定向回归为 108/108；严格 desktop Clippy、rustfmt、`git diff --check` 和升级后的
  生命周期架构门禁均通过。Desktop 全套为 318/322，分类按钮 selector、网络尾距、剪贴板空态宽度
  和下载详情 progress selector 四项失败与前序基线完全相同。完整验证单独留证后，官方包装脚本以
  `-SkipVerification` 避免重复运行同一失败基线，并在 6.81 GiB headroom 下完成 Release、可信二进制
  清单、PlatformHost publish、PE subsystem 与产品图标门禁。包与受保护运行副本的 desktop /
  PlatformHost SHA-256 逐一相等后，通过真实 PlatformHost 捕获 96 DPI、652×434（含原生阴影）的
  [动作模式实机图](audits/quick-launch-ql04-2026-08-24/01-action-mode-100pct.png)；烟测先以屏幕像素
  判据确认首个选中动作行已经呈现才允许捕获，图中轻量上下文、共用动作行轨道和原生 Windows 动作
  连续面均已人工确认。125%/150% 继续由 Release 截图矩阵统一
  收口。QL-04 行为、生产窗口和包级 96 DPI 验收完成；下一切片为 QL-05 预览。

第三十四个 P1 速启预览切片（2026-08-24）：

- QL-05 抽出共享 `PreviewShell` 与媒体承托面：速启和剪贴板现在共用不透明石墨外壳、窗口边界、
  中性媒体 surface 与 4 px 圆角，但仍各自拥有尺寸、窗口、指针和资源生命周期。速启预览保持
  400×410 的不可激活独立窗口，并固定为 72 px 身份头部、8 px 内容内边距和 30 px 路径尾栏；
  图片、文本和不可预览状态都进入同一连续内容面，内容类型不再改变外壳高度或生成内嵌卡片。
- 预览文字迁移到共享 Body 12/18 与 Label 11/16，正文/路径只在扫描节奏需要时使用 Mono。真实
  GPUI 生产窗口回归逐一锁定 400×410 外壳、72/30 px 头尾和 382×290 媒体区域；剪贴板生产渲染
  同时由静态契约确认调用同一外壳与媒体 helper，防止两边重新分叉出私有边框和承托色。共享 helper
  只提供 surface，不强加图片式 flex 居中；第二个真实 GPUI 回归确认文本从媒体面 12 px 左上内边距
  开始，图片和空态才在各自分支中居中。
- 修复复用预览时“先退场再入场”的双动效：已显示的旧内容会保留到 generation 匹配的新 payload
  完成离屏 staging，然后只在 `set_payload` 播放一次共享 130 ms 入场；目标切换路径不再调用 exit。
  明确离开仍保留 90 ms 退场，旧 load/stage/window 回调继续受 entry、generation 和 HWND 身份保护，
  也不取得键盘焦点。真实生命周期测试直接读取复用中的生产 view，确认等待替换时 `is_exiting`
  始终为 false；若旧 90 ms 退场被新目标打断，动画 generation 也保持不变，不会先让旧 payload
  重新入场，旧 payload 会直接移除动画 wrapper，直到 replacement payload 发布。pending stage 通过
  单一 window identity 同时冻结 GPUI window 与原生 HWND，不支持预览的目标先作废旧 generation，
  next-frame 和原生结果只有双身份仍匹配时才可发布；Release 烟测还会拒绝没有同时保留 620×410
  面板与 400×410 预览 HWND 的截图。
- Quick Launch 定向回归为 110/110；严格 desktop Clippy、rustfmt、`git diff --check` 和升级后的
  生命周期架构门禁均通过。Desktop 全套为 320/324，分类按钮 selector、网络尾距、剪贴板空态宽度
  和下载详情 progress selector 四项失败与前序基线完全一致。完整验证留证后，官方包装脚本以
  `-SkipVerification` 避免重复运行已知失败基线，并在 7.60 GiB headroom 下完成最终 Release、可信
  二进制清单、PlatformHost publish、PE subsystem 与产品图标门禁；产物 desktop SHA-256 与本次
  生成清单逐字匹配，配套 PlatformHost 也来自同一 publish。便携 Release 的受控服务初始化进入终态后，
  烟测通过 `-ReuseExistingProcess` 复用同一 desktop / PlatformHost 进程，真实 PlatformHost 捕获的
  95.9866 DPI
  [联合实机图](audits/quick-launch-ql05-2026-08-24/01-preview-shell-100pct.png) 为 1070×434（含
  两窗原生阴影），展示真实 Shell 应用图标、固定预览骨架和不可预览项目的共享中性承托面；同一
  烟测完整通过。QL-05 行为、生产窗口与包级 96 DPI 检查点已经完成；125%/150% 尚未留证，
  因此本切片不声明 Release 视觉批次验收完成，仍由截图矩阵统一收口，也未开始 QL-06 工作。

第三十五个 P1 速启文件对话框搜索切片（2026-08-24）：

- QL-06 保留已经稳定的文件对话框识别、独立 GPUI 辅助 HWND、owner、首次非激活 Topmost、宿主
  跟随、输入焦点、65 ms debounce、索引 generation 与目录/文件激活语义，只迁移固定栏的呈现。
  参考复核确认 Listary 由 `DialogSearchWindowFactory` 创建独立 `DialogSearchWindow`，按最终
  `ActualWidth` 居中贴住宿主底边并把结果向宿主展开；SwiftList 同样冻结文件对话框 HWND、使用
  独立窗口并恢复原编辑框焦点。旧 NxTools 没有可迁移的速启或文件对话框搜索表面。
- 搜索头继续为 500 logical px 宽、36 px 高；结果面保持 264 px 宿主内重叠和 40 px 行，空查询的
  Quick Switch 来源行保持 36 px。输入与结果标题迁移到 Body 12/18，路径和 `Ctrl+G` 迁移到
  Label 11/16；所有轨道使用 12 px 外边距、8 px 间距和 2 px 双行间距。结果现在固定一条透明
  26 px 图标轨，去掉 13/10.5/10 px 私有字号、9/1 px 任意间距和 Medium/Semibold 私有字重；
  搜索中与空结果复用共享 Work State，不添加卡片、徽章或无行为工具按钮。
- 根面使用 `Theme::window_border`，外部浮动阴影继续由带 Client decorations 的原生 HWND 提供，
  避免在精确尺寸不透明根节点内绘制会被裁掉的假阴影；默认来源不再伪装为永久选中项。红—绿生产
  GPUI 回归直接渲染 `FileDialogSearchPanel` 并读取搜索头、输入、结果行、图标轨、标题和路径
  bounds；另一个设备像素回归逐一验证 96/120/144 DPI 下可见宽度不越过宿主左右边界，结果重叠
  精确收于宿主底边。窄宿主 16 px logical 安全边距和 C# 校准后的 visible-bounds 居中门禁继续通过。
- QL-06 定向 Rust 为 12/12，Quick Launch 为 114/114；文件对话框视觉、首次显示生命周期、无
  Z-order 跟随与完整 Quick Launch 生命周期架构门禁全部通过。相关 PlatformHost 过滤集 15/15
  通过，覆盖真实结构识别、现代地址栏导航、文件提交、居中定位与校准跟随；严格 desktop Clippy
  通过。Desktop 全套为 324/328，分类按钮 selector、下载详情 progress selector、网络尾距与
  剪贴板空态宽度四项失败与前序基线完全一致。当前自动化已经锁定 96/120/144 DPI 设备像素边界，
  但尚未分别留存 100%/125%/150% 的 Release OS 截图，因此不声明速启 Release 视觉批次验收完成。

第三十六个 P1 速启主窗口设置切片（2026-08-24）：

- QL-07 将主窗口速启设置从一个同时堆叠呼出、索引、上下文、命令和诊断的长滚动面，拆成固定
  160 px 局部导航轨与一个可伸缩内容页。五个明确页面为常规、索引、上下文、命令、诊断；模块页头、
  局部导航和 48 px 当前页标题保持固定，只有当前页使用原有平滑滚动 viewport。换页会取消旧滚动
  generation、把内容复位到顶部并仅改变呈现状态，不修改任何设置。
- 页面归属已经按行为重新分组：常规保留呼出热键和键盘动作，索引承载扫描状态、文件索引、卷与排除
  目录，上下文承载资源管理器快速切换和文件对话框搜索，命令承载现有自定义命令编辑器，诊断承载
  排序学习、最近查询性能与日志入口。现有即时设置动作、目录/程序选择器、索引与日志 Rust/C# 边界
  均未改变；QL-08 至 QL-12 仍分别拥有热键、卷、排除、命令和诊断控件的后续视觉整改。
- 参考复核确认旧 NxTools 没有速启设置面；Listary 用稳定 `Pages`/`SelectedPage` 与窗口级
  Apply/OK/Cancel 保持导航和设置提交分离，SwiftList 同样固定 titlebar/sidebar/footer 并只替换当前
  page。NxTools.Next 采用该信息架构，不复制两者的 WPF/反编译视觉。设计系统同步锁定 160 px rail、
  40 px 导航行、2 px accent edge、Title 14/20 与 Label 11/16 页头。
- 新增两个真实生产 `MainWindow` GPUI 回归：在 1060×500 窗口中验证五个 40 px 导航行、默认常规
  页、48 px 页头与共享字阶，并依次点击索引、上下文、命令、诊断和常规；五个独立首帧逐一证明
  只挂载当前页。真实速启实体另以 20 条排除目录保留非平凡设置快照，启动平滑滚动后立即换页并推进
  虚拟时钟，证明旧 generation 不能回写新页且导航前后设置完全相同。定向 Quick Launch 为 116/116，
  严格 desktop Clippy 与 Quick Launch 生命周期架构门禁通过；门禁已
  改为读取 QL-06 的共享 compact-result 40/26/48 metrics，不再匹配退休的私有常量名。Desktop 全套
  为 326/330，分类上移 selector、下载详情 progress selector、网络尾距与剪贴板空态宽度四项失败
  与前序基线完全一致。
- 本切片没有制作或接受 100%/125%/150% Release OS 截图，因此不声明速启 Release 视觉批次收口；
  QL-07 的结构、行为与自动化验收完成，下一切片为 QL-08 热键设置。

第三十七个 P1 速启热键设置切片（2026-08-24）：

- QL-08 将常规页的私有 26 px/10.5 px 热键按钮替换为共享 `HotkeyField`：固定 260 px 宽、
  32 px 内凹录制字段和 16 px 同位状态行。当前或候选组合键使用 Mono Body 12/18，设置标题与
  字段值使用 Body 12/18，说明与状态使用 Label 11/16；6 px 语义标记和克制边界分别表达已应用、
  等待、录制、应用中与失败，不增加 keycap 色块、状态卡片或行宽变化。
- 录制仍沿用现有类型化 `SetQuickLaunchHotKeyRecording -> ConfigureQuickLaunch` Windows 边界。
  点击已应用/失败字段开始录制，原生暂停与应用期间拒绝第二次点击；Escape 与失焦继续取消。候选
  组合键在相关命令等待期间保持可见，只有 correlation-matched 成功结果才替换并持久化当前设置；
  非匹配成功结果保持应用中，Win32 冲突或其他失败保留上一个已应用热键并在字段下方显示错误。
- 参考复核确认旧 NxTools 只有剪贴板热键录制链路而没有速启界面；Listary 与 SwiftList 均使用一个
  聚焦、只读、原位录制字段。SwiftList 的裸 Ctrl 双击语义与 Listary 的字段行为仅作为行为证据，
  本切片不复制其 WPF/反编译视觉，也不改变 NxTools.Next 已有 C# 双击 Ctrl 适配器。
- 新增真实生产 `MainWindow` GPUI/内存 PlatformHost 回归，在 1060×500 常规页先验证初始配置的
  应用中、correlation-matched 失败与重试成功，再逐步验证已应用、等待原生暂停、录制、候选
  `Ctrl+Alt+V` 应用中和 Win32 1409 失败五态；录制态点击不会取消或发出第二条命令，非匹配成功
  correlation 不能完成应用或替换当前设置。回归同时锁定 32 px 字段、
  Body/Label 行高、类型化命令及失败后设置不变。共享录制/应用跨实体回调同时延迟到下一 application
  turn，架构门禁锁定五态、共享令牌、真实测试和 deferred 边界。热键定向为 3/3，Quick Launch
  为 117/117，严格 desktop Clippy、rustfmt、生命周期门禁与 `git diff --check` 通过；Desktop
  全套为 327/331，仍仅有分类上移 selector、下载详情 progress selector、网络尾距与剪贴板空态
  宽度四项既有失败。
- 按用户要求，本切片未执行 Release 打包、未启动 NxTools，也未制作或接受 100%/125%/150% OS
  截图；自动验证仅限源码测试和静态门禁。QL-08 完成，下一切片为 QL-09 索引卷。

第三十八个 P1 速启索引卷切片（2026-08-24）：

- QL-09 将索引页原先会换行的 24 px 私有 chip 集合替换为一个全宽 `VolumeScope` 复选列表：
  48 px 顶部模式行明确显示“自动包含全部 NTFS 卷”，每个已发现卷使用稳定的 40 px 行、标准
  `Checkbox`、Body 12/18 挂载点和 Label 11/16 文件系统/索引项元数据。等待发现卷也留在同一
  40 px 列表行；选择态仅由复选框表达，不再使用 accent chip、badge 或卡片。
- 既有持久化语义保持不变但已在界面中显式说明：空卷向量代表自动包含当前及未来发现的全部 NTFS
  卷；在自动模式取消任一卷会把其余已发现挂载点物化为明确集合；重新勾满全部卷或点击自动模式会
  规范化回空向量，即使旧配置仍含已移除挂载点。由于空向量专用于自动全部，列表拒绝取消最后一个
  当前选中卷。禁用卷仍由 indexer 状态保留为可重新启用的列表行，Rust 配置、索引器同步协议及刷新
  行为没有改变；所有跨实体设置更新继续延迟到下一 application turn。
- 参考复核确认旧 NxTools 没有速启或本地索引范围界面；Listary 的稳定卷根/禁用集合和 SwiftList
  的逐卷复选行、文件系统及项目数只作为行为证据，未复制其 WPF 或反编译视觉。设计系统同步锁定
  单列表、48/40 px 行高、共享字阶、标准复选框及自动/指定卷语义。
- 新增真实生产 `MainWindow` GPUI 回归，在 1060×500 索引页注入 C/D/E 三个真实 `IndexDriveStatus`
  状态，锁定列表左右轨、三行几何、Body/Label 行高及自动模式下全部勾选；真实点击 C 卷后验证配置
  物化为 D/E，并分别点击 checkbox indicator 和行内容，逐次锁定只产生一条类型化 indexer 设置命令；
  回归还验证最后卷拒绝取消、无重复同步及自动行恢复空向量。Quick Launch 定向为 118/118，严格 desktop Clippy、
  rustfmt、生命周期架构门禁与 `git diff --check` 通过；Desktop 全套为 328/332，仍仅有分类上移
  selector、下载详情 progress selector、网络尾距与剪贴板空态宽度四项既有失败。
- 按用户要求，本切片未执行 Release 打包、未启动 NxTools，也未制作或接受 Release OS 截图。
  QL-09 的结构、行为与自动验收完成，下一切片为 QL-10 排除目录。

第三十九个 P1 速启排除目录切片（2026-08-24）：

- QL-10 将索引页原先 210 px 宽、24 px 高、9.5 px 私有字阶的目录块替换为一个全宽连续
  `PathExclusionList`：标题与说明使用共享 Body 12/18、Label 11/16，添加目录复用 28 px action；
  每条规则固定为 40 px 行，包含 16 px folder rail、可伸缩且截断的完整路径主栏，以及固定
  40×40 px 尾部删除命中区。长路径 Tooltip 保留精确原值，空集合仍显示一条中性 40 px 行；
  删除按钮静止态保持中性，仅在 hover/press 使用 danger 语义。
- 列表现在拥有单一选中索引；点击规则后 Up/Down 在当前列表内移动，Escape 清除选择，Delete
  与 Backspace 均删除当前规则，删除最后一项安全回到空列表。选中只使用共享 neutral fill 和
  非位移 2 px accent edge。指针删除、键盘删除及添加目录均冻结当前 intent，并把跨实体更新延迟到
  下一 application turn；每次删除只持久化并向 indexer 发送一条类型化 `IndexSettings`。
- Windows 目录选择器继续保留同一 typed `PickDirectory` 边界、correlation identity、大小写不敏感
  去重、持久化、取消 no-op 与失败通知。新增 start/terminal 诊断记录 correlation、selected/
  cancelled/failure outcome、elapsed 和 native error code，但不记录当前目录或用户选中的路径。
  参考复核确认旧 NxTools 无对应界面；SwiftList 的稳定路径行、尾部动作、Tooltip 与去重只作为
  行为证据，未复制其 WPF 卡片、嵌套 Tab 或批量文本编辑器。
- 红—绿真实生产 `MainWindow` GPUI 回归在 1060×720 索引页验证连续列表左右轨、三条 40 px 行、
  Body 路径行高、40×40 删除区、长路径 Tooltip、Up/Down/Escape/Backspace/Delete、指针删除、
  最后一项删除与 40 px 空态；连接的内存 Platform/Indexer 端点同时验证每个 gesture 仅一条命令、
  stale picker result 被拒绝、大小写重复与取消/失败不写入、匹配选择只同步一次。Quick Launch
  定向为 119/119；严格 desktop Clippy（仅放行既有 build-script lint）、rustfmt、生命周期架构门禁
  与 `git diff --check` 通过。Desktop 全套为 329/333，仍仅有分类上移 selector、下载详情 progress
  selector、网络尾距与剪贴板空态宽度四项既有失败。
- 按用户要求，本切片未执行 Release 打包、未启动 NxTools，也未制作或接受 Release OS 截图。
  QL-10 的结构、行为与自动验收完成，下一切片为 QL-11 自定义命令。

第四十个 P1 速启自定义命令切片（2026-08-24）：

- QL-11 将命令页原先 190 px 高的私有紧凑卡片替换为一个 360 px 连续 master-detail。左侧固定
  224 px master rail：标题栏承载标准 28 px“新增”，每条命令固定 42 px，只显示 Body 12/18 名称和
  Mono Label 11/16 关键词/停用上下文；选中使用 neutral fill 与独立的非位移 2 px accent edge。
  逐行 18 px 启用框和 20 px 删除动作已移除，列表行只承担身份与选择。
- 右侧只有一个详情编辑器：48 px 标题栏承载标准 `Checkbox` 启用与静止态中性的 28 px 删除；关键词/
  显示名称、参数模板/工作目录各自形成等宽双列并保持 8 px gutter，程序行使用同一 32 px 输入与
  28 px picker，管理员运行和静默运行改用标准 `Checkbox`。保存是唯一 primary action，输入字段只在
  显式保存时校验并提交；新增、启用、删除继续保持既有显式即时持久化语义。
- 所有命令列表跨实体动作均延迟到下一 application turn。程序选择器继续使用 typed `PickProgram`，
  新增 start/terminal 诊断保留 correlation、selected/cancelled/failure outcome、elapsed 与 native
  error code，不记录当前或选中路径。MainWindow 同时使用 correlation、命令索引和 draft generation
  保护未保存编辑器；切换、删除或重新创建选择后，旧 picker 回执不能写进新草稿。
- 参考复核确认旧 NxTools 无自定义命令界面；Listary 的集合/当前选择/新增后选中/删除当前项只作为
  行为证据，SwiftList 对象数组的左列表右字段编辑器只作为 master-detail 布局证据；没有复制其 WPF
  视觉，且明确拒绝 SwiftList 的逐行尾部删除布局。
- 红—绿真实生产 `MainWindow` GPUI 回归在 1060×720 命令页注入两条真实命令，锁定连续 shell、
  224 px master/detail 邻接、42 px 行、Body/Label 行高、2 px 选中边、32 px 字段以及五个 28 px
  动作；真实点击依次验证选择不持久化、详情启用、新旧 picker correlation、匹配结果只改未保存草稿、
  离开并回到同一索引仍拒绝 stale result、取消 no-op、保存、新增与删除。内存 Platform 端点验证每次
  picker gesture 只发送一条 typed `PickProgram`。
  Quick Launch 定向为 120/120；严格 desktop Clippy、rustfmt、生命周期架构门禁与
  `git diff --check` 通过。Desktop 全套为 330/334，仍仅有分类上移 selector、下载详情 progress
  selector、网络尾距与剪贴板空态宽度四项既有失败。
- 按用户要求，本切片未执行 Release 打包、未启动 NxTools，也未制作或接受 Release OS 截图。
  QL-11 的结构、行为与自动验收完成，下一切片为 QL-12 诊断页收尾。

第四十一个 P1 速启诊断页收尾切片（2026-08-24）：

- QL-12 将诊断页原先带任意 10 px 字号、24 px 私有按钮、5 px 卡片圆角和装饰性 LIVE/IDLE 文案的
  三行紧凑组，替换为一个连续无圆角诊断面。面内依次为五个相邻 48 px 行：历史标题、历史指标、
  性能标题、性能指标和日志目录；历史按排序项目/启动次数/关键词关联/最近目录四等分，最近查询按
  本地合并与 MFT/USN 两等分。标签统一使用 UI Label 11/16，计数与耗时统一使用 Mono Body 12/18，
  有无样本不会改变任一轨道的几何；性能信息只挂载于诊断页，不再用 accent 状态干扰普通设置路径。
- 诊断页只读取既有 `FinderCatalog::history_summary`、本地合并耗时和 `IndexSearchBatch.elapsed_us`
  内存快照，没有增加采样线程、空闲轮询、逐结果计时或 SQLite 查询。完整日志目录在可截断轨内保留
  精确值 Tooltip；“打开日志”继续只发送一条 typed `OpenShellTarget(Open)`，“清空历史”继续原子
  清理排序、查询关联和最近目录的 SQLite/内存状态并使用全局通知，不删除应用/文件索引或用户设置。
  两个 28 px 动作均把跨实体工作延迟到下一 application turn，清理动作静止态中性、仅 hover/press
  使用 danger。
- 参考复核确认旧 NxTools 没有速启诊断界面；Listary 的独立诊断入口与类型化索引服务链路只支持把
  运维信息与普通设置分离，不作为 WPF 视觉或 native 计时算法来源。NxTools.Next 继续使用自身既有
  历史、查询批次和日志目录边界，不复制第三方对话框或增加新的跨进程协议。
- 红—绿真实生产 `MainWindow` GPUI 回归在 1060×720 诊断页注入两次真实排序激活和 240 µs/
  12.35 ms 内存快照，锁定连续 5×48 px 结构、四等分/二等分轨、Mono Body 数值、两个 28 px 动作与
  精确日志 Tooltip；连接的内存 Platform 端点验证一次点击只有一条 typed Shell 命令，真实清理点击
  验证历史摘要归零且仍停留诊断页。独立真实窗口逐页证明性能轨只在诊断页挂载；同一窗口实际切换
  常规、索引、上下文和命令后，原动作位置不能再发 Shell 命令或清理历史，返回诊断页后才恢复。
  Quick Launch 定向为 121/121；严格 desktop Clippy、rustfmt、
  生命周期架构门禁与 `git diff --check` 通过。Desktop 全套为 331/335，仍仅有分类上移 selector、
  下载详情 progress selector、网络尾距与剪贴板空态宽度四项既有失败。
- 按用户要求，本切片未执行 Release 打包、未启动 NxTools，也未制作或接受 Release OS 截图。
  QL-01 至 QL-12 的结构、行为和自动化切片全部完成；速启的 100%/125%/150% Release 人工截图仍按
  总体验收矩阵保留，下一实现批次为 CL-01 剪贴板保护性迁移。

第四十二个 P2 剪贴板主窗口设置切片（自动化实施完成，视觉验收待补；2026-08-24）：

- CL-01 保留现有高密度双栏信息层级：左侧弹性主列继续承载全局呼出热键与 Ctrl/Alt/Shift 单击动作，
  右侧固定 410 px 次列继续承载最大记录、清理天数、排除进程、归档、数据库维护和存储迁移。没有
  移动、合并或重命名任何用户操作，也未改变默认 Alt+C、持久化、picker 或 Windows 注册边界。
- 原先写在 `MainWindow::render_clipboard` 内的 5 px 圆角私有双栏实现抽成共享
  `settings_split_surface`：一个方角连续 surface、一个低对比外边界、两侧 12 px 内容 inset 和一条
  1 px 中缝。主列保持可伸缩，次列保持固定宽度；两列不再各自形成卡片、accent 填充或独立 elevation。
- 红—绿真实生产 `MainWindow` GPUI 回归在 1060×500 窗口中直接进入剪贴板，锁定工作区 12 px
  安全边距、单一连续面、主列/中缝/次列无间断相接、410 px 次列与两列共同上下边界；旧实现因缺少
  共享 surface seam 和 selector 先失败，迁移后通过。独立架构门禁同时禁止重新加入圆角私有分栏、
  accent 或 shadow，并锁定原有两组职责名称。
- 剪贴板双栏定向回归为 1/1；严格 desktop Clippy、rustfmt、新剪贴板 UI 架构门禁与
  `git diff --check` 通过。Desktop 全套为 332/336，仍仅有分类上移 selector、下载详情 progress
  selector、网络尾距与剪贴板弹窗空态宽度四项既有失败。
- 按用户要求，本切片未执行 Release 打包、未启动 NxTools，也未制作或接受 Release OS 截图。
  当前仓库也没有批次 1 的剪贴板主窗口设置迁移前基准，只有 popup/preview 证据；因此共享布局实现
  与保护性回归已经完成，但 CL-01 仍保留视觉验收，不能据此宣告整个切片完成或提前进入 CL-02。
  待用户要求打包运行并提供迁移后主窗口截图后，需与补录的 100% 基准逐项核对密度、主次层级和
  长中文裁切；通过后下一切片才是 CL-02 剪贴板弹窗字体、图标、菜单与状态组件。

  2026-09-01 补充:CL-01 对照证据已补齐——`docs/audits/clipboard-settings-cl-01-2026-09-01/`
  保存了同一几何、同一 100% DPI 下的迁移前（由 `36db4aa` 工作树构建渲染）与迁移后截图及
  `audit.md` 逐项对照；密度、层级与长中文均无退化，仅一处说明文字因 11px 令牌升幅换行。
  2026-09-01 用户授权自动化管线：每个切片的验收（自动化门禁 + 截图对照证据）通过后自动开始
  下一片并推送远端；CL-01 据此标记完成并进入 CL-02。阶段级人工验收项（下载阶段 6 第 4 项、
  速启阶段 5 清单、125%/150% DPI Release 截图矩阵）仍保留给用户，管线不得冒领。

第四十四个 P2 剪贴板弹窗令牌与菜单切片（2026-09-01，自动化管线运行）：

- CL-02 将剪贴板弹窗与共享表面从 theme.rs 的旧私有字号/圆角常量整体迁入设计令牌：
  SEARCH_TEXT_SIZE 13.5 与 RESULT_TITLE_SIZE 13.0 → Body 12,RESULT_DETAIL_SIZE 10.5 与
  META_TEXT_SIZE 10.0 → Meta 10,COMPACT_RADIUS → RADIUS_CONTROL,quick_launch 的
  SURFACE_INSET/ROW_HORIZONTAL_PADDING → SPACING_6/SPACING_8;theme.rs 删除全部 7 个旧常量
  （66 处引用）。PANEL 300×440 默认尺寸与 8px 安全边距保持不变。
- ui-token-boundaries.ps1 扩展"退役旧令牌"规则：任何源码（含注释）重新出现这些名字即失败;
  先红（66）后绿（0）。
- 弹窗菜单统一：历史行右键（粘贴/复制/纯文本粘贴/去除换行/编辑/置顶/收藏/删除）与分类右键
  （重命名/删除分类）迁入共享 context_menu_shell（192–240px 壳层）+ context_menu_item
  （28px 行、14px 图标槽、Body 12/18、normal/danger 语义）；共享 context_menu_item 的 label
  泛化为 `impl Into<SharedString>` 以支持动态标签。打开目标选择器保留 180×320 壳层（整条注册表
  路径/文件位置需要更宽），行组件与按类型图标（注册表 Settings、系统工具 Terminal、目录 Folder、
  文件 OpenInNew、网址 Language）统一；来源分类子菜单改用共享壳层，勾选行保留 checked 语义。
- 验证：cargo fmt；cargo check 通过无警告；严格 clippy -D warnings（all-targets）通过——教训已记录：
  命令行 `-A` 标志会泄漏到 rustc 编译的 build-script 并在 -D warnings 下报 renamed_and_removed_lints
  （ERRORS.md ERR-20260901-CLIPPY-ALLOW-FLAG-LEAK）；clipboard 过滤测试 20 通过、1 失败为既有基线
  `production_clipboard_history_uses_the_shared_empty_state`（精确测试名补记
  ERRORS.md ERR-20260901-CL02-KNOWN-BASELINE-TEST-NAME;CL-02 字号缩幅不可能增大该宽度断言）；
  ui-token-boundaries / clipboard-ui-lifecycle / ui-state-lifecycle 门禁全部通过。
- 视觉证据：docs/audits/clipboard-popup-cl-02-2026-09-01/ 保存迁移前后弹窗对照（同几何
  316×449,Alt+C 经 PlatformHost 呼出,独立 NXTOOLS_BASELINE_DATA_ROOT 数据根,实例捕获后即退出）；
  空历史下右键菜单与行排版的像素对照无法在空数据上呈现,菜单呈现依赖共享组件的 DL-05 视觉门禁
  与本次通过的过滤测试。
- 本切片未启动 Release 打包;125%/150% DPI 截图矩阵仍归统一人工验收。

第四十五个 P2 剪贴板历史行共享外壳切片（2026-09-01，自动化管线运行）：

- CL-03 在 `ui/components/list.rs` 新增共享 `compact_result_row` 外壳： Neutral raised 静止面、
  hover 提升面与边框、7/4 px 紧凑内距、4 px 内容间距与 `RADIUS_CONTROL` 圆角收进组件常量,
  调用方只保留 on_hover/on_click 与内容子节点;外壳返回 `Stateful<Div>` 以保留 id/group 交互语义。
- `ui/history.rs::history_row` 的手写外壳（含私有 `history_row_border`/`history_row_background`
  辅助函数）退役,行渲染改经共享外壳;图片比例算法、行高计算、元数据（来源图标/时间/次数/字符数）、
  hover 管理按钮与右键菜单等调用方所有权不变。
- 红色门禁：`clipboard-ui-lifecycle.ps1` 新增 CL-03 契约（list.rs 必须定义共享外壳、history.rs
  必须经过它渲染、私有边框/背景辅助函数必须消失）,先红后绿。
- 验证：cargo fmt;cargo check 通过无警告;严格 clippy -D warnings(all-targets) 通过;
  clipboard/row_metrics/image_preview 过滤测试 24 通过、1 失败为既有基线
  `production_clipboard_history_uses_the_shared_empty_state`（与 ERRORS.md 已记录条目一致,
  非本切片回归）;ui-token-boundaries / clipboard-ui-lifecycle / ui-state-lifecycle 门禁通过。
- 视觉证据：`docs/audits/clipboard-history-cl-03-2026-09-01/` 保存三条真实历史行（URL 截断、
  短文本、104 字符长中文）在迁移前后的同几何（316×449）对照,行高、截断点、元数据与分类条
  完全一致;时间标签因捕获间隔从"刚刚/1 分钟前"变为"N 分钟前",属预期内容演化而非渲染差异。
  历史内容经系统剪贴板写入并由真实 PlatformHost 捕获落库,实例捕获后即退出。
- 本切片未启动 Release 打包;125%/150% DPI 截图矩阵仍归统一人工验收。

第四十六个 P2 剪贴板分类栏令牌切片（2026-09-01，自动化管线运行）：

- CL-04 将剪贴板弹窗内残余的 3 处 gpui `.text_xs()` 非令牌字号迁入 ui::typography：分类 tab 与
  弹窗对话框按钮 → `TEXT_LABEL/LINE_HEIGHT_LABEL`（分类名与按钮中文保持 ≥11px）,右下角缩放把手的
  装饰字形 → `TEXT_META`;分类栏不再出现任何非令牌字号。
- Tooltip 已经由 gpui_component `Tooltip` 统一承载并保持"名称 · 拖动排序 · 右键管理"内容契约;
  菜单已在第四十四个切片迁入共享 `context_menu_shell`——本切片在门禁中将两者锁定为必需契约,
  防止回退。
- 焦点态：核实 gpui 0.2.2 仅提供 `focusable()` 标志而没有焦点样式钩子(无 when_focused/focus
  border API),为分类栏添加真实键盘焦点可视化被与 DS-08 相同的依赖缺口阻塞,本切片如实记录阻塞
  而不做无样式效果的半实现;待依赖升级后与 DS-08 一并解决。
- 红色门禁：`clipboard-ui-lifecycle.ps1` 新增 CL-04 契约——弹窗源码禁止 `.text_xs()`,分类条
  必须使用 TEXT_LABEL/LINE_HEIGHT_LABEL 令牌且保留 `Tooltip::new`、`context_menu_shell`、
  `on_drag`/`on_drop`/`on_scroll_wheel` 生命周期,先红后绿。
- 验证：cargo fmt;cargo check 通过无警告;严格 clippy -D warnings(all-targets) 通过;
  category/clipboard 过滤测试 24 通过、2 失败均为既有基线并逐一定位(`production_clipboard_history_
  uses_the_shared_empty_state` 空态宽度、`category_settings_use_a_readable_list_...` 即记录中的
  "分类上移 selector",断言点 `download-settings-category-up` debug_bounds);横向滚轮与拖动排序
  两个保护性测试在通过集合中;ui-token-boundaries / ui-state-lifecycle 门禁通过。
- 视觉证据:`docs/audits/clipboard-category-cl-04-2026-09-01/03-clipboard-category-strip-post-CL-04.png`
- 本切片未启动 Release 打包;125%/150% DPI 截图矩阵仍归统一人工验收。

第四十七个 P2 剪贴板预览窗确认切片（2026-09-01，自动化管线运行）：

- CL-05 为确认性切片：速启与剪贴板共享的 `preview_shell`/`preview_media_surface` 已由 QL-05
  落地并有既有生产测试锁定,本切片把该共享 seam 与资源上限提升为静态架构契约——
  `clipboard-ui-lifecycle.ps1` 新增 CL-05 契约：preview.rs 必须定义共享外壳与媒体承托面、
  `ClipboardPreview` 及其共享外壳测试必须存在,`IMAGE_RESOURCE_CACHE_CAPACITY` 必须保持 96,
  且必须存在资源逐出回归测试;先红（缺逐出测试）后绿。
- 新增 `image_resource_cache_evicts_the_oldest_entry_beyond_capacity` 回归：容量 2 下第三条
  插入逐出最旧项、重触刷新新旧顺序、空路径永不入缓存,并断言生产容量恰为 96;预览位置/尺寸
  由既有 `preview_layout` 多分辨率测试继续保护,本切片未改动任何预览渲染代码。
- 验证：cargo fmt;clipboard 过滤测试 18 通过、1 失败为既有基线
  `production_clipboard_history_uses_the_shared_empty_state`（ERRORS.md 已记录）;严格 clippy
  -D warnings(all-targets) 通过;clipboard-ui-lifecycle（CL-01~05 全部契约）/ ui-token-boundaries /
  ui-state-lifecycle 门禁通过。
- 本切片无渲染变化,无需新增截图对照;未启动 Release 打包,125%/150% DPI 截图矩阵仍归统一
  人工验收。

第四十八个 P2 批次 5 回归基准收口切片（2026-09-01，自动化管线运行）：

- CL-06 将批次 5 的固定截图对照集整理为持久回归基准：
  `docs/audits/clipboard-batch5-2026-09-01/audit.md` 索引 CL-01~05 的全部迁移前后对照对、
  保护项清单（300×440 尺寸、8px 边距、滚轮/拖动测试、图片比例、96 项资源 LRU）与必跑静态
  门禁（ui-token-boundaries / clipboard-ui-lifecycle / ui-state-lifecycle）,并如实记录证据边界
  （空历史下菜单/悬停像素对照待补、125%/150% DPI 归用户矩阵）。
- 新增批次收口实拍 `final-clipboard-popup-batch5-close.png`（`e58458d`,同几何 316×449）,
  确认批次结束态与各切片证据一致。
- 批次 5 清单三项全部勾选;未修改任何渲染代码,无需红色门禁;clipboard-ui-lifecycle 三组契约
  复跑通过。未启动 Release 打包。

第四十九个 P1 浏览器扩展 Popup 色彩 token 切片（2026-09-01，自动化管线运行）：

- EX-01 新建共享浏览器 token 文件 `popup-tokens.css`,逐值镜像桌面
  `theme.rs::zed_dark` 语义色(canvas/surface 阶梯、border、文本四级、accent/accent-soft/
  accent-text、warning/danger/danger-soft),文件头注明与桌面主题保持同步;EX-14 的网页注入层
  将复用同一文件。
- `popup.css` 的 :root 旧私有调色板整体改为解析到 token(`--bg/--surface-*/--line/--text/
  --accent/--accent-soft/--warning/--danger` 等),两个头牌旧色(#0d0f11 背景、#53c7d8 accent)
  与 4 处青色 rgba 字面量(0.08/0.11×2/0.42)全部退役;0.42 分隔线与 0.11 光点光环改用
  `color-mix(in srgb, var(--nt-accent) …)` 由 token 派生,径向渐变内的青色 tint 同步换色
  (渐变结构的移除属 EX-02)。`popup.html` 在 popup.css 之前加载 token 文件。
- 契约测试先红后绿:`popup-contract.test.js` 的视觉系统测试改为逐值锁定 14 个 token、
  禁止 #53c7d8/#0d0f11/rgba(83,199,216,…) 回流、并断言 token 文件先于 popup.css 加载;
  原"#53c7d8 旧色锁"删除(兑现计划"更新现有 Popup contract tests,不再锁死旧青绿色"的约定,
  EX-09 的扫光动画断言保留至该切片)。
- 验证:`node tests/popup-contract.test.js` 通过;扩展全部 5 个测试套件(policy×4+contract)
  全部通过。本切片不涉及 Rust/C# 编译;扩展打包为整目录复制,popup-tokens.css 随目录分发,
  无需 manifest 变更;未生成 Release 包,125%/150% DPI 矩阵仍归统一人工验收。

第五十个 P1 浏览器扩展 Popup 背景 flatten 切片（2026-09-01，自动化管线运行）：

- EX-02 将 Popup 背景层全部改为纯色连续 surface：body 的径向青色 tint + 双层渐变收敛为
  `var(--bg)`;`.shell` 的 `repeating-linear-gradient` 竖网格删除,边框对齐
  `var(--nt-border-subtle)`;历史/资源行、悬停态、panel/settings-group、site-toggle 等
  6 处半透明面板(`rgba(21,25,29,…)`/`rgba(30,36,41,…)`/`rgba(255,255,255,…)` 全形)替换为
  `--surface-1/--surface-hover` 纯色 token;页面简介条、选中资源行与 feature-card 高亮的
  3 处 `accent-soft` 静态渐变收敛为纯色 `var(--accent-soft)`。
- 顺带清理指向已废弃旧色值的 alpha 字面量:旧 warning `rgba(215,168,90,…)` ×2、旧 danger
  `rgba(228,124,114,…)`、旧 warning-soft `rgba(85,58,25,.15)` 与输入框/页脚残留
  `#101316`、`#343d45`,全部改为 `color-mix(var(--nt-warning/danger) …)` 与
  `var(--surface-0)/var(--line)`。
- 红色门禁:`popup-contract.test.js` 新增 EX-02 契约——body 规则禁 gradient、禁
  `repeating-linear-gradient`、禁 accent-soft 静态渐变、禁上述全部半透明与旧色字面量,
  `.shell` 必须使用 token 边框;signal-sweep 扫光渐变按计划保留至 EX-09。先红后绿
  (红由缺失契约触发,实现中又先后暴露 2 处 accent-soft 渐变与 2 处 #101316 残留,
  均已清除并复绿)。
- 验证:扩展全部 5 个测试套件通过。遗留事项如实记录:popup.css 仍散布约 10 处小范围
  边框/文本/滚动条 hex 字面量(#343c44、#46515c、#364049、#071013 等)与扫光内的
  `#b5f4fb`,不属于 EX-02 的背景层范围,归入批次 6 后续 EX 切片统一收口。
- 本切片不涉及 Rust/C# 编译;未生成 Release 包,125%/150% DPI 矩阵仍归统一人工验收。

第四十三个 P0 设计令牌静态门禁切片（2026-08-31）：

- 新增 `tests/architecture/ui-token-boundaries.ps1`：除 `ui/typography.rs`、`ui/metrics.rs`、
  `ui/motion.rs` 三个令牌模块外，桌面生产 UI 源码不得出现字体家族字符串、原始
  `text_size(px(N))`、`line_height(px(N))`、`.rounded(px(N))` 或组件主题字号赋值；门禁同时锁定
  两个令牌模块的常量契约。先红后绿：同一规则对迁移前 HEAD 报告 62 处违规，迁移后为 0。
- 本切片只迁移"原始字面量"，不改变既有令牌值的视觉：等于令牌刻度的字号机械替换为
  `TEXT_BODY/LABEL/META`；偏离刻度字号（8.5–13.0px）按 DS-02/DS-03 语义就近映射——中文普通信息
  升到 Label 11（任务窗指标标签、速启设置行、剪贴板维护按钮、indexer 状态 chip），机器/元数据
  （FIXED/READY/PARTIAL、字节、路径、状态徽章、热键字母）归 Meta 10，13.0px 行名降为 Body 12。
  圆角 2/3/4/5/8px → `RADIUS_CONTROL`，6px → `RADIUS_FLOATING`，显式方角 → 新令牌
  `RADIUS_NONE`，76px 圆形状态徽章半径改为由自身尺寸变量推导。
- `main.rs` 组件主题引导改用 `UI_FONT_FAMILY`/`UI_MONO_FONT_FAMILY`/`TEXT_BODY`/`TEXT_LABEL`，
  字体家族字符串现在只存在于 `ui/typography.rs`；令牌模块新增 `TEXT_STATUS_GLYPH`（28px，
  完成/关机窗单字符状态符号，非普通文本）。
- `theme.rs` 旧剪贴板常量（`SEARCH_TEXT_SIZE` 13.5、`RESULT_TITLE_SIZE` 13.0、
  `RESULT_DETAIL_SIZE` 10.5、`META_TEXT_SIZE`、`COMPACT_RADIUS`，45 处引用）有意保留：它们是
  CL-02 的迁移对象，提前改写会破坏 CL-01 尚未采集的迁移前基准截图；CL-02 完成后应删除并并入
  令牌模块。
- 已知视觉影响：部分站点的中文文本按规则上升 0.5–2.5px，任务窗与设置行的真实渲染核对归入既有
  Release 100/125/150% DPI 截图矩阵；本切片未启动 GUI、未打包。
- 验证：新门禁通过；`cargo fmt -p nxtools-desktop` 稳定；`cargo check -p nxtools-desktop`
  （单作业、禁增量、256 codegen-units）通过且无警告。desktop 测试套保持既有 4 项已记录基线
  失败，本切片未运行完整测试套，也不把 check 冒充为测试结果。

### 批次 2：主壳层与全局设置

- [ ] 完成 SH-01 至 SH-08；
- [ ] 主标题栏、侧栏、模块页头和通知统一；
- [x] 全局设置迁移到 SettingsShell。

### 批次 3：下载全套界面

- [x] 完成 DL-01 至 DL-28；
- [x] 先主任务页，再设置，再新建/媒体弹窗，最后辅助窗口和覆盖条；
- [x] 每个子切片保持原有下载生命周期和协议行为。

### 批次 4：速启

- [x] 完成 QL-01 至 QL-12；
- [x] 保留现有 popup 尺寸和性能门禁；
- [x] 主窗口设置拆页后再统一自定义命令编辑器。

### 批次 5：剪贴板保护性迁移

- [x] 完成 CL-01 至 CL-06（2026-09-01 自动化管线收口,对照索引见
  `docs/audits/clipboard-batch5-2026-09-01/audit.md`）；
- [x] 每次迁移都与批次 1 的剪贴板基准截图对照；
- [x] 不改面板、预览和粘贴生命周期。

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
