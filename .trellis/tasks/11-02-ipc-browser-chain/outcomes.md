# IPC/browser 交接生命周期结果

## 结果

M7、L5、L6、L7、L9 已完成，隔离回归与相关编译、lint、静态契约通过。有界集成复核未发现具体可达缺陷或源代码阻塞。

本轮基线为138项既有脏状态和20份相关源文件副本；已有下载、索引、输入和扩展改动保留。未提交、打包、安装、启动产品、重启服务、注册 Native Messaging、访问真实网络或真实浏览器配置。测试使用临时文件/管道/回环、测试 apphost、fake host 和 TestApp；DOM 门禁使用单独临时 Chrome profile 并阻断网络。

## 根因、实现与取舍

| 问题 | 根因与修复 | 主要文件 / 符号 |
| --- | --- | --- |
| M7 | 空闲读依靠 PeekNamedPipe + 1→8ms睡眠；缺失管道时 WaitNamedPipe立即失败后空转。两端改为 FILE_FLAG_OVERLAPPED；ReadFile/WriteFile 各自持有完成事件，通过完成/绝对期限/退役事件等待。取消指定操作后用 GetOverlappedResult 等待完成，操作结构、事件和缓冲区在此之前保持有效。NotFound重试睡眠25ms，受剩余总期限约束。 | `rust/crates/nxtools-ipc/src/local_pipe.rs::{LocalPipeReader,LocalPipeWriter,complete_io,overlapped_read,overlapped_write,connect_local_pipe}` |
| L5 | 客户端在 ConnectNamedPipe 前连接又关闭会产生232，原 accept 直接返回终止调用者。对 NoData/NotConnected/BrokenPipe/OperationAborted 关闭该实例、10→100ms退避并重建；固定原生错误诊断最多每秒一次。CreateNamedPipe和不可恢复错误仍返回。Tokio平台 accept 使用相同分类。 | `local_pipe.rs::{accept_pipe_with_sddl_after_create,transient_accept_error}`；IPC `lib.rs::run_platform_server` |
| L6 | Header开关只清空文本副本，原始headers仍发送；Cookie同时存在映射和独立字段。单项/批量共用 `{headers,cookie?}` 投影，删除宿主未使用的headerText。Header为总开关，关闭时不发送Header/Cookie/Referrer，也不读Cookie API；Cookie子开关会移除已捕获Cookie。 | 扩展 `bg/request-context.js::{enrichHeadersForDownload,headerObjectToPayload}`、`bg/handoff.js::{sendDownloadPayload,sendResourcesBatch}` |
| L7 Blob | 宿主一小时启动清理和可无限等待的确认窗口共享顶层blob文件。引擎在交付确认前无替换移动到 `browser-blob-staging/pending/<id>.blob`；原import/discard优先解析已领取位置。宿主仍只扫顶层上传文件；引擎启动只回收pending孤儿，不碰未提交/正在上传文件。明确拒绝清理新领取文件；确认超时保留可能仍排队的窗口输入，由迟到确认/取消/关闭或下次启动释放。 | `rust/crates/nxtools-download/src/engine/browser.rs::{open_browser_blob_dialog_correlated,browser_blob_staging_path,discard_browser_blob,cleanup_pending_browser_blobs}`；`engine.rs` 初始化 |
| L7 Popup | 1.5秒刷新间隔大于1秒空闲释放，每次刷新都重启宿主。受信任popup.html的 `nxtools-popup` runtime端口从打开到断开持有一份原传输租约；关闭后仍按原1秒空闲释放。断连后定时刷新没有重连权，显式测试连接/新请求/重新打开可重试。状态固定“已连接 / NxTools 未运行”。 | 扩展 `bg/router.js`、`bg/state.js`、`bg/popup-state.js::checkApiStatus`、`popup.js`、`popup/render.js::renderConnection` |
| L9 | broker以int.MaxValue构造有限累计读取器，生命周期控制流超过2GiB就被拒绝。增加显式Unlimited模式，broker真实工厂选用它；无限模式不累计整数。其他调用者仍使用有限构造，单行/UTF8/截断和每次运行预算不变。 | C# `ExtensionProcessProtocol.cs::ExtensionLineReader`、`ExtensionHostBroker.cs::CreateControlReader` |

M7选择overlapped而非只替换成同步ReadFile，是因为复制的同步句柄不能保证等待读与响应写独立；既有NOWAIT写模式也会影响共享管道。显式处理两端I/O和直接可唤醒的 `PipeRetirement`，同时保留原Index服务lease flag、单coordinator和会话队列。`SharedIndexerRuntime::serve_admitted_native` 在替换、错误、退出和Drop路径传播通知；身份认证、四个接入槽、16帧输出队列、500ms绝对写期限、3秒初始Hello、1MiB帧及旧lease隔离不变，I/O线程结束后才释放槽。

