# 方案书：按 MDM 重做 NxTools HTTP 分段下载的调度、重试与限连

> 本文件是交给实现模型的完整任务说明。行为规格只以 Mister Download Manager（下称 MDM）为准，不参考其他下载器。

## 0. 你的任务

你要修改 `D:\Code\NxTools.Next-main` 的 Rust 下载引擎（`rust/crates/nxtools-download`），让 HTTP 分段下载在**慢连接、服务器限连、响应体中途断开**三种情况下的行为和 MDM 一致，并通过第 7 节的测试和第 8 节的基准验收。

开始前必须按顺序读：
1. `AGENTS.md`（构建内存约束、诊断日志约定、参考迁移流程）。
2. `.trellis/spec/nxtools-download/backend/quality-guidelines.md` 第 194-210 行（连接上限现行契约，本任务会改写其中一部分，见 §4.5 和 §5）。
3. 本文 §2 列出的 MDM 源码。MDM 是 GPL-3.0：**只按行为重新实现，不复制代码、注释、测试或资源**（见 `docs/reference-code/mister-download-manager.md:5`）。

## 1. 要修的问题（已用基准日志确认）

基准：`.diagnostics/download-bench/20261112-125137-release66-retest/`。逐请求日志在 `server-<场景>-<客户端>-<序号>-*.jsonl`，入口脚本是 `runner/retest_download_comparison.py`。

| 场景（labserver 参数） | NxTools R66 现象 | 根因 |
|---|---|---|
| `slowconn-32m`：`rate=4M&slow_every=3&slow_rate=128K`，16 连接 | 16.04 s（MDM 2.43 s）。11 个快分段 0.5 s 下完，5 个 2 MiB 慢分段各用 16 s，之后没有任何新请求 | 空出连接后找拆分候选，要求剩余量 ≥ `min_segment_mib`（3 MiB），2 MiB 分段永远不满足，空闲连接闲置；也没有慢连接重连（`engine/segmented_runner.rs:1592-1593,815-842,2091-2135`） |
| `maxconns-50m`：`rate=2M&max_conns=2`，超额响应回 503、无 Retry-After | 0/3 成功，每次 14 个分段 × 3 次 503 = 42 次后失败 | 非 206 一律报成 `RangeNotHonored`（`http.rs:2355`）；限连学习只认原生 TCP 拒绝（`engine/connection_ceiling.rs:126-131`）；每个分段 3 次 / 100、200 ms 退避（`engine.rs:3312-3313`）；任一分段耗尽重试就让整个任务失败（`segmented_runner.rs:771,2044-2047`） |
| `drops-50m`：`rate=4M&drop_after=2M&drop_every=2` | 0/3 成功，`error decoding response body` | 重试时从本次运行开始时的偏移重下，断开前写的数据不存盘也不记进度（`segmented_runner.rs:722-805`，`:778`；`http.rs:1044-1047`），一次运行重复下载了 24 MiB；分段 3.125 MiB 大于 2 MiB 断流点，连续三次碰上会断流的请求就失败 |

## 2. MDM 行为规格（实现必须逐条对应）

