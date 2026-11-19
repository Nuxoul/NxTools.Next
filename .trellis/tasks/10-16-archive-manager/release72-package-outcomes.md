# 正式 Release72 — 产品与浏览器扩展0.7.1

2026-11-14。原 `scripts/package-local.ps1 -SkipVerification` 完成实际Release编译、原发布身份签名和正常Inno安装器生成。原入口及package.ps1均正常退出0，用时596.65秒。独立核验与交付读回通过。仅交付正常安装器，未生成外层完整分发ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.1 / 0.7.1 |
| 发布序号 | 72 |
| Release ID | `e0a48a33-6f04-4300-aff7-f1dc48f66804` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.1.exe` |
| 大小 | 29,184,982 字节 / 27.83 MiB |
| SHA256 | `D2F218A22AC0D87CF6810297FE888F70332BA3B7C109CB976B01E13ACAA21CB6` |
| 内部发布签名 | 原公开信任锚，RSA3072-PSS/SHA256，通过 |
| Manifest SHA256 | `f2d7b35fb5c16e52b70c592bf031cf1d3fa67ad5914e53b95c58fc6d0799bf18` |
| 签名成员 / 精确散装文件集合 | 107 / 109，全部匹配 |
| 实际构建输出 / 扩展源文件 | 10 / 94，与包内匹配 |
| 归档批准 / helper Desktop绑定 | 5项批准及绑定匹配 |
| 固定构建输入 | 4,658文件，复制前后及构建期间内容一致 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

## 版本与身份

- Rust工作区及14个第一方锁文件包、C#程序集、正常/维护安装器、浏览器扩展清单统一为0.7.1。第三方依赖、协议和schema版本不变。
- Desktop Windows资源版本直接派生自Cargo版本；两个原图标组保持。两份Inno定义显式设置文件版本及产品版本。
- 实际Windows数字文件版本读回：Desktop、PlatformHost、ShellSelectionHelper EXE/DLL和正常安装器均为0.7.1.0，ProductVersion均为0.7.1。
- 扩展清单仅修改version；公钥/ID、权限、脚本顺序和行为保持。包内清单与冻结源码完全一致。
- 原发布公钥、AppId及安装信任边界不变。助手SHA256为 `61BF530E71493B4715C5174FBF9052988A7B73BBCCF87B85F6178B2353688EA1`。

## 验证

- 构建使用固定源码、独立Release缓存和独立输出，不与工作树共享可写构建目录；独立缓存由本会话已结束构建的私有副本转移，原输出及证据保留。未使用硬链接或目录联接，未结束无关进程。
- 实际完成Rust产品、归档worker及C#宿主Release编译、原发布签名和Inno生成。打包进程4676及入口包装器正常结束；没有额外构建。
- 本轮身份30项、原生退出47项，各自在x86/x64通过；16个安装器隔离场景、5个BrowserHost替换/回滚场景通过。
- 当前版本一致性5项、打包入口12项、源码材料5项、构建树清理定向1项、扩展Node测试25项通过。C#浏览器签名fixture精确选择7项，RUN7/PASS7/FAIL0/SKIP0/not_run0；测试宿主构建0警告/0错误。
- 独立核验原签名、107成员、精确109文件、10实际输出、94扩展源、5项批准及helper绑定。两份installer-support源与Inno输入一致。
- 对应材料与当前126个源文件、3个crate清单、147个依赖及许可证匹配，保留在 `artifacts/installer/third_party/archive-worker/`，不扩展受信安装白名单。
- 交付前只读确认默认产物目录无运行中程序、安装器或关联服务；发布逐文件读回与已验证构建一致。原Release70备份保留在本轮 `previous-package/` 和 `previous-installer/`。
- 规范合同、GPUI组件目录、发布合同及Markdown编辑器合同源文档检查通过；不作为物理UI接受。

持久证据：`.diagnostics/packaging/20261114-094143-1ec46b84/`，含原日志/状态、源码基线、快照/缓存证明、公开构建输入、版本读回、版本测试、独立/发布核验和验证复用记录。`artifacts/installer/INSTALL.md` 与 `evidence.json` 对应本次发布。历史保留包的交付状态独立记录，不作为当前产物。

## 已知限制与待验项

本次未重跑产品全量Rust/C#，按 `verification-reuse.json` 复用已有定向验证；历史结果不计为本轮新执行。当前对应源码材料的独立离线重构建未执行。未执行真实安装、卸载、产品启动、服务操作或注册表修改；真实安装/UAC/恢复、LocalSystem索引器重启/base-plus-delta/空闲CPU及实际搜索、物理UI/焦点/DPI和既有未通过或未运行项目保持原状态。

归档仍 **Phase E — Format expansion and ZIP editing / in_progress**；HZ5、源回收及原范围限定保持。B4实现未恢复。未提交或推送。
