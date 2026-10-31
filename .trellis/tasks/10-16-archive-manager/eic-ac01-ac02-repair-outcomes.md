# EIC-AC01 / EIC-AC02 — bounded repair delivery, independent reacceptance pending

2026-10-29。仅修[第四步独立验收](edit-interaction-acceptance.md)的两项具体缺陷，仍为 **Phase E — Format expansion and ZIP editing**。第三步/TAR-AC01保持已接受；本报告不是自行关闭验收、整阶段/产品/发布完成。**完成后停止，等待协调会话窄验收；不进入HZ3–HZ5。** 原A–F标题/顺序、第四步原交付/负面日志/本次失败验收均保留。

实现前边界：[eic-ac01-ac02-repair-route.md](eic-ac01-ac02-repair-route.md)。适用owner契约增量：[edit-interactions.md](../../spec/nxtools-archive/backend/edit-interactions.md)。未开新会话/子代理/review，父会话已用完的一次只读评审不重复。

## 1. Opening / evidence / exact change boundary

本轮证据根 `$R`：`C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\eic-ac01-ac02`。

开头保存唯一checkpoint完整 **186377 bytes**，SHA256 **838C846EC529934425386A110359A7ECEB37DC3AC74894627B3AD67FCE288CA4**，含父会话2208bytes验收前缀及原184169bytes完整历史；`checkpoint-before.bin/json`。开头HEAD/status、43项源码/事务保护/历史报告hash保存在opening文件。最后追加前再次证明live与opening完整相同，只允许prepend；最终suffix证明见 `$R/checkpoint-suffix.json`。

本轮**产品仅改两个 Rust 文件**：
1. `worker_zip_edit.rs::additions`：补原ZIP隐式目录merge，不改record editor。
2. `worker_output.rs::PrivateOutput::call_payload`：exact native `cancelled` failure映射到ArchiveError::Cancelled。

新增Rust actual-worker回归及独立Python fixture；C#只在原EditInteractionTests新增两项owned-native用例及Program/TestEffects精确注册。C# Job/Runner、native SourceLease/OutputSession/VolumeSources/CreationSources、ZIP/7z/TAR codec/planner/outer、Desktop/UI/public DTO/Cargo.toml/lock全未改。没有替换取消框架、全局failure优先级、预算、权限、原生handoff或publication实现。

## 2. EIC-AC01 root cause / minimal fix / regression

### 根因

**E（Implicit Assumption）+D（Test Coverage Gap）**：Step4原ZIP adapter把“已有目录”仅等同于snapshot中的显式directory record。原ZIP只有`folder/same.txt`（没有`folder/`）也已经存在逻辑目录；添加本地`folder/`时，目录被误列为新增。即使唯一冲突子文件选择Skip，selected仍有`folder/`，于是创建候选并commit1次。原四决策测试使用显式folder/或单file source，未覆盖“implicit folder + source directory + allSkip”的组合。宽集合通过不能否定这个反例。

### 修复

仍在原adapter，根据**原始snapshot**的slash-boundary descendant判定逻辑目录已存在，合并该incoming directory而不写冗余directory record；精确显式directory merge原逻辑保留。只在没有exact entry时判定隐式目录，不把exact同名file当目录。保留原大小写冲突policy，`folder/`与`folder-other/`不能混淆。**未来incoming/reserved计划不得冒充原目录证据**。

因此原目录merge+全Skip：0 source_file_open、0候选begin/write/read/commit、0publication、1skip warning、原bytes/hash不变。真正新空目录、真实非Skip文件仍进入同一完整候选/一次提交。file/directory类型冲突依旧拒绝。

### 真实RED→GREEN

- 新Rust `eic_ac01_implicit_and_explicit_directory_merge_all_skip_never_builds_candidate`：RED implicit-skip `complete/commit1/begin1/source_open0`；GREEN implicit/explicit两种均0candidate。
- 新Rust `eic_ac01_new_empty_directory_and_real_file_survive_merge_without_prefix_confusion`：RED real-file候选额外含`folder/`；GREEN new-empty/real-file/folder-other边界/type-conflict四controls。独立CPython zipfile exactnames/content/testzip，source始终不变。
- 原协调脚本 **逐字节不改**重放：`edit-implicit-probe.py` SHA256 **A5079B7DB86C116D4A5CD174405B8A5685290D10FB852699CC9B1BE5FAD3A458**。旧3396worker exit1，新8831worker exit0：complete/files_verified0/bytes_verified0、commits0/opens0/candidate_entries[]/source_unchanged=true。
- 新C# `archive EIC AC01 native directory merge zero publish`：原worker真实Job/native RED；新worker GREEN，implicit/explicit allSkip都是succeeded_with_warnings/skipped1/committed0/journal[]/native before_flush0/native_published0/原件完整相同；new-empty及real-file controls各一次真正publication并由.NET解码exact names/content。