源码根目录：`D:\Code\misterdownloadmanager-main\misterdownloadmanager-main\crates\engine\src\`

### 2.1 调度器 `planner.rs`（纯逻辑，不做 I/O）
- 分段字段：`start`、`cursor`（已写盘到哪）、`reserved`（已认领到哪）、`end`（不含，拆分时会变小）、`worker`、`receiving`（本次分配已收到数据）、`given_at`（`planner.rs:22-38`）。
- `assign(w, after)`（`planner.rs:182-236`）：先 `release(w)`，然后按顺序：
  1. 取一个无人负责且还有未认领字节的分段，优先选 `start == after` 的（接着自己刚下完的位置继续）；
  2. 抢占：`start == after` 的下一分段已经有人负责，但还没收到数据，且分配时间 ≥ `preempt_after`，就把它转给 w，原负责者的下一次认领会返回 `Lost`；
  3. 拆分：选未认领字节（`end - reserved`）最多的分段，并列时取下标小的；`half = unclaimed / 2`，`half < min_split` 时返回 `None`（这个 worker 结束）；`mid = reserved + half`，向下对齐到 4096 后若仍 ≥ `reserved + min_split` 就用对齐值；原分段 `end = mid`，新分段 `[mid, 原 end)` 交给 w。**原连接不停**。
- `claim(w, pos, len)`：`pos` 必须等于 `reserved`，可认领量 `n = min(len, end - reserved)`，`reserved += n`，`n > 0` 时标记 `receiving`；不再负责任何分段时返回 `Lost`（`planner.rs:245-259`）。拆分点从 `reserved` 算起，所以永远切不到正在写的字节（`planner.rs:11-13`）。
- `commit(w, upto)`：`cursor = clamp(upto, cursor, reserved)`；`cursor == end` 时释放 worker 并返回 `Some(end)`（`planner.rs:264-273`）。
- `release(w)`：放弃未提交的认领，`reserved = cursor`（`planner.rs:277-283`）。
- `active_workers` = 有人负责的分段数；`receiving_workers` = 其中已收到数据的（`planner.rs:170-180`）。

### 2.2 连接 worker 循环（`download.rs`）
- 请求范围：从 `from - min(written, verify_overlap)` 到 `to - 1`；重叠的这 64 字节必须和磁盘上的一致，否则报 `FileChanged`，这是致命错误（`download.rs:2667-2677,2966-2975`）。
- 接收：先缓冲，满 `BATCH = 256 KiB` 或距上次写入超过 `FLUSH_EVERY = 1 s` 时，按"认领 → 写入 → 提交"处理（`download.rs:38-41,3114-3131`）。认领量比收到的少时（分段被拆了），写完认领的部分就停止读取；请求的范围没读完的，直接丢掉这个响应体，不再读完（`download.rs:3068-3077`）。
- 出错时：先把已认领的缓冲数据写入并提交，再返回错误；响应体提前结束报 `ShortBody`（`download.rs:3061-3065,3079`）。
- 慢连接重连（`download.rs:43-50,3046-3054,3085-3110`）：
  - 收数据期间最多每 500 ms 检查一次；`speed = 本响应已收字节 / 本响应已用时间`；
  - `best_speed` 取本次下载中所有收过 ≥ 256 KiB 的响应里速度的最大值；
  - 同时满足以下条件就断开：允许重连、没有任何限速生效、`elapsed ≥ 2 s`、`speed × 4 < best`、`剩余 / speed > 2 s`（剩余 = 分段 end − 当前位置）；
  - 断开时先把已收数据写入并提交，返回 `SlowConnection`。worker 释放分段后立即重新 `assign`，**不计失败**（`download.rs:2709-2713`）。
- 出错后的处理顺序（`download.rs:2703-2775`）：
  1. 释放分段；
  2. 致命错误：整个下载停止；
  3. `SlowConnection`：立即重新分配；
  4. 被拒绝（`refused()`）、还有其他 worker 在负责分段、主机上限已学到、且 `working ≥ cap`：这个 worker 标记为"已退役"后正常退出；
  5. 链接失效（`link_dead()`）：有其他 worker 时这个 worker 退出，否则整个下载失败；
  6. 其余：本连接累计下载量 `received` 比上次失败时多，就把 `failures` 清零；然后 `failures += 1`；超过 `max_retries` 时，有其他活跃 worker 就正常退出，否则返回错误；没超过就按 `retry_wait` 等待后重新 `assign`。
- `retry_wait`（`download.rs:398-409`）：
  - 有 `Retry-After` 的 `Throttled`：等 `min(after, 300 s)`；
  - `failures == 1`：立即重试；
  - 否则：`retry_delay × 2^(failures - 2)`，上限 30 s。
- 错误分类（`download.rs:244-260,326-395`）：
  - `Throttled(status, retry_after)` 只用于 429/503；`Retry-After` 支持秒数和 HTTP-date（`download.rs:2853-2854,3231-3242`）；
  - `refused()`：`Throttled`、403、连接阶段的错误；
  - `link_dead()`：401、403、404、410、416；
  - `is_fatal()`：文件已变、I/O/磁盘错误、取消等。

### 2.3 主机上限学习（`hosts.rs:1-75`）
- `refused(host, working)`：`working = max(working, 1)`，`cap = max(cap, working)`，`seen += 1`；`seen ≥ 2` 时才生效并返回 cap。
- `working(host, n)`：同时收数据的连接数超过 cap 时把 cap 调高，不会调低。
- 10 分钟没再观察到就作废。按 **host 字符串**索引，不含协议和端口。
- 学到的上限对本次下载和之后同主机的下载都生效。

### 2.4 主管 `supervise`（`download.rs:2543-2620`）
- 初始 worker 数 = `min(学到的上限, 连接数设置, ceil(文件大小 / 64 KiB))`，至少 1。
- 某个 worker 返回致命错误：停止全部连接，下载失败。返回其他错误：记为 `last_err`。
- 文件下完：停止仍在重试中的连接，结束。
- worker 全部退出但文件没下完：有 `last_err` 就失败；没有就再起一个 worker。

### 2.5 常量（`download.rs:38-61,184-206`；`planner.rs:20`；`hosts.rs:13-15`）

| 名称 | 值 |
|---|---|
| `min_split` | 12 KiB |
| `max_retries` | 5 |
| `retry_delay` | 1 s（翻倍，上限 30 s） |
| Retry-After 上限 | 300 s |
| `verify_overlap` | 64 B |
| `BATCH` | 256 KiB |
| `FLUSH_EVERY` | 1 s |
| 慢连接：宽限 / 速度倍数 / 剩余时间 | 2 s / 4 / 2 s |
| 计入 best_speed 的最小样本 | 256 KiB |
| 拆分对齐 | 4096 |
| `preempt_after` | `max(2 × 探测首字节时间, 20 ms)` |
| 每多少字节开一个初始连接 | 64 KiB |
| 主机上限：生效所需观察次数 / 作废时间 | 2 / 10 min |

## 3. 已做出的设计决定

1. **执行模型改成 MDM 的模式**：每条连接一个 worker，循环向本次运行共享的调度器要活；拆分时原连接继续下载，靠"先认领再写"保证不重叠。取代现在"一个分段一个任务、先停下再拆"的做法，同时取代 `pending_rebalance`/`Rebalanced` 这条按需拆分路径。
2. **NxTools 自己的持久化和生命周期保留**：SQLite 传输计划、分段检查点、暂停/取消/恢复、临时文件和完成发布、HLS/DASH、单流降级、探测、请求上下文、限速器，都继续由原有代码负责（映射规则见 §4.3）。
3. **503/429 计入跨任务的主机上限**（MDM 的做法），推翻 REF-13 里"不从 HTTP 503 学习"的约定；上限表的主机键改为不含协议和端口的 host。原有的路由、代理、证书、系统客户端这些作用域维度保留。
4. **HTTP 分段下载不再使用 `adaptive_segment_policy`**：MDM 没有自适应扩容或收缩，按需拆分已经覆盖它的作用。这个字段在请求、设置、IPC、数据库里的结构都不动。设置页上这三项对 HTTP 分段下载不再生效，界面如何处理另开任务，本任务不改 UI。
5. 不在本任务范围：MDM 复用探测 GET 的响应（REF-14 已决定不采用）、多 IP 竞速、按需提高连接数上限、POST 下载。

## 4. NxTools 改造说明

### 4.1 新增：本次运行的调度器
- 在 `engine/` 下新建模块（例如 `segment_planner.rs`），按 §2.1 实现，纯逻辑、可以单独做单元测试。
- 从运行开始时的 `TransferPlan` 初始化：`cursor = start + 已持久化的下载量`，`reserved = cursor`，`end = end_byte + 1`。
- 拆分新建的分段，`index` 取当前最大 index + 1。
- 新增 `synced_cursor`：记录已经 `sync_data` 过的位置，只有它能写进检查点。

### 4.2 改写 `segmented_runner.rs`
- `run_segmented_task_with_samples` 改为 §2.4 的主管逻辑，`spawn_segment_transfer` 改为 §2.2 的连接 worker 循环。
- 初始分段布局 `create_initial_segments` 不变，它的效果等同于 MDM 逐个连接对半拆分后的结果。初始 worker 数按 §2.4 计算。
- `HttpDownloadClient::download_segment` 要支持：
  - 请求起点带 64 字节回读校验；
  - 在写入前，用调度器给出的认领量限制本次写多少；
  - 返回时报告已写并已同步的字节数和结束原因：完成、被拆分后到达新终点、`Lost`、慢连接重连、错误。
  `SegmentWriter` 的写入上限不能再在打开时固定死；可以改成每次写入前按认领量检查，也可以让 `download_segment` 直接按认领量控制。

### 4.3 持久化映射（必须保持的不变式）
- 写检查点前一定先同步写入器；检查点里的下载量取 `synced_cursor - start`。
- 拆分顺序：在调度器锁内改原分段的 `end`、插入新分段 → 释放锁 → 在数据库里持久化新计划（原分段的新 end 加上它当前的检查点，以及新分段）→ 新 worker 才能发请求。持久化失败按存储错误处理，属于致命错误。
- 每 2 s（MDM 的 `save_interval`）统一同步一次各写入器、写一次检查点；暂停、取消、完成、致命错误时也各写一次。
- 暂停：所有 worker 写入并同步已认领的数据 → 写检查点 → 状态改为 Paused。恢复：按 §4.1 从计划重建调度器；If-Range 严格校验的规则不变。
- 进度显示：`progress_by_segment` 和 `segment_lengths` 在拆分时一起更新，加锁顺序沿用 `quality-guidelines.md:163` 的规定；持有这两个锁时不做网络或持久化操作。
- 不能持有调度器锁去做 I/O，包括网络、磁盘和 SQLite。

### 4.4 错误分类（`http.rs`）
- `validate_segment_response`：
  - 429/503 → 新增 `HttpDownloadError::Throttled { status, retry_after }`，`kind() = "throttled"`，错误文案为"服务器繁忙"；
  - 其他非 2xx → `Status(status)`；
  - 只有 2xx 且不是 206 才保留 `RangeNotHonored`，单流降级和断点续传失败后重新开始这两条判断保持原样。
- 按 §2.2 实现三种分类，分别对应 MDM 的 `refused()`、`link_dead()`、`is_fatal()`：
  - 被拒绝：`Throttled`、403、`Client` 错误中 `is_connect()` 为真的；
  - 链接失效：401/403/404/410/416；
  - 致命：存储/IO、范围或长度校验失败、资源验证器不一致、回读校验不一致、取消。
- 新增 `ShortBody`（响应体提前结束）和 `SlowConnection`（内部结束原因，不对外暴露）。

### 4.5 主机上限（`engine/connection_ceiling.rs`）
- 按 §2.3 修改：被拒绝的判断改用 §4.4 的分类；`working = max(本次运行中正在收数据的连接数, 1)`，前提是至少还有一个其他 worker 在负责分段；观察到 2 次后生效；收数据的连接数更多时上调；10 分钟作废；键为 host 加原有作用域维度。
- 准入：初始 worker 数取学到上限的较小值（§2.4），运行中靠 §2.2 的退役机制收缩。
- `download.connection_ceiling.learned` 事件的 `reason` 改为 `refused_extra_connection_with_working_peers`，并增加 `status` 字段（HTTP 状态码或原生错误码）。

## 5. 必须同步修改的文档

1. `docs/reference-code/mister-download-manager.md`：在 "Engine and Ghost comparison" 表和 B6 一节补上 §2 的源码入口和不变量；把 "Mister host ceiling learning"、"Mister slow connection retirement" 两行的结论改为"采用"，注明本任务。
2. `.trellis/spec/nxtools-download/backend/quality-guidelines.md`：
   - 第 199-210 行的连接上限契约按 §4.5 重写，删去"不从 HTTP 503 学习"的反例；
   - 新增一节"分段调度器"：认领/提交/释放的不变式、拆分持久化顺序、检查点只用已同步的位置、重试规则、慢连接重连条件、退役和主管规则，以及 §7 的测试名；
   - 第 14、25 行以及其他描述"先停下再拆分"、"自适应"的条目，改成新模型。
3. `.trellis/tasks/11-11-reference-b6-connection-experiments/outcomes.md` 不改；在本任务目录写 `outcomes.md`，说明为什么推翻 REF-13 和 REF-15 的结论，并附基准证据。

## 6. 保持不变

- `run_single_stream_task`、HLS/DASH 执行器、探测和解析、请求上下文、网络路由和代理、证书策略、限速器（任务、全局、运行时）、队列和完成协调、存储结构（schema 13）、IPC、桌面 UI。
- 诊断日志不记录 URL、请求头、Cookie，也不按字节或按采样点记录进度。
- 不加功能开关，不保留旧调度器的第二条路径，不新增依赖或设置项。

## 7. 测试（先写测试并确认旧实现失败，再改实现）

测试服务器：扩展 `engine/connection_experiments.rs` 里现有的 Python 受控服务器，或 `tests/support/mod.rs` 的 `DeterministicHttpServer`，增加四种故障模式：`max_conns`（超额回 503，可选带 `Retry-After`）、`drop_after`/`drop_every`、`slow_every`/`slow_rate`、按请求序号返回指定状态码。每个请求都记录起止偏移、状态、已发送字节、结果。

| 测试 | 设置 | 断言 |
|---|---|---|
| `planner_*`（单元测试） | 纯调度器 | 三条分配规则的优先级；拆分对齐和 `min_split`；拆分后的认领量；`Lost`；`release` 把 reserved 退回 cursor；`commit` 的范围限制；任何操作序列下分段都首尾相接、刚好铺满整个文件 |
| `body_drop_resumes_from_written_offset` | 8 MiB，4 连接，每第 2 个响应在 1 MiB 后断开 | 任务完成，字节完全一致；断开后的下一次请求起点 = 断开处 − 64；服务器总发送量 ≤ 8 MiB + 请求次数 × 64 B；没有出现 `failures > 1` |
| `throttle_learns_cap_and_retires_extra_connections` | 8 MiB，8 连接，同时最多 2 个，超额回 503 | 任务完成；503 次数 ≤ 8；同时发送响应的峰值 ≤ 2；学到的上限 = 2；同一 host、换一个端口启动第二个任务时 503 = 0，初始 worker 数 = 2 |
| `retry_after_is_honoured_and_cancellable` | 单连接，前两次 503 带 `Retry-After: 1` | 两次重试间隔 ≥ 1 s；等待期间暂停能在 200 ms 内生效；恢复后完成 |
| `exhausted_connection_retires_while_others_work` | 某个范围连续回 500 六次后恢复 | 任务完成；把所有请求都改成持续回 500 时，任务在有限次请求内失败，错误是 `Status(500)` |
| `slow_connection_is_shed_and_tail_is_split` | 8 MiB，8×1 MiB，每第 3 个请求 64 KiB/s、其余 4 MiB/s | 完成且字节一致；耗时 < 4 s；出现慢连接重连；拆分后原连接的响应被丢弃（服务器记录为 aborted）；开启任务限速后不发生慢连接重连 |
| `split_while_writing_never_overlaps_and_survives_pause` | 拆分发生的同时暂停，然后恢复 | 持久化计划首尾相接；每个分段的检查点 ≤ 已同步字节；恢复后最终字节完全一致 |
| `overlap_mismatch_is_fatal` | 恢复时把磁盘上重叠区内的字节改掉 | 报 `FileChanged`，整个任务失败，不再重试 |

现有测试处理方式：
- `connection_ceiling::tests`、`connection_experiments::b601`、`ref13_*`：按新契约改写断言（503 参与学习、键改为 host），原先"503 不学习"的断言删除，并在 outcomes 里说明。
- 只覆盖已删除的自适应扩容/收缩或"停下再拆分"的测试（例如 `sampled_runner_shrinks_a_grown_saved_plan_and_resumes_exact_artifact`、`connection_experiments/ref15.rs`）随这些代码一起删除，并在 outcomes 里逐个列出替代它的新测试。
- `b4_coverage_contract`、`engine_contract`、`http_client_contract`、`b5_completion_contract`、`resolution_contract`、`storage_contract` 以及整个 Download lib 必须全部通过。

## 8. 验收

先跑定向测试，再跑完整套件，最后打包（读 `PACKAGING.md`），用 `runner/retest_download_comparison.py` 和 MDM 同日交替对比：

| 场景 | 通过条件 |
|---|---|
| `slowconn-32m` | 3/3 成功，中位耗时 ≤ 3.0 s |
| `maxconns-50m` | 3/3 成功，中位耗时 ≤ MDM × 1.10；正式运行的 503 次数 ≤ 2（预热时学到上限） |
| `drops-50m` | 3/3 成功，中位耗时 ≤ 1.5 s |
| `fast-1g`、`capped-100m`、`latency-100m`、`small-256k` | 中位耗时变化不超过 R66 的 +5%，全部字节一致 |

## 9. 构建约束

- 每条 Cargo 命令都设置 `CARGO_BUILD_JOBS=1`、`CARGO_INCREMENTAL=0`、`RUSTFLAGS=-C codegen-units=256`，超时按整个构建时长估算；超时后先检查是否还有 cargo/rustc 在运行，不要重复启动。
- 一次只跑一条 Cargo 流水线。打包前先检查可提交内存的余量，满足才开始。

## 10. 交付报告

按以下顺序给出：
1. 修改的文件和各自负责的部分；
2. 每个新测试先失败、后通过的命令和结果；
3. 完整套件的命令、列出的测试数和运行数、退出码；
4. 基准表（R66、新版本、MDM 同日对照）；
5. 删除的测试和替代它们的测试；
6. 已知问题。
