# Step5 — HZ3 批量任务汇总（实现前路线）

2026-10-29。用户原七步第五项，**Phase E — Format expansion and ZIP editing**。已接受第四步及EIC修复不重做；只做HZ3，不做HZ4/HZ5/持久历史/新调度器/新窗口controller/stop-on-error。当前opening checkpoint194042bytes SHA2565CBED44CE726DE8CFD4B76E0965765A4F236298DF1C8B9073896E881EDC4F248，完整保存于session scratch/hz3/checkpoint-before.bin。只prepend且证明整个opening exact suffix。原验收/失败/并行dirty保护。

## Reference and actual gaps

先读batch-summary.md/source-manifest并沿D:\Code\hatchzip-main实际main.rs233distinct/501extract_all/578summary、runner69give_back/103start_queue/115step/136next/219close、task_window39job_for/77build、work228extract和several.rs全测试定义。参考串行/遇错停止/primarypathHashSet不是NxTools政策，不搬Runner/clock/retryjournal/destructiveconsent。NxTools只借逐archive结果与notstarted表达，保留实际2running/32queued/2GiBaggregate、FIFO/head-overlap、cleanupheldslot、原finite readmission和其他batch公平性。未执行参考代码。

原jobs.rs submit_form循环additional_requests、submit_batch预检后逐项submit_job/defer_pump；pump_jobs原queue.next；job_notification currentattempt/session/generation；readmission nativeattempt与logicaljob分离；job_terminal once/queue.finish(cleanup)/retired64；原show_job/JobProgressView/locate_job_result。缺batch身份/成员投影/入口与显式安全通用retry。submit_batch早return不能漏defer复位，已接收/未开始必须如实。

## Smallest implementation boundary

- 在原Jobs/JobRecord加private bounded batch metadata（batch UUID != logical UUID != native attempt UUID），不新增调度器或复制执行状态。batch成员引用logicalid，状态每次从原record投影；attempt和finite readmission不增member/done。排队未开始与取消未开始可同时表达，cleanupunknown/partialpublish突出且不能报整体成功。archivejob和file counts分别展示，不造aggregate byte percent。
- 原multi-request form和launch submit_batch绑定batch，单job维持原detail；原JobProgressView同surface增加batchsummary模式/入口，成员详情仍show_job、控制仍control_job、结果仍locate_job_result，不信任summary新路径。无新HWND/windowcontroller。批次关窗只是presentation，不取消任务。
- 提交预检保持原atomic structural/capacity/memory语义。同步逐项异常确保defer复位、原pump继续，保存真正accepted与notstarted/refusalcounts；不会停止已运行兄弟/阻挡无关批次。原queuedcancel留neverstarted事实。
- Explicit retry只对terminal failed/rejected、零publication/knownprecommit、cleanuptrue且有来源/目标观察基线的logicaljob开放；密码/已发布/unknown/sourcechanged/targetchanged/清理未完/缺事实failclosed。复用原ArchiveSpace只读source/output held-owner重验，匹配旧native source/target身份后原queue重新enqueue SAMElogicalid，newnativeattempt和generation。原editcurrentorigin/capability仍重验，无新budget/readmissionloop，不双计member。基线仅在原terminal之前收到有效原生观察才采用，不从失败之后重抓身份冒充旧源；观察失败只禁retry，不改变原job执行结果。
- 若实际launch selection含同一分卷族，先经原ArchiveSpacePreflight→ArchiveVolumeSources的只读boundedsource-set事实去重；没有事实不按basename/case/后缀猜合并。沿原observation增加严格optionalbounded源身份/主路径/成员字段，RustC#对称、legacy/default/invalid tests。原volume parser/native保护/workerexecution重新准入，不新codec。后续卷单独不能识别时等待当前selection里已验证family覆盖，否则明确refusal/notstarted，不虚报accepted。相同basename不同dir保持不同。
- Retention绑定原retired64；active/cleanupheld/retrypending不能淘汰。只保留有界批次关系/安全终态projection，丢失record后不可retry/控制，不能永久保留password/request/payload或声称跨重启history。

## Exact UI composition / defaults

已读mandatoryselection/current080/table/API。此处是归档批次状态小表，不是catalogtiles或新DataTable controller。原JobProgressView中官方stateless Table/TableHeader/TableBody/TableRow/TableHead/TableCell、Label/Tag或Alert/DescriptionList；publicslots排布，不画假控件。原footer加“批次汇总/任务详情”，成员“详情/重试/停止”等Button.outline().with_size(Medium)32DIP，直接layout/Toolbar.content，禁Toolbar.child鬼影compact。沿现有Graphite/字体/Root/titlebar/windowmin480x260，一bounded scrollbody+flexnone可达footer；长CJK/state/error不撑出。原Dialog密码/typedprompt/focus清理不改，summary不抢当前prompt。所有view→owner及owner→view副作用defer nextturn，包括即时缓存结果。无额外tick/clock/永久动画。

## Diagnostics before code

复用tracing/原jobcorrelation：batch_id、logicaljob、activeattempt、generation、accepted/start/state/terminal及fixedcategory/count/elapsed/nativecode；新增batchaccepted/memberbound/summaryterminal及retryaccepted/refused只低频state边界。projection不是每chunk日志。不得log路径/归档或membername/content/password/headers/cookies/requestDebug/exceptionDisplay。只读原space诊断继续原boundednativeowner，sink失败不改结果。

## Agreed RED→GREEN / acceptance

原productionapp/bridge三jobs两运行一排队→prompt/pause/fail/cancel/queuedcancel/cleanupunknown/另一batch；counts warning/partial/fail/notstarted/allcancelled；stale/duplicate event/readmission attempt不双计；原queuefull/memory/duplicate/overlap+defer复位；source/target变化拒retry、合法same logical/newattempt一次派发；原launch/formswhole-selection/nativefamilysourcefactdedup。rendered官方summary入口/详情/disabledretry/pointer/TabSpaceEnter/heldduplicate/minwindow/长CJK/focus/bodyfooter，原archive332/HZ1/icon/WAC/HZ2/edit保持。先真实边界RED，不把compile/fixture失败算行为RED。

所有Cargo先idle同process jobs1/incremental0/RUSTFLAGS codegen-units256，lockedoffline/serial/长timeout，无duplicate/namekill/占道要求。只按改层查Protocol/IPC/Core/Desktop/C#，effects→exactLIST→sameRUN，skip不算PASS。worker未改可沿accepted8831hash兼容；若变则当前buildfreeze。无100k/fullnative/liveproduct。交付hz3-outcomes+matrix/exactcommands/counts/failures/hashdrift/unrun层，checkpointprepend/suffix，停等独立验收。不改旧source_unavailable/helper_untrusted/FB-AC05/ZIPreadmissionbug/真实Explorer-DPI-IME-UIA-user-installed边界。