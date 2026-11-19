# 0.7.1 Release89

2026-11-16。正式重新打包完成，原打包进程退出0，包装器状态`Completed`。保留Bootstrap最低兼容门槛82，普通发布序列递增不触发重复迁移授权。仅交付正常安装器，未生成外层ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 89 |
| Release ID | `ec78848e-2c30-44d2-b336-51c54305f68c` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 29,227,905 字节 / 27.87 MiB |
| SHA256 | `F003A40A72FAAFD1DD225366333EC266E9C93351413107D8609AFC6A4ACDA45F` |
| Manifest SHA256 | `45704070699833575ae24ab6224c47bcf4cb60cca3251826976aa32d7fa5b437` |
| 发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 内部发布签名 | RSA3072-PSS/SHA256，独立验证通过 |
| 签名成员 / 精确散装文件 | 107 / 109，全部匹配 |
| Bootstrap最低兼容门槛 | 82 |
| Windows数字文件版本 | Desktop / PlatformHost / ShellSelectionHelper / 安装器均0.7.1.0 |
| Windows Authenticode | `NotSigned` |

## 本轮验证

- 安装器runtime16项、BrowserHost替换/回滚5项全部通过，记录来自本轮`package.log`。
- 独立核对签名、公钥、107成员大小/哈希、精确109文件、扩展版本、实际Windows版本、迁移Host绑定和兼容门槛82。
- 安装器哈希与本轮`status.json`及旁侧`.sha256`一致，原进程退出0，无残留包装器。
- `-SkipVerification`复用既有产品验证；未重复全量Rust/C#测试。Bootstrap迁移92项等既有定向结果见[Release88](release88-package-outcomes.md)，不计入本轮新执行。

证据：`.diagnostics/packaging/20261116-180605-e82527d8/status.json`、`package.log`、`independent-verification.json`。前一Release88保留于同目录`previous-package/`和`previous-installer/`。

## 已知限制

真实覆盖安装、UAC、服务/注册表操作和物理UI验收未执行。归档保持 **Phase E — Format expansion and ZIP editing / in_progress**，B4保持暂停；未提交或推送。
