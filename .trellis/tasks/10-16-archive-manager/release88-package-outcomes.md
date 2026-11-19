2026-11-16。Release88 重新生成 0.7.1 正式安装器，并包含稳定 Bootstrap 兼容门槛修复：普通发布序列递增不再重复要求 Bootstrap 维护；最低维护门槛为82。真实安装、UAC、服务、注册表及受保护目录操作未执行。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 88 |
| Release ID | `8c0f0c7b-bc29-417f-989b-1770a0997083` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 29,233,267 字节 / 27.88 MiB |
| SHA256 | `FE3C93441FE23B838ECDFC04C47AA50515FF0F3759496E3DA19B795941F35470` |
| Manifest SHA256 | `a10d7e949c03f399021c937b8211d70a744d84925d158d4d99ae5118aea839ad` |
| 新发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 历史迁移公钥 SHA256 | `03e979d5c6babda29aa217b7ce72268aab12870bda46a9072397fe52ba083b69` |
| 签名成员 / 散装文件集合 | 107 / 109，成员哈希与大小全部匹配 |
| Bootstrap 最低兼容门槛 | 82 |
| Windows 二进制版本 | 0.7.1.0 |
| 浏览器扩展版本 | 0.7.1 |
| 内部发布签名 | 新发布身份，RSA3072-PSS/SHA256，独立验证通过 |
| Windows Authenticode | `NotSigned` |

## 修复与验证

- 迁移架构回归92项通过，x86/x64实际检测入口验证普通包序列999在marker82下返回`compatible`；marker81、缺失marker、journal、篡改和轮换边界保持原行为。
- Inno runtime fixture维护成功、取消、失败、篡改、修复成功/取消/失败/篡改和检测失败共9项通过；完整安装器 runtime 16项及BrowserHost替换/回滚5项通过。
- 精确109文件清单、107成员哈希/大小、RSA3072-PSS签名、安装器哈希、Windows版本、扩展版本和兼容门槛输入全部核验通过。
- 外层 `package-local.ps1` 在子进程结束后未返回最终退出码；已停止本轮精确包装器进程并保存独立收尾证据，未据此宣称包装器正常退出。
- 未生成外层 ZIP；只交付正常安装器。

Release88 尚未在用户机器上实际安装验证。请运行本次安装器；同一兼容 Bootstrap 的普通新包不应再次弹出组件迁移授权。若仍失败，请提供新的 `preparation.jsonl`、`bootstrap-detection.jsonl` 和 `setup.log`。

持久证据：`.diagnostics/packaging/20261116-152454-cbe81232/`。