L7使用引擎专属子目录，是原文件所有权的移动，不是新任务存储或新确认控制器。直接Blob导入仍接受原顶层路径，无替换最终发布和数据库失败补偿保留，补偿回到实际领取的同一暂存位置。GPUI原确认、取消和窗口关闭命令保持不变。

## 参考与兼容性

参考顺序为AB Download Manager、旧NxTools；WinShun补充可检查的原生通道实现。ABDM本地应用只有普通HTTP/HLS Header映射及Native Messaging入口，单独的浏览器扩展仓库不可用，未声称其存在Blob/Popup等价实现。旧NxTools证明Header总开关/Cookie子开关和Blob普通确认行为，不继承其冗余报文、Base64缓存清理或自动重连。完整源链见 `reference-evidence.md` 和两份更新后的参考索引。

- Browser Native Messaging保持protocol2、256KiB；Header值16KiB、Cookie64KiB限制不变。
- 宿主从protocol2基础实现起就只使用headers、独立cookie和referrer，缺失字段默认，未知headerText被忽略。旧扩展→新宿主仍可解析；新扩展→旧protocol2宿主可省略headerText而保留既有字段名，无需升级协议。
- Cookie只发送独立一份；启用时保留浏览器Cookie API返回的HttpOnly/Strict合成Cookie行为，不增加过滤。
- Download command9 / Download IPC handshake2及稳定管道的显式不兼容处理保持不变；PlatformHost outer protocol1保持不变。
- 浏览器启动/诊断不启动Native Messaging；被动快照不启动Desktop；完整Blob操作仍有外层租约；没有自主重连。
- L9保留每次运行有限预算：legacy stdin1MiB/stdout4MiB，data_plane2合计25MiB/28MiB，stderr64KiB，单行256KiB。

## 测量与先红后绿

| 边界 | 旧行为证据 | 最终结果 |
| --- | --- | --- |
| M7同一临时管道、单读端空闲10秒 | 1,224次唤醒（122.4次/秒） | 0次周期唤醒；计数仅在测试中存在 |
| M7不存在管道、180ms连接期限 | 26,832次原生重试，超过测试上限 | 重试≤10，且总耗时<500ms断言通过 |
| L5连接后立即断开再接入 | 实际ERROR_NO_DATA232导致accept返回错误，后续健康客户端无法接入 | 原生和Tokio临时管道后续接入通过 |
| L6/L7扩展8项真实后台夹具 | 7项行为失败：Header泄漏、Cookie重复超限、Popup十秒启动7次、断连刷新自动重连 | 8/8；32项×每项4KiB Cookie实际141,408字节<262,144；Popup十秒只启动1次，关闭999ms未断/1000ms断开 |
| Blob初始4项跨宿主/引擎夹具 | 3项失败：确认文件被宿主清理、无独立领取文件、pending孤儿未回收；consumer-not-ready保持性通过 | 最终5/5，增加明确拒绝和超时→迟到取消保持性 |
| Popup真实DOM标签 | 两个固定状态标签断言失败 | 38/38，包括单端口获取、pagehide释放、稳定标签和原有交互布局 |
| L9真实broker读取器工厂 | 流式超过2GiB后 `control_total_limit` | 8,193个合法262,144字节帧，共2,147,745,792字节，通过；不分配/收集2GiB输入 |

L9生成器只保留一个帧模板；读取器保留8KiB块和256KiB行缓冲，测试只保留当前解码帧。有限累计、精确行边界、超长行、UTF8和截断错误分别通过。M7数据是隔离管道唤醒计数，不是安装态服务CPU或整机性能结论。

证据日志位于会话scratch `D:\.pi-desktop\data\scratch\6a21b011-4230-4aa5-a2ed-f2dc1b4153e6`：`ipc-native-idle-before.log`、`ipc-native-red.log`、`ipc-native-idle-after.log`、`ipc-extension-red.log`、`ipc-extension-green.log`、`ipc-extension-final.log`、`ipc-blob-red.log`、`ipc-blob-green.log`、`ipc-popup-dom-red.log` 和最终分项日志。L9完整独立执行记录为该目录 `ipc-l9-outcomes.md`。

## 实际验证

所有Rust命令在自己的同一PowerShell进程中设置：

```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
$env:CARGO_NET_OFFLINE='true'
```

