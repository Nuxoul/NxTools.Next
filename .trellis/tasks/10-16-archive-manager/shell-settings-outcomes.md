# 文件关联与上下文菜单设置补齐 — outcomes

日期：2026-10-25。范围：本轮用户指定三项设置的源码与隔离行为实现；
**UI-AC01／02 保持独立 CLOSED，不重开、不重复验收。整体 UI 整改未完成。**

## 1. 参考选项 → 实际状态所有者 → 消费入口 → 验证

| 参考选项 / 实际控件 | 实际状态所有者 | 消费入口 | 结果与验证边界 |
| --- | --- | --- | --- |
| Windows 默认应用设置 / outlined Button | SettingsView pending/error；ArchiveApplication 原 pending map、关联 ID 和 IPC router | 固定空参数 `OpenDefaultAppsSettings` → C# 严格 parser/原 detached dispatcher → WindowsPlatformAdapter → DefaultAppsSettingsOpener | 已接通真实固定 `ms-settings:defaultapps` 操作；注入原生操作验证成功、失败、取消、日志及串行 reader 不被阻塞。测试不启动 Windows 设置；`opened` 仅代表 Shell 接受请求，不是窗口就绪。 |
| 在 NxTools 中打开 / 双击动作 dropdown | Core `shell_integration.double_click_action=open`，原锁内 save_settings/有序成功结果 | 原 ArchiveShellIntegrationCommand → ArchiveIntegrationPlan 的 owned ProgID open command → `--archive-open` | 规划真实消费；保存不改 UserChoice、不替用户选默认程序；实际关联应用未授权。 |
| 智能解压、解压到同名文件夹 / 同一 dropdown | `double_click_action=smart|name`；原启动意图、提取表单与策略 | owned ProgID `--archive-extract smart-dialog|name-dialog` → ArchiveExtractMode → 原 launch_entry → open_extract_confirmation/open_form | 原确认表单预选 Smart/Name，回归断言零已启动作业；不自动解压，不批准 Risk/输出预算，不改变目标确认/取消所有者。 |
| 打开、智能/当前位置/同名/对话框解压、测试、创建 / 七个 Checkbox + 全选/取消全选 | Core `menu_commands`；最多七个稳定命令，空集合保持为空 | 原类型化集成命令 → ArchiveIntegrationPlan/ArchiveSelectionHandlerPlan → 原 Apply/Revert transaction | 只规划选中命令；创建复用 Compress CLSID 和整组选择，ExtractDialog 使用固定新增 CLSID；纯规划/fake-store 测试覆盖，不执行 Registry/COM。 |
| 级联 / 平铺经典菜单 / dropdown | Core `menu_layout=cascade|flat`；旧 `classic_cascade` 仍是菜单启用字段 | 原 owned HKCU classic verb 规划；整组 IObjectWithSelection/IExecuteCommand helper 不变 | 两种布局均真实消费，保留身份租约/补偿/混合所有权；不宣称 Windows11 顶层菜单。 |
| 保存 / 请求 / 已应用状态 | 原 desired_revision 与 C# observed ledger/result | SettingsView 原 merge/save/publish → 显式 Apply/Revert/Status → validated observed facts | 新保存版本与旧 applied_revision 不同明确显示“尚未应用”；失败不声称已补偿，disabled 不伪装成功。ProductApplyAuthorized=false 保持不变。 |

参考入口与许可限制见 [archive-shell](../../../docs/reference-code/archive-shell.md) 和
[archive reference](../../../docs/reference-code/archive.md)：Bandizip 公开 SimpChinese.lang 的
IDD_SETTING_ASSOC/SHELL 只提供选项词汇，不是其闭源运行时/视觉证据；Archiver 的 shell.rs
提供 classic SubCommands/owned verb 形态，不能照搬整树删除或把 `%1` 当成整组选中。
原 NxTools 无通用归档设置所有者这一既有索引结论不用于省略本轮功能。
用户原始 PNG 未可靠读取，本轮未重试失效截图接口，不作视觉匹配结论。

## 2. 实现与保留契约

- 固定命令 wire：`{"command":"open_default_apps_settings","data":{}}`。Rust 自定义空对象反序列化和
  C# parser 均拒绝 null、数组、路径、URI 及额外字段。不改变 OpenShellTarget 的 HTTP(S)/文件限制。
- Core/wire 新字段：`double_click_action` 为 open/smart/name；`menu_layout` 为 cascade/flat；
  `menu_commands` 为 open/extract_smart/extract_here/extract_name/extract_dialog/test/create。
  缺字段保留旧 open/cascade/五命令；null/未知/重复/超限拒绝，显式空列表不扩大成默认。
