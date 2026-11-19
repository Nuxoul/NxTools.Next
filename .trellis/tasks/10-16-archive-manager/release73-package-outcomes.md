# 正式 Release73 — 0.7.1 当前源码重新打包

2026-11-14。原 `scripts/package-local.ps1 -SkipVerification` 完成实际Release编译、原发布身份签名和正常Inno安装器生成。原package进程4020及入口均正常退出0，用时1274.25秒。独立核验与交付读回通过，仅交付正常安装器，未生成外层完整分发ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 73 |
| Release ID | `5beb6c5a-b90f-4cc4-838b-42fcc0038ed0` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 29,185,432 字节 / 27.83 MiB |
| SHA256 | `1901B9B7B74071420D261297271ABE5DF9D5EB40193693DA8F97BA34FBD45481` |
| 内部发布签名 | 原公开信任锚，RSA3072-PSS/SHA256，通过 |
| Manifest SHA256 | `86e14d49279db2fffcb911518cf1229407f12966a7b5e0375c7d895e666412c9` |
| 签名成员 / 精确散装文件集合 | 107 / 109，全部匹配 |
| 实际构建输出 / 扩展源文件 | 10 / 94，与包内匹配 |
| 归档批准 / helper Desktop绑定 | 5项批准及绑定匹配 |
| 固定构建输入 | 4,662文件，复制前后及构建期间内容一致 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

## 验证

- 固定源码包含Release72之后的提权输入、快速启动等修改；具体文件保留在 `changes-since72.json`。工作树未提交修改保持。构建使用会话私有独立缓存与输出，无共享可写目录、硬链接或目录联接；此前构建输出和证据保留。
- 本轮实际完成Rust产品、归档worker、C#宿主Release编译和Inno生成。原入口正常结束，未启动重复构建或结束无关进程。
- 本轮身份30项、原生退出47项各自在x86/x64通过；16个安装器隔离场景、5个BrowserHost替换/回滚场景、版本一致性5项通过。
- 独立核验原签名、107成员、精确109文件、10实际输出、94扩展源、5项归档批准及helper绑定。两份installer-support源与Inno输入一致。助手SHA256为 `61BF530E71493B4715C5174FBF9052988A7B73BBCCF87B85F6178B2353688EA1`。
- 实际Windows数字文件版本读回：Desktop、PlatformHost、ShellSelectionHelper EXE/DLL、正常安装器均为0.7.1.0，ProductVersion均为0.7.1。包内扩展清单与冻结源码一致，版本0.7.1；14个第一方Rust包及第三方锁文件与上一轮保持。
- 对应源码材料与126个源文件、3个crate清单、147个依赖记录及许可证匹配，保留在 `artifacts/installer/third_party/archive-worker/`；不扩展可信安装白名单。
- 交付前只读确认默认产物目录无运行中程序、安装器或关联服务；发布逐文件读回与已核验构建一致。Release72备份保留在本轮 `previous-package/` 和 `previous-installer/`。

持久证据：`.diagnostics/packaging/20261114-134319-8220e329/`，含原日志/状态、源码基线、快照/缓存证明、公开构建输入、版本读回、独立/发布核验及验证复用记录。`artifacts/installer/INSTALL.md` 与 `evidence.json` 对应本次发布。

## 已知限制与待验项

本次未重跑产品全量Rust/C#，按 `verification-reuse.json` 复用既有定向验证；此前打包入口、源码材料、清理、扩展Node及浏览器签名fixture结果属于历史验证，不计本轮新执行。当前提权输入、快速启动等修改的交互验收不由打包代替。当前对应源码材料的独立离线重构建未执行。

未执行真实安装、卸载、产品启动、服务操作或注册表修改。实际安装/UAC/恢复、LocalSystem索引器重启/base-plus-delta/空闲CPU及实际搜索、物理UI/焦点/DPI和既有未通过或未运行项目保持原状态。

归档仍 **Phase E — Format expansion and ZIP editing / in_progress**；HZ5、源回收及原范围限定保持。B4实现未恢复。未提交或推送。
