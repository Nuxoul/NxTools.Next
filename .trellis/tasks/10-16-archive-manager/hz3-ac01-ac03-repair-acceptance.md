# HZ3-AC01–03 — independently accepted; Step5 scoped closeout

2026-10-29. **Phase E — Format expansion and ZIP editing**. Parent independently accepts the three bounded repairs and the original **第五步 HZ3：批量任务汇总** delivery **only together with these repairs**, at source/compiled synthetic GPUI/original bridge/owned-native fixture levels. Original [HZ3 withheld acceptance](hz3-acceptance.md), delivery, REDs and earlier accepted EIC/TAR/7z reports remain unchanged. This does not accept whole PhaseE/A–F/product/physical-user/release/installed behavior. Next authorized item in the original sequence is **第六步 HZ4：真实进程崩溃／恢复测试**, not HZ5.

Repair message `f9cf17f4-c62b-48cb-a46f-72d1ea5eb7b3`, turn `9c25a850-95fc-4264-9ae5-fc1ec186d45d`; original HZ3 message `4a1bd2d2-44e3-42c6-bbd3-c5fb6b4373e1`, turn `5d517496-f70b-43f4-8031-fe260122abe3`. Implementation attribution/history: [repair outcomes](hz3-ac01-ac03-repair-outcomes.md).

## Independent evidence and actual results

Parent root: `C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\hz3-repair-acceptance-20261029-153117`. `results.json` retains60 complete argv/exit/time records; `accounting.json`, source baseline/identities, all raw logs and **fresh parent native-terminals.json** remain. Opening checkpoint **206976bytes**, SHA256 `385F2AA88E3A52EBED23ADBCF2D88B2EA4639C2DB461A3E1852553D242AB8C8A`, saved completely. All15 delivered source/test/fixture hashes matched before tests. Frozen/current worker, actual Host/tests DLL and Python/7Zip/bsdtar hashes matched; **739-file test-time/post-verification baseline zero drift**, including installer/Cargo/CLI initializer/old HZ3 refusal.

Matching frozen worker `C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\hz3-ac01-ac03\frozen-final\nxtools-archive-worker.exe`, SHA256 **B0DF89DAE9527D0E16471434ABBCDCFA949DAEAD0CCB07878293333491DC7F7C**. ActualTestsDLL `2C60260A92199D080101F74E52387D22DBEE03873470FE71503692F178C7436D`; HostDLL `669BD7D522672513F649E2FA749A174C46B9A69FED1B01E7AF4C110D755D1E5E`.

| Parent independent check | Actual result |
| --- | --- |
| Fresh incremental C# tests build | exit0,0warnings/0errors |
| effects-reviewed exact23 LIST -> identical RUN, matching worker | **23PASS/0FAIL/0SKIP**, every selected1/run1/name audited; new native directory-vs-zero export generated first |
| Full Desktop `archive` with **parent fresh native export** | **349PASS/0FAIL/0ignored/1167filtered**, including all7 repair production-owner/bridge tests |
| Same AC01 test with native env deliberately omitted | **1PASS/0FAIL/1515filtered** using checked-in historical transcript; separate replay, not fresh native execution |
| Protocol/IPC/Core | **91/43/100PASS**, total234/0FAIL/ignored/filtered; Core93+6+1; doc-tests0 separate |
| Archive lib+11 specified integration targets | **100PASS/0FAIL/0ignored/0filtered**, includes EIC4/14, TARraw4/20; not full45targets |
| affected Desktop+Protocol+IPC strict Clippy tests `-D warnings`; workspace tests check | exit0 |
| scoped7Rustfmt /15source diff; spec/component/release/editor/test-selection gates | exit0; source/static only |

Direct Cargo first checked shared idle lane, same-process jobs1/incremental0/codegen-units256, locked/offline/serial and sufficient timeout. No duplicate build/name kill/timeout. Parent changed no product source, no additional review/subagent. Original failing probes/reports are preserved rather than rewritten to suit new DTO fields; stronger production-owner regressions below now exercise the exact repaired boundaries.

## Closed defects and preserved invariants

### HZ3-AC01 — explicit publication facts, not absence of file commits

Accepted. Source review verifies optional three-state `output_published`: true for real FILE/directory publication, false only from a fully observed zero-output owner, null/missing for unknown/legacy/live/uncertain state. Created-directory success is recorded at the original native success boundary before fallible inspection; failed open/uncertain commit cannot silently become false. Original Runner drains and disposes resources before freezing publication facts. No new execution permission, file journal replacement, rollback or native commit precedence.

