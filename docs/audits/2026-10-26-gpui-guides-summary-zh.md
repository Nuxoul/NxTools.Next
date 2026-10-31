# GPUI 指南审查整改：最终用户摘要

日期：2026-10-26。工作区：`D:\Code\NxTools.Next-main`。本轮仅整理交付文档，未委派、未启动新整改或验收。

详细英文项目报告：[Final engineering review](D:/Code/NxTools.Next-main/docs/audits/2026-10-26-gpui-guides-final-review.md)。逐项文件、证据与哈希：[证据索引](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-gpui-guides-final-handoff/evidence/source-index.json)。

## 一、结论：不是“全部完成”，也不是“全项目全绿”

- **既有有限父验收仍为 7/13**：#1、#2、#3、#4、#5、#9、#10。认可范围是当时交付的源码、编译及生产边界合成回归，不代表物理操作、Windows 原生、视觉、用户或 Release 验收。
- **另有 4 项局部实现并有有界回归**：#8、#11、#12、#13。没有据此新增全局验收，不能写成“11/13 已全部完成”。
- **2 项仍有实质缺口**：#6 ordinary-rem 部分完成，#7 cursor 尚未修复。不是只差最终跑测试。
- **最终综合验证没有准入**：Queue08 未完成、Queue09/10 未实施、Queue11 未修复。最终收敛阶段 Rust 调用 **0 次**、完整 Desktop 套件尝试 **0 次**；那一次获准的最终完整套件机会尚未消耗。没有新的 passed/failed/ignored/exit 数据，不能用旧 pass 填入。

最终准入原始记录：[admission outcome](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-final-convergence-admission/outcomes.md)。

## 二、原 13 项分别到了哪里

| 原项 | 已做的实际工作与生产边界回归 | 当前结论 |
| --- | --- | --- |
| #1 新下载 Enter/Popover | 限定默认 Enter 只属于原输入框；子 Select/Popover/IME/忙碌状态不误提交。真实 GPUI 按键回归8项、父跑 Downloads168项通过 | 有限父验收；真实 IME/原生仍待验 |
| #2 下载排序键盘等价 | 排序表头改为官方 Button，复用原排序/滚动/保存；实际 Tab、Enter/Space 释放、pointer、方向/选择/配置恢复3项通过 | 有限父验收 |
| #3 实际 Button 名称 | 名称落在真实 Button，不依赖图标/Tooltip/父容器；完整动作＋对象用途、禁用/忙碌/释放路径7项通过 | 有限父验收；不是 UIA/读屏认可 |
| #4 Archive 当前完整对象 | 原 ticket 携带完整 incoming/target 展示快照，长 Unicode 可滚动，缺失旧字段诚实回退；Dialog显示6项及原job11项通过，另有生产/协议/C#夹具 | 只验收展示范围；不改变覆盖安全策略或宣称混版本双向兼容 |
| #5 默认/聚焦确认键盘 | 原 Form/TimeField 与官方 Confirm→Dialog→defer reply；恢复原76行诊断并修正真实焦点/释放边界。audit05共15项通过 | 有限父验收；旧失败仍保留 |
| #6 ordinary-rem | Schedule、下载行/header、新下载基础/子层、Settings/task、共享控件有局部红绿回归；Queue08部分、09/10未实施 | **OPEN**；不能用代表性测试清除所有普通 px |
| #7 cursor | 已列出11个生产 cursor_pointer 声明及真实链接/文本/resize/drag例外，但没有有效 paint-hit 红灯和实现 | **未修复**；没有伪造框架 getter 或改 vendor |
| #8 命令措辞 | 按真实下一步补省略号，区分立即移除/取消与需确认删除，原动作不变；新增12项真实控件/假IPC/取消回归通过 | 局部交付；MainWindow206通过/6失败及全局检查限制仍在 |
| #9 picker 身份 | 原应用绑定 view/window/operation/revision/currentvalue，双阶段 defer 复核，防 close/reopen/ABA/跨窗口回写；picker14＋原键盘8通过 | 有限父验收；真实 picker/HWND 待验 |
| #10 Clipboard 身份/焦点 | 原 ListState 使用不可变记录ID＋动作种类，保持实际 FocusHandle；prepend/reorder/remove/armed release/新会话11项通过 | 有限父验收；没有真实 clipboard/paste |
| #11 下载页复制 | 原所有者共享不可变任务快照，索引投影与ID护栏，真实 uniform_list 范围内才读进度/图标并借用行payload；新增5项及相关组通过 | 源码/编译/合成局部闭环；不是全局性能或Release验收 |
| #12 详情 IO | 原下载 runtime 后台执行实际 TaskDetail/SQLite，最多1物理运行＋1最新待办；当前scope/generation/correlation等限定发布，真实窗口/选择/失败重试9项通过 | 局部闭环；未把其他同步IO误称全异步 |
| #13 Quick Launch 文本 | 原Panel持有完整SharedString展示投影，render克隆handle，hover保留同一旧快照；编辑buffer/协议/领域/完整执行payload不机械替换。新增5＋原121＋polish11通过 | 局部闭环；没有补完Queue09 rem或#7cursor |

