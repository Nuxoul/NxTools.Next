# Feedback / HZ2 repairs and operation-capability foundation — independent acceptance

2026-10-28. **FB-AC01 PASS / FB-AC02 PASS / FB-AC03 PASS / FB-AC04 PASS; common non-ZIP editing foundation PASS at source and automated boundaries. Overall PARTIAL.** FB-AC05 remains unidentified/unfixed without marked screenshot; independent native union retains55PASS/1FAIL. No product fix, task replanning, package/install/product/Explorer launch/system mutation/commit/push in this audit. Existing HZ1/WAC/icon/ZIP/installer acceptance and old failures remain unchanged.

## Itemized decision

| Item | Independent conclusion |
| --- | --- |
| FB-AC01 no-event speed/ETA | PASS. Production view advances monotonic display time, expires samples without observe/fake bytes, <=100ms active-visible refresh; hidden/background/terminal retires it. Actual Progress observation timestamp is retained in JobRecord so UI-only refresh does not renew rate freshness. No-event6.1s/visibility/UI-only/unknown-total/stage/terminal tests pass. |
| FB-AC02 control failure recovery | PASS. Original JobRecord owns bounded correlated controls with receipt/action/attempt/generation/connection; failed enqueue/refusal/5sreceipt-timeout/10sstate-timeout resolves UI wait, explicit retry only. ACK is not actual Paused/Running; original State clears corresponding wait. Actual button→queuefull/native refusal/retry/late result/attempt/disconnect/terminal/cancel competition tests pass. Returned reserved-cancel receipt is used; native slot is not freed on UI timeout. |
| FB-AC03 directory-only result location | PASS. Completed native EnsureDirectory facts plus verified held handles participate even when file commits0; prepared root/wrapper/uncommitted file ancestry do not. Real worker13directory shape/collision cases,6empty cases and3guarded terminals pass; current fresh22terminal export consumed by original Desktop location gate passes,13actual successful cases locate once. True empty archive stays no-output/no-location; failure/cancel/unknown do not auto-locate. |
| FB-AC04 formatting | PASS. Exact18reported paths scopedfmt and diff pass; additional7capability/ZIP paths scopedfmt pass. No globalfmt claim, formatter repair or lint exemption by acceptance. |
| FB-AC05 two lines | OPEN. Target lines still not identified, no alignment change or marked current screenshot. General geometry pass does not close user complaint. |
| Non-ZIP common foundation | PASS for existing capability/request/admission foundation, not actual non-ZIP editing. Opened format/state/revision and semantic operation drive typed refusal, browser/application/worker gates; reads remain independent. 7z/TAR/all other new writers and ReplaceContent remain unavailable; no new writer/ABI/codec/native permission. |

## Source and original RED review

Read current checkpoint, repair outcomes/route, previous independent findings, applicable owner/reference contracts and actual code. Original compiled RED logs were inspected: rendered no-event1FAIL; queuefull/native-refusal2FAIL; real empty-directory worker terminal1FAIL; ZIP unknown-record deletion compatibility1FAIL. These are original recorded failures, not pre-fix runs newly performed here. No compile/selection failure treated as behavior RED.

Progress implementation: `archive/progress.rs:142–285` connects actual observation time, monotonic elapsed, expiry, active visibility and presentation retirement; `progress_metrics.rs:44–51` expires bounded samples. `jobs.rs:714–729` changes observation time only for actual Progress event. Production tests at progress_feedback_tests.rs164/236/280 explicitly advance clock without new events, hide/show, and apply UI-only refresh with old progress timestamp. Long failed/cancelled/unknown-cleanup terminal and footer tests remain active. Not a physical visibility/DPI or real stalled disk run.

Control implementation: `jobs.rs:417–627` installs request before dispatch, uses real returned receipt, distinguishes enqueue/refusal/ACK/two deadlines, matches attempt/generation/connection and clears only UI wait. `:760–775`requires actual matching State. No source/queue slot release or automatic retry. View local pending prevents duplicate clicks, app result projection restores retry. Tests drive actual buttons and original PlatformBridge, not only ideal view.apply.

Directory implementation: `ArchiveOutputSession.EnsureDirectory` marks Completed after successful native verify; `ResultLocation` checks actual file/Completed-directory outputs before selecting actual held path. No invented file commits. Source tests protect prepared-only/wrapper-only/uncommitted ancestry and retained failure/cancel/unknown facts. Fresh export/replay evidence below is stronger than reading checked-in transcript alone, but still not a live Desktop named-pipe/Explorer flow.

One bounded readonly capability review completed and found **no additional confirmed defects**; main independently read relevant paths and ran tests. `ArchiveCapabilities::opened/admit` in model.rs189–236, Engine open/source inspection, protocol Modify semantic operation, browser/application guards and worker dispatch preserve actual ZIP-only writer authorization. Unknown verified ZIP metadata allows Delete only; candidate validates every retained local/central record and full candidate/source again. Directory rename intent is checked against immutable entry kind. Existing AES/ZipCrypto packed-byte/ZIP64/source/native-handoff regressions remain green. This foundation is not a claim of every future format state or all metadata fidelity already supported.

## Independent execution

