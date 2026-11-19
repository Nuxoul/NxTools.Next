# 速启索引命令事件唤醒 — 2026-11-17

## 结果

索引客户端已从100ms停止通道轮询改为事件驱动命令派发。搜索和取消命令直接唤醒原有有界命令接收器；空闲收包结束直接唤醒重连。没有新增依赖，也没有缩短轮询间隔或引入忙循环。

实现入口：`repo:rust/apps/nxtools-desktop/src/runtime.rs#run_indexer_service_at`。
真实管道回归：`repo:rust/apps/nxtools-desktop/src/runtime/indexer_dispatch_tests.rs`。

## 参考与实现边界

参考资料固定保存在 `D:\Code\Listary.Decompiled\6.3.5.94`，搜索链路说明为该目录的 `notes\search-dispatch-and-publication.md`。已核对普通输入、provider任务、UniFFI回调完成和取消复查边界。借鉴的是请求/完成事件唤醒与过期请求隔离，不是复制专有实现；Listary原生引擎内部索引和队列仍未知。

- 原有索引supervisor线程运行一个current-thread Tokio runtime；停止、reader完成和原命令通道共同等待，停止和reader完成优先。
- 收包仍由原同步reader线程负责；其完成sender在返回或unwind时关闭，通知supervisor。
- 原连接/握手、命令FIFO、请求ID、取消关联ID、查询代次、有界准入和断连清队列保持不变。
- 重连保留1秒退避，停止可打断；重连join已结束reader，桌面退出不等待进程生命周期reader。
- 65ms输入防抖、150ms首批发布期限、终态补齐、稳定追加、匹配排序和GPUI跨entity延后更新保持不变。
- 原下载资源关闭改动保留，没有修改索引服务、缓存、卷配置或用户历史。
- 诊断沿用请求关联的搜索/取消生命周期；runtime/reader启动失败在传输边界记录固定事件和原生错误码，不记录查询内容或高频帧日志。

## 验证

证据目录：`artifacts/performance/quick-launch-dispatch-20261117/`。
其中 `verification.json` 保存命令退出码、子集计数和完整失败名称；其余文件为UTF-8日志。

| 检查 | 结果 |
| --- | --- |
| 基线真实管道搜索派发回归 | 原100ms轮询实现未在50ms内将完整搜索帧送到对端；`baseline-regression.log` |
| 事件唤醒真实管道回归 | 8通过，0失败；`desktop-tests.log` |
| 完整Desktop单元测试 | 1957通过、21失败、1忽略；耗时804.57秒 |
| 上述完整运行的Quick Launch子集 | 211通过，0失败 |
| 上述完整运行的runtime子集 | 14通过，0失败，包含新增8项 |
| Desktop HTTP集成测试 | 10通过，0失败；`extension-http.log` |
| `cargo clippy -p nxtools-desktop --tests -- -D warnings` | 通过；`clippy.log` |
| `cargo check --workspace --tests` | 通过；`workspace-check.log` |
| 两个修改Rust文件的rustfmt检查 | 通过 |
| `python tests/architecture/spec-contracts.py` | 通过：130文档、14workspace owners |
| `python tests/architecture/gpui-release-contracts.py` | 通过 |
| `python tests/architecture/markdown-editor-contracts.py` | 通过 |
| `tests/architecture/quick-launch-lifecycle.ps1` | 未通过：OneCommander源码门禁与当前拆分/API不一致，见已知问题 |

8条新增回归使用随机命名的本机真实Windows管道，经过生产连接、Hello/HelloAck、编码和发送路径，覆盖：

1. 搜索在50ms内到达真实管道对端。
2. 异步取消独立唤醒，不依赖后续搜索，并保留原请求关联ID和代次。
3. 搜索、取消、替换搜索的顺序和身份保持完整。
4. 没有待发送命令时，非终态和终态批次仍完整送达原事件接收器。
5. 空闲服务端EOF触发重连，断连拒绝准入，重连握手后可及时发送新查询。
6. 空闲停止无需新的管道输入。
7. 停止sender关闭可唤醒退出。
8. 停止可打断重连退避。

完整运行同时通过旧结果隔离、首批发布期限、终态追加、清空/隐藏取消、满队列取消和旧关闭回调不干扰重开查询等原有速启回归。

所有Cargo命令串行执行，使用 `CARGO_BUILD_JOBS=1`、`CARGO_INCREMENTAL=0`、`RUSTFLAGS=-C codegen-units=256`。

## 已知问题与验收限制

完整Desktop单元测试的21项未通过不在本次索引调度修改范围内。7项缺少专用原生夹具导出，14项断言失败；全部名称和原始断言保存在 `verification.json`、`desktop-tests.log`。

| 分组 | 数量 | 本次实际失败 |
| --- | ---: | --- |
| Archive AC进度wire | 3 | 缺少 `NXTOOLS_ARCHIVE_AC_PROGRESS_WIRE` 导出 |
| Archive OPT03创建结果wire | 4 | 缺少 `NXTOOLS_OPT03_CREATE_WIRE` 导出 |
| Archive树目录标签 | 1 | 固定树行内的单行标签bounds断言失败 |
| Archive拖动读取hydration | 1 | 接收文件夹条目的 `ordinal.is_none()` 断言失败 |
| 下载失败通知 | 1 | 预期1条通知，实际0条 |
| 下载headers输入框布局 | 1 | 实际713×120，预期756×120 |
| 后台限速设置校验 | 1 | 草稿错误校验未返回预期错误 |
| 下载设置任务夹具 | 7 | 夹具显式pause时任务已Completed，返回InvalidAction |
| 主窗口下载菜单 | 1 | 不可用操作未按断言从菜单省略 |
| 主窗口titlebar布局 | 1 | 断言实际76px，预期4px |

速启生命周期源码门禁在 `tests/architecture/quick-launch-lifecycle.ps1:1720`、`:1721`、`:1723` 未匹配。门禁只读取 `OneCommanderFileManagerOperations.cs`，要求 `CurrentPathGet`、`CurrentPathSet` 和 `AutomationElement.FromHandle`；前两项现位于 `OneCommanderUiaSession.cs`，当前实现采用独立UIA封装。

本次未打包、未部署、未替换受保护发行元组、未重启真实索引服务，也未执行优化后的安装版A/B。真实管道测试证明发送不再受100ms周期门槛限制，不能据此宣称整个输入到可见结果链路已达到某个延迟。安装版收益仍需单独授权部署匹配的Desktop/PlatformHost/Indexer发行元组，并按既有测速口径复测。
