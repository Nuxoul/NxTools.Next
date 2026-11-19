2026-11-16。Release84 重新生成 0.7.1 正式安装器，沿用历史公钥迁移修复和新包准入边界。真实安装、UAC、服务、注册表及受保护目录操作未执行。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 84 |
| Release ID | `5135ef5d-8d64-4b1e-9c6e-267cef7a68b0` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 29,207,323 字节 / 27.85 MiB |
| SHA256 | `3FD09B000A236A2D6CC55BC42D5F8A8D4A13533B8EC0A2BD998A7ED16C3DBDBB` |
| 新发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 签名成员 / 散装文件集合 | 107 / 109，成员哈希与大小全部匹配 |
| 内部发布签名 | 新发布身份，RSA3072-PSS/SHA256，独立验证通过 |
| Windows Authenticode | `NotSigned` |

## 验证与限制

- Inno Setup 已生成序列84安装器。
- 107 个成员全部匹配清单；RSA-PSS 签名验证通过；安装器哈希已写入旁侧 `.sha256`。
- 外层 `package-local.ps1` 在子进程结束后未返回最终退出码；已停止本轮精确包装器进程并保存独立收尾证据，未据此宣称包装器正常退出。
- 未生成外层 ZIP；只交付正常安装器。
- Release84 尚未在用户机器上实际安装验证。若仍失败，请提供新的 `preparation.jsonl`、`bootstrap-detection.jsonl` 和 `setup.log`。

持久证据：`.diagnostics/packaging/20261116-101209-0bba02f2/`。
