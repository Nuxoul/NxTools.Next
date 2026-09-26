# Rayburst 对 NxTools.Next 下载引擎的优化方案书

> 审计对象：`D:\Code\rayburst-main`（只读参考）
>
> 目标对象：`NxTools.Next` 的 Rust 下载引擎、桌面宿主、IPC 和任务展示边界
>
> 证据标记：`[源码证实]` 表示可由读取到的源码/测试直接证明；`[架构推断]` 表示基于调用链和现有契约提出的设计判断。

## 1. 结论摘要

Rayburst 和 NxTools.Next 不是同一种下载架构：

- Rayburst 的 Rust/Tauri 代码主要负责浏览器意图、请求收据、任务策略、历史和 Aria2 JSON-RPC 编排；最终传输、分段、恢复和输出发布由独立的 Aria2 Next sidecar 负责。
- NxTools.Next 在 `rust/crates/nxtools-download` 内拥有 HTTP 探测、单流/分段传输、HLS/DASH、限速、队列、SQLite 分段 checkpoint、原子最终化和事件序列。

因此，[架构推断] Rayburst 最值得迁移的是“身份、边界和失败语义”，不是 Aria2 适配层或 Vue store 结构。当前最重要的优化项是：

| 优先级 | 建议 | 原因 |
| --- | --- | --- |
| P0 | [架构推断] 为外部提交增加持久化幂等收据、请求指纹和冲突语义 | 当前 `request_id` 是相关 ID，不是持久幂等键；浏览器 handoff 只按 `handoff_id` 去重，不能发现同一 ID 对应的请求体变化 |
| P1 | [架构推断] 增加下载引擎实例 generation，并让 snapshot/event 在重连时按 generation 重同步 | 当前 snapshot 的 `generation` 固定为 `1`，event sequence 在新引擎实例中从零开始，跨宿主重启的 stale-event 边界不够明确 |
| P1 | [架构推断] 明确 `Retry` 与 `Redownload` 的终态操作契约 | 当前 `Failed -> resume` 能重新入队，但没有把“保留 checkpoint 重试”和“新任务重新下载”区分成稳定的产品契约 |
| P1 | [架构推断] 轻量化常规 snapshot，延续已有独立 `TaskDetail` 边界 | 当前 `command_snapshot` 为所有任务构造 `DownloadEngineTask`；大任务量下会把探测、路径和错误投影重复带过 IPC |
| P1 | [架构推断] 在现有 typed fault/diagnostic 基础上补齐提交阶段和宿主 readiness | 不应引入 Rayburst 的 RPC；应把其阶段、终态和重连语义应用到 NxTools 已有的 Rust/IPC 边界 |
| P2 | [架构推断] 按实际产品需要引入显式 internal/visibility policy、更多类别匹配能力和历史投影 | Rayburst 的 P2P/Aria2 专属能力不适用于当前 HTTP/HLS/DASH 引擎，只有出现对应产品需求时才值得增加 |

## 2. 许可证与可复用边界

### 2.1 已确认的许可信息

- `[源码证实]` `D:\Code\rayburst-main\LICENSE` 是 MIT License，版权归属为 `AnInsomniacy`。
- `[源码证实]` `D:\Code\rayburst-main\src-tauri\Cargo.toml` 的 package license 也是 MIT。
- `[源码证实]` `D:\Code\rayburst-main\src-tauri\generated-binaries\README.md` 明确写出：native messaging launcher 是本仓库生成物，而 bundled engine binaries 来自独立的 Aria2 Next 项目。
- `[源码证实]` `D:\Code\rayburst-main\docs\DOWNLOADS.md` 将 Aria2 Next 列为独立责任方，并把输出路径、冲突、恢复、传输和完成媒体发布归给它。

### 2.2 不能据此推出的结论

Rayburst 的 MIT 许可不自动覆盖 Aria2 Next 的源码、sidecar 二进制、补丁或其依赖。此次在 Rayburst 仓库内没有发现 Aria2 Next 源码、独立 sidecar commit、构建 provenance 或二进制 hash 记录；`Cargo.lock` 也没有一个 `aria2-next` Rust package。

因此本方案采用以下规则：

1. 可以引用 Rayburst 自有 Rust/TypeScript 代码中的架构事实，并在独立实现中复现行为。
2. 不复制 Rayburst 的 Aria2 RPC、任务服务或 Vue 代码到 NxTools.Next。
3. 不复制或重新分发 Aria2 Next 相关代码/二进制，除非另行完成其上游许可证、补丁来源、构建来源和发布义务审查。
4. 当前建议全部属于 NxTools.Next 自有 Rust 契约的设计，不构成第三方代码复用申请。

## 3. 责任边界对照

### 3.1 Rayburst 的实际调用链

`docs/DOWNLOADS.md` 给出的责任分工是：浏览器扩展捕获浏览器意图；Rust desktop 管理偏好、确认/提交收据、任务创建/控制和历史；Vue desktop 编辑用户意图和呈现状态；Aria2 Next 执行最终命名、冲突处理、恢复、传输和媒体发布。

普通浏览器提交的源码链路为：

```text
services/http_api.rs::handle_add
  -> services/downloads::dispatch
  -> database::reserve_submission
  -> [needs-confirmation | services/downloads::submit]
  -> services/downloads::submit_reserved
  -> services/tasks::TaskService
  -> aria2/rpc.rs::RpcClient::call
  -> Aria2 Next
```

关键证据：

