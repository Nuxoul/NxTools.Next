# 第五步 HZ3：批量任务汇总 — 实现交付，待独立验收

2026-10-29。仅用户原七步第五项，仍为 **Phase E — Format expansion and ZIP editing**。第四步及EIC已独立限定接受，不重做；原A–F标题/顺序不变，不标整个PhaseE/A–F/产品/发布完成。**本轮交付后停止，等待协调会话独立验收；不自动进入第六HZ4/第七HZ5。**

- [实现前路线/诊断与不变量](hz3-route.md)
- [精确验证矩阵、命令、计数与失败历史](hz3-matrix.md)
- [可执行跨层owner契约](../../spec/nxtools-desktop/backend/archive-batches.md)
- 本轮证据 `$H`：`C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\hz3`。

## 1. Opening、参考与边界

当前任务派发message `4a1bd2d2-44e3-42c6-bbd3-c5fb6b4373e1`明确用户继续后仅执行第五步；顶部旧LIMIT_EXCEEDED记录保留，不误认为本轮未授权或重发绕过。原七步附件已读。开头先保存唯一checkpoint完整 **194042 bytes**，SHA256 **5CBED44CE726DE8CFD4B76E0965765A4F236298DF1C8B9073896E881EDC4F248**，包含原通信拒绝前缀、EIC独立接受和全部旧历史；`$H/checkpoint-before.bin/json`。HEAD/status、25项入口/队列/native/engine/Cargo/验收历史哈希保存在opening文件，后续只prepend且证明整个opening exact suffix。

先读当前checkpoint与EIC接受、适用Trellis/shared/UI mandatory component selection/current080/table/API、原Job/Queue/Form/attempt/readmission/result-location/native生命周期。HatchZip先读batch-summary/source-manifest，再核对实际 `D:\Code\hatchzip-main`：
- CLI main.rs233distinct、501extract_all、578summary；
- GUI runner69give_back、103start_queue、115step、136next_in_queue、219may_close；
- task_window39job_for/77build、work228extract；
- CLI several.rs全部259行测试定义。

五文件与原source-manifest SHA256均一致，`reference-hashes-final.json`；参考本地包0.1.3，无gitmetadata，**不编造commit**。未运行参考CLI/GUI/测试，未改参考归档或源码。仅借用逐归档结果/未开始表达；不搬串行Runner、遇错停止、pathHashSet去重、journal恢复、源空间回收或clock。新增核对的NxTools入口已补参考索引。

## 2. 实际交付

### 原Jobs上的批次关系，不是第二队列

原Jobs/JobRecord新增private batch metadata；batch UUID、logical job UUID、active operation/attempt UUID各自独立。`batch.rs`仅官方控件投影，`batch_jobs.rs`是原ArchiveApplication的私有实现分块，不拥有进程/队列/Windows API。成员仅引用原logical ID，不复制请求/密码/执行状态。

原 `submit_batch` 和multi-request表单把成员绑定到一个批次；多选启动确认的原表单先记录待确认form IDs，提交后挂原logical jobs，取消保留“提交前取消/未开始”而不造任务。原逐项确认语义保持。提交预检仍检查整批格式/结构/容量/内存；同步中途拒绝会复位defer_pump并保留真实accepted/notstarted事实，不虚报全部接收。没有新stop-on-error，没有取消已运行兄弟或阻塞无关批次。

**queue.rs字节未变**：2running/32queued/2GiBaggregate、FIFO/head-overlap、同归档/输出重叠序列和cleanup-held slot保留。原入队/内存/重复/overlap测试包含在完整archive回归；接受不等于开始，receipt/private Complete不能代表执行结束。

### 汇总状态与终态事实

投影原record的排队、正在准入、实际运行、等待交互、暂停、成功、警告、失败、取消；另独立显示未开始、待确认、提交前取消、清理未确认、有实际发布。原worker Started记录ever_started：占执行槽/准入不是Started；排队取消同时保留Cancelled和neverstarted。archive任务数与native文件提交/跳过数分开，未知bytes不合成总体百分比，全取消不叫成功，warning/failure/partial/unknown不会被“整体完成”掩盖。

