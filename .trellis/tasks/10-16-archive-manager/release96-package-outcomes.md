# 0.7.2 Release96

2026-11-19。正式重新打包完成，原打包进程退出0，包装器状态`Completed`。使用冻结源码、独立缓存和输出目录；Bootstrap最低兼容门槛保持82。仅交付正常安装器，未生成外层ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.2 / 0.7.2 |
| 发布序号 | 96 |
| Release ID | `aedd86ab-1cb4-4fdb-bc95-7c86bab396ab` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.2.exe` |
| 大小 | 30,076,990 字节 / 28.68 MiB |
| SHA256 | `F4888C86817F4CFCFAAE6C993CF925892694AC66F8B9B74D60D994AF92BAC886` |
| Manifest SHA256 | `165933af2d536fec84f3a4db49effa0f236ee918de60425512f62412c563d964` |
| 发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 内部发布签名 | RSA3072-PSS/SHA256，独立验证通过 |
| 签名成员 / 精确散装文件 | 107 / 109，全部匹配 |
| Bootstrap最低兼容门槛 | 82 |
| Windows数字文件版本 | Desktop / PlatformHost / ShellSelectionHelper / 安装器均0.7.2.0 |
| Windows Authenticode | `NotSigned` |

## 本轮验证

- 冻结4,782个源码输入，复制前后内容一致，构建期间0漂移；从本会话Release95缓存独立复制，未使用工作区target作为正式发布输出。
- 安装器runtime16项、BrowserHost替换/回滚5项全部通过，记录来自本轮`package.log`。
- 独立核验签名、公钥、107成员大小/哈希、精确109文件、94个扩展源文件、5个Rust构建输出、实际Windows版本、迁移Host绑定和兼容门槛82。
- 安装器哈希与本轮`status.json`及旁侧`.sha256`一致；归档对应源码ZIP完整可读。发布后及缓存清理后均读回当前安装器和107个发布成员。
- `-SkipVerification`复用既有产品验证，未重复全量Rust/C#测试。既有验证链见[Release95](release95-package-outcomes.md)；最新阅读器24项回归和大纲指针Debug交互记录见[大纲指针结果](../11-18-markdown-reader/outline-pointer-outcomes.md)，不计入本轮新执行。

相对Release95的快照包含Markdown阅读器大纲指针边界及其回归；具体路径清单保存在`source-capture/changes-from-release95.json`。

## 清理结果

按用户要求，在新包发布与核验成功后清除旧包和构建缓存：

- 工作区`target`、C# `bin/obj`、旧验证包与历史安装器。
- 本会话发布快照、独立构建缓存、遗留打包临时树及其他已确认的编译输出。
- 历史打包的`previous-package`和`previous-installer`备份，包括本轮交付前的Release95备份。

共清除325个目录/文件，按已删除文件逻辑长度计124,621,396,087字节（116.06 GiB），删除失败0项。清单中的路径均已不存在，独立编译输出残留检查为0项。源码、界面截图、全局Cargo/NuGet依赖缓存及其他项目未清理。受信恢复测试固定使用的Release74样本保留在原路径。

当前安装器、`artifacts/win-x64/`、许可证与对应源码材料、发布密钥和序号账本保留；账本序号96。已完成的构建快照不再保留，下次构建需要重新生成缓存。冻结输入清单、原始日志、签名清单及清理记录持续保留。

持久证据：`.diagnostics/packaging/20261119-134437-f002075b-published96/`。`published-verification.json`记录交付身份，`source-capture/`保留冻结输入及缓存复制记录；`cleanup-summary.json`、两份清理清单及结果记录实际清理范围和数量。

## 已知限制

既有桌面全量记录为1,986通过、26失败、1忽略；失败分布于Archive、Downloads、Extensions和主窗口测试。严格Desktop Clippy的既有诊断仍未清零。本轮未修复或重跑这些集合。

真实覆盖安装、UAC、服务/注册表操作和物理UI验收未执行；对应源码材料的独立离线重构建未执行。归档保持 **Phase E — Format expansion and ZIP editing / in_progress**，B4保持暂停；未提交或推送。