- `services/downloads/mod.rs::dispatch` 验证请求 ID/URL，计算 request fingerprint，调用 `reserve_submission`，并对已提交/已取消请求返回旧结果。
- `database/submissions.rs::reserve_submission` 用 `id + fingerprint + gid + state` 持久化意图；相同 ID 的不同 fingerprint 返回冲突。
- `services/downloads/mod.rs::submit_reserved` 在任务不存在时创建任务，保存 native session，然后 `commit_submission`。
- `services/tasks/mod.rs::TaskService` 负责 task snapshot、批量 RPC、pause/resume、文件状态和 native policy。
- `aria2/rpc.rs::RpcClient::call` 负责 JSON-RPC framing、认证、HTTP status、native RPC error envelope 和耗时诊断；策略不放在 transport 中。

### 3.2 NxTools.Next 的实际调用链

NxTools.Next 的下载路径为：

```text
GPUI / CLI / browser IPC
  -> DownloadCommandEnvelope
  -> DownloadEngine::handle_command
  -> submit / submit_browser_correlated
  -> SqliteDownloadStore + scheduler_gate
  -> spawn_download_worker
  -> probe / run_single_stream_task / run_segmented_task / media runner
  -> transfer checkpoint + atomic final commit
  -> bounded DownloadEventEnvelope
  -> DownloadApplication / IPC subscribers
```

关键证据：

- `rust/crates/nxtools-download/src/message.rs` 定义版本化 command/result/event 契约、`request_id`、`SnapshotRequired` 和 `DownloadFault`。
- `rust/crates/nxtools-download/src/engine.rs::handle_command` 是统一入口；`submit_browser_correlated` 在持久化 `handoff_id` 后返回 task ID；`command_snapshot` 和 `command_task_detail` 是查询边界。
- `rust/crates/nxtools-download/src/storage.rs` 以 SQLite schema 8 保存 task、queue、artifact、segment、HLS/DASH plan、timeline 和 browser handoff。
- `rust/crates/nxtools-download/src/engine/segmented_runner.rs` 负责 worker、Range 验证、重试、checkpoint、动态 split/merge 和最终化。
- `rust/apps/nxtools-desktop/src/runtime.rs::run_background_download_host` 使用同一 `downloads.sqlite3` 和同一 `DownloadEngine`，后台宿主退出前会等待无 active task、无 IPC client 和 idle 超时。

### 3.3 已有能力，不应重复迁移

NxTools.Next 当前已经具备或超过 Rayburst 对应层的能力：

- SQLite WAL、schema migration、unsupported schema 拒绝、孤立空库 quarantine 和启动恢复。
- 分段 durable plan、逐段 checkpoint、Range/Content-Range/validator 验证、暂停/取消可中断重试。
- 动态分段 split/merge 的 writer sync 边界。
- HLS/DASH 有独立 typed request、bounded manifest、专用 plan 和媒体 runner。
- 全局/任务限速共享 limiter、队列并发、队列顺序和延迟/每日计划。
- bounded event stream、event sequence、`SnapshotRequired`、primary event drain 和额外 watch subscribers。
- typed `DownloadFault`、native error code、correlation ID、阶段/耗时日志。
- 分类、重复文件名、reserved target 和路径边界由 `organization.rs` 统一处理。

## 4. 方案总则

### 4.1 [架构推断] 身份分层

必须明确区分以下四种身份：

| 身份 | 作用 | 是否持久化 |
| --- | --- | --- |
| `correlation_id` | 一次观测链路的诊断关联 | 否，可跨 IPC 传递 |
| `request_id` | 一次传输请求/回复的关联 | 否，除非调用方另行声明幂等 |
| `idempotency_key` / `handoff_id` | 客户端重放时保持同一业务意图 | 是 |
| `task_id` | NxTools 引擎内部的任务生命周期 | 是 |

不能把 `request_id` 直接当作幂等键，也不能仅凭 `task_id` 推断客户端已经收到 acceptance。

### 4.2 [架构推断] 接受语义

建议保持两级语义：

- `accepted`: 请求已完成校验并写入 task/receipt/queue 的原子提交；不保证 worker 已经打开网络连接。
- `started`: scheduler 已经把任务置为 `Probing`/`Waiting` 并成功创建 worker；这是独立的状态事件，不作为提交 RPC 的同步前提。

这与 NxTools 当前 `schedule_queue_locked` 的 deferred admission 行为相容，也避免把宿主繁忙误报为提交失败。

### 4.3 [架构推断] 诊断语义

每个跨进程提交或用户可见下载操作都应记录：

- correlation ID；
- accepted/start、重要状态变化、terminal success/failure；
- elapsed time、fault code、native error code；
- task ID、handoff/idempotency key 的非敏感摘要；
- 不记录 URL、完整 path、file name、Cookie、Authorization、Header value 或响应正文。

## 5. P0：持久化提交幂等与指纹冲突

### 5.1 参考证据

`D:\Code\rayburst-main\src-tauri\src\services\downloads\mod.rs::dispatch` 在业务入口生成 fingerprint，并在任务创建前调用 `Database::reserve_submission`。`database/submissions.rs::reserve_submission` 对同 ID 的相同 fingerprint 返回相同 GID，对不同 fingerprint 返回 `Conflict`。`services/downloads/tests.rs::lost_creation_replies_reconcile_the_original_gid_for_uri_and_torrent_inputs` 模拟 native `addUri/addTorrent` 已创建任务但回复丢失，第二次提交仍解析到旧 GID，最终 additions 计数不增加。

### 5.2 [源码证实] NxTools 当前缺口

- `message.rs::DownloadCommandEnvelope::request_id` 是一次 command 的相关 ID，没有 SQLite receipt。
- `engine.rs::submit_browser_correlated` 在两处只按 `handoff_id` 查旧任务；同 handoff ID 的不同 request body 会静默返回旧任务。
- `storage.rs::save_browser_task_with_transfer_and_queue` 的唯一性由 `download_browser_handoffs.handoff_id` 提供，没有 request fingerprint 或 body-conflict 字段。
- 普通 `DownloadCommand::Submit` 每次都会构造新的 `DownloadTaskId`；命令回复丢失后，调用方重试可能创建第二个任务。

