# Step5 HZ3 — independent acceptance withheld (HZ3-AC01–03)

2026-10-29. **Phase E — Format expansion and ZIP editing**, user's fifth step only. Delivery message `4a1bd2d2-44e3-42c6-bbd3-c5fb6b4373e1`, turn `5d517496-f70b-43f4-8031-fe260122abe3`; user supplied completed delivery/progress attachment and requested acceptance. Fourth-step/EIC acceptance remains unchanged. **HZ3 is not accepted; no HZ4/HZ5 dispatch until these bounded defects are repaired and narrowly reaccepted.** Parent did not modify product source.

## Independent baseline and checks

Evidence root: `C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\hz3-acceptance-20261029-133510`. `results.json` retains55 exact argv/exit/time records; raw logs, `baseline.json`, `identities.json`, `accounting.json` and probe provenance/results remain. Opening checkpoint **199466bytes**, SHA256 `9DC19AAA4B25297B7646670092FBAA184000DD5A7DE6C1A618F32B1C323949D0`, saved complete. All19 delivery source/test hashes matched before tests; frozen/current worker, actual test DLL, Host DLL, CPython/stock7Zip/bsdtar hashes matched. Test-time and post-probe **736-file baseline zero drift**, including Cargo/installer/checkpoint/protected source.

Matching worker: `C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\hz3\frozen\nxtools-archive-worker.exe`, SHA256 **6CD1C363895DB88BFD741DD5B8924D076E018C1734184DC7CC31E5567F14C0E1**. Actual test DLL `A2EBE89E7F8F311A09BF3C869EDDDF7FA3B45626FFCC5F678321EDA88378B65E`; Host DLL `AA55B3472D32C2C6E67FC066D5769782C76FE30374629DE46169A66AFACFC817`.

| Parent independent check | Result |
| --- | --- |
| Full Desktop `archive` filter, locked/offline/serial | **342PASS/0FAIL/1167filtered**; includes final finite-readmission batch assertions |
| Protocol/IPC/Core | **90/43/100PASS** (Core93+6+1), total233,0FAIL/ignored/filtered; doc-tests0 separate |
| Archive lib+same10 integration targets | **98PASS/0FAIL/0ignored/0filtered**; EIC4/14 and TARraw4/20 retained; not full45targets |
| affected Desktop+Protocol strict Clippy tests `-D warnings`, workspace tests check | exit0 |
| fresh incremental C# tests build | exit0,0warnings/0errors |
| reviewed exact21 LIST -> identical RUN | **21PASS/0SKIP/0FAIL**, selected1/run1 for every exact name; actual name accounting retained |
| scoped15Rustfmt /19source diff; spec/component/release/editor/test-selection gates | exit0; source/static only |

Every Cargo and supplemental direct rustc operation checked idle shared lane and inherited jobs1/incremental0/codegen-units256 in the same process. No duplicate build/name kill. Original source observation/queue/UI/detail/finite-attempt call chains were read; original queue2running/32queued/2GiB/head-overlap remains unchanged. The passing55-command sweep **excludes** the new probes below. No old readmission chain was rerun seeking green; its prior SourceChanged-vs-Cancelled failure remains OPEN.

One bounded readonly reviewer covered retry identity/publication boundaries. Parent independently traced both findings and verified source/native counterexamples; no second review or broad engine audit. Source-extracted Rust probes retain exact original method bodies with minimal shape-compatible containers/context stubs. They establish those local predicates/lifecycle mechanisms, **not production GPUI execution, a real timing race run, or physical/user acceptance**. Required fixes must add red-capable production-owner regressions.

## HZ3-AC01 — directory publication is incorrectly eligible for generic retry (high)

`batch_jobs.rs::retry_facts` lines23–39 treats `files_committed==0`, `commit_state=None` and empty journal as proof of zero publication. Original native `ArchiveOutputSession.EnsureDirectory` lines190–206 completes a real directory; `CommitJournal` lines71–73 describes file transactions only. Disposal does not remove completed directories. Existing target-root identity can stay identical despite new children.

Parent **actual original Job + current worker + native output** probe: scratch `hz3-native-probe/{Probe.csproj,Program.cs}`, GUID root `archive-product-8d6ee822-9df3-46be-9bb6-38980d2ca60e` in evidence. Archive contains only `published-empty/`; original `before_directory_metadata` fixture throws `ArchiveOutputException("io_failure")`. No real-user input or unapproved system action. Measured `result.json`/`native-directory-run-v2.log`:

- actual directory survives; source bytes unchanged;
- terminal **failed/io_failure, files_committed0, commit_state=null, journal[], cleanup_complete=true**;
- original source/target fresh expected-identity observation succeeds with identical identities;
- terminal even has truthful `result_location` identifying completed directory, which `retry_facts` does not check.

Parent `hz3-retry-probe.py` executes exact extracted `retry_shape/retry_facts` against those measured native facts: **retry_facts=true**, failing the no-published-output assertion (`retry-predicate-probe/run.log`). Thus this is not merely a guessed DTO shape or a claim that zero file count means zero output. Summary also uses file/journal-only `published` at `batch_jobs.rs:145–147`, so this real output is hidden from its published count.