## 3. EIC-AC02 root cause / exact semantic propagation

### 根因

**B（Cross-Layer Contract）+D（Test Coverage Gap）**，是既有边界缺陷，本轮独立验收揭示，不声称由Step4新引入。C#原native reply `success=false,error_code=cancelled`进入Rust `call_payload`时没有对应分支，落IoFailure。Job保存worker failure时可能盖过runner真实cancelled，因control/native回复先后不同而表现为原TAR final-cancel偶发failed/io_failure。旧同名native复测PASS并不能根治；只有去掉第二个取消control frame、只保留native回复才能给出确定性信号。

### 修复

原映射中增加**唯一exact `Some("cancelled") => ArchiveError::Cancelled`**。不检查“是否曾请求cancel”，不把所有失败重标cancelled；source_changed→SourceChanged、target_exists→OutputCollision、write_limit→ResourceLimit、其余原IoFailure保持。C#Job precedence完全不改，真实committed/commit_unknown/sourcechanged/cleanupfailure/timeout依旧拥有原权威；不自动retry未知提交，不放松lease/FILE交接/预算。

### 确定性与owned-native证据

- Rust `eic_ac02_native_reply_only_cancelled_preserves_zip_tar_sevenz_semantic`：完整候选readback、真正oldreaderclose/edit_reads_complete后，**唯一commit回复cancelled，不发送cancel control frame**。旧worker真实failure/IoFailure RED；新worker ZIP/TAR/7z三个scenario均failure/Cancelled、commit_attempt1、source unchanged。
- `eic_ac02_other_native_failures_are_not_relabelled_cancelled`五个reply-controls：source_changed、target_exists、write_limit、replace_file、commit_outcome_unknown保持原mapping，无取消请求覆盖。实际unknown/native5/64/sourcechange/latecancel由原native兼容另证。
- 原协调脚本逐字节不改：`edit-cancel-probe.py` SHA256 **953728B63455A6EE10E095B2341C96E196A5B2F4B524277FE305214AADA2D187**；旧3396worker exit1/new8831worker exit0，failure/Cancelled、唯一commit_attempt1、native_reply cancelled、source_unchanged=true。两原脚本复制/断言完全相同，只有isolatedenv路径/identities指定old/newworker；父原负面证据不写入。
- 新C# `archive EIC AC02 native cancelled reply only ZIP TAR 7z`使用原held-source `edit_source_rechecked` probe返回cancelled且不dispatch cancel control，新worker三格式均原Job cancelled/code cancelled/0commit/cleanuptrue/0published/原bytes不变/ownedpartial清理。
- **限制如实保留**：该C#注入经过真实Runner，其FailNative也可能先取消worker，所以它是actual owned-native结果证明，**不是每次都能RED的纯reply隔离器**。初轮旧worker失败，最终同一测试定义旧worker重放却PASS；未反复寻红。确定性RED authority是上面的actual-worker私有reply合约及原父脚本。原真实control cancel、native5/64/unknown/latecancel用例在新worker均通过。

## 4. Exact verification, counts and commands

所有Cargo前检查共享cargo/rustc/clippy idle；**同process** jobs1/incremental0/RUSTFLAGS=`-C codegen-units=256`，locked/offline、serialtests、长timeout。无重复build/namekill/共享通道保留请求。普通incremental，不叫cleanbuild。完整argv/exit在 `$R/command-ledger.json` 与逐command JSON，原stdout日志保留（PowerShell多为UTF16BOM）。

