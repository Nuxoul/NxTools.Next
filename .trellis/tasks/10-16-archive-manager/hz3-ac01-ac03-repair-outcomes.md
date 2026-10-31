# HZ3-AC01–03 bounded repair outcomes — awaiting narrow independent acceptance

2026-10-29. Task `f9cf17f4-c62b-48cb-a46f-72d1ea5eb7b3`. Remains **Phase E — Format expansion and ZIP editing**, user's **Step5 — HZ3：批量任务汇总**. Only the three independently demonstrated defects were repaired. Original `hz3-acceptance.md`, `hz3-outcomes.md`, `hz3-matrix.md`, earlier acceptance/refusal reports and negative evidence are unchanged. This report does **not** accept HZ3, the whole phase, product or release. Fourth-step/EIC/TAR accepted scope remains unchanged. Stop for parent narrow reacceptance; **no HZ4/HZ5**.

## 1. Scope, opening and evidence

Implementation evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\hz3-ac01-ac03`.

Before edits, saved the entire actual checkpoint: **202215bytes**, SHA256 **41DEF2F0EA5091141D6FC4AE668E8EFB0C381AAAADCB91E87CBF0AFF4434EC27** (`checkpoint-before.bin/json`). Opening HEAD/status,202 bounded source/history copies+hashes and fresh736-file wide hashes saved. Final checkpoint is top-prepend only; `checkpoint-suffix.json` proves the whole202215byte opening exact suffix. The original parent refusal stays in that suffix. Pre-code scope/invariants/seams are in [repair route](hz3-ac01-ac03-repair-route.md).

No subagent/new session/second review. Original HatchZip source map was reread; new native/terminal/retirement call-chain entries appended to `docs/reference-code/hatchzip/batch-summary.md`. No reference CLI/GUI execution or imported scheduler. No package/sign/install/product/Explorer/Registry/service/ACL/UserChoice/recycle/real-user-source operation, commit/push/reset/staging, dependency/vendor/backend upgrade or self-app GUI bypass.

## 2. Repairs in original owners

### HZ3-AC01 — complete native publication facts, directory summary and retry refusal

- Original `ArchiveOutputSession.OutputPublished() -> bool?` observes publication history: known published FILE or created/completed output directory =>true; original disposed/known zero-output owner =>false; live/unknown directory/uncertain FILE commit =>null. Relative directory native-success records true **before fallible inspection**, covering prepared root, exclusive wrapper and file ancestry as well as completed empty-directory requests. Existing directory retention and FILE-only journal/commit/cleanup remain unchanged.
- Optional/default-null/omit-null field flows original `ArchiveWorkerEvent.OutputPublished` -> `ArchiveJobSession` -> `ArchiveJobTerminal.OutputPublished` -> Rust `output_published: Option<bool>`. No version/permission/native error or cancellation precedence change. Unknown/legacy is not zero. Runner freezes facts after drainage/resources; no new call loop/ABI/write permission.
- `retry_facts` now requires affirmative `Some(false)` **and no result_location**, besides existing failed/rejected,clean/precommit/unpublished file facts, limited code/shape and original immutable baseline. Summary counts true native publication and truthful existing file/result-location facts. No directory removal/recursive rollback; directory-only result_location is untouched.
- Native regression `Hz3BatchTests.NativeDirectoryPublicationAndZeroOutput` uses actual original Job+matching worker+native: empty directory survives `before_directory_metadata` io_failure, files0/journal[]/commitstateNone/cleanuptrue, unchanged source/expected target and preserved directory result_location; new fact=true. Genuine existing-root file `before_flush` io_failure cleans exact partial, no directory/final output, fact=false. Both measured requests/baselines/terminals export to `NXTOOLS_HZ3_NATIVE_EVIDENCE`.
- Actual production app test consumes current native export through original PlatformBridge notifications and finite coordinator, asserts directory published/nonretryable, and proves genuine zero-output explicit recheck dispatches exactly one NEW attempt on SAME logical member. Legacy missing fact refuses. Checked-in exact actual-native transcript `fixtures/hz3-native-publication.json` is byte-identical to initial GREEN native export (SHA256 **E959B48F14258EDFD27A0BF2A8345BB5ECE8A9720DE1F30DE33BB424A9FB36BC**); ordinary env-absent replay test also passes. Replay is not a fresh native execution or filesystem action on stored paths.

### HZ3-AC02 — attempt-bound, pre-native-terminal first baseline

- `JobObservation.job` now carries `(logical_id, concrete_native_attempt_id, retry_purpose)`. Initial observation/response requires current connection/session/generation, active attempt == original command operation ID, and that exact operation still in original live `Jobs.attempts`.
- `retire_job_observations` closes pending first-baseline acquisition at **native Terminal before finite coordinator delivery**, original attempt dispatch/local failure/disconnect/logical terminal. Original native attempt table is reused; no second lifecycle manager. First valid baseline remains immutable; missing original facts cannot be reacquired on a replacement finite attempt with unchanged logical generation.
- Explicit terminal recheck is bound to its completed attempt; duplicate/stale response cannot dispatch a new attempt. Existing original queue/readmission policy is unchanged.
- Real production app+bridge tests reproduce native-ended/logical-pending gap by delivering native Terminal then delayed sourceB observation in the **same app turn**, and a separate real finite output-budget prompt/approval/new-attempt transition before the delayed observation. Neither can capture sourceB or enable replay; missing first baseline is never recaptured. Valid current preterminal sourceA is retained. Finite attempts keep one batch member/one retirement terminal. These are deterministic production-owner/bridge tests, **not physical timed filesystem source-swap acceptance**.

### HZ3-AC03 — recoverable original retirement tracking

- Original `prune_retired_jobs` scans at most the opening retired length, requeues protected IDs instead of dropping them, and removes only closed cleanup-complete terminal records outside running/retry-pending/active-or-pending-batch protection. Stops at64 or after one pass if protections require more; no busy loop and no count-driven protected eviction.
- Original completion, background-window close, form retirement and failed/finished retry recheck reuse that same owner. No persistent/second history manager. Active native cleanup-held slots and original queue fairness remain.
- Real original app submissions, Start receipts,cleanup notifications and `background_job` exercise closed completedA + runningB,70 other completions, Bends and130 further completions. RED left1orphan; GREEN has0orphan, records64/retired64, oldA/B reclaimed.
- Additional original app control holds cleanupfalse native slot, pending-confirmation relationship and retry-recheck wait across70 completions; all remain tracked/protected. Actual form close and failed recheck release those members;70more completions reclaim them while cleanupheld remains rooted. Original channel capacities unchanged; notification receivers/receipts consumed normally.

## 3. RED→GREEN and retained failure history

| Boundary | Before repair | After repair |
| --- | --- | --- |
| AC01 exact original native Job publication fixture | build0warn/error, **0PASS/1FAIL/0SKIP**, terminal missing explicit directory/zero distinction | **1PASS/0FAIL/0SKIP**; two measured native cases |
| AC01 actual native→original app summary/retry | **0PASS/1FAIL**, directory published assertion false | **1PASS/0FAIL**; current native export, safe zero-output retry control |
| AC02 actual bridge terminal-gap + finite transition + valid preterminal | **1PASS/2FAIL**, both late sourceB branches stored first baseline | **3PASS/0FAIL** |
| AC03 actual app closedA/lateB/64-bound pressure | **0PASS/1FAIL**, orphan_untracked_completed=1 | **1PASS/0FAIL**, records64/retired64/orphans0 |
| Combined seven repair app tests (incl legacy and three protected controls) | separate seam REDs above | **7PASS/0FAIL**; subsequently included in full349 |

Raw `native-red`, `ac01-desktop-red.log`, `ac02-desktop-red.log`, `ac03-desktop-red.log` preserved. Old worker for nativeRED is original accepted HZ3 **6CD1C363…14C0E1**; nativeGREEN first used current rebuilt901446E1, final current build usesB0DF89DA. AC01 deterministic nativeRED is a missing host publication-fact assertion, not a codec change. AppRED uses actual measured old terminal.

One final supplemental C# control initially passed `u8` span where original `Write` requires `ReadOnlyMemory<byte>`: **CS1503×2**, preserved `native-final-fixture-compile-failure`; fixed only test arrays, no product API change. A test-edit line-number offset was corrected before running build and is not productRED. PowerShell evidence `.Properties.Count` serialized15ones in `verification-source-drift.json`; original retained, canonical `verification-source-drift-v2.json` hasfiles15/drift[]. Console codepage displayed non-ASCII oracle path poorly; original UTF8 identities remain correct, normalized `identities-utf8.json` verified exact hashes. None is relabelled product failure/pass. No reruns seeking green for preexisting SourceChanged-vs-Cancelled instability.

## 4. Final exact verification

Every Cargo invocation checks idle `cargo/rustc/clippy-driver`, then in the **same process** sets `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`; locked/offline/serial tests and full-operation1200/3600second timeouts. No timeout/duplicate/namekill. Final tail observed0shared Rust processes and0owned fixture workers. Ordinary incremental builds, not clean/release/package builds.

```powershell
cargo build -p nxtools-archive --features isolated-codecs --bin nxtools-archive-worker --locked --offline
cargo test -p nxtools-desktop --locked --offline --bin nxtools-desktop archive -- --test-threads=1
cargo test -p nxtools-protocol -p nxtools-ipc -p nxtools-core --locked --offline -- --test-threads=1
cargo test -p nxtools-archive --features isolated-codecs --locked --offline --no-fail-fast --lib --test job_execution_contract --test eic_repair_worker_contract --test edit_interaction_worker_contract --test sevenz_edit_safety_contract --test sevenz_edit_worker_contract --test tar_edit_matrix_contract --test tar_edit_raw_header_contract --test tar_edit_worker_contract --test zip_edit_contract --test zip_edit_worker_contract --test worker_wire -- --test-threads=1
cargo clippy -p nxtools-desktop -p nxtools-protocol -p nxtools-ipc --locked --offline --tests -- -D warnings
cargo check --workspace --tests --locked --offline
dotnet build csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore -m:1 -nr:false -p:UseSharedCompilation=false
```

| Final selection | Actual result |
| --- | --- |
| Desktop archive filter | **349PASS/0FAIL/0ignored/1167filtered** (original342+7repair), actual current native export |
| Protocol / IPC / Core | **91 /43 /100PASS** (Core93+6+1), total234,0FAIL/ignored/filtered; doc-tests0 separate |
| Archive lib+11 integration targets | **100PASS/0FAIL/0ignored/0filtered**, not full45target suite |
| Archive target detail | lib42, editinteraction2, EIC4, jobexecution2,7zsafety4,7zworker1,TARmatrix8,TARraw4,TARworker3,wire19,ZIPedit10,ZIPworker1; raw4covers20samples |
| Repair-only app seven tests | **7PASS/0FAIL** before full349; final transcript-fallback exact1PASS additionally |
| Affected Desktop+Protocol+IPC strict Clippy tests / workspace tests check | both exit0 |
| C# final test-host build | exit0,**0warnings/0errors** |
| effects→exact23LIST→same-nameRUN | **23PASS/0SKIP/0FAIL/0not_run**, each selected1/run1 |
| scopedRustfmt7 / source diff15 | exit0 |
| spec / component catalog / release / editor / test-selection static | all exit0; spec118documents/14owners; source/doc only |

Final native exact names (each invoked as `<Tests.exe> --fixtures --list '<name>'` then `<Tests.exe> --fixtures '<identical name>'`; argv,exits,summary,time/log in `native-final/results.json`):

1. `archive HZ3 AC01 native directory publication zero output`
2. `archive HZ3 AC01 publication wire retained directories`
3. `archive space fixture held read-only observation`
4. `archive space fixture native allocation identity`
5. `archive space fixture missing suffix target binding`
6. `archive space fixture expected source target changes`
7. `archive space fixture capacity byte override`
8. `archive space fixture typed native open failures`
9. `archive space fixture native reparse refusal`
10. `archive space fixture cancellation publication`
11. `archive space fixture strict wire private diagnostics`
12. `archive space fixture original dispatcher retained bounded slots`
13. `archive space fixture detached cancellation no late receipt`
14. `archive Phase E source all-volume protection missing`
15. `archive Phase E source single ZIP exact disk count`
16. `archive EIC AC01 native directory merge zero publish`
17. `archive EIC AC02 native cancelled reply only ZIP TAR 7z`
18. `archive ZIP edit native handoff gates and ownership`
19. `archive ZIP edit native handoff failure unknown no retry`
20. `archive tar edit native source cancel handoff`
21. `archive sevenz edit native source cancel handoff unknown`
22. `archive preview fixture actual output budgets`
23. `archive job fixture actual conflict global admission cancel`

Effects: explicit matching frozen worker/hash and explicit CPython/stock7Zip/bsdtar envs; GUID-owned tiny archives/source/targets/exact owned children only, original native seams and no GUI/Shell/system/user archive effects. `native-final.ps1` contains exact env bindings. Rust full349 explicitly uses `native-final/native-terminals.json`; fallback separate test omits env intentionally. New C# fixture unknown/null and strictbool tests exercise original native owner, not permission changes. Skips would fail accounting, not pass.

Static exact entries: `python tests/architecture/spec-contracts.py`, `python tests/architecture/gpui-component-catalog.py`, `python tests/architecture/gpui-release-contracts.py`, `python tests/architecture/markdown-editor-contracts.py`, `powershell -NoProfile -ExecutionPolicy Bypass -File tests\architecture\platformhost-test-selection.ps1`; `rustfmt --edition 2024 --config skip_children=true --check` on seven changedRustpaths, `git diff --check --`15sourcepaths. Expanded args/exits/times are in `command-ledger.json`, per-command JSON/rawlogs, `test-counts-v2.json`, `static-final.json` and scripts. Source/static tests do not certify native/physical/visual acceptance.

## 5. Build and source identities

Final frozen/current worker **B0DF89DAE9527D0E16471434ABBCDCFA949DAEAD0CCB07878293333491DC7F7C**:
`hz3-ac01-ac03\frozen-final\nxtools-archive-worker.exe` equals current `target\debug\nxtools-archive-worker.exe`. No source changes after final compilation/checks.

ActualTestsDLL **2C60260A92199D080101F74E52387D22DBEE03873470FE71503692F178C7436D**;
actualHostDLL (testcopy and productDebug output equal) **669BD7D522672513F649E2FA749A174C46B9A69FED1B01E7AF4C110D755D1E5E**.
CPython **7D96A4ED35D6E596CD9DD8933FEAFC66349CC21F75BE3B15C89FD336E50140C1**;
stock7Zip **4BCC005758C5E604C0AAA257B6097846A4A708E62A9C0C7471FA360FAD863F1F**;
bsdtar **92FAFED485AF0FE4C291457DC3C00D0241F6A136731BAB63EF18A3F0C949D37A**. ExactUTF8paths in `identities-utf8.json`.

All15tasksource/fixture hashes in `delivery-source-hashes.json`:

| Path (workspace relative) | SHA256 |
| --- | --- |
| `rust/apps/nxtools-desktop/src/archive/jobs.rs` | `BA943823C69DEB1FB3D2C59E783C926150C54CF642D115DC9EF955849922CE91` |
| `rust/apps/nxtools-desktop/src/archive/batch_jobs.rs` | `3901B79BBB0B5C8DF8DAFE239AC5730B3B0966A9478832A47211334EF653E807` |
| `rust/apps/nxtools-desktop/src/archive/job_tests.rs` | `7AF4257337853F31AB647B1ECAAE94D8E5BD9EA4C34214FCA794D42660DC405A` |
| `rust/apps/nxtools-desktop/src/archive/fixtures/hz3-native-publication.json` | `E959B48F14258EDFD27A0BF2A8345BB5ECE8A9720DE1F30DE33BB424A9FB36BC` |
| `rust/crates/nxtools-protocol/src/archive_job.rs` | `81B96BF400C7C30871EA338961C8D51560E6010617A77DA7AF6E7BBD7CE91DA7` |
| `rust/crates/nxtools-ipc/src/archive_job.rs` (test initializer only) | `73D351839F8CFCB9657B7811A5E9DE2186A8107509C1D8A415A0681F741D3107` |
| `rust/crates/nxtools-archive/tests/job_execution_contract.rs` (initializer only) | `3FA4B9F4E7261F962ACC2ECB040807DE6726F61B5CD21F0C6496C527FD861FEF` |
| `rust/apps/nxtools-cli/src/archive/execution/tests.rs` (initializer only) | `94DDC16CA3334EF0AF64B8BC379BDA3C4150026B037F029D511A1F0FDB39B1F0` |
| `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs` | `199A2A28AC0E59FF6EF89C201A261F9D86FF7A2771088FC99ACD551A9EEAFFF3` |
| `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs` | `B923B067E097F265BB35DB2AC8853D7EA3E85C0E4D15AB760EA2549F20552B2F` |
| `csharp/NxTools.PlatformHost/Protocol/ArchiveJobContract.cs` | `73AEBB1028EEAA2F1F8E0C64A70677BEA43DECF31C613D64E473809915B67134` |
| `csharp/NxTools.PlatformHost/Protocol/ArchiveJobSession.cs` | `5004A8450E50D9CE9065F3B82D25A212F1FD9E7EFEEC7A0CCC5E30032E9691AF` |
| `csharp/NxTools.PlatformHost.Tests/Hz3BatchTests.cs` | `8B28320BDCFDA243168DA475343467C979FD93FB9E54275920D8390B9282291F` |
| `csharp/NxTools.PlatformHost.Tests/Program.cs` | `ACC51A96C1869CB71FD2710331C28BA446111B5E0B34110F1E4F2ADE26F29ED1` |
| `csharp/NxTools.PlatformHost.Tests/TestEffects.cs` | `1ECBF9AFE06F918F1952B25DDDE205169EEA5C5A24D078D843C51CDC0426E622` |

Seven production files only: original Desktopjobs/batch bridge, RustterminalDTO and four nativeOutput/Runner/JobDTO/JobSession owners; other eight are tests/registration/measuredfixture. IPC/CLI/archive-test constructor initializers addNone only; CLI product behavior unchanged. Original queue/application/nativeSourceLease/VolumeSources/CreationSources/OutputBridge/Rustworkerengines/codecs/EIC/TARraw/ZIP7zplanners/Cargo/vendor/installer untouched. Owner docs updated, not old outcomes.

## 6. Protection, evidence levels and remaining work

- Fresh736wide opening→delivery:12expected modified source/test files,724unchanged. New2files and CLItest initializer outside that prior selection are included in15tasksource hashes. During final tests: **736-file wide baseline and15tasksource baseline both zero drift**.
- Bounded opening202copies/hashes:10expected owner/DTO/testregistration changes,**192unchanged**; all**159saved older task reports** unchanged, including HZ3refusal/delivery and acceptedEIC/TARhistorical negative evidence. Queue and accepted engine hashes remain exact. Checkpoint stayed original202215bytes through verification, only later prepended; entire opening suffix proof sealed separately.
- Evidence differentiates actual GUID-owned native publication, current-native-terminal→actual GPUI app/original IPC bridge, deterministic native-terminal/finite-attempt order and transcript replay. Parent source-extracted probes remain valid at their declared layer and untouched; no claim of physical timed source replacement/full product GUI/user acceptance follows from them or these tests.
- **OPEN/unrun**: old `source_unavailable`, CLI `helper_untrusted`, FB-AC05, original ZIPreadmission `SourceChanged` versus `Cancelled` instability (old negative preserved, chain not rerun seeking green); physical Explorer/drop, mixedDPI/IME/UIA/theme/visual/user/installed/release acceptance. No full45Archive/100k/resource/performance/default fullnative/product test. CLI test initializer compiled by workspace check; no new CLI product acceptance claim. Broad historical green never overrides these limits.
- No in-scope implementation/check blocker remains. **Await narrow independent acceptance of HZ3-AC01–03; HZ3 itself remains acceptance-withheld until parent decides. Stop now, rely on host completion callback, no duplicate communication task and no automatic Step6/HZ4 or Step7/HZ5.**
