# 代码与工程质量审查：问题记录

日期：2026-10-31。工作区：`D:\Code\NxTools.Next-main`（审查基于当时含 821 项未提交改动的工作树）。

本记录为只读静态审查结论：未修改业务代码，未编译、未运行测试，未新增复现用例。每项给出的“验证方法”是建议，不是已执行结果。

标注约定：

- **确定性**：已确认（代码链路足以证明）/ 高可信风险（触发路径明确，未复现）/ 待验证。
- **†**：仅经初步通读，尚未逐行复核；修复前需先打开对应源码确认。
- 行号以审查当日工作树为准，后续改动后请按符号名检索。

## 1. 项目概况

| 进程 | 技术 | 职责 |
| --- | --- | --- |
| `NxTools.exe` | Rust + GPUI | 全部 UI、领域状态、SQLite、编排 |
| `NxTools.PlatformHost.exe` | .NET 8 | Windows API 适配；管道 `nxtools-next-platform-v1`，NDJSON |
| `NxTools.Indexer.exe` | Rust，LocalSystem 服务 | MFT/USN 索引，内存映射快照 + delta |
| `NxTools.Cli.exe` / `NxTools.BrowserHost.exe` | Rust | 经 `nxtools-next-download-v1` 访问下载引擎 |
| 浏览器扩展 | MV3 | 下载拦截、媒体嗅探、Blob 交接 |

规模：Rust 约 33 万行（desktop 18.9 万），C# 业务约 4 万行、测试约 3 万行。

## 2. 覆盖范围

| 批次 | 范围 |
| --- | --- |
| A | nxtools-download、download-ipc、CLI 下载路径 |
| B | nxtools-indexer、finder、ipc/local_pipe、protocol |
| C | nxtools-clipboard、core 配置、desktop clipboard_worker / runtime / downloads 连接层 |
| D | C# PlatformHost：管道、调度、钩子、剪贴板、提权/更新、归档、扩展代理 |
| E | browser-host、浏览器扩展、nxtools-archive、nxtools-extensions |

未覆盖：desktop GPUI 视图层大部分（main_window、ui、quick_launch 视图）；C# FloatingAssistant、ShellContextMenu、文件管理器适配器、ACL 加固；archive `*_edit*.rs` 写入器与加密实现；dash_runner 传输/合并逻辑。

## 3. 优先处理（Top 5）

1. H1 下载引擎在 GPUI 线程上阻塞发送事件，批量操作可致桌面死锁。
2. H2 索引批量删除超出工作量预算，触发整卷重建。
3. H3 索引首次扫描期间被修改的文件永久缺失。
4. H4 C# 低级键盘钩子与剪贴板捕获/粘贴共用线程。
5. H5 C# `launch_process`、`ensure_indexer_service` 阻塞串行命令读取。

## 4. 问题清单

### 高

#### H1 下载引擎事件阻塞发送导致 GPUI 死锁

- 类别：并发 / 稳定性 ｜ 确定性：已确认
- 位置：
  - `rust/crates/nxtools-download/src/engine.rs:2560` `emit`
  - `rust/crates/nxtools-download/src/engine.rs:616` 事件通道 `bounded(256)`
  - `rust/crates/nxtools-download/src/engine.rs:1352` `run_batch_action`（无任务数上限）
  - `rust/apps/nxtools-desktop/src/downloads.rs:760` 事件消费（GPUI 前台 `cx.spawn`）
  - `rust/apps/nxtools-desktop/src/downloads.rs:1254` `batch_action`（GPUI 线程同步 `handle_command`）
- 证据：

  ```rust
  if matches!(envelope.event, DownloadEvent::TaskProgress { .. }) {
      if self.event_sender.len() < 192 { let _ = self.event_sender.try_send(envelope.clone()); }
  } else {
      let _ = self.event_sender.send_blocking(envelope.clone());
  }
  ```