### 5.3 [架构推断] 建议设计

新增一个由 `SqliteDownloadStore` 独占的 receipt 表，概念字段如下：

```text
receipt_id / external_key
origin
fingerprint
correlation-independent task_id
state: pending | accepted | rejected | cancelled
created_at / updated_at
fault_code (nullable)
```

实施规则：

1. 业务调用方明确传入 `idempotency_key`；`request_id` 继续只做 transport correlation。
2. fingerprint 必须来自稳定序列化后的业务请求；不得包含运行时 correlation ID。
3. `(origin, idempotency_key)` 唯一。
4. 相同 key + 相同 fingerprint：返回已保存的 outcome/task ID，并标记 `duplicate=true`。
5. 相同 key + 不同 fingerprint：返回 typed `Conflict`，绝不改变原任务。
6. receipt 与 task、queue membership、初始 eligibility 在一个 SQLite transaction 中写入。
7. 如果网络/worker 尚未开始，receipt 仍可提交；如果 task 创建后的回复丢失，重放只读取 receipt/task，不重新建任务。
8. Browser batch 每一项使用自己的 handoff key；父 correlation 只用于整批诊断。
9. 对现有 `download_browser_handoffs` 做兼容迁移：补 fingerprint 和状态，而不是另起第二套 browser 去重表。

### 5.4 [架构推断] 收益与风险

收益：消除浏览器/CLI/IPC 丢回复后的重复任务；可以对 handoff 内容变化给出明确冲突；重启后仍然有效。

风险：fingerprint 设计不稳定会把同一意图误判为不同请求；receipt 清理策略不当会让数据库无限增长；不能把含 Cookie 的完整序列化结果写入日志。

### 5.5 [架构推断] 红测和验收

必须先添加以下可失败测试：

- 同一 `idempotency_key + body` 两次 submit 只产生一个 task。
- 第一次 task 已持久化但返回被模拟丢弃，重试返回旧 task ID。
- 同 key 不同 body 返回 conflict，原 task/request 不变化。
- 关闭并重开 SQLite 后仍能重放旧 receipt。
- batch 中一项重复、一项新建、一项冲突时，三项分别返回确定结果。
- receipt/task/queue 任一步写失败时，不能留下孤立 task 或孤立 receipt。

诊断门：`download.submission.accepted`、`download.submission.duplicate`、`download.submission.conflict`、`download.submission.failed`；只记录 key 的 hash、task ID、fault、elapsed 和 native error code。

### 5.6 当前实施状态

- `[已完成]` `download_browser_handoffs` 已升级到 schema v9，保存 `request_fingerprint`；旧 schema 的空 fingerprint 会在首次 replay 时兼容接纳并回填调用方原始 request 的 fingerprint，不再从经过 policy/classification 改写的 task request 反推。普通幂等提交 receipt 通过 schema v10 的 `download_submission_receipts` 持久化。
- `[已完成]` `BrowserSubmit`、`BrowserBatchSubmit` 和已持久化的 Blob import 在相同 `handoff_id + request` 下返回原 task 的 duplicate 结果；相同 handoff 对不同 request body 返回 conflict，不创建第二个 task。为保持现有 protocol v8 客户端兼容，公共 fault 暂映射为不可重试的 `InvalidRequest`，engine/storage 内部保留 typed `BrowserHandoffConflict`。
- `[已完成]` 回归测试覆盖 storage conflict、legacy fingerprint 回填和后续 conflict、browser replay across engine restart、不同 body conflict、receipt retention，以及 Blob staging 已消费后的 duplicate/conflict 边界。
- `[已完成]` legacy 空 fingerprint 回填使用条件更新并校验竞争 claim 结果；engine-boundary reopen/replay/conflict 测试覆盖真实 fast path，Blob scheduler 二次 duplicate path 也记录 duplicate 诊断。
- `[已完成]` 普通 `Submit` 现在带可选 `idempotency_key`；缺省字段兼容旧 protocol v8 JSON 和旧无 key 行为，有 key 时复用 schema v10 receipt、raw fingerprint、restart replay、conflict 和原子回滚语义。CLI `download add` 支持 `--idempotency-key`，桌面新建下载窗口为一次提交会话保存 key 并通过同一 Submit 入口发送，IPC 继续透传同一 typed command。
- `[已完成]` 定向验证覆盖 origin 隔离、同 key replay、丢回复等价的重放、engine restart、不同 body conflict、receipt/task/queue 回滚、旧无 key Submit、CLI parser、IPC transport 和 desktop check。
- `[未完成]` 其他外部调用方仍需按产品场景决定是否持久化并重用业务 key；显式 Retry/Redownload、engine generation/readiness 核心和 snapshot/detail 收窄已完成，桌面/CLI 的完整 stale-event store 迁移仍需继续验收。

## 6. P1：Engine generation 与重连重同步

### 6.1 参考证据

Rayburst `engine/supervisor.rs` 用 `operation_id`、`revision`、desired state 和 cancellation watch 保护并发 start/stop/recovery；旧 operation 的 publish 会被丢弃。`stores/engine.ts::applySnapshot` 按 `revision` 忽略 stale event，并在 command 失败后重新读取 authoritative supervisor state。

### 6.2 [源码证实] NxTools 当前状态

NxTools 已有好的局部机制：

