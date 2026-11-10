# 索引服务正确性与资源修复

## 结果

H2、H3、M3、M4、U4 和已确认的 L11 已完成；M8 在测量确认调度成本后采用不可变行共享。485 项互不重复的普通测试通过，另有2项默认忽略的百万行测量通过。未运行产品、安装、打包、Windows 服务操作或真实卷 MFT 扫描。

## 根因、改动与方案

| 项目 | 根因与结论 | 实现边界 |
| --- | --- | --- |
| H2 | 每条删除在判断可见性和类型前消耗递归预算；基础子项数包含墓碑和移出项。 | `usn.rs::delete_visible_subtree` 与 `bounded_visible_child_candidates` 只计算实际可见递归展开。5000条直接叶子删除与不可见删除保持增量；真正的大子树仍受4096预算约束，失败不污染原快照。 |
| H3 | 以扫描前 NextUsn 作为枚举上界会遗漏扫描中被修改的既有文件和目录；后续 Metadata 不负责插入未知项。 | `mft.rs::enumerate_initial_snapshot` 使用 HighUsn=i64::MAX，保留扫描前 journal ID/NextUsn 回放。生产 Windows 枚举与合成夹具走同一上界入口。文件、目录及未修改子项完整；create+modify重复回放幂等；未知 Metadata 不新增或复活项，不扩大原生元数据读取预算。 |
| M3 | 旧版本只在恢复时清理；压缩完成事件发出时旧输入仍可能被 worker 持有。 | `mft/cache.rs::cleanup_published_snapshot` 复用旧清理规则，目录来自实际已发布缓存路径。`spawn_snapshot_compaction_with` 先保存结果、显式释放输入快照，再发送成功或失败事件；`lib.rs::cleanup_published_caches` 在发布、释放旧服务引用且全部构建/压缩活动归还后运行。单独 apply 不清理另一 worker 未发布的文件；保留当前 base/delta。原生共享/删除失败留待后续清理，旧搜索映射仍有效。 |
| M4 | 会话持锁且拥有构建完成路径，新连接依赖旧连接的读写与扫描结束。 | `session_runtime.rs::SharedIndexerRuntime` 让原 `run_coordinator` 由服务持有，保留构建/压缩/搜索/日志调度与快照；连接只拥有可替换 I/O。服务签发租约隔离请求、EOF、错误和搜索结果，新会话仅重置一次 generation。16帧非阻塞输出队列、500ms绝对原生写期限与退休感知 reader/writer，两个 I/O worker 退出后才释放准入槽。 |
| U4 | 卷状态没有维护投影，无法区分初建、重建与就绪。 | 协议增加可选 maintenance；新 `maintenance.rs` 由原服务按卷记录 accepted/started/completed，同一关联 ID、固定状态/原因/结果、耗时与边界原生错误码。成功空快照也 Ready；重建保留 last-good，真实失败保留 last_error。原 Settings Index 页使用官方 Small info/warning/success Tag，并显示“正在重建，搜索结果可能不完整”及原因。 |
| M8 | 游标推进/更新时深拷贝 overlay 每个行名和路径；100k场景同步占用67–78ms。 | `MftOverlay.replacements: HashMap<u64, Arc<MftEntry>>`，实际修改使用写时复制；独立事务 map/set 与不可变搜索快照不变。`lib.rs`、`recent.rs` 和持久化编解码适配 Arc 借用，格式、排序、基础快照驻留策略不变。 |
| L11 | 旧格式 delta 在长度可行性校验前按头部计数分配。 | `mft/cache.rs::load_delta` 先用 checked arithmetic 验证最小 replacement/delete 表能容纳于实际 payload，再分配。损坏最大计数被拒绝，原恢复入口继续使用基础快照。 |

