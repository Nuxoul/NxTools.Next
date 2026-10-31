# Step4 — independent acceptance: withheld for EIC-AC01 / EIC-AC02

2026-10-29. **Phase E — Format expansion and ZIP editing**, user's fourth step only. Delivery message `a672a1d9-375b-4171-bef8-1dfec2f3d69e`, turn `36fb6eb1-3bf0-451d-aceb-aacef466e750`. Step3/TAR-AC01 remains accepted. **Step4 is not accepted; do not dispatch HZ3–HZ5 until bounded repair and independent reacceptance.** No product source was changed by this acceptance round.

## Evidence and independent checks

Parent evidence: `C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\edit-acceptance-20261029-041811`. `results.json` retains all83 expanded commands/exits/times; `baseline.json`, `identities.json`, `accounting.json`, raw logs and probe wire/result files remain immutable. Opening checkpoint184169bytes, SHA256 `35E9BFFD1610E3896D3C976FC1066DFB412E7E90B6BE4DF8BAE91E03F6682870`, saved complete. All23 delivery source hashes matched before checks. Test-time source/protected Cargo/installer/checkpoint baseline had **zero drift** (`drift.json`).

Frozen isolated worker: `C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\edit-interaction\frozen\nxtools-archive-worker.exe`, SHA256 **3396A7A9215892D7E76E57FF7058DF939495A3618E18D343618EFE587D044FA7**. Independent CPython/7-Zip/bsdtar identities matched delivery tools before testing; exact environment in parent runner/identities, including codec upstream and stream fixtures. This is the current Step4 worker, not the accepted Step3 worker.

| Independent verification | Actual result |
| --- | --- |
| `cargo test -p nxtools-desktop --locked --offline --bin nxtools-desktop archive -- --test-threads=1` | **332PASS/0FAIL/1167filtered**, exit0 |
| Same expanded Archive44 lib/integration targets, isolated-codecs, no-fail-fast, serial, same explicit100k exclusion | **261PASS/0FAIL/0ignored/1filtered**, exit0; TAR raw4/20 combinations retained |
| affected Archive+Desktop strict Clippy `--tests --features nxtools-archive/isolated-codecs -- -D warnings` | exit0 |
| workspace check `--tests --locked --offline` | exit0 |
| fresh incremental C# test build `--no-restore -m:1 -nr:false -p:UseSharedCompilation=false` | exit0 |
| effects-reviewed exact35 LIST -> identical exact RUN, matching worker/oracles | **35PASS/0FAIL/0SKIP**, one independent sweep; each selected1/run1, exact name accounting |
| scoped fmt17 / diff23 | exit0 |
| spec/component catalog/GPUI release/editor provenance/tree-root-drag/test-selection gates | exit0; source/static evidence only |

All Cargo operations checked shared cargo/rustc/clippy idle immediately before starting and inherited jobs1/incremental0/codegen-units256 in the same process; no duplicate build or process-name kill. The83-command sweep excludes the two new failing probes below. The older implementation full-run260PASS/1FAIL ZIP readmission and33PASS/1FAIL/1SKIP native history remain unchanged: this independent sweep does not root-cause or erase those failures.

Source review covered final form origin/path/source/generation/snapshot/member-kind checks, frozen target drop, terminal warning/no-op/retry guards, deferred cross-entity work, one-generation path-only restoration to NEW RowIds, private bounded conflict presentation and original official controls. Existing source/lease/FILE-only commit protections remain; successful broad tests do not override the concrete counterexamples below.

## EIC-AC01 — ZIP implicit-directory merge causes publication despite all files skipped

**New Step4 adapter defect, independently reproduced.** `worker_zip_edit.rs::additions` recognizes directory-directory merge only via an exact snapshot entry (lines94–100). A ZIP containing `folder/same.txt` and `untouched.txt`, with **no explicit `folder/` record**, already has a logical folder. Adding local directory `folder/` with only `same.txt`, answering Skip to the original conflict, leaves that directory in `selected` (lines184–188). `execute` therefore bypasses zero-candidate handling and requests one original commit (lines363–370,427–428).

Parent scratch probe `edit-implicit-probe.py` derives a separately saved fixture from the existing protocol oracle, changes only the archive's explicit-directory record and protected-source directory enumeration. Same current frozen worker, original ticket/Skip, no source file opens. Evidence directory `implicit-probe-4f1adafe-33f9-44a7-9480-e943b5c94449` contains `fixture.py`, `wire.json`, `result.json`, independent CPython candidate decode:

- terminal complete, **commit requests1**, source opens0;
- candidate entries `folder/same.txt`, `untouched.txt`, **new `folder/`**;
- original source unchanged (probe intentionally acknowledges rather than actually publishing);
- expected logical directory merge + all-skipped is **zero candidate/publication**, warning, original archive untouched.

One bounded readonly reviewer identified this scenario; parent independently verified the call chain and actual worker counterexample. No second broad review is requested. Repair the original adapter's implicit/explicit directory semantics, not ZIP record codec or native transaction. Preserve truly new empty directories and real non-skipped additions; add a red-capable original worker regression plus owned-native zero-publication proof.

## EIC-AC02 — native cancelled is deterministically mapped to IoFailure

**Existing boundary defect exposed by delivered cancellation failure; not asserted newly introduced by Step4.** Original implementation TAR native log explicitly shows `archive.output.cancelled`/partial_removed, `archive.worker.cancelled`, zero commit and cleanuptrue, then `archive.job.terminal` failed/io_failure. Original evidence remains at `C:\Users\Administrator\AppData\Local\Temp\archive-product-c2188708-5348-4e63-9b9d-f2d942215042\evidence`.

The parent independently reproduced the underlying protocol boundary without a timing race: scratch `edit-cancel-probe.py` creates a GUID-owned tiny TAR, completes candidate/readback/readers-close handoff, then answers the single native `commit` with `success=false,error_code=cancelled` (no separate control frame needed). Evidence `cancel-probe-e3559853-4633-48af-9b21-8c935efe12ad/{wire,result}.json`:

- worker emits **failure/IoFailure**, expected failure/Cancelled;
- exactly one commit attempt, original source unchanged;
- `worker_output.rs::PrivateOutput::call_payload` lines397–408 has no cancelled mapping, falls into IoFailure;
- `ArchiveJobSession.cs::OnFrame` line179 stores failure/io_failure and terminal projection line154 prefers it over runner cancelled, explaining why the native/control delivery order can decide the final visible outcome.

The independent35-case sweep's TAR cancel test passed; that does not contradict the deterministic reply-only failure or repair its race. Repair only the semantic cancellation propagation boundary, with deterministic regression and actual owned TAR/ZIP/7z cancellation checks. Keep native publication/unknown/source-change/cleanup failure authority; do not globally override failures merely because cancellation was requested, and do not rewrite native commit/lease ownership or weaken assertions/budgets.

## Decision and limits

Return only EIC-AC01 and EIC-AC02 to the existing implementation session, then stop for its callback. No HZ3 dispatch, packaging/sign/install/product/Explorer/system mutations/real-user archive edits/commit/push. Preserve the negative probes and all original reports. Current installer sequence32 remains unchanged and does not contain these source-only features.

Known `source_unavailable`, CLI `helper_untrusted`, FB-AC05, old ZIP readmission instability, physical Explorer/drop/DPI/IME/UIA/theme screenshot/user/installed acceptance stay separate and open. Process/protocol fixtures are not power-loss or user acceptance. The requested lifecycle counterexamples, not speculative scope expansion, are the only Step4 repair blockers here.