| Command / selection | Actual result / evidence |
| --- | --- |
| `cargo test -p nxtools-archive --features isolated-codecs --locked --offline --test eic_repair_worker_contract -- --test-threads=1`，产品改前 | **1PASS/3FAIL/0ignored/0filtered**，exit101，worker-red.log；AC01两failure/AC02replyfailure/其他nativeerrorcontrolPASS |
| 同command修后 | **4PASS/0FAIL**，exit0，worker-green.log；共14tiny scenarios（AC01六、AC02三格式+五错误control） |
| `python $R/replay-parent.py` | copied原2scripts×old/newworker：old两exit1、新两exit0，断言/脚本hash不变；parent-replay-results.json/wire/result/stdout/stderr保留 |
| `cargo clippy -p nxtools-archive --features isolated-codecs --locked --offline --tests -- -D warnings` | exit0，strict-archive.log，无lint suppression |
| `cargo check --workspace --tests --locked --offline` | exit0，workspace.log |
| `cargo build -p nxtools-archive --features isolated-codecs --bin nxtools-archive-worker --locked --offline` | exit0，worker-build.log，冻结文件与Cargo生成exe同hash |
| Archive原44targets+本轮eic_repair_worker_contract（lib+44integration） | **45targets/265PASS/0FAIL/0ignored/1filtered**，exit0，archive-regression.log/archive-target-counts.json；展开argv见archive-regression.json |
| `cargo test -p nxtools-desktop --locked --offline --bin nxtools-desktop archive -- --test-threads=1` | **332PASS/0FAIL/0ignored/1167filtered**，exit0，desktop-regression.log；原Desktop源码无本轮修改 |
| `dotnet build csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore -m:1 -nr:false -p:UseSharedCompilation=false` | 最终exit0/0warning/0error，dotnet-final.log |
| effects审核→exactLIST→同nameRUN16项 | **16selected/16run/16PASS/0SKIP/0FAIL/0not_run**，一轮native-green-final；每项list1/run1/exits0 |
| scoped3Rustfmt/7source diff | exit0/source-checks.json |
| spec117documents14owners、GPUIrelease、pinnededitor、test-selection | exit0/source-only，不是user/native视觉验收 |

Archive target范围是原交付/父独立同44target集合增加本轮1target，不删旧失败目标、不降预算或断言。`output_readmission_chain`本轮PASS但不宣称修复历史Cancelled-vs-SourceChanged不稳定。原TAR-AC01 raw4tests/20samples继续PASS，TARengine不变。`default_budget_lists_one_hundred_thousand_independent_entries`唯一明确filtered；独立rar/sevenz/tar_stream_readmission_chain未选，无100k/performance/fullnative主张。Core/IPC/Protocol未改，不把父前轮数据称本轮重跑。

### Archive45 exact target PASS counts

全部FAIL0/ignored0；仅engine_contract filtered1。完整逐targetJSON `$R/archive-target-counts.json`。

| Target | PASS | Target | PASS |
| --- | ---: | --- | ---: |
| lib |42| creation_plan_contract |4|
| creation_worker_contract |1| edit_interaction_worker_contract |2|
| eic_repair_worker_contract |4| encoding_contract |4|
| engine_contract |14| extraction_plan_contract |5|
| interaction_contract |4| job_execution_contract |2|
| metadata_wire_contract |1| output_readmission_chain |1|
| output_readmission_contract |3| product_read_cancellation |10|
| product_read_chain |24| product_read_consumer |7|
| prompt_display_contract |2| rar_readmission_contract |1|
| sevenz_edit_safety_contract |4| sevenz_edit_worker_contract |1|
| sevenz_readmission_contract |1| source_deletion_contract |9|
| stream_contract |12| tar_edit_eligibility_contract |1|
| tar_edit_matrix_contract |8| tar_edit_raw_header_contract |4|
| tar_edit_worker_contract |3| tar_stream_readmission_contract |2|
| tar_stream_readmission_guard |1| tar_time_contract |6|
| transaction_contract |5| tree_creation_worker_contract |1|
| worker_cancel_contract |1| worker_extraction_contract |9|
| worker_wire |19| xz_read_contract |5|
| zip_boundary_contract |2| zip_creation_worker_contract |1|
| zip_crypto_contract |5| zip_edit_contract |10|
| zip_edit_worker_contract |1| zip_methods_contract |13|
| zip_volume_contract |5| zstd_read_contract |1|
| zstd_safety_contract |4| — |—|

### Native16 exact effects / LIST / RUN

审过effects：仅GUID-owned包/输入/归档、原Job/BrowserACK/native事务、已有commitprobe、exactownedchildren、explicit小oracle；无product/Explorer/Registry/service/ACL/userarchive。命令每项先 `NxTools.PlatformHost.Tests.exe --fixtures --list "<exact name>"`，确认只1matching，然后**同name** `... --fixtures "<exact name>"`；不能用LIST代RUN。脚本 `$R/run-native.ps1 -Stage green-final -Worker $R/frozen/nxtools-archive-worker.exe`；01..16-list/run.log、results.json保留。每项PASS1/skip0/fail0/notrun0：

