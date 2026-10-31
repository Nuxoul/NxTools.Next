# HZ4-TERM01 单项返修交付 — native failure 不再被 cleanup Cancelled 掩盖

2026-10-29，**Phase E — Format expansion and ZIP editing／原第六步 HZ4**。父任务 `cf724fa5-c30b-47fb-bd57-7ca386298169`。本单项实现和限定验证已完成，**待父窄验收，不自行接受 HZ4**；不进入 HZ5，不出体验包。

父 `hz4-7z-release-repair-acceptance.md` 的实际结果仍是 **exactnative23＝22PASS/1FAIL/0SKIP**。该 FAIL、原 native-11 夹具、前三项修复交付、旧 HZ4 报告／矩阵／seal 全部保留。本轮没有重跑23项寻绿，也不把这次限定12项选择冒称原23全绿。

## 1. 实际根因、边界和最小修复

父相关操作 `1df5eede-6c89-430e-84b8-25200ba6fabc`：commit_in_flight 注入 `ArchiveOutputException("replace_file",5)` 一次后，Runner 正确保留 failed/replace_file/native5/FailedBeforeCommit，Job 却输出 cancelled/cancelled/native5。0commit、publicationfalse、partial_removed、cleanuptrue，源前后hash一致；这是**真实用户可见原因误报，不是已证数据损失或发布**，不宣称由压缩头修复引入。

实际链路：`ArchiveWorkerOutputBridge.Execute` 的 operation/call-bound `ArchiveNativeOutputFailure` → `Run.FailNative` 将 `ArchiveFault(..., OutputFailure)` 首因CAS保存 → 停止worker产生后果性的 Cancelled frame → `ArchiveJobSession.OnFrame` 保存 op.Failure=cancelled → `ResolveTerminal` 正确保留原native原因 → Job旧优先级因replace_file不在显式nativeCause集合而选中op.Failure。

仅两个生产文件改变：

1. `ArchiveWorkerEvent` 增加内部 `[JsonIgnore] bool HasNativeFailure=false`；`ResolveTerminal` 仅从当前首因 `failure?.OutputFailure is not null` 赋值，不从 native code／字符串／最后一次文件错误猜测。
2. `ArchiveJobSession.OnRunnerEvent` 将该拥有者事实纳入原 nativeCause 条件。cleanup-incomplete、publication/unknown/postcommit、timeout 分支及真实取消分类均不改。

未改 FailNative 接纳规则／CAS／停止路径、native commit/source/readback/cleanup、worker codecs/writer、队列／预算／权限／重试、协议 DTO／worker wire／IPC、GPUI 或依赖。内部标记不携带call/token等私有native内容，也不进入序列化或诊断字段。实际13份Runner序列化记录验证该标记完全缺席；原低噪声相关诊断仍复用既有字段，无临时DEBUG probe。

参考先行：按 archive.md 记录重查旧 NxTools tracked archive terms，仅 package/FFmpeg ZIP 和图像／视频相关实现，无通用 archive Job；只读复核已索引 NanaZip `CWorkDirTempFile::MoveToOriginal/WorkDir.cpp:85–92`，native替换失败返回HRESULT。Next实际拥有者链已补到现有archive reference和diagnostics owner contract。未设计新错误框架或将参考删除优先事务导入产品。

## 2. 确定性 RED → GREEN，不靠原生时序重复取样

新增 `ArchiveJobTerminalTests.cs`，通过反射调用**真实**私有 `Run.FailNative/ResolveTerminal`、原Job `OnFrame/OnRunnerEvent`，不复制投影算法，不新增产品fixture入口／环境中止开关。仅内存状态和GUID自有scratch目录，不 Start worker、不调用nativeAPI、不改归档。顺序固定为 native首因已保存 → worker Cancelled frame先送Job → Runner exit送Job。

