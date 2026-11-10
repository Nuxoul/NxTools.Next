# 正式 Release56 — 更新组件缺失安装修复

2026-11-06。当前源码通过原 `scripts/package-local.ps1 -SkipVerification` 完成实际 Release 构建、原发布身份签名、正常 Inno 安装器和完整分发 ZIP 生成。统一入口与 package.ps1 均退出0，独立签名/身份/集合及分发材料核验通过。未安装、启动产品、恢复真实服务、修改注册表或提交推送。

## 产物

| 项目 | 结果 |
| --- | --- |
| 版本 / 发布序号 | 0.7.0 / 56 |
| Release ID | `666ec4b2-cf23-4e5c-9950-ec4f055bdd7a` |
| 正常安装器 | `artifacts/installer/NxTools.Next-setup-0.7.0.exe` |
| 安装器大小 | 27,957,621 字节 / 26.66 MiB |
| 安装器 SHA256 | `47E6F59B5E9F713109F2502BE4678076909C69C62383C75C71C97114651633E3` |
| 完整分发 ZIP | `artifacts/installer/NxTools.Next-release-0.7.0.zip` |
| 分发 ZIP 大小 | 62,514,029 字节 / 59.62 MiB |
| 分发 ZIP SHA256 | `2F90BF2280AB272850B028CD41BA8A4A94F29B03AB356EB98351EB41A2358F73` |
| 内部发布签名 | 原公开信任锚，RSA3072-PSS/SHA256 独立核验通过 |
| Manifest SHA256 | `102418d1b426888496d0b4cf786b6689906b0b1dc8143f87f62c3e35a40ab29c` |
| 签名成员 / 精确文件集合 | 78 / 80，全部匹配 |
| 实际构建输出 / 扩展源文件 | 10 / 65，与包内匹配 |
| 归档批准 / helper Desktop 绑定 | 5 项批准及绑定匹配 |
| 安装器 / BrowserHost 替换回滚场景 | 16 / 5，通过 |
| 源码基线 | 1,298 项，打包期间无新增/删除/内容漂移 |
| Windows Authenticode | `NotSigned`，与内部签名区分 |

包内 Host 含新的显式修复命令与初始部署状态完成事件。当前准备助手 SHA256 为 `38F0A7D24A22CBBC29F7163BE68B69245437853756FFF1D87CBBAE1ED14F9001`，新增分支不改变原进程归属与同句柄退出规则。

## 修复范围

[缺失更新组件修复](installer-missing-bootstrap-repair.md)区分真正首次安装、兼容更新组件、已确认同发布方且整个 Updater 目录缺失和其他失败。首次安装直接跳过；只有明确用户确认/UAC和完整同公钥签名包、固定服务路径/账户/登记哈希、真实缺失根目录校验通过，才执行受控恢复。存留目录、事务/恢复记录、权限错误或外来登记不会被清空。

原登记初始化/SCM/部署锁被复用。初始恢复快照记录 DeploymentPending，修复调用先释放父部署锁，再通过原 LocalSystem broker 实际部署并验证受保护程序。部署失败保留待部署事实；同版本请求不能在初始部署未完成时假称 already-current。完成登记及实际 bootstrap 验证后写原协议标记，正常安装重试识别兼容状态。原普通更新事务、回滚、防降级和签名身份规则保持。

探测日志现在在早退前保留真实 started/exit、固定阶段/原因、异常类型、原生错误和关联ID。主文件安装准备在任何子探测之前建立关联ID。用户取消确认/UAC、校验拒绝、修复失败均不进入正常文件安装。

## 验证

- 真正 PowerShell 探测脚本入口、原助手派发及既有决策：x86/x64 共49断言通过。
- C# 最终构建0warnings/0errors；精确 LIST/RUN13，PASS13/SKIP0/FAIL0/not_run0。真实签名临时包、临时 bootstrap、原登记状态机和原事务被执行；SCM/注册表/原生部署效果注入到自有目录。
- 新旧受保护文件、初始部署待定、失败重试、同版本幂等、后续升级、降级/变更拒绝及普通维护/回滚保留通过。缺协议标记回归保留先红后绿证据。
- 原准备诊断8、进程身份22、升级进程18检查通过。一次只读定界复核确认的初始部署问题由上述原事务回归保护，未开展重复广审。
- 正式打包再次运行16个实际 Inno 回调隔离场景及5个 BrowserHost 回滚场景，全部通过。
- spec契约126文档/14owner、PowerShell语法、源码分发回归4和定向差异检查通过。产品全量 Rust/C#、大数量、HZ5和物理矩阵未重跑，原定向结果复用并逐项记录。

分发 ZIP 的12项文件集合与内容匹配，里面的安装器与本次产物完全相同；WIM/LGPL 源码与许可证材料保留，编译输入与已通过离线重构建的材料一致。两个主产物的相邻 SHA256 文件匹配，未含 PEM。

持久证据：`.diagnostics/packaging/20261106-232739-c80ca6a4/`，含日志、状态、旧包备份、`independent-verification.json`、`verification-reuse.json` 和 `installer-repair/`。控制器及源码基线：`D:\.pi-desktop\data\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\formal-repack-20261106-232738`。历史包和失败占号记录保留。

## 已知限制与待验项

本次没有执行真实修复安装。新用户/报告机器实际UAC、恢复服务、完成安装和安装后签名文件读回尚待确认，不能以隔离测试声明该机器已经恢复。Log003只证明探测失败；具体缺失状态另由本机只读原入口和原生错误3复现，不追溯推断日志未记录的内部异常。

仅接受整个 Updater 目录缺失且原登记归属明确的修复；部分残留目录或未决恢复记录保持拒绝和证据，不自动擦除。普通产品/索引器安装态CPU、重启恢复、真实搜索及物理UI/DPI验收保持原开放范围。

归档仍 **Phase E — Format expansion and ZIP editing / in_progress**。原失败/未运行项、PlatformHost内存原因未定及回收/HZ5限定不因安装器修复扩大。检查点仅prepend，原305826bytes保持精确suffix。
