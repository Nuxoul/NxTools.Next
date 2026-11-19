# 正式 Release75 — 0.7.1 新发布身份手动迁移包

2026-11-15。原 `scripts/package-local.ps1 -SkipVerification` 使用新发布身份完成实际Release编译、签名和正常Inno安装器生成。原package进程25092退出0；外层包装器等待常驻工具时按保留的进程退出证据完成收尾。仅交付正常安装器，未生成外层完整分发ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 75 |
| Release ID | `a93beec0-4d28-4d5b-bde0-580ba0c303c9` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 29,192,471 字节 / 27.84 MiB |
| SHA256 | `3E5FAB95EC5AAA9F486F11B3D61336FF8ECE0C81DEEC1B74755B1D9D0FC67899` |
| 新发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 内部发布签名 | 新发布身份，RSA3072-PSS/SHA256，通过 |
| Manifest SHA256 | `c9a3c69d53b688fab7a0ff4d763875834dae781232914cd1131ae7d7d23fe44f` |
| 签名成员 / 精确散装文件集合 | 107 / 109，全部匹配 |
| 实际构建输出 / 扩展源文件 | 10 / 94，与包内匹配 |
| 归档批准 / helper Desktop绑定 | 5项批准及绑定匹配 |
| 固定构建输入 | 4,666文件，复制前后及构建期间内容一致 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

## 手动迁移模型

- 新私钥仅保存在仓库外的 `D:\NxTools-private-release-keys\`，未复制到仓库、scratch、安装器或日志；对应 ledger 从历史序号74继续，Release75使用序号75。
- 新安装器沿用原 AppId、默认安装目录和文件布局，可由用户直接运行覆盖旧版本，不要求先卸载。
- 旧版本不会自动验证新公钥；本次迁移依赖用户明确运行新安装器。新版本安装完成后，后续手动安装包使用新公钥验证。
- 本包未执行真实安装，因此未宣称已完成旧版本现场覆盖、旧受信组件轮换或服务注册状态迁移；这些仍需单独授权的真实安装验收。

## 验证

- 固定源码包含新公钥、外部发布材料路径、手动迁移文档和Release74之后的当前源码修改；具体文件保留在 `changes-since74.json`。构建使用会话私有独立缓存与输出，无共享可写目录、硬链接或目录联接。
- 实际完成Rust产品、归档worker、C#宿主Release编译和Inno生成。签名、107成员、精确109文件、10实际输出、94扩展源、5项归档批准及helper绑定通过。
- 实际Windows数字文件版本读回：Desktop、PlatformHost、ShellSelectionHelper EXE/DLL、正常安装器均为0.7.1.0，ProductVersion均为0.7.1。包内扩展清单与冻结源码一致，版本0.7.1；14个第一方Rust包及第三方锁文件保持。
- 本轮身份30项、原生退出47项各自在x86/x64通过；16个安装器隔离场景、5个BrowserHost替换/回滚场景和版本一致性5项通过。产品全量Rust/C#未重跑，既有定向结果按 `verification-reuse.json` 复用，不计本轮新执行。
- 对应源码材料与126个源文件、3个crate清单、147个依赖记录及许可证匹配，保留在 `artifacts/installer/third_party/archive-worker/`。
- 交付前只读确认默认产物目录无运行中程序、安装器或关联服务；Release74备份保留在本轮 `previous-package/` 和 `previous-installer/`。

持久证据：`.diagnostics/packaging/20261115-121004-bb6cb18d-published/`，含原日志/状态、源码基线、快照/缓存证明、公钥指纹、版本读回、独立/发布核验及验证复用记录。未执行真实安装、卸载、产品启动、服务操作或注册表修改；未提交或推送。

## 已知限制与待验项

当前对应源码材料的独立离线重构建未执行。旧版本到新发布身份的实际覆盖安装、受信组件轮换、UAC/恢复、LocalSystem索引器重启/base-plus-delta/空闲CPU及实际搜索、物理UI/焦点/DPI仍未验收。归档仍 **Phase E — Format expansion and ZIP editing / in_progress**；HZ5、源回收及原范围限定保持。B4实现未恢复。