各项具体修改文件、原问题、真实回归入口、检查结果及 S（source）/C（compiled）/Y（synthetic）/P（父验收）等级，均逐项列在英文报告第3节。初始问题表按原PRD/父验收重建，不冒称另找到了完整原始审查逐字稿。

### 性能测量的真实含义

- #11 在实际1000/10000条生产投影中，深复制任务payload从1000/10000降至0；观测批次真实构造行仍为26→26（原本已虚拟化），行payload复制26→0，进度/图标读取分别2000/20000→26。**26不是通用可见行上限**，含框架测量回调；没有声称零遍历、零排序或GPU/RSS提升。
- #12 实际 SQLite mutex 红灯中，下一UI turn不能先运行、由3秒watchdog解锁；修复后下一turn先运行、无需watchdog，再等实际结果。0ms为四舍五入观察，**不是响应时间SLA**。
- #13 重复真实render时，动作目标标题backing allocation **6→1**、结果副标题 **18→1**。只证明长展示文本稳定共享；初次构建、引用计数、高亮、布局和实际事件payload仍有成本，**不是整个渲染零分配**。

## 三、仍然阻挡完成的事项

1. **Queue08**：Extensions catalog vector28、确认summary/technical80/88普通角色未修；Currency/Floating完整rem消费端/末控件/错误取消矩阵没有完成。
2. **Queue09**：Quick Launch/Clipboard普通字号、间距、栏位和测量角色没有实施；当前#13文本共享不能替代rem整改。
3. **Queue10**：Archive普通行、Tree、toolbar、Textarea、viewport、列持久化与PageUpDown耦合等尚未闭环。
4. **#7 cursor**：尚未修复，仍缺真实当前帧paint-hit观察和链接/文本/resize/drag例外回归。
5. 最新记录的 **workspace fmt仍失败4个文件**；生产目标严格Clippy仍有`SPACING_12`、`DecodedMedia.animated`两个既有问题。**测试目标严格Clippy和workspace check最近通过**，不能把两者混成“当前编译错误”或“所有检查全绿”。
6. 完整Quick Launch source gate仍在 **QL-11第458行**缺`CONTROL_HEIGHT_INPUT`字面量处停止。发布委派链目标检查100项通过，不等于完整gate通过。
7. #8广域MainWindow **206通过/6失败**缺少后续收敛结果；本报告不重测，不断言这6项目前仍复现或已修。

本轮没有自动新增整改、队列、worker或后续任务，以上仅列真实遗留。

## 四、残余 px 如何解释，哪些才是批准例外

不能把“还有px”全当bug，也不能把旧几何测试通过全当豁免。准确依据：[角色矩阵](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-rem-archive-final-matrix/role-matrix.md)。

