# 普通速启输入与统一提交 — 2026-11-17

## 结果

普通速启按已核实的 Listary 普通启动器链路处理输入与结果提交：

- 输入变化在下一应用轮次派发，没有固定65ms防抖；扩展调用没有固定150ms启动等待。
- 当前索引和扩展provider全部结束后，走现有合并排序入口，一次替换结果；没有150ms部分发布或终态稳定追加。
- 等待期间保留上次已提交的列表，但阻止其执行、预览和动作入口。完整空结果清空旧列表。
- 索引失败或断连结束对应provider，剩余当前扩展仍参与完成汇合。索引失败清除该来源候选，最终提交保留查询错误；新查询清除该失败状态。
- 保留请求取消、代次/查询/窗口身份、IME、焦点、预览和原生显示/隐藏生命周期。输入和跨实体副作用仍延后执行，避免GPUI实体重入。

文件对话框附着搜索是独立模式，仍保留65ms输入防抖、150ms首发期限和稳定后续合并。匹配算法、历史、索引服务、缓存及用户配置未改。

## 参考与实现边界

固定参考：`D:\Code\Listary.Decompiled\6.3.5.94`，持久研究为该目录的 `notes\search-dispatch-and-publication.md`。

已核对的托管链路：

- `Core/ViewModels/SearchMode.cs::Query`、`method_0`：输入变化启动provider；普通模式等待 `Task.WhenAll`，检查取消，合并排名，再检查取消并替换Results。
- `Core/Search/Class269.cs`、`Class268.cs`：替换查询释放旧scope并取消token。
- `Core/Launcher/Class293.cs::Class294.method_0`：普通启动器未启用另一个20ms增量发布模式。
- `Main/Views/LauncherSearchWindowFactory.cs::CreateView` → `LauncherSearchWindow.xaml.cs`构造 → `SearchWindowBase.cs`构造/`method_7` → 原Show/Hide/activate/close绑定。

实现入口：

| 文件 | 职责 |
| --- | --- |
| `repo:rust/apps/nxtools-desktop/src/quick_launch/panel.rs` | 下一轮输入派发，保持当前输入、代次与窗口检查 |
| `repo:rust/apps/nxtools-desktop/src/quick_launch.rs` | 索引终态、统一完成汇合和现有合并排序后的替换 |
| `repo:rust/apps/nxtools-desktop/src/quick_launch/extension_providers.rs` | 无固定启动等待，积累当前批次并按provider完成触发汇合 |
| `repo:rust/apps/nxtools-desktop/src/quick_launch/search.rs` | pending更新不提交，查询会话失败状态随退休清除 |
| `repo:rust/apps/nxtools-desktop/src/quick_launch/file_dialog_search.rs` | 独立拥有附着模式150ms期限，保持原行为 |

沿用原有带请求身份的provider、发布和取消诊断，不记录输入内容或高频帧。传输层事件唤醒的独立证据见[索引命令事件唤醒审计](2026-11-17-indexer-dispatch-wakeup.md)。

## 验证

证据目录：`artifacts/performance/quick-launch-normal-search-20261117/`。`verification.json`记录命令、退出码、完整失败名称及与前一阶段的集合比对；日志为UTF-8。

| 检查 | 结果 |
| --- | --- |
| 原输入策略行为回归 | 未推进防抖时钟时没有派发；有效行为红灯保存在 `input-behavior-red.log` |
| 速启回归 | 213通过、0失败 |
| 完整Desktop单元测试 | 1959通过、21失败、1忽略；840.25秒 |
| 完整运行中的速启/runtime | 213/213、14/14通过 |
| Desktop HTTP集成测试 | 10/10通过 |
| `cargo clippy -p nxtools-desktop --tests -- -D warnings` | 通过 |
| `cargo check --workspace --tests` | 通过 |
| 本次9个Rust文件的rustfmt检查、相关文件差异空白检查 | 通过 |
| 规范链接与契约结构 | 通过：130文档、14workspace owners |
| GPUI release / Markdown editor契约 | 通过 |
| 普通速启无定时器/完整provider提交源码门禁 | 通过 |
| 完整速启生命周期源码门禁 | 未通过：既有OneCommander断言，见已知问题 |

`repo:rust/apps/nxtools-desktop/src/quick_launch/normal_search_tests.rs`驱动真实InputState与typed Search：逐键输入无需推进时钟；非空partial在推进1秒虚拟时钟后仍不发布；terminal整体提交；替换输入和旧代次隔离有效。

`repo:rust/apps/nxtools-desktop/src/quick_launch/provider_completion_tests.rs`覆盖正常完成、断连、失败三类索引终态，以及索引先完成/扩展先完成两种顺序。两个当前扩展必须全部结束，旧代次完成不能释放当前计数；最终结果不携带旧查询行，失败提示保留且新查询清除失败状态。

现有取消、满队列退休、隐藏/重开隔离、IME、动作信任和内容身份、Enter/指针执行、预览控制及附着搜索回归继续通过。所有本阶段Cargo命令串行，使用 `CARGO_BUILD_JOBS=1`、`CARGO_INCREMENTAL=0`、`RUSTFLAGS=-C codegen-units=256`。

## 已知问题与验收限制

完整Desktop的21个失败名称与前一阶段完全一致，新增失败集合为空：Archive9项、Downloads10项、Main window2项。完整断言保存在 `desktop-tests.log`；前一阶段分类见[索引调度审计](2026-11-17-indexer-dispatch-wakeup.md)。这些失败未在本次搜索修改中修复。

完整速启生命周期门禁仍要求旧OneCommander文件中的 `CurrentPathGet`、`CurrentPathSet` 和 `AutomationElement.FromHandle`。当前前两项位于 `OneCommanderUiaSession.cs`，实现使用独立UIA封装；本次未放宽该断言。普通搜索新检查在该失败之前实际执行并通过。

本次未打包、部署、替换受保护发行元组、重启真实索引服务或修改个人历史。未执行新安装版A/B，因此没有新的输入到可见结果延迟数值。托管参考和编译回归不证明Listary原生引擎内部等价，也不代替真实安装版交互、DPI及用户验收。
