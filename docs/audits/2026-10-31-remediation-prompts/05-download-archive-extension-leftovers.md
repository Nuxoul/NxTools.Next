# 第五批：下载、压缩、扩展遗留问题（L1、L2、L3、M5、L10）

```text
你在 Windows 工作区 D:\Code\NxTools.Next-main 中工作（Rust + GPUI 桌面端，PowerShell）。任务：修复下载引擎的 L1、L2、L3，压缩模块的 M5，扩展模块的 L10。

## 通用规则
- 工作区里有他人未提交的改动，必须保留；不要回退、格式化或提交与本任务无关的文件。不要 commit/push，不要打包、安装或启动产品，不要访问真实网络和真实用户数据。
- 开始前阅读 AGENTS.md、.trellis/workflow.md、.trellis/spec/nxtools-download/**（尤其 database-guidelines.md：schema 版本、提交顺序）、.trellis/spec/nxtools-archive/**、.trellis/spec/nxtools-extensions/**、nxtools-core 相关规范、docs/reference-code/download.md、docs/reference-code/archive.md，以及第一批的结果记录 .trellis/tasks/10-31-download-backpressure-conflict/outcomes.md（它改动了临时文件、prepared 状态和发布路径，本批不得破坏）。按 workflow 建立任务记录（建议 11-0x-download-archive-extension-leftovers/）。
- 先写能在旧代码上失败的回归测试，再修复，并给出先红后绿的证据。
- Rust 命令在同一进程内设置 $env:CARGO_BUILD_JOBS='1'; $env:CARGO_INCREMENTAL='0'; $env:RUSTFLAGS='-C codegen-units=256'；给足超时；超时后先检查残留进程，不要重复构建。
- 已知的无关失败（日期相关的日历测试、nxtools-archive 既有格式差异）不要修、不要压制，在报告中列出。行号以 2026-11-01 的工作区为准；标 † 的事实先核实，不成立就说明并跳过。

## 1. L1：长文件名导致临时文件名超过 255 字符（已确认）
- rust/crates/nxtools-download/src/lib.rs:360 `is_safe_windows_file_name` 允许 UTF-16 长度不超过 255 的文件名；rust/crates/nxtools-download/src/http.rs:1230 `artifact_paths` 在 :1244 生成 `.{file_name}.{task_uuid}.nxtools-part`，额外增加 51 个字符。所以 205–255 字符的文件名每次创建临时文件都失败。
- 第一批已把准确的临时路径持久化到 download_artifacts（prepared 状态）。已有任务的临时路径必须原样沿用，新命名只用于新生成的路径。
- 要求：列出所有派生的产物名（分段临时文件、checkpoint 名，例如 engine.rs 测试 :4920 中出现的 `.partial.bin.checkpoint.nxtools-part`、HLS/DASH 工作目录与分片名、mux 临时文件），保证每个组件不超过 255 个 UTF-16 单元。推荐：保留可读的文件名前缀，按 UTF-16 截断（不得切断代理对），保留任务 UUID 和后缀以保证唯一性。
- 回归：255 字符的中文和含代理对的文件名，新任务能创建所有临时产物并最终发布为原名；数据库中已有旧格式临时路径的任务可以恢复。

## 2. L2：取消已暂停的 HLS/DASH 任务时，在持有全局锁的情况下同步删除大量文件（已确认）
- rust/crates/nxtools-download/src/engine.rs:2305 `cancel_correlated`：:2311 取得 scheduler_gate，:2317 取得 tasks 互斥锁；对 Created/Paused/Failed 状态的 HLS/DASH 任务，在锁内调用 `remove_hls_owned_artifacts` / `remove_dash_owned_artifacts`（:2355、:2383）。它们最终执行 std::fs::remove_dir_all（engine/hls_runner.rs:1065、engine/dash_runner.rs:1330），可能删除上万个分片。
- 影响：删除期间所有需要 tasks 锁的下载 worker（每个数据块都要取锁）和 UI 快照都会停顿。
- 要求：参照同文件 :2502 `remove_correlated` 的做法，锁内只完成状态迁移、持久化和清理意图的记录，释放两把锁之后在阻塞执行器上删除；删除失败按现有诊断事件记录。要考虑崩溃发生在“已持久化 Cancelled”和“删除完成”之间时孤儿目录由谁回收（查看现有启动清理逻辑；没有的话补一个有界的回收点，并说明理由）。HLS/DASH 计划的清除（clear_hls_transfer_plan / clear_dash_transfer_plan）与数据库事务的顺序必须符合 database-guidelines。
- 回归：5000 个合成分片的已暂停 HLS 任务被取消时，另一线程在限定时间内能取得 tasks 锁或完成快照；目录最终被删除；模拟删除失败时任务仍为 Cancelled 并记录诊断。

## 3. L3（†）：分段下载中一个分段失败后，其他分段继续下载到结束
- rust/crates/nxtools-download/src/engine/segmented_runner.rs：:1553 `first_error`；:2006-2007 记录首个错误；:1600、:1863、:1876 的守卫只阻止启动新分段，正在运行的分段继续把自己的范围下载完，任务在此期间保持 Downloading；如果随后回退到单流下载，这些字节会被丢弃并重新下载。
- 要求：先核实上述行为。若成立，记录首个错误后立即通知其余运行中的分段停止，复用现有的暂停/停止路径，把进度保存为 Waiting（约 :1713-1722），不重启分段。注意错误分类（可重试/不可重试）和自适应分段（grow/shrink）的既有语义与测试，不得改变。
- 回归：本地 HTTP 测试服务器上两个分段，一个立即返回错误，另一个被限速；断言另一个分段在限定时间内停止，任务进入失败或回退路径，且已下载的进度与 checkpoint 一致。

## 4. M5：以 `./` 开头的 TAR 包无法解压（已确认）
- rust/crates/nxtools-archive/src/path_policy.rs:9 `components` 在 :25 拒绝任何等于 "." 的路径段；`./` 目录项去掉末尾斜杠后变成 "."，同样被拒绝。:66 `annotate` 给这些项打上 UnsafePath 警告；planning.rs:153 只要有一项带这个警告，整个解压计划就失败（:165 和 :224 也调用 validate_relative）。
- formats/tar.rs 约 :284 的 display_path 是原始名称，没有归一化。
- 触发：`tar -czf x.tgz -C dir .` 生成的包（条目形如 `./`、`./a.txt`、`./d/b.txt`）在 TAR/TGZ/TBZ/TXZ/TZST 中很常见。可以列出内容，但所有解压任务都报“不安全路径”。
- 要求：增加一个共用的输出相对路径投影函数（例如 path_policy::output_relative）：去掉开头重复的 `./`；单独的 `.` 或 `./` 目录项视为根目录并跳过；路径中间出现的 `.` 仍然拒绝。在 components/annotate、planning.rs 的各个 validate_relative 调用、worker_codecs.rs（约 :631、:802）、worker_conflict.rs（约 :116-122、:177）的成员名比较中统一使用这个投影。display_path 和 raw_name 保持原样（TAR 编辑和 7z/RAR 身份比对依赖原值）。
- 回归：新增 dotprefix.tar 夹具（`./`、`./a.txt`、`./d/b.txt`），解压全部内容的计划成功，输出 a.txt 与 d/b.txt，单根智能判断正确；`a/./b`、`../x`、`./../x` 仍被拒绝；冲突重命名与列表展示不受影响。

## 5. L10（†）：扩展包 ID 接受 Windows 保留设备名；旧代码快照永不清理
- rust/crates/nxtools-extensions/src/lib.rs:383 `valid_id` 只允许小写字母、数字和 `._-`，不检查 con/nul/aux/prn/com1 等保留名；bundle.rs:51 `packages_root.join(&self.manifest.id)` 后 create_dir_all，导入确认阶段才以泛化的 FileOperation 失败。script.rs 中的 relative_code_path 已有保留名检查，可复用。
- bundle.rs:40-97 每次导入或更新都生成 packages/<id>/<content_hash>；rust/crates/nxtools-core/src/extensions.rs 的导入、替换（约 :202、:227）和移除（约 :451-540）从不删除旧的哈希目录；崩溃残留的 `.stage-*` 目录也只在进程内失败时清理。
- 要求：
  1. valid_id 复用保留名检查，返回 InvalidManifest。
  2. 在配置锁内、提交成功之后，删除 packages/<id>/ 下没有任何已安装包引用的哈希目录；包被移除时删除整个 packages/<id>/；暂存开始前清理过期的 `.stage-*`。只删除确定由本模块创建、且无引用的目录，遇到重解析点要拒绝。保持 core/extensions.rs :199-201 注释所说的共享哈希不删除的语义。
- 回归：ID 为 "nul" 时预览返回 InvalidManifest；安装 → 替换 → 移除后 packages/<id> 不存在；仍被引用的哈希目录保留。

## 6. 验证
- cargo test -p nxtools-download --lib
- cargo test -p nxtools-download --test engine_contract -- --test-threads=1
- cargo test -p nxtools-download-ipc
- cargo test -p nxtools-archive（含新夹具）
- cargo test -p nxtools-extensions -p nxtools-core
- 桌面：过滤运行 downloads:: 与扩展页受影响测试（--config 'profile.test.package.nxtools-desktop.debug=0'，单线程）
- cargo clippy（以上各包）--tests -- -D warnings；对改动文件做 rustfmt 检查；git diff --check；改动规范文档时运行 python tests/architecture/spec-contracts.py

## 7. 范围外
其他审查问题、下载界面和压缩界面的改动、协议版本变更（如必须变更，先停下来说明理由）、依赖升级、打包。

## 8. 完成后的报告
每个问题的根因结论（尤其是 † 项的核实结果）、改动文件与符号、方案理由；对已有任务和数据库的兼容说明；先红后绿证据；完整验证命令的实际结果；未完成事项与无关的既有失败。
```