- 触发：有下载运行时进度事件最多占 192 槽，批量暂停/取消/删除约 64 个以上任务即可填满；无下载时约 256 个。另一路径（†）：下载 worker 持 `tasks` 锁时 `emit` 阻塞，UI 下一次快照在该锁上等待。
- 影响：GPUI 线程等待只有自己能清空的通道，永久无响应。
- 修复：`emit` 不阻塞——状态事件 `try_send`，满时置位并在下次有空位时补发 `SnapshotRequired`。桌面端已能处理：`downloads.rs:3632` 的 match 分支为空，但随后落入 `refresh_snapshot(true, cx)`（`downloads.rs:3639`），即收到 `SnapshotRequired` 会全量刷新，无需改动桌面事件处理。其次把 `emit` 移出 `tasks` 锁。
- 验证：300 个任务、不消费接收端，同线程调用 `Batch{Remove}`，断言限时返回；修复后断言收到 `SnapshotRequired`。

#### H2 批量删除超出工作量预算，触发整卷重建

- 类别：性能 / 稳定性 ｜ 确定性：已确认
- 位置：`rust/apps/nxtools-indexer/src/usn.rs:234`（Delete 分支）、`:429`（`reserve_recursive_work`）、`:499`（`bounded_visible_child_candidates`）、`:30`（`MAX_RECURSIVE_WORK_UNITS_PER_BATCH = 4_096`）
- 证据：每条 Delete 记录进入 `delete_visible_subtree`，循环首先计费 1 单位，叶子文件或不在索引中的项同样计费；`live_work` 在整次轮询累加。子项计数使用 `base_children(parent)`，包含已被 overlay 删除的子项。
- 触发：一次轮询内删除超过 4096 个文件（`cargo clean`、删除 `node_modules`、清空回收站），或删除基础快照中子项超过 4096 的目录。
- 影响：返回 `RebuildRequired`，整卷重新枚举 MFT；与 M3 叠加时旧快照持续堆积。
- 修复：仅对需要展开的可见目录子项计费；计数前过滤 `overlay.deleted`。
- 验证：仿照 `usn.rs` 1282–1315 附近的 upsert 预算测试，新增 5000 个可见叶子删除批次断言 `Ok`；发布前运行 `tests/performance/indexer-idle-cpu.ps1 -RestartService -AssertRegression`。

#### H3 首次扫描期间被修改的文件永久缺失

- 类别：正确性 ｜ 确定性：代码路径已确认；依赖 `FSCTL_ENUM_USN_DATA` 按 HighUsn 过滤的文档语义
- 位置：`rust/apps/nxtools-indexer/src/mft.rs:612`、`:662`；`rust/apps/nxtools-indexer/src/usn.rs:215`
- 证据：

  ```rust
  let journal = query_journal(handle.0)?;
  let records = enumerate_mft(handle.0, journal.NextUsn) // HighUsn = 扫描开始时的 NextUsn
  ```

  扫描中被修改的文件 USN 大于 HighUsn，不被枚举；随后回放日志时这类变化（DATA_EXTEND、CLOSE 等）归为 `Metadata`，该分支只更新已可见条目，从不新增。
- 触发：建索引或重建期间（分钟级）有文件/目录被写入。
- 影响：文件不出现在搜索中，重启后仍缺失；目录被跳过时整个子树缺失。
- 修复：`HighUsn = i64::MAX`，起始游标仍取扫描前的 `NextUsn`；回放 upsert 幂等。
- 验证：真实卷上扫描开始后持续写入新目录中的文件，完成后搜索；单元层构造“枚举缺失 + 仅 Metadata 记录”场景。

#### H4 低级键盘钩子与剪贴板捕获/粘贴共用 STA 线程