- 247文件/2882个直接px调用是**词法导航清单**（含2028个测试调用），不是2882个缺陷；间接单位仍需逐角色判断。旧v1解析错误已标为过时准备，不能当验收。
- Archive158个直接调用中104测试、54生产；生产里16原生client、3批准Tree宽度、23普通候选、4持久几何未定、2viewport未定、6其他未完成。**35未完成不等于35已证实bug**。
- 批准保留的边界包括：Archive固定client/Tree200且160..280范围；原下载/NewDownload/Settings/task/title/footer/native预算；Main1060×650/min920×480、caption36/rail176/controls36×36；Tools180×160tile；Quick620×44/410与被动400×410preview；位图/细线/实际native/hit坐标；官方Form/Field、Switch、Progress内部混合单位。
- Clipboard最新批准为文本卡cap120..320/default160（不是历史210）、image190/media144/file62/gap8/source24、仅溢出正文feather14、黑8%阴影offset1/sigma1及原source-icon渐变/完整payload。**整个Clipboard文字/栏位并未因此固定豁免。**
- Archive普通row/toolbar/Tree缩进、持久列/viewport等未知角色仍是UNFINISHED，不能创造新固定例外；Root字体14/16/18也不等于Windows100/125/150%DPI。

## 五、旧失败与外部改动没有被“洗绿”

- 历史完整Desktop仍为 **1237通过/1失败/0ignored，exit101**。UTF-16原始日志SHA256：`7753e62dcaa29bb39f7ea683db082ceb6cc97485e48220b24d486cc7551d8c48`。
- 更早MainWindow **185/1失败**与#8 **206/6失败**保留；曾有的历史1196/1196或1232/1232也只属于当时worker交付，不是本次最终综合结果。
- Archive搜索问题已用seed3证明是错误TextInput定位，测试修复后0–7通过；不是产品搜索/保存重写。
- Extensions Cancel受控实验确认进入动画让按钮移动21px；业务夹具稳定化＋正常动画独立回归已通过，但**没有证明历史1237/1恰好就是这个时序**。
- 原第1765行安全复合条件74项中，仅build直接包含writer的过时假设不满足；实际`build→publisher→writer→Host`链已核实，原73安全谓词保留并逐项变异拒绝。没有据长报错宣称产品被攻破，也没有改认证/ACL/hash/三二进制tuple/超时策略。
- 早期#6取消、编译准备失败、后续磁盘/PDB/API/fixture错误、blocked检查点和不完整结果全部保留；后来的单项pass不覆盖旧manifest。
- Queue01生产编译修复、独立Extensions改动、#4外部格式片段和#11Cargo.lock工具链归因分别记录，不冒领为本会话own-delta。
- 早期#1实施记录中的错误“要求其他会话预留Cargo”请求已被用户否决并撤回，未实际暂停/取消/终止进程；这一过程偏差也保留披露，不能被后来的自然等待合规掩盖。

## 六、待用户单独授权的验收：本轮全部未执行

先固定源码/二进制/清单SHA256、测试账号/VM、合成数据和回滚清理范围。每项需明确允许的进程、窗口、路径、设备、显示设置和OS副作用；缺环境记BLOCKED/SKIP，不记PASS。