原finite output readmission和create后Test仍是一个logical member的多个attempt。新增重试保留logical ID，new operation UUID/generation；重复/stale/native旧attempt终态不污染当前成员。原finite coordinator回归已加“新attempt后成员仍1、逻辑成功只计1”断言并PASS。

### 官方组件、原详情与控制

原JobProgressView同表面加入“批次汇总/任务详情”切换，官方Kit0.8.0 stateless Table/Header/Body/Row/Head/Cell+Label/Alert；不是新的DataTable/controller/HWND。详情按钮仍 `show_job`，停止仍 `control_job`，定位结果仍原详情的 `locate_job_result`，不从summary新路径造authority。原prompt优先，summary不盖密码/冲突，原focus/typedticket/secret清除/取消和HZ2刷新owner保持。

决策按钮直接layout的官方 **outlined Medium32**，不经Toolbar.child变ghost/compact；原Root/titlebar/Graphite/字体、一个bounded scrollbody、flex-none footer保留。pointer、Tab/Space、disabled/no-dispatch、原480x260最小窗口已合成渲染验证。首个原详情承载批次入口，其余单任务保留原keep-open/完成关闭政策；用户关闭只是presentation，原通知可恢复结果。没有永久动画/新clock/新全局控制器。所有跨entity更新继续下一turn，包括即时观察结果。

### 合格重试：显式原生重验，不盲重放

通用summary retry刻意限定为**单源Test或非选定Extract**，并要求：原logical job已Failed/Rejected；cleanuptrue；files_committed0；journal各项明确unpublished/precommit；known precommit state及有限retryable code；**首个终态前**取得的有效native source/target观察基线。缺baseline或无法证明零发布不可重试。

点击后先置retry_pending，沿原ArchiveSpace再观察expected_source/expected_target；匹配旧source_set与targetidentity才同logical ID重新enqueue原队列，newattempt/newgeneration一次派发。重复点击/响应不二次提交；原worker/native仍重新准入source/capability/target/budget，不暗加预算或放宽权限。源变/目标变、unknown/已发布/未清理、创建/编辑/选定条目或需重新确认的草稿，保持disabled并走原表单/重新打开，不保存/复用密码。不是自动retry或新recoverymanager。

### 原native source-set观察与多选去重

原ArchiveSpacePreflight本来就通过 `ArchiveVolumeSources.Open` 和 `ArchiveOutputSession.Prepare`只读持有来源/输出namespace。显式 **ArchiveSpace version2**新增optional private source_set {identity,primary,members}，v1请求响应形状不变，Rust/C#对称strict/default/bounds。身份ASCII<=256、1..1024唯一local成员、primary及本次source必须在集合、aggregate UTF8成员bytes<=1MiB，UI encodedresponse<=2MiB并绑定receipt/connection/session/generation。身份/路径仅私有上下文，不进diagnostics或写权限。

同次launch多选先沿原观察取得真实源集，再进入原确认表单/submit_batch；只有同一native identity或成功验证的完整family覆盖才能去重。后续卷单独拒绝时，必须被同次其他已验证family覆盖，否则精确拒绝，不按basename/case/后缀猜合并。同名不同目录保留不同归档。原volume parser/nativelease/源保护/workercodec未改；观察不创建目标、不解码、不修改源件。原执行时仍重新准入，观察不是源集快照或额外执行authority。

### 有界留存与低噪诊断

批次关系最多保留64个已完成group，配合原retired64；active/cleanupheld/retrypending/待确认form关系不错误淘汰。待确认关系完整初始化后才prune，避免新空批次在边界被回收。不增加persistent config/跨重启history承诺；原active/native所有权照旧。

实现前路线定义诊断，现复用原tracing/sinks/correlation：batchaccepted/memberfinished/batchfinished、explicitretryaccepted与原job start/state/terminal elapsed/nativecode。终态日志含batch/logical/attempt/generation和固定分类计数，不宣称aggregate成功，once-only避免重复；不每chunk/frame/pointer记录，不log路径/成员/content/password/headers/cookies/requestDebug/exceptionDisplay。未引入日志/通知clock。

## 3. 验证结果