| 记录标签 | 实际结果 |
| --- | --- |
| `red-build` | 完整Debug Tests项目exit0，0warning/0error；生产源码未修 |
| `red-cause-list` → `red-cause-run` | exact1；RUN exit1 / pass0 fail1，精确错误 `TERM01 expected failed/replace_file but actual cancelled/cancelled` |
| `red-controls-list` → `red-controls-run` | exact1；12个有限control全部PASS，exit0 |
| `green-build` | 完整Debug Tests项目exit0，0warning/0error |
| `green-cause-list` → `green-cause-run` | 同一确定性测试源码，exact1PASS/0SKIP/0FAIL，exit0 |
| `green-controls-list` → `green-controls-run` | 同12control全部PASS |

RED/GREEN的回归测试文件 SHA256 均为 `5944652DB437CF9D236F8A7AA67FFFEFCE3D27A83218130E4A28A20AC8052ED6`。RED不是编译／启动失败或夹具误定位；它先断言Runner已正确保留native5，再断言Job投影。原测试未放宽，父原native11失败材料未覆盖。

12个同Job边界control：无nativefailure的取消、实际native类别cancelled/995、仅附带code5但无nativefailure事实、source_changed、target_denied、64unknown、timeout、cleanup_incomplete、domainCorruptArchive、已发布latecancel、已发布postverify失败、已发布targetchanged。断言原nativecode/count/bytes/commitstate/cleanup/三态publication/resultlocation/journal原样转交、terminal只发一次。它们是投影控制，不伪装为真实native发布／真实取消；实际链另有下表覆盖。

## 3. 本轮 exact 12 fixture 选择，原 native5 只执行一次

effects先审，逐项 exact LIST → identical RUN，记录 `native-01…12-list/run`；每项 **exit0、selected1 run1 pass1 skip0 fail0 not_run0**。总 **12PASS/0SKIP**＝8个自有native兼容fixture＋2个确定性projection fixture＋2个HZ4门禁，不是全native23。

| 标签 | 精确名称 |
| --- | --- |
| native-01 | archive sevenz edit native conflict skip native terminal |
| native-02 | archive sevenz edit native source cancel handoff unknown |
| native-03 | archive EIC AC02 native cancelled reply only ZIP TAR 7z |
| native-04 | archive ZIP edit native handoff failure unknown no retry |
| native-05 | archive tar edit native failure unknown late cancel |
| native-06 | archive job fixture actual conflict global admission cancel |
| native-07 | archive job fixture actual cancel after publication native facts |
| native-08 | archive HZ3 AC01 publication wire retained directories |
| native-09 | archive HZ4 TERM01 native cause before worker cancellation |
| native-10 | archive HZ4 TERM01 terminal authority controls |
| native-11 | archive HZ4 process fixture capability release gate |
| native-12 | archive HZ4 process fixture no durable restart authority |

native-01 原代码仍完整包含seed、all-skip warning、native5失败、latecancel后保留发布及独立7Zip完整manifest断言，本轮仅RUN一次。已按该**精确运行时间窗＋GUID＋当前workerhash**只读定位并原样复制失败步骤保留物，不递归／不清理：

- 自有root：`C:\Users\Administrator\AppData\Local\Temp\archive-product-0cffe285-f14a-4fdc-acda-e4cfa4546091`。
- correlation：`d0ae49ab-832c-46cf-b140-d3171abd227a`。
- 原 `3.terminal.json`：**failed / replace_file / native5 / FailedBeforeCommit /0commit / cleanuptrue / output_publishedfalse / unpublished+partial_removed**。
- 源 `3.before.sha256`＝`3.after.sha256`：`167887E1117B27CE0A2B91819EDECA4B630D27A690392C95D140A4014895107D`。
- 同correlation Runner failed和Job terminal均为replace_file/native5；原生源／候选身份及203bytes失败候选的journal原样保留，无retry。
- `native01-evidence.json`、`native01-retained/` 保存实际terminal／diagnostics／before档及字符串hash／原副本SHA，不把timewindow定位另计作一次nativePASS。