- 类别：稳定性 / 性能 ｜ 确定性：共用线程已确认；钩子被移除依据 Windows `LowLevelHooksTimeout` 文档行为
- 位置：
  - `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs:263` 在 EventWindow 构造中安装 `LowLevelKeyboardMonitor`
  - 同文件 `:493` `WmClipboardUpdate` → `HandleClipboardUpdate`（`WindowsPlatformAdapter.ClipboardPause.cs:231`）→ `ReadClipboardContent`（`EventWindow.cs:872`）
- 证据：同一 WndProc 线程同步执行 `NativeClipboardFormats.Capture`（最多 32 MB，延迟渲染需等待源程序）、`Clipboard.GetImage()`、`image.Save(..., Png)`；粘贴路径 `WriteClipboard` 重试含 `Thread.Sleep(20)`（†）。`LowLevelKeyboardMonitor.IsInstalled` 只检查句柄（†）。
- 触发：输入时复制大截图或 Office 选区；其他程序占用剪贴板时粘贴。
- 影响：全系统按键卡顿；钩子超时被系统静默移除后双击 Ctrl 失效直至重启 PlatformHost，能力标志仍显示可用。
- 修复：`LowLevelKeyboardMonitor` 使用专用线程，仅运行 `GetMessage` 循环，回调继续 `PostMessage` 给 EventWindow。
- 验证：阻塞 EventWindow 线程约 2 秒，同时 `SendInput` 注入两次 Ctrl，断言 `WmQuickLaunchGesture` 仍被投递。

#### H5 `launch_process` / `ensure_indexer_service` 阻塞串行命令读取

- 类别：并发 / 稳定性 ｜ 确定性：已确认
- 位置：`csharp/NxTools.PlatformHost/Protocol/PlatformCommandSequenceDispatcher.cs:11`（`DetachedCommands` 不含这两个命令）、`:74`（非分离命令直接 `return executeAsync(...)`，读循环等待）
- 证据：以管理员运行走 `Verb="runas"` 的 `Process.Start`，ShellExecuteEx 待 UAC 处理完才返回；`EnsureIndexerServiceAsync` 含最长 30 秒安装门、哈希、`runas` 安装与最长 5 分钟可信更新等待（†）；`LaunchProcess` 返回的 `Process` 未释放（†）。同文件 34–38 行注释记录了 `open_shell_target` 的同类事故。
- 触发：速启“以管理员运行”；版本变化后首次 `prepare_file_index`。
- 影响：窗口显隐、粘贴、速启隐藏、`archive_job` 取消等命令全部排队。
- 修复：两个命令加入 `DetachedCommands`；`LaunchProcess` 中 `using` 释放 `Process`。
- 验证：扩展 `ExtensionRuntimeTests.DetachedCommandsKeepReaderResponsive` 覆盖两个命令名。

### 中

#### M1 网页可用合成事件绕过用户手势触发 NxTools 下载

- 类别：安全 / 信任边界 ｜ 确定性：已确认
- 位置：`browser_extension/nxtools-interceptor/content-script.js:1179` `handleDirectDownloadClick`、`:1110` `handleDrop`；全文件无 `isTrusted` 检查
- 证据：页面 `a.click()` 或派发合成 `drop` 同样进入处理函数并发送 `content_direct_download_request` / `drop_target_download`。后台以 `chrome.cookies.getAll` 附带目标站点 Cookie（含 HttpOnly、SameSite=Strict）（†，`request-context.js`）；`drop_target_download` 路由不检查拦截开关与站点禁用（†）。
- 影响：绕过浏览器多次下载保护；`openDialog:false` 时静默入队；对第三方站点发起带 Strict Cookie 的 GET。
- 修复：两个处理函数开头 `if (!event.isTrusted) return;`。脚本触发的真实下载仍走浏览器 `onDeterminingFilename` 路径。
- 验证：测试中派发 `new MouseEvent('click')`，断言未调用 `sendMessage`。

#### M2 最终 rename 覆盖已有文件；Failed 任务不占用目标名

