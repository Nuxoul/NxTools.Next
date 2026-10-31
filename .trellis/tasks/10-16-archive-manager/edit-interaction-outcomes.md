# Step4 — 编辑操作入口与冲突交互接入：实现交付，待独立验收

2026-10-29。用户原七步清单**第四步**，仍为 **Phase E — Format expansion and ZIP editing**。第三步及 TAR-AC01 已独立接受，不重复其引擎工作；原 A–F 标题/顺序不变。本报告是源码/自动化/owned-native 实现证据，不是自验收、Phase E/产品/发布完成。**交付后停止，等待协调会话独立验收；不进入第五 HZ3、第六 HZ4、第七 HZ5。**

- 实现前路线：[edit-interaction-route.md](edit-interaction-route.md)。
- 精确 operation×entry×format×state、真实 RED→GREEN、逐 target/逐 native 名称/命令/失败历史：[edit-interaction-matrix.md](edit-interaction-matrix.md)。
- 可执行 owner 契约：[edit-interactions.md](../../spec/nxtools-archive/backend/edit-interactions.md)。
- 证据根：`C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\edit-interaction`，以下简记 `$E`。

## 1. 参考、原始状态与约束

开头先保存唯一 checkpoint 完整 **179544 bytes**，SHA256 **969CAD58444D30986D08F8A2114F58391CFA022ADDA507F1A4070C961A4E93A2**，原文件 `$E/checkpoint-before.bin`；开头 HEAD **97d9a9dd21c2fd7563af218d29fa002730cc2520**、dirty status 和24项源/保护/历史文件哈希保存在 opening 文件。工作区大量既有/并行 dirty 改动保留，不用 git diff HEAD 冒充本轮全部改动，不 staging/reset/commit/push。

先读 AGENTS/适用 owner/shared/UI component/API/spec、当前 checkpoint、TAR/AC01/7z/HZ2 验收、archive-editing note/manifest，以及原 Browser→ModifyDraft→Form→Job→worker→native lease/FILE handoff。实际核对 PeaZip clean worktree `D:\Code\archive-edit-references\PeaZip`，commit **a6259b90031827887bae6a041db1f1c1787c6825**：peach.pas add eligibility/form/precheck/staging/rename 与 unitgwrap.pas process start/wait/password/cancel 原边界；实现前路线保留这些入口和拒绝迁移的不变量，后续参考索引补入精确链。末次仍 clean，同 commit/hash。

- `peazip-sources/dev/peach.pas` SHA256 **DEEF5B0ECC65839DEA4C2B032A4050CAA5EBCDBD51FD5602F571905F878E3560**，LGPL3+。
- `peazip-sources/dev/unitgwrap.pas` SHA256 **13A7B2513CEAFAE200A7598A35226165B7D9ACF6338DB3BA34628CE2DA4B9B4F**，LGPL2.1+。

仅使用交互编排意图/反例，没有引入 Pascal UI、全局状态、强制不支持格式编辑、argv 密码、外部7z/WinRAR 产品引擎、先删后改名、多命令事务或强杀提交。Rust 状态/布局、C# Windows APIs、GPUI 渲染边界不变。无新 controller/队列/配置系统、公共 DTO 字段、ABI、依赖/vendor 升级或原生事务重写。

## 2. 实际完成的最小缺口与根因

### 2.1 原入口统一，不推断写权限

原 Browser capability 已允许合格 TAR，但 app `modify_browser` 仍只接 ZIP/7z，ReplaceContent 只认7z；最终表单 freshness 也只查7z。导致 TAR 真实 outlined toolbar/menu/F2 能被触发，却到不了原表单。

现在同一 typed intent 接通合格 ZIP/7z/TAR/TGZ/TXZ 原表单。所有 Modify 在最终 app 入口重查**原归档路径/window generation/browser/snapshot/source/逐操作能力**；同一 Browser 不可变成员目录校验用于初始入口和最终提交，缺失/warning ordinal、wrong-kind rename/replacement 在队列/IPC 前拒绝。校验冻结意图，不把随后选中的其他行变成草稿身份；worker/native 仍独立重校真实原 ordinal/revision/native identity/成员种类。

