# 全软件主题正式接入验证

## 基线

接入前备份 `a712ce7ca82f1ab574ceacb3d0ac792cf93cb83a` 已在远端main核对。后续接入保留本地，未提交、推送或打包。当前固定Kit/Base/Component/Assets0.10.0、gpui-pre0.3.12，批准的Component/Base/core/Windows实现例外为1/5/2/4。

## 功能边界

- Solid/Acrylic是独立风格；配色、System/Light/Dark及字体独立。原锁内重读、字段合并、持久化与延迟发布保持。
- 原29处窗口构造链及共享标题栏入口沿原准备/ACK/帧/显示/激活所有者接入；C#唯一拥有原生材质，稀疏宿主仍用Transparent。
- 公共Theme/Root角色覆盖正文、输入、浮层；25处官方Dialog/Sheet保留原状态与回调。About/Reader无私有固定配色，原字体和控件尺寸保持。
- Quick Launch/Clipboard主外壳使用每实例8DIP圆角和匹配子表面；原生区域随实际尺寸/DPI维护，原保存、resize、焦点与被动预览不变。
- Markdown标题定位在减少/未知动态效果下直接呈现结果，不启动装饰帧循环。

## 编译与行为结果

完整工作区命令覆盖14个第一方owner及固定编辑器，退出码0、失败0；4项受限测试明确过滤，原ignored保持。按测试组记录，组间存在重叠。

| 边界 | 结果 | 证据类型 |
| --- | --- | --- |
| Desktop完整二进制测试 |2228通过、1ignored|全软件原交互、准备/保存、临时数据与TestWindow|
| 固定编辑器库/story |234/6通过|完整工作区原编辑与示例测试|
| 归档实际读取链 |24通过|原typed IPC、独立ZIP、真实worker与原生清理|
| TAR/流再准入 |3通过|独立合成输入、实际worker/宿主与原生输出边界|
| 扩展传输整组 |16通过|实际Python/Node SDK、大Blob、原期限/信任和清理|
| 材质生命周期与表面 |119通过|原IPC回执与TestWindow|
| 两种主外壳圆角 |4通过|实际Root/子Quad、1.25合成scale、实体/焦点保持|
| 全局设置 |66通过|临时配置、官方控件、保存/取消及迟到结果|
| Markdown Reader |38通过|原文档、编辑与窗口行为|
| 标题动态效果 |4通过|实际Reader Quad与有限帧|
| 官方Dialog/Sheet |2通过|实际官方外层绘制|
| Quick Launch主准备 |10通过|原ACK/会话/取消|
| Clipboard主准备 |6通过|原材质/帧/显示屏障|
| Clipboard缩放与激活 |8通过|原尺寸保存、125/150合成DPI、激活后输入焦点|
| 原外壳布局 |2通过|完整客户区与原生缩放角命中目标|
| 辅助表面 |2通过|保留实体及稀疏透明区域|
| 下载整组 |363通过|连续事件水位/缓存、原真实HTTP重平衡、原窗口与输出|
| 下载事件投影 |3通过|实际事件入口、超前快照、缓存保留/重置；包含于下载整组|
| 扩展环境 |6通过|官方按钮、来源页返回、覆盖保存/继承恢复及信任/启用独立|
| 扩展页面整组 |60通过|活动排序/过滤、原Sheet、配置与环境行为；包含环境6项|
| About整组 |16通过|原界面/动效、真实IPC材质回执及关闭前后下载存活|
| 第一方全目标严格Clippy |通过|`--workspace --all-targets --exclude gpui-kit-markdown-editor -- -D warnings`|
| 固定编辑器生产库严格Clippy |通过|`-p gpui-kit-markdown-editor --lib -- -D warnings`|
| 全工作区格式 |通过|`cargo fmt --all --check`|
| C#测试项目构建 |0警告/0错误|编译|
| 原生圆角 |2通过|自有隐藏HWND、GDI区域、原事件与恢复入口|
| 原有界工作队列 |1通过|固定预算与同HWND顺序|
| 原生材质 |13通过|合成事实/回执及自有隐藏HWND|
| 共享HTTP fixture默认覆盖 |1通过|真实范围重平衡与最终字节|
| 正式GPUI source-over/UnderBorder |2通过|真实D3D混合与GPU采样|
| 正式GPUI窗口外框 |8通过|自有HWND、客户区/保存位置及原生状态|

GPUI来源/锁定版本、固定编辑器来源、12处显式透明创建加共享入口、官方弹层源码覆盖、132份spec导航及空白检查通过。Clipboard、预览隔离、Quick Launch和Desktop Shell生命周期门禁按当前调用链检查，保留延迟发布、身份/代次、原ACK/帧/显示及焦点约束。全工作区格式通过，依赖版本与条目保持检查前锁定集合。

下载连续状态事件仅推进实际消费的水位，缓存按当前任务状态与字节兼容性保留；权威刷新、旧序号拒绝与重试重置保持。原真实HTTP重平衡在8秒内通过，并纳入363项下载整组。[下载投影](download-followup.md)与[环境继承](environment-followup.md)记录对应边界。

