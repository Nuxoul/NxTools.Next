# 0.7.1 Release90

2026-11-17。正式重新打包完成，原打包进程退出0，包装器状态`Completed`。使用冻结源码、独立缓存和输出目录；Bootstrap最低兼容门槛保持82。仅交付正常安装器，未生成外层ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 90 |
| Release ID | `433e191a-e1a1-4a08-ac44-85e53bd54716` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 29,232,101 字节 / 27.88 MiB |
| SHA256 | `9F55801372C687226D89E8D176E037316CA0C61496216E6624DB49F0ED0E33CE` |
| Manifest SHA256 | `338b30519fb4181cdaf2e8840eaef33d968977463e086ee90deae8276676b861` |
| 发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 内部发布签名 | RSA3072-PSS/SHA256，独立验证通过 |
| 签名成员 / 精确散装文件 | 107 / 109，全部匹配 |
| Bootstrap最低兼容门槛 | 82 |
| Windows数字文件版本 | Desktop / PlatformHost / ShellSelectionHelper / 安装器均0.7.1.0 |
| Windows Authenticode | `NotSigned` |

## 本轮验证

- 冻结4,724个源码输入，复制前后内容一致，构建期间0漂移；缓存从本会话独立缓存复制，未使用工作区target作为正式发布输出。
- 安装器runtime16项、BrowserHost替换/回滚5项全部通过，记录来自本轮`package.log`。
- 独立核验签名、公钥、107成员大小/哈希、精确109文件、94个扩展源文件、5个Rust构建输出、实际Windows版本、迁移Host绑定和兼容门槛82。
- 安装器哈希与本轮`status.json`及旁侧`.sha256`一致；归档对应源码ZIP完整可读。发布到默认路径后签名成员和安装器读回一致。
- `-SkipVerification`复用既有产品验证；未重复全量Rust/C#测试。既有定向结果见[Release89](release89-package-outcomes.md)，不计入本轮新执行。

持久证据：`.diagnostics/packaging/20261117-224413-23cb03b1-published90/`。原始`status.json`记录隔离构建路径，`published-verification.json`记录最终交付路径，`source-capture/`保留冻结输入与缓存证据。前一Release89保留于`previous-package/`和`previous-installer/`。

## 已知限制

真实覆盖安装、UAC、服务/注册表操作和物理UI验收未执行；对应源码材料的独立离线重构建未执行。归档保持 **Phase E — Format expansion and ZIP editing / in_progress**，B4保持暂停；未提交或推送。
