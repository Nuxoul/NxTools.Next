# WAC01–03 password / host focus repair

Scope: only the three P2 findings in window-tabs-icons-drag-acceptance.md. Original independent1398unit+10integration/related219/Core93/IPC43/Protocol83/C#6 PASS are the accepted baseline, not new execution here. Original acceptance remains immutable historical evidence. No package/install/product GUI/Registry/association/service/commit/push, dependencies/vendor/C#/wire/worker/config/budget changes. Current dirty files were read and task-specific pre-round copies stored in session scratch/wac-password-baseline; cumulative HEADdiff is not round ownership.

## Pre-change chain and component selection

Read original task/current checkpoint, applicable Desktop/shared specs, archive reference and exact current Component0.8.0 Root/Dialog/WindowExt plus gpui-pre0.3.9 focus callbacks. Licensed Archiver AppWindow115->callbacks->ui.run158/view_item697 remains selected-extract-before-open evidence, not reusable GPUI password focus. Legacy tracked archive grep still finds package ZIP, no general archive UI owner; no proprietary native lifecycle inferred.

- WAC01: ArchiveApplication::handle_drop -> existing Root.open_dialog (drop decision); notification(ArchivePassword) -> password_prompt installs original passwords ticket -> deferred source.update -> has_active_dialog returnsNone. There is no post-close replay. This is a source-confirmed gap, not observed behavioral RED yet.
- WAC02: ordinary entry_result removes original pending, then passwords.remove only. Read consume separately calls hide_password_dialog; preview terminal calls dismiss_preview_password. PasswordDialog::alive/input/scope stay live for ordinary terminal. Original failure notice/lease/cleanup remain authoritative.
- WAC03: BrowserHost.activate -> deferred app.tab_activated -> hide_password_dialog queues window update; separate host focus defer sees old Dialog and skips; hide closes exact scope -> Component root.rs288–290 restores old previous focus(A). No subsequent current B focus. Deferred native open/close/rebuild require original host/session checks.

Exact released Component root::WindowState is private, as are ActiveDialog slots. Public WindowExt owns has_active_dialog/open/close; calling it before Root exists panics. Dialog on_close executes original lifecycle, Root close restores previousfocus; deferred close may restore after its original animation. Original code already captures Dialog scope from window.focused and alive token; use that owner, never unconditional pop or private stack access. No vendor edit.

Presentation unchanged: original official Dialog+DialogFooter/Action/Close, masked Input/InputState and existing Medium buttons; original browser official TabBar/Tab32 and original root focus handle. No new ordinary control, layout, palette, menu, queue or selection owner. Parent adaptations and private password transport unchanged.

## Required unified ordering

1. Callback changes only local state, then schedules bounded/coalesced work on original application owner next turn.
2. Ordinary terminal marks original displayed operation dead, clears input/ticket and retires exact Dialog scope; covered retired scope must not pop a newer modal. Late confirmation/cancel returns without password dispatch. Actual native cleanup is not inferred from UI retirement.
3. After old Dialog removal/Root close, a current-host lifecycle recovery revalidates host ID/active session/generation/operation. Resume only valid waiting original tickets, once, no new password queue or timeout/attempt expansion.
4. Current/new Dialog always wins. Content focus is restored to the current retained browser only after matching retirement, never to a hidden/closed/rebuilt entity. Reopening A uses retained original ticket and a new empty Input, not a stored secret draft.
5. Recovery is event/presentation-change driven and bounded, not sleep/periodic timer/unlimited retry. No synchronous application/entity writeback during leased host/Root callbacks.

## Approved test seams / planned validation

Actual GPUI Root/Dialog + original handle_drop/notification/entry result/open/active focus and keyboard boundaries as specified by user; synthetic platform endpoint only, no native product. WAC01 competing dropDialog -> original event -> actualclose -> same-tab matching prompt, duplicate/retired/background/closed guarded. WAC02 originalentrypending -> password Input sentinel -> failed/timeout result -> maps/alive/input/scope retired; covered newer Dialog retained/latezeroDispatch/newoperationworks. WAC03 A Dialog -> original open request B -> all defers/current frame -> actual B originalfocus+CtrlF affectsB only; A ticket restored, newerDialog/rapidA-B-C/close/rebuild/stale guards.

Each must have actual behavioral RED before product fixes; compile failures are not RED. Run affected password/tabs/entry/preview plus single-thread archive collection, Desktop strictClippy--tests/workspace--tests/scopedfmt/diff and applicable source/spec/catalog/release/editor gates. No fullsuite rerun; original production SPACING_12/animated/globalnetworkfmt limits remain separate. Every Cargo checks idle lane and sameprocess jobs1/incremental0/codegen256, full timeout.

## Evidence

