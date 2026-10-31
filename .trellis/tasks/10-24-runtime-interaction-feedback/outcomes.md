# Runtime interaction feedback outcomes — 2026-10-24

## Scope and state
User approved ten-item plan and clarified pin remains after paste but lasts only the current opening. Source implementation is complete for R1-R4 and R6-R10. **R5 rare oversized startup remains unresolved/unreproduced.** No main/startup/framework patch was made for it. No package/install/service/formal launch/commit/push/delegation or real user clipboard/history/config/proxy read occurred.

## Implemented
- R1: menu opens -> deferred original preview retirement; menu guard in row admission/application schedule/show and native notification handling; pending/visible/late ACK cannot resurrect preview. One original preview/800ms retained.
- R2: nonmenu/source HWND presses differ from external; transparent full-display menu overlay click uses recorded source device bounds and pointer screen position. Inside source dismiss restores original focus only, no panel hide. Source click consumption protects item activation, overlay path does not consume a later normal click.
- R3: official Small Toggle with Keep icon immediately left of Recording; false per new panel, cleared on close/no config; automatic foreground/pointer/paste close suppressed. Paste terminal does not reactivate panel. Current external target updates exclude same-process/preview/IME. Explicit close remains original owner.
- R4: reproduced native bounded wait failure with scaled30ms budget/100ms untouched user signal and Rust601s logical expiry. Removed background timeout for human choice, retained one bounded STA, stop cancellation, pending callerID and stale/disconnect rejection; cancel quiet, source/busy/real failure distinct. This demonstrates one cause, not every possible intermittent failure.
- R6: local OCR on146x70feedback image (scaled8x) reads 添加时间. Actual mounted sort Button centerline100.5 vs leftsegments102.5DIP red; parent flex/items_center produces equal centers green, no hardcoded offset. No reliable AI visual/user acceptance claimed.
- R7: three original tool tiles share official Badge.dot+text, actual enabled state, original geometry and status selector retained.
- R8: additive read_system_proxy typed no-payload command; native WinHTTP currentuser config only, frees allocations, no PAC evaluation/network/credentials/bypass/PACURL output. <=8 bounded endpoint rows, readonly independent InputStates, route/manual draft preserved, ID/viewgeneration/window guards and5s logical read deadline.
- R9: original add editor before list, bounded rowviewport120DIP leaves footer reachable at720x480, existing draft/validation/Cancel/save/row controls retained.
- R10: Small outlined Firefox button opens exact supplied AMO URL through existing Shell command; pending-before-send, currentACK/10sdeadline/disconnect terminal, settings remain. No Chrome URL invented.

## Verified regressions and intermediate failures
- Initial new Clipboard tests: compile succeeded then3 expected assertion failures (candidate admission/source classification/missing toggle). Green5 including an unrelated matching theme test.
- Clipboard efficiency54/54; native menu4/4 (before later overlay addition); finalpreview15/15 including menu visible+lateACK.
- Overlay test first failed in fixture because menu window was destroyed before synthetic mouseup; corrected to actual mousedown+App terminal observation, then actual source-inside assertion failed, then green.
- Native picker causal test red (waiting returned failure); Rust601s red; finalnativepicker wait+cancel1/1, Floating Rust42/42 before later stale test (latest targeted runtimefeedback includes stale case).
- Nativecoordinator8/8; systemproxy pureparser1/1; C#build0warnings/0errors after native fixes.
- Final focused runtimefeedback12/12 (includes existing unrelated theme test), Firefox actualclick/exactURL, proxyModeABA/manualpreservation, fixedToggleactualclick/IMEtarget/noactivation included.
- Sort actualgeometry red100.5vs102.5DIP then green1/1.
- Protocol budget test initially used slice.repeat requiring Copy; corrected to clone9 synthetic endpoints. No wire change from this test repair.
- FirstfullDesktop **1113passed/5failed/0ignored**,1118total,152s. Four failures attributable to this batch: removed tool status selector(two tests), blacklist moved editor causing footer overflow(one), old list-before-editor assertion(one). Repaired selector and bounded viewport, updated intentional-order assertion. **Affected globalsettings34/34 and tools5/5 subsequently pass. Full suite was NOT rerun and must not be relabeled green.**
- Remaining unchanged full failure: `downloads::tests::production_delayed_start_uses_shared_popover_and_preserves_draft_lifecycle`, downloads/tests.rs1376: observed2026-10-24 vs expected2026-10-25. Historical known failure, not changed.
- StrictClippy Desktop+Protocol --tests -Dwarnings and workspacecheck --tests pass after own lint/test fixes. Protocol76/76 pass. Logs `runtime-feedback-repaired-{clippy,workspace,protocol}.log`.
- Sourcegates: clipboard-ui,clipboard,desktop-shell,download-ui,ui-state,ui-token6/6; floating lifecycle32contracts passes. BrowserNativeMessaging remains failing existing startup focus literal assertion; main/platform_events not changed by this batch. No fix/weakening.
- Final closeout: speccontracts114documents,componentcatalog,coherentGPUIrelease,pinnededitor provenance pass. Scoped rustfmt initially found only the new floating stale-reply test formatting; formatted that exact file and its check passes. All other scopedfiles already passed. Scoped gitdiffcheck passes. Mainstartupsource hash equals scopedbaseline. No remaining owned FeedbackImageViewer process observed.

