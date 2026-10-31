# 扩展开发与调试

## 从现有示例开始
- 零代码命令：[commands.json](examples/commands.json)。
- 一次性 Python/Node/PowerShell 传输：[协议](PROTOCOL.md)、[完整示例](examples/README.md)。
- 编辑器结构提示：[manifest.schema.json](manifest.schema.json)、[methods.schema.json](methods.schema.json)。Schema 是作者工具，正式解析器仍负责 UTF-8 字节预算、路径、跨字段、代码/运行时身份和信任等校验；不能凭 Schema 通过宣称插件安全。
- 可选轻量客户端：[Python](sdk/nxtools.py)、[Node ESM](sdk/nxtools.mjs)、[TypeScript 声明](sdk/nxtools.d.mts)。复制到包内时必须列入 `files`；不需要 pip/npm，不自动重试副作用。

## 离线校验（不安装、不执行）
在仓库根目录 PowerShell，按项目约束串行运行：
```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
cargo run -p nxtools-extensions --example validate_package -- docs/extensions/examples/clipboard-text-tools
```
支持目录、ZIP 或独立声明式 JSON。目录/ZIP 只读取和校验清单列出的内容，不安装、不暂存、不启动脚本；退出0有效、1校验失败、2参数错误。不要用普通 Python JSON 解析替代正式路径/身份校验。

## 必需 API 与能力发现
可选顶层字段：
```json
"requires": [{"method":"downloads.list_page","version":1}]
```
默认省略/空数组，旧包内容哈希不变。未知方法、重复要求或不支持版本在导入时拒绝；调用前再次检查宿主当前能力。GUI/后台、Clipboard 所有者是否可用，以及解析器禁止递归 probe/submit/enqueue，由 `host.describe.methods` 的 `available/unavailable_code` 表示。不要把方法存在等同于当前可调用。

## 网络目标声明（P4 实施中）
可选顶层 `"network":{"origins":["https://api.example.test","http://127.0.0.1:8080"]}` 声明最多 32 个精确 HTTP(S) origin。正式导入按协议、IDNA 主机和有效端口规范化；默认端口和单个末尾 `/` 会归一化，规范化后的重复项拒绝。支持显式 localhost、规范 IPv4/IPv6 和私有地址，不支持通配符、网段、路径、查询、片段或凭据。`network` 必须是对象，省略、`{}` 或空 origins 保持旧清单哈希；非空声明参与代码/信任哈希。
此处仅为声明解析与作者结构提示，**`http.request` 尚未实现或发布，不能加入 requires**。后续宿主 HTTP 将单独检查当前信任与目标；声明不是 OS 网络沙箱、防火墙或 DNS 重绑定防护。可信脚本仍具有当前用户的直接 OS 权限，尤其应审慎审阅本机与私有网络目标。

## 显式 v2 once（能力扩展 P0）
旧包和 SDK 默认仍为 v1/once，显式默认值不会改变旧清单哈希。新包可声明 `runtime.protocol_version:2`，并用 Python `Session(protocol_version=2)` 或 Node `Session.start(process.stdin,process.stdout,{protocolVersion:2})` 完成初始化和结束确认。进度和临时 blob 已完成源码验证，原生/视觉验收另计；宿主 `host.describe.transport` 返回实际模式、字节和调用上限，blob/存储方法要求 `minimum_transport:2`。P3 存储已接入正式 GUI/headless Desktop 组合、真实脚本、Core 持久化、延迟元数据刷新及原管理入口；阶段最终检查与原生/视觉验收另行记录。未配置存储宿主会返回 `capability_unavailable`。常驻、HTTP、图片和全源搜索仍未实现，不要仅凭清单声明判断可用。详见[协议](PROTOCOL.md)。
进度使用 `progress` helper，已知总量显示真实比例，未知不伪造百分比；100%不是成功，最后仍需complete/request_finished和原生清理。临时二进制使用 `blob_upload/blob_read/blob_release`（Node为camelCase），每blob8MiB、每调用16MiB、块64KiB、累计4transfer/512chunk；handle仅当前调用有效，不持久保存。释放/取消不代表阻塞IO和文件已实际清理；失败保留配额重试。它不是直接文件系统权限或加密存储。
持久数据使用 `storage_get/set/delete/list`（Node 为 camelCase），不接受包ID、命名空间或路径参数。JSONnull 与不存在通过 `found` 区分；CAS 冲突、失效游标和清理待办不自动重试。数据未加密，不能替代凭据存储。`host.describe` 和无效调用不分配数据；只有实际合法存储调用经当前信任与 Core 配置锁复核后才惰性分配。完整限制与 P3 集成状态见协议存储段落。

P3 的 Core/存储源码边界包含显式恢复与保留数据清理/重试：恢复要求当前信任、精确确认和空目标，提交到全新 UUID，不复活旧句柄、不合并或覆盖数据；配置提交失败保留原指针，实际删除失败保留容量并报告待清理。这不是新增脚本方法。原移除确认默认不勾选清理；插件设置中的“管理保留数据”和工作区“保留的扩展数据”进入同一管理页，全部插件移除后仍可访问。每批最多 16 条元数据，不读取值；恢复、清理和重试均需当前记录与明确确认，操作结束后需刷新再操作，不自动重放。取消仅提出请求，真实 I/O 结束前保持忙碌。数据未加密，不替代凭据存储，也不承诺安全擦除；原生/视觉/安装验收仍未执行。
恢复后的旧锁文件不是可再次恢复的数据记录；页面“重试待清理”需要另一次明确确认，仅处理此前已授权的清理义务，不会清理普通保留数据或修改当前安装。刷新只读元数据，不会自动执行清理。