| 验证组 | 结果 |
| --- | --- |
| nxtools-ipc library | 54通过，其中1项是10秒测量；新增EOF保持性通过 |
| nxtools-download-ipc | 7通过 |
| nxtools-browser-host | 38通过，含5项所有权和2项报文兼容/大小回归 |
| Indexer安全library | 134通过、3忽略、5项真实卷/进程边界排除 |
| Indexer admission | 2通过 |
| Download library | 127通过 |
| engine_contract浏览器相关 | 11通过 |
| Desktop downloads::tests | 110通过 |
| Desktop runtime::tests / 索引桥 | 5通过 |
| CLI --tests检查 | 通过 |
| 七个受影响包严格Clippy --tests | 通过，新增EOF测试后IPC严格Clippy再次通过 |
| C# build | 0警告、0错误 |
| C#精确安全fixture | 3通过 |
| 全部Node测试 | 73通过 |
| 内容/精修/布局/Popup真实DOM | 27 / 20 / 38 / 38通过 |
| 定向Rust格式 | 11个文件通过，含受保护的既有Indexer tests文件 |
| diff与静态契约 | git diff --check、spec-contracts、gpui-release-contracts、markdown-editor-contracts、indexer-lifecycle通过 |

主要执行命令：

```powershell
cargo test -p nxtools-ipc -p nxtools-download-ipc -p nxtools-browser-host -- --test-threads=1
cargo test -p nxtools-ipc --lib -- --test-threads=1
cargo test -p nxtools-download --lib -- --test-threads=1
cargo test -p nxtools-download --test engine_contract browser -- --test-threads=1
cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop 'downloads::tests::' -- --test-threads=1
cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop 'runtime::tests::' -- --test-threads=1
cargo check -p nxtools-cli --tests
cargo --config 'profile.test.package.nxtools-desktop.debug=0' clippy -p nxtools-ipc -p nxtools-download-ipc -p nxtools-browser-host -p nxtools-download -p nxtools-indexer -p nxtools-cli -p nxtools-desktop --tests -- -D warnings
cargo clippy -p nxtools-ipc --tests -- -D warnings
cargo test -p nxtools-indexer --lib -- --test-threads=1 `
  --skip the_real_windows_mft_boundary_is_either_searchable_or_actionably_denied `
  --skip the_real_windows_usn_boundary_observes_a_new_file_or_is_actionably_denied `
  --skip refreshing_volumes_preserves_the_search_generation_high_water_mark `
  --skip rediscovering_volumes_keeps_the_last_good_snapshot_searchable `
  --skip stdio_handshake_returns_acknowledgement_then_current_status

# C#只选择已审查的夹具，未运行广泛原生套件。
dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build -- --fixtures `
  'extension process fixture broker unlimited control stream' `
  'extension process fixture finite reader boundaries' `
  'extension process fixture parent identity and malformed controls'

$extensionTests=(Get-ChildItem browser_extension/nxtools-interceptor/tests -Recurse -Filter '*.test.js').FullName
node --test $extensionTests
node browser_extension/nxtools-interceptor/tests/content-browser.cjs
node browser_extension/nxtools-interceptor/tests/refine-browser.cjs
node browser_extension/nxtools-interceptor/tests/layout-browser.cjs
node browser_extension/nxtools-interceptor/tests/redesign-browser.cjs

python tests/architecture/spec-contracts.py
python tests/architecture/gpui-release-contracts.py
python tests/architecture/markdown-editor-contracts.py
powershell -NoProfile -ExecutionPolicy Bypass -File tests/architecture/indexer-lifecycle.ps1
git diff --check
```

定向格式命令为 `rustfmt --edition 2024 --config skip_children=true --check`，对象为IPC lib/local_pipe、Indexer windows_service_host/session_runtime/tests、Download engine/browser、BrowserHost lib/blob_upload及两个新测试文件。未运行会改写无关Archive文件的全工作区格式化。

## 已知限制与待授权验收

没有剩余源代码阻塞。真实安装态Native Messaging stdio退出/文件锁、物理浏览器和用户交互验收不由合成夹具替代。已知无关的日期日历测试及10个Archive格式差异未修改；本轮没有运行对应日历用例或全工作区格式门禁。

M7改变了索引服务读端。当前授权不包含打包、部署或服务重启；在另行授权、当前Desktop/PlatformHost/Indexer原子包及受保护副本一致后，仍须执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\performance\indexer-idle-cpu.ps1 -RestartService -AssertRegression
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\performance\indexer-service-smoke.ps1 -Query 'NxTools.Indexer.exe' -Limit 20 -TimeoutMs 10000
```

旧安装包的结果不能作为本次源码的发布验收。
