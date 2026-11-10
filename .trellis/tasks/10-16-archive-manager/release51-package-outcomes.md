# 正式 Release51 安装包

2026-11-05。当前源码经原 `scripts/package-local.ps1 -SkipVerification` 重新完成 Release 编译、原发布身份签名和正常 Inno 安装器生成。原 package.ps1 子进程退出码 0，独立产物核验通过。未安装、启动产品、部署保护副本、重启服务或提交推送。

## 产物核验

| 项目 | 结果 |
| --- | --- |
| 版本 / 发布序号 | 0.7.0 / 51 |
| Release ID | `12273a05-69c8-47e3-b0cb-52b0696371a9` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.0.exe` |
| 大小 | 27,525,021 字节 / 26.25 MiB |
| SHA256 | `7EB60377AF56218426ED94CB7DCEE3E203A58868C61BD7022F80FE6C1F14FBFF` |
| 内部签名 | 原公开信任锚，RSA3072-PSS/SHA256 独立核验通过 |
| Manifest SHA256 | `a78fd85e77bb838313adc0c601a1f390fe30eaa259e478f1204dad15967657b7` |
| 签名成员 / 精确文件集合 | 78 / 80，全部一致 |
| 实际构建输出 / 扩展源文件 | 10 / 65，与包内一致 |
| 归档批准 / helper Desktop 绑定 | 5 项批准及绑定一致 |
| 安装器 / BrowserHost 替换隔离场景 | 11 / 5 通过 |
| 源码基线 | 1,273 项，打包期间无新增、删除或内容变化 |
| Windows Authenticode | `NotSigned`，与内部发布签名区分 |

安装准备 helper 保持 `4CDFEAC77D33715CD6800AD27CDCC3D60F58E05198500FEB8CECAAD36A76FC1A`。旧包与历史记录保留；本次使用新发布序号和 Release ID。

持久证据：`.diagnostics/packaging/20261105-233745-a005bdb4/`，含原日志、旧包备份、实际进程退出事实、`independent-verification.json` 及 `verification-reuse.json`。控制器与源码基线：`D:\.pi-desktop\data\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\formal-repack-20261105-233743`。构建使用 jobs1、incremental0、codegen-units256 和 C# `UseSharedCompilation=false`；构建后无 Cargo/rustc/clippy/ISCC 活动。

## 包含范围与验证复用

- [速启搜索头与窗口几何](../11-05-quick-launch-header-geometry/outcomes.md)：更大的搜索头、稳定中间批次视口及完成边界尺寸更新。原定向 Quick Launch 190 项、Clippy、工作区检查和隔离明暗预览证据保留。
- [新建下载精简输入](../11-05-download-intake-compact/outcomes.md)：紧凑 URL 输入、相邻继续/取消及首次打开的单次剪贴板链接预填。原 downloads 243 项、主题 1 项、协议 108 项、剪贴板 48 项及 C# 2 项结果保留，不累加重复子集。
- [归档工作量减少](../11-02-archive-workflow-optimization/bandizip-gap-implementation.md)及后续 [Root 拖出与读取优化](../11-02-archive-workflow-optimization/root-drop-and-read-passes.md)：保留当前受保护来源、原生输出和清理所有者。后续 Root/读取记录是当前实现范围，前一轮性能表保留其历史代码与测量归属，不直接推广为本包性能。
- [直接添加与统一通知](../11-02-archive-workflow-optimization/incoming-add-independent-acceptance.md)及原速启排序、拖出修复、其他 OPT 与 HZ5 限定证据保留。

本次仅执行构建、签封及安装器必要检查，没有重跑全量 Rust/C#、HZ5、OPT 或各任务定向测试。复用来源完整列于 `verification-reuse.json` 与安装器 `evidence.json`；源码基线只证明打包期间一致性。

## 已知限制与待验项

- 索引器安装态发布门禁未执行。须另行授权部署匹配原子包后运行 `tests/performance/indexer-idle-cpu.ps1 -RestartService -AssertRegression`，再运行 `tests/performance/indexer-service-smoke.ps1 -Query 'NxTools.Indexer.exe' -Limit 20 -TimeoutMs 10000`。
- 原完整工作区的下载菜单文本断言失败未计作通过；当前速启记录的预览隔离聚合门禁未通过，原生 wire/TAR 等显式夹具相关未运行项保持各任务记录，未宣称全量绿色。
- Root 拖出以受控 CF_HDROP 接收者测试验证，未运行物理 Explorer/DOpus 手势。目标位置不明时返回 `drop_location_unknown`，不盲目解压。混合 DPI、物理 IME、用户视觉和实际安装状态仍待各自验收。
- 不把旧测量、Debug/合成夹具或受控接收者结果当作当前包普遍性能承诺。归档仍 **Phase E — Format expansion and ZIP editing / in_progress**，整体与原开放门禁不因打包关闭。源回收和 HZ5 的默认关闭、限定格式/卷、不可逆确认及继续剩余解压语义不扩大。

检查点仅 prepend，原 301,283 字节保持精确 suffix。
