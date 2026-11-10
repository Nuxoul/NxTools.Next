# 第四批：跨进程通道与浏览器链路（M7、L5、L6、L7、L9）

```text
你在 Windows 工作区 D:\Code\NxTools.Next-main 中工作（Rust + GPUI 桌面端，C# PlatformHost，Rust BrowserHost Native Messaging，MV3 浏览器扩展，PowerShell）。任务：修复 M7、L5、L6、L7、L9。

## 通用规则
- 工作区里有他人未提交的改动，必须保留；不要回退、格式化或提交与本任务无关的文件。不要 commit/push，不要打包、安装或启动产品，不要重启服务，不要访问真实网络、真实浏览器配置或真实用户数据，不要注册 Native Messaging。
- 开始前阅读 AGENTS.md（Diagnostic contract、Indexer release gate）、.trellis/workflow.md、.trellis/spec/nxtools-ipc/**、nxtools-download-ipc/**、nxtools-browser-host/**、browser-extension/**、platformhost/**，以及 docs/architecture.md 开头的进程模型（Blob 交接：随机回环端口、一次性、当前用户、1 MiB 分块、2 GiB 上限；引擎拥有确认、最终导入和取消清理）。.trellis/spec/shared/ui-design.md 中 “Installed neutral/focus and idle-host correction” 一段规定：浏览器启动和诊断从不启动 Native Messaging；只有待处理请求和完整的 Blob 操作持有按需传输，空闲 1 秒后释放，不自动重连；被动快照不得启动桌面。按 workflow 建立任务记录（建议 11-0x-ipc-browser-chain/）。
- 先写能在旧代码上失败的回归测试，再修复，并给出先红后绿的证据。
- Rust 命令在同一进程内设置 $env:CARGO_BUILD_JOBS='1'; $env:CARGO_INCREMENTAL='0'; $env:RUSTFLAGS='-C codegen-units=256'；给足超时；超时后先检查残留进程，不要重复构建。
- 扩展测试遵循 .trellis/spec/browser-extension/index.md：没有 npm，用 node --test 运行 tests 下的 *.test.js，加上真实 DOM 门禁（临时 Chrome profile，屏蔽网络）。
- C# 测试：dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj，然后 dotnet run --project ... --no-build -- --fixtures '<fixture 名>'（选择语法见 Program.cs 的 TestRunOptions 与 scripts/build.ps1）。
- 已知的无关失败（日期相关的日历测试、nxtools-archive 格式差异）不要修、不要压制，在报告中列出。行号以 2026-11-01 的工作区为准；标 † 的事实先核实，不成立就说明并跳过。

## 1. M7：local_pipe 的同步 I/O 用轮询和自旋实现（已确认）
- rust/crates/nxtools-ipc/src/local_pipe.rs:65-115 `LocalPipeReader::read`：PeekNamedPipe 读不到数据时睡 1→8 ms 后重查（:102-105）。空闲时每个读端约每秒唤醒 125 次。索引服务、桌面的索引桥、下载 IPC 都使用它。
- :187-221 `connect_local_pipe`：管道尚不存在时 WaitNamedPipeW 立即失败（NotFound），循环没有任何睡眠，直到超时都在空转。桌面冷启动期间，CLI（rust/apps/nxtools-cli/src/lib.rs 约 :1018-1026，5 秒）和 BrowserHost 连接会占满一个核心。
- 要求：
  1. 连接循环：WaitNamedPipeW 返回 NotFound 时有界睡眠（例如 25-50 ms，不超过剩余期限）；保留现有错误分类和总超时语义。
  2. 读端：去掉空闲轮询。可选方案：(a) 以 FILE_FLAG_OVERLAPPED 打开/创建管道，ReadFile 加事件等待，用 initial_frame_deadline 计算等待超时，到期后 CancelIoEx；(b) 阻塞 ReadFile，期限由独立机制配合 CancelSynchronousIo 实现。选择改动面最小且能保持现有语义的方案（握手期限、BrokenPipe/NoData/NotConnected 视为 EOF、写端行为、跨线程使用方式），并写明理由。改为 overlapped 会影响写端，必须一并处理。
  3. 先测量再修改：用一个测试或测量脚本，统计 10 秒空闲期间的读端唤醒次数（或线程 CPU 时间），修复前后各测一次，写入 outcomes。
- 回归：握手期限仍然生效；对端关闭时读返回 EOF；1 MiB 帧上限不变；连接循环在管道不存在时不空转（统计调用次数或耗时）。

## 2. L5：ConnectNamedPipe 的瞬时错误让服务端永久退出（已确认）
- local_pipe.rs:169-174：除 ERROR_PIPE_CONNECTED 外的任何错误都关闭句柄并返回 Err。客户端连接后在服务端调用 ConnectNamedPipe 之前就关闭，会得到 ERROR_NO_DATA，这是正常的瞬时情况。
- 调用方：rust/crates/nxtools-download-ipc/src/lib.rs:275-291 `run_download_host` 收到 Err 后 mark_failed 并 return；rust/apps/nxtools-desktop/src/runtime.rs:311-339 没有重启这个线程，所以此后所有浏览器交接和 CLI 都失败，直到桌面重启。索引服务的 accept 循环（rust/apps/nxtools-indexer/src/windows_service_host.rs）与 tokio 版平台服务器（rust/crates/nxtools-ipc/src/lib.rs:894-921，`connect().await.context(...)?`）也要检查同类问题。
- 要求：ERROR_NO_DATA 等瞬时错误：关闭该实例、重建并继续 accept；不可恢复的错误（如 CreateNamedPipe 失败）保持现有行为。连续失败加有界退避和限速日志，不得刷日志或空转。
- 回归：模拟客户端连接后立即断开，服务端继续接受下一个客户端。

## 3. L6：浏览器交接请求重复携带 Cookie；“携带必要 Header”开关关闭后 headers 仍被发送
- browser_extension/nxtools-interceptor/bg/handoff.js：单个请求 :174-176 同时发送 `headers`、`headerText`、`cookie`；批量请求约 :328 发送 requestHeaders，:382 再发送 cookie。
- bg/request-context.js:336-339：includeRequestHeaders 关闭时只清空 headerText 和 cookie，`headers`（可能含 authorization）照常发送。
- rust/apps/nxtools-browser-host/src/lib.rs：:827 从 headers 中过滤掉 cookie，:840 使用独立的 cookie 字段；是否读取 headerText（†），请核实。:32-33 限制：单个 header 值 16 KiB，cookie 64 KiB。扩展侧 bg/native-transport.js:307 限制整个请求不超过 MAX_NATIVE_REQUEST_BYTES。
- 影响：Cookie 很长的站点，32 项批量交接可能超过请求上限导致整批失败；关闭开关后认证头仍被发送，与用户预期不符。
- 要求：
  1. 先查 README 和规范，确认“携带必要 Header”开关和 includeCookies 的原意，按原意让 headers 受开关控制（关闭时只保留原意允许的最小集合，或完全不发送）。
  2. 去掉宿主不使用的冗余字段，cookie 只发送一份；按扩展与宿主的版本兼容规则处理（宿主是否拒绝未知字段，旧扩展配新宿主是否兼容），并写明结论。
- 回归：开关关闭时，交接报文中没有 authorization、cookie 和 headerText；32 项、每项 4 KiB Cookie 的批量报文低于上限；宿主端解析和过滤测试通过。

## 4. L7：Blob 暂存清理可能删除确认框仍需要的文件；弹窗轮询反复拉起本机宿主
- rust/apps/nxtools-browser-host/src/blob_upload.rs:74 每次宿主启动都调用 :642 `cleanup_stale_staging_files`，删除超过 1 小时的 *.blob。文件交给桌面确认框后，宿主不再跟踪它（lib.rs 约 :555，†），而用户点确认时引擎还需要它（rust/crates/nxtools-download/src/engine/browser.rs 的 BrowserImportBlob/import_blob_artifact 路径）。宿主进程频繁启停，所以确认框开着超过 1 小时后确认会失败。
- browser_extension/nxtools-interceptor/popup/actions.js:170-174 每 1500 ms 刷新一次，而 bg/state.js:25 `DESKTOP_IDLE_RELEASE_MS = 1000`：每次刷新都会启动再关闭一次宿主进程，并写日志、跑一遍暂存清理。
- 要求：
  1. 按所有权规则（引擎拥有确认、最终导入和取消清理）处理暂存文件的生命周期：交给确认框后，由引擎或桌面持有，宿主的启动清理不得删除仍被持有的文件；确认框取消或关闭时由持有者清理；孤儿文件由持有者在启动时按规则回收。选最小方案并说明。
  2. 弹窗打开期间持有一个传输租约（或改为订阅），弹窗关闭后按原规则 1 秒释放；不得改变“浏览器启动和诊断不启动 Native Messaging、被动快照不启动桌面”的约束。弹窗显示稳定的连接状态（已连接 / NxTools 未运行），不随轮询闪动。
- 回归：确认框开着超过清理阈值时文件仍在，确认成功；取消后文件被清理；弹窗打开 10 秒内宿主只启动一次（用现有 fake host 夹具 tests/harness 统计）。

## 5. L9：扩展代理控制通道有 2 GiB 累计上限（高可信风险）
- csharp/NxTools.PlatformHost/ExtensionHost/ExtensionHostBroker.cs:94 `new ExtensionLineReader(input, int.MaxValue, "control")`；Windows/Extensions/ExtensionProcessProtocol.cs:140-161 用 int 累计已读字节，:159 一旦超过 totalLimit 就抛出 control_total_limit。
- 影响：broker 生命周期内控制流量累计接近 2 GiB 后（例如多次大块数据写入），broker 退出，所有扩展运行被拒绝。单行仍有 256 KiB 上限。
- 要求：控制流使用不检查累计总量的模式（显式参数或独立构造，不要靠 int.MaxValue 的特殊值）；其他调用点的总量限制保持不变；单行上限不变。
- 回归：用假流灌入超过 2^31 字节的合法帧（流式生成，不要在内存中分配），读取器持续工作。

## 6. 验证
- cargo test -p nxtools-ipc -p nxtools-download-ipc -p nxtools-browser-host
- cargo test -p nxtools-indexer（读端改动会影响索引服务）
- cargo check -p nxtools-cli --tests
- 桌面过滤运行受影响测试（下载交接、索引桥）
- cargo clippy（以上各包）--tests -- -D warnings；对改动文件做 rustfmt 检查
- C#：新增和受影响的扩展代理 fixture
- 扩展：node --test 全部 *.test.js，以及 content-browser.cjs、refine-browser.cjs、popup 相关门禁
- git diff --check；改动规范文档时运行 python tests/architecture/spec-contracts.py
- M7 改动了索引服务的管道读端，AGENTS.md 的 Indexer release gate 需要打包和服务重启，不在授权内：不执行，在报告中列为“待授权执行”并写出命令。

## 7. 范围外
其他审查问题、扩展的界面改版、page-hook.js 的信任模型、drop target 的产品语义、依赖升级、打包、Native Messaging 注册。

## 8. 完成后的报告
每个问题的根因结论、改动文件与符号、方案理由（尤其是 M7 读端方案和 L7 所有权设计）；扩展与宿主的兼容说明；M7 修复前后的测量数据；先红后绿证据；完整验证命令的实际结果；待授权执行的发布门禁；未完成事项与无关的既有失败。
```
