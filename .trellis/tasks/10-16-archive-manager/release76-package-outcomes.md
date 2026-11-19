# 正式 Release76 — 0.7.1 手动信任轮换修复包

2026-11-15。原 `scripts/package-local.ps1 -SkipVerification` 完成实际Release编译、签名和正常Inno安装器生成。原package进程9076退出0；外层包装器等待常驻工具时按保留的进程退出证据完成收尾。仅交付正常安装器，未生成外层完整分发ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 76 |
| Release ID | `be45645d-4b48-4fdb-8013-f6e3fcc3db56` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 29,195,241 字节 / 27.84 MiB |
| SHA256 | `FFB8E6661A33D6A326D9485D077A4767CFFD7209D5CDBA64817F7C8AADA52DBC` |
| 新发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 内部发布签名 | 新发布身份，RSA3072-PSS/SHA256，通过 |
| Manifest SHA256 | `11a897114cf14d7ae51a5ed9289a3fea17dbb49f288535afb6fb65b3fe630ecb` |
| 签名成员 / 精确散装文件集合 | 107 / 109，全部匹配 |
| 实际构建输出 / 扩展源文件 | 10 / 94，与包内匹配 |
| 归档批准 / helper Desktop绑定 | 5项批准及绑定匹配 |
| 固定构建输入 | 4,667文件，复制前后及构建期间内容一致 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

## 轮换修复

- `bootstrap-migration.ps1` 在旧公钥不同但服务登记、Bootstrap 路径、LocalSystem 身份和旧 Bootstrap 哈希均完整匹配时，返回显式信任轮换结果；其他登记异常继续拒绝。
- Inno 安装器显示一次明确的旧发布身份迁移确认，调用新 `--rotate-trusted-bootstrap` 命令，再进入原有精确进程关闭、文件准备和覆盖安装流程。
- PlatformHost 轮换事务验证新包，锁定旧 enrollment/Bootstrap，写入候选和恢复记录，在部署门内更新 Bootstrap、公钥登记及 enrollment；失败时恢复旧 Bootstrap 和旧 enrollment。普通同密钥维护路径保持不变。
- 本轮只完成源代码、Release 编译和隔离检查；未执行真实安装、UAC、服务、注册表或旧版本现场覆盖，因此不能宣称实际轮换已验收。

## 验证

- 固定源码包含轮换命令、C# 事务、安装器检测/提权参数和轮换说明；具体文件保留在 `changes-since75.json`。构建使用独立缓存和输出，无共享可写目录、硬链接或目录联接。
- PlatformHost Release 编译通过，0警告/0错误；PowerShell 语法检查通过。
- 实际完成Rust产品、归档worker、C#宿主Release编译和Inno生成。签名、107成员、精确109文件、10实际输出、94扩展源、5项归档批准及helper绑定通过。
- 实际Windows数字文件版本读回：Desktop、PlatformHost、ShellSelectionHelper EXE/DLL、正常安装器均为0.7.1.0，ProductVersion均为0.7.1。包内扩展清单与冻结源码一致，版本0.7.1。
- 交付前只读确认默认产物目录无运行中程序、安装器或关联服务；Release75备份保留在本轮 `previous-package/` 和 `previous-installer/`。

持久证据：`.diagnostics/packaging/20261115-125815-042c8d1d-published/`，含原日志/状态、源码基线、快照/缓存证明、公钥指纹、版本读回、独立/发布核验及验证复用记录。未提交或推送。

## 已知限制与待验项

真实旧版本覆盖安装、旧 Bootstrap/公钥登记轮换、UAC、服务恢复/回滚、LocalSystem索引器重启/base-plus-delta/空闲CPU及实际搜索、物理UI/焦点/DPI仍未验收。产品全量Rust/C#未重跑，当前材料独立离线重构建未执行。归档仍 **Phase E — Format expansion and ZIP editing / in_progress**；HZ5、源回收及原范围限定保持。B4实现未恢复。
