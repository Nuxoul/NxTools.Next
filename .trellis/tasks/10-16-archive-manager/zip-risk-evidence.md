# B3 ZIP high-expansion confirmation — implementation evidence

2026-10-20. **ZIP current-budget risk confirmation implemented; scoped checks PASS; pending independent acceptance.** Not full B3/SAFE04/Phase A/A–F. [Sources](research/zip-risk-sources.md), [results](zip-risk-results.json), [current checkpoint](phase-a-checkpoint.md). [ZIP conflicts](zip-conflict-acceptance.md) and [password](password-operation-acceptance.md) are independently accepted historical slices, not reimplemented or fully rerun. Sequence16 package is not compression acceptance/deployment evidence.

## Rules / minimal connection

- Production warning is exactly member expanded size>64MiB AND expanded>1000×known positive packed size. Strict integer/u128 comparison, no floating confidence/division or new threshold. Unknown/zero packed size has no invented ratio. Boundary unit assertions cover equality, one-over, unknown and overflow-scale values.
- Original ZIP path/internal collision, local/header/extents and declared budget checks precede risk handling. Per-member warning sits before actual current-member decode and file partial. Original Rust InteractionPort Risk installs ticket before private risk_prompt; C# RespondRisk only forwards original stdin answer. Original source/worker/Job/outputsession/timeout/cancel own lifecycle; no new runtime/framework or public ArchiveRead modification.
- This delivered portion confirms ONLY within original immutable admitted ResourceLimits. approved_limit must equal original entry limit and cover declaration. Smaller/larger/stale/duplicate answers do not approve. Larger-budget escalation/new admission is **not implemented**, rather than mutating an accepted request/Job. No limit or dictionary/output/metadata check is disabled. No automatic retry or cross-task/permanent trust. Each risky ordinal has a fresh prompt, no approval cache.
- Existing streaming actual member+total counters, codec/work/dictionary admission, full CRC/size/end/source checks and original verified native publication remain. Refuse/cancel stops later members; current file never publishes; owned unpublished partials removed, old/foreign files and previous committed output/count retained. Corruption/path/internal collision cannot be approved away.
- No-interaction public output adapter returns ResourceLimit for a production-risk member, not implicit approval or permanent UnsupportedFormat. Ordinary output adapters/other formats remain unchanged. C# typed extraction cancellation keeps cancelled/cancelled_with_committed_output truthful. Safe low-noise awaiting_risk/risk_confirmed/terminal diagnostics use original correlation, fixed fields/count/elapsed only.

## Small independent fixture and real red → green

.NET ZipArchive independently produces tiny actual payloads (<=8448bytes/ZIP). Production-threshold test changes only declared local+central size to64MiB+1; actual data is4096bytes, never a64MiB expansion or compressed bomb. First real original runner/Job/worker regression LIST1/RUN1/FAIL1/skip0/exit1 (`risk-red-run.log`) proved missing AwaitingRisk. Same declaration pending+cancel is green (`risk-green-run.log`).

For positive actual publication at small size, DEBUG-only internal C#test-apphost seam supplies positive thresholds1024bytes/2×, each <=production. This can ONLY increase test sensitivity, never disable/increase warning thresholds; strict Rust validates range/extract/schema/read restrictions. Release source excludes the field and strict serde rejects it. No environment/public setting or relaxed budget. Release compilation/runtime not performed or claimed.

Final newfixture13small scenarios through original C#runner/Job/actualisolatedworker/nativeoutput:
ordinary no prompt; production declaration pending/cancel; exact-confirmed actual content; typed decline; original cancel after prior256byte commit; two risky members/stale firstticket; larger-answer ignored then current finite answer; understated declaration with actual entryoverflow; actual totaloverflow after priorcommit; declared limit refuse before prompt; CRCdamage after confirmation; unsafe member and internal casecollision before prompt. Assertions check installed pending,0currentwrites/partials before approval, original limits in prompt, source writer denied during wait,1originalworker/norespawn, latermember absent after decline/cancel, exact native4096bytes, actual commitcounts, oldtarget/foreignpartial preserved, ownedpartialcleanup, sourceexclusivereopen, retired/priorjobtickets, correct categories and safe correlated logs. No fakeworker success or InteractionPort-only substitute.

## Actual commands and counts

All Cargo calls inherited CARGO_BUILD_JOBS=1/CARGO_INCREMENTAL=0/RUSTFLAGS=-C codegen-units=256, offline and serial. C#isolatedapphost built first; matchingisolatedworker rebuilt AFTER checks before actual C#execution. No100k/longpathmatrix/1GiB/largeZIP64/resourcesampling or global/workspace suite.

| Actual command | Result |
| --- | --- |
| `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore` |exit0,0warnings/errors; risk-final-dotnet.log |
| `cargo test -p nxtools-archive --features isolated-codecs --offline --test interaction_contract --test worker_wire --test transaction_contract --test extraction_plan_contract --test zip_boundary_contract --test metadata_wire_contract -- --test-threads=1` |31pass/0fail/ignored,6targets (4+14+5+5+2+1),exit0; risk-small.log |
| `cargo test -p nxtools-archive --features isolated-codecs --offline --lib zip_risk::tests -- --test-threads=1` |2pass/14filtered,exit0; risk-domain.log |
| `cargo clippy -p nxtools-archive --features isolated-codecs --tests --examples --offline -- -D warnings`; same without feature |both exit0; risk-clippy-isolated/default.log |
| `cargo fmt -p nxtools-archive -- --check` |exit0; risk-fmt.log |
| `cargo build -p nxtools-archive --features isolated-codecs --offline --bin nxtools-archive-worker` |exit0; risk-final-worker.log |
| C#apphost --fixtures --list then same exact filters --fixtures |LIST4=RUN4=PASS4,skip/fail/not_run0,exit0; risk-final-list/run.log; exactnames compared |

Exact C#filters: `archive risk fixture actual ZIP confirmation`; `archive product fixture actual open test privacy`; `archive ZIP remediation fixture directory and comment boundaries`; `archive extraction actual Rust handle-session fixture`.
Env: worker=<workspace>/target/debug/nxtools-archive-worker.exe; ZIPfixtures=<workspace>/rust/crates/nxtools-archive/tests/fixtures; codecfixtures=<ZIPfixtures>/upstream; Pythonoracle=existing Python313/python.exe. Original product privacy fixture generates its tiny3member input via existing Python producer. No skip treated as PASS. Early test-line placement corrections were preparation only, not runtime red evidence.

Logs/commandledger/baselines under session scratch `C:\Users\Administrator\.pi-desktop\scratch\949da288-1ef9-46b3-a698-90f543613cf5`, prefix risk-.111historical task files unchanged before checkpoint update; previous whole checkpoint retained byte-identically in phase-a-checkpoint-before-zip-risk.md. No broader preservation claim for unrelated concurrent code without its baseline. No formalprogram/package/install/sign/commit/push/system/UNC/sourceRecycle/delete/dependency action. No optional review loop. Remaining: independent ZIP-risk acceptance, larger-budget escalation and otherformats/fullB3/SAFE04/PhaseA/A–F. Stop this slice.