- `message.rs` 有 `DownloadEventSequencer` 和 `SnapshotRequired`。
- `engine.rs::handle_command` 的 Subscribe 会比较 generation/sequence；generation 不同或 sequence gap 都返回新的 `Subscribed`/`SnapshotRequired` 语义。
- `DownloadSnapshot`、`DownloadEventEnvelope` 和 `SnapshotRequired` 携带 engine session generation；engine 每次创建新 session 生成非零 generation。
- `runtime.rs::run_background_download_host` 保证后台模式使用单一 engine owner，并持续 drain primary event queue。
- `nxtools-download-ipc` 的 HelloAck 返回 `DownloadHostSnapshot`，包括 readiness、generation、sequence、active task/client count 和 failure code。

### 6.3 当前实施状态

- `[已完成]` stale-generation、engine restart generation、sequence gap 和 snapshot-required contract tests 已加入；旧宿主迟到 event 在 desktop generation mismatch 路径被丢弃并触发 snapshot refresh。
- `[已完成]` IPC readiness 支持 `starting | ready | draining | stopped | failed`；draining 阶段拒绝 Submit/Task/Queue 等 mutating command，Snapshot/TaskDetail/Subscribe 仍可执行。

### 6.4 [架构推断] 建议设计

在 `DownloadEngineInner` 创建时生成一个 engine-session generation，例如 UUID 或启动单调编号，并把它放进：

- `DownloadSnapshot.generation`；
- `DownloadEventEnvelope`；
- IPC handshake/subscribe response；
- `SnapshotRequired` 的 expected/observed payload。

重连规则：

1. client 先发送自己最后看到的 generation + sequence。
2. generation 不同，立即拉 snapshot；不要尝试按旧 sequence 增量恢复。
3. generation 相同但 sequence 有 gap，发送 snapshot-required，再从新 sequence 接收 event。
4. 旧宿主迟到的 event 在 client 层直接丢弃。
5. GPUI/CLI store 按 `(generation, sequence)` 更新；同 generation 内再按 sequence/revision 单调过滤。
6. scheduler wake generation 继续保留，它是 timer generation，不要与 engine-session generation 混用。

### 6.5 [架构推断] 验收

- 启动 A，订阅并收到 task event；关闭 A、启动 B；旧 client 收到 B 的 snapshot 后不会接受 A 的迟到 event。
- 同一 generation 内人为丢掉中间 event，client 收到 `SnapshotRequired` 后恢复正确任务/队列状态。
- 背景宿主退出后重新启动，历史任务和 active recovery 不会因为 generation 变化而重复创建。
- CLI JSON/NDJSON 输出在重连时不会重复打印旧 task transition。

## 7. P1：终态 Retry / Redownload 契约

### 7.1 参考证据

Rayburst `src/stores/task/resubmit.ts::resubmitTask` 明确区分 `retry` 和 `redownload`：

- retry 保留 continuation 语义；
- redownload 关闭 continuation 并启用自动改名；
- 每次 native 创建后通过 `assertSubmissionAccepted` 轮询到 active/waiting/paused/complete；
- 多项重提交发生部分失败时，按已创建 GID 回滚；
- 旧历史记录仅在新提交被接受后才删除。

前端 `taskResubmit.test.ts` 覆盖旧记录保留、终态新任务回滚和正常替换。

### 7.2 [源码证实] NxTools 当前状态

`message.rs::DownloadTaskAction` 与 `DownloadBatchAction` 已包含显式 `Retry`、`Redownload`；`Resume` 仍保留为兼容入口。`DownloadEngine::handle_command` 是 UI、CLI、BrowserHost 共用的 action seam，batch 结果按输入 task 逐项返回，Redownload 成功项的 `task_id` 是新任务 ID。

`DownloadEngine::record_failure` 保留原 task、request、validator、transfer plan、segment checkpoint 和 temporary artifact。schema v11 新增 `download_tasks.error_retryable`，所以 retryability 会跨 SQLite/engine restart 保持，而不是由重启后的 fault code 猜测。

### 7.3 [已完成] 实施契约

继续保持一个 Rust-owned task owner，不复制 Rayburst 的 Aria2 GID 替换流程：

- `Retry` 只接受带 `retryable=true` 的 `Failed` task；保留 `DownloadTaskId`、request、validator 和可用 durable transfer/media state，由现有 queue admission 重新启动。非 retryable protocol/permission/disk fault 返回 rejection，不改变 task/checkpoint。
- 重复 Retry 在命令边界 coalesce：已 eligible、Waiting 或 active 的同一 task 直接返回原 task ID，不创建第二个 task/worker。
- `Redownload` 首先以 `QueueOnly` 原子接受新 task，因此新 task 拥有新 ID、没有旧 transfer/media plan 或 checkpoint；提交仍经过现有 organization policy 和 duplicate strategy。旧 task 的 temporary、media plan 和 task row 只有在新 task acceptance 成功后才清理。
- 旧 artifact/media 清理失败时删除尚未 admission 的 replacement task，返回明确失败并保留旧 task；replacement rollback 失败则返回独立的 rollback fault，两个 task 的保留状态写入低频诊断。
- 新 task 清理成功后才通过现有 `Resume`/queue admission seam 启动；admission 暂时失败只保留已接受的 queue-only replacement，不伪造传输成功。
- batch Retry/Redownload 是 best-effort：每项独立提交并返回 fault；一项失败不回滚已经成功的独立项，单项内部 artifact rollback 除外。
- `Resume` 仅是兼容别名：Created/Paused 继续沿用旧启动语义，Failed 进入同一个 retryability gate。CLI/GPUI 展示和发送显式 `retry`/`redownload`。

### 7.4 [已验证] 验收