## 有界分页
旧 `downloads.list`、`clipboard.query` 和 `quick_launch.search` 保持原数组返回。
- `downloads.list_page {limit:1..32,cursor?:UUID,statuses?:[]}`。
- `clipboard.query_page {query,limit:1..32,cursor?:UUID}`。
- 返回 `{items,has_more,next_cursor,cursor_ttl_ms:60000,consistency}`。每页JSON预算128KiB，最多32项，不暗中截断内容；超限返回 `content_too_large`，应显式改小limit/查询。
- 游标仅当前调用/包/处理器/代次/方法/查询有效，最多128个元数据槽、60秒有效。不可在下一次调用中继续，不要记录/分享游标；失效或预算耗尽时由用户/作者显式决定重新查询，宿主不自动重跑副作用。
- 下载按创建时间降序、ID升序的稳定键实时遍历，不受队列重排影响。并发删除不会补回，新条目和状态过滤变化不构成冻结快照；需要完整当前视图时显式重新开始。
- 剪贴板复用原串行读取通道，存储会话/历史写入/外部数据库变化后返回 `cursor_stale`，需显式重查，不继续旧排序。内容未变化时可遍历超过32项。

## 搜索范围
`quick_launch.search_catalog {query}` 返回 `{items,scope:"local_catalog",sources,excluded_sources,complete,truncated}`，最多32项。它不会启动文件索引查询或异步脚本提供者，也不会打开搜索窗口。`complete` 仅针对声明的本地目录范围，绝不是全盘完备性承诺。旧 `quick_launch.search` 同样只是该范围，保留数组兼容。

## 动作适用性
Clipboard/Downloads 手动action可声明：
```json
"accepts":{"kinds":["text"],"min_items":1,"max_items":1,"max_bytes":65536}
```
Kinds为text/files/image/download；数量1–32，字节上限1–131072。省略时保留旧包行为。菜单做保守类型/数量筛选，最终调用用完整当前输入校验。图片暂时只有引用/尺寸，不能声明为已有图像读取API；`max_bytes`仅允许可测量的text/files。声明不是安全沙箱，脚本仍应验证输入。

## 完成与错误
手动动作 `success` 或 `handled` 是插件报告完成，`not_handled` 为未处理，`error` 为失败。有效error完成仍保留有界纯文本`value.text`供用户详情显示，不是原生/协议故障；最多4096 UTF-8字节，无控制序列/可执行标记。stdout仅协议；不要输出输入原文、URL或密钥作为诊断。
宿主失败保留稳定 `value.code`，增加`message/retryable`；原生失败另有`native_error_code`。`retryable`只是提示，不是自动重试授权。取消和超时不能撤回已提交复制、派生或外部效果。

## 设置和自动化
路径字段支持浏览或手动输入，只更改当前草稿，必须保存；取消、手动编辑和旧草稿的迟到结果不会覆盖当前值。使用方法的“打开设置”直接进入插件设置草稿。
“最近活动”包含近期手动调用和最多64条自动事件的固定结果类别/时间/关联ID，不保存载荷。失败暂停后“恢复自动触发”只等待下一次新事件，不重放旧事件。移除需确认受影响完整解析链；不会静默跳过被移除阶段。

设置声明可对 text/file/folder 增加 `validation:{required:true,min_length:2,max_length:64,pattern:"[a-z]+"}`。长度按 Unicode 标量计数，同时保留32768 UTF-8字节上限；pattern按整个字符串匹配，最多512 UTF-8字节/编译与DFA各1MiB，不支持回溯引用/环视。required拒绝空白但不裁剪已接受值；文件/目录不隐式探测存在性或网络共享。密码/bool不支持这些规则。必填但未配置允许导入和加载，保存/调用会拒绝，且校验失败不会触碰凭据。choice保留旧字符串数组及旧重复项；新同构对象数组使用 `{value:"raw",label:"显示标签"}`，value必须唯一，界面显示label、保存value，未知选择不会自动替换成首项。P1源码和隔离测试已通过，真实界面及安装验收仍未运行；不代表其余六项能力已交付。

## 隔离测试
```powershell
$env:PYTHONDONTWRITEBYTECODE='1'
python -B docs/extensions/sdk/test_sdk.py -v
node --test docs/extensions/sdk/test_sdk.mjs
python -B docs/extensions/examples/test_examples.py -v
```
SDK测试使用内存流/假宿主；示例测试使用合成数据和子进程，不访问真实用户状态或网络。真实运行还需通过宿主当前信任、启用、运行时和原生生命周期检查；测试通过不等于安装/视觉/IME/DPI验收。

## 本轮验证与部署边界
设置直达已通过真实官方输入节点的宽度、点击聚焦、Ctrl+A 编辑和不自动保存回归；路径字段逐个统计用户编辑事件，同一轮 A→B→A 后的迟到选择不会回填。扩展聚焦10/10、设置23/23、严格Clippy及工作区测试编译通过。历史集成1159/1和1193/3保留；后续Archive夹具和原负责会话的排序修复已通过验证，最新一次完整Desktop1196通过/0失败/0忽略，并对并发加强后的排序用例另行复核3/3。详见[实施结果](../../.trellis/tasks/10-24-extensions-api-ux-repair/outcomes.md)。这不是冻结整个工作树或安装验收；没有部署，也没有真实路径对话框、视觉、IME、混合DPI、读屏器或已安装版本验收，使用新接口仍需后续正式构建与验收。
