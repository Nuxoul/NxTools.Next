# Windows 本机打包交接（人和 AI 共用）

**打包先读本文件，不依赖聊天记录，也不要从 scratch 猜密钥或复制旧命令中的发布序号。**
正式实现仍为 `scripts/package.ps1`；本机统一入口为 `scripts/package-local.ps1`。

安装失败先读 [运行中升级故障与真实验收记录](.trellis/tasks/10-16-archive-manager/installer41-running-upgrade-outcomes.md)
和 [原生进程识别契约](.trellis/spec/platformhost/installer-preparation.md)。必须读取用户本次实际 correlation 日志，
不能用全量测试数、历史同一错误码的修复或安装器编译成功替代真实安装结果。
空闲状态安装成功不代表运行中升级成功。更改进程关闭边界后，应在另行明确授权下验证已安装 Desktop 正在运行时的升级；
记录实际 PID、原生 API、提权状态及安装后文件读回。UAC取消不算 helper 执行失败或安装成功，不自动重复。
首次安装/新账号报“无法检查更新组件”时，先核对本次 `bootstrap-detection.jsonl` 与 Setup 日志。
无更新服务应直接跳过；固定同公钥服务登记存在但整个 Updater 目录缺失时，安装器可在明确确认/UAC后
调用限定恢复入口。不得把访问拒绝或仍有恢复记录的目录当成缺失，不手动删服务/目录来强行安装。
源码/隔离验证与真实安装修复接受分开，见 [缺失更新组件修复](.trellis/tasks/10-16-archive-manager/installer-missing-bootstrap-repair.md)。
安装器在任何升级准备或文件替换前试运行包内 PlatformHost 的 `--check-runtime` 无副作用入口。
目标电脑必须具备 Microsoft .NET 8 桌面运行时（Windows x64，包含 Core Runtime）；单文件发布不自带运行时。
缺失或无法加载时阻止安装，提示具体依赖与微软下载页，并仅在用户同意后打开浏览器；静默安装不打开网页。
官网：`https://dotnet.microsoft.com/zh-cn/download/dotnet/8.0`，选择 **.NET Desktop Runtime / Windows x64**。
`0x80008096` 是兼容框架缺失，不是发布签名失败。普通与维护安装器共用此检查；WebView2 仅影响 PDF 导出，不作为全局安装门槛。

## 1. 直接使用

在仓库根目录执行：

```powershell
# 只检查路径、工具链、内存、构建占用和下一序号；不读取私钥内容、不占号、不构建
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\package-local.ps1 -CheckOnly

# 正常 Release 打包：自动定位现有 PEM，计算新序号，运行原验证与打包流程
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\package-local.ps1

# 仅在已有相关验证、明确允许复用时使用；仍会重新构建 Release 和安装器
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\package-local.ps1 -SkipVerification

# 仅显式需要完整分发 ZIP 时追加；普通打包不生成外层 ZIP
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\package-local.ps1 -SkipVerification -CreateDistributionArchive
```
同一产品源码已有相关验收、用户明确要求只重新打包或不重复全量测试时，使用上述 `-SkipVerification`
复用既有结果；不要再调用 `build.ps1` 重跑全量集合。交付中记录验证出处、源码一致性和复用标记，
仍执行实际 Release 构建、原发布签名及安装器必要检查。复用不改变正式包身份，也不等于 unsigned/staging 包。


`-CheckOnly` 的 `Ready=false` 表示当前构建占用或提交内存不足；不是可以跳过的门禁。
检查成功只证明材料存在和前置条件，不证明 PEM 有效、签名成功或产品通过验收。
不需要管理员权限来运行打包入口；不要自行安装服务、启动产品或运行安装器。

## 2. 本机材料在哪里

当前工作机（2026-10-26 核对，材料已存在）：

| 材料 | 路径 / 作用 |
| --- | --- |
| 发布私钥 PEM | `D:\NxTools-private-release-keys\nxtools-next-release.pem` |
| 原序号账本 | `D:\NxTools-private-release-keys\nxtools-next-release.pem.release-sequence.json` |
| 序号锁 | 同账本路径加 `.lock`，由原预约脚本管理，不要删除 |
| 公共信任锚 | `csharp/NxTools.PlatformHost/Generated/UpdateTrust.g.cs`，只含公开 SPKI，不是私钥 |
| 原发布预约器 | `scripts/reserve-release-sequence.ps1` |
| 签名器 | `scripts/write-trusted-release-manifest.ps1` → C# ReleaseSigning |
| 原生发布 / 身份绑定 | `scripts/publish-release-hosts.ps1` |
| 打包日志、状态及上个包备份 | `.diagnostics/packaging/<时间-唯一ID>/`（Git 忽略） |

