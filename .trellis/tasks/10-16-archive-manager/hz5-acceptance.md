# 第7步：HZ5 — 边解压边释放原件空间

2026-10-31。限定实现与自动化/原生进程验收完成，仍为 **Phase E — Format expansion and ZIP editing**。
原第1–6步的限定验收不变，正式包仍为 **Release42**。

## 已交付行为

- 原解压表单增加默认未选中的官方 Checkbox、持续 Small Alert 和独立不可逆确认。
  不保存空间回收为默认设置，不复用 DeleteSource，不因空间不足自动启用。
- 原首页提供“继续空间回收解压…”。输入标记源、原目标和恢复 UUID，以新的 Start/operation/session
  读取持久状态；不是旧进程 Resume。原 FIFO、Job、worker、输出事务、取消及诊断所有者保持。
- 普通单卷、未加密 Stored ZIP，以完整 regular file 为输出/释放单元，在同卷本地 NTFS 打洞。
  只有完整内容、CRC、大小、readback、必要元数据/MOTW、实际发布及保护输出通过，并且 pending
  记录 append/flush/readback 完成后，原生 owner 才修改持有源对象的已批准载荷区间。
- 保护所有头、描述符、中央目录和尾记录。源逻辑长度不变，保留消耗后的稀疏外壳、ADS 标记、
  companion 与输出。不会删除/回收整个原件，也不承诺恢复完整原压缩包或重建剩余归档。
- 新进程核对源/父目录/目标/companion 身份、保留 ZIP 结构和完整计划，再核验所有已消费输出。
  ready 部分文件可经原 CommitCore 完成发布；pending 已打洞但 applied 未记录时按真实分配协调，
  不重复解码或计数。完成收据可由再次启动识别，且不重复追加。

## 实际支持矩阵

| 输入/组合 | 空间回收行为 |
| --- | --- |
| 普通单卷 Stored ZIP，未加密，完整文件/普通目录，同卷本地 NTFS | 支持；完整文件发布后释放载荷内完整稀疏单位 |
| 小文件/空文件 | 可随合格计划输出；区间 `(0,0)` 不修改源。全计划无完整释放单位则拒绝 |
| Deflate、其他 ZIP 方法、加密、ZIP64、SFX、签名/应用容器 | 明确拒绝，不静默改为其他破坏行为 |
| TAR/压缩 TAR、7z/固实块、RAR、独立流、分卷 | 未启用空间回收；普通源保留能力不变 |
| 链接/特殊成员、重叠/歧义区间、间隙/附加尾数据 | 拒绝 |
| 非 NTFS、异卷、稀疏/命名流能力不符 | 拒绝 |
| 初始 sparse/compressed/encrypted/reparse/多硬链接源 | 拒绝作为新回收源 |
| 活跃浏览/预览/拖出/编辑源活动 | 原所有者拒绝冲突；不关闭别人的租约或降低只读保护 |
| 选定解压、Skip/Overwrite/AutoRename、普通 readmission/batch retry | 不授予回收权限 |

第一版边界：最多4096个 regular files、32MiB journal、64KiB record/private body、1024byte marker，
保留 ZIP 结构最多44KiB。释放量按 `max(卷簇,64KiB)` 向内对齐；仍需要当前完整文件的输出空间。

## 持久状态和调用链

[已确认设计](hz5-design.md) · [schema1 线契约](hz5-wire-contract.md) ·
[代码规范](../../spec/nxtools-archive/backend/reclamation.md)。

`ArchiveApplication -> ArchiveJobSession -> ArchiveWorkerRunner -> source_acquire ->`
`ArchiveReclamationSession / 原 ArchiveWorkerOutputBridge -> ArchiveOutputSession.CommitCore ->`
`pending -> exact-handle sparse punch -> applied -> 实际 worker/pump/output cleanup -> Job terminal`。

