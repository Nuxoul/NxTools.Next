# R5 hidden main first-show repair — outcomes

## Outcome and scope
The user requested repair after the live post-reboot oversized window was captured. R5 source repair is implemented and relevant synthetic/native-boundary checks pass. No framework/vendor patch or dependency change. Current live NxTools window/process was not moved, resized, hidden, activated, restarted, replaced or packaged. No product install/service/commit/push/delegation. **Installed reboot / physical mixed-DPI / user visual acceptance is not performed.**

## Cause and corrected boundary
GPUI Windows0.3.9 creates HWND withCW_USEDEFAULT. For hidden construction it caches requested1060x650logical placement rather than applying it. Existing native tray/summon/floating show+activate routes bypass GPUI.activate's placement consumer, leaving oversizednative bounds. Live release28 mainHWND firsttray-show1904x1021clientat96DPI corroborates this; source WindowOptions tests alone were insufficient.

Correction is a one-time hidden startup barrier, NOT a per-show default reset:
- `main.rs` defers advisoryHWND hint publication/notificationhost attach/floatingowner start for hiddenmain until preparation confirms. Normalshownstartup remains original.
- Original `PlatformEventDispatcher` owns one optionalpreparation, one callerID installedbeforequeue, bounded128ordinarynotificationbuffer and8sdeadline. It does not add a competing notification receiver. A matching success requires boundedtypednativeclient/DPIreadback; readycallback runs outside dispatcherlease on a laterturn, then originalnotifications replay. Duplicate/oldACK cannot publish twice. Disconnect betweenACK anddeferredcommit, queue/native/readbackfailure/deadline/budget terminatehidden and retain terminalguard; no lateentrypoint revival.
- New additive `PrepareMainWindow` carries explicitlogicalDIP dimensions andsourcePID/HWND. C# validates strictfields andauthenticatedpipepeer, hiddenroot/actualprocess/session, stagesprimarymonitor withoutshow/zorder/activation, readsactualtargetDPI, scalesonce/roundsAwayFromZero, appliesclientwithliveinsetcompensation and returns exactclient/hidden/DPIreadback. Work isdetached fromserialreader andboundedSTA5s; hungnativecall retains its slot. Nativecancel islogical, never claimed as forcibleWindowsAPI interruption.
- Existing tray/summon/floating/nativefocus/render owners continue after readiness. Later userresize/maximize/hide/restore does not call startup preparation again. Browsercoldstart show/focus flags remain unchanged and originalminimize only follows readiness. No GPUI.activate workaround/staleconstructorplacement replay, no show-first/resizelater flash, nofakeWM_SIZE.

