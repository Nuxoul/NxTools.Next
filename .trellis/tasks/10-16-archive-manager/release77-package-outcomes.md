# 正式 Release77 — 0.7.1 手动信任轮换状态修复包

2026-11-15。Release76 的实际安装日志证明旧公钥登记轮换在停服务阶段仍使用新公钥校验，安装器在替换应用文件前失败。Release77 修复该状态切换后重新完成实际Release编译、签名和正常Inno安装器生成。原package进程11696退出0；仅交付正常安装器，未生成外层完整分发ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 77 |
| Release ID | `28fd21c9-27c7-4cc4-b818-bdcfb6384199` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 29,196,614 字节 / 27.84 MiB |
| SHA256 | `40FF7AE5C611E525055F053C6F4DE1D8620AD75DBF751D9BCB62F48EEE3D23A5` |
| 新发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 内部发布签名 | 新发布身份，RSA3072-PSS/SHA256，通过 |
| Manifest SHA256 | `c94df8b668c7d188761c69df9d42e43dcf839e93f1b0f90c3d5fc26dfcc24751` |
| 签名成员 / 精确散装文件集合 | 107 / 109，全部匹配 |
| 实际构建输出 / 扩展源文件 | 10 / 94，与包内匹配 |
| 归档批准 / helper Desktop绑定 | 5项批准及绑定匹配 |
| 固定构建输入 | 4,668文件，复制前后及构建期间内容一致 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

## 轮换修复

- Release76 日志阶段为 `trust_rotation_required`，新 Host 命令已启动但在停服务前仍被 `ValidateFixedRegistration` 用新公钥拒绝。
- Release77 允许轮换前使用已核验的旧公钥登记；完成 Bootstrap、公钥登记和 enrollment 切换后，服务校验状态更新为新公钥。
- 轮换仍只对精确服务路径、LocalSystem 身份和旧 Bootstrap 哈希完整匹配的登记开放；其他登记异常拒绝。失败恢复旧 Bootstrap 和旧 enrollment。
- 本轮未执行真实安装，因此旧版本现场覆盖、UAC、服务/注册表轮换和回滚仍待验收。

## 验证

- 固定源码包含轮换状态修复；具体变更保留在 `changes-since76.json`。构建使用独立缓存和输出，无共享可写目录、硬链接或目录联接。
- PlatformHost Release 编译通过，0警告/0错误；PowerShell轮换脚本语法通过。
- 实际完成Rust产品、归档worker、C#宿主Release编译和Inno生成。签名、公钥、107成员、精确109文件、10实际输出、94扩展源、5项归档批准及helper绑定通过。
- 实际Windows数字文件版本读回：Desktop、PlatformHost、ShellSelectionHelper EXE/DLL、正常安装器均为0.7.1.0，ProductVersion均为0.7.1。包内扩展清单与冻结源码一致，版本0.7.1。
- 交付前只读确认默认产物目录无运行中程序、安装器或关联服务；Release76备份保留在本轮 `previous-package/` 和 `previous-installer/`。

持久证据：`.diagnostics/packaging/20261115-171527-fdbd0c1e-published/`，含原日志/状态、源码基线、快照/缓存证明、公钥指纹、版本读回、独立/发布核验及验证复用记录。未提交或推送。

## 已知限制与待验项

真实旧版本覆盖安装、旧 Bootstrap/公钥登记轮换、UAC、服务恢复/回滚、LocalSystem索引器重启/base-plus-delta/空闲CPU及实际搜索、物理UI/焦点/DPI仍未验收。产品全量Rust/C#未重跑，当前材料独立离线重构建未执行。归档仍 **Phase E — Format expansion and ZIP editing / in_progress**；HZ5、源回收及原范围限定保持。B4实现未恢复。