实际能力未扩张：
- 合格 ZIP：Add/Delete/Rename/NewDirectory/Comment；**ReplaceContent 不可用**；原 ZIP UI 不伪造 implicit-directory ordinal。
- 合格7z/TAR/TGZ/TXZ：Add/Delete/单 regular ReplaceContent/文件及显式隐式子树 Rename/NewDirectory；**Comment 不可用**。
- Loading/stale/缺事实/坏源/分卷/特殊结构/不支持加密等保持原具体 refusal，读/解压/预览不全局禁用。

继续使用原 Toolbar.content 的官方 outlined Medium32 actions、菜单/右键/F2/Delete、ModifyDraft、protected picker/opaque IDs、Form 和一次性 submitted 状态。取消 picker/dialog 不提交任务或保存草稿；ReplaceContent 与 Add/overwrite 是不同 action，保留原名称/metadata 规则。

### 2.2 私有冲突对象与 ZIP 原 adapter Ask

原7z/TAR有 typed InteractionPort/ApplyAll，但 display=None，真实冲突 UI 不能辨识对象。沿现有 optional bounded `ArchiveJobPromptDisplay` 填入精确 incoming/existing 归档相对成员（各≤32767 UTF-8 bytes/no controls，超限仅缺失展示，不授权）。Modify 对话框明确“归档内已有成员”，大小 None 显示未知、0显示0 B；无日期事实就显示未知，不造日期/大小。原 ticket 的 logicaljob/attempt/prompt/generation、stale/duplicate/wrongattempt 拒绝与 ApplyAll 当前 job/attempt scope 保持。

ZIP Add 原 worker 不接受 Ask；仅在原 adapter 做候选前冲突计划：
- Ask/Overwrite/Skip/AutoRename/Cancel 通过既有 InteractionPort；C# 原 Job 对所有 Modify 传 Ask，Runner 原 private port允许 modify_zip。
- overwrite 的旧原 ordinal Delete 与 additions **组成同一个 ZipEdit 候选、一次 native publish**，不是先 commit Delete 再 Add；ZIP ReplaceContent 仍不公开。
- skip 不读该输入；全部跳过0commit+真实 warning，取消原件 hash 不变；directory-directory merge，file/directory conflict拒绝，不承诺目录替换。
- AutoRename 同时预留原归档及**整批尚未处理输入名称**，防止 `same (2).txt` 被抢占使后续输入冲突；真实 worker RED 后修复。
- 维持有限 metadata/4096 delete IDs/2..4097命名/匿名 Spool/取消/显式 finish+flush+源重验；原 ZIP record codec/candidate kept-record/readback/readerclosed/edit_reads_complete/native FILE-only singlepublish 不变。

UI 用原 Root/Dialog/DescriptionList/Checkbox/Alert/Field，冲突决策按钮为官方 outlined Medium32。私有名字/路径不进业务 diagnostics、native filesystem authority 或 request Debug；未引入新公共字段/日志框架/计时器。

### 2.3 表单、拖入、终态与新快照恢复

Add 原表单明确本地受保护来源、归档内部目标目录和“逐项询问”当前 policy；不是新增持久策略选择器。选中成员私有展示最多32项/8192bytes，不参与授权。Delete 解释所选/后代去重/合法空结果及取消；rename/newdir解释 occupied/self collision拒绝，不先删目标。

Drop 原 dialog 的 ZIP-only 文案改为中性“添加到当前归档”，捕获原 generation/snapshot/currentfolder 并在确认后重查。复用原 handle_drop/add_drop/Modify 表单；真实外部物理 drag/drop 未执行，只证明生产 app handler/dialog与 frozen-target guard。

原 job_terminal 把成功 warning/全skip 当成可重试失败，现按真实成功关闭原表单；只有实际 files_committed>0且cleanup_complete才刷新原 browser。真正 FailedBeforeCommit保留输入/显式重试并重校源，unknown/已发布不盲重试；同归档 FIFO/cleanup-held slot/其他archive queue保持。

