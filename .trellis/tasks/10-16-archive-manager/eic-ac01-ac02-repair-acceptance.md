# EIC-AC01 / EIC-AC02 — independently accepted, Step4 scoped closeout

2026-10-29. **Phase E — Format expansion and ZIP editing**. Parent independently accepts both bounded repairs and the original fourth-step delivery **only together with these repairs**, at the source/synthetic/owned-worker/native-fixture evidence levels below. Original withheld report [edit-interaction-acceptance.md](edit-interaction-acceptance.md) and all negative evidence remain unchanged. This is not whole PhaseE/A–F/product/user/release acceptance. Next authorized item is exactly **Step5 — HZ3：批量任务汇总**; HZ4/HZ5 remain later steps.

Delivery: repair message `c8f966c0-ca80-4dd6-90e3-c05d63591c86`, turn `6d0ed671-36c1-4b5c-961e-fb56a622b873`; original Step4 `a672a1d9-375b-4171-bef8-1dfec2f3d69e`, turn `36fb6eb1-3bf0-451d-aceb-aacef466e750`. Implementation facts/history: [repair outcomes](eic-ac01-ac02-repair-outcomes.md).

## Independent evidence

Root: `C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\eic-reaccept-20261029-050357`. `results.json` retains44 exact commands/exits/times; `accounting.json`, `baseline.json`, `identities.json`, original-script wire/results and raw logs are retained. Before tests, all7 delivered source/test hashes and38 protected hashes matched. Frozen worker, current Cargo worker, actual test DLL, PlatformHost DLL and Python/7-Zip/bsdtar identities matched delivery. Test-time baseline, including installer and opening checkpoint, had **zero drift**.

Current matching frozen worker: `C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\eic-ac01-ac02\frozen\nxtools-archive-worker.exe`, SHA256 **8831BEAD51689176FB4409568B024ED7809ADABD131A6A437AC4109FE2E341F7**. Test DLL SHA256 `CD57795D67ADAC445F074D29BD78F7F9C44AA3F47A34C49133E978FD1BD8FAB3`; executable apphost hash alone is not used as test-content identity.

| Independent check | Actual result |
| --- | --- |
| Original `edit-implicit-probe.py`, same bytes and assertions, only isolated evidence pointer/new worker | PASS: complete/verified0/bytes0, commits0/opens0/no candidate/source unchanged |
| Original `edit-cancel-probe.py`, same bytes and assertions, new worker | PASS: one commit attempt, reply-only cancelled -> worker failure/Cancelled, source unchanged |
| Archive lib+11 relevant integration targets, isolated-codecs/serial/no-fail-fast | **80PASS/1FAIL/0ignored/0filtered**, exit101; only old output_readmission_chain failed; do not label batch all-green |
| New `eic_repair_worker_contract` within that run | **4PASS**,14 scenarios: implicit/explicit skip, new empty/real file/prefix/type controls, ZIP/TAR/7z native reply-only cancellation and5other-error controls |
| Relevant original ZIP/7z/TAR conflict/candidate/readers-close/cancellation/raw-header targets | PASS; accepted TAR-AC01 raw4/20 combinations included |
| Archive strict Clippy tests `-D warnings`, workspace tests check | exit0, no suppression |
| Fresh incremental C# test build | exit0,0warnings/0errors |
| effects -> exact16 LIST -> identical16 RUN, matching new worker/oracles | **16PASS/0FAIL/0SKIP**, every selected1/run1/exact names; includes two new native tests and prior unknown/latecancel/sourcechange/cleanup/preview boundaries |
| Scoped3Rustfmt /7source diff; spec117/14owners; GPUIrelease/editor provenance/test-selection | exit0; source/static only |

Direct Cargo checked idle lane before each operation and inherited jobs1/incremental0/codegen-units256 in the same process. No duplicate build, name kill or additional reviewer. Parent did not edit product code. The prior independent broad Step4 sweep remains332Desktop/261Archive/35native PASS before repair; Desktop/UI/C#product owners are protected unchanged in this repair. Parent did not claim to rerun fullDesktop or fullArchive45 in this narrow round; implementation's332/265 results remain separately attributed.

## Closed defects / actual boundaries

**EIC-AC01 accepted.** Source review confirms incoming directory merge checks only ORIGINAL snapshot descendants at slash boundary, only when no exact entry exists. Future incoming/reserved names cannot create fictitious old directories. Exact file-kind collision is not merged. Parent original failing probe is now zero candidate/zero commit; new worker controls preserve genuinely new empty directories and real files. Original native Job tests prove implicit/explicit allSkip warning/skipped1/committed0/emptyjournal/no flush/no publication/original bytes unchanged, and real additions still publish once with independent .NET decode.

**EIC-AC02 accepted at exact native semantic mapping.** Only `Some("cancelled") => ArchiveError::Cancelled` was added to original reply mapping. No blanket "cancel requested" override, new cancellation controller or C# precedence change. Parent unchanged reply-only probe deterministically changed IoFailure -> Cancelled. New actual worker ZIP/TAR/7z cases and other-error controls passed; actual owned-native three-format Job results and original native failure/unknown/latecancel/source-change/cleanup controls passed. This closes the reproduced mapping defect, not a claim that every cancellation race anywhere is resolved. Real native Runner may send cancellation independently, so its old-worker test sometimes passes; deterministic red/green authority remains isolated native reply plus worker tests, not repeated attempts to seek a red native run.

Product source changes remain exactly `worker_zip_edit.rs` (implicit merge) and `worker_output.rs` (exact error mapping). ZIP record editor, accepted7z/TAR planners/headers/outer codecs, C#Job/Runner/native held-source/OutputSession/FILE-only handoff/single publication, Desktop/UI, protocol/Cargo dependencies and protected history remain unchanged.

## Explicit retained failure and limits

Parent narrow run reproduced old `output_readmission_chain::real_rust_coordinator_reenters_original_native_admission_only_after_cleanup`, `source_changed` scenario at line240: **left Cancelled/right SourceChanged**. Exact same failure existed in original Step4 delivery before this repair; parent prior broad run passed, current chain source SHA256 `14260949051607E785D009EF4E09C0AAEB8742A02656B0779494747E42521FD4` matches parent pre-repair baseline. This failure is retained in `focused.log`; it was not rerun to manufacture green and is **not closed or root-caused** by EIC-AC02. Acceptance is limited to requested repairs/Step4 boundary, not a claim all regressions are green.

Historical `source_unavailable`, CLI `helper_untrusted`, FB-AC05, physical Explorer/drop/DPI/IME/UIA/theme/user/installed acceptance remain OPEN. No package/sign/install/product/Explorer/Registry/service/ACL/UserChoice/recycle/real-user source writes/truncate/punch/commit/push. Installer sequence32 remains unchanged. Original checkpoint190667bytes saved before checks; acceptance prepends only and proves complete original bytes as exact suffix.
