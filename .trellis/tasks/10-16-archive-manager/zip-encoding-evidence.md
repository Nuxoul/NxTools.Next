# ZIP Auto filename candidates and safe reprojection — B1/C4

2026-10-20. Implemented and targeted checks passed; **submitted for independent acceptance**, not Phase A or A–F completion. Only [current checkpoint](phase-a-checkpoint.md) owns current state. Historical ledger/reports/results are unchanged. [Source boundary](research/zip-encoding-sources.md); [structured results](zip-encoding-results.json).

## Delivered behavior

- Valid UTF8 flag / CRC-bound valid Unicode Path extra remain authoritative, ahead of manual pages and Auto. No Unicode normalization, sanitization or authority override.
- ZIP-only Auto considers strict UTF8, GBK and ShiftJIS. Require at least four non-ASCII letters. UTF8 requires alphabetic non-ASCII characters; GBK requires Han; ShiftJIS permits Han/kana and requires two full-width kana to select. All-Han/half-width-only ShiftJIS alternatives may veto another candidate but cannot select themselves. Two plausible candidates, short or unsupported evidence -> existing CP437 fallback/UncertainCp437 and manual choice. Example `日本語のファイル.txt` is genuinely competing GBK/ShiftJIS, so remains uncertain. A ShiftJIS sample with an additional half-width kana has invalid GBK and enough full-width kana and selects ShiftJIS. No language-wide correctness claim, statistical score, global voting or framework.
- Unique candidates carry `Heuristic(page)` and **still** `UncertainEncoding`. Existing worker/product provenance/codepage fields expose Heuristic with65001/936/932; Rust/C# validate these symmetrically (C# only for Auto). DTO shape, protocol versions, product limits and helper trust unchanged. No dependency changes. TAR's old decoder is unchanged.
- At most fallback + one candidate + current strict decode; additional candidate scratch<=6×raw bytes is admitted before candidate decoding against remaining original metadata/work budgets. Authoritative/manual/ASCII names bypass candidate admission. Existing ZIP central-record limits/checkpoints remain.
- `ArchiveEngine::reproject` rereads the original bounded ZIP directory, checks source revision before/after, preserves original session/ordinal/raw bytes and reruns original path annotation. Fresh ExtractionPlan refuses dangerous, case and file/directory-prefix conflicts before output. Plans share their immutable entries Arc; extract rejects an old projection plan before ANY output call. No second safety framework/state machine.
- Big5/CP949/1250/1251/1252 use independent small raw-byte literals. Existing GBK/ShiftJIS/CP437 and Unicode-extra CRC cases retained.

## Red → green and real boundaries

1. Before implementation: new domain Auto test1FAIL/exit101, real ArchiveReadClient→typed IPC→C# ArchiveReadSession→original runner/Job→actual isolated worker1FAIL/exit101, exact C#LIST1/RUN1/FAIL1/skip0/exit1. All observed wrong CP437 display/actual output name for the independent raw GBK ZIP. Logs `b1-red-domain.log`, `b1-red-product.log`, `b1-red-csharp-{list,run}.log` in session scratch.
2. Missing reproject API first produced E0599 (not runtime evidence). After adding it, old-safe-plan/new-dangerous-projection test was actual3PASS/1FAIL: extract reached the output capability (`IoFailure` instead of `SourceChanged`). Arc projection binding -> same4/4PASS with0output calls. Native fresh unsafe/case/prefix requests independently produced typed rejection,0commits,empty output roots.
3. Actual product metadata green checks exact GBK name/Heuristic936 and short ambiguous CP437/UncertainCp437437, original ordinal0/snapshot/batch counts, warning,29bytes and real cleanup. Independent Python raw-name fixture uses no product encoder. Original native output checks exact name and literal29bytes `independent encoding payload\n`; manual936 works; all3unsafe/collision cases write nothing.

## Actual commands and final results