- `retry_preserves_identity_checkpoint_and_coalesces_after_engine_restart`：retryable fault 跨重启保留 fault、task ID、segment checkpoint，重复 Retry 只保留一个 task。
- `retry_rejects_non_retryable_fault_without_touching_checkpoint`：non-retryable protocol fault 被拒绝且 checkpoint 不变。
- `redownload_creates_numbered_replacement_after_acceptance`：新 task ID、AddNumbered duplicate strategy、旧 final artifact 保留。
- `redownload_acceptance_failure_keeps_the_old_task_and_artifact`：Reject strategy 使新 acceptance 失败，旧 task/artifact 仍存在。
- `redownload_artifact_cleanup_failure_rolls_back_the_new_task`：旧 temporary cleanup 失败时 replacement 被删除，旧 task/owned artifact 保留。
- `redownload_batch_reports_partial_success_and_returns_new_task_id`：batch 一项成功、一项失败，结果逐项可见。
- CLI parser、GPUI context menu、failed task window 和 BrowserHost action parser 均发送显式 `Retry`/`Redownload`；`Resume` compatibility path 仍可解码。

## 8. P1：常规 Snapshot 与 TaskDetail 分层

### 8.1 参考证据

Rayburst `services/tasks/mod.rs::raw_task_snapshot` 通过 `system.multicall` 聚合 active/waiting/stopped，去重后返回 native task snapshot；Vue `stores/task/index.ts::publishSnapshot` 再把 live snapshot 与 history records 合并并负责排序。详情通过 `fetchTaskItemWithPeers` 单独读取。前端测试覆盖 history 在 idle refresh 中复用、action invalidation 后再读，以及 live task 在 history 不可用时继续更新。

### 8.2 [源码证实] NxTools 当前问题

`engine.rs::command_snapshot` 会先调用 `tasks()`，再把每个 task 转为 `DownloadEngineTask` 放进 `DownloadSnapshot.task_details`。虽然它没有加载 timeline/segment plan，但仍会把 probe、error、temporary/final path 和完整 task projection 在每次 snapshot 中重复传输；真正的 `command_task_detail` 已经存在并加载 bounded timeline/segments。

### 8.3 [架构推断] 建议设计

将 `DownloadSnapshot` 的默认形态收窄为：

- task summary：ID、origin、status、durable bytes、total、queue eligibility、timestamps、fault category；
- queues；
- aggregate metrics；
- generation/sequence。

将以下内容留在 `TaskDetail`：probe metadata、validator、artifact paths、segments、timeline、媒体具体元数据。若现有 GPUI 列表确实需要其中某个字段，增加小型 typed summary 字段，而不是重新发送完整 `DownloadEngineTask`。

### 8.4 [架构推断] 验收与指标

- 以 1,000 / 10,000 个历史任务构造 snapshot，比较 JSON/IPC bytes、SQLite 查询耗时和 GPUI refresh 时间。
- 列表显示和排序结果与现状完全一致。
- 打开详情只读取选中 task，不触发全量 plan/timeline 加载。
- event gap 后的 snapshot 可以完全重建列表，不依赖上一次 task_details 缓存。

### 8.5 [已完成] 测量结果与收窄决策

基准 fixture 在 engine 内部批量写入 1,000/10,000 个任务和默认队列，避免把公共 `enqueue` 的逐项组织策略成本混入 snapshot 测量。每个场景先 warm up，再执行 7 次；输出的时间单位为微秒，格式为 `median/p95`。当前工作区直接运行结果如下：

| 投影 | 任务数 | snapshot JSON bytes | command_snapshot us | snapshot serialization us | tasks+queue SQLite us | selected TaskDetail bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| legacy full | 1,000 | 1,461,192 | 7,775/11,227 | 46,160/58,211 | 19,060/20,923 | 972 |
| compact summary | 1,000 | 423,305 | 6,812/9,996 | 13,692/14,389 | 19,742/21,638 | 972 |
| legacy full | 10,000 | 14,632,193 | 92,246/112,475 | 439,193/461,393 | 180,467/207,491 | 974 |
| compact summary | 10,000 | 4,241,306 | 90,912/103,486 | 124,156/130,510 | 195,511/201,976 | 974 |

compact 在 1,000/10,000 任务下分别减少约 71% 的序列化 payload；10,000 任务序列化中位数减少约 72%。command construction 只有小幅改善，SQLite task+queue 读取没有材料改善，说明收益来自重复 full task projection 的序列化/传输，而不是 SQLite 查询形状；因此没有引入第二个数据库、sidecar 或新的查询存储。

桌面列表投影 benchmark 的前置记录为 1,000 任务 `813/1,543 us`、10,000 任务 `8,694/12,133 us`；当前同一 projection fixture 重跑为 1,000 任务 `1,240/1,522 us`、10,000 任务 `8,358/11,090 us`。该微基准受 Windows 调度和分配噪声影响，没有把这组变化宣称为性能收益；compact 收窄的硬门槛由 payload 和 serialization 数据满足。

实施形态保持 protocol v8 兼容：`DownloadQuery::compact()` 是显式兼容路径，desktop、CLI 和 BrowserHost 的活动列表调用使用 compact；`DownloadQuery::default()` 仍返回 legacy full projection，旧 JSON 中不存在 `task_summaries` 时按默认空数组解码。`DownloadTaskSummary` 只保留列表/重建所需的 ID、状态、时间、字节数、名称、来源、分类、fault、队列资格、计划状态和 artifact capability；probe metadata、validator、request headers/cookie、save paths、segments、media details 与 bounded timeline 只从选中任务的 `TaskDetail` 读取。

契约与回归证据：