- 类别：正确性 / 数据安全 ｜ 确定性：已确认
- 位置：`rust/crates/nxtools-download/src/http.rs:1166` `commit_prepared`（`tokio::fs::rename` 在 Windows 上替换已存在文件）；`rust/crates/nxtools-download/src/engine.rs:2984` `target_reserved_by_tasks`（跳过 Failed，而 Failed 可 Retry）
- 触发：任务 A 失败 → 新建同名任务 B 获得同名 → 重试 A；后完成者覆盖先完成者。
- 影响：已完成文件被静默覆盖。
- 修复：提交改用 `MoveFileExW` 且不带 `MOVEFILE_REPLACE_EXISTING`，“已存在”映射为 `DestinationExists`；未提交产物的 Failed 任务继续占用目标名。
- 验证：两个同名任务依次提交，第二个返回冲突。

#### M3 索引旧快照文件仅在服务启动时清理

- 类别：资源 ｜ 确定性：已确认
- 位置：`rust/apps/nxtools-indexer/src/mft/cache.rs:471` `persist_system_snapshot` 每次写新 `{prefix}-{nonce}.idx`；`cleanup_obsolete_cache_files` 唯一生产调用点在 `cache.rs:425` 恢复流程
- 影响：每次重建/压缩在 `%ProgramData%` 留下一份完整快照，服务生命周期内累积。
- 修复：新快照发布且旧 `Arc` 释放后调用 `cleanup_obsolete_cache_files`（已容忍删除失败）。
- 验证：连续两次重建后目录只剩当前 base/delta。

#### M4 索引会话在整个生命周期内持有服务锁

- 类别：并发 ｜ 确定性：加锁范围已确认；旧会话卡在构建中为高可信风险（†）
- 位置：`rust/apps/nxtools-indexer/src/windows_service_host.rs:215`：`service.lock()` 后执行完整 `serve_admitted_session`
- 影响：桌面在扫描/重建期间重启，新连接收不到 HelloAck、状态与搜索结果，直到旧会话结束。
- 修复：锁只包单个命令处理；或新会话握手时通知旧会话退出。
- 验证：会话 A 保持连接不读时，会话 B 限时收到 HelloAck。

#### M5 以 `./` 开头的 TAR 包无法解压

- 类别：正确性 ｜ 确定性：已确认
- 位置：`rust/crates/nxtools-archive/src/path_policy.rs:25` 拒绝 `"."` 段；`planning.rs:153` 任一项带 `UnsafePath` 则整个计划失败；`formats/tar.rs:284` `display_path` 未归一化
- 触发：`tar -czf x.tgz -C dir .` 产生的包（`./`、`./a.txt`）。
- 影响：可列出，所有解压任务报“不安全路径”。
- 修复：共用输出路径投影——去掉开头重复 `./`，单独 `.` / `./` 目录项跳过，路径中间的 `.` 仍拒绝；`display_path`、`raw_name` 保持原样。
- 验证：新增 `dotprefix.tar` 夹具；`a/./b` 仍被拒绝。

#### M6 剪贴板清理批量清除时为 O(n²)

- 类别：性能 ｜ 确定性：高可信风险（取决于数据规模）
- 位置：`rust/crates/nxtools-clipboard/src/lib.rs:988`、`:1012` `cleanup`；索引定义 `lib.rs:518` 起
- 证据：每个被清除行单独 `prepare` 格式查询；每个候选资源执行 `image_path = ?1 OR source_process_icon_path = ?1` 与 `data_path = ?1` 的 EXISTS 查询，三列无索引，逐个全表扫描。
- 触发：`max_items` 从数万调低，7 天后软删除行在单事务内物理清除。日常小量清理不受影响。
- 影响：清理在剪贴板 worker 线程运行，期间捕获/查询排队，队列（256）满后新捕获被丢弃。
- 修复：事务内临时表存候选路径，一条 `NOT EXISTS` JOIN 判断引用；或为 `clipboard_item_formats(data_path)` 建索引并把 OR 拆为两次索引查找。
- 验证：参照 `lib.rs:3501` 现有 10k/100k 基准，增加“10 万软删除后清除”用时。

