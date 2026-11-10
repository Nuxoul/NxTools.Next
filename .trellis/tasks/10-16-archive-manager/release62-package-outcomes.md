# 正式 Release62 — 已停止更新服务的安装准备修复

2026-11-09。修复[实际安装准备失败](installer-stopped-updater-repair.md)，经原 `scripts/package-local.ps1 -SkipVerification` 完成Release编译、原发布身份签名和正常Inno安装器生成。原package.ps1退出0，独立核验通过。仅交付安装器，未生成外层完整分发ZIP；未执行真实安装、卸载、服务操作或注册表修改。

## 产物

| 项目 | 结果 |
| --- | --- |
| 版本 / 发布序号 | 0.7.0 / 62 |
| Release ID | `eb6d209d-2726-4519-ae5f-b3018589c351` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.0.exe` |
| 大小 | 28,212,392 字节 / 26.91 MiB |
| SHA256 | `ACA23678A3BB2341A46C0510DCD7F17764E4F8C3FFF95C1B5000BED6BB3F5C30` |
| 内部发布签名 | 原公开信任锚，RSA3072-PSS/SHA256，通过 |
| Manifest SHA256 | `707461b0b311ecce9cf8fcfec410f5e5c8467b19b100952cb34e800a9e0d4d34` |
| 助手 SHA256 | `61BF530E71493B4715C5174FBF9052988A7B73BBCCF87B85F6178B2353688EA1` |
| 签名成员 / 精确散装包集合 | 78 / 80，全部匹配 |
| 实际构建输出 / 扩展源文件 | 10 / 65，与包内匹配 |
| 归档批准 / helper Desktop 绑定 | 5项批准及绑定匹配 |
| 固定源码输入 | 8,605文件，复制前后及构建期间内容一致 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

## 修复边界

实际correlation `20261109110405-is-9R3J9TM5Z7-tmp` 探测compatible后，在32位提权助手的process_close阶段拒绝，异常InvalidDataException/scriptline187，关联Host PID39164。原只读函数在匹配登记/文件身份但SCM报告Stopped/PID0时重现同一拒绝。后续观察时该PID已不存在，不推断其原角色。

Resolver保留固定路径、账户、公钥字段、bootstrap哈希、SCM状态形状和own-process类型验证。仅有效Stopped/PID0返回无排除进程，继续原候选QueryImage、精确目标与独立同句柄Stop边界；Running仍需有效正PID。外来身份、Stopped/nonzero、异常类型、未知查询失败继续拒绝。没有服务启动绕行、同名跳过、错误码忽略或恢复文件删除。

## 验证

- 原只读生产函数与新增回归保留先红后绿证据；修复后本机x86/x64原入口均返回excluded_pid0，实际更新服务仍为Manual/Stopped/PID0。
- 身份回归30项与原生退出回归47项各自在x86/x64通过。覆盖停止状态、有效运行排除、错误登记/文件/type、普通Host原生查询、无关Host保留、已退出证明及真实查询错误/PID归因。
- 原隔离升级18/18、准备诊断8场景、源码材料5测试、PowerShell语法及规范文档126文档/14owner检查通过。
- 原打包脚本新增每次编译前x86/x64身份门禁，本轮实际执行两次30项身份回归及两次47项退出回归，并通过16个Inno场景和5个BrowserHost替换/回滚场景。
- 实际Release编译、原签名、78成员、精确80文件、10输出、65扩展源、5项批准及helper Desktop绑定一致。助手与两份卸载支持源/Inno编译输入匹配；正式交付读回与已核验产物一致。
- 独立源码快照和Release缓存保留当前未提交修改，不共享可写构建目录。8,605输入无内容漂移。对应材料与106个源文件、3个crate清单、147个依赖记录及许可证匹配。

持久证据：`.diagnostics/packaging/20261109-111508-d6303d3a/`，含原日志/状态、退出证据、源码基线、快照证明、公开构建输入、独立/已发布核验、`repair-regressions/`及本次实际失败的`actual-failed-installation/`。原package进程43760退出0；常驻编译工具未被结束，未重复构建。此前产物和失败记录保留。

## 已知限制与待验项

本次没有执行真实安装，不能声明报告机器已恢复。实际UAC/preparation、后续trusted update、安装/finalization及签名文件读回仍需实际结果。修复只覆盖已证实的正常停止状态误拒绝，不代表后续所有原生阶段已接受。

产品全量Rust/C#和当前对应源码的独立离线重构建未重跑；历史定向结果按`verification-reuse.json`复用。既有完整集合失败/忽略、三条Desktop可见性警告、Inno命名提示、索引器安装态CPU/重启/搜索及物理UI等开放项保持。

归档仍 **Phase E — Format expansion and ZIP editing / in_progress**；HZ5、源回收及原范围限定不变。未提交或推送。
