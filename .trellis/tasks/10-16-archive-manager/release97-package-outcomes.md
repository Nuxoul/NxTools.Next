# 0.7.2 Release97 — 待服务验收

2026-11-19。候选安装包生成与文件核验完成，打包进程退出0，包装器状态`Completed`。本轮在冻结源码和独立空输出目录中冷构建，不复用工作区编译缓存。Bootstrap最低兼容门槛保持82；仅交付正常安装器，无外层ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.2 / 0.7.2 |
| 发布序号 | 97 |
| Release ID | `e91e3d3d-8fac-436d-be89-2ab167d3fa68` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.2.exe` |
| 大小 | 30,156,301 字节 / 28.76 MiB |
| SHA256 | `E18843B8F8C4E56673F74889E177BFA8D33A5A2928EEBD2586CA4D994578A24C` |
| Manifest SHA256 | `0e0bcc9a2a56a4aeae839873d255bfa78dcf0078777d2f4408ee4d39ba5a6e45` |
| 发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 内部发布签名 | RSA3072-PSS/SHA256，独立核验通过 |
| 签名成员 / 精确散装文件 | 108 / 110，全部匹配 |
| Windows数字文件版本 | Desktop / PlatformHost / ShellSelectionHelper / 安装器均0.7.2.0 |
| Windows Authenticode | `NotSigned` |
| 发布验收 | 索引服务发布门禁待验，`release_accepted=false` |

## 本轮验证

- 4,803个冻结源码输入复制前后内容一致，构建期间0漂移。未使用工作区target或历史发布缓存。
- 安装器runtime16项、BrowserHost替换/回滚5项全部通过，来自本轮`package.log`。
- 独立核验内部签名、公钥、108成员大小/哈希、精确110文件、95个扩展源文件、5个Rust输出、实际版本、迁移Host绑定及兼容门槛82。
- 安装器哈希与`status.json`和旁侧`.sha256`一致；对应源码ZIP完整可读。交付后及清理后安装器与108成员均读回一致。
- `-SkipVerification`复用已有产品结果，未重复全量Rust/C#测试。此前验证链见[Release96](release96-package-outcomes.md)。新增数据目录/外观、Firefox及图标已有Core103项、主题2项、Desktop定向65项、扩展Node363项和2项C#导出签名结果，见[全局数据与外观结果](../11-18-global-data-appearance/outcomes.md)；阅读器通知28项结果见[通知结果](../11-18-markdown-reader/settings-notification-outcomes.md)。这些不是本轮新执行。

相对Release96的快照包含全局数据目录与字体/主题、Firefox响应交接、阅读器通知及MFT初建优化等改动；路径清单在`source-capture/changes-from-release96.json`。

## 索引器发布门禁

[MFT初建审计](../../../docs/audits/2026-11-19-initial-index-build.md)记录批读取8项、非真实卷Indexer168通过/4忽略、授权C盘独立扫描及Indexer严格Clippy/工作区检查。其参考为已记录的Listary历史托管扫描器和不透明原生边界，不是Listary现行原生引擎源码。

本轮未部署或重启本机服务。正式放行仍需匹配包的LocalSystem服务重启与持久base+delta恢复、`indexer-idle-cpu.ps1 -RestartService -AssertRegression`的空闲CPU/IO回归、真实服务搜索及批准/运行哈希核对。源码测试、独立管理员扫描、`Ready`或安装器构建成功均不替代该门禁。状态记录在`release-acceptance.json`。

## 清理结果

候选包交付并核验后，清理工作区`target`、C# `bin/obj`、本轮独立构建树及Release96旧包备份，共10个目录，18,590,028,508字节（17.31 GiB），失败0项；已删除路径均不存在，遗留独立编译输出检查为0项。

当前安装器、散装包、许可证/对应源码、轻量发布证据、密钥/序号账本及受信恢复测试的Release74固定样本保留。源码、截图、其他项目和全局依赖缓存未清理。构建快照及旧包备份不再保留；下次构建需要重新生成缓存。

持久证据：`.diagnostics/packaging/20261119-181524-da3a83df-published97/`。`status.json`记录打包完成，`published-verification.json`记录交付身份与待验状态；`source-capture/`保留冻结输入记录，`cleanup-summary.json`及清理清单/结果记录删除范围。

## 已知限制

最新既有Desktop全量为2,005通过、31失败、1忽略；严格Core/Desktop测试Clippy、9文件格式差异及5项UI token问题见全局数据与外观结果，本轮未修复或重跑。Firefox物理交互、数据迁移/字体DPI等安装验收仍待执行。

真实覆盖安装、UAC、服务/注册表操作、物理UI及对应源码独立离线重构建未执行。归档仍 **Phase E — Format expansion and ZIP editing / in_progress**，B4暂停；未提交或推送。
