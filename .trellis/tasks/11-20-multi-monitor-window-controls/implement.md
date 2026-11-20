# Implementation plan

## Readiness
- [x] Task-creation/planning consent obtained.
- [x] Reference notes, recorded source chains and existing owners inspected; evidence in research/boundaries.md.
- [x] R5 confirmed: new-task parallel connections default to 16; simultaneous-task queue concurrency is unchanged.
- [x] PRD converged and final planning summary prepared for approval.
- [x] Final summary approved for implementation; task activation follows the approved scope.

This task implements seven related feedback items in one task; the order below is execution order, not a renumbering of the existing archive phases. Archive integration remains Phase F — Explorer integration and genuine native drag (P206/P207).

## Ordered work
1. Refresh dirty state and applicable specs; preserve preexisting installer/PlatformHost/runtime admission work. Add newly verified source chains/invariants to owning reference notes before production changes.
2. Use research/listary-placement.md as the primary placement/lifecycle reference. Add red-capable monitor-boundary tests for C# snapshot serialization -> actual Rust panel placement -> device-pixel command, with native HMONITOR IDs unlike ordinals, negative monitor origins, nonzero taskbar work-area origins, 96/120/144 DPI, first show and cross-monitor reopen/reuse/preview. Assert cursor rather than foreground monitor selection and Listary's actual-outer-width horizontal center/work-height-quarter top edge. Then repair R1 and R2 at original owners.
3. Add actual archive job/batch show/reuse/terminal/close activation regressions, then route intended presentations through existing confirmed native activation. Explicit background and passive surfaces must not activate or resurrect.
4. Add actual controlled-browser confirmation/Held/Commit progress regressions and repair R4 through the original pending/keyed lifecycle. Cover duplicate event, close/later event, disabled automatic progress, QueueOnly, delay, abort and source-anchor retirement. Preserve ordinary immediate submission and explicit start/resume/retry semantics.
5. Set Core's new-task parallel connection default to 16. Add missing/default/explicit-value/new-request propagation tests. Preserve existing settings and task requests; assert simultaneous-task queue concurrency remains unchanged.
6. Add actual main-page Switch pointer/keyboard and temporary-config tests plus native enabled/parser/hotkey/capture fixtures. Implement R7 section-owned persistence, native apply and local generation gates. Test save failure, disconnect/reconnect, queued events, delayed loads/previews/reveals, temporary pause/deadline and re-enable with retained history/preferences.
7. Add install-entry/readiness/original-user/terminal-forwarding and transaction restoration regressions before R6 changes. Mandatory automatic Apply after successful finalization must not depend on optional UI launch. Preserve explicit upgrade opt-outs. Repair failed restoration evidence and uninstall via original owner. Tests inject stores/default claims/ledgers/process boundaries; actual installs are separately authorized.
8. Run scoped compiled/format/lifecycle checks, then one integrated verification. Update owning specs/references with final executable contracts. Report evidence levels and remaining native acceptance. Do not package, commit or mutate live integration/services implicitly.

## Verification selection
Before each command state the concrete failure it detects and the consequent change. No ceremonial reruns. Run only one Cargo operation at a time, with full-operation timeout or background polling and logs in session scratch:

```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
```

| Check | Detects / response |
| --- | --- |
| New production-boundary panel/preview tests | Wrong monitor ID, origin/scale/work-area conversion, stale reveal or reentry; fix the actual sampler/placement/generation owner |
| Core default/serde/section tests | Wrong R2/R5 defaults or lost explicit values; fix owning defaults and section merge |
| Existing/new controlled-handoff + keyed-close tests | Progress opens while Held, never opens after Commit, duplicates or resurrects; fix pending admission/identity consumption |
| Archive job production tests | Missing native activation request/confirmation, stale result or background focus theft; fix show/terminal/current-surface routing |
| Main-page production Switch tests | Double callback, no actual persisted disable, wrong semantic state or disabled re-enable; fix page/setter/apply/guard boundary |
| Protocol Rust/C# round trips and native fixture fakes | Snapshot or enabled field dropped/misparsed, hotkey/capture still enabled; fix original serializer/parser/native owner |
| Integration fake store/ledger/default-choice fixtures | Capture/compensation/restoration/ledger retirement errors; preserve original transaction and incomplete retry state |
| Installer source gates and non-effect entry fixtures | Automatic entry gated by optional launch, wrong ordering/context or lost terminal result; fix original callback/entry |
| cargo fmt --all -- --check | Formatting differences; format task-owned source only without changing unrelated files |
| cargo test -p nxtools-core --lib; cargo test -p nxtools-protocol --lib | Defaults/config/wire regressions; fix relevant owner |
| cargo test -p nxtools-desktop -- --test-threads=1 | Integrated GPUI state/lease/window and cross-feature regressions; diagnose concrete failure without repeated green hunting |
| Scoped cargo clippy --tests -- -D warnings; cargo check --workspace --tests | Type/lint/cross-package contract defects; fix scoped production/test errors |
| dotnet build csharp/NxTools.PlatformHost/NxTools.PlatformHost.csproj --no-restore | C# API/type mismatch; fix parser/adapter/integration owner |
| Exact reviewed C# --fixtures filters, listed before execution | Native-boundary fake/owned HWND failures; fix corresponding owner; no broad --native selection |
| Affected quick-launch/clipboard/download/desktop-shell lifecycle gates | Lost source lifecycle boundary; fix original boundary rather than weaken assertions |
| python tests/architecture/gpui-release-contracts.py; python tests/architecture/markdown-editor-contracts.py | Unintended locked release/vendor/editor delta; remove out-of-scope edits |
| python tests/architecture/archive-phase-f-shell.py; python tests/architecture/installer-update-lifecycle.py | Lost integration/installer ownership wiring; fix new flow while preserving existing installer changes |
| python tests/architecture/spec-contracts.py | Broken owner-spec navigation/evidence contracts after spec edits; repair documentation |

Add exact new test names/filters when the red tests exist. Existing download filters include controlled_, download_window_lifecycle and a_native_task_window_close. Source gates alone do not prove a visible/focused HWND.

## Separate native acceptance
When authorized, verify actual monitor layouts with primary/right/left/above displays and 100/125/150% DPI, taskbar work areas, hotkey summon/reopen, passive preview pointer/keyboard, notification/link positions, foreground secondary windows and browser-download progress. Installation coverage requires normal/silent/launch-unchecked, original-user/elevated context, already-running Desktop, protected redirect, failed Apply/retry and uninstall restoration preserving later user changes. Installer fixture scripts that execute isolated installers also require explicit authorization. Indexer MFT/USN/cache code is out of scope; no LocalSystem release gate is triggered by this plan alone.

## Rollback boundaries
Use scoped edits/diffs, never reset unrelated changes. Before native acceptance keep all verification on synthetic files/stores/owned windows. If integration fails after finalized package files, retain truthful registration failure and retry facts rather than pretend package rollback. Restore prior live setting on save failure; do not invent a config migration framework or overwrite old explicit values.
