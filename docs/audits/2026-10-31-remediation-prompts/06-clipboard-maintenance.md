# 第六批：剪贴板维护路径（M6、L4、L8）

前置条件：第二批（`11-01-platformhost-input-and-extension-trust`）已完成并提交，因为 L8 与它同在 `WindowsPlatformAdapter.EventWindow.cs`。

```text
你在 Windows 工作区 D:\Code\NxTools.Next-main 中工作（Rust + GPUI 桌面端，C# .NET 8 PlatformHost，PowerShell）。任务：修复剪贴板的 M6、L4、L8。

## 通用规则
- 工作区里有他人未提交的改动，必须保留；不要回退、格式化或提交与本任务无关的文件。不要 commit/push，不要打包、安装或启动产品，不要读写真实剪贴板或真实用户的历史数据库。
- 开始前阅读 AGENTS.md（Diagnostic contract：不得记录剪贴板内容）、.trellis/workflow.md、.trellis/spec/nxtools-clipboard/**、.trellis/spec/nxtools-desktop/backend/database-guidelines.md 与 quality-guidelines.md、.trellis/spec/platformhost/**、docs/reference-code/clipboard.md。按 workflow 建立任务记录（建议 11-0x-clipboard-maintenance/）。
- 先写能在旧代码上失败的回归测试，再修复，并给出先红后绿的证据。
- Rust 命令在同一进程内设置 $env:CARGO_BUILD_JOBS='1'; $env:CARGO_INCREMENTAL='0'; $env:RUSTFLAGS='-C codegen-units=256'；桌面测试使用 --config 'profile.test.package.nxtools-desktop.debug=0' 并单线程；给足超时；超时后先检查残留进程，不要重复构建。
- C# 测试：dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj，然后 dotnet run --project ... --no-build -- --fixtures '<fixture 名>'。
- 已知的无关失败（日期相关的日历测试、nxtools-archive 格式差异）不要修、不要压制，在报告中列出。行号以 2026-11-01 的工作区为准；如有偏移，按符号名定位。

## 1. M6：清理大量软删除记录时为 O(n²)（高可信风险，取决于数据规模）
- rust/crates/nxtools-clipboard/src/lib.rs:936 `cleanup`，整个过程在一个事务里：
  - 约 :995 为每一条待清除的行单独 prepare 一次格式查询；
  - :1013 对每个候选资源路径执行 `EXISTS(... image_path = ?1 OR source_process_icon_path = ?1) OR EXISTS(... clipboard_item_formats WHERE data_path = ?1)`。
- 索引定义（约 :518-592）中，image_path、source_process_icon_path 和 clipboard_item_formats.data_path 都没有索引，所以每个候选都要全表扫描。
- 触发：用户把 max_items 从数万调低，7 天后这些软删除行在一个事务里被物理清除。日常每小时的少量清理不受影响。
- 影响：清理在剪贴板 worker 线程上运行（rust/apps/nxtools-desktop/src/clipboard_worker.rs 的 apply_cleanup），期间捕获和查询都要排队；队列（容量 256）满后，新的捕获被丢弃。
- 要求：
  1. 先测量：参照 lib.rs 中现有的查询基准（约 :3501 `clipboard_history_query_benchmark_emits_json_for_10k_and_100k_fixtures`），新增默认 #[ignore] 的测量，记录 10k 和 100k 条软删除行物理清除的耗时，修复前后各测一次。
  2. 改为基于集合的实现：在事务内用临时表收集候选路径，用一条带 NOT EXISTS 的语句找出不再被引用的路径；把 prepare 移出循环。是否为 data_path 等列加索引，要权衡写入成本和 database-guidelines 的迁移规则（CREATE INDEX IF NOT EXISTS 属于向前兼容的迁移），说明结论。
  3. 返回值语义不变：soft_deleted、purged、purged_asset_paths（排序后、只包含不再被引用的路径）。共享资源不得被误删。
- 回归：现有保留策略测试（约 :2922-3200）全部保持通过；新增“两条记录共享同一图片，只清除其中一条 → 不报告该图片”和“10k 软删除清除”在合理时间内完成的断言（时间阈值宽松，避免不稳定）。

## 2. L4：find_history_item 分页线性查找（已确认）
- rust/apps/nxtools-desktop/src/clipboard_worker.rs:1239 `find_history_item` 以每页 500 条遍历全部历史，并加载完整正文，直到找到目标 ID。调用方：:814 `extension_get_async`、:1127 派生捕获时查找源条目。
- :1237 的注释说行查询函数是私有的，但 nxtools-clipboard 现在已有公开的 `ClipboardHistory::active_item(id)`（lib.rs 约 :812），同样只返回未删除的条目。
- 要求：核实 `active_item` 与“BUILTIN_ALL 分类、空查询、Content 范围”的分页结果语义等价（是否只包含未删除条目、字段是否一致），等价则直接替换并删除 find_history_item 和过时的注释；不等价就说明差异，选择保持现有语义的最小实现。
- 回归：现有 extension_get_async 与派生捕获测试（clipboard_worker.rs 约 :1680-1840）保持通过；新增“1000 条历史中查找最早一条”的测试，断言结果正确。

## 3. L8：剪贴板捕获重试时，暂存的 PNG 没有删除（已确认路径）
- csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs 的 `ReadClipboardContent`（约 :859）：约 :884-893 先把 Clipboard.GetImage() 保存为 %TEMP%\NxTools.Next\clipboard-staging\<guid>.png，随后约 :897-899 调用 Clipboard.ContainsText / GetText，可能抛出 ExternalException（剪贴板被其他程序占用）。
- WindowsPlatformAdapter.ClipboardPause.cs 约 :286 捕获该异常并重试（最多 MaxClipboardReadAttempts 次），每次都写一个新的 PNG，之前的文件没有任何代码删除。
- 要求：从保存 PNG 之后到把 imageData 交给发布路径之前，任何异常路径都调用现有的 `TryDeleteStagedImage`（约 :991）再重新抛出；成功路径和捕获上限拒绝路径（约 :941）的行为不变。同样检查 ClipboardImagePromotion 生成的暂存文件在异常时是否会泄漏。
- 回归：注入在图片暂存后抛出的文本读取异常，断言暂存目录为空，而且重试计数行为不变（参考现有 ClipboardPause 测试的注入方式）。

## 4. 验证
- cargo test -p nxtools-clipboard
- 桌面：过滤运行 clipboard_worker、剪贴板扩展适配、剪贴板页面的受影响测试
- cargo clippy -p nxtools-clipboard -p nxtools-desktop --tests -- -D warnings；对改动文件做 rustfmt 检查
- C#：新增和受影响的剪贴板 fixture；dotnet build 无警告
- git diff --check；改动规范文档时运行 python tests/architecture/spec-contracts.py

## 5. 范围外
其他审查问题、剪贴板面板的界面和交互改动、保留策略语义调整、依赖升级、打包。

## 6. 完成后的报告
每个问题的根因结论、改动文件与符号、方案理由；M6 修复前后的测量数据和索引权衡结论；L4 语义等价性的核实结论；先红后绿证据；完整验证命令的实际结果；未完成事项与无关的既有失败。
```
