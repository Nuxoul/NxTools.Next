# 正式 Release81 — 0.7.1 迁移 Bootstrap/active 快照最终修复包

2026-11-15。Release81 包含迁移期旧 active 快照历史公钥回退验证，以及 Bootstrap 序号门禁：登记公钥相同但 Bootstrap 版本低于当前包时，强制同公钥维护更新。完成实际Release编译、签名和正常Inno安装器生成，原package入口退出0。仅交付正常安装器，未生成外层完整分发ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 81 |
| Release ID | `f87da92b-0200-420b-875b-63f4cb4f36e5` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 29,202,343 字节 / 27.85 MiB |
| SHA256 | `5ACE0D5486E6993530BA4C95E799C414AE5D3ADCD3C460EF2CE3D3EAB6DD5522` |
| 新发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 内部发布签名 | 新发布身份，RSA3072-PSS/SHA256，通过 |
| Manifest SHA256 | `56c7c7e66d782b64eba4d55d29b6725208eaa4f30c892c0b7504ca9a9ba68108` |
| 签名成员 / 精确散装文件集合 | 107 / 109，全部匹配 |
| 实际构建输出 / 扩展源文件 | 10 / 94，与包内匹配 |
| 归档批准 / helper Desktop绑定 | 5项批准及绑定匹配 |
| 固定构建输入 | 4,673文件，复制前后及构建期间内容一致 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

## 修复与验证

- 迁移期旧 active/recovery snapshot 只使用历史公开锚验证，新包及后续发布仍使用新公钥。
- 当前安装包序号高于 Bootstrap 维护状态时，强制同公钥 Bootstrap 更新，避免旧 Bootstrap 代码处理新迁移状态。
- PlatformHost Release 编译0警告/0错误；PowerShell轮换脚本语法通过。签名、公钥、清单、扩展、Windows版本和精确109文件核验通过。
- 交付前只读确认默认产物目录无运行中程序、安装器或关联服务；Release80备份保留在本轮 `previous-package/` 和 `previous-installer/`。

持久证据：`.diagnostics/packaging/20261115-194149-d5343b2a-published6/`。未执行真实安装、UAC、服务/注册表操作；未提交或推送。

## 已知限制

Release81 尚未在用户机器上实际安装验证；旧版本覆盖、Bootstrap轮换、受保护 active 部署、恢复回滚、LocalSystem索引器重启/base-plus-delta/空闲CPU、实际搜索和物理UI仍未验收。产品全量Rust/C#未重跑，当前材料独立离线重构建未执行。归档仍 **Phase E — Format expansion and ZIP editing / in_progress**；HZ5、源回收及原范围限定保持。