- 仍由原配置锁执行改变字段合并；仅合并用户改变的嵌套集成字段，revision 从最新磁盘值递增。
  仍只发布原有有序成功保存结果，实体间更新 defer，不覆盖 UI-AC01／02 修复。
- 双击确认只在原表单初始化克隆的设置上预选策略，不改全局提取默认、已接受作业快照、
  密码/引擎/预算/风险确认、选择/拖出所有者；未加删除源文件或管理员解压能力。
- 原 P206 Apply/Revert 继续拥有原生状态；原 `OpenArchiveRelease` 验证保留 helper 身份租约后
  才生成 grouped 规划，无失败时退回单文件 `%1` 的假整组能力。
- ExtractDialog 固定 CLSID `306F1051-91FA-4D1B-87BC-E02E9D68FB46`；Create 复用 Compress
  `BC9707AF-8361-431D-82CC-36554BDDA04C`，其他身份不变。完整 selection 数组/一次 intent 不变。
- 原 transaction 写前全量预捕获、attempt-before-write、逆序补偿、compare-before-restore 不变；
  不写 UserChoice、不删除共享 Registry 树。ProductApplyAuthorized=false 在捕获/写入前拒绝。
- 新默认设置操作沿用同一关联 ID；Rust start/terminal 与 C# accepted/started/terminal 记录固定
  状态、elapsed/native code，不记录 payload。原 bounded pending/router timeout/disconnect 负责退出；
  重复/迟到结果不能复活请求；失败、取消、断线、超时不报告成功。

### 官方组件选择记录

要求形态：原 Settings 的“集成”页，选项为主、命令列表纵向排列，按钮可识别且保留边界。
锁定 Kit/Base/Component/Assets0.8.0，默认 gpui-pre0.3.9：原 Settings/Page/Normal Group/Item/Field；
`SettingField::scrollable_dropdown`、`SettingField::render` 公共组合、`Checkbox`、
`Button.outline().with_size(Size::Small)`（默认设置按钮含 loading/disabled）和官方 Alert。
菜单命令 Item 使用 vertical 公共布局，父层 flex-col/w_full/gap 安排实际控件，不自绘替代物；
不经会强制 ghost/compact 的 Toolbar.child。保留原设置搜索/滚动/焦点所有者。
生产视图隔离测试覆盖 Light/Dark、最小窗口可达及 dropdown 键盘选择；不等于原图外观、
物理 UIA/DPI/IME 或用户视觉验收。

## 3. 实际验证与证据

原始证据根（会话 scratch，不是发布产物）：
`C:\Users\Administrator\.pi-desktop\scratch\b83f39e9-16ef-4c5d-b436-4c52a8fda443\archive-shell-settings-20261025-091713`。
所有 Rust 检查均设置 jobs1/incremental0/codegen-units256，并自行检查共享通道和 cargo/rustc；
不要求其他会话暂停或预留。计数为各代表集合，不重复相加为一个“大矩阵”。

| 检查 | 实际结果 | 证据 |
| --- | --- | --- |
| 行为 red | Core 新旧默认测试实际 FAIL；smart/name 确认启动解析实际 FAIL | 01-core-red.log、03-route-red.log |
| 固定命令解析 red | 首次 Rust 空 struct 派生反序列化错误接受数组，测试 FAIL；已改严格空对象 visitor 后 PASS | 06-core-protocol.log → 07-protocol-ipc.log |
| UI red 尝试 | 被中间编译问题挡住，不冒充实际行为 red；后续 keyboard 测试原用不受支持 End，按官方 Up/Down/Enter 与 Ctrl+A/Backspace 修正测试 | 04/05、11/12、13 日志保留 |
| Core archive 范围 | 13 PASS / 0 FAIL；保存重开、兼容、非法值等 | 06-core-protocol.log |
| Protocol 固定命令 + 集成 | 1 PASS + 3 PASS；空对象/非法输入/边界/observed revision | 07-protocol-ipc.log |
| IPC 原 router timeout | 1 PASS | 12-targeted-retest.log 末尾；前部旧 UI 失败已由13替代 |
| Desktop 新生产边界代表 | 最终同一当前产物5 PASS / 0 FAIL，1203 filtered，10.70s；表单零任务、pending终态、CLI确认、实际控件保存重开/all-none、键盘主题最小窗口/失败保留 | 20-final-current-binary-tests.log；13早期5PASS亦保留 |
| 原请求边界代表（加强新字段断言） | 1 PASS / 0 FAIL，1207 filtered；smart/flat/[test,create]准确传送，disabled不伪报applied、重复/断线终态等；exit0 | 16-exact-request.log/result.json |
| Desktop 生产 cargo check | PASS；既存 animated dead-code warning | 08-desktop-check.log |
| Core+Protocol+IPC strict tests Clippy | PASS；本轮 single_match lint 已修正 | 15-layer-strict.log；14的旧失败保留 |
| Desktop strict tests Clippy | FAIL101：并发 ArchiveJobPrompt/audit 测试 job_tests.rs 中18处 unwrap，非本轮修改；不修、不加 allow | 14-rust-strict.log |
| Desktop 生产 strict Clippy | FAIL101：既存 floating_assistant/media.rs:62 animated 未读取 | 14-rust-strict.log |
| C# build + exact fake-only fixtures | 0 warnings / 0 errors；16 selected/run/PASS，0 skip/FAIL | 09-native-fixtures.log、09-native-fixtures-readable.txt |
| 文档/组件/发布/editor/Phase F 静态门禁 | 最终五项 PASS；spec114 docs/14 owners，catalog79；均仅源/文档/provenance检查 | 17-final-static.log |
| 任务上下文 / 限定格式 / diff | task82/79 entries PASS、保留8条过长截断警告；限定rustfmt/diff PASS；最终文案换行曾使19格式FAIL，修正后21PASS | 17/18/19/21 日志；不是完整工作区全绿 |

