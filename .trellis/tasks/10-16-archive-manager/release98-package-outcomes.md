# 0.7.2 Release98 — 待服务验收

2026-11-20。候选包生成与文件核验完成，打包进程退出0、包装器`Completed`。使用冻结源码和独立空输出目录冷构建，不复用工作区编译缓存；Bootstrap最低兼容门槛保持82，仅交付正常安装器，无外层ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.2 / 0.7.2 |
| 发布序号 | 98 |
| Release ID | `44027b71-475c-4047-b63f-65c8fc16baed` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.2.exe` |
| 大小 | 30,157,564 字节 / 28.76 MiB |
| SHA256 | `739A204CF4C519784111745B935B2CE51B3A0E6DA6CB08DF1119B82086FF7A49` |
| Manifest SHA256 | `8196dcca95886bec22fc74dba19a5850e9e30374a5ea38b79c3aeca647e8b50e` |
| 发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 内部签名 | RSA3072-PSS/SHA256，独立核验通过 |
| 签名成员 / 精确文件 | 108 / 110 |
| Windows数字文件版本 | Desktop / PlatformHost / ShellSelectionHelper / 安装器均0.7.2.0 |
| Windows Authenticode | `NotSigned` |
| 发布验收 | 索引服务门禁待验，`release_accepted=false` |

## 本轮验证

- 冻结4,806输入复制前后内容一致，构建期间0漂移。未使用工作区target或历史发布缓存。
- 本轮安装器runtime16项及BrowserHost替换/回滚5项通过。
- 独立核验内部签名、公钥、108成员大小/哈希、精确110文件、95扩展源文件、5个Rust输出、实际版本、迁移Host绑定及兼容门槛82。
- 安装器哈希与`status.json`、旁侧`.sha256`一致，对应源码ZIP完整可读；交付后及清理后安装器与108成员读回一致。
- `-SkipVerification`复用已有产品测试，不重复全量Rust/C#测试。既有验证链和产品限制见[Release97](release97-package-outcomes.md)。新增索引优化的173通过/5忽略、严格Indexer测试Clippy及隔离Debug/Release持久化、重开、发布/搜索结果见[Release初建审计](../../../docs/audits/2026-11-19-initial-index-release.md)，不计入本轮新执行。

相对Release97的快照包含索引初建路径构造、持久化行序和批读取等优化，路径清单在`source-capture/changes-from-release97.json`。参考为已记录的Listary历史托管扫描器和不透明原生边界，非Listary现行原生引擎源码。

## 索引器发布门禁

本轮未安装、部署或重启服务。正式放行仍需匹配包的LocalSystem重启/base+delta恢复、`indexer-idle-cpu.ps1 -RestartService -AssertRegression`空闲CPU/IO回归及真实服务搜索/批准与运行哈希核对。隔离Release基准、源码测试和安装器构建均不替代该门禁。待验状态记录在`release-acceptance.json`。

## 清理结果

交付核验后清除工作区Rust缓存、本轮独立构建树与Release97旧包备份，共4目录、6,334,620,171字节（5.90 GiB），失败0项；清单路径均不存在，遗留独立编译输出检查0项。

当前安装器、散装包、对应源码/许可证、轻量证据、密钥/序号账本及受信恢复测试固定样本保留。源码、截图、全局依赖缓存和其他项目不清理；旧包备份和构建快照不再保留。

持久证据：`.diagnostics/packaging/20261119-231820-947b1f16-published98/`。原始`status.json`记录打包完成，`published-verification.json`记录交付身份和待验状态；`source-capture/`保留冻结输入，清理清单/结果与`cleanup-summary.json`记录实际删除范围。

## 已知限制

最新既有Desktop全量2,005通过、31失败、1忽略；Core/Desktop严格测试Clippy、格式差异及UI token限制仍见既有产品记录。本轮未修复或重跑这些集合。

真实覆盖安装、UAC、服务/注册表操作、物理UI及对应源码独立离线重构建未执行。归档仍 **Phase E — Format expansion and ZIP editing / in_progress**，B4暂停；未提交/推送。