[精确矩阵](hz3-matrix.md)列每条test/name/argv/log/exit/skip及失败分类，`command-ledger.json`和 `verify-final.ps1`保留完整展开命令。全部Rust命令先检查共享cargo/rustc/clippy idle，再同process设置 CARGO_BUILD_JOBS=1、CARGO_INCREMENTAL=0、RUSTFLAGS=`-C codegen-units=256`，locked/offline/serial/充分timeout；未占道/killname/重复build。均普通incremental，不是cleanbuild。

| 本轮检查 | 实际结果 |
| --- | --- |
| Desktop完整archive | **342PASS/0FAIL/0ignored/1167filtered**，原332+10新HZ3回归，HZ1/icon/WAC/HZ2/edit/launch/原queue全部保留 |
| finite readmission额外批次断言 | 同原生产coordinator test **1PASS**；full342后只加测试断言，产品未改；其后strict再PASS |
| Protocol全量 | **90PASS/0FAIL**，新v2strict/default/bounds1项 |
| IPC全量 | **43PASS/0FAIL**，原bridge reservation/cleanup/whole-selection输送保持 |
| Core兼容 | lib93+integration6+1=**100PASS/0FAIL**，Core源码未改 |
| Archive定向lib+10targets | **98PASS/0FAIL/0ignored/0filtered**，原EIC4/14、TARraw4/20、ZIP/7z/TAR候选/handoff/privatewire保留，不冒称full45target |
| affectedDesktop+Protocol严格Clippy--tests-Dwarnings | PASS，无suppression；最后测试断言后再次PASS |
| workspace--tests check | PASS |
| 当前isolated-codecs worker build/freeze | PASS，freeze与Cargo生成exe同hash |
| C#incremental tests build | PASS，**0warnings/0errors** |
| effects→exact21LIST→同nameRUN | **21selected/21run/21PASS/0SKIP/0FAIL/0not_run**，一完整扫，非LIST代RUN |
| scoped15Rustfmt/19source diff | PASS；最终docs/checkpoint门禁另记 |
| spec/component/release/editor/test-selection | PASS，spec118docs/14owners；只代表source/static |

原 `output_readmission_chain`及另外三独立specialtychains本轮未选择，不复跑旧失败寻绿。父窄轮80PASS/1旧SourceChanged-vs-Cancelled失败、原Step4/EIC负面探针、source_unavailable/helper_untrusted/FB-AC05及其他未验收层继续保留。无100k/performance/fullnative/physicalGUI主张。

## 4. RED→GREEN和全部失败历史

- 原production三jobs/twoStarts后的rendered入口缺失：entry-red **0PASS/1FAIL**→entry-green-v2 **1PASS**。
- 原C#held-observation显式v2原本拒绝：source-native-red-run **1FAIL**→同名source-native-green-run **1PASS**；不是新unsafe nativeowner。Protocol插入初次工具拒绝造成selector0tests不算RED/PASS；一次busy lane未启动命令。
- completed-window batch metadata超64是真实retention失败，修正只回收completed/cleanuptrue且非active/held/pending，后续PASS。部分早期lifecycle fixtures直接调用terminal跳过原bridge cleanup或丢endpoint，产生not_submitted/断连；修fixture走原deliver/receipt，不改queue/assertion。
- 汇总真实内容缺可访问文本快照/身份：productionrender失败后，沿原Label语义包装修正，最小窗口/disabledkeyboard回归PASS；不是伪造测试内容。新enabledretry最初同名选择命中disabled兄弟，修测试定位实际enablednode后原pointer一次重验/一次newattempt PASS。
- syntax/trait/ElementId/modulevisibility/missinglegacyNone、fullnotificationchannel等compile/fixture失败均保留；strict-first4lint错误用空格/then_some/collapsedif修复，无压制。它们不冒充behaviorRED。
- 原outside-archive并行 `SPACING_12`/animated warnings曾出现在普通编译日志，未顺手改；最终affectedowner严格tests lint PASS不等于宣布所有其他owner无warning。

