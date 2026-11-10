# 正式 Release60 — 当前源码安装器重新打包

2026-11-07。原 `scripts/package-local.ps1 -SkipVerification` 完成实际 Release 编译、原发布身份签名和正常 Inno 安装器生成；原 package.ps1 退出0，独立核验通过。只交付安装器，未生成外层完整分发ZIP。未执行真实安装、卸载、服务操作、注册表修改、产品启动或提交推送。

## 产物

| 项目 | 结果 |
| --- | --- |
| 版本 / 发布序号 | 0.7.0 / 60 |
| Release ID | `31c5f27f-fed5-4664-90ab-289f2fd1a5d9` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.0.exe` |
| 大小 | 28,085,626 字节 / 26.78 MiB |
| SHA256 | `0EE071FB3B1EF0CE5ED8A3A799F5D555ABC3352FD128E14CBB6E97DD48EFF719` |
| 内部发布签名 | 原公开信任锚，RSA3072-PSS/SHA256，通过 |
| Manifest SHA256 | `b150a40a2416f63ff03d8a3dd3c1c37ab33e4f5935c0215f5823835af9771648` |
| 签名成员 / 精确散装包集合 | 78 / 80，全部匹配 |
| 实际构建输出 / 扩展源文件 | 10 / 65，与包内匹配 |
| 归档批准 / helper Desktop 绑定 | 5项批准及绑定匹配 |
| 源码输入 | 8,587个固定文件，复制前后及构建期间内容一致 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

## 本次范围与验证

- 使用当前完整工作树内容，保留未提交修改；独立复制Release缓存，构建输入和输出不与工作树共享可写目录。8,587个文件逐项核验，内容无漂移。
- 实际完成产品、归档worker和C#宿主Release编译、原发布签名、16个Inno回调隔离场景及5个BrowserHost替换/回滚场景。原打包进程33640退出0；包装器仅等待常驻vctip，保留原状态/退出证据后收尾，未结束该工具或重复构建。
- 独立核验原信任锚、78签名成员、精确80文件、10个实际输出、65扩展源、5项归档批准和helper Desktop绑定。正式交付逐文件读回与已核验产物一致。
- 安装器另安装两份卸载支持脚本：`installer-support/close-owned-processes.ps1`、`installer-support/uninstall-cleanup.ps1`。已核对源文件、固定快照和Inno编译输入；它们不计入上述受信散装包80文件集合。未执行真实卸载。
- 当前首次索引、设置、原生选择器、Shell推荐和卸载实现及其已有定向验证，参见[任务记录](../11-07-onboarding-settings-native-dialogs/outcomes.md)和[卸载记录](../11-07-onboarding-settings-native-dialogs/uninstall-outcomes.md)。本次将这些当前实现打入包，不把历史定向结果计为新执行的测试。
- 产品全量Rust/C#未重跑，既有定向结果复用，出处保留在 `verification-reuse.json`。原打包入口12项、源码材料5项回归的输入与已通过版本一致，本次复用，不计作新运行。
- 对应源码材料与当前104个源文件、3个crate清单一致；147个依赖记录和许可证匹配。材料继续保留在 `third_party/archive-worker/`，原生产安装清单和签名身份不变。
- 默认交付目录没有运行中使用者或关联服务，未停止真实服务。历史产物保留在本次证据的 `previous-package/`、`previous-installer/`。

持久证据：`.diagnostics/packaging/20261107-232843-76c0e449/`，含原日志/状态、进程退出证据、固定输入证明、公开生成输入、独立/已发布核验、验证复用和源码变化记录。`artifacts/installer/INSTALL.md`、`evidence.json` 已更新为本次发布。

## 已知限制与待验项

本次不是全量产品或实机接受。最近完整Desktop运行仍记录1765通过/15失败/1忽略，其中七项范围内修复后定向通过，完整集合未重跑；外部归档wire条件及既有下载菜单断言问题保持记录。Release编译保留既有三条 `private_interfaces` 警告及Inno的 `FileCopy` 命名提示。

当前源码材料的独立离线重构建未重跑，旧输入的结果不复用。真实安装、一次性UAC/恢复、首次索引性能、LocalSystem重启/base-plus-delta/空闲CPU与真实搜索、物理选择器/焦点/DPI、Explorer集成及正常/保留数据卸载接受均未执行。未知运行时载荷的保留策略和其他既有未通过/未运行项不因包生成而关闭。

归档仍 **Phase E — Format expansion and ZIP editing / in_progress**；HZ5、源回收及原范围限定保持。
