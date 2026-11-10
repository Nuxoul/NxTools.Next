# 正式 Release45 安装包

2026-11-04。当前源码经原 `scripts/package-local.ps1 -SkipVerification` 完成实际 Release 构建、原发布身份签名与正常 Inno 安装器生成。正式 `package.ps1` 子进程退出码为 0，独立产物核验通过。未安装或启动产品、部署受保护副本、重启服务或提交推送。

## 产物核验

| 项目 | 结果 |
| --- | --- |
| 版本 / 发布序号 | 0.7.0 / 45 |
| Release ID | `8648890a-50db-41d3-959d-4ded6ee1f82c` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.0.exe` |
| 大小 | 27,482,071 字节 / 26.21 MiB |
| SHA256 | `73180348434EBD98DE22F165F18021D9FE1ED4775C4B163659747655113E9459` |
| 内部签名 | 现有公开信任锚，RSA3072-PSS/SHA256 独立验证通过 |
| Manifest SHA256 | `23b89c7e55a8fe8841ec5183b17bacc68732839f9e3a328a035df7fc56807cb3` |
| 签名成员 / 精确包集合 | 78 / 80，大小、内容与集合一致 |
| 实际构建输出 | 10 项一致 |
| 浏览器扩展源文件 | 65 项与包内文件一致 |
| 归档批准 / helper Desktop 身份 | 5 项批准及绑定一致 |
| 安装器 / BrowserHost 替换隔离场景 | 11 / 5 通过 |
| 源码基线 | 1,259 项，打包期间无新增、删除或内容变化 |
| Windows Authenticode | `NotSigned`，不同于内部发布签名 |

安装准备 helper 保持 `4CDFEAC77D33715CD6800AD27CDCC3D60F58E05198500FEB8CECAAD36A76FC1A`。既有 [运行中升级记录](installer41-running-upgrade-outcomes.md) 保留历史归属，不作为本包已安装的证明。

持久证据：`.diagnostics/packaging/20261104-151404-22f10991/`，包含原日志/旧包备份、状态与进程退出证据、`independent-verification.json`、`verification-reuse.json` 和 `installed-tuple-observation.json`。源码基线与控制器记录位于 `D:\.pi-desktop\data\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\formal-repack-20261104-151402`。构建后无 Cargo/rustc/clippy/ISCC 活动。

## 验证复用与包含范围

本次复用已有定向验证，不重跑全量 Rust/C#、HZ5 矩阵或各 OPT 原生矩阵；复用来源逐项列于 `verification-reuse.json` 和安装器 `evidence.json`。打包期间的源码一致性不等于重新执行全部历史验收。

新增归档能力采用 [工作流优化任务](../11-02-archive-workflow-optimization/task.json) 的当前限定结论：OPT-02～07 已有自动化/自有原生接受，包括新建/添加输入与结果、按需保留拖出、格式计划复用、有界二进制传输、批次进度/密码选择、Find、定位、父目录导航和安全 Retry。O5-AC01/02 与 O7-AC01/02/03 的关闭以各自报告为准；最新 [OPT-07 定向独立验收](../11-02-archive-workflow-optimization/opt07-ac01-ac03-independent-acceptance.md) 的 61 项通过为既有结果，不是本次新增执行。

此前下载批量/同名冲突、输入钩子隔离、索引器正确性/持久化、IPC/浏览器交接以及 HZ5 的原记录继续保留。没有因打包扩大源释放、回收、重试或格式准入。

## 已知限制与待验项

- 只读观察时现有服务为 Running / LocalSystem，受保护 Desktop、PlatformHost、Indexer 及批准哈希均不匹配 Release45。须另行授权匹配原子包部署，再执行 `tests/performance/indexer-idle-cpu.ps1 -RestartService -AssertRegression` 与 `tests/performance/indexer-service-smoke.ps1 -Query 'NxTools.Indexer.exe' -Limit 20 -TimeoutMs 10000`；本次未运行这些安装态门禁。
- 归档仍 **Phase E — Format expansion and ZIP editing**，整体 in_progress。G-00、OPT-00/01 整体、完整 G-04、G-18 实机、真实 G-28 系统选择器、G-29 原对象回收及 Create-through-post-Test 源寿命前置仍开放。本次只关闭打包产物核验，不覆盖这些交互或业务门禁。
- 源回收生产能力未放开；HZ5 仍默认关闭，仅普通单卷未加密 Stored ZIP 完整文件、同卷本地 NTFS，保持独立不可逆确认与继续剩余解压边界，不承诺完整原归档重建或断电认证。
- 历史定向验证中排除的日期相关下载日历失败不计作通过；未重跑的历史失败、性能取舍与冻结证据保持原归属。无全格式 Release 性能、物理 Explorer/浏览器/输入、DPI、辅助技术或用户视觉接受声明。

原 Release44 记录和包备份保留。检查点仅 prepend，原 295,385 字节保持精确 suffix。