About关闭/下载组合夹具显式绑定独有合成HWND，复用当前事实及原IPC回执工具后再点击真实关闭按钮；原主窗口和已接受下载的存活断言保持，生产缺失HWND/失败回执拒绝逻辑不变。

## 构建与目录契约

第一方生产、测试及示例均使用严格Clippy；固定编辑器生产库另行严格检查，完整工作区测试仍包括编辑器库与story。编辑器固定源码及批准的适配范围保持，历史示例的上游弃用别名不扩大生产源码补丁。

构建宿主绑定回归3项（含5个准备场景）、构建树清理回归8项（含112个工具桩子场景）、GPUI释放来源、编辑器来源与组件目录门禁通过。离线查看器实际DOM的警告、链接和摘要指向Kit0.10.0/pre0.3.12当前释放索引，79行历史目录保持。

完整Rust测试前准备独立TAR/流输入、Debug隔离worker/测试apphost及实际AC/OPT03导出，退出恢复原环境并清理本次目录。开发/测试的现有sha2库启用优化；Node原30秒期限和全部信任校验保持。归档定位标记只定位目录，不进入虚拟内容成员或进度所有者。

## 默认Solid、设置导航与剪贴板

Windows隐藏创建在第一次GPUI布局前读取真实客户区尺寸和原点，同步渲染目标。
官方Sidebar保留其原ListState，不再把CW_USEDEFAULT负尺寸缓存为overdraw。
普通自定义标题栏窗口将显式Client请求交给官方WindowBorder，最大化/全屏不画外框；
原生标题栏、Server请求和tool/PopUp保持原壳。原生resize样式、命中测试、C#放置和焦点所有者保持。

剪贴板主弹窗在合格材质回执和topmost完成后显式构建隐藏场景，下一应用轮重新校验身份、
代次、事实与回执，再进入原Show命令及ACK后激活。原20秒期限、关闭、取消和旧请求拒绝保持。

| 定向验证 | 结果 | 边界 |
| --- | --- | --- |
| 实际Windows GPU读回 | 1通过 | 隐藏生产设置视图；深/浅Solid各5行文字和左右官方外框像素 |
| 设置事务与布局 | 62通过 | 原草稿、输入实体、页面和Apply/Cancel保存链路 |
| 剪贴板主弹窗 | 7通过 | 无模拟首帧即可Show；ACK后激活；超时、过期和失效回执拒绝 |
| 原生窗口边界 | 8通过 | 自有HWND的客户区、保存边界、resize、最大化/最小化/还原与全屏 |
| Desktop严格Clippy与格式 | 通过 | 全targets `-D warnings`及workspace fmt |
| 来源、生命周期及文档门禁 | 通过 | GPUI1/5/2/4、编辑器57文件、Clipboard源码、132份spec、任务JSON及空白 |

深浅两张实际GPUI捕获已查看，五个左侧导航标签、右侧表单及普通窗口外框可见。
测试使用临时路径、内存引擎和断开IPC，未读取或改动真实剪贴板、历史和用户配置。

## 打包状态

当前未执行打包。默认Solid外框、设置导航和剪贴板主弹窗回归已完成源码修复及上述定向验证；新安装包和当前安装版本的实际交互仍未验收。现有自动化结果和只读环境预检不代替产品验收。已有独立7-Zip路径记录在[打包交接](../../../PACKAGING.md)。

Release构建、实际签名和安装器核验属于打包执行结果；当前未生成新的安装包，未安装或推送。

## 受限测试

完整工作区命令明确过滤4项会访问真实卷或启动生产索引器的测试：

- `the_real_windows_mft_boundary_is_either_searchable_or_actionably_denied`
- `the_real_windows_usn_boundary_observes_a_new_file_or_is_actionably_denied`
- `closing_the_desktop_pipe_stops_the_real_indexer_process`
- `an_oversized_real_process_frame_is_rejected_and_terminates_the_session`

这4项不计为通过；原本ignored的真实环境测试保持ignored。TestAppContext剪贴板、临时配置、合成历史、本地HTTP/pipe及自有子进程测试仍在执行范围内。

## 原生与视觉验收边界

当前自有HWND证据来自Windows11/96DPI。默认Solid设置的深浅色导航文字及左右外框已有实际GPUI像素证据，其余正式表面GPU边缘、拖动/Snap、IME、被动窗口物理点击、Win10、混合显示器、Release60Hz/功耗和用户视觉验收尚未完成。最大化分支使用隐藏窗口状态位读回；125/150及1.25scale是合成测试，非物理混合DPI结果。

测试使用临时配置、合成历史/本地HTTP和自有窗口；未启动普通产品或操作安装、服务及真实剪贴板/历史。下一验收范围按[实现计划](implement.md)的“阶段五：交互、回退与原生验收”执行。
