# 正式 Release79 — 0.7.1 迁移期旧 active 快照验证回退包

2026-11-15。Release78 解决了预提交 journal 残留，但迁移后首次 `--prepare-trusted-update` 仍会读取旧公钥签名的 protected active 快照。Release79 增加迁移期历史公钥只读验证回退：仅用于当前旧快照/恢复证据，新包和后续发布仍只接受新公钥。完成实际Release编译、签名和正常Inno安装器生成，原package入口退出0。仅交付正常安装器，未生成外层完整分发ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 79 |
| Release ID | `64c97271-f7c4-4883-8e43-cab023a0952a` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 29,194,961 字节 / 27.84 MiB |
| SHA256 | `523A97822C522F501DAB2FE920512B58FADA8704DC4E5A5D0803D64FD1041E4B` |
| 新发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 内部发布签名 | 新发布身份，RSA3072-PSS/SHA256，通过 |
| Manifest SHA256 | `6a43a7b1c2dd82f9ceac2500ad5c315ce8cf950f62676ccc380f2bc13d7e2f9e` |
| 签名成员 / 精确散装文件集合 | 107 / 109，全部匹配 |
| 实际构建输出 / 扩展源文件 | 10 / 94，与包内匹配 |
| 归档批准 / helper Desktop绑定 | 5项批准及绑定匹配 |
| 固定构建输入 | 4,671文件，复制前后及构建期间内容一致 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

## 修复与验证

- 迁移期只使用历史公开锚验证当前 protected active/recovery snapshot；新发布包的 admission、签名和后续包仍使用新公钥。
- PlatformHost Release编译0警告/0错误；PowerShell轮换脚本语法通过。签名、公钥、清单、扩展、Windows版本和精确109文件核验通过。
- 交付前只读确认默认产物目录无运行中程序、安装器或关联服务；Release78备份保留在本轮 `previous-package/` 和 `previous-installer/`。

持久证据：`.diagnostics/packaging/20261115-183837-e9cc2825-published4/`。未执行真实安装、UAC、服务/注册表操作；未提交或推送。

## 已知限制

Release79 尚未在用户机器上实际安装验证；旧版本覆盖、完整信任轮换、恢复回滚、LocalSystem索引器重启/base-plus-delta/空闲CPU、实际搜索和物理UI仍未验收。产品全量Rust/C#未重跑，当前材料独立离线重构建未执行。归档仍 **Phase E — Format expansion and ZIP editing / in_progress**；HZ5、源回收及原范围限定保持。
