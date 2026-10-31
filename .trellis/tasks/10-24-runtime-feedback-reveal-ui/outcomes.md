# Runtime feedback implementation and verification — 2026-10-24

## Outcome
Requested repairs are implemented locally; no package/install/service/release/normal-product launch, commit or push occurred. Unrelated dirty work was preserved. Final integrated verification is **not fully green**: one Downloads schedule regression and the pre-existing elevated-navigation source gate remain outside this repair slice.

## Findings and changes
### Floating assistant visibility
- Read-only running renderer/input both already had TOPMOST+NOACTIVATE; logs also showed explicit hide/update/show. Initial missing-topmost alone was not established.
- Reread legacy FloatingAssistantWindow.axaml.cs1153–1223 `_topmostGuardTimer -> EnsureNativeTopmost`.
- Owned native `HWND_NOTOPMOST` regression ran red before repair. Existing500ms observer now restores lost TOPMOST on visible current renderer then receiver, with NOMOVE|NOSIZE|NOACTIVATE, actual readback and no show/revive. Current lease/identity/hidden/retired/click-through safeguards remain.
- Fullscreen policy excludes Shell desktop, invalid/hidden/minimized and ordinary framed/maximized windows; genuine borderless fullscreen still honors configured hide. Production reason-only/correlated repair/environment diagnostics are low-noise; repeated repair failures rate-limited30s.
- Safe native verification uncovered a test-infrastructure defect: a WinForms Form replayed its managed non-topmost policy on initial show (matching dimensions/visibility but exstyle lacked8). Coordinator fixture now uses explicit passive TopMost initialization; cross-process renderer uses minimal self-owned NativeWindow with original green-pixel/hit/OLE/move/pass-through checks. No production retry/timeout inflation. Temporary DEBUG-RUNTIME-FLOAT/FirstChanceException probes removed.

### Two PlatformHost locations
Read-only SCM/process evidence: `NxToolsUpdateV1`, Running/Auto, Session0 PID5416 runs Updater/Bootstrap PlatformHost with `--trusted-update-service`. Service desktop PID8460 with `--silent-start` has Session1 child PlatformHost PID16424 `--parent-pid8460`. These are updater broker and desktop Windows adapter, not duplicate adapters to terminate/merge. Later unrelated unidentified PlatformHost was not stopped.

### Currency Select clipping
Same official32DIP Select/SearchableVec, original460x320feature window, query/keyboard/save/reuse owner. Full choice text Chinese Yuan Renminbi reproduced actual inputx280..600 before fix. Wrapper w_full/min_w_0/max_w164 and Select w_full constrain actual triggerx284..448 inside12DIPright padding; existing menu320x160 stays. Regression checks actual official input snapshot, not only its parent.

### Focus outline
Use released public Theme.focus_ring=false: remove redundant outside halo, retain ring/selection palette and official thin/internal focus indicator. No vendor patch, focusability removal or selection-color replacement. Theme-preference and optional-theme regressions pass. Screenshot copies were OCR/pixel-analyzed locally; browser/desktop tools did not return usable image attachments, so no visual/aesthetic acceptance claim.

### Reveal false failure — actual root cause proved
- Shared QuickLaunch/Downloads route remains SelectInExplorer -> detached STA OpenShellTarget -> ClipboardShellTarget -> ExplorerShellActivation -> native launch/readback -> correlated original result. No Rust/protocol/caller toast workaround.
- Actual logs: native_request_accepted -> observation_browser_interfaces_missing -> target_not_confirmed. Initial read-only service-only ShellWindows QI probe was insufficient to identify an Explorer-version defect: QueryService also failed and no Explorer folder root existed. Earlier reference appendices corrected accordingly.
- User separately authorized full native verification for an owned scratch target and narrowly verified cleanup. Actual native request opened Directory Opus13.22, not an Explorer folder window. Explorer-only confirmation therefore missed successful exact selection.
- Reviewed existing Next/SwiftList DOpus source and official DOpusRT information API before implementation. Read-only paths/listsel exports bind lister/tab/current filesystem directory and exact selected file. Localized display_path is never identity.
- New DirectoryOpusTargetObservation uses existing ExplorerTargetView/ObserveTarget: Explorer first, then on-demand DOpus; require visible same-session/PID/starttime dopus.lister and child filedisplaycontainer, active source tab flag, exact parent and selected target, second paths/native identity check. Wrong/inactive/hidden/stale/foreign/bad XML cannot succeed.
- Sibling helper from actual running process, no PATH search; only /info paths and listsel,<exacttab>, no mutation commands. Command-local1500ms, helper/output250ms, bounded termination confirmation<=25ms; XML2MiB/256tabs/4096selected/4matching candidates, DTD disabled, unique fresh temporary output and cleanup. Existing8attempts/75ms/onefallback unchanged.
- Added fixed vocabulary observation stages with original correlation; no raw paths/titles/URLs/exception messages. Accepted native launch/helper exit/window appearance/folder-only evidence still insufficient.
- Regression red: shared Reveal fixture failed with no target observer (pass0/fail1). Final synthetic DOpus3/3 pass. Actual product native entry confirms same owned file in228ms, target+foreground confirmed. Independent actual observer accepts exact selected target and rejects another existing unselected same-directory synthetic file. Native test does not imply broad third-party/UNC/mixed-DPI/release acceptance.