M4 选择服务级原协调器，而非仅缩短互斥锁：后者仍把 worker 完成与排队工作留在旧会话，无法保证重连不丢完成、不重复扫描。实际 PID/会话/受保护路径/哈希认证、3秒首帧期限、4个准入槽及1MiB输入帧上限均保留；本批不新增哈希或部署机制。当前认证连接的 command handler 取代旧连接捕获的身份。

同一发布与诊断边界还覆盖两项确认缺陷：`scan_volume_and_persist_with` 不再吞掉基础快照落盘失败，失败按构建失败处理并保留已有 last-good；`windows::win32_error` 保留 io::Error 来源，使终端诊断得到原生错误码。初建落盘失败不会伪装 Ready。增量 delta 写失败原有的可搜索内存与按卷重试策略不变。

## 协议与界面契约

外层协议保持1；`IndexDriveStatus.maintenance: Option<IndexDriveMaintenance>` 使用 serde default，None 不序列化。状态为 `initial_building`、`rebuilding { reason }`、`ready`；受限原因 `journal_wrapped`、`bulk_changes`、`cursor_invalid`、`requested` 直接来自触发点，不解析错误句子。旧报文缺省字段可读，旧 DTO 忽略新字段；没有新增外层事件或 C# DTO。

现有 Index SettingPage、卷选择、原延迟保存、焦点与滚动所有者保留。官方 Component0.8.0 Small Tag 和 Label 使用官方语义色；维护行仍40DIP，缺省状态仍保持旧196DIP元数据槽。就绪空卷显示0项。重建说明与完整 tooltip 不作为错误；速启弹窗不增加提示栏，620×410与原生生命周期未变。界面细节及测试见 `ui-outcomes.md`。

## 先红后绿