## Red/green evidence and intermediate failures
1. Beforeproductionchange, auditedownedhiddenSTATIC/CW_USEDEFAULT -> originalnativeShowNoActivate -> actualclient1060x650DIP assertion **FAILED**: actual1904x990at96DPI. Log `r5-native-red.log`, buildsuccessful. No userHWND involved.
2. Same actualownedHWND sequence withproductionprepare -> exactclient1060x650at96DPI **PASS**. Prepare remainshidden/foregroundunchanged. Then syntheticallyresize1200x700 -> hide/show -> dimensionspreserved. Actualframeinsets compensated; source/visibleforeignwindow policy retained.
3. NativeDPI96/120/144/192, negativeworkorigin, invalidsource/retirement/visible/readbackmismatch/precancel/strictDTO/peer0orwrong, detachedlatercommandresponsiveness: **3selected/3run/3pass/0skip/0fail**. Latest C#build **0warnings/0errors**, `r5-native-complete{,-build}.log`.
4. OneintermediateFakeDPItestexpecteddefaultMath.Round(banker'srounding) whileproduction120DPI650DIP gives812.5 ->813AwayFromZero. Correctedsynthetictestexpectation, notproductiongeometry.
5. Earlynewfile edits had nativepointtype/methoddeclaration andlifetimemissingargumentcompilefailures. Diagnosed/currentanchorscorrected; preservedbuild/checklogs. No broad suppression, no testweakened toclaimgreen.
6. OriginalRustdispatcher startup tests **6/6**: pendingbeforequeue, Connected/trayblockeduntilACK, validreadback/deferredonce/replay, capturedoversizedreadbackrejected, staleID, duplicateACK, disconnectbeforedeferredcommit, deadline,128notificationbudget, rejectedqueue/lateACK. `r5-final-startup-focused.log`.
7. Protocol **77/77** inclnewtypedroundtrip/DPIexact/mismatch/capturedscene rejection. StrictClippy Desktop+Protocol --tests -Dwarnings **exit0**, workspacecheck --tests **exit0**; logs `r5-final-{protocol,clippy,workspace}.log`.
8. OnefinalDesktopfullrun **1123passed/1failed/0ignored**,1124total,160.05s. Unchangedknownfailure `downloads::tests::production_delayed_start_uses_shared_popover_and_preserves_draft_lifecycle`, downloads/tests.rs1376, observed2026-10-24 vs expected2026-10-25. No repair/rerun ofthat unrelatedfeature. Fullsuite remainsfailing, notrelabeledgreen.
9. Sourcegatesdesktop-shell/shared-ui-state/download-ui pass. BrowserNativeMessaging knownliteralstartupfocuscheck remainsfailing; originalmain_focus/main_show derive fromarchive_launch and areunchangedbyrepair, executabletests coverlaunchflags. Gatewasnotweakened.
10. ScopedRustfmt andgitdiffcheck pass (oneaddedmoduleorderfixed). Speccontracts,GPUIrelease,markdowneditorprovenance passedbeforefinaldocappend; closeout rerun recorded below. Rootvendorunchanged. PackagedDesktophash stillCE331D8EEC1E0A71BF830A2E7884977B123B43040581DC3AA804186B4763295C.

## Diagnostics
`product.main_window.prepare.requested/completed/failed` inRust andnativeaccepted/start/terminal sharethecallerUUID. Nativefixedreason/actualerrorcode/elapsed/requestedDIP only; payload/readbacknotlogged asrawData. No clipboard/query/file/URL/path/credential/pointer/frame logs or DEBUGprobes. Pending-before-dispatch andallterminalretirement enforced.

## Files / ownership
- Rust startup: `rust/apps/nxtools-desktop/src/main.rs`, `platform_events.rs`, `platform_events/main_preparation.rs` anditsprivate tests.
- TypedDTO: `rust/crates/nxtools-protocol/src/main_window.rs`, additiveenumexport inlib.rs.
- C#boundary: `Windows/MainWindowPreparation.cs`, `Windows/WindowsPlatformAdapter.MainWindow.cs`, narrowparser/pipe/sequence-dispatchadditions.
- Safeauditedfixtures: `MainWindowStartupTests.cs` + exactProgram/TestEffects registrations. Onlyself-ownedwindows/pureDTO/fakenative/signals; no registry/tray/input/liveuserdata.
- References/specs/taskrecords pluslocaldesktop-shell sourcegate. Baseline andred/greenlogs areinsessionscratch `r5-*`; no baseline restore overpeerfiles.

## Remaining blockers and delivery
No knownin-scopecompiled/syntheticdefect remains. Globalfullsuite stillhasoneunchangeddatefailure; browserstaticgate stillhaspreexistingliteralassertionfailure. Sourcecompletion does notmean installedbinaryupdated. Releasepackaging/installandrebootacceptance needseparateauthorization. Existinglivewindowremainsuntouched; afterdeploymenttest silentbootfirstopen, close-to-trayrestore anduserresize/mixedDPI withcurrentpackage tuple.

## Final closeout
Final result logs were re-read during handoff: native selected/run/pass3/3/3, zero skipped; strictClippy/workspace/Protocol/startup checks exited0; the single fullDesktop run exited101 with the unchanged date assertion noted above. Desktop-shell/sharedUI/downloadUI source gates pass, existing browser literal gate remains failing. Final documentation checks cover spec contracts, locked GPUI release contracts and Markdown editor provenance; scoped diff checks pass. Task remains `source_completed_acceptance_pending`, not archived or deployed.
