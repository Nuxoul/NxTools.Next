2026-11-16。Release86 重新生成 0.7.1 正式安装器，沿用历史公钥迁移修复和新包准入边界。真实安装、UAC、服务、注册表及受保护目录操作未执行。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 86 |
| Release ID | `3dbdcd40-1c83-4681-92a6-eafecc89fc15` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 29,234,852 字节 / 27.88 MiB |
| SHA256 | `94CB3B815927E0AA4F20D26226B47830D21B4D4E9B94E275581B12B838E70039` |
| 新发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 历史迁移公钥 SHA256 | `03e979d5c6babda29aa217b7ce72268aab12870bda46a9072397fe52ba083b69` |
| 签名成员 / 散装文件集合 | 107 / 109，成员哈希与大小全部匹配 |
| Windows 二进制版本 | 0.7.1.0 |
| 浏览器扩展版本 | 0.7.1 |
| 内部发布签名 | 新发布身份，RSA3072-PSS/SHA256，独立验证通过 |
| Windows Authenticode | `NotSigned` |

## 验证与限制

- Inno Setup 已生成序列86安装器。
- 精确109文件清单通过；107成员哈希/大小、RSA-PSS签名、安装器哈希、Windows版本、扩展版本和历史 SPKI 嵌入核验通过。
- 外层 `package-local.ps1` 在子进程结束后未返回最终退出码；已停止本轮精确包装器进程并保存独立收尾证据，未据此宣称包装器正常退出。
- 未生成外层 ZIP；只交付正常安装器。
- Release86 尚未在用户机器上实际安装验证。若仍失败，请提供新的 `preparation.jsonl`、`bootstrap-detection.jsonl` 和 `setup.log`。

持久证据：`.diagnostics/packaging/20261116-125756-a7c8a83c/`。