## Verification
| Check | Actual result |
| --- | --- |
| C# Tests project build --no-restore | PASS,0warnings/0errors |
| Reviewed exact safe fixture union: floating14+DOpus3+Explorer feedback6+native-first/readback/PIDL3 | PASS26/26,skip0,fail0 |
| Authorized owned DOpus actual product native entry | PASS228ms + exact selected positive/unselected negative |
| Desktop full suite, serial | **1102passed/1failed**,1103total,135.25s |
| Currency actual Select lifecycle/bounds test within full run | PASS |
| All3theme_graphite_tests within full run | PASS |
| Floating34Rust tests within full run | PASS |
| Desktop strict Clippy --tests -D warnings | PASS |
| cargo check --workspace --tests | PASS |
| cargo fmt --all -- --check | PASS |
| spec-contracts | PASS111documents/14owners |
| gpui-release-contracts | PASSKit0.8.0/pre0.3.9; original two narrow vendor deltas |
| markdown-editor-contracts | PASS |
| gpui-component-catalog | PASS |
| floating-assistant-lifecycle | PASS32sourcecontracts |
| download-ui-lifecycle | PASS |
| platformhost-test-selection | PASS |
| quick-launch-lifecycle | **FAIL**existing elevated-navigation source contract (line1765) |
| Scoped git diff --check | PASS |

Every Cargo command inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256. One optional read-only review returned one concrete issue: unbounded post-Kill WaitForExit; changed to bounded<=25ms and rebuilt/retested. No second review pass.

## Remaining blockers / acceptance limits
1. FullDesktop failure: downloads::tests::production_delayed_start_uses_shared_popover_and_preserves_draft_lifecycle, downloads/tests.rs1376. Actual selected draft date remains2026-10-24 while expected2026-10-25. No source edits to that unrelated test/owner; no broad rerun to hunt green. This is an observed failed click/date assertion, not a proved midnight-clock diagnosis.
2. Quick Launch elevated-navigation architecture gate remains failing outside this slice; raw error/log retained in scratch runtime-gate-quick-launch-lifecycle.log. No change to elevation/service boundary.
3. No packaged/installed/user visual/physical pointer/mixed-DPI/reader acceptance. Current source/native repair results do not change the running installed product.
4. DOpus HWND1314112 was the original newly opened test lister, but later readonly export showed5tabs including unrelated non-test directories. **Did not close the whole lister**, since that could close user tabs. Active test-target tab remains; no tab-close API guessed. Old owned Paint PID19972 and preview HTTP PID3728/port8976 no longer exist, so no termination performed.

## Evidence location
Session scratch contains feedback-topmost-red.log, feedback-currency-window-red.log, feedback-currency-green2.log, feedback-halo-green.log, feedback-dopus-red.log/green.log, feedback-native-reveal-green.log, runtime-native-fixtures-verified.log, runtime-csharp-build.log, runtime-desktop-full.log, runtime-desktop-clippy.log, runtime-workspace-check.log, runtime-workspace-fmt.log and runtime-gate-*.log. Scratch is temporary and not a product artifact. No original provided image/config/history/clipboard data changed.

Final follow-up: additional explicit foreign-renderer TOPMOST readback assertion plus DOpus3fixtures passed4/4. All five Python documentation/provenance/lifecycle gates reran green after final documentation changes; scoped diff check passed. Temporary debug/FirstChanceException search returned0matches.
