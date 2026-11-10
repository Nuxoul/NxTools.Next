# 第三批：索引服务正确性与资源（H2、H3、M3、M4、U4，附 M8 测量、L11）

```text
你在 Windows 工作区 D:\Code\NxTools.Next-main 中工作（Rust + GPUI 桌面端，C# PlatformHost，Rust LocalSystem 索引服务 NxTools.Indexer.exe，PowerShell）。任务：修复索引服务的 H2、H3、M3、M4，实现对应的用户可见状态 U4，并对 M8 做测量、修复 L11 中可确认的部分。

## 通用规则
- 工作区里有他人未提交的改动，必须保留；不要回退、格式化或提交与本任务无关的文件。不要 commit/push，不要打包、安装、启动产品，不要安装、启动或重启 Windows 服务，不要访问真实网络和真实用户数据，不要读取真实卷的 MFT（只用已有的合成夹具和单元测试）。
- 开始前阅读 AGENTS.md（尤其 “Indexer release gate” 与 Diagnostic contract）、.trellis/workflow.md、.trellis/spec/nxtools-indexer/**、.trellis/spec/nxtools-protocol/**、.trellis/spec/nxtools-ipc/**、.trellis/spec/nxtools-desktop/backend/quality-guidelines.md 与 ui-experience-contracts.md、docs/architecture.md 的 “Quick-launch phase-2” 与 “Initial NTFS snapshot” 两节、docs/reference-code/ 中速启/索引相关笔记。按 workflow 在 .trellis/tasks/ 下建立本任务记录（建议 11-0x-indexer-journal-correctness/，含 prd.md、outcomes.md）。
- 先写能在旧代码上失败的回归测试，再修复；报告中给出先红后绿的证据。
- Rust 命令在同一进程内设置 $env:CARGO_BUILD_JOBS='1'; $env:CARGO_INCREMENTAL='0'; $env:RUSTFLAGS='-C codegen-units=256'；桌面测试使用 --config 'profile.test.package.nxtools-desktop.debug=0'，并加 -- --test-threads=1。给足超时；超时后先检查残留的 cargo/rustc 进程，不要重复启动构建。
- 已知的无关失败：downloads::polish_tests::polished_calendar_eligible_keyup_and_disabled_past_preserve_draft_and_bounds（依赖当天日期）；nxtools-archive 既有的格式差异。不要修、不要压制，在报告中列出。
- 行号以 2026-11-01 的工作区为准；如有偏移，按符号名定位。标 † 的事实只经过初步阅读，修复前先核实；不成立就在报告中说明并跳过。
- 不得声称做过真实卷、服务、安装或用户验收。

## 1. H2：批量删除超过工作量预算，触发整卷重建（已确认）
- rust/apps/nxtools-indexer/src/usn.rs:30 `MAX_RECURSIVE_WORK_UNITS_PER_BATCH = 4_096`；:211 的 `live_work` 在一次 `apply_journal_batch`（:177）内累加，而一次轮询最多读取 MAX_READS_PER_POLL 个 256 KiB 缓冲（:723-777）。
- :234 每条 Delete 记录都调用 `delete_visible_subtree`（:417）。该函数在循环开头 :429 先 `reserve_recursive_work`，再判断是否可见；被删的普通文件、甚至从未进过索引的项都会计费。
- :490 `bounded_visible_child_candidates` 用 `snapshot.base_children(parent).len()` 与剩余预算比较（:496-505）。这个数包含已被 overlay 标记删除的子项，所以删除一个在基础快照中子项超过 4096 的目录总会触发 RebuildRequired。
- 触发：一次轮询内删除超过 4096 个文件（cargo clean、删除 node_modules、清空回收站）。影响：整卷重新枚举 MFT。
- 要求：只对真正需要展开的可见目录子项计费；叶子文件和不可见项不计。候选计数前排除 overlay.deleted 中的子项。真正的大子树仍须受预算约束（保留现有 “large subtree remains bounded” 语义与测试）。
- 回归：参照 usn.rs 中现有 upsert 预算测试（约 :1282-1315），新增 “5000 个可见叶子文件的删除批次 → Ok，且全部从可见集合移除”、“删除一个基础子项超过 4096 但其中大部分已在 overlay 中删除的目录 → 不触发重建”，并保留 “真正的大子树 → RebuildRequired” 的断言。

## 2. H3：首次扫描或重建期间被修改的已有文件永久缺失（代码路径已确认）
- rust/apps/nxtools-indexer/src/mft.rs:612-619：先 `query_journal`，再 `enumerate_mft(handle, journal.NextUsn)`，:662-667 把它作为 MFT_ENUM_DATA_V0.HighUsn。按 FSCTL_ENUM_USN_DATA 的文档语义，扫描期间被修改（最新 USN > HighUsn）的已有文件不会被枚举返回。
- 之后从 NextUsn 回放日志：这些文件只有 DATA_EXTEND/CLOSE 等元数据原因，归入 `UsnRecordKind::Metadata`。usn.rs:215-229 的 Metadata 分支只更新 `visible_entry` 已存在的项，从不新增。新建文件有 FILE_CREATE，不受影响；受影响的是“扫描前已存在、扫描中被写入”的文件。
- 影响：这些文件不出现在搜索中；delta 照常持久化，服务重启后仍缺失。目录是否同样受影响（†）请核实。
- 要求（推荐方案）：枚举使用 HighUsn = i64::MAX，起始游标仍保留扫描前读到的 NextUsn；回放时 upsert 必须幂等（核实现有实现，补测试）。如果你认为对 Metadata 中的未知引用做按 ID 回读并插入更合适，需要说明它对 MAX_METADATA_READS_PER_POLL（usn.rs:27）的预算影响，并给出理由。
- 回归：用合成的 MftEnumeration/journal 夹具（参考 tests.rs、mft.rs 现有测试）构造“枚举阶段缺失某项，日志只有 Metadata 记录”的场景，修复后该项可见；“扫描期间新增 + 修改”的组合不重复、不丢失。

## 3. M3：旧快照文件只在服务启动时清理（已确认）
- rust/apps/nxtools-indexer/src/mft/cache.rs:471 `persist_system_snapshot` 每次写入新的 `{prefix}-{nonce}.idx`；`cleanup_obsolete_cache_files`（:544）唯一的生产调用点是恢复流程 :425。
- 影响：每次重建或压缩都在 %ProgramData% 留下一份完整快照，在服务生命周期内持续累积；与 H2 叠加时增长更快。
- 要求：新快照发布、旧快照的内存映射全部释放之后再清理旧文件。Windows 不能删除仍被映射的文件，函数本身已容忍删除失败，仍被映射的文件留到下次清理。不得删除当前 base/delta 对，也不得删除恢复流程依赖的文件。
- 回归：连续两次重建（合成快照、临时目录），每次在旧映射释放后，目录中只剩当前 base/delta 对。

## 4. M4：索引会话在整个生命周期内持有服务锁（加锁范围已确认，影响为高可信风险）
- rust/apps/nxtools-indexer/src/windows_service_host.rs:215 在握手成功后 `service.lock()`，然后整个 `serve_admitted_session`（:219）都在锁内运行，结束后才更新 published_status（:228）。
- 影响（†）：桌面在扫描或重建期间重启后，新会话收不到 HelloAck、状态和搜索结果，要等旧会话（其桌面已经消失）结束；旧会话可能卡在构建过程中。
- 要求：先读懂会话、构建 worker 与搜索调度的所有权，再选一个最小方案并写明理由，例如：
  (a) 锁只包住单个命令的处理，不包住会话读循环；
  (b) 同一已认证桌面身份的新会话握手时，通知旧会话退出，并有界等待。
  不得削弱现有准入（PID/会话/路径/哈希认证、3 秒握手期限、4 个 pending 槽位、1 MiB 帧上限）。
- 回归：会话 A 已连接但不再读取时，会话 B 在限定时间内收到 HelloAck 和状态；A 的残留不影响 B 的 generation 语义。

## 5. U4：设置页显示索引维护状态
- 现状：rust/crates/nxtools-protocol/src/lib.rs:376 `IndexStatus { phase, indexed_items, drives, last_error }`，:384 `IndexDriveStatus` 没有任何“首次建立 / 重建中 / 原因”的字段。按 AGENTS.md，有可搜索的 last-good 快照时的 RebuildRequired 属于维护状态，不是用户可见的失败，last_error 只用于真正的轮询、压缩、持久化或 worker 失败。
- 要求：
  1. 协议增加向后兼容的可选字段（serde default），按卷表达：首次建立、正在重建（含受限的原因码，例如 journal_wrapped / bulk_changes / cursor_invalid）、就绪。阅读协议兼容规范，确认旧桌面能忽略新字段。
  2. 索引服务在维护状态转换时更新该字段，并记录维护事件（只记状态转换，不按记录打日志）。
  3. 桌面端速启设置的“索引”页（rust/apps/nxtools-desktop/src/main_window/quick_launch_page.rs，找到现有卷状态的渲染处）用官方 Tag 或 Alert 显示每卷状态；重建中显示“正在重建，搜索结果可能不完整”和原因。不要把维护显示成错误。
  4. 速启弹窗本身不新增提示栏（遵守 .trellis/tasks/10-30-quick-results-ux/design.md 的结果行契约和 620×410 预算）。
- 回归：协议往返序列化（含缺省字段的旧报文）；服务状态转换；设置页生产渲染下三种状态的文字与边界。

## 6. M8：每次有变化的轮询都克隆整个快照 overlay（先测量，再决定）
- usn.rs:206 `let mut updated = snapshot.clone();` 会深拷贝 overlay（最大可达基础行数的 10%，见 :315 `overlay_requires_compaction`）；还有 :212 `overlay_children_index` 全量重建等成本。“空闲轮询也复制”（†）请核实。
- 要求：先写一个默认 #[ignore] 的测量测试（合成 100 万行基础快照，overlay 分别为 1k 和 100k 项），测量单次 `apply_journal_batch` 和 `classify_applied_update` 的耗时与分配次数；测量值写入 outcomes。只有在 100k overlay 下单次轮询明显影响搜索调度（报告给出数据和你的判断依据）时，才实施 Arc + 写时复制或原地修改加失败回滚；否则只记录结论。不要为测量在产品中加入高频日志。

## 7. L11（†）：快照内存驻留与旧格式 delta 预分配
- mft/cache.rs 约 :614 的旧格式 delta 加载器，按文件头声明的数量（最高 u32::MAX）预分配，然后才校验文件长度。若属实：分配前先用文件长度约束数量，损坏文件应被拒绝并回退到基础快照，而不是中止服务。补损坏头部的回归测试。
- mft.rs 约 :542 重建或压缩后，新快照以堆内完整副本保留，而不是重新映射已持久化的文件：只测量并报告内存差异，本批不改。

## 8. 验证
- cargo test -p nxtools-indexer（包括 mft/cache/journal/tests.rs）
- cargo test -p nxtools-protocol
- cargo test -p nxtools-ipc
- 桌面：过滤运行 quick_launch 设置页与索引状态相关测试
- cargo clippy -p nxtools-indexer -p nxtools-protocol -p nxtools-desktop --tests -- -D warnings；对改动文件做 rustfmt 检查；git diff --check；改动规范文档时运行 python tests/architecture/spec-contracts.py
- AGENTS.md 的 Indexer release gate（打包后的 LocalSystem 服务上运行 tests/performance/indexer-idle-cpu.ps1 -RestartService -AssertRegression，再做真实服务搜索冒烟）需要打包和服务重启，不在本任务授权内。不要执行，在报告中列为“待授权执行”，并写出确切命令。
- 按 AGENTS.md 明确写出检查过哪些参考源（只观察到 Listary 托管层，不要暗示看过其原生引擎源码）。

## 9. 范围外
审查记录中的其他问题、速启弹窗的任何布局改动、搜索排序与引擎改动、依赖升级、打包、服务安装与重启。

## 10. 完成后的报告
每个问题的根因结论、改动文件与符号、方案理由（尤其是 H3 的枚举方案和 M4 的会话方案）；协议兼容说明；先红后绿证据；M8 的测量数据与结论；完整验证命令的实际结果；待授权执行的发布门禁；未完成事项与无关的既有失败。
```
