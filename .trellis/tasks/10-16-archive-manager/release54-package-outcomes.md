# 正式 Release54 分发包

2026-11-06。当前工作树通过原 `scripts/package-local.ps1 -SkipVerification` 完成实际 Release 编译、原发布身份签名、正常 Inno 安装器及完整分发 ZIP 生成。原 package.ps1 与统一入口退出码均为 0，独立产物与分发材料核验通过。未安装、启动产品、部署保护副本、重启服务或提交推送。

## 产物核验

| 项目 | 结果 |
| --- | --- |
| 版本 / 发布序号 | 0.7.0 / 54 |
| Release ID | `7d413365-e409-49b1-9b96-1677a91255b4` |
| 完整分发包 | `artifacts/installer/NxTools.Next-release-0.7.0.zip` |
| 分发包大小 | 62,505,898 字节 / 59.61 MiB |
| 分发包 SHA256 | `08B0E0A3AE911D1710DED8894C96ED2B2B9BD50BDCB97F7239C63851ED94E25D` |
| 正常安装器 | `artifacts/installer/NxTools.Next-setup-0.7.0.exe` |
| 安装器大小 | 27,949,457 字节 / 26.65 MiB |
| 安装器 SHA256 | `39962577C0B142F3472EDA8989529E86935DF155FF986D259C320C780B6A25F7` |
| 内部签名 | 原公开信任锚，RSA3072-PSS/SHA256 独立核验通过 |
| Manifest SHA256 | `a8aa6d6dbd9b2ae65d1ae14a1873c8a18d1f86a976bcf3a4d0fd3c0cffabef86` |
| 签名成员 / 精确文件集合 | 78 / 80，全部一致 |
| 实际构建输出 / 扩展源文件 | 10 / 65，与包内一致 |
| 归档批准 / helper Desktop 绑定 | 5 项批准及绑定一致 |
| 安装器 / BrowserHost 替换隔离场景 | 11 / 5 通过 |
| 源码基线 | 1,295 项，打包期间无新增、删除或内容变化 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

安装准备 helper 保持原字节与身份。完整 ZIP 的 12 项文件逐项核对，内含的安装器与本次正式签封产物完全一致，材料与当前对应源码相符；两个主产物的相邻 SHA256 文件匹配。

持久证据：`.diagnostics/packaging/20261106-182923-3c160402/`，含原日志、旧包备份、状态、`independent-verification.json`、`verification-reuse.json` 和源码重构建/内容证据。控制器及源码基线：`D:\.pi-desktop\data\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\formal-repack-20261106-182922`。构建使用 jobs1、incremental0、codegen-units256、C# UseSharedCompilation=false。构建后无 Cargo/rustc/clippy/ISCC 活动。

## WIM 源码与许可证材料

按用户选择保留 WIM。完整分发 ZIP 中的 `third_party/archive-worker/` 提供 LGPL 许可证、当前 worker/IPC/Protocol 源码、147 个锁定依赖及离线重构建/重链接说明。使用完整 ZIP 分发，安装器仍执行原生产签名清单，不增加可信路径或绕过任何身份校验。

[源码分发验证](../../../docs/audits/2026-11-06-archive-source-distribution.md)：4 项分发回归、11 项原打包入口回归通过。102 个本项目源文件、三份 crate 清单与当前源码一致；WIM/压缩库原许可证内容一致，未包含 PEM。源码包已实际离线构建 Release worker，退出码 0、315.15 秒；最终分发的编译输入与该次重构建材料一致。完整 ZIP、源码 ZIP 的集合与内容均已独立核对。

## 产品验证复用

- [归档兼容性](../../../docs/audits/2026-11-06-archive-compatibility.md)：额外只读格式、连续 ZIP 分卷、7z 空成员布局和分层预览预算。
- [大数量与速启](../../../docs/audits/2026-11-06-large-count-and-launcher.md)：70,000 项 ZIP 的原 Job/native 路线及有界终端、取消/损坏/准入；速启原生产 provider/framed IPC 首批发布。
- 原速启布局/排序、下载输入、Incoming Add/统一通知、其他 OPT 与 HZ5 的限定记录保留。

本次复用这些已执行的定向结果，未重跑全量产品 Rust/C#、归档矩阵或大数量操作。分发相关测试与离线源码构建是本次执行；它们不替代 codec 行为、物理交互、性能或安装态服务验收。完整来源列于 `verification-reuse.json` 和安装器 `evidence.json`。

## 已知限制与待验项

- 索引器安装态 CPU/I/O、重启恢复及真实搜索门禁未执行。须另行授权部署匹配原子包后运行 `tests/performance/indexer-idle-cpu.ps1 -RestartService -AssertRegression`，再运行真实服务搜索 smoke。
- 归档样本中的 `wim-lzms.wim` 实为未压缩 WIM，不把它当作运行时 LZMS 接受。CPIO 已被生产者替换的不安全名称保持拒绝；不宣称全部格式变体或普遍性能通过。
- 原下载菜单文本断言失败、Desktop lint 阻塞及显式原生夹具未运行项保持各记录归属，未称全量绿色。物理 Explorer/DOpus、输入到首帧、IME/DPI、用户视觉及实际安装待验。
- [PlatformHost 内存记录](../../../docs/audits/2026-11-06-platformhost-memory.md)中的约 19GiB 私有提交原因仍未确认；受控十分钟观察未复现，不宣称问题已修复。
- 归档仍 **Phase E — Format expansion and ZIP editing / in_progress**。原开放门禁、源回收和 HZ5 默认关闭、格式/卷限制及不可逆确认不因打包扩大。

历史包及日志保留。检查点仅 prepend，原 304,061 字节保持精确 suffix。