Repair fail-closed publication evidence in the original owner: do not infer zero output from absence of file journal/commitstate, and do not blindly retry this directory/partial-output case. Preserve original directory-only result-location behavior and budgets; do not delete/recurse/rollback directories to make a retry eligible. If adding bounded native facts, keep version/backcompat and authority clear. Add original owned-native -> app retry refusal and summary truth regression, plus a genuinely zero-output eligible retry control.

## HZ3-AC02 — baseline is not bound to native attempt/first native terminal (high)

`JobObservation.job` stores only logical ID and retry bool; baseline `job_observation_result` checks current connection and logical command generation but **not original active attempt or whether a native terminal already arrived** (`batch_jobs.rs:487–498`). Native extraction terminal path `jobs.rs:829–849` forwards it to finite coordinator while leaving `record.terminal=None`; the logical terminal comes later from `readmission.rs:81–104`. Finite attempts keep generation and change only operation ID (`product_job.rs:53–54`). Therefore a delayed earlier/final-attempt observation can become the first baseline in this real coordinator gap, including an observation that only acquires a replacement source after the original native attempt released it. Fresh retry would then compare the replacement against itself, not original execution context.

Parent traced those call chains and `observe_job`/deferred receipt routing. `hz3-late-probe.py` runs the exact original record-filter/baseline branch with same logical generation, changed active attempt/native-terminal-seen but logical terminal pending: **stored_baseline=Some(source-B)** instead of refusal (`late-baseline-probe/run.log`). Stub `native_terminal_seen` is evidence of the lifecycle condition, not an existing production guard; original code never consults such a fact. This is deterministic local branch evidence, **not a claim a physical timed source-swap/GPUI scenario was executed**.

Bind baseline observation to current native attempt and its pre-native-terminal lifecycle, not logical generation alone. Invalidate/retire late observations at actual native terminal and attempt transition; do not acquire a new first baseline after any relevant native terminal just because coordinator has not yet published logical Done. Preserve logical-member/finite-attempt behavior and fail-closed when original identity was unavailable. Add production original-bridge regressions delivering native terminal/attempt transition before the delayed observation, and ensure no retry/new attempt from swapped context; valid current preterminal baseline remains a control.

## HZ3-AC03 — batch-held completed record loses retirement tracking (medium)

New `batch_retirement_held` correctly protects a closed completed member while its sibling/pending confirmation remains active. But `jobs.rs:1115–1126` **pops its ID from `retired` even when held and not removed**, with no later requeue/sweep. Once sibling ends or batch metadata is pruned, the record remains outside all retirement tracking. Retained source requests and control/result data can accumulate beyond the intended64 completed history. This differs from preexisting open-window retention: probe members all have `window=None` and no native cleanup-held slot remains after completion.

Parent `hz3-retention-probe.py` extracts exact `prune_batches`, `batch_retirement_held` and original `job_terminal` retirement block. Minimal original-shape containers exercise80 two-member groups: first member completed+closed, sibling occupies one slot while65 unrelated jobs complete on the other, then sibling finishes. After130 extra completions, measured **records144 / retired64 / batches0 / orphan_untracked_completed80** (`retention-probe/run.log`, assertion exit101). No scheduler/native process was created; this is an executable source-container lifecycle proof, not a GPUI integration test.

Keep held IDs reclaimable or sweep them after protection ends, within original retirement owner; do not evict active/cleanupheld/retrypending/pending-confirmation records to meet a count. Add production-owner completion/closed-window/late-sibling/64-bound regression and stable control with live slots/relations protected. No new history manager.

## Probe setup history and decision

Supplemental generator initially used system GBK; explicit UTF8 fixed scratch decoding (not product RED). First native probe was rejected by original owned-root naming; rerun used required `archive-product-<GUID>` unchanged guard, assertions and injection. Initial and corrected logs retained separately. Scratch probe build had one MSB3277 WindowsBase reference warning,0errors; this is **not** the product tests build, which had0warnings. The corrected real native probe succeeded and emitted the exact problematic facts; extracted predicate/late/retention probes deliberately fail their expected safety/lifetime assertions. No logs/negative evidence overwritten.

Return only HZ3-AC01–03 for bounded repair to the same implementation session; wait for completion then narrow independent reacceptance. No HZ4/HZ5 dispatch. Keep original HZ3 reports, accepted EIC/TAR7z/ZIP engines, original failure logs/checkpoint history, installer/keyledger and unrelated dirty changes intact. No package/sign/install/product/Explorer/Registry/service/ACL/UserChoice/recycle/real-user source modification/truncate/punch/commit/push. Known source_unavailable/helper_untrusted/FB-AC05/readmission instability and physical/UI/user/installed acceptance remain separate OPEN items. Checkpoint only top-prepend with entire saved199466bytes preserved as exact suffix.