- `compact_snapshot_preserves_legacy_filtering_and_queue_order` 比较 legacy/compact 的 source、name、category 搜索、状态过滤、limit 和队列顺序，并校验 selected `TaskDetail` 等于引擎真实任务。
- `compact_snapshot_rebuilds_list_and_keeps_detail_selected_task_only` 校验 compact 列表没有 full task details，详情只返回选中任务，legacy 查询仍可用。
- `compact_snapshot_is_self-contained_after_a_sequence_gap_refresh` 校验 gap 后只用 compact snapshot 重建任务列表。
- `compact_query_is_opt_in_and_legacy_snapshot_json_stays_decodable` 校验缺少新增字段的旧 snapshot JSON 仍可解码。
- desktop 的 selected-detail contract 校验 compact 占位只用于列表，详情到达后概览/Header/校验页统一使用真实 `detail.task`。

### 8.6 [已完成] 当前边界

Rust engine 继续拥有 task map、queue、SQLite、snapshot 构造和 `TaskDetail` 读取；IPC 只传 typed values，desktop/CLI 不直接访问 SQLite。列表过滤、排序和 queue admission 的语义没有迁移到 Rayburst 的 Vue/Pinia 结构，也没有复制 Aria2 sidecar；compact 仅在测量证明重复 full task projection 有材料成本后启用。

## 9. [架构推断] P1：宿主 readiness 与轻量 Supervisor

### 9.1 适用边界

这里不能照搬 Rayburst 的 Aria2 sidecar phase machine。NxTools 的 transfer engine 是同进程 Rust object，后台宿主可以退出/重启，IPC 客户端需要知道的是“宿主是否可接收 command”和“engine session 是否变更”。

### 9.2 [架构推断] 建议设计

在 `runtime.rs` / `nxtools-download-ipc` 边界增加小型 `DownloadHostSnapshot`：

```text
starting | ready | draining | stopped | failed
session_generation
active_task_count
connected_client_count
last_failure_code
```

约束：

- 启动只允许一个 owner 初始化 SQLite/DownloadEngine。
- 已有 owner 启动请求返回 `starting`/`ready`，不再另开第二个数据库连接。
- 停止进入 draining，拒绝新的 mutating command，但允许 snapshot/detail 和 terminal event drain。
- idle exit、初始化失败、IPC server 退出都发布终态诊断。
- supervisor 只管理宿主和 IPC 生命周期，不管理 HTTP segment worker；segment worker 仍由 `DownloadEngine` 负责。

### 9.3 当前实施状态

- `[已完成]` `nxtools-download-ipc` 已提供 `DownloadHostSnapshot`、readiness transitions 和 HelloAck handshake snapshot；runtime idle exit 进入 draining/stopped，IPC server failure 标记 failed。
- `[已完成]` draining 会在 IPC seam 拒绝 Submit、Browser、Task、Batch、Queue 和 speed-limit mutation，Snapshot/TaskDetail/Subscribe 仍可查询并排空事件；single-owner 继续由现有 desktop instance lock 保证。

### 9.4 [架构推断] 验收

- desktop 与 background host 同时启动时只有一个 `downloads.sqlite3` owner。
- engine 初始化失败时 CLI 得到稳定的 `engine_starting` 或 `engine_unavailable` fault，而不是管道断开超时。
- host restart 后 generation 改变、snapshot 可恢复、active task 不重复提交。
- draining 期间的新提交被拒绝，旧任务的 pause/cancel 和 event drain 语义明确。

## 10. [架构推断] P1：统一提交/失败诊断，而不是引入 RPC

Rayburst 的 `aria2/rpc.rs::record_rpc_result` 对 polling error 降低日志级别、对 mutation error 记录失败、对慢操作记录耗时；这是好的诊断分类，但 NxTools 没有 Aria2 RPC，不应复制 `RpcClient`。

NxTools 已有：

- `probe.started/completed/failed`；
- `download.task.attempt_started/attempt_completed/failed`；
- `download.segment_transfer.*`；
- `DownloadFault` 和 `native_error_code`；
- correlation ID。

需要补齐的是[架构推断]入口与终态的一致性：

1. 所有 `Submit`/`BrowserSubmit`/`Batch` 入口都先记录 accepted/rejected；
2. receipt duplicate/conflict 与 scheduler admission deferred 分开；
3. `record_failure` 把 `fault.retryable`、terminal status、persist result 和 cleanup result 分开记录；
4. 事件订阅丢失只记录 bounded snapshot-required，不记录每个 progress tick；
5. 日志中的 idempotency key 只用 hash 或短 opaque token，不打印 request body。

## 11. [架构推断] P2：有条件的适配项

### 11.1 [源码证实] 显式 task policy

Rayburst `services/tasks/policy.rs::TaskPolicy` 将 internal、automatic、pending start、deleted 分开保存，并在 `visible_tasks` 时过滤内部任务。NxTools 当前主要依靠 `DownloadStatus`、`RequestOrigin`、queue eligibility 和 media runner。

建议：只有当媒体探测、FFmpeg 安装、索引/内部任务进入同一任务列表并需要隐藏时，再增加显式 `visibility`/`execution_policy` 字段。不要为了文件结构相似而加入 Rayburst 的 Aria2 internal GID 集合。

### 11.2 [源码证实] Category policy

Rayburst `services/downloads/category.rs` 支持 relative/absolute 目录、URL pattern、候选文件必须共同落入同一目录，以及路径模式校验。NxTools `organization.rs` 已有 bounded category count/rules/text、explicit/automatic category、relative path containment、reserved target 和 duplicate strategy。

[架构推断] 可适配的只有行为测试：多文件候选必须同目录、URL specificity、路径逃逸、大小上限。当前不需要重写 NxTools 的 organization module。

### 11.3 [源码证实] 历史/文件 watcher

Rayburst `services/tasks/files.rs` 的 notify watcher、seeding 文件缺失暂停和 P2P 共享完成事件是 Aria2/BT/ED2K 专属。NxTools 当前传输协议为 HTTP/HLS/DASH，已由 artifact ownership 和 destination conflict 管理文件。

