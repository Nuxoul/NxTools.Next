# HZ1 AC01 / AC02 / AC03 independent repair acceptance

2026-10-28. **AC01 PASS / AC02 PASS / AC03 PASS at source and automated boundaries.** This is independent acceptance of the scoped repair, not overall A–F completion, physical/user visual, measured peak or package/install acceptance. No additional confirmed in-scope blocker was found. Previous PARTIAL acceptance remains unchanged as the pre-repair record.

## Review and decisions

Read submitted transcript, current checkpoint, repair outcomes, original independent findings, applicable Desktop/Protocol/PlatformHost contracts, current production chain and affected regressions. Inspected original Desktop/native RED logs: compiled Desktop1PASS/3FAIL (close, actual Start submit, missing-target demand), native RUN1/FAIL1 at exact missing-directory count. These are original recorded REDs, not new pre-fix runs in this acceptance. No repair or broad/repeated delegated review was performed.

### AC01 — PASS

Original owner now calls `retire_form_space(form)` only on accepted submission and `retire_form(id)`. The helper retains other sessions and drops only the matching UI pending/deadline; bounded transition diagnostics use original operation IDs. No native cancellation/slot-release/replacement-worker command was added. Failed submission does not invoke cleanup. `space_result` rejects absent retired IDs.

Independently passing original close regression plus actual official Start pointer/accepted-submit, second-form receipt, late retired receipt, failed-submit/current-state and close isolation regressions support closure. Native retained-slot/cancellation fixtures independently pass as well. This does not claim a blocked Win32 call can be interrupted.

### AC02 — PASS

`ArchiveOutputSession.ObserveSpace` returns checked uint count directly from held `pendingRootParts.Length`; `ArchiveSpacePreflight` forwards it without IO/probe/create/source mutation. Target identity/suffix hash semantics and native checks remain original. Rust receives optional `missing_destination_directories`, checks0..64 and flag consistency/current IDs before plan application. Missing old-v1 field with missing target remains unknown; known existing target needs no missing-directory allowance.

`space::estimate_observed` reuses original member calculation and adds count × real observed allocation unit exactly once with checked multiplication/addition. Unknown metadata/size/count and overflow do not become certain zero. Here/Name/Smart wrapper semantics remain separate; no source/overwrite credit/reserve/budget/concurrency change. Complete empty-archive exception remains explicitly scoped to existing no-output behavior.

Independent actual browser→extract-form→correlated result regression now reports8192B for one0B file + Here + two missing target components +4096unit +0available. Plan shortage tests agree; legacy absent count, contradictory/>64/wrong-type values, existing/single/two-level destinations and target appearance/replacement are covered across Protocol/Rust/native fixtures.8192B is advisory allowance consistent with the estimator, **not measured disk-allocation or peak guarantee**. The app/form test injects correlated observation facts; native observer fixtures establish actual count separately. No complete production named-pipe end-to-end run is claimed.

### AC03 — PASS

Owned PathBuf allocation was replaced by borrowed `std::path::Path::new(path)` equality. Original matching semantics retained. Independent Desktop+Protocol strict Clippy --tests -D warnings exits0 without suppression. All/selected browser identity regressions remain green.

## Independent execution evidence

Directory:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\hz1-repair-acceptance-20261028-134306`

The exact directory is also recorded by session scratch `hz1-repair-acceptance-work.txt`; `results.json` and `accounting.json` retain arguments/exits/counts. Commands were serial and each Cargo checked idle lane, inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256. No timeout/duplicate build/process stop. Normal incremental Cargo/MSBuild outputs were reused when current; not clean isolated rebuild claims.

| Independently executed command/check | Result |
| --- | --- |
| cargo test -p nxtools-desktop --bin nxtools-desktop --locked --offline hz1 -- --test-threads=1 |18PASS/0FAIL/0ignored;1.74s test execution |
| Same Desktop command with archive filter |279PASS/0FAIL/0ignored/1157filtered;104.86s test execution,106.24s command |
| Icon/WAC within archive run |8icon/12WAC PASS; exact names counted, not additional runs |
| cargo test -p nxtools-protocol --lib --locked --offline |86PASS/0FAIL/0ignored |
| cargo clippy -p nxtools-desktop -p nxtools-protocol --tests --locked --offline -- -D warnings |PASS exit0 |
| cargo check --workspace --tests --locked --offline |PASS exit0 |
| dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -m:1 -nr:false -p:UseSharedCompilation=false |PASS exit0,0warnings/0errors; normal incremental build |
| Same C# project --no-build --no-restore -- --fixtures --list, then same filters without --list: 'archive space fixture' 'archive destination fixture missing low space confirmation' |Ordered LIST=RUN=PASS12/SKIP0/FAIL0/not_run0;11observation+1original execution-confirmation fixture |
| rustfmt --check --edition2024 --config skip_children=true on six touched Rust files |PASS |
| scoped git diff --check on ten touched code/test files |PASS |
| spec-contracts.py / platformhost-test-selection.ps1 |PASS114documents/14owners; static evidence/classification only |

C# reviewed fixtures use GUID-owned tiny files/directories/symlinks, held native facts, byte overrides and original bounded STA/publication. Original execution-confirmation fixture reused existing target/debug archive worker; not rebuilt/Release-approved worker provenance. No default full C#/native, full Desktop or engine suite was executed.

## Preservation and remaining boundaries

Independent baseline covers current source/protocol/IPC/native/tests,Cargo files, prior independent report, checkpoint and installer. Test-time drift.json is empty. Round-start repair snapshots were independently compared: exactly10code/test files changed; original forms_space plus9protected WAC/icon/titlebar/assets/installer-helper/Cargo files unchanged. Original independent PARTIAL report byte-identical; repair's previous151180byte checkpoint is verified exact suffix. This acceptance preserves the current153979byte checkpoint as an exact suffix after its new status prepend. Final preservation/statics are recorded separately in the evidence directory.

No product edits during acceptance; only this report and sole current checkpoint status are written. No packaging/signing/install/product launch/Registry/service/ACL/UserChoice/commit/push. Existing installer32 remains unchanged and does not include this un-packaged repair. No HZ2–HZ5/source truncation/punch/recycle or dependency/vendor change.

Remaining NOTRUN: actual Windows/DPI/mixed monitors/IME/UIA/focus/Explorer OLE/user visual; real restricted-user ACL/mount swaps/quotas/FAT/remote/disk exhaustion/huge archives; actual capacity-peak measurement, production named-pipe end-to-end and release/install acceptance. Historical unrelated production-only lint/globalfmt and broader product acceptance limits are not closed by this scoped --tests success. Current AC01/AC02/AC03 source/automated acceptance has no remaining blocker.