C# 精确集合：`archive association settings fixture`、`archive Phase F integration`、
`archive Phase F P206`、`archive Phase F P207 future registration plan unapplied`。
覆盖 strict JSON/pure plans/in-memory store/ledger/managed selection/injected no-argument operation；
仅 GUID 临时身份字节和隔离 ledger 文件，不操作用户设置/Registry/COM/Explorer/服务。
最终 Desktop artifact：`target/debug/deps/nxtools_desktop-5f49fb86e1516e0a.exe`；
16由Cargo编译并执行一条原边界代表，20直接执行该同一测试产物的五条本轮代表，
执行前后SHA256一致，见20-final-test-artifact.json；这不是正式产品启动或整工作区冻结证明。
最新格式调整仅换行、无语义改动。PowerShell把一次Cargo编译进度stderr包装成NativeCommandError，
实际Cargo exit0/精确test名/1PASS均保留，不能据该进度包装误报失败。


## 4. 基线、历史和范围

本轮开工保存897个文件 SHA256、opening status、原 checkpoint。保留所有已有 dirty/untracked
内容，未 reset，不用累计 HEAD diff 归属本轮。closeout 审计记录每个 opening 文件的变化、
本轮新增文件和明确并发路径；同文件并发变更标记 mixed，不宣称整个工作区被冻结。
原 negative acceptance、repair outcomes、independent acceptance 三份报告保持原字节；
唯一 phase-a-checkpoint.md 只在开头插入本轮状态，原1247行完整作为后续历史保留。
详见 scratch 的 final-baseline-audit.json 和 final-owned-source-hashes.json。初次收尾审计：
897基线、37已存在路径改变、3本轮新增路径；其中10条明确并发路径单列，shared/mixed不独占归属。
未删除任何基线文件；三份旧报告哈希一致；原1247行 checkpoint 后缀完整保留。
无有效原生结果时使用“应用状态未确认”，不承诺超时/错误一定未改动 Windows。

没有打包、安装、正式产品启动、实际 Windows 设置打开、Registry/COM Apply/Revert、
服务/updater/受保护部署/发布账本操作、UNC/源回收、提交或推送。没有依赖升级/vendor 编辑、
归档引擎改造，也没有修改本轮之外的浏览器工具栏/目录树/选择/原生拖出。

## 5. 交付状态与剩余边界

1. **默认应用入口：已实现并接通固定原生操作，隔离测试可用；真实窗口打开验收未做。**
2. **双击动作：已保存并由原生关联规划消费，smart/name 进入原确认表单；真实关联未应用。**
3. **菜单选择/组织：七命令与 all/none、cascade/flat 已保存并由原规划/补偿所有者消费；
   真实 Registry/COM/Explorer 验收未做，产品 Apply 仍禁用。**

最近归档历史、临时目录/保留期/退出清理留后续任务；原图视觉、真实 Windows 集成和既有
物理交互验收继续单独保留。上述源码/隔离测试结论不替代独立验收或整体 UI 完成。

**STOP：文件关联与上下文菜单设置补齐，等待独立验收。**