除非产品增加长期 seeding 或外部文件被任务持续占用，否则不要复制 watcher、P2P sharing 状态或 `tellStopped` 历史模型。

## 12. [架构推断] 明确不应复制的模式

| Rayburst 模式 | 不复制原因 |
| --- | --- |
| Aria2 JSON-RPC `RpcClient`、`system.multicall` | NxTools 直接拥有 HTTP/媒体执行；引入第二个 transfer framework 会破坏 Rust domain ownership |
| Aria2 `EngineSupervisor` 的 sidecar spawn/probe/port recovery 全量状态机 | NxTools 只需宿主 readiness/session supervisor；segment worker 已由 engine 自己管理 |
| Vue/Pinia `engine.ts`、`task/index.ts` 文件结构 | GPUI/Rust 的 entity/event 生命周期不同；文件一对一迁移会掩盖所有权边界 |
| Rayburst history + Aria2 stopped result 双记录模型 | NxTools SQLite 已同时保存 task、queue、artifact、segment、timeline；新增第二套历史会造成一致性问题 |
| `services/tasks/files.rs` 的 BT/ED2K seeding policy | 当前 NxTools 没有对应协议或产品行为 |
| Aria2 sidecar binary、配置、补丁或测试 | sidecar provenance/许可证未在 Rayburst 仓库内完成闭环 |
| 依赖 frontend poll 解决 native lifecycle | NxTools 已有 bounded event stream、snapshot 和 IPC host；应修复边界而不是增加轮询 |

## 补充：Rayburst 反模式与能力契约

### [源码证实] Rayburst 自身的限制

- `src-tauri/src/services/tasks/mod.rs::get_global_stat` 在存在 internal task 时额外执行一次 `tell_task_snapshot(false)`，而 `monitor_loop` 已经周期性读取同一快照；这是 Rayburst 的计数修正路径，不应作为 NxTools 的 polling 模式。
- `src-tauri/src/services/tasks/mod.rs::resume_eligible` 对每个候选 task 串行执行 `recheck_before_resume`，其中包含 `tell_status` 和可能的 `force_bt_recheck`；只读状态可以批量化，带副作用的 recheck 仍需保持受控顺序。
- `src-tauri/src/commands/aria2.rs::remove_engine_task` 通过最多 100 次、每次 50 ms 的全量 `tell_task_snapshot(true)` 等待删除终态；NxTools 不应复制该 O(N) 全量轮询，应使用 task-specific state/event 或已有 `TaskDetail` seam。
- `src-tauri/binaries/aria2.conf` 的 `max-tries=0` 表示无限重试，`aria2_events.rs` 主要在 start/terminal 边界发通知；这解释了 Rayburst UI 可能长期只显示 active 的产品风险。
- `src-tauri/src/services/tasks/files.rs::FileMonitor::availability` 对每个选中文件执行 open/metadata，服务于 BT/ED2K seeding 文件可用性；它不是当前 NxTools HTTP/HLS/DASH 的通用优化。
- `src-tauri/src/services/media/journal.rs` 使用独立 SQLite 和 `synchronous=FULL`，`docs/DOWNLOADS.md` 明确说明 media journal 与主 database 分离是因为生命周期和 lease 不同；这不是可以无条件合并或复制的 schema。

### [架构推断] 对 NxTools.Next 的补充决策

1. 保留 NxTools 当前 `segmented_runner.rs` 的 bounded retry 和 typed fault。不要引入 Rayburst 的无限重试语义；retryable transport fault、protocol/validator fault、disk/permission fault 必须继续有不同终态。
2. 将 correlation ID 从提交入口贯穿到 `DownloadTransferContext`、segment terminal、artifact commit 和 IPC result。NxTools 已有 `uuid::Uuid` transfer correlation seam，但 browser/CLI receipt、host generation 和终态诊断仍应统一使用同一个业务 correlation。
3. 不为媒体或下载新增第二套 journal/database。先保持 `SqliteDownloadStore` 的 task/queue/artifact/plan/timeline 原子边界；只有真正引入不同 lease 生命周期时，才单独建立 journal，并先写跨库不变式测试。
4. 将 capability contract 作为 P2 的版本化接口，而不是把引擎细节散落在 UI。Rayburst 当前 `/downloads/capabilities` 只声明 `protocolVersion` 和 `filenameHints`；NxTools 可在自己的 typed IPC snapshot/handshake 中声明 adaptive segments、media protocol、retry semantics 和 snapshot generation，旧 client 必须忽略未知能力。
5. 大规模任务操作使用 task-specific query、bounded batch 或事件驱动；禁止为了一个计数、删除确认或 UI 刷新重复拉完整 task snapshot。

### [架构推断] 新增验证项

- fake RPC/HTTP fixture 证明 internal task 计数不会触发重复全量读取；删除等待只读取目标 task 或消费终态事件。
- 503/连接断开 fixture 证明 retry 次数有上限，失败后任务可显式 retry，不会无限保持 active。
- 一次 browser/CLI submit 的 correlation ID 在 receipt、task acceptance、worker terminal、artifact commit 和 IPC result 中一致，日志不含 URL、path 或 Header payload。
- 能力字段向前兼容：旧 client 读取带未知 capability 的 response 仍能完成基础 snapshot/submit；新 client 在能力缺失时明确降级，不显示不可用操作。

## 13. [架构推断] 实施顺序与变更范围

### 阶段 A：P0 submission receipt

涉及：