只有一次有界自查及具体失败修复，无新增review/subagent/广泛重审循环。旧报告未改，复测PASS不抹除历史失败。完整轮计数和各早期窗口在test-counts.json与矩阵；不把不同集合相加冒称同轮全量。

## 5. 匹配worker与独立工具身份

`$H/frozen/nxtools-archive-worker.exe`，**6CD1C363895DB88BFD741DD5B8924D076E018C1734184DC7CC31E5567F14C0E1**，11513344bytes，与本轮Cargo生成worker同hash。worker产品源码/acceptedEIC引擎未改，但Protocol图有变化，所以仍构建当前worker后freeze，native显式指定新文件/marker，不用旧8831冒充当前build。

环境/精确name/effects在native-final.ps1；NXTOOLS_ARCHIVE_WORKER_FIXTURE与ZIP/TAR/7z/EIC markers匹配，原Python/stock7Zip/bsdtar/ZIP+codecfixtures/TARoracle/volume scratch/evidence root均显式设置。21native检查只owned GUID文件/原heldlease/小候选/已有injection/exactownedchildren，不是产品或外部工具新backend。

| Identity | SHA256 |
| --- | --- |
| **actual C# Tests.dll** | `A2EBE89E7F8F311A09BF3C869EDDDF7FA3B45626FFCC5F678321EDA88378B65E` |
| PlatformHost.dll | `AA55B3472D32C2C6E67FC066D5769782C76FE30374629DE46169A66AFACFC817` |
| CPython3.13.2 | `7D96A4ED35D6E596CD9DD8933FEAFC66349CC21F75BE3B15C89FD336E50140C1` |
| stock7Zip26.00（非ASCII路径从UTF8manifest读取） | `4BCC005758C5E604C0AAA257B6097846A4A708E62A9C0C7471FA360FAD863F1F` |
| bsdtar/libarchive3.7.7 | `92FAFED485AF0FE4C291457DC3C00D0241F6A136731BAB63EF18A3F0C949D37A` |

完整paths/bytes/hash见identities-utf8.json，不用exe apphost alone作为tests内容身份。原ZIP/TAR/7z独立解码/metadata和新观察源集测试均在对应ownedfixture层；source-set小分段family fixture仅证明native识别/lease，不冒称新codec互操作或powerloss/HZ4。

## 6. 本轮source/protected drift与交付文件

opening25项中7项预期变化：Jobs/Progress/Form/Launch/Readmission/jobtests/archive.rs；**18项完全同hash**，opening-drift.json。原queue.rs、application.rs、EIC worker_zip_edit/worker_output、TARrawengine/ZIPrecordengine、C#JobSession/Runner/SourceLease/OutputSession/VolumeSources/CreationSources、Cargo.toml/lock及opening收录的EIC/Step4验收/结果历史均不变。其他未收录opening文件不伪造本轮逐字节比较证明；原TAR/7z报告未编辑。

19个交付源码/测试路径（其中5个legacyfixture文件只补optionalNone/保留原行为，新增batch/batch_jobs是原owner分块），完整bytes/hash在delivery-source-hashes.json：