No new behavior tests run at the time this initial boundary was recorded. Append observed commands/counts/red/green/failures after execution. No password/input/path payload logging; production diagnostics use correlation IDs/reason/state/elapsed only.

## Final result — WAC01–03 repaired at source / automated interaction boundary

2026-10-27 continuation. All three findings now have compiled behavioral RED before production fixes and corresponding GREEN. This closes these P2 source/automated findings only, not native/user acceptance or the canonical phase. The original independent acceptance report is byte-identical to round start.

### Final lifecycle and minimal ownership

- `application.rs`: only two round-start-relative additions: `BrowserHostSlot.password_recovery` and its default initialization. No constructor, native window option, worker or transport rewrite.
- `password.rs`: original application owns per-host `PasswordRecovery`: one coalesced deferred request, one focus subscription, one current session focus intent, and already-retired Dialog scopes (not another pending-password queue). Original `passwords` remains the sole ticket owner.
- WAC01: suppression by an existing modal schedules recovery. Recovery subscribes through public gpui-pre0.3.9 `Window::on_focus_out` to the current focus. Immediate Root close or original animated focus restoration triggers the next application turn. If another/newer modal exists, it is preserved and the subscription follows it. When no modal remains, resume only original tickets whose current host/session/generation/operation/entry snapshot/cancellation checks pass. The original prompt equality check prevents stacking duplicates. No timer, sleep, frame retry or attempt/timeout expansion.
- WAC02: ordinary `entry_result` removes its ticket and calls original `hide_password_dialog` before continuing the unchanged identity/lease/source/cleanup/failure-notice handling. Retirement immediately invalidates original `alive`, independently defers clearing its original masked Input, and retains only exact retired scopes. A covered scope is not popped while a newer Dialog owns focus; after Root restores the retired scope, recovery removes it. The loop removes one matching retired scope each iteration and is bounded by the actual retired list. Late reply/result is idempotent and dispatches no password; new operations still work.
- WAC03: `browser_host.rs` removes only the too-early competing content-focus defer/imports. `tab_activated` records current session focus intent on original host. Recovery first removes matching old Dialogs (including Root previous-focus restoration), then focuses only the currently active retained browser if no modal exists. A newer modal wins. Old host IDs and missing/closed sessions are rejected; host teardown drops its subscription and independently queued Input clear survives dropped recovery.
- All host/focus listeners defer application work; no synchronous leased-entity writeback or private Component WindowState/ActiveDialog access. `WindowExt` is queried only from deferred work after Root exists. Existing preview suspend/dismiss and private authorization/read/entry ownership remain unchanged.
- Production diagnostic addition is one low-frequency `archive.password.dialog_retired` event with original operation correlation ID and session ID only. Existing entry/read/host lifecycle events remain; no secret, Input, path or per-frame/pointer logging and no DEBUG probes.

### Exact RED → GREEN evidence

Session log root: `C:\Users\Administrator\.pi-desktop\scratch\9d8169e5-656b-47ae-9113-af5789f5091c`. Logs are local execution evidence, not publishing inputs.

Every Cargo command below first checked no `cargo`/`rustc` occupied the shared lane and set, in the same PowerShell command: `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`. Commands ran serially with full-operation timeouts; none timed out and no process was killed.

1. Before production fixes: `cargo test -p nxtools-desktop --bin nxtools-desktop --locked --offline wac0 -- --test-threads=1`; compiled, exit101, **0PASS/3FAIL/1398filtered** (`wac01-03-red-initial.log`). WAC01 actual drop decision Escape/close left zero password nodes (expected1), with no tab activation. WAC02 actual ordinary materialization failure left original alive token true. The initial WAC03 CtrlF failure mixed in loading-state UI and is explicitly **not** the precise WAC03 focus RED.
2. Before production fixes, corrected original retained-B ingress/ready-content fixture (no artificial activation-state mutation): `cargo test -p nxtools-desktop --bin nxtools-desktop --locked --offline wac03_new_tab -- --test-threads=1`; compiled, exit101, **0PASS/1FAIL/1400filtered** (`wac03-focus-red.log`). Exact assertion: `visible B must own its original focus after old Dialog restores previous A focus`. Its historical name said `new_tab`; final accurate name is `wac03_retained_tab_after_old_password_close_has_current_focus_and_keyboard`. Actual-new-B has a separate final regression.
3. First production fix: same `wac0` command **3PASS/0FAIL/1398filtered**, exit0 (`wac01-03-green-initial.log`). Same Root/Dialog/entry/host boundaries, not compilation-only evidence.
4. Expanded boundary suite: same `wac0` command **12PASS/0FAIL/1398filtered** (`wac-expanded-initial.log`). Final stronger actual-new-B keyboard assertion also passed **12PASS/0FAIL/0ignored**, 3.47s, exit0 (`wac-final-12.log`).