| 待验类别 | 具体验证步骤 | 必需授权/注意 |
| --- | --- | --- |
| 物理键鼠 | 实按Tab/ShiftTab/Enter/Space、hold/repeat/修饰键、按下后转焦点再释放；子Popover/Select/Dialog、disabled/busy；排序方向/选择/配置恢复；picker ABA/close-reopen；Clipboard row重排/删除时的焦点和旧release；子菜单/Escape | 仅指定owned窗口/合成对象；真实下载/归档/粘贴/Shell另批。必须核对一次真实动作，不用直接业务方法替代 |
| Windows IME | 微软拼音候选/预编辑Enter与Space不误触发；commit/cancel后下一次普通Enter恰好执行一次；在Input/TimeField/password/multiline/Select/动作模式间换焦点及close/reopen | 真实IME与物理操作授权；合成marked-text事件不能替代；不得记录秘密输入 |
| native focus/HWND | hidden创建→NOACTIVATE→最终device-client bounds/readback→frame/reveal；重复preview复用同一HWND；真实点击预览不抢primary输入；show/hide/close/reopen/晚ACK/断连/超时/旧generation不作用于新窗口 | exact binary/PID/HWND与C#精确native filter授权；primary仍需激活，不能全部设nonactivate。返回码检查不等于物理焦点验收 |
| 100/125/150及混合DPI | 每档默认/最小窗口、长CJK/path/error滚到底、footer/action可达、虚拟row/page步长；跨不同DPI/负坐标/taskbar边缘移动primary和preview，含pending与close-reopen | 允许指定显示器/缩放修改及恢复；记录实际HWND DPI/workarea/client读回。Root14/16/18不是替代品 |
| UIA/reader | 实际provider里Button名称/role/state/disabled/focus/value；真实Tab事件；完整冲突对象/动作文字；Narrator/NVDA读屏与modal/Escape返回、不因被动preview乱报/抢焦点 | Inspect/Accessibility Insights/读屏器运行和树/音频隐私授权；测试语义快照、Tooltip或父label不能冒充真实provider |
| 视觉/用户 | Light/Dark/System、默认/最小、各状态/长文本/DPI；核对层级、中文可读性、按钮边界、对齐、完整对象、固定预算；用户亲自操作并逐面记录接受/拒绝/阻断 | 明确人工reviewer与截图授权；测试截图或AI检查不是用户同意；不能自行调色/加大窗口掩盖问题 |
| Release/installed/service | 前置整改和质量收敛后，单次完整合成集成；另批签名/sequence/commit-headroom/package/install；exact路径与受保护Desktop/Host/Indexer atomic tuple/hash/ACL验证；普通用户启动升级回滚；授权LocalSystem重启后idleCPU+真实service search | 独立签名、UAC/Registry/SCM/安装/重启/测试卷/查询授权，最好VM。只操作exact package进程/服务，不按同名杀进程；Ready或sourcegate不是release通过 |

特别注意：现有Quick Launch UI smoke固定输入`q`，含全局键鼠/窗口消息，默认会停止精确匹配的Desktop/PlatformHost，且没有roots/query参数。它不是只读service查询，也不能直接在用户运行中的产品上执行；需要先批准隔离的真实服务搜索方案或该UI smoke的全部具体副作用。

可用但**本轮没有执行**的既有入口：[owned native wrapper](D:/Code/NxTools.Next-main/tests/native/gpui-kit-acceptance.ps1)、[C#效果分类规则](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost.Tests/README.md)、[Indexer idle CPU](D:/Code/NxTools.Next-main/tests/performance/indexer-idle-cpu.ps1)。所有精确步骤、验收准则和失败处理见英文报告第8节。真实服务测试先核实当前脚本参数/范围，不直接扫描用户文件。

## 七、本轮交付与保护

- 只新增英文报告、此中文摘要及新的报告证据目录；没有修改产品代码、旧outcomes或旧manifest。
- 写入前冻结 **8045个Git可见regular文件**，包含既有dirty/untracked；最终逐字节哈希核对。Git-ignored构建产物和外部仓库不在该保护声明内。
- 本轮只运行文件/链接核对、`spec-contracts.py`、文档scoped diff/空白与只读完整性检查；最终退出码和日志哈希见[checks-final.json](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-gpui-guides-final-handoff/evidence/checks-final.json)。初次[检查记录](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-gpui-guides-final-handoff/evidence/checks.json)的FAIL保留：先修正UTF-8读取，再通过干净/故意尾随空格夹具确认Git no-index返回1/3的差异语义，未覆盖旧结果。文档通过不代表产品通过。
- 未委派、未添加后续队列、未启动整改/编译/完整Desktop/native/服务/产品/真实用户数据操作；未提交推送、未package/install。

**交付到此停止，等待用户决定如何处理遗留整改和单独授权的验收；不会自动继续。**
