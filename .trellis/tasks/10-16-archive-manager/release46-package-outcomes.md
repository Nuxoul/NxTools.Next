# 正式 Release46 安装包

2026-11-05。当前源码通过原 `scripts/package-local.ps1 -SkipVerification` 重新构建、以原发布身份签名并生成正常 Inno 安装器。原 package.ps1 子进程退出码 0，独立产物核验通过。未安装、启动产品、部署保护副本、重启服务或提交推送。

## 产物与核验

| 项目 | 结果 |
| --- | --- |
| 版本 / 发布序号 | 0.7.0 / 46 |
| Release ID | `5419a572-f9ec-4ea5-8c84-d6c15810c4d4` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.0.exe` |
| 大小 | 27,493,857 字节 / 26.22 MiB |
| SHA256 | `97262BD8CFCC397B277BA172B7B06A51B90E90DF2D84759A7656B587FFBB5B5B` |
| 内部签名 | 现有公开信任锚，RSA3072-PSS/SHA256 独立核验通过 |
| Manifest SHA256 | `3a0b6619bbfeb0b59dbcfdf285e80dc4b552d60335b4bd30d4f5ed868df7ed77` |
| 签名成员 / 精确文件集合 | 78 / 80，全部一致 |
| 实际构建输出 / 扩展源文件 | 10 / 65，全部与包内一致 |
| 归档批准 / helper Desktop 身份 | 5 项批准及绑定一致 |
| 安装器 / BrowserHost 替换隔离场景 | 11 / 5 通过 |
| 源码基线 | 1,264 项，打包期间无新增、删除或内容变化 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

安装准备 helper 保持 `4CDFEAC77D33715CD6800AD27CDCC3D60F58E05198500FEB8CECAAD36A76FC1A`。历史 Release45 安装包备份保留，本次使用新的序号、Release ID 和安装器校验值。

持久证据：`.diagnostics/packaging/20261105-050827-428445a5/`，包括原日志和旧包备份、子进程退出事实、`independent-verification.json` 与 `verification-reuse.json`。控制器及源码基线：`D:\.pi-desktop\data\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\formal-repack-20261105-050826`。实际构建环境为单任务、非增量、codegen-units256，本次 C# 使用 `UseSharedCompilation=false`。构建后无 Cargo/rustc/clippy/ISCC 活动。

## 包含范围与验证复用

- [速启 Listary 对齐](../11-04-quick-launch-listary-alignment/outcomes.md)：类别排序、历史/别名合并、路径中间省略、结果层级及内容尺寸窗口；其最终定向 Quick Launch 验证覆盖此前平滑刷新与默认选择。
- [归档拖出限定验收](../11-02-archive-workflow-optimization/release45-independent-acceptance.md)：标题、当前/整体进度、简洁状态及 UTF-16 Root 名称。原报告的输出清理缺陷由 [AC01 独立接受](../11-02-archive-workflow-optimization/release45-ac01-independent-acceptance.md) 关闭，原生 7z 低预算分类由 [后续独立接受](../11-02-archive-workflow-optimization/release45-sevenz-limit-independent-acceptance.md) 关闭，原十个原生保留场景完整通过。
- 原 OPT-02～07、下载、输入、索引器、IPC/浏览器和 HZ5 的限定证据保留。

复用来源逐项列于 `verification-reuse.json` 和安装器 `evidence.json`。本次未重跑全量 Rust/C#、HZ5 或各 OPT 原生矩阵；源码基线证明打包期间一致性，不表示重新执行全部历史验收。

## 已知限制与待验项

- 索引器安装态门禁未执行。须另行授权匹配原子包部署后，运行 `tests/performance/indexer-idle-cpu.ps1 -RestartService -AssertRegression`，再运行 `tests/performance/indexer-service-smoke.ps1 -Query 'NxTools.Indexer.exe' -Limit 20 -TimeoutMs 10000`。本包编译、签名不替代实际服务 CPU/I/O、base+delta 恢复或搜索验收。
- 速启记录的 Finder 95 / Quick Launch 185 定向通过保持原归属；此前完整 Desktop 为 1,718 通过、10 失败，其中 7 项缺原生 wire 夹具、2 项 Retry 计数不符、1 项下载菜单可用性断言失败。未重跑、未计作通过；原日期相关下载日历等历史结果同样不被覆盖。
- DOpus 可见暂存、物理 Explorer/输入/混合 DPI、用户视觉、Bandizip 实际分支及 Release 性能仍未接受。后续直接 Add/unsafe_path/通知请求不属于本次复用的拖出验收。
- 归档仍 **Phase E — Format expansion and ZIP editing / in_progress**。G-00、OPT-00/01 整体、完整 G-04、真实 G-18/G-28 和 G-29 前置保持原状态。源回收生产能力未放开；HZ5 默认关闭和独立不可逆确认、Stored ZIP/同卷 NTFS/继续剩余解压边界不变，不承诺原归档重建或断电认证。

检查点仅 prepend，原 296,988 字节保持精确 suffix。