Rust `reclamation.rs` 拥有资格、布局、有限日志 replay 和下一单元决策；C# 拥有源 RW capability、
保护父链、输出/日志句柄及 Windows API；GPUI 仅呈现和显式选择。普通 `ArchiveSourceLease` 仍只读。
源活动预约覆盖 deferred virtual content 和 Modify 的 FILE handoff；浏览快照通过下一 GPUI turn 失效。

一个源父目录内 stable companion，源 ADS `:NxTools.HZ5` 绑定 recovery/source/companion identity。
Start 的 info 仅准入几何/目标，首个资格化 plan entry 创建 companion；完整目标计划在原输出 owner 通过已有文件/源身份冲突检查后才 sealed，并持久绑定源 ADS 标记。拒绝的未 sealed 原件没有消耗标记，源字节保持完整。
记录为有界 `u32LE长度 + JSON(schema/sequence/kind/value) + u32LE长度`，每次 append、flush、完整读回后确认。
完整损坏/未知 kind/不连续序号拒绝；不完整末追加仅在既有完整 pending 下允许协调，不丢弃 pending。

诊断沿用 operation/recovery correlation，记录 accepted、checkpoint、pending/applied、净分配下降、终态、
耗时和原生错误码，不记录内容/密码/路径/原始日志。普通 write/flush 的原生原因也保留至 Job 终态。

## 真实进程与失败矩阵

最终矩阵 **61 PASS / 0 FAIL / 0 SKIP**：37项完成剩余解压，20项明确保持性拒绝，4项普通源保护对照。
使用原 HZ4 精确 Popen handle/PID/creation/image/parent 所有权和原 worker Job；只终止自有测试宿主，
观察原 worker 退出。续作是全新进程和新 operation/session，不复用旧运行态。

| 边界/故障 | 实际结果 |
| --- | --- |
| 输出 flush、MOTW、metadata、ready 前后 | 未形成 ready 的部分文件保持并拒绝；有效 ready 经原发布 owner 续作 |
| 单次 native publish 前后 | 原输出身份/内容/元数据核验后完成 pending 和释放 |
| pending append/flush/readback 前后 | 新进程读取真实完整记录，无未经证明的释放 |
| 三个文件各自 pending、打洞前后、applied flush 后 | 新进程完成剩余文件，所有输出与独立 Python 字节 oracle 相同 |
| 打洞成功、applied 前注入112；torn applied 尾记录 | 保留完整 pending；按真实 allocated ranges 协调，不重复 applied/count |
| 完成记录及清理前后；再次识别完成收据 | 保留源外壳/authority/输出，无重复记录 |
| 重复中断 | 第二次在下一文件释放后中断，第三个新进程完成 |
| 原 Job Cancel：打洞前/后 | 终态保持 Cancelled，停止后续单元；新的 Start 继续剩余解压 |
| 输出/日志故障112/1117、原生修改前失败5 | 注入点确实被访问，保留 owning native code，不批准下次释放 |
| 输出缺失/同字节替换/内容损坏 | 拒绝且源不再推进 |
| marker/journal 缺失或损坏；源或目标根被替换 | 拒绝，保持替换对象和已存文件 |
| 原目录正常解压；成员与原件同名；已有目标文件 | 正常完成或全计划前置拒绝；冲突时零输出、零释放，原件与既有目标保持完整 |
| 普通解压 flush/publish 中断、flush112、正常完成 | 源字节/身份/长度/mtime不变；新进程旧 Resume 仍 stale-operation 拒绝 |
| 无完整释放单元、Deflate | 无输出、无 marker/companion、无源变更的明确拒绝 |

最终矩阵原始证据：
`C:\Users\Administrator\.pi-desktop\scratch\da53445c-cf27-495f-b4a4-46ef35871e5c\hz5-final61-286f17b8-8205-41b1-bf1e-625b10f13f5b`。
其中 identities/results/accounting 记录冻结二进制与具体 `hz5-<GUID>` 子树；子树保留命令、终态、
关联诊断、精确进程 receipt、allocation-samples 及实际 source/output/journal。
生产 `flush_file` 阶段另有同一驱动的普通故障对照通过；TERM01 覆盖普通 write/flush 原因优先于泛化 worker 帧。

