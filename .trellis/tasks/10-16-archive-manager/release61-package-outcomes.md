# 正式 Release61 — 安装器重新打包

2026-11-09。原 `scripts/package-local.ps1 -SkipVerification` 完成实际 Release 编译、原发布身份签名和正常 Inno 安装器生成；原 package.ps1 退出0，独立核验通过。仅交付安装器，未生成外层完整分发 ZIP。未执行真实安装、卸载、产品启动、服务操作或注册表修改。

## 产物

| 项目 | 结果 |
| --- | --- |
| 版本 / 发布序号 | 0.7.0 / 61 |
| Release ID | `34b339a9-79db-427c-91b5-8d304e6c95ee` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.0.exe` |
| 大小 | 28,212,963 字节 / 26.91 MiB |
| SHA256 | `1F9264250DB8CB305E060A68EC8B71A57E258EDEEFD8CF9C445A7448A5127639` |
| 内部发布签名 | 原公开信任锚，RSA3072-PSS/SHA256，通过 |
| Manifest SHA256 | `ba4adc049d7b26de1a97597deab2f6d52908d1004d94b2ba6e385304560b81ce` |
| 签名成员 / 精确散装包集合 | 78 / 80，全部匹配 |
| 实际构建输出 / 扩展源文件 | 10 / 65，与包内匹配 |
| 归档批准 / helper Desktop 绑定 | 5项批准及绑定匹配 |
| 源码输入 | 8,603个固定文件，复制前后及构建期间内容一致 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

## 验证

- 当前工作树内容被固定为独立源码快照，保留未提交修改；Release缓存为独立副本，构建输出不与工作树共享可写目录。8,603文件逐项内容核验通过。
- 实际完成Rust产品、归档worker和C#宿主Release编译、原发布签名及Inno安装器生成。原打包进程24100退出0，保留包装器原状态和进程退出证据；常驻编译工具未被结束，未重复构建。
- 本次执行16个安装器回调隔离场景、5个BrowserHost替换/回滚场景，全部通过。独立核验签名、78成员、精确80文件、10实际输出、65扩展源、5项归档批准及helper Desktop绑定。
- 两份 `installer-support` 卸载脚本已核对固定源文件和Inno编译输入，不计入上述散装包80文件集合。未执行真实卸载。
- 产品全量Rust/C#未重跑，已有定向验证按 `verification-reuse.json` 复用。打包入口12项和源码材料5项回归输入与历史已通过版本一致，本次复用，不计作新执行数量。
- 对应材料与当前106个源文件、3个crate清单一致，147个依赖记录和许可证匹配。当前材料的独立离线重构建未执行，历史结果不作为当前输入接受。
- 正式交付前只读确认产物目录没有运行中使用者或关联服务；正式文件逐项读回与已核验构建一致。旧产物保留在本次证据的 `previous-package/`、`previous-installer/`。

持久证据：`.diagnostics/packaging/20261109-104330-39bb44e4/`，含原日志/状态、进程退出证据、源码基线、快照证明、公开构建输入、独立/已发布核验及验证复用记录。安装说明和 `artifacts/installer/evidence.json` 已更新为本次发布。未提交或推送。

## 已知限制与待验项

本次不是全量产品或实机接受。既有完整Desktop运行的失败/忽略及后续定向修复仍按原记录保留，不声明完整集合已通过。Release编译保留既有三条 `private_interfaces` 警告及Inno的 `FileCopy` 命名提示。

真实安装、一次性UAC/恢复、卸载、LocalSystem索引器重启/base-plus-delta/空闲CPU与真实搜索、物理选择器/焦点/DPI及其他既有未通过或未运行项目保持开放。原性能和交互验证范围不因包生成扩大。

归档仍 **Phase E — Format expansion and ZIP editing / in_progress**；HZ5、源回收和原范围限定保持。