原刷新 install重建模型会丢 currentfolder/selection。现仅存下一 generation 的路径 presentation hint：按原 intent转化改名/删除路径；新快照成功就绪后找最近仍存在目录、按新路径匹配**新 RowIds**，不能将旧71套新1；wrong generation丢弃、retire清除，其他 tab/archive/origin不受影响。生产 app→原 read IPC handshake/ACK/new terminal regression 已验证 file rename/currentfolder，新 Browser 回归覆盖隐式目录改名/删除选中项/过期generation，原多tab回归保留。

原7z全重编码/固实加密参数、TAR完整内外重写/outer参数/新metadata/TAR无内置payloadchecksum/原件候选spool并存的成本 Alert保持，不改变接受的保真/预算。原五真实阶段、receipt/freshness/NotificationCenter/clock不重写。

## 3. 真实 RED→GREEN、保留失败与范围自查

详见矩阵 exact test/log，主要行为证据：
- TAR/TGZ/TXZ production pointer/menu/F2→original app/form：**3FAIL→3PASS**。
- 7z/TAR 原 worker display Null 各1FAIL→精确对象 GREEN；实际 rendered conflict Dialog archive scope、未知/0、ApplyAll/Skip/current ticket GREEN。
- Warning/noop/form/drop 文案：presentation轮3PASS/4FAIL→GREEN。
- 真正 ready-new-snapshot 刷新 currentfolder `""` vs `"a"` RED→GREEN；早期 missing trait/handshake/fixture失败另存，不算产品 RED。
- ZIP Ask real worker EOF RED→四决策/原ticket/独立 zipfile候选 GREEN。
- ZIP未来输入名称保留 real worker候选拒绝 RED→GREEN；首轮fixture写错source_id的KeyError只算fixture失败。
- 最终 app伪造missing999/wrong-kind原成员 RED→同一入口及final-form0queue/IPC GREEN。
- 一次有界自查找到了上述未来名称与最终成员守卫缺口；无第二轮广泛review/新会话/子代理。改后完整回归，无压断言/降预算/缩选集。

历史完整保留：
- `desktop-archive-first.log`328PASS/1旧retry fixture缺origin；改为绑定原合格TAR/source后329PASS。后来追加三条边界最终332PASS。`archive::`298的较窄轮不冒充完整集合，最终用原archive过滤。
- `strict-first.log`两条本轮cmp_owned/collapsible_if错误→借用Path比较/collapsed if修复，无lint suppression。
- `archive-complete.log`44targets **259PASS/2FAIL**，缺显式 stream fixture env；补独立owned fixtures后同2项PASS，最新完整轮也PASS。
- `archive-delivery.log`44targets **260PASS/1FAIL**：既有output_readmission_chain source_changed场景期望SourceChanged而返回Cancelled；同名未改代码精确复跑1PASS。**完整轮不是全绿，历史不稳定未修。**
- 原生首轮35项 **33PASS/1FAIL/1SKIP**。TAR final held-source cancel日志已Cancelled/0commit/partial_removed/cleanuptrue，但Job终态failed/io_failure；未改测试/原生源码同名复跑PASS，保留取消投影不稳定。格式冲突SKIP因缺codec fixtures env；补原repository upstream路径同名PASS，skip不算pass。

## 4. 最终验证与精确命令

所有direct Cargo先检查共享cargo/rustc idle，再在同一process设置 jobs1/incremental0/RUSTFLAGS codegen-units256，locked/offline/serial，长timeout。没有超时后重复构建或name kill。下列均普通增量验证。

| 验证 | 最终事实 |
| --- | --- |
| Desktop `cargo test -p nxtools-desktop --locked --offline --bin nxtools-desktop archive -- --test-threads=1` | **332PASS/0FAIL/0ignored/1167filtered**，exit0；原321+11新增，HZ118/icon8/WAC12/原prompt keyboard/focus/secret/queue/tab/preview保留 |
| Archive lib+43 integration targets | 最新完整 **260PASS/1FAIL/0ignored/1filtered**，exit101；target列表/完整argv见矩阵及archive-delivery.json。单独旧链复跑1PASS不覆盖失败 |
| TAR-AC01 real worker raw-header |4tests/20combinations PASS，CPython/bsdtar内容metadata/原始字节；`tar_edit.rs`未改 |
| affected Archive+Desktop strict Clippy `--tests --features nxtools-archive/isolated-codecs -- -D warnings` | exit0，strict-delivery.log |
| `cargo check --workspace --tests --locked --offline` | exit0，workspace-delivery.log |
| current isolated worker build | exit0，worker-build-v2.log，copy/freeze后与Cargo生成worker同hash |
| C# `dotnet build ...Tests.csproj --no-restore` | exit0/0warnings/0errors |
| effects→exact35LIST→sameRUN |首轮35/35/33PASS/1SKIP/1FAIL；两项精确复跑2PASS；最新逐name35PASS/0SKIP，**不是单批全绿** |
| scoped Rustfmt17/diff23source paths | exit0 |
| spec contracts/component catalog/GPUI release/editor provenance/drag3/test-selection | PASS/source-only；最终文档与checkpoint追加后再跑spec/diff |

