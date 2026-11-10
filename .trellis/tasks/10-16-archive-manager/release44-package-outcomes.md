# 正式 Release44 安装包

2026-11-02。当前工作树通过原 `scripts/package-local.ps1 -SkipVerification` 完成实际 Release 构建、原发布身份签名和正常 Inno 安装器生成。复用已记录的定向验证，不重跑全量 Rust/C# 或 HZ5 进程矩阵。打包产物核验完成，索引器安装态发布验收仍待完成。

## 产物与核验

| 项目 | 结果 |
| --- | --- |
| 版本 / 发布序号 | 0.7.0 / 44 |
| Release ID | `32487b85-cff6-44ea-a340-732b43f3ddd8` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.0.exe` |
| 大小 | 27,284,125 字节 / 26.02 MiB |
| SHA256 | `6D40393212C8F77014549E5C69323E60727B5FFA082F0F594B9DAAA6D45CABF5` |
| 内部发布签名 | 现有公开信任锚，RSA3072-PSS/SHA256 独立验证通过 |
| Manifest SHA256 | `86ab3f3e379e645bc824f6e8926267abc160a54dd015fec16a3f85f21a20e9c5` |
| 签名成员 / 精确文件集合 | 78 / 80，全部大小、内容和集合一致 |
| 实际构建输出 | 10 项与包内文件一致 |
| 浏览器扩展源文件 | 65 项与包内文件一致 |
| 归档批准 / helper Desktop 绑定 | 5 项批准及绑定通过 |
| 安装器相关隔离场景 | 11 通过 |
| BrowserHost 替换隔离场景 | 5 通过 |
| 源码输入 | 1,189 项，打包期间无新增、删除或内容变化 |
| Windows Authenticode | `NotSigned`，与内部发布签名分开记录 |

安装准备 helper 保持 `4CDFEAC77D33715CD6800AD27CDCC3D60F58E05198500FEB8CECAAD36A76FC1A`；其先前运行中升级证据见 [Release41 记录](installer41-running-upgrade-outcomes.md)，不作为本包已安装的证明。统一入口 state=Completed、package_exit_code=0；构建后无 Cargo/rustc/clippy/ISCC 遗留进程。

持久日志、旧包备份与核验：`.diagnostics/packaging/20261102-101400-79f5589e/`。`independent-verification.json` 记录签名、集合和身份图；`verification-reuse.json` 记录验证来源；`installed-tuple-observation.json` 为只读安装态比对。控制器及源码基线：`D:\.pi-desktop\data\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\formal-repack-20261102-101358`。

## 验证复用与包含范围

复用来源：

- [下载批量与目标冲突](../10-31-download-backpressure-conflict/outcomes.md)。
- [输入隔离与扩展手势](../11-01-platformhost-input-and-extension-trust/outcomes.md)及其 [follow-up](../11-01-platformhost-input-and-extension-trust/followup-outcomes.md)。
- [索引器正确性、会话与缓存](../11-01-indexer-journal-correctness/outcomes.md)。
- [IPC 与浏览器交接生命周期](../11-02-ipc-browser-chain/outcomes.md)。
- [HZ5 独立限定验收](hz5-independent-acceptance.md)，其默认关闭与格式/卷限定不变。

这些是各任务先前执行的定向结果，不是本次重跑或整包全量验收；本次源码基线证明打包期间一致性，不把历史源码状态一概认定为当前全量通过。当前扩展随包包含，Native Messaging 注册和实际浏览器配置未被修改。

## 已知限制与待验项

- 未安装、启动产品、部署受保护副本或重启服务。只读检查时原服务为 Running / LocalSystem；受保护 Desktop、PlatformHost、Indexer 及对应批准哈希均不匹配 Release44。
- 索引器安装态发布门禁未执行。必须在另行授权、匹配原子包部署后运行 `tests/performance/indexer-idle-cpu.ps1 -RestartService -AssertRegression`，再运行 `tests/performance/indexer-service-smoke.ps1 -Query 'NxTools.Indexer.exe' -Limit 20 -TimeoutMs 10000`。旧服务的 Ready/搜索或本包编译成功不替代 CPU/I/O、重启后 base+delta 和真实服务搜索验收。
- 下载既有日期相关日历夹具失败在定向验证中排除，不计作通过；归档既有格式检查差异保留。
- 压缩工作流优化仍处计划阶段，本包不代表该优化或整体 UI/UX 修缮已完成。物理键盘/Explorer、真实浏览器、DPI、辅助技术和用户视觉验收保持各任务记录的未验范围。
- HZ5 仍只支持普通单卷未加密 Stored ZIP 完整文件、同卷本地 NTFS；独立不可逆确认、持久日志先于源数据释放及新进程继续剩余解压的语义不变，不承诺原归档重建或断电认证。

归档规范阶段保持 **Phase E — Format expansion and ZIP editing**。历史 Release43 记录和原包备份保留，未提交或推送。