#### M7 local_pipe 同步 I/O 以轮询/自旋实现

- 类别：性能 / CPU 空转 ｜ 确定性：已确认
- 位置：`rust/crates/nxtools-ipc/src/local_pipe.rs:102`（`PeekNamedPipe` 无数据时睡 1–8 ms 重查）；`:211`（管道不存在时 `WaitNamedPipeW` 立即返回，循环无睡眠）
- 影响：空闲时每个读端约 125 次/秒唤醒（索引服务与桌面各有），空闲 CPU 门禁难以察觉；桌面冷启动约 5 秒内 CLI/BrowserHost 连接占满一核。
- 修复：短期 `WaitNamedPipeW` 返回 NotFound 时 `sleep(25ms)`；长期读端改 overlapped I/O + 事件等待。
- 验证：空闲时 ETW / Process Explorer 统计上下文切换；冷启动时测 CLI 连接期间 CPU。

#### M8 有变化的轮询克隆整个快照 overlay

- 类别：性能 ｜ 确定性：`usn.rs:206` `snapshot.clone()` 已确认；“空闲轮询也复制”（†）未确认
- 影响：overlay 最多达基础行数 10%，百万级卷上每次有变化的轮询复制几十万条，且与搜索调度同线程。
- 修复：overlay `Arc` + 写时复制，或原地修改失败回滚。是否值得做以第 5 节测量为准。

### 低

| 编号 | 问题 | 位置 | 修复要点 |
| --- | --- | --- | --- |
| L1 | 文件名 205–255 字符时临时名 `.{name}.{uuid}.nxtools-part` 超 255，必然失败 | `nxtools-download/src/http.rs:1188`、`lib.rs:365` | 临时名只用任务 ID |
| L2 † | 取消暂停的 HLS/DASH 任务时持全局锁同步 `remove_dir_all` 上万分片 | `engine.rs:2228` 起，`engine/hls_runner.rs:1046` | 仿照 `remove_correlated`，锁内标记、锁外删除 |
| L3 † | 一个分段失败后其余分段继续下载至结束 | `engine/segmented_runner.rs:1851` | 记录首个错误后通知其余分段停止 |
| L4 | `find_history_item` 分页线性查找并加载完整正文 | `nxtools-desktop/src/clipboard_worker.rs:1239` | 直接调用 `history.active_item(id)`（同为仅未删除项） |
| L5 | `ConnectNamedPipe` 任一错误（含 `ERROR_NO_DATA`）使下载 IPC 服务永久退出且无重启 | `nxtools-ipc/src/local_pipe.rs:169`、`nxtools-download-ipc/src/lib.rs:283` | `ERROR_NO_DATA` 时关闭句柄继续 accept |
| L6 † | 批量交接 Cookie 同时在 `headers` 与 `cookie` 中，32 项可能超 256 KiB；关闭“携带必要 Header”后 `headers`（含 authorization）仍发送 | `handoff.js:370`、`request-context.js:336` | 去重；关闭开关时清空 `headers` |
| L7 † | BrowserHost 启动清理超 1 小时的 `*.blob`，可能删除仍开着的确认框所需文件；弹窗 1.5 秒轮询反复拉起 host | `blob_upload.rs:642`、`popup/actions.js:170` | 跳过已交给对话框的 staging；轮询间隔大于端口释放时间 |
| L8 † | 剪贴板捕获重试时暂存 PNG 未删除 | `EventWindow.cs:897` | 异常时 `TryDeleteStagedImage` |
| L9 † | 扩展代理控制通道累计流量上限 `int.MaxValue`，长期运行超约 2 GiB 后 broker 退出 | `ExtensionHostBroker.cs:94` | 控制流不检查累计总量 |
| L10 † | 扩展包升级留下旧代码快照、卸载不清理；包 ID 可为 `con`/`nul` | `nxtools-extensions/src/bundle.rs:54`、`lib.rs:383` | 提交后清理无引用哈希目录；`valid_id` 复用保留名检查 |
| L11 † | 重建后快照驻留堆而不重新映射；旧格式 delta 按头部数量预分配 | `nxtools-indexer/src/mft.rs:542`、`mft/cache.rs:614` | 重建后重新 mmap；分配前校验文件长度 |