本轮未触碰 Core/IPC/Protocol product source；不冒称重跑其前轮93+7/43/89。明确不跑无关100k/性能、3条独立RAR/7z/TAR-stream coordinator专项、全native或product。原 source_unavailable/CLI helper_untrusted未重跑更不冒称修好。

## 5. 本轮匹配冻结 worker 与独立 oracle

`$E/frozen/nxtools-archive-worker.exe`，SHA256 **3396A7A9215892D7E76E57FF7058DF939495A3618E18D343618EFE587D044FA7**。当前源码以isolated-codecs构建后复制冻结，native显式指定；`worker-match-delivery.json`证明与当前Cargo生成exe同hash。不是使用Step3/AC01旧worker。

Native显式环境：NXTOOLS_ARCHIVE_WORKER_FIXTURE；NXTOOLS_EDIT_INTERACTION_WORKER_SHA256、NXTOOLS_TAR_EDIT_WORKER_SHA256、NXTOOLS_ARCHIVE_SEVENZ_EDIT_WORKER_SHA256均上述hash；NXTOOLS_ARCHIVE_PYTHON_ORACLE、NXTOOLS_ARCHIVE_7ZIP_ORACLE、NXTOOLS_ARCHIVE_ZIP_FIXTURES、NXTOOLS_TAR_EDIT_ORACLE、NXTOOLS_TAR_EDIT_BSDTAR、各证据root；格式冲突复跑另设NXTOOLS_ARCHIVE_CODEC_FIXTURES=repository tests/fixtures/upstream。

| 工具 | 身份 / SHA256 |
| --- | --- |
| CPython3.13.2 `C:\Users\Administrator\AppData\Local\Programs\Python\Python313\python.exe` | **7D96A4ED35D6E596CD9DD8933FEAFC66349CC21F75BE3B15C89FD336E50140C1** |
| stock7-Zip26.00 `D:\Tool\Directory Opus\Directory Opus\_Green\7z极限压缩\bin\x64\7z.exe` | **4BCC005758C5E604C0AAA257B6097846A4A708E62A9C0C7471FA360FAD863F1F**；仅独立oracle，不作产品backend；非ASCII路径从UTF8manifest读取 |
| bsdtar3.7.7/libarchive3.7.7 `C:\Windows\System32\tar.exe` | **92FAFED485AF0FE4C291457DC3C00D0241F6A136731BAB63EF18A3F0C949D37A** |
| 当前 C# test apphost | **D80CD6FD9EEA9C10142EC899DA787F21416B346DFA4433B4130A7930C33E0E8E**，native-final/tools.json |

新Rust ZIP fixture独立zipfile exactnames/content/testzip/sourcehash；新C# ZIP fixture原Browser ACK/Job/private Ask→原native单次publication，.NET ZipArchive独立decode/count/sourcebytes。原7z oracle/metadata、TAR CPython/bsdtar、native5/64/latecancel/112、lease/preview/control小兼容仍在精确35项中。native故障是原已批准owned注入，不冒称真实网络故障/磁盘填满/电源故障。

## 6. 源码保护、交付路径和哈希

`opening-drift-delivery.json`：24项opening中10项本轮预期产品入口/adapter路由变化，14项不变；不把缺baseline当drift。以下保护项与本轮opening逐字节hash相同：