## 实际空间数据

固定夹具：三个4MiB Stored 文件和一个空目录，载荷合计12,582,912字节；源逻辑长度12,583,306字节始终不变。
以句柄 `FileStandardInfo.AllocationSize`、源 allocated ranges、独立 Python 查询和同卷 free-space 观察交叉记录。
峰值统计 source + output/partial + journal + marker 数据流分配；固定测试二进制和 evidence 不计入功能峰值。
采样包括每个完整输出 flush 前、publish 后、打洞前后、完成/cleanup 以及独立5ms轮询。

| 场景 | 源分配前 → 后（bytes） | 实际净释放（bytes） | 观测峰值额外分配（bytes） |
| --- | ---: | ---: | ---: |
| HZ5，CopyMotw关闭 | 12,587,008 → 262,144 | 12,324,864 | 4,468,960 |
| HZ5，实际 Zone.Identifier | 12,587,008 → 262,144 | 12,324,864 | 5,255,392 |
| HZ5，原压缩包所在目录 | 12,587,008 → 262,144 | 12,324,864 | 4,980,960 |
| 普通完整解压对照 | 12,587,008 → 12,587,008 | 0 | 13,434,880 |

HZ5关闭 CopyMotw 的实际观测峰值额外分配为4.26MiB，对照12.81MiB。完成后 journal分配8192bytes。
首次 sparse 转换的保留尾部开销计入净收益，`released_bytes` 等于源实际分配前后净下降，
不是释放区间名义长度。HZ5普通完成案例的同卷 free bytes 为10,835,472,384 → 10,834,571,264；
卷上其他写入和保留输出也影响 free bytes，不将卷差值单独当作该操作净回收量。

## 定向验证

- Rust：protocol2、journal/layout5、真实 worker协议4、普通 worker wire18、metadata wire1通过。
  CLI/Archive/Desktop相关 all-targets 编译检查通过；严格 Archive/Protocol all-targets Clippy通过。
- Desktop：Archive相关330项串行通过，另有首页真实官方续作按钮可达测试通过。
  覆盖未选中默认、不可逆确认前不准入、新Start绕过readmission/retry、延迟快照退休和保留语义。
- C#：Debug构建零警告零错误；8项相关 fixture 与4项自有 native fixture通过。
  元数据/MOTW/ready/不完整部分文件/源活动/持久 framing 均在原 owner验证。
- Release C#测试宿主构建零警告零错误，严格wire与私有能力门禁2项通过；进程故障入口在路径/能力读取前
  返回 `release_fixture_disabled`，无Debug钩子权限。未执行发布打包。

## 已知限制与未验证项

- 只能继续剩余完整文件解压。没有 durable ready 的不完整部分文件会保持并明确拒绝，不能承诺任意字节续解。
  缺失/替换/损坏的 indispensable 输出、源或authority同样拒绝，不清理也不能从已消耗载荷重建数据。
- 112/1117/5为原生边界故障注入，不是实际填满物理卷或制造设备损坏。
  当前证据是真实进程终止/新进程续作，不是VM断电、硬件断电或文件系统持久性验收。
- 峰值为该固定夹具的生命周期/5ms观测，非所有负载/设备的空间上限或统计性能承诺。
- GPUI自动化与源码/API核对不代表真实Desktop视觉、物理输入、混合DPI或用户验收。
- 一项旧跨7z/RAR源保护可选测试因缺少显式codec夹具跳过；不计为通过。新的普通ZIP真实进程对照已通过。
- 未打包、安装、改服务/系统设置、处理用户原件、提交或推送；Release42及原1–6步验收不变。