默认私钥路径按 `$env:USERPROFILE` 推导；其他账号／机器使用以下覆盖方式：

```powershell
$env:NXTOOLS_RELEASE_SIGNING_KEY = 'E:\secure\nxtools-next-release.pem'
.\scripts\package-local.ps1 -CheckOnly
# 或显式传入
.\scripts\package-local.ps1 -SigningKeyPath 'E:\secure\nxtools-next-release.pem' -CheckOnly
```

优先级：显式 `-SigningKeyPath` > `NXTOOLS_RELEASE_SIGNING_KEY` > 当前用户默认目录。
账本必须与 PEM 配套，路径固定为 `<PEM完整路径>.release-sequence.json`。

**私钥内容不得写入文档、Git、日志、聊天或安装包。** 它已经在本机持久目录，不在 session scratch。
不要新生成密钥替代现有发布身份，不要使用 `-LegacyUnsigned` 绕过缺失材料。
换机器时通过安全离线方式迁移原 PEM 和原账本、限制访问权限；本文件不是密钥备份。
若缺失，停止并恢复材料，不能重置计数或靠生成新 PEM 继续升级原受信安装。

当前公开锚 SHA256（不是秘密）：
`a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68`

## 3. 序号怎么取

统一入口读取原 PEM 账本和 `artifacts/win-x64/NxTools.release.json`：
`next = max(ledger.sequence, package.release_sequence) + 1`。
**检查模式不预约**。真正打包仍由原预约器持锁、持久化并拒绝重复／倒退序号。
并发预约冲突应停止检查，不自动重试构建、不清理账本。

2026-10-26 最近已完成序号29，显示版本0.7.0；这只是历史记录，不是下一次要硬编码的值。
失败可能已经占号，允许跳号，禁止复用。不得修改 ledger 回退来“重试同一版本”。

底层手动入口仍可用，但必须先算好新序号：

```powershell
.\scripts\package.ps1 -Configuration Release `
  -SigningKeyPath 'D:\NxTools-private-release-keys\nxtools-next-release.pem' `
  -ReleaseSequence <严格递增的新序号>
```

## 4. 工具与内存

- Windows 10/11、项目要求的 Rust 1.96/MSVC 与 .NET 8 SDK、Git。
- Windows SDK x64 `fxc.exe`；可用 `GPUI_FXC_PATH` 覆盖，否则走原工具查找。
- Inno Setup 6 `ISCC.exe`，标准 Program Files 安装位置；缺失时统一入口拒绝，不能把只有散装目录称为安装包。
- 提交内存余量至少6GiB（不是仅物理空闲内存），不擅自降低门槛。
- 每个直接／后台 Rust 命令都需要：

```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
```

入口继承原低内存初始化，检查共享 Cargo/rustc/clippy/ISCC 占用。
自行等空闲，不要求其他 AI 暂停，不结束无关进程，不开第二套构建。
原 package.ps1 只按精确包路径处理运行中进程／匹配服务；不能按名称杀其他安装副本。
长任务要给足时间，或使用后台任务后读日志；超时先查存活进程，不直接重启。

## 5. 输出、日志、核验

- 安装包：`artifacts/installer/NxTools.Next-setup-0.7.3.exe`
- 散装包：`artifacts/win-x64/`
- 签名清单：`NxTools.release.json` / `NxTools.release.sig`
- 对应源码材料：`artifacts/installer/third_party/archive-worker/` 下的 LGPL 许可证、对应源码及离线重构建说明，仍由原入口生成；不改变受信任安装清单或生产身份校验。Python 要求 3.11+；材料生成失败不得交付缺材料的包。
- 外层完整分发 ZIP 默认不生成。仅显式传入 `-CreateDistributionArchive` 时生成 `artifacts/installer/NxTools.Next-release-0.7.3.zip`，包含正常安装器与上述材料；对外再分发时同时提供对应源码和许可证。普通重新打包只交付安装器，不把历史 ZIP 当成本次产物。
- SHA256：安装包旁 `.sha256`
- 统一入口的本地 `status.json`：序号、实际成功／失败、安装包哈希、大小、文件核验计数及 Authenticode。
- 同目录 `package.log`（stdout）、`package.stderr.log`（原生 stderr）和 `previous-package` / `previous-installer` 保留原始过程和旧包。
  Cargo 的正常编译进度写入 stderr 不代表失败；入口等待真实子进程退出，以 `package_exit_code` 判断，并继续执行签名清单／哈希核验。