| Protected owner | SHA256 |
| --- | --- |
| `rust/crates/nxtools-archive/src/tar_edit.rs` | `30F80D56C7921C6F3B5B9F332E90DFD28B0ED34DED00A5EF7BDC9AC90B4D3E6A` |
| `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveSourceLease.cs` | `2108A40EBB8A88E8E4190772BD9A67A09A8E19534D7F0F0DF34CE3E2D768FD40` |
| `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs` | `74142C1CC03C451CED9C38C687CCF8EF8A61CF61910ECCF6ECE6E5CE013A6365` |
| `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVolumeSources.cs` | `2A417DE6B3549648A5147A4B2251DE262B5F2D9CFBC6E918F6D53962B1111DB3` |
| `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveCreationSources.cs` | `BBCA5998C1C8FA78672C829417E2256ED6A6B49F3CCE0C0747A8F60E5DE7832A` |
| `rust/apps/nxtools-desktop/src/archive/draft.rs` | `309E10C55EA47C4531B9015C48067181CDD875E0DD6968D5381A71B2EA6B07C8` |
| `rust/crates/nxtools-protocol/src/archive_job.rs` | `5AAC39660C95CBB7142161B1E287868C308301D69AFA2879B2BC5454D81C3363` |

Cargo.toml/lock与opening相同；原tar-edit-outcomes、tar-edit-acceptance、tar-edit-ac01-repair-outcomes/acceptance、sevenz-edit-acceptance全部原hash不变。`tar_edit_outer.rs`、`sevenz_edit_plan.rs`、`sevenz_edit_header.rs`、`zip_edit.rs`本轮没有修改，当前hash记录在codec-final-no-opening-baseline.json，但opening清单未包含它们，**不伪造本轮opening逐字节比较证明**。原negative历史不重写。

本轮23源码/测试交付文件（delivery-source-hashes.json含完整path/bytes，主业务之外dirty改动不属于本轮）：

| Source | SHA256 |
| --- | --- |
| `rust/apps/nxtools-desktop/src/archive/jobs.rs` | `1E98CD52D567D80B88E7102DD17D5D855EBC9037EE266681D05CEBC6B83270D5` |
| `rust/apps/nxtools-desktop/src/archive/application.rs` | `5E212D1916AE7361B62200C36740EC58A5579FE807A39B02F9711EB002469E59` |
| `rust/apps/nxtools-desktop/src/archive/browser.rs` | `17887FCC05F66177F22AB28A77EA7395DE346EEADF2AC365FFDB8340EAE92EE2` |
| `rust/apps/nxtools-desktop/src/archive/forms.rs` | `09CDA40C374913FFCAAFB533C95452C53602BEB5AFEAA73F64F4A910EFB88D00` |
| `rust/apps/nxtools-desktop/src/archive/progress.rs` | `C7078FDDE5B4BDBA27F0B6C58909E9E0FA5E48EC09A4053B20EEA774B6D1142B` |
| `rust/apps/nxtools-desktop/src/archive/application_tests.rs` | `3C104819A3643987C6981153274924003E13FF2F9F3C394FA48B845F3B3525CD` |
| `rust/apps/nxtools-desktop/src/archive/job_tests.rs` | `55FDC4FEA0B36880A5A7AB204C55F46A19C40604042A53CCB53A983C7C4C29F0` |
| `rust/apps/nxtools-desktop/src/archive/forms_tests.rs` | `88D0F223925464910F0C2FD196EDBB850CC53207C24DC776E99AE9E5FCF9FFF2` |
| `rust/apps/nxtools-desktop/src/archive/browser_tests.rs` | `927747D3A838C1A074201BD6178032686026255C60EA783080E642C09A2283ED` |
| `rust/crates/nxtools-archive/src/worker.rs` | `FD65AAEFF07F64906C5F73786B99FD4379EA2EB2976CE4CD7CD428016FB3C6F8` |
| `rust/crates/nxtools-archive/src/worker_conflict.rs` | `3911D13A2DC4E06A51748F7C28ACFD34EC560463B84253052372193BC2F6EF3B` |
| `rust/crates/nxtools-archive/src/worker_zip_edit.rs` | `4675FF0782991E4BC4E7FD71679C2A809B8B04265F0587763FE68A88E987B16D` |
| `rust/crates/nxtools-archive/src/worker_sevenz_edit.rs` | `B18972A7B373A8C49169ABBB11772F195F468F8CA115C8A5C2AA812AA39A58D6` |
| `rust/crates/nxtools-archive/src/worker_tar_edit.rs` | `87B5ACDCF5D66C8005B2DC6ACA2CE3E7145EE64DAD17C5C1C33364A0A30F0176` |
| `rust/crates/nxtools-archive/tests/sevenz_edit_safety_contract.rs` | `4FB3EAE11F0288EAE18AFB85236B52725D39A38C9191C1DC5794F44046FBF3D8` |
| `rust/crates/nxtools-archive/tests/tar_edit_matrix_contract.rs` | `948099FADCE5D85B83834BC8C870B5AA78CC56BEDB9A42B7EC05231C2B92D5E6` |
| `rust/crates/nxtools-archive/tests/edit_interaction_worker_contract.rs` | `6B0004C5F4C673EB1290E2A8D8E59644F5E5C7A649FCDC0500C499C963523067` |
| `rust/crates/nxtools-archive/tests/fixtures/edit-zip-conflict.py` | `FAF488091F5D2ECC9E5175D218B8F58B8263010477FEAA97F4D605839AB5EA25` |
| `csharp/NxTools.PlatformHost/Protocol/ArchiveJobSession.cs` | `4E66BA3463F9065A0B950CFAB09FDE601577342DD7659CF5621A7DEEAB5BFD13` |
| `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs` | `A704AF1508C1BE54FD6B9E2C98F3606901EE7F5E4DC09F1DB1E6998EE96A80BD` |
| `csharp/NxTools.PlatformHost.Tests/EditInteractionTests.cs` | `ECF74C6C48C899385CAA9D3BA68FF23F1AB487C414B363C4799E540E02A76ADD` |
| `csharp/NxTools.PlatformHost.Tests/Program.cs` | `F074A24589D11626027EE4493EE30198115E9190806822F03D44E5633990086D` |
| `csharp/NxTools.PlatformHost.Tests/TestEffects.cs` | `E5BE3D2702A16DA1DB077D99D7DB28A7F0997496E5673A26D9C300623DCAC366` |