### 复核后无需改动的部分

- 下载：Range/Content-Range 校验严格；SQLite 只在分段状态变化时写；进度事件每任务 100 ms 限流；文件名过滤保留名与非法字符；ffmpeg 有 SHA-256 校验且解压防穿越。
- 配置：`AppConfig::save_to` 临时文件 + `sync_all` + rename，外层跨进程锁。
- 剪贴板存储迁移在暂存目录完成并 `quick_check` 后提交。
- 索引：USN_RECORD_V2 解析做越界/对齐检查；父引用成环保护（`usn.rs:344`）；管道准入验证 PID/会话/哈希，握手 3 秒超时，单帧 1 MiB。
- C#：管道写入经单一写循环；钩子委托由字段持有；可信部署同一句柄哈希并复制，无 TOCTOU；归档输出逐级组件校验、相对句柄打开。
- 归档：路径策略覆盖绝对路径、`..`、ADS、设备名；密码 zeroize 且不入日志。

## 5. 待验证的性能假设

| 假设 | 何时值得优化 | 测量方法 |
| --- | --- | --- |
| M6 清理 O(n²) | 单次物理清除上万行 | 扩展现有基准，记录 10k / 100k 软删除清除用时 |
| M8 overlay 克隆 | 百万文件卷且 overlay 接近 10% | `apply_journal_batch` 前后打点，比较 overlay 1k / 100k 单次轮询；同时跑 `indexer-idle-cpu.ps1` |
| M7 管道轮询 | 常驻空闲 | ETW / Process Explorer 统计空闲上下文切换与 CPU |
| H2 重建频率 | 开发机日常 | 统计服务日志 `RebuildRequired` 次数与原因，修复前后各删一次 `node_modules` |
| L4 线性查找 | 历史数万条且扩展频繁 get | 10 万条历史下测 `extension_get_async` 用时 |

## 6. 执行顺序

- **优先修复**：H1 → H5 → M1 → H4 → H2 + M3 → H3 → M2。
- **后续改进**：M5、M4、M7（先 sleep，后 overlapped）、M6、L1、L4、L5、L6；M8、L2 视测量结果。
- **暂不处理**：后台下载 host 每 250 ms 查询 `engine.tasks()`（开销小，仅后台模式）；L10 保留名（仅影响报错文案）；索引 delta 尾部全零被整体拒绝（待验证假设）；Chrome Local Network Access 拦截 Blob 上传（需真实浏览器验证）。

建议验证命令（按 AGENTS.md 先设置环境变量并确认内存余量）：

```powershell
$env:CARGO_BUILD_JOBS='1'; $env:CARGO_INCREMENTAL='0'; $env:RUSTFLAGS='-C codegen-units=256'
cargo test -p nxtools-download --lib
cargo test -p nxtools-indexer
cargo test -p nxtools-archive
cargo test -p nxtools-clipboard
.\scripts\test-platformhost-fixtures.ps1
node --test browser_extension/nxtools-interceptor/tests
```

## 7. 已知限制

- 无运行时数据（ETW、日志、真实 NTFS 卷），H3、M7、M8 的实际影响程度需测量。
- H1 的“约 64 个任务”阈值取决于下载列表是否支持全选批量操作；引擎本身对批量数无上限。
- M1 中 Cookie 附带范围来自对 `request-context.js` 的初步阅读（†）。
- 全部结论基于源码阅读，未新增或运行复现测试。