### Final affected checks (performed, not inherited)

| Check | Actual result / evidence |
| --- | --- |
| `cargo test -p nxtools-desktop --bin nxtools-desktop --locked --offline archive -- --test-threads=1` | **253PASS/0FAIL/0ignored/1157filtered**, final80.48s, exit0; `wac-archive-final.log`. Covers original password/entry/preview/tab/model/Tree/drag/forms plus archive ingress/settings. Initial same-filter run also253PASS in77.94s. Final additional new-B keyboard assertion was subsequently rerun in the12-test WAC subset; no production code changed after253PASS. |
| `cargo test -p nxtools-desktop --bin nxtools-desktop --locked --offline titlebar -- --test-threads=1` | **8PASS/0FAIL/0ignored**,0.50s, exit0; `wac-window-regressions.log`. |
| `cargo clippy -p nxtools-desktop --tests --locked --offline -- -D warnings` | Final PASS,32.44s; `wac-desktop-clippy-final.log`. First attempt failed10 test-only lint diagnostics (7unwrap_used/3collapsible_if); fixed without suppressions and rerun. `wac-desktop-clippy.log` retains the original failure. |
| `cargo check --workspace --tests --locked --offline` | Final PASS,27.08s; `wac-workspace-check.log`. Initial chained invocation was not reached after Clippy failure; later actual runs passed. |
| `rustfmt --check --edition 2024 --config skip_children=true` on application/browser_host/entry/password/wac_password_tests | PASS. Scoped formatting only, not global formatting. |
| `git diff --check --` on this round's existing production/docs paths | PASS; new test formatting also checked directly. |
| `python tests/architecture/spec-contracts.py` | PASS114documents/14owners. Documentation contract, not runtime acceptance. |
| `python tests/architecture/gpui-component-catalog.py` | PASS current0.8.0/historical catalog consistency. |
| `python tests/architecture/gpui-release-contracts.py` | PASS pinned Kit0.8.0/pre0.3.9/vendor provenance. No vendor edit this round. |
| `python tests/architecture/markdown-editor-contracts.py` | PASS pinned editor/provenance. |
| `python tests/architecture/archive-tree-root-drag.py` | PASS3source regression checks. |
| `tests/architecture/desktop-shell-lifecycle.ps1`, `download-ui-lifecycle.ps1`, `ui-state-lifecycle.ps1` | PASS; desktop gate also ran approved resource checks9PASS. All source-only, not physical lifecycle acceptance. `wac-source-gates.log`. |

### Final 12-case coverage

WAC01(5): actual competing drop close resumes once; duplicate waiting ticket/newer overlay respects both modal closes; refresh/cancel/generation replacement rejects old ticket; switched background then closed A never covers B; actual disconnected notification retires waiting operation.

WAC02(3): ordinary timeout failure, ordinary failure under newer modal, cancelled ordinary terminal under newer modal. Each asserts original alive/input/maps retired, original late reply zero dispatch, duplicate result/late cancel no-op, and next original materialize gets a fresh password input. Covered cases assert newer modal survives, original failure notice remains authoritative, and retired lower scope disappears after newer modal closes.

WAC03(4): retained-B duplicate-open owns actual root focus and CtrlF input while hidden A is unchanged; actual new-B owns focus and CtrlF input followed by rapid A→C/B close; newer modal+B password wins over retired A focus; old deferred host recovery cannot affect a rebuilt host and retired host Input clears. Return to A resumes its retained original ticket.

### Preservation and remaining limits

Round-start copies/hash inventory are under `wac-password-baseline`; reviewed `wac-round-only.diff` isolates the four production files from cumulative dirty HEAD changes. `application_tests.rs` is unchanged from round start. Independent `window-tabs-icons-drag-acceptance.md` is byte-identical. New project test is `rust/apps/nxtools-desktop/src/archive/wac_password_tests.rs`; it is nested under password.rs and introduces no production test API. Documentation changes are this report, original checkpoint prepend and archive reference follow-up only.

Reuse independent **1398unit+10integration / Core93 / IPC43 / Protocol83 / C#6** as prior baseline, not rerun or relabeled here. No new fullDesktop suite, C# build/native fixtures, package/sign/install/productlaunch/Registry/association/service/commit/push. No dependency, vendor, wire, worker/cache/lease or settings changes. Production-only strict SPACING_12/animated and three network-file globalfmt differences remain known prior limitations, not newly rerun or fixed. Installationerror1 remains independently OPEN. Physical GUI/OLE/Explorer/DPI/IME/UIA/user visual acceptance and fresh-process preference restore remain NOT RUN. No remaining in-scope source/automated blocker.
