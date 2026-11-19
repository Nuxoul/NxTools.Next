# 正式 Release78 — 0.7.1 轮换预提交残留恢复修复包

2026-11-15。Release77 实际安装后留下了“旧 Bootstrap 未移动、候选 Bootstrap 和轮换 journal 已存在”的预提交状态。Release78 增加了严格的预提交恢复：仅当旧身份、旧 Bootstrap 哈希和候选哈希全部匹配时清理候选并继续；已有替换或身份不一致仍保留现场并拒绝。完成实际Release编译、签名和正常Inno安装器生成，原package入口退出0。仅交付正常安装器，未生成外层完整分发ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 78 |
| Release ID | `f4973613-9624-452f-a5ae-3f549d3e03d3` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 29,196,788 字节 / 27.84 MiB |
| SHA256 | `B0F0D3FBB90E1D9CBA35CA06E08031B3382613661D8025918EC3A60FA441B5E7` |
| 新发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 内部发布签名 | 新发布身份，RSA3072-PSS/SHA256，通过 |
| Manifest SHA256 | `e4134bc6ad66d459878ffb1621fd23626d1de065b7a9eaec94231d0c91c7411b` |
| 签名成员 / 精确散装文件集合 | 107 / 109，全部匹配 |
| 实际构建输出 / 扩展源文件 | 10 / 94，与包内匹配 |
| 归档批准 / helper Desktop绑定 | 5项批准及绑定匹配 |
| 固定构建输入 | 4,669文件，复制前后及构建期间内容一致 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

## 修复与验证

- 现场 correlation `20261115174418-is-R4KUATHKC0-tmp` 表明 Release77 已在 `trust_rotation_required` 后进入轮换，但旧失败留下了预提交 journal/candidate。
- Release78 在新轮换开始前恢复该预提交状态：校验旧 key/bootstrap、候选文件和 journal；只有完全匹配才删除候选和 journal，随后继续当前新身份轮换。
- PlatformHost Release 编译0警告/0错误；PowerShell轮换脚本语法通过。签名、公钥、清单、扩展、Windows版本和精确109文件核验通过。
- 交付前只读确认默认产物目录无运行中程序、安装器或关联服务；Release77备份保留在本轮 `previous-package/` 和 `previous-installer/`。

持久证据：`.diagnostics/packaging/20261115-174828-5ae3bbbf-published3/`。未执行真实安装、UAC、服务/注册表操作；未提交或推送。

## 已知限制

Release78 尚未在用户机器上实际安装验证；旧版本覆盖、旧 Bootstrap/公钥登记轮换、恢复回滚、LocalSystem索引器重启/base-plus-delta/空闲CPU、实际搜索和物理UI仍未验收。产品全量Rust/C#未重跑，当前材料独立离线重构建未执行。归档仍 **Phase E — Format expansion and ZIP editing / in_progress**；HZ5、源回收及原范围限定保持。
