# 正式 Release50 安装包

2026-11-05。当前工作树通过原 `scripts/package-local.ps1 -SkipVerification` 重新构建、原发布身份签名并生成正常 Inno 安装器。正式 package.ps1 子进程退出码为 0，独立产物核验通过。未安装、启动产品、部署保护副本、重启服务或提交推送。

## 产物核验

| 项目 | 结果 |
| --- | --- |
| 版本 / 发布序号 | 0.7.0 / 50 |
| Release ID | `8a76f89c-04d1-4c9f-bff3-1964cf11af16` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.0.exe` |
| 大小 | 27,511,029 字节 / 26.24 MiB |
| SHA256 | `E7B8818D8281560FA6248390AD4B09AF8822DC0DC89EEF3729A3D90E3AF96E9A` |
| 内部签名 | 现有公开信任锚，RSA3072-PSS/SHA256 独立核验通过 |
| Manifest SHA256 | `498aa55dd98158c3ba2a0dd3896db35393f9c271a421faab9706815b632daf7e` |
| 签名成员 / 精确文件集合 | 78 / 80，全部一致 |
| 实际构建输出 / 扩展源文件 | 10 / 65，全部与包内一致 |
| 归档批准 / helper Desktop 绑定 | 5 项批准及绑定一致 |
| 安装器 / BrowserHost 替换隔离场景 | 11 / 5 通过 |
| 源码基线 | 1,266 项，打包期间无新增、删除或内容变化 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

安装准备 helper 保持 `4CDFEAC77D33715CD6800AD27CDCC3D60F58E05198500FEB8CECAAD36A76FC1A`。原包与历史记录保留，不复用已占用发布序号。

持久证据：`.diagnostics/packaging/20261105-124405-4e09c0a9/`，包含原日志、旧包备份、原打包进程退出事实、`independent-verification.json` 和 `verification-reuse.json`。控制器与源码基线：`D:\.pi-desktop\data\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\formal-repack-20261105-124404`。构建采用 jobs1、incremental0、codegen-units256，C# `UseSharedCompilation=false`；构建后无 Cargo/rustc/clippy/ISCC 活动。

## 包含范围与验证复用

本包首次纳入 [Incoming Add 独立限定接受](../11-02-archive-workflow-optimization/incoming-add-independent-acceptance.md)：明确选择“添加到当前压缩包”后直接提交原 Add，请求保留完整来源和原目录/快照/连接绑定，不再创建额外编辑表单。原主动添加和粘贴保留表单。

ZIP 既有反斜杠结尾目录的比较空间规范化修复保留原编码记录、来源和一次发布边界。失败通过原 NotificationCenter 提供友好原因、去重及主窗口隐藏时的独立通知显示；真实已发布、未发布、未知和清理事实不被文案覆盖。

复用的独立记录包含 Rust 546 项、C# 精确 11 项通过，另有四次原 Desktop 按钮到原生桥接、14 项实际 incoming 操作、五格式 145 场景及 ZIP 保留 17 场景。它们是先前的限定验收，不是本次重新执行。原输出清理、7z ResourceLimit、速启 Listary 对齐和其他 OPT 阶段证据保留，各自范围不扩大。

本次未重跑全量 Rust/C#、HZ5 或 OPT 原生矩阵。复用来源逐项列于 `verification-reuse.json` 与安装器 `evidence.json`；源码一致性只证明打包期间状态不变。

## 已知限制与待验项

- 最新串行工作区验证在 69 个已完成测试二进制记录 2,487 通过、1 失败、2 个固有忽略；Desktop 为 1,744 通过、1 失败、1 忽略。唯一观察到的失败是下载菜单测试拒绝源码中的 `.disabled(batch_busy)`，与当前异步批量契约冲突。后续目标未执行；本次未修改相关产品逻辑或断言，不宣称全量通过。
- 索引器安装态 CPU/I/O、重启后的 base+delta 恢复及真实搜索发布门禁未执行。须另行授权部署匹配原子包，再运行 `tests/performance/indexer-idle-cpu.ps1 -RestartService -AssertRegression` 与 `tests/performance/indexer-service-smoke.ps1 -Query 'NxTools.Indexer.exe' -Limit 20 -TimeoutMs 10000`。
- 物理 DOpus/Explorer、输入/混合 DPI、用户视觉、真实安装和整体性能不由本次打包接受。归档仍 **Phase E — Format expansion and ZIP editing / in_progress**，G-00、OPT-00/01 整体、完整 G-04、真实 G-18/G-28 和 G-29 前置保持原记录。
- HZ5 默认关闭、Stored ZIP/同卷 NTFS/独立不可逆确认/继续剩余解压边界不变，不承诺原归档重建或断电认证；源回收能力不因打包放开。

检查点仅 prepend，原 299,670 字节保持精确 suffix。