日志根目录：`D:\.pi-desktop\data\scratch\6a21b011-4230-4aa5-a2ed-f2dc1b4153e6\`。

| 边界 | 已编译的原行为失败证据 | 最终通过证据 |
| --- | --- | --- |
| H2/H3/M3/L11/服务U4 | `indexer-main-red.log`：9项预期行为断言失败，2通过、1忽略。包括直接删除预算耗尽、既有文件/目录遗漏、旧缓存未退休、损坏小头部分配991248字节及维护字段缺省。 | 最终安全库133项包括所有对应回归。初次 H2/H3/L11 定向绿测8/8，M3发布1/1。 |
| M4 | `indexer-m4-red.log`：B的HelloAck/状态不能在A停止读取且扫描被挂起时于500ms内到达。 | 五项生产 admitted-runtime 回归及三项原生临时管道退休回归通过；保留单次扫描、离线完成、新连接搜索与旧租约隔离。 |
| 协议U4 | `indexer-protocol-red.log`：维护字段不能保留。 | 三种状态、旧报文缺省与旧 DTO 忽略新增字段通过，包含于100项协议单元测试。 |
| 设置页U4 | `indexer-desktop-red.log`：`official per-volume maintenance Tag must be mounted`。 | 实际 MainWindow Index 页在920×480 Light/Dark 的文字、Tag/说明行内边界及滚动后原复选框单次保存命令通过，包含于200项桌面测试。 |
| M8 | `indexer-m8-sharing-red.log`：2000行克隆4002次分配，未满足共享断言。 | 行共享、实际修改隔离、内容相等和 owned lookup隔离通过。 |
| 发布失败/原生错误码 | `indexer-u4-publication-red.log`：2项预期断言失败，落盘失败伪报成功且原生码丢失。 | `indexer-u4-publication-green.log`：对应2项及相关握手/大子树回归4/4通过；纳入最终安全库。 |
| M3 worker交接 | `indexer-m3-handoff-red.log`：实际worker成功/失败两条路径均在完成事件接收前仍持有旧输入映射，2项预期断言失败。 | `indexer-m3-handoff-green.log`：2/2通过；`indexer-m3-handoff-safe-lib.log`：133/133通过。零容量完成通道与真实临时映射的Weak所有权检查不依赖本机删除映射文件是否失败。 |

最终库额外覆盖初建失败、排队去重/禁用取消、重新发现后维护保留、压缩发布、旧搜索可读与原生删除失败后再清理、最大损坏计数的基础快照恢复。完整 `mft/cache/journal/tests.rs` 的12项临时文件恢复/身份/截断/损坏/检查点回归均通过。

## M8 / L11 测量

百万行基础快照；Indexer包opt-level=3；单线程；setup不计入apply/classify。单次受控样本，不是分位数或已安装服务性能。

| Overlay | 修改一行 | Apply µs 前→后 | 分配次数 前→后 | 请求字节 前→后 | Classify µs 前→后 |
| ---: | :---: | ---: | ---: | ---: | ---: |
| 1k | 否 | 622→314 | 2013→13 | 330059→72067 | 50→13 |
| 1k | 是 | 477→485 | 2015→16 | 330104→72240 | 17→11 |
| 100k | 否 | 62663→30125 | 200019→19 | 22719491→4588003 | 15368→1072 |
| 100k | 是 | 60787→30125 | 200021→22 | 22719536→4588176 | 6339→340 |

Classify均零分配。100k总同步成本降为30.465–31.197ms；map/set克隆、子项索引仍为O(n)。1k元数据更新样本基本不变。原生不变游标的空闲轮询在clone之前返回，“空闲也复制”不成立。

L11一百万唯一名称行：基础文件117000260字节；owned堆保留226040775字节、峰值244176924；mapped堆峰值18136531字节。最终进程owned后工作集/私有提交为271491072/274857984字节，mapped打开后10416128/1888256，逐行访问后110419968/1888256。mapped高唯一名称夹具放弃可选搜索辅助数据，467字节堆保留值不可泛化；映射文件不等于私有提交，逐行访问不证明每个映射字节驻留。产品重建/压缩后仍保留owned表示。完整分配与进程计数见 `m8-outcomes.md`。

## 实际验证命令与结果

所有Cargo命令在同一PowerShell进程设置以下环境，串行、离线、21600秒工具超时，无超时或重复构建：

```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
$env:CARGO_NET_OFFLINE='true'
```

| 实际命令 | 结果 | 最终日志 |
| --- | --- | --- |
| `cargo test -p nxtools-indexer --lib -- --test-threads=1 --skip the_real_windows_mft_boundary_is_either_searchable_or_actionably_denied --skip the_real_windows_usn_boundary_observes_a_new_file_or_is_actionably_denied --skip refreshing_volumes_preserves_the_search_generation_high_water_mark --skip rediscovering_volumes_keeps_the_last_good_snapshot_searchable --skip stdio_handshake_returns_acknowledgement_then_current_status` | 133通过，3默认忽略，5排除 | `indexer-m3-handoff-safe-lib.log` |
| `cargo test -p nxtools-indexer --lib m3_compaction_worker_releases -- --test-threads=1` | 旧顺序2项预期断言失败；最终2通过 | `indexer-m3-handoff-red.log`、`indexer-m3-handoff-green.log` |
| `cargo test -p nxtools-indexer --bin nxtools-indexer windows_service_host::admission_tests -- --test-threads=1` | 2通过 | `indexer-final-indexer-admission.log` |
| `cargo test -p nxtools-protocol -- --test-threads=1` | 100单元+2集成通过，0文档测试 | `indexer-final-protocol.log` |
| `cargo test -p nxtools-ipc -- --test-threads=1` | 48通过，0文档测试；隔离临时管道/合成桥 | `indexer-final-ipc.log` |
| `cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop -- quick_launch index_settings volume_scope ordinary_rem_tests --test-threads=1` | 200通过；HTTP集成测试0命中 | `indexer-final-desktop.log` |
| `cargo --config 'profile.test.package.nxtools-indexer.opt-level=3' test -p nxtools-indexer --lib -- m8_million l11_million --ignored --nocapture --test-threads=1` | 基线/最终均2通过 | `indexer-measurements.log`、`indexer-measurements-final.log` |
| `cargo --config 'profile.test.package.nxtools-desktop.debug=0' clippy -p nxtools-indexer -p nxtools-protocol -p nxtools-desktop --tests -- -D warnings` | 通过 | `indexer-m3-handoff-clippy.log` |
| `rustfmt --edition 2024 --config skip_children=true --check` +本批23个Rust文件 | 通过 | `indexer-final-rustfmt.log` |
| `git diff --check` | 通过 | `indexer-final-diff-check.log` |
| `python tests/architecture/spec-contracts.py` | 通过，120文档/14所有者 | `indexer-final-spec-contracts.py.log` |
| `python tests/architecture/gpui-component-catalog.py` | 通过 | `indexer-final-gpui-component-catalog.py.log` |
| `python tests/architecture/gpui-release-contracts.py` | 通过 | `indexer-final-gpui-release-contracts.py.log` |
| `python tests/architecture/markdown-editor-contracts.py` | 通过 | `indexer-final-markdown-editor-contracts.py.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File tests/architecture/indexer-lifecycle.ps1` | 通过，源码边界检查 | `indexer-final-indexer-lifecycle.ps1.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File tests/architecture/quick-launch-lifecycle.ps1` | 通过，源码边界检查 | `indexer-final-quick-launch-lifecycle.ps1.log` |

不重复累计定向绿测；485项普通测试与2项显式测量分开统计。M3交接改动后重跑安全Indexer库133项及严格Clippy；未改动的Protocol/IPC/Desktop/性能证据沿用上表实际运行结果。未运行 `tests/process_lifecycle.rs` 的两个真实进程测试。五项排除测试会触及真实卷、系统发现或生产扫描。没有用编译、Ready或源码门禁替代原生发布验收。

## 参考源与规范

先核对Listary托管层：MiniCore IndexedSearchService/IndexerInfo/IndexerState、GUID SearchSessionToken、Options卷状态投影；未获得其原生引擎源码。再核对SwiftList的JournalReader、MftIndexScanner、初始化→UsnMonitor、幂等DeltaLinkOps、DeltaCascade、LiveIndex/文件替换清理、独立管道及每卷维护视图。旧NxTools针对性搜索未找到对应速启/MFT/USN所有者。核对入口、调用链与迁移不变量记录于 `docs/reference-code/quick-launch-research.md` 的2026-11-01节。

已同步Indexer持久化/质量、Protocol、IPC与Desktop界面契约。只进行了本任务一次定界集成复核；两项成立的发布/诊断缺陷均有合成先红后绿回归。

## 已知问题与待授权验收

- 无本批实现阻塞。未提交、推送、归档、打包、安装或操作Windows服务；保留现有下载、输入、扩展与Archive改动。
- 无关既有失败：`downloads::polish_tests::polished_calendar_eligible_keyup_and_disabled_past_preserve_draft_and_bounds` 的日期相关past hit bounds；本批未选择该测试，也未修复或压制。Archive既有10个文件格式差异保留，未运行工作区级格式重写。
- 临时管道/TestApp/合成记录不证明真实卷、已安装LocalSystem服务、物理输入、原生DPI/辅助技术或用户视觉验收。M4时限覆盖异步扫描与停止读取，不覆盖任意同步原生poll/persist/command-handler永久阻塞。
- 发布验收必须先另行授权按PACKAGING.md产生匹配的原子包，再针对该包的受保护LocalSystem服务执行以下命令；本任务未执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\tests\performance\indexer-idle-cpu.ps1 -RestartService -AssertRegression

powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\tests\performance\indexer-service-smoke.ps1 `
  -Query 'NxTools.Indexer.exe' -Limit 20 -TimeoutMs 10000
```

这些门禁要求实际CPU/I/O、重启后的持久化base+delta、真实服务搜索及批准/运行的原子保护二进制一致性；不能使用未更新的旧安装服务证明新源码通过。