## R5 diagnosis / remaining blocker
Source concern: GPUI hidden create stores initial_placement while HWND stays CW_USEDEFAULT. Tray restores via C# ShowWindow(SW_SHOWNOACTIVATE), bypassing GPUI activate's cachedplacement path. No agent-runnable causal native loop for the user's rare boot enlargement was obtained. Sourcefixed logical size/DPI tests do not prove actual native size. Do not reset every show or modify vendor speculatively. Needs separately authorized owned native hiddencreate/firstshow/DPI replay or redacted user trace. Product behavior not changed for R5.

## Evidence levels / limitations
Source and compiled/synthetic/owned HWND tests only. Download image fetched/OCR identified; browserpanel unavailable, native viewers repeatedly unavailable; self-owned finalviewer screenshot metadata returned but no reliably interpretable attachment. No AI/user visual acceptance. No physicalWindowsIME/mixedDPI/realpicker/liveproxy/browserlisting/installed acceptance. Existing running NxTools observed in window lists but not automated/terminated. Standalone FeedbackImageViewer was created only in scratch for read-only feedback image; final process inspection confirms none remains.

## Artifacts and logs
Task PRD/design/implement/context remain scoped here. Baseline and all red/green/failure logs under session scratch `runtime-feedback-*`; baseline never restored. Initial status records dirty concurrent work. Keep historical full failure and partial repair distinct. Repaired-results JSON includes PowerShell stream metadata due a capture mistake; filter objects with name/exit to obtain measured statuses, logs retained independently.

## Final handoff
R5 remains open and task status stays in_progress/partial_r5_blocked rather than completed. Own regression fixes were verified with targeted34settings+5tools, not a second fullsuite. Strictlint/workspace/protocol76 are measured exit0 in `runtime-feedback-repaired-verdicts.json`. Architecture7pass/1knownbrowsergatefail. No added warnings suppressions, temporary DEBUG probes, private payload logs, package or deployed-binary replacement. Task context validation reports all entries valid with oversized-reference truncation warnings; direct agent read the applicable reference windows, no delegate was used.

## R5 live follow-up (2026-10-24, later observation)
User supplied the untouched oversized post-reboot scene. Read-only native/log capture now confirms exact mainHWND132698, silent-start,96DPI/notmaximized,1904x1021client vs1060x650requested; first tray-show DWMframe215,208,2121,1230 equals currentframe, stable across two reads. RunningDesktop matches package28. This supersedes unavailable-live-scene statements above, not the unchanged no-fix status. Hidden-create pendingGPUIplacement bypass through C# tray show/activate is strongly supported; controlled native causal regression/repair remains pending. See `r5-live-boot-evidence.md`. No input/windowmutation/build/productpatch/deployment in this read-only follow-up.

## R5 repaired source (later approved continuation)
User explicitly requested repair. OwnedCW_USEDEFAULThidden/directnativeShow regression ran red1904x990vs1060x650at96DPI; one-timehiddenprepare/clientreadback + originaldispatcherACK barrier now green. Native3/3, Ruststartup6/6, Protocol77/77, strictClippy/workspacepass; latestDesktop1123pass/1unchangeddatefailure/0ignored, nofullrerun. R5sourcecompletion supersedes previousblockedsource status, not installedacceptance. No vendorchange/deployment/livewindowmutation. Details: `r5-repair-outcomes.md`.
