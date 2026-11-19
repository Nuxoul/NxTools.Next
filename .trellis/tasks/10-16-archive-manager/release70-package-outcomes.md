# 正式 Release70 — 当前源码安装器重新打包

2026-11-14。原 `scripts/package-local.ps1 -SkipVerification` 完成实际Release编译、原发布身份签名和正常Inno安装器生成。原package.ps1退出0，独立核验通过。仅交付安装器，未生成外层完整分发ZIP。未执行真实安装、卸载、产品启动、服务操作或注册表修改。

## 产物

| 项目 | 结果 |
| --- | --- |
| 版本 / 发布序号 | 0.7.0 / 70 |
| Release ID | `83c09a9c-3a6a-4444-93df-9f914febf398` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.0.exe` |
| 大小 | 29,177,542 字节 / 27.83 MiB |
| SHA256 | `6D51E08C72B962C9F2FB6E9B6E0EBEEE80E5D42B9EB7F808C3F3CA1D022DD853` |
| 内部发布签名 | 原公开信任锚，RSA3072-PSS/SHA256，通过 |
| Manifest SHA256 | `a7457cd5bb5d20318efee07a84dde5e7260d106d0029c5ab943f0f1ad5183842` |
| 助手 SHA256 | `61BF530E71493B4715C5174FBF9052988A7B73BBCCF87B85F6178B2353688EA1` |
| 签名成员 / 精确散装包集合 | 107 / 109，全部匹配 |
| 实际构建输出 / 扩展源文件 | 10 / 94，与包内匹配 |
| 归档批准 / helper Desktop 绑定 | 5项批准及绑定匹配 |
| 固定构建输入 | 4,656文件，复制前后及构建期间内容一致 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

## 验证

- 当前源码、资源、依赖、规范和任务记录固定为4,656个构建输入文件，保留未提交修改；Release缓存为独立副本，构建输出不与工作树共享可写目录。输入逐项内容核验通过。
- 实际完成Rust产品、归档worker和C#宿主Release编译、原发布签名及Inno安装器生成。原package进程23092退出0，真实退出证据及包装器原状态保留；常驻编译工具未被结束，未重复构建。
- 本轮执行身份30项和原生退出47项，各自在x86/x64通过；16个安装器回调隔离场景及5个BrowserHost替换/回滚场景通过。按需更新服务停止状态修复和助手身份保持。
- 独立核验签名、107成员、精确109文件、10实际输出、94扩展源、5项归档批准和helper Desktop绑定。两份installer-support源文件与Inno编译输入一致，不计入上述散装包109文件集合。
- 对应材料与当前126个源文件、3个crate清单、147个依赖记录及许可证匹配。材料保留在 `third_party/archive-worker/`，不改变可信安装清单或发布身份。
- 产品全量Rust/C#未重跑，已有定向验证按 `verification-reuse.json` 复用。历史只读入口重放、升级/诊断及源码材料回归不计为本次新执行；打包相关回归输入与已验证版本匹配。
- 正式交付前只读确认默认产物目录没有运行中使用者、安装器使用者或关联服务；交付逐文件读回与已核验构建一致，文件集合保持精确109项。旧产物保留在 `previous-package/`、`previous-installer/`。

持久证据：`.diagnostics/packaging/20261114-083338-0a02063b/`，含原日志/状态、进程退出证据、源码基线、快照证明、公开构建输入、独立/已发布核验及验证复用记录。安装说明和 `artifacts/installer/evidence.json` 已更新为本次发布。未提交或推送。

## 已知限制与待验项

本次不是全量产品或实机接受。当前对应材料的独立离线重构建未重跑，历史结果不作为当前输入接受。实际安装、UAC/恢复、卸载、LocalSystem索引器重启/base-plus-delta/空闲CPU与真实搜索、物理UI/焦点/DPI及其他既有未通过或未运行项目保持原状态。

归档仍 **Phase E — Format expansion and ZIP editing / in_progress**；HZ5、源回收和原范围限定保持。
