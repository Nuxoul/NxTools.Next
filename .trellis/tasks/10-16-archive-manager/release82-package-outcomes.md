2026-11-15。Release82 修正迁移期 active 快照验证所使用的历史公开锚，并保留新包准入的新公钥边界。正式 Release 输出已生成；真实安装、UAC、服务、注册表及受保护目录操作未执行。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 82 |
| Release ID | `ae9ce5a0-caf6-4687-9a6e-482ed45ad079` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 29,197,500 字节 / 27.84 MiB |
| SHA256 | `5E1FD0E03932F6DE322919938C13671291202CEE8DA19A06DE767A70F21239D7` |
| 新发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 历史迁移公钥 SHA256 | `03e979d5c6babda29aa217b7ce72268aab12870bda46a9072397fe52ba083b69` |
| 签名成员 / 散装文件集合 | 107 / 109，成员哈希与大小全部匹配 |
| 内部发布签名 | 新发布身份，RSA3072-PSS/SHA256，独立验证通过 |
| Windows Authenticode | `NotSigned` |

## 修复与验证

- `LegacyUpdateTrust` 现使用与历史 Release74 active 快照一致的旧 SPKI；旧快照 recovery 验证通过。
- 同一旧快照使用当前新公钥做普通 package admission 仍被拒绝；新包准入边界未放宽。
- 可信更新相关 Fixture 11 项通过，包括旧快照回滚、重启恢复、身份拒绝、Bootstrap 维护和 recovery-only 回归。
- 打包日志完成 Inno Setup 编译并生成序列82安装器；本轮外层 `package-local.ps1` 在子进程结束后未返回，未取得其最终退出码。未据此宣称包装器正常退出；安装器、清单、签名和全部成员已独立核验。
- 未生成外层 ZIP；只交付正常安装器。

持久证据：`.diagnostics/packaging/20261115-223053-4f2635d6/`，独立收尾记录见同目录 `independent-finalization.json`。

## 已知限制

Release82 尚未在用户机器上实际安装验证。请运行本次安装器后保留新的 correlation 日志；如果仍失败，提供新的 `preparation.jsonl`、`bootstrap-detection.jsonl` 和 `setup.log`。归档仍 **Phase E — Format expansion and ZIP editing / in_progress**，B4 保持暂停。
