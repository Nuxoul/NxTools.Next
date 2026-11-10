# 正式 Release63 — 当前源码安装器重新打包

2026-11-09。原 `scripts/package-local.ps1 -SkipVerification` 完成实际Release编译、原发布身份签名和正常Inno安装器生成。原package.ps1退出0，独立核验通过。仅交付安装器，未生成外层完整分发ZIP。未执行真实安装、卸载、产品启动、服务操作或注册表修改。

## 产物

| 项目 | 结果 |
| --- | --- |
| 版本 / 发布序号 | 0.7.0 / 63 |
| Release ID | `1c02d96c-419f-4f45-be88-58bf7492cda1` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.0.exe` |
| 大小 | 28,276,433 字节 / 26.97 MiB |
| SHA256 | `3BF86431C917C0D34B3B40AA2FDACB17D71BE8CDE0249892C4B8BB0FFE8BAC69` |
| 内部发布签名 | 原公开信任锚，RSA3072-PSS/SHA256，通过 |
| Manifest SHA256 | `f94585bb36fc2f8e72e9f6be7583fd2fccf591a712a75b5f572b634948e76596` |
| 助手 SHA256 | `61BF530E71493B4715C5174FBF9052988A7B73BBCCF87B85F6178B2353688EA1` |
| 签名成员 / 精确散装包集合 | 78 / 80，全部匹配 |
| 实际构建输出 / 扩展源文件 | 10 / 65，与包内匹配 |
| 归档批准 / helper Desktop 绑定 | 5项批准及绑定匹配 |
| 固定源码输入 | 8,625文件，复制前后及构建期间内容一致 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

## 验证

- 当前工作树内容被固定为独立源码快照，保留未提交修改；独立复制Release缓存，构建输出不与工作树共享可写目录。8,625文件逐项内容核验通过。
- 实际完成Rust产品、归档worker和C#宿主Release编译、原发布签名及Inno安装器生成。原package进程45596退出0，进程退出证据和包装器原状态均保留；常驻编译工具未被结束，未重复构建。
- 本轮实际执行身份30项和原生退出47项，各自在x86/x64通过；16个安装器回调隔离场景及5个BrowserHost替换/回滚场景通过。
- 独立核验签名、78成员、精确80文件、10实际输出、65扩展源、5项归档批准及helper Desktop绑定。两份installer-support源文件与Inno编译输入一致，不计入上述散装包80文件集合。
- [按需更新服务停止状态修复](installer-stopped-updater-repair.md)保留，助手身份与Release62一致。此前实际只读入口重放、升级18项、诊断8项及源码材料5项为历史验证，本次复用，不计作新执行数量。打包回归输入与Release62匹配。
- 对应源码材料与当前106个源文件、3个crate清单、147个依赖记录及许可证匹配。材料继续保留在 `third_party/archive-worker/`，不改变可信安装清单或发布身份。
- 正式交付前只读确认默认产物目录没有运行中使用者、安装器使用者或关联服务；正式文件逐项读回与已核验构建一致。旧产物保留在本次证据的 `previous-package/`、`previous-installer/`。

持久证据：`.diagnostics/packaging/20261109-170424-3490ac21/`，含原日志/状态、进程退出证据、源码基线、快照证明、公开构建输入、独立/已发布核验和验证复用记录。安装说明及 `artifacts/installer/evidence.json` 已更新为本次发布。未提交或推送。

## 已知限制与待验项

产品全量Rust/C#和当前对应材料的独立离线重构建未重跑；已有定向验证按 `verification-reuse.json` 复用，不声明完整集合已通过。Release编译保留既有Desktop可见性警告和Inno命名提示。

本次未执行真实安装或服务操作，不能作为完整安装/升级接受。实际安装、UAC/恢复、卸载、LocalSystem索引器重启/base-plus-delta/空闲CPU与真实搜索、物理UI/焦点/DPI及其他原有未通过或未运行项目保持原状态。

归档仍 **Phase E — Format expansion and ZIP editing / in_progress**；HZ5、源回收和原范围限定保持。
