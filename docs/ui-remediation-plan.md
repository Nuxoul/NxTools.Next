# NxTools.Next UI 整体整改计划

> 状态：P0 实施中
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