- `rust/crates/nxtools-download/src/message.rs`
- `rust/crates/nxtools-download/src/storage.rs`
- `rust/crates/nxtools-download/src/engine.rs`
- `rust/crates/nxtools-download/src/lib.rs`
- `rust/crates/nxtools-download/src/engine/segmented_runner.rs`（仅确认 worker 终态，不重写传输）
- browser/CLI/IPC 调用方及其协议测试

产物：schema migration、typed duplicate/conflict outcome、process-reopen red tests、相关 lifecycle diagnostics。

### 阶段 B：P1 session/snapshot/host

涉及：

- `rust/crates/nxtools-download/src/message.rs`
- `rust/crates/nxtools-download/src/engine.rs`
- `rust/crates/nxtools-download-ipc/src/lib.rs`
- `rust/apps/nxtools-desktop/src/runtime.rs`
- CLI subscriber/store

产物：session generation、重连 snapshot gate、host readiness state、stale event tests、snapshot payload benchmark。

### 阶段 C：P1 retry/detail

涉及：

- `rust/crates/nxtools-download/src/message.rs`
- `rust/crates/nxtools-download/src/engine.rs`
- `rust/crates/nxtools-download/src/storage.rs`
- GPUI/CLI task actions and tests

产物：explicit retry/redownload semantics、partial batch result、lazy task detail、terminal rollback tests。

当前状态：Retry/Redownload 已完成并通过 engine library、CLI parser 和 GPUI context-action 定向测试；snapshot/detail 已完成 measured compact projection、selected-task lazy detail、legacy decode、gap rebuild 和列表过滤/排序等价性验收。

### 阶段 D：P2 policy cleanup

只在产品需求出现时实施：internal visibility、额外 category pattern、外部文件 availability。不得为了“对齐 Rayburst 文件结构”提前改动。

## 14. [架构推断] 验证矩阵

| 领域 | 必须通过的验证 | 当前状态 |
| --- | --- | --- |
| 幂等 | 同 key replay、lost reply、body conflict、重启 replay、batch partial result | `[已完成]` P0 receipt/fingerprint contract |
| 存储 | schema upgrade、WAL/sidecar、transaction rollback、receipt/task/queue 原子性 | `[已完成]` 相关 storage contract |
| generation | host restart、stale event、sequence gap、snapshot reconstruction | `[部分完成]` engine/IPC 与 desktop gap gate 已覆盖，完整 stale-event store/real host restart 仍待补 |
| 队列 | stable reorder、eligible/user-paused、并发 slot refill、delayed schedule 不回归 | `[已完成]` compact 保留 queue order，现有 queue contracts 继续运行 |
| retry | retry 保留 checkpoint、redownload 新 task、非 retryable fault、重复点击 coalesce | `[已完成]` |
| snapshot | 1k/10k task payload benchmark、detail lazy load、列表排序等价性 | `[已完成]` compact/legacy benchmark、TaskDetail、gap rebuild 和 filter/sort contract |
| 传输回归 | 现有 HTTP Range、validator、HLS/DASH、pause/cancel、artifact commit tests 全部通过 | `[未全绿]` 本轮 engine_contract 为 92/101，9 个既有网络/动态 fixture 时序失败，见下方记录 |
| 诊断 | correlation 连贯、terminal success/failure 成对、无 URL/path/header/cookie payload 泄漏 | `[已完成]` 本 slice 未新增敏感 payload 诊断 |
| 集成 | desktop、background host、CLI 只能共享一个 engine/database owner | `[部分完成]` 相关 crate/desktop checks 已通过，real host restart 仍是后续验证 |

所有 Rust 验证遵守仓库约束：`CARGO_BUILD_JOBS=1`、`CARGO_INCREMENTAL=0`、`RUSTFLAGS=-C codegen-units=256`。本轮通过：`nxtools-download` library 100/100、artifact 4/4、domain 10/10、message contract 15/15、CLI 10/10、IPC 6/6、BrowserHost 26/26、desktop 409/409；相关 crate `cargo check`、`cargo fmt --all -- --check` 与 scoped `git diff --check` 通过。完整 `engine_contract` 为 92/101，9 个失败为 `cli_dash_url_auto_selects_tracks_and_persists_the_media_request`、`dynamic_rebalance_splits_a_slow_tail_without_overlapping_output`、`a_dynamically_split_plan_resumes_after_engine_restart`、`selected_dash_video_track_downloads_init_and_segments_in_manifest_order`、`selected_hls_vod_bounds_parallel_segment_transfers_by_connection_limit`、`selected_hls_vod_downloads_segments_and_commits_them_in_playlist_order`、`reopening_engine_admits_a_durable_eligible_pending_task`、`single_stream_retry_budget_stops_after_three_attempts` 和 `pausing_after_aes128_decryption_failure_cleans_retry_staging`；它们是现有网络/动态 fixture 时序漂移，compact/detail 新增测试均通过，因此完整 integration runner 仍明确标记为未全绿。

## 15. [架构推断] 最终决策

1. P0 browser handoff 和统一普通 `Submit` 的可选 key/receipt/fingerprint/conflict 已落地；下一步按产品场景补齐其他外部调用方的 key 持久化策略，再进入 P1 generation + snapshot/host readiness。
2. 接着做 P1 generation + snapshot/host readiness，把后台宿主重启和 IPC 重连变成可观测、可恢复的生命周期。
3. Retry/Redownload 已按 Rust task ownership 落地；snapshot/detail 已按测量结果完成，继续保持 compact 列表与 selected-task lazy detail 边界。
4. 保留 NxTools 现有的 Rust-owned transfer engine、segmented runner、SQLite schema、organization policy 和 bounded event model；这些不应被 Rayburst 的 Aria2/Tauri 结构替换。
5. 在任何代码复用前，单独核对 Rayburst sidecar Aria2 Next 的 source/license/build provenance；本审计没有把它当作已获授权的可复制依赖。