文档仅新增route/outcomes/matrix/owner edit-interactions与必要owner index/native契约/reference note增量；原已验收报告/原引擎不重写。唯一checkpoint仅顶部追加，最终suffix证明记录在 `$E/checkpoint-suffix.json`；原完整179544字节必须是新文件精确suffix，协调接受前缀也保留。

## 7. 诊断、真实限制与停止边界

诊断在route实现前定义：复用既有 correlated logicaljob/attempt/receipt/ticket、accepted/start/fixedstate/terminal/elapsed/native-code；成员/路径仅私有UI与受保护源授权，不加业务路径/内容/密码/headers/cookies/request Debug/exception Display。无每chunk/frame/pointer事件、新clock或日志系统。原bounded/nonblocking sink、失败不改操作结果保持。

仍OPEN：历史 C# source_unavailable、CLI helper_untrusted、FB-AC05、ZIP readmission Cancelled-vs-SourceChanged，以及本轮一次TAR final-cancel Job投影io_failure后同名通过的不稳定性。未顺手修原事务/预算/断言或伪装全部完成。

未执行产品/真实Explorer/外部物理drop/DPI/IME/UIA/LightDark实际截图/用户/安装验收；synthetic official control bounds/keyboard/focus/长CJK/最小窗口回归不等于视觉或用户接受。真实主题截图缺陷FB-AC05未猜改。Dates未知、固定Ask/per-prompt策略、ZIP implicit UI identity限制如矩阵明确列出。没有承诺完整POSIX/NTFS保真、原TOCTOU已变ID-CAS、崩溃/重启/电源故障恢复。

无package/sign/install/product/Explorer、Registry/service/ACL/UserChoice/Recycle、真实用户归档修改/truncate/punch、依赖/vendor/backend/全局权限变更、commit/push、新会话或委派；未绕过self-app GUI拒绝。**现在停止等待协调独立验收，保留失败历史；不进入Step5/HZ3–HZ5。**