Parent **new actual native execution** generated directory case: failed/io_failure/files0/journal[]/commitstateNone/cleanuptrue with **output_published=true** and preserved directory-only result_location. Source/target recheck matched and directory still exists after verification. True-zero file failure case has exact partial_removed/unpublished journal, **output_published=false**, no result_location and empty target. Fresh export SHA256 **046BAE36479520B2FDD3C2724E87BB78875516576DD99723EEC4A85A94904C40**; its GUID roots/commands/baselines/terminals are sealed privately in parent evidence.

The full349 run then consumed **this parent export**, not implementation-session output or just stored JSON. Actual original app/PlatformBridge/finite coordinator rejects directory replay, counts its publication, preserves result_location; genuine zero-output explicit recheck dispatches one NEW operation/generation on SAME logical member. Legacy/null fact refuses. Original native tests additionally preserve created root/ancestry directories and unknown/null authority. No recursive/delete cleanup was added to make a retry pass.

### HZ3-AC02 — native-attempt and pre-native-terminal baseline

Accepted. Observation carries logical ID, concrete native attempt and purpose. First-baseline responses require current connection/session/generation, original current operation and the original live attempt mapping. `jobs.rs::Terminal` retires observations **before** forwarding native terminal to the finite coordinator; dispatch/localfailure/disconnect/logical terminal also retire them. Missing initial identity is not reacquired on a replacement finite attempt; accepted first baseline remains immutable.

Parent independently reran real production-owner/bridge tests delivering native Terminal then delayed sourceB in the same app turn while logical terminal is still pending, plus finite-budget prompt/approval/new-attempt with unchanged logical generation and delayed old observation. Both refuse baseline acquisition and replay; valid current preterminal sourceA control passes. Finite attempts retain one member/once terminal. These prove deterministic event-order behavior, **not a physical timed filesystem source-swap or power-loss test**.

### HZ3-AC03 — original bounded retirement remains recoverable

Accepted. Original `prune_retired_jobs` scans at most its starting queue length, requeues protected IDs rather than discarding tracking, and reuses original completion/window close/form retirement/recheck owner boundaries. No unbounded loop or second history manager. It cannot evict active,cleanup-held,retry-pending or pending-confirmation records merely to meet64.

Parent independently reran actual app/IPC completion + `background_job` scenarios: completed/closedA protected while B runs through70 unrelated completions, B ends,130 further completions yield **records64/retired64/orphans0**, A/B reclaimed. Separate current production test protects cleanupfalse native slot, pending confirmation and retry recheck across70 completions, then releases pending/recheck and reclaims them while native cleanup-held ownership remains. Original queue2running/32queued/2GiB/FIFO/head-overlap and accepted source/FILE-only single-commit engine ownership remain unchanged.

## Decision, unrun checks and retained blockers

Step5 and its repairs are accepted **only at these bounded evidence levels**; record host acceptance for both original delivery and repair, then dispatch Step6 HZ4 only under existing sequential authorization. No HZ5 work yet. Opening checkpoint206976bytes is preserved as an exact suffix when this conclusion is prepended; previous refusal/negative evidence remains in full.

Old ZIP readmission SourceChanged-vs-Cancelled instability, `source_unavailable`, CLI `helper_untrusted`, FB-AC05 and physical Explorer/drop/DPI/IME/UIA/theme/visual-user/installed/release acceptance remain **OPEN**. Old failing readmission chains, full45Archive,100k/performance/resource and whole native/product suites were not run or repaired here. CLI initializer compiled by workspace checks; no extra CLI behavioral acceptance claim. Process/fixture green is not power-loss or persistent recovery proof.

No package/sign/install/product/Explorer launch, Registry/service/ACL/UserChoice/recycle/real-user archive changes/truncate/punch/commit/push, dependency/vendor/backend changes or self-app GUI bypass. Current installed sequence32 remains unchanged and does not gain these code-only features from this acceptance.

## Host receipt distinction after verification

Repair delivery `f9cf17f4-c62b-48cb-a46f-72d1ea5eb7b3` was successfully host-accepted. Attempt to attach acceptance to original HZ3 message `4a1bd2d2-44e3-42c6-bbd3-c5fb6b4373e1` returned **Only a completed host delivery with a final report can be accepted**; no original host-accept receipt was created. Original source delivery was supplied through the user's attachment and repository reports, then independently validated together with the completed repair. Parent scoped Step5 acceptance remains as documented; do not misstate the original host delivery status or retry to manufacture a receipt. The completed repair acceptance plus explicit parent evidence decision is the handoff basis for Step6.