原签名器在打包时使用生产验证器验证签名；统一入口额外逐项核对清单文件大小／SHA256，
并写安装包 SHA256。不能把 Authenticode `NotSigned` 与内部发布签名混为一谈。

产品发布版本0.7.3统一记录在Rust工作区、C#程序集、正常/维护安装器与浏览器扩展清单；
桌面EXE文件版本由Cargo版本派生。`python tests/architecture/product-version.tests.py` 检查这些当前定义、
第一方锁文件版本和发布文件名的一致性；构建后仍需读回实际EXE与包内扩展版本。扩展ID/公钥和协议版本不随此更新改变。
包装器失败后可能留下不完整输出：不要安装，读本次日志；备份只用于恢复与证据，不能绕过防降级。

原安装器会跑隔离场景；其中预期取消／失败场景的非零退出码不等于总体失败，检查 `pass=True` 和最终脚本退出码。
成功不代表已安装、UI通过、服务通过或用户验收。`-SkipVerification` 必须在交付中披露，不能称全量测试通过。

验证配置明确分工：正常 Release 流程实际构建 Release Rust/宿主，并运行 Release 私有测试能力禁用门禁；
随后用同一次验证构建的匹配 Debug worker/宿主执行无名称过滤的完整 C# `--fixtures` 集合。
低阈值 Risk/暂停场景依赖 DEBUG 注入，不能让 Release 注入生效来强行通过测试。
两个配置的测试宿主均使用当前实际二进制生成的身份清单，不使用默认空发布身份；调用者的 fixture 环境在退出时恢复。
结果仍分别披露 PASS/SKIP/FAIL/not_run；原有可选环境导致的 SKIP 不计作通过。参数回归为 `tests/architecture/build-host-bindings.tests.py`。

临时 .NET 构建树由创建该次精确路径的脚本在最外层 `finally` 删除：`build.ps1` 的 `vh-*`、
Release fixture 验证的 `vf-*`、`package.ps1` 的 `package-build-*`，成功与失败相同。
有 `PI_SCRATCH_DIR` 时使用该目录，否则使用系统 Temp；不扫描或删除其他调用的临时树。
`publish-release-hosts.ps1` 借用调用方目录，生成输入和发布文件一直保留到测试构建／包复制消费结束。
fixture 默认证据保留在 `.diagnostics/verification/fixtures-<唯一ID>/`，自定义证据目录继续生效，
退出时恢复调用方 fixture 环境。IsolatedStaging 的公开生成输入保留在输出旁 `.generated-inputs/`，
本次 managed Host DLL 另保留为 `<输出>.TEST-PlatformHost.dll`，供原 staging 身份测试读取实际程序集。
`.build-inputs.json` 的 `generated_directory` / `host_assembly` 只引用这些保留输入，不引用已删除的构建树；
原 TEST 公钥旁文件保留，私钥不复制。旁文件均为验证证据，不加入生产安装成员。
这些 staging 材料仍不具备生产安装／发布身份。脚本回归使用工具桩，不授权真实构建或服务操作。

`artifacts/installer/INSTALL.md` / `evidence.json` 是人工交付摘要；统一入口不自动更新这两份文字，
后续 AI 应按本次 `status.json` 更新，不能沿用旧序号／旧哈希。

## 6. 常见阻塞

| 症状 | 正确处理 |
| --- | --- |
| 找不到 PEM / 账本 | 查看第2节和环境变量；恢复原配套材料，不造新密钥、不删账本 |
| 序号不递增 | 重新检查账本与包清单；保留失败占号，不重置 |
| Cargo 正忙 | 等原任务结束；不杀、不复制构建 |
| 提交内存不足 | 等压力下降／用户调整分页文件；不做全系统内存清理或擅自降门槛 |
| 安装器已生成但包装状态停在 Building | `Start-Process -Wait` 等待整个后代进程树，可能包含常驻 C# 编译器。先核对本次打包子进程的真实退出状态；不能以安装器存在推断成功，不结束共享编译器或重复构建。若需独立收尾，保留原状态、控制器结果和原打包进程退出证据，按原规则完成签名、成员和安装器核验后再交付 |
| 编译 / lint 失败 | 保留具体错误和现有改动；不屏蔽或修无关代码来伪造通过 |
| 签名 / 身份验证失败 | 停止发布，保留日志；不 unsigned fallback、不单换二进制 |

更详细的可信升级机制见 [release operations](.trellis/tasks/10-01-one-time-uac-trusted-updates/release-operations.md)，
通用构建说明见 [building](docs/building.md)。打包不自动授权安装、服务迁移或真实系统验收。