1. archive EIC AC01 native directory merge zero publish
2. archive EIC AC02 native cancelled reply only ZIP TAR 7z
3. archive edit interaction native ZIP Ask decisions single publish
4. archive tar edit native source cancel handoff
5. archive tar edit native failure unknown late cancel
6. archive tar edit native conflict injected lowspace
7. archive sevenz edit native source cancel handoff unknown
8. archive sevenz edit native conflict skip native terminal
9. archive ZIP edit native same path cancellation truth
10. archive ZIP edit native changed revision refusal
11. archive ZIP edit native handoff gates and ownership
12. archive ZIP edit native handoff failure unknown no retry
13. archive job fixture actual cancel after publication native facts
14. archive output fixture native failure diagnostics privacy
15. archive conflict fixture actual 7z RAR TAR decisions
16. archive preview fixture actual output budgets

Native5/64/diskfull112是原owned注入边界，不冒称真实磁盘填满/网络断线/电源故障。

## 5. All failed/fixture rounds retained

- `dotnet-red.log` exit1是新测试构造ArchiveJobModify漏Value=null的编译错误，非behaviorRED；修fixture后dotnet-red-corrected exit0。
- `native-red`旧worker两新增用例 **0PASS/2FAIL/0SKIP**，原生产边界失败，产品改前。
- `native-green`新worker首轮 **15PASS/1FAIL/0SKIP**。AC02ZIP/TAR已cancelled正确，新增stock7Zip目录种子在7z编辑前corrupt_archive、probe未到，不是取消修复失败。未改7z产品资格/codec；仅测试改用既有SevenzEditTests同一路原Jobcreate合格seed+stock7Zip独立test，最终native-green-final16PASS。
- `native-red-final`最终定义旧worker **1PASS/1FAIL**：AC01稳定fail，AC02actualRunner这次PASS，证明真实nativecontrol竞态不能作为唯一确定性回归。未重跑寻红或降低断言，exact父script/worker-only对照仍旧失败/新通过。
- PowerShell opening path/value序列化使最初drift比较报找不到驱动器；原hash完整，Python规范解析value后43基线准确5changed/38unchanged，原JSON与失败输出不覆盖。非ASCIIoracle路径另用UTF8 identities-utf8.json完整保留，不用有损显示推断文件不存在。

父原83命令/Archive261/Desktop332/native35通过、以及本次EIC两反例失败仍是原独立验收事实。本轮不改 `edit-interaction-acceptance.md`，不删除原Step4 260PASS/1FAIL与33PASS/1FAIL/1SKIP，也不冒称旧source_unavailable/helper_untrusted/FB-AC05/ZIP链已全修。

## 6. Matching worker / oracle identities

本轮源码build后复制冻结 `$R/frozen/nxtools-archive-worker.exe`，SHA256 **8831BEAD51689176FB4409568B024ED7809ADABD131A6A437AC4109FE2E341F7**，11513344bytes；与`target/debug/nxtools-archive-worker.exe`同hash。新native与父probeGREEN都显式用它，非旧3396worker。旧worker保留只用于RED对照。

`NXTOOLS_ARCHIVE_WORKER_FIXTURE`指向本轮freeze；`NXTOOLS_EDIT_INTERACTION_WORKER_SHA256`、`NXTOOLS_TAR_EDIT_WORKER_SHA256`、`NXTOOLS_ARCHIVE_SEVENZ_EDIT_WORKER_SHA256`=上述hash；Python/7Zip/fixtures/codec/TARoracle/bsdtar显式设定，newroots在NXTOOLS_EIC_REPAIR_EVIDENCE。身份完整path/bytes/hash见 `$R/identities-utf8.json`：

| Identity | SHA256 |
| --- | --- |
| CPython3.13.2 `C:\Users\Administrator\AppData\Local\Programs\Python\Python313\python.exe` | `7D96A4ED35D6E596CD9DD8933FEAFC66349CC21F75BE3B15C89FD336E50140C1` |
| stock7Zip26.00 `D:\Tool\Directory Opus\Directory Opus\_Green\7z极限压缩\bin\x64\7z.exe` | `4BCC005758C5E604C0AAA257B6097846A4A708E62A9C0C7471FA360FAD863F1F` |
| bsdtar/libarchive3.7.7 `C:\Windows\System32\tar.exe` | `92FAFED485AF0FE4C291457DC3C00D0241F6A136731BAB63EF18A3F0C949D37A` |
| C# tests apphost (host exe alone不代表tests内容) | `D80CD6FD9EEA9C10142EC899DA787F21416B346DFA4433B4130A7930C33E0E8E` |
| **actual tests DLL** | `CD57795D67ADAC445F074D29BD78F7F9C44AA3F47A34C49133E978FD1BD8FAB3` |
| current PlatformHost DLL | `15C89BF05BF2CB4B411DBF7FD1223EDE2E09D6D88F701585125DCAAE88E2D9E3` |

