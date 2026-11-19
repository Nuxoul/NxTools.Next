# 0.7.2 Release95

2026-11-19。正式重新打包完成，原打包进程退出0，包装器状态`Completed`。使用冻结源码、独立缓存和输出目录；Bootstrap最低兼容门槛保持82。仅交付正常安装器，未生成外层ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.2 / 0.7.2 |
| 发布序号 | 95 |
| Release ID | `cf3329e0-fee7-482d-8283-becc4a0bcc7c` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.2.exe` |
| 大小 | 30,077,160 字节 / 28.68 MiB |
| SHA256 | `E3A1FB1C3E2A545A2C33C94D7FEFD101917311415E626F0CCDB59D55A731FECD` |
| Manifest SHA256 | `01a199dd3e83f83105a61f45c4b94bb1c98218d38b86ed24443c0344b6a07819` |
| 发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 内部发布签名 | RSA3072-PSS/SHA256，独立验证通过 |
| 签名成员 / 精确散装文件 | 107 / 109，全部匹配 |
| Bootstrap最低兼容门槛 | 82 |
| Windows数字文件版本 | Desktop / PlatformHost / ShellSelectionHelper / 安装器均0.7.2.0 |
| Windows Authenticode | `NotSigned` |

## 本轮验证

- 冻结4,779个源码输入，复制前后内容一致，构建期间0漂移；从本会话Release94缓存独立复制，未使用工作区target作为正式发布输出。
- 安装器runtime16项、BrowserHost替换/回滚5项全部通过，记录来自本轮`package.log`。
- 独立核验签名、公钥、107成员大小/哈希、精确109文件、94个扩展源文件、5个Rust构建输出、实际Windows版本、迁移Host绑定和兼容门槛82。
- 安装器哈希与本轮`status.json`及旁侧`.sha256`一致；归档对应源码ZIP完整可读。发布到默认路径后签名成员和安装器读回一致。
- `-SkipVerification`复用既有产品验证，未重复全量Rust/C#测试。既有验证链见[Release94](release94-package-outcomes.md)；阅读器代码/分栏20项及原生Debug交互记录见[分栏结果](../11-18-markdown-reader/winc-code-split-outcomes.md)，最新阅读器23项回归与字体/标题栏Debug交互记录见[字体及标题栏结果](../11-18-markdown-reader/header-typography-outcomes.md)。这些均不是本轮新执行。

相对Release94的快照包含Markdown阅读器字体、标题栏、代码/分栏、滚动与大纲等当前改动；具体路径清单保存在`source-capture/changes-from-release94.json`。

持久证据：`.diagnostics/packaging/20261119-115324-92a4ff1a-published95/`。原始`status.json`记录隔离构建路径，`published-verification.json`记录最终交付路径，`source-capture/`保留冻结输入与缓存证据。前一Release94保留于`previous-package/`和`previous-installer/`。

## 已知限制

既有桌面全量记录为1,986通过、26失败、1忽略；失败分布于Archive、Downloads、Extensions和主窗口测试。严格Desktop Clippy的既有诊断仍未清零。本轮未修复或重跑这些集合。

真实覆盖安装、UAC、服务/注册表操作和物理UI验收未执行；对应源码材料的独立离线重构建未执行。归档保持 **Phase E — Format expansion and ZIP editing / in_progress**，B4保持暂停；未提交或推送。