All Cargo commands serial in the workspace root, with `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, offline. C# apphost built BEFORE matching isolated-codecs worker; worker execution/relink serial. Temporary files/logs under `C:\Users\Administrator\.pi-desktop\scratch\949da288-1ef9-46b3-a698-90f543613cf5` (logs begin `b1-`).

| Actual command | Result |
| --- | --- |
| `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore` | exit0,0warnings/errors (`b1-final-dotnet.log`); earlier new-test uint/int compile failure retained, fixed |
| `cargo test -p nxtools-archive --features isolated-codecs --offline --test encoding_contract --test engine_contract --test extraction_plan_contract --test transaction_contract --test zip_boundary_contract --test worker_wire --test metadata_wire_contract -- --test-threads=1 --skip default_budget_lists_one_hundred_thousand_independent_entries` | final40pass/0fail/ignored,7targets,1filtered (`b1-small-final.log`); encoding final4/4again after fresh-plan assertion (`b1-encoding-final.log`) |
| `cargo test -p nxtools-archive --features isolated-codecs --offline --test product_read_chain product_auto_encoding_reports_actual_candidate_and_uncertain_fallback -- --exact --test-threads=1` | final1/1,23filtered,exit0 (`b1-product-final.log`) |
| Earlier `cargo test -p nxtools-archive --features isolated-codecs --offline --test product_read_chain -- --test-threads=1 --skip product_open_and_test_preserve_local_long_source --skip product_100k` |24/24exit0; **incorrect skip names did not exclude existing long-source test**, unintended old local-long-path check ran once. No repeat/expanded acceptance claim |
| Earlier same7archive targets with `--skip independent_100k --skip long_` |41pass; **incorrect skip names did not exclude old domain100k test**, ran once. User excluded100k this turn: this was a test-selection mistake, not necessary or newly authorized acceptance; no repeat/performance sampling |
| `cargo test -p nxtools-protocol --offline --lib`; `cargo test -p nxtools-ipc --offline --lib` |54/30pass,0fail/ignored,exit0 |
| `cargo clippy -p nxtools-archive -p nxtools-protocol --features nxtools-archive/isolated-codecs --tests --examples --offline -- -D warnings`; same without feature |both exit0; isolated repeated after final assertion |
| `cargo fmt --all -- --check` |exit0 |
| `cargo build -p nxtools-archive --features isolated-codecs --offline --bin nxtools-archive-worker` |exit0; final build precedes final C#execution |
| C# apphost `--fixtures --list` then `--fixtures`, same5filters below |LIST5=RUN5=PASS5,skip/fail/not_run0,exit0 (`b1-close-csharp-{list,run}.log`); exact names compared |
| `python tests/architecture/spec-contracts.py`; `python .trellis/scripts/task.py validate .trellis/tasks/10-16-archive-manager`; `powershell -NoProfile -File tests/architecture/platformhost-test-selection.ps1`; `git diff --check` |exit0. Specs106documents/14owners; context82/79. Reference32732bytes; only pre-existing native-contracts35497byte injection warning remains |

Exact C# filters:
- `archive encoding fixture Auto manual native output`
- `archive product fixture strict wire validation`
- `archive product fixture metadata sequence and budgets`
- `archive ZIP remediation fixture directory and comment boundaries`
- `archive extraction actual Rust handle-session fixture`

Environment: `NXTOOLS_ARCHIVE_WORKER_FIXTURE=<workspace>\target\debug\nxtools-archive-worker.exe`; `NXTOOLS_ARCHIVE_ENCODING_FIXTURES=<scratch>\b1-encoding-inputs` (Python producer explicitly run); `NXTOOLS_ARCHIVE_ZIP_FIXTURES=<workspace>\rust\crates\nxtools-archive\tests\fixtures`; `NXTOOLS_ARCHIVE_PYTHON_ORACLE=C:\Users\Administrator\AppData\Local\Programs\Python\Python313\python.exe`. No SKIP treated as PASS. Fixed helper absent and0archive workers/test hosts/Cargo/rustc at close; input ZIP hashes retained in scratch.99pre-existing task files other than current checkpoint unchanged at pre-document closeout.

One optional bounded read-only review was requested then stopped without a completed report; **not review PASS**, no second review. Independent acceptance is pending. No workspace-wide build/test, product UI,1GiB/largeZIP64/performance,packaging/install/sign,system/sourceRecycle,commit/push or next feature. Two unintended old regression executions are disclosed above. Remaining B2/B3,C1/C2/C3/C5,D1–D3 remain open; nativeRecycle remains native_protection_unproven/NotAttempted.

Final documentation closeout: spec/context/diff gates exit0;99historical task hashes unchanged and original checkpoint historical bytes reconstruct to its opening SHA256. The separate whole-unrelated-diff scratch comparison is NOT a byte-preservation proof: opening PowerShell Set-Content snapshot lost non-BMP characters (emoji became `??`), while current source retains them. No writes were made to those unrelated paths. No further tool work is required for this slice.
