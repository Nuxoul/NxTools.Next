# 0.7.1 Release92

2026-11-18。正式重新打包完成，原打包进程退出0，包装器状态`Completed`。使用冻结源码、独立缓存和输出目录；Bootstrap最低兼容门槛保持82。仅交付正常安装器，未生成外层ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 92 |
| Release ID | `5716cd3c-bbc7-44e4-86d3-bb3d575096c3` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 30,007,863 字节 / 28.62 MiB |
| SHA256 | `C84471B62819268BF242C843BF82E13C0BE4D50C45470D2D39E9F7A94722CECC` |
| Manifest SHA256 | `a79223332717b84c1a575bbf77b1814bd1fdd473edbf536608942df98b46b727` |
| 发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 内部发布签名 | RSA3072-PSS/SHA256，独立验证通过 |
| 签名成员 / 精确散装文件 | 107 / 109，全部匹配 |
| Bootstrap最低兼容门槛 | 82 |
| Windows数字文件版本 | Desktop / PlatformHost / ShellSelectionHelper / 安装器均0.7.1.0 |
| Windows Authenticode | `NotSigned` |

## 本轮验证

- 冻结4,762个源码输入，复制前后内容一致，构建期间0漂移；缓存从本会话独立缓存复制，未使用工作区target作为正式发布输出。
- 安装器runtime16项、BrowserHost替换/回滚5项全部通过，记录来自本轮`package.log`。
- 独立核验签名、公钥、107成员大小/哈希、精确109文件、94个扩展源文件、5个Rust构建输出、实际Windows版本、迁移Host绑定和兼容门槛82。
- 安装器哈希与本轮`status.json`及旁侧`.sha256`一致；归档对应源码ZIP完整可读。发布到默认路径后签名成员和安装器读回一致。
- `-SkipVerification`复用既有产品验证；未重复全量Rust/C#测试。既有定向结果见[Release91](release91-package-outcomes.md)，不计入本轮新执行。

持久证据：`.diagnostics/packaging/20261118-133725-429ef06b-published92/`。原始`status.json`记录隔离构建路径，`published-verification.json`记录最终交付路径，`source-capture/`保留冻结输入与缓存证据。前一Release91保留于`previous-package/`和`previous-installer/`。

## 已知限制

真实覆盖安装、UAC、服务/注册表操作和物理UI验收未执行；对应源码材料的独立离线重构建未执行。归档保持 **Phase E — Format expansion and ZIP editing / in_progress**，B4保持暂停；未提交或推送。
