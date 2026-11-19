# 正式 Release80 — 0.7.1 Bootstrap 序号门禁修复包

2026-11-15。Release79 解决了迁移期旧 active 快照的公钥验证，但现场仍可能保留旧版本 Bootstrap，登记公钥已更新而 Bootstrap 代码未更新。Release80 增加 Bootstrap 维护序号门禁：当前安装包序号高于 Bootstrap 维护状态时，即使公钥相同也强制同公钥维护更新，确保新 Bootstrap 代码先于 trusted update 请求生效。完成实际Release编译、签名和正常Inno安装器生成，原package入口退出0。仅交付正常安装器，未生成外层完整分发ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 80 |
| Release ID | `01971a00-bf36-4255-8108-8e176813a049` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 29,199,313 字节 / 27.85 MiB |
| SHA256 | `026FAFBB6C0FA8CB7154A70144242B227517CB818D43770E9D3F49184E144397` |
| 新发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 内部发布签名 | 新发布身份，RSA3072-PSS/SHA256，通过 |
| Manifest SHA256 | `3940dc3b78f1973743605a376ce41458e591e0432eb4d30648b1d59d244818f1` |
| 签名成员 / 精确散装文件集合 | 107 / 109，全部匹配 |
| 实际构建输出 / 扩展源文件 | 10 / 94，与包内匹配 |
| 归档批准 / helper Desktop绑定 | 5项批准及绑定匹配 |
| 固定构建输入 | 4,672文件，复制前后及构建期间内容一致 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

## 修复与验证

- 迁移检测现在比较 `bootstrap-maintenance-state.json` 的序号与当前安装包序号；旧 Bootstrap 序号较低时强制维护更新，不再误判 compatible。
- 旧 active 快照仍仅在迁移期使用历史公开锚验证，新包和后续发布仍只用新公钥。
- PlatformHost Release编译0警告/0错误；PowerShell轮换脚本语法通过。签名、公钥、清单、扩展、Windows版本和精确109文件核验通过。
- 交付前只读确认默认产物目录无运行中程序、安装器或关联服务；Release79备份保留在本轮 `previous-package/` 和 `previous-installer/`。

持久证据：`.diagnostics/packaging/20261115-191139-5f3011d4-published5/`。未执行真实安装、UAC、服务/注册表操作；未提交或推送。

## 已知限制

Release80 尚未在用户机器上实际安装验证；旧版本覆盖、Bootstrap轮换、受保护 active 部署、恢复回滚、LocalSystem索引器重启/base-plus-delta/空闲CPU、实际搜索和物理UI仍未验收。产品全量Rust/C#未重跑，当前材料独立离线重构建未执行。归档仍 **Phase E — Format expansion and ZIP editing / in_progress**；HZ5、源回收及原范围限定保持。