独立ZIPoracle为CPythonzipfile/.NETZipArchive；TARraw保真由原CPython/bsdtar，7zactualseed用原Jobcreate再stock7Zip验证。外部oracle不是产品backend，未新增依赖。

## 7. Exact source hashes / zero unrelated drift

43项opening中仅5项预期变化：2个Rust产品+3个C#test/registration，**38项完全同hash**，`drift-normalized.json`及unchanged-protection-hashes.json。新增2个worker回归文件，故本轮源码测试共7路径：

| Source | SHA256 |
| --- | --- |
| `rust/crates/nxtools-archive/src/worker_zip_edit.rs` | `F53B5979D53EA354C9F3D16A7E671F3395C22660FC2DC3EEDB2A247EAB2FAF3D` |
| `rust/crates/nxtools-archive/src/worker_output.rs` | `3060EA411088C8D7FB7CED9C4E465B9303FBA12D401A5CA478BFDC9992269475` |
| `csharp/NxTools.PlatformHost.Tests/EditInteractionTests.cs` | `039D279B844BCF4835DE49065005066210B6CDD5E0A4A8C0D603077EDC77D73A` |
| `csharp/NxTools.PlatformHost.Tests/Program.cs` | `8CD7A70773E6A79F10884AB6E202E8267C1744B5D4E2CFA0CE70D424FAEC44B7` |
| `csharp/NxTools.PlatformHost.Tests/TestEffects.cs` | `100B20917891748B73FF73C8CAEB9B6A2879A58A7A7828F674E9DD95BE1D1E28` |
| `rust/crates/nxtools-archive/tests/eic_repair_worker_contract.rs` | `D419DDE6984F378D0730BE1EF32CCDADAE654543ECF353A381ABD6D4DB1FFD39` |
| `rust/crates/nxtools-archive/tests/fixtures/eic-edit-repair.py` | `AD151E6626D349F21B56C26B00FF2C172CAAC8CD7279A7358758889468C28121` |

关键保护hash与opening相同：JobSession **4E66BA34…BFD13**、Runner **A704AF15…A80BD**、SourceLease **2108A40E…FD40**、OutputSession **74142C1C…6365**、VolumeSources/CreationSources、tar_edit **30F80D56…D3E6A**；tar_edit_outer/sevenzplan/header各自完整hash不变，zip_edit.rs为 **E7CBEC91…02F8**。前轮23交付清单中的其他源码、Cargo.toml/lock、原Step4outcomes/matrix/独立acceptance、原TAR/AC01/7z报告均不变。本轮不覆盖父negativeprobe/旧frozenworker，不复写历史已验收数据。

文档仅本轮repair route/outcomes及两owner contract增量，唯一checkpoint仅topprepend。最后完整186377bytes opening必须保持精确suffix（含父拒绝验收前缀），证明及新的size/hash见checkpoint-suffix.json。并行dirty更改不归本轮、不reset/stage/commit。

## 8. Prevention / diagnostics / stop

P0已完成：逻辑目录/显式record区别、allSkip零candidate而非仅0fileopen、reply-only取消语义而非control竞态，均由真实worker test与owner七节契约固化。宽套件绿不再替代最小反例。无需新controller/协议errorframework或额外review；不为此扩大到所有未知原生错误或7z种子准入研究。

复用原logicaljob/attempt/receipt/terminal/elapsed/nativecode诊断，只有exact固定cancelled category传播；无新日志框架/clock、路径成员/内容/密码/request Debug/exception Display、高频chunk/frame/pointer日志，也未加临时DEBUG探针。

限制：本轮不验证真实Explorer/physicaldrop/DPI/IME/UIA/LightDark视觉user/安装发布；无package/sign/install/product/Shell/Registry/service/ACL/UserChoice/Recycle/真实用户归档写入/truncate/punch/依赖vendor/预算/权限变更/commit-push/新会话委派。已接受TOCTOU不变ID-CAS，nativefixture不等于powerloss/restart实证。历史source_unavailable/helper_untrusted/FB-AC05/ZIPreadmission不稳定继续OPEN；本轮只针对两确定性EIC缺口，不宣称全部取消竞态已消失。

**现在停止，提交EIC-AC01/AC02修复结果供协调独立窄验收；不自行进入第五步HZ3或后续。宿主自动回调，不重复派发callback任务。**