| Source | SHA256 |
| --- | --- |
| `rust/apps/nxtools-desktop/src/archive.rs` | `0D38274186CE5B6655D2402D92F9C24D0C7D0043BDE577E4195A77D64064A8CA` |
| `rust/apps/nxtools-desktop/src/archive/batch.rs` | `DA2CF6DFD5766312AEECEE4D966C449A12ED27B5367AE7DEF448C53DF34780F1` |
| `rust/apps/nxtools-desktop/src/archive/batch_jobs.rs` | `AD1D6013EE1D5FD59A97E3843D7BAB7580D14B479DF4B12ADB47787A754D7FE7` |
| `rust/apps/nxtools-desktop/src/archive/jobs.rs` | `0892ECC5A8FFE6727CB51FF30324E2FB3B3D757E3402C9EE316DE3F562054587` |
| `rust/apps/nxtools-desktop/src/archive/progress.rs` | `6BE39FCA2FA388D12FCE42572A4A0406A78C906C0E13D30162E13F2093043640` |
| `rust/apps/nxtools-desktop/src/archive/job_tests.rs` | `1D48BCD7DE35DCD20127B51B20CF6EADF691406F53BCE409F0875EA364EDAC9F` |
| `rust/apps/nxtools-desktop/src/archive/forms.rs` | `1248FDD4F77E11063AAF3DCEE6C9241EB8D58CBDEC1CEF75E51AEBA1384BB62C` |
| `rust/apps/nxtools-desktop/src/archive/forms_tests.rs` | `0187B28C82174914DE71079DEACB155B4AFFF8A60712F145C4E0636CA2ED91A6` |
| `rust/apps/nxtools-desktop/src/archive/hz1_lifecycle_tests.rs` | `5E7B5D830D1872FEB21CC7031D3649B1DC0BBC7769A533B46465C0293F0E15A7` |
| `rust/apps/nxtools-desktop/src/archive/space.rs` | `7F332D30BB207AD297FFDA0F28264477E3F6233CEBFA7BE9998ABA89EF0281A3` |
| `rust/apps/nxtools-desktop/src/archive/space_requests.rs` | `78AB98F7FB750B6E28F8B09E6AA32845814BBB8ACE3F35216EE06091EC74B583` |
| `rust/apps/nxtools-desktop/src/archive/readmission.rs` | `9570FF111DE20878FC70D407C02B15DDE02E3980D402A8EB22AA368EC2F89A7E` |
| `rust/apps/nxtools-desktop/src/archive/launch_entry.rs` | `95D5600470B47E1B96080F36E2CB967590DBCC60D0FD660A4F51AF56082115C6` |
| `rust/apps/nxtools-desktop/src/archive/progress_feedback_tests.rs` | `73555660E8E2BC8D48C40F77ADCC9994C976B5961AED8B422847A2331E94F940` |
| `rust/crates/nxtools-protocol/src/archive_space.rs` | `27ECB8E998D15062D133598D63253AA8BF1AF7A7E7050E4C32F4E1799136DAF5` |
| `csharp/NxTools.PlatformHost/Protocol/ArchiveSpaceContract.cs` | `A1D2DD2F29B22CC301091ACB99E496681E181AE031FC67D484DD54B8E6C54134` |
| `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveSpacePreflight.cs` | `A6ABDF0A510B74B391D5700A4C93A21A4E3C42D6CC893E8F982EE3965B3029DA` |
| `csharp/NxTools.PlatformHost.Tests/ArchiveSpacePreflightTests.cs` | `35F2F446DD2C723C4FA2F5E6F00573276F7FC7277C9FE9AABDCB80A25EC658BD` |
| `csharp/NxTools.PlatformHost.Tests/ArchiveVolumeSourceTests.cs` | `1865B4B4F8F8CE302B6515E5F1578F815218C82ACB22E782C7CA68EDF422BC1F` |

文档新增route/outcomes/matrix/ownerbatch七节契约，必要Desktop/Protocol/nativeowner索引与HatchZip参考说明增量；不覆盖原阶段/旧验收/负面探针。唯一checkpoint仅topprepend，最后整个194042bytes opening必须是精确suffix，大小/新hash/证明在 `$H/checkpoint-suffix.json`。大量既有/并行dirty不归本轮、不reset/stage/commit。

## 7. 未执行、限制与停止

本轮source/compiled合成GPUI/owned-worker/nativefixture证据分层，**没有**真实product或Explorer启动、physical pointer/drop/DPI/IME/UIA/LightDark截图/视觉用户/安装发布验收；synthetic按钮/最小bounds不等于USER批准。无跨重启persistenthistory/HZ4真实崩溃恢复/HZ5原件回收；不暗示原TOCTOU已变ID-CAS。summarygenericretry资格受上文限制，缺事实不是默许重放。原readmission旧失败/source_unavailable/helper_untrusted/FB-AC05保持OPEN。

无package/sign/install/product/Explorer/Registry/service/ACL/UserChoice/recycle/真实用户归档修改/truncate/punch/依赖vendorbackend变更/commitpush/新会话或递归委派；未绕过self-appGUI拒绝。**现在停止，等待协调会话对第五步独立验收；不自行派发或进入第六HZ4/第七HZ5。宿主自动回调，不发送重复callback任务。**