真实user cancel、已接受的native-cancelled→Cancelled、sourcechanged／unknown64、ZIP/TAR native failure和latepublish语义由本次有限原fixture选择保护。timeout/cleanupfailure还有同生产Job边界确定性control；不声称执行过实际系统超时／磁盘或网络故障。

## 4. 当前完整Debug/Release身份与Release门禁

相关Rust／manifest实际453项与父保留基线一致，本轮**没有运行Cargo**。复用原冻结worker而非冒称新构建：`CA67D018EF04F8F3ACBD09F34C6D1E85768D609E16A791E8949D8B0552AFEA34`。

| 实际内容 | SHA256 |
| --- | --- |
| 新Debug Tests DLL | `21CEAB3D5A0B629D7E9F3B7F2AA8A394C4BA7B43C535EBBFB5F0F53CB1C499B1` |
| 新Debug Host DLL | `4D75B06BD0ED934349D614D3480FFC96A111708D9CE809AA4A7197BAA249713D` |
| 新Release Tests DLL | `9B641D1488B8FCE55C7B18726DBBA4EEA6D0CC5662F3E374AF383DDE52BA6AAC` |
| 新Release Host DLL | `A42366FD6F853E4C5CAE1F5B0AF97A90D6411B801B691E64E2516057FA722C80` |

apphost EXE hash不代表测试内容；完整程序集和Host、当前源码、worker及明确oracle身份／env分别记录。`fg/`、`fr/` 分别是完整Debug／Release输出的自有冻结副本，不是产品部署。Python和stock7Zip显式路径沿用原hash冻结oracle，env在native/env.json保留。

Release前实测 committed headroom **7211864064bytes**，最低6GiB／6442450944bytes，Rust进程0。完整Tests csproj Release实编译 exit0／**0warning/0error**；完整Debug同样如此。未启动ReleaseCargo／Releaseworker／package。

- `release-1/2-list/run`：实际完整Release程序集原exact2 Release-inert门禁 **2PASS/0SKIP**，包含risk seam预effects拒绝及无durable恢复。
- `release-3/4-list/run`：实际完整Release程序集两项新projection测试 **2PASS/0SKIP**，其12control仍PASS。
- `release-entry`：实际完整Release `--hz4-owned-process` **预期exit2 / release_fixture_disabled**，指定不存在路径仍不存在。
- 不用scratch mini-probe代替完整Tests，不削弱Debug风险断言，不抑制warning／SkipVerification，不将risk拒绝说成真实风险交互PASS。

## 5. 命令、质量门禁及证据封存

证据根 `R = C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\h4t1`。

`command-ledger-final.json` 保留所有记录器调用的完整argv／exit／PID／start／elapsed／环境／logSHA；不是仅绿列表。RED exit1和Release拒绝exit2分别保留。外层wrapper exit0不是内部全绿的替代，确切子命令／RUN统计已逐项核对。`red-source-binary-identities.json`、`current-identities-verified.json`、`release-results.json` 证明实际程序集／源码／worker；初版`current-identities.json`的零项Rust证明无效，保留并明确由已核实版本取代该证明，不能误用。

实际命令索引：`opening-baseline`（命令正常但清单发生下述基础设施覆盖）、red/green-build、red/green-cause与controls-list/run、freeze-current、native-exact及24个exact选择子命令、baseline-reconciliation-run、release-wrapper/release-build、release-check-run及8个LIST/RUN、release-entry、retain-native01-run、proof-static-run及static-spec/selection/cli/release/editor/diff。最终账本给出精确记录数和每条argv，wrapper不加成产品用例。

质量门禁已PASS：完整C# Debug/Release构建、上述限定fixture、spec导航／contracts118文档14owners、C#effects选择源门禁、archiveCLI16源断言、release/editor provenance、`git diff --check`。这些static/provenance不是GUI/native/release验收。两个生产文件和两个catalog文件相对开工保存原文的变更被确定性重建核对，未夹带其他逻辑变化；没有temporary production probe或新的诊断载荷。

