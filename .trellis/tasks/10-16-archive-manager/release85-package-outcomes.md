2026-11-16。Release85 重新生成 0.7.1 正式安装器，沿用历史公钥迁移修复和新包准入边界。真实安装、UAC、服务、注册表及受保护目录操作未执行。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 85 |
| Release ID | `ea7d1241-f928-44aa-a6dd-eddf1d68c306` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 29,212,911 字节 / 27.85 MiB |
| SHA256 | `37D9DEC1134A0256518C5BBEB471526ECC93091E2717EF899A8E4B562CD90E5F` |
| 新发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 签名成员 / 散装文件集合 | 107 / 109，成员哈希与大小全部匹配 |
| 内部发布签名 | 新发布身份，RSA3072-PSS/SHA256，独立验证通过 |
| Windows Authenticode | `NotSigned` |

## 验证与限制

- Inno Setup 已生成序列85安装器。
- 107 个成员全部匹配清单；RSA-PSS 签名验证通过；安装器哈希已写入旁侧 `.sha256`。
- 外层 `package-local.ps1` 在子进程结束后未返回最终退出码；已停止本轮精确包装器进程并保存独立收尾证据，未据此宣称包装器正常退出。
- 未生成外层 ZIP；只交付正常安装器。
- Release85 尚未在用户机器上实际安装验证。若仍失败，请提供新的 `preparation.jsonl`、`bootstrap-detection.jsonl` 和 `setup.log`。

持久证据：`.diagnostics/packaging/20261116-105132-71696d99/`。
