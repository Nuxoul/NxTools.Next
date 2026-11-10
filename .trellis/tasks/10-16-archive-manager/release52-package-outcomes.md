# 正式 Release52 安装包

2026-11-06。当前源码通过原 `scripts/package-local.ps1 -SkipVerification` 重新完成实际 Release 编译、原发布身份签名和正常安装器生成。原 package.ps1 子进程退出码为 0，独立产物核验通过。未安装、启动产品、部署保护副本、重启服务或提交推送。

## 产物核验

| 项目 | 结果 |
| --- | --- |
| 版本 / 发布序号 | 0.7.0 / 52 |
| Release ID | `4682399f-7a03-40b4-804a-07ab958d3d5b` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.0.exe` |
| 大小 | 27,528,547 字节 / 26.25 MiB |
| SHA256 | `105C8C1984027121EA31C59D13066E3865B7A67A27DF444DD0EA5128C573212F` |
| 内部签名 | 原公开信任锚，RSA3072-PSS/SHA256 独立核验通过 |
| Manifest SHA256 | `a95176c8b21762159868250e82f4350c5fd5a8b41ee69fd8ae01800cfaea2b1a` |
| 签名成员 / 精确文件集合 | 78 / 80，全部一致 |
| 实际构建输出 / 扩展源文件 | 10 / 65，与包内一致 |
| 归档批准 / helper Desktop 绑定 | 5 项批准及绑定一致 |
| 安装器 / BrowserHost 替换隔离场景 | 11 / 5 通过 |
| 源码基线 | 1,273 项，打包期间无变化 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

原安装准备 helper 保持不变。持久证据 `.diagnostics/packaging/20261106-003343-2f3cc7e3/` 保留原日志、旧包备份、原进程退出事实、`independent-verification.json` 和 `verification-reuse.json`。控制器与源码基线位于 `D:\.pi-desktop\data\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\formal-repack-20261106-003341`。构建采用 jobs1、incremental0、codegen-units256 和 C# UseSharedCompilation=false；构建后无 Cargo/rustc/clippy/ISCC 活动。

## 包含范围与验证复用

当前归档 `progress.rs` 和 `progress_feedback_tests.rs` 的最新改动纳入本包。此前速启搜索头/结果窗口、下载精简输入、Root 拖出与读取优化、Incoming Add/统一通知和其他限定修复保持。历史来源见 [Release51记录](release51-package-outcomes.md) 及本次 `verification-reuse.json`、安装器 `evidence.json`。

本次没有重跑全量 Rust/C#、归档矩阵或定向交互测试。最新进度界面改动经过实际 Release 编译，未以旧验收记录认定其新行为已独立接受。源码基线证明打包期间一致性，不等于全量行为验收。

## 已知限制与待验项

- 索引器安装态门禁未执行。须另行授权部署匹配原子包后运行 `tests/performance/indexer-idle-cpu.ps1 -RestartService -AssertRegression`，再运行 `tests/performance/indexer-service-smoke.ps1 -Query 'NxTools.Indexer.exe' -Limit 20 -TimeoutMs 10000`。
- 历史下载菜单断言失败、预览隔离聚合门禁及显式原生夹具相关未运行项保持各任务记录，不能称全量通过。
- 物理 Explorer/DOpus、IME/混合DPI、用户视觉、实际安装和整体性能不由安装器生成或受控测试替代。归档仍 **Phase E — Format expansion and ZIP editing / in_progress**，原开放门禁不因打包关闭。
- 源回收和 HZ5 原默认关闭、限定格式/卷、独立不可逆确认与继续剩余解压边界不扩大。

历史包与证据保留。检查点仅 prepend，原 302,781 字节保持精确 suffix。