Evidence directory:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\feedback-repair-acceptance-20261028-175803`

`results.json`, `fresh-results.json`, `accounting.json`, per-command logs and baseline record args/exits/times. Every Cargo checks idle lane and inherits same-process jobs1/incremental0/RUSTFLAGS=-C codegen-units=256; --locked --offline,serial tests, adequate timeout. No duplicate Cargo/timeouts/process kills. Incremental Cargo/MSBuild reuse disclosed; no clean/full-engine/fullDesktop claim.

| Executed check | Independent result |
| --- | --- |
| Desktop archive serial filter |310PASS/0FAIL/0ignored/1164filtered;111.53s test execution,113.23s command |
| Subsets within archive |HZ118/icon8/WAC12 PASS; exact names counted, not extra runs |
| Archive isolated-codecs --lib + job_execution_contract + worker_wire + zip_edit_contract + zip_edit_worker_contract |22+2+19+10+1 =54PASS,0FAIL/ignored |
| Core / IPC / Protocol --lib |93/43/88PASS,0FAIL/ignored |
| Five-owner strict Clippy --tests -Dwarnings, isolated-codecs |PASS exit0,40.94s |
| cargo check --workspace --tests |PASS exit0 |
| C# tests project serial incremental build --no-restore / no shared compilation |PASS exit0,0warnings/errors |
| Original56native --fixtures --list |LIST56/RUN0; exact set retained |
| Exact readmission failure --list then same RUN |LIST1/RUN1/PASS0/FAIL1/SKIP0,exit1,source_unavailable |
| Exact other55 --list then same RUN |LIST=RUN=PASS55/SKIP0/FAIL0/not_run0,exit0 |
| Distinct native union |**55PASS/1FAIL/0SKIP/0not_run**,not a56case all-green result |
| Exact18Rust fmt/diff |PASS exit0; additional7capability paths fmtPASS |
| Spec contracts / fixture selection gate |PASS; source/document/classification only |
| Fresh actual directory export --list then same RUN |2PASS,13shape/collision+6zero+3guarded real terminal cases;22files exported |
| Desktop exact transcript test using those fresh22files |1PASS/0FAIL;13successful actual outputs locate once, all22zero-file-count terminals consumed |

Fifteen primary commands:14exit0,1expected retained failure. Extra fresh directory two-fixture and Desktop one-test runs are separate, not added as distinct native union cases. C# reviewed selection matches delivery original56names; no default full native/realExplorer/Registry action. Explicit fixture paths supplied; no missing fixture skip hidden.

Worker used is delivery's frozen isolated-codecs binary:
`C:\Users\Administrator\.pi-desktop\scratch\a03c841f-63f5-4955-8c7b-3f30a8aac0cf\archive-feedback-repair\frozen-worker\nxtools-archive-worker.exe`
SHA256 `771EEBDB507500E7E8F5E44B233F880798743EA597F195440CD4F1409150072D`, verified before/after. This audit rebuilt affected test targets/C# incrementally but **did not independently rebuild that frozen worker**. It is actual matching delivery-artifact execution, not new source-reproducibility or signed Release approval. Normal shared target worker is not substituted.

## Retained native failure and attribution

`archive job fixture actual readmission shortfall no output` independently fails at original preflight with source_unavailable, before receiving the expected resource_limit/OutputShortfall. No budget/source policy/test assertion was altered. Main checked fixture source paths and inspected the first owned budget.zip setup.

Relevant source path: Runner sourceLimit=limits.max_output_bytes for non-BrowserEntry (`ArchiveWorkerRunner.cs:749–753`); VolumeSources.CheckEncoded rejects encoded source bytes greater than that limit (`ArchiveVolumeSources.cs:175–182`); native catch maps most such failures to source_unavailable (`Runner:781–789`). Thus the1byte-output fixture has an explainable source-admission/type-mapping issue before output-shortfall discovery; no claim that its user source was missing. Exact native stage was not instrumented/reproduced with a new diagnostic probe here.

**Historical attribution independently grounded:** Runner, VolumeSources and SourceLease hashes are byte-identical to prior independent feedback acceptance baseline saved before this repair. Frozen current worker reproduces failure; relevant source chain was not changed by this repair. This supports separate pre-existing failure status, not a declaration that the broader product is green or that every prior build had the same observed result. CLI cleanup helper_untrusted failure was not rerun; remains unaccepted historical evidence, not PASS or proven fixed.

## Preservation, concurrent work and limits

Opening693file baseline covers relevant source/tests/Cargo/current reports/checkpoint/installer. Primary verification completed with empty drift.json at18:02:02; fresh directory rerun completed18:03:33. Later final audit detected `rust/apps/nxtools-desktop/src/main_window.rs` changed (mtime18:05:02), **after compiled verification**. This other-session drift is preserved, not reset or edited; cumulative HEAD diff is not its exact change attribution. All other baseline paths remained matching at accounting. Consequently results certify the tested snapshot and unchanged archive owners, **not the entire later live workspace**; no repeated whole-suite green hunting.

Original checkpoint162051bytes is saved and retained as exact suffix after acceptance prepend. Prior reports/RED/native failure remain unchanged. Final preservation check excludes only intended checkpoint prepend and separately records concurrent main_window drift. Only this acceptance document and checkpoint status are repository writes by this audit; no product code/fixtures/formatter/task-plan edits.

Overall PARTIAL because FB-AC05 and broader native readmission failure remain; actual Explorer responsiveness/target publication/reuse/focus, physicalDPI/IME/UIA/user visual, live pipe saturation and installed Release32 acceptance remain NOTRUN. No product/Explorer launch, destructive source operation, package/sign/install/Registry/service/ACL/UserChoice/commit/push. No HZ3–HZ5 or7z/TAR edit implementation started. This audit closes only the named repaired behavior/common foundation at documented evidence levels, not A–F/first release.