本轮不运行Rust349/102/234、strict/workspace或原7z14点／restart大组：Rust及wire未改，父已实证前三项修复，本单项只作必要C#边界回归。历史349/102/234等只能引用为父上轮结果，**不冒称本轮再次执行**。三无关networkfmt不动。

## 6. 基线失效披露与实际保护依据

必须披露：开工脚本产生1080-file hash inventory后，`run.py`使用相同 `opening-baseline` 标签将它覆盖成命令记录。因此**该1080开工hash清单不再存在，不能声称其零漂移，也没有重生成后冒充开工快照**。`opening.json`和命令日志只证明当时生成过；最初freeze的Rust检查零项是无效证明，已被检查发现并纠正，保留原记录。该基础设施错误不是产品RED，也未改变已保存checkpoint／四目标原文／父负面材料或真实测试结果。记录器已改为最终JSON独占创建，防止子命令同名文件再被覆盖。

实际替代保护证据是**父已封存的历史1045-file baseline**，不是新开工快照：

- 父原文件 `h4r-220225/baseline.json` SHA256 **AEB1161BD4A964979635FE6A855AFCAB8CA116BE74A14374D8978B5390D2FF5C**，原样保留到 `parent-baseline-retained.json`。
- 对账1045项：**1040 unchanged**，4个声明C#文件及checkpoint变化；checkpoint这一历史差异原已包含父prepend，本轮另外只允许顶部追加并对本轮actualopening证明suffix。不把父历史checkpoint差异误称全由本轮造成。
- **453 Rust／manifest文件实际全部未变**，证明worker复用；四个本轮保存原目标内容可匹配该历史rawhash（显式LF/CRLF候选匹配），最小差异重建也一致。
- `docs/reference-code/archive.md`不在父1045内，单独与上轮真实保留基线核对：原SHA256 **ECF7C665D61EAD891E6C81A9A2FD373D030B89A6B7BD7C658DBED4673EC9BC60**，只有本轮声明6行append。
- diagnostics owner spec也不在父1045内，单独以本轮完整初读和精确逆插入核对：原重建SHA256 **2CA0AE3FA102376D86F103C9FDC99FE9C6EA57E90C8D97FADFEDC676B948FC3D**（初读anchorFC3D），只有声明段落及空行插入；不虚算进1045覆盖。
- 上轮 **1971**项私有证据、原HZ4 **1645**项证据逐一hash不变，旧报告／矩阵／seal也核对不变；父 native-11 **13项**负面材料hash不变。父新暂缓报告保持不动。

`baseline-reconciliation.json`／`proof-current.json`／最终seal清楚标示这些来源与覆盖限制；不宣称任意其他workspace或私有工作站发布材料全量盘点。

## 7. Checkpoint、剩余限制与停止点

本轮actualopening **225348bytes**，SHA256 **CA8D272D3D2F592F6FFDAB2D13F3B958784829380128C30DC18FB7E9DB4CE8BF**，完整 `checkpoint-before.bin` 真实保留且不受清单覆盖影响。只topprepend，`checkpoint-suffix-final.json`／`delivery-seal-final.json` 证明**整个225348-byte actualopening精确suffix**，不只匹配标题或历史222502-byte版本。最终seal另记录命令、源码／报告、程序集、父负证据和当前fixture hash／进程观察，封存自身不递归自hash。

剩余：待父本单项窄验收；此前bsdtar特定结构reader-unsupported、无可信durable恢复／fresh stale_operation只是安全拒绝、旧password／CLIhelper_untrusted／readmission／source_unavailable／FB-AC05／真实物理用户／installed验收仍OPEN。无power-loss／真实diskfull/networkloss／大资源／fullfixture/fullworkspace/fullnative／完整23sweep的新证明。

没有用户源write/delete/truncate/punch/reclaim、系统Registry/service/ACL/UserChoice/Recycle／产品Explorer操作；没有package/sign/install／密钥账本installer占号／commitpush／依赖vendorbackend变更；没有新增会话／review。完成封存后停等自动hostcallback父窄验收，不重复派callback任务，不自动HZ5或体验包。
