# Archive icons / windows / preferences / Tree / tabs — final source handoff

Date:2026-10-27. Source implementation requested in D:\Code\NxTools.Next-main, not a research-only response. Current dirty code and latest checkpoint were inspected first. Canonical phases A–F unchanged. Current task state is only phase-a-checkpoint.md; this is the detailed evidence report. All historical failures preserved. Installation error1 remains independently OPEN.

## Delivered features

1. **Approved icons:** immutable approved PNGs retained/hash-verified. Blue application and orange archive designs generate genuine16/20/24/32/40/48/64/128/256 ICO frames and matching PNGs. Premultiplied-alpha resize, invisible speck removal in derivatives only, aspect/clear margin preserved. Original EXE resourcegroup1blue/group2orange; GPUI brand aliases, current native app HWND/taskbar/tray consumption, SetupIconFile/uninstall/shortcuts and existing browser companion entries integrated. Archive ProgID DefaultIcon/classiccascade use explicitresourceID-2. Tree root/officialTab prefix use orangebrand/archive.png. No new loose runtime resource or extra trust schema: original Desktop/extension hashes own signing/approvals/install resource chain. No actual signing/package/association mutation/cache clearing. Actual rebuilt DebugDesktop PE's18frames independently decoded and matched. Offline twooriginals+Light/Darkproof actually inspected; actualShell/tray/taskbar/DPI/useracceptance not performed.
2. **One shared caption/control component:** existing browser reference34DIPcaption/controlwidth/16glyph/12inset and official theme states; public styled Buttons/nativeWindowControlArea, one shared composition, no per-window copies/vendor edits. Main preserves original C#command/trayclose; actualtitlebardoubleclick now uses originalToggleWindowMaximized instead of anotherBeginWindowMove. Archive host/form/progress use explicitClient/transparentnativecaption to remove duplicate decorations at creation. Appropriate min/max/close by capability; fixedsettings/newdownload/dialog close-only, fixedtask/toolmin+close, resizablearchive3buttons. Passivepopup/preview/notification/menu/specializedpanels unchanged; editor layers are not independentHWNDs. Complete inventory in window-progress-preferences-outcomes.md.
3. **Progress and form layout:** compact560x300/min480x260 progress, naturalbody/boundedscroll/fixedfooter; extract/create/edit min480x360 body scroll and target/browse/Cancel/Submit accessible. B/KiB/MiB/GiB/TiB uses one archive formatter; duration uses existing sharedformatter. Runningprocessedmetrics and unknown totals stay honest; terminal primary is actualcommitted/skipped or Testverified, partial/cancel/failure/uncertaincleanup/nativecode retained. No fabricated totals/percentage/statistics, no cancelrollback claim. Originalpause/stop/background/close and worker slot remain.
4. **Preferences:** acceptedform+Remember only; extractstrategy/conflict/smartsinglefolder/MOTW/openafter/keepprogress and separatelyoptedin validatedcustomtarget. Here remainssourceparent. Source/secret/grant/permission/snapshot/ordinals/All-vs-Selected/runningstate/deleteconsent not persisted. Originalsectionlock/latestdiskreread/changedfieldmerge/atomicwriter/ordereddeferredpublication, no secondconfigframework. Missing/nulloptionalcustomtarget=None; corrupt/invalid non-null config preserved. Explicitprogresscheckbox saves only correspondingcreate/extractkeepfield via originalbackgroundsectionwriter. Cancel/render/picker no draftsave.
5. **Tree native drag:** originalTreeicon/labeldown -> held>=4DIP move -> originalnativevirtualcommand/readconnectionlockedadmission/HWND/capturecancel/buttoncheck/dedicatedSTA/nonblockingreader; no GPUIon_drag/preextract/renderIO. Childbasename/hierarchy/implicit/explicit/emptydirs preserved; root adds one validated compoundstemfolder, noflatten. Rootwrapper counts128descriptors,32innerroots/32files/512MiB/256KiB unchanged; empty/unsafe/overbudget refuseswhole. Originalordinal/worker/cache paths unchanged behindpresentationprefix. Samegrant/source/snapshot/generation/lease cleanup.
6. **Browser tabs/open reuse:** onehost/nativewindow, officialTabBar/TabMedium/maxwidth/scroll/overflow and officialsuffixcloseButton. Canonicalsourcepath+Windowscaseequivalence, neverfilename-only. Duplicateactivateswithoutread; differentadds/activates, originaltypedC#ActivateWindow restoresminimizedhost, correlatedpending/result guards. OriginalArchiveBrowserentities retained pertab, including model/folder/selection/sort/scroll/read/preview/sourcegeneration. Tabclose cancels/releases onlyitsoriginalsession, lastcloseshost, X retiresall, stalehost cannotclose replacement. Nesteddepth/parentlease unchanged. Backgroundpasswordticket waitsownactivetab, switch clears/hidesoldinput beforefocus, ticket retained/nopassworddraft. Hostedconstructor explicitinitialfocus avoidsRoot-dependentWindowExtbeforeRoot. No tabpaths/sessionrestore persistence.

Preserved actual current detailedcolumns/nativefoldericons/marquee/internalimage/text/code/Markdownpreview and originalrightdrag/password/cancel controllers. No dependency/vendor/versionupgrade, parallelmenu/selection/drag/configuration owner or install-diagnostic-chain edit.

## Source/file map

- `scripts/generate-approved-icons.py`, `tests/architecture/approved-icons.py`, `rust/apps/nxtools-desktop/assets/brand/*`, original4browserextensionicons, `build.rs`, `src/assets.rs`, `installer/nxtools-next.iss`, archiveIntegrationPlan.cs/tests, originaldesktopicon/sourcegates/packagegate. Fullconsumermap/rebuildconditions: icons-integration-outcomes.md.
- `src/ui/components/titlebar.rs` + newtitlebar_tests.rs, sharedmetrics/dialog/settings; main_window/shell.rs and existingcaptioncallers in downloads/new/task/settings/schedule/delete/shutdown/globalsettings/toolsettings; correspondingactualfocus/geometrytests.
- `archive/progress.rs`, `forms.rs`, `draft.rs`, `jobs.rs` andactualforms/jobtests; Corearchive.rs/archive_tests.rs foroptionalfield/validation/changedfieldmerge.
- `archive/browser.rs`, `browser_drag.rs`, `model_drag.rs`, browser_drag_tests.rs; Protocol/IPCarchive_virtual_drag.rs; C#ArchiveVirtualDragContract.cs/ArchiveVirtualTreeTests.cs. No Session/Content/Cache production rewrite.
- new`archive/browser_host.rs`, `application.rs/application_tests.rs`, `launch_entry.rs`, `password.rs`, previewpasswordguard in`entry.rs`. Original source/read/entry/preview/lease owners retained.
- taskcontract/outcomes/checkpoint, referencearchive/virtualdrag andDesktopdatabaseguidelines scenario updated. Otherdirtychanges retained, including concurrentHatchZipresearch note.

## Actual commands and evidence

EveryCargo/direct/background used CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-Ccodegen-units=256 and checkedseriallane. No duplicatebuild/kill/cleantarget. Temporarylogs in session scratch9d8169e5-656b-47ae-9113-af5789f5091c; names below.

| Command / boundary | Observed result |
| --- | --- |
| Initial samehost regression | RED0PASS/1FAIL: oldWindowId1!=2 (`archive-tabs-red.log`) |
| Iconoffline gate beforeimplementation | REDRUN7/fail4/error2; finalRUN9/PASS9/SKIP0 plus25reproduciblederivedresources |
| `python tests/architecture/archive-tree-root-drag.py` | StaticRED3FAIL ->GREEN3PASS; actualTreebehaviorRED not observedbeforeimplementation, explicitly retained evidence limitation |
| ActualKeepOpencheckbox->diskreopen | REDmemoryonly; GREENthroughoriginalwriter (`archive-progress-red8.log`, finalarchivecollection) |
| Newpasswordbeforetabopen | ActualREDoldDialogcoverednewtab ->GREENexplicitinitialfocus/orderedcleanup/currentDialogguard |
| FailedpreRootDialogquery attempt |143PASS/65FAILsameComponentWindowStatemissingpanic; correctedoneconstructorpolicy, noframework/testinitializationbypass |
| `cargo test -p nxtools-desktop --bin nxtools-desktop archive:: -- --test-threads=1` final | **208PASS/0FAIL/0ignored**,48.72s (`archive-after-focus-root-fix.log`); includes7hosttests/7newTreegestures/3models andretainedpreview/marquee/row/password/cancel |
| Maincaptiondoubleclick actualevent->originaltransport | RED0/1, minimalfix, **GREEN1/1** (`caption-doubleclick-red/green.log`) |
| `cargo test -p nxtools-core -p nxtools-protocol -p nxtools-ipc --lib --locked --offline -- --test-threads=1` | **Core93/93,IPC43/43,Protocol83/83**,0ignored (`archive-owners-check10.log`) |
| FullDesktop firstattempt | Interruptedbyuserquestion, nofinalsummary; fixedoldToolbarTabcountthenindividuallyPASS; retainedlog |
| `cargo test -p nxtools-desktop --locked --offline -- --test-threads=1` resumed | **1393PASS/4FAIL/0ignored of1397**,436.93s,exit101 (`archive-full-desktop-resumed.log`/resultJSON). Not fullPASS |
| Exactfourfailedtestssubsequent | **4/4PASS**:shutdownCheckboxactualfocus,Tools34caption,globalclose34x33,archiveexclusionpage-margin scroll/fullvisiblebutton+originaldiskrules. Each`*-settled.log`; nosecondfullunitrun |
| `cargo test -p nxtools-desktop --test extension_http --locked --offline -- --test-threads=1` | **10/10PASS**,0ignored (`desktop-integration-settled.log`), noactualproductruntime |
| Final`cargo clippy -p nxtools-desktop -p nxtools-core -p nxtools-protocol -p nxtools-ipc --tests --locked --offline -- -D warnings` | **PASS** (`strict-delivery.log`) |
| Final`cargo check --workspace --tests --locked --offline` | **PASS** (`workspace-delivery.log`) |
| Productiononlystrict`clippy -p nxtools-desktop --bin nxtools-desktop ... -Dwarnings` | **FAIL2originalwarnings**:SPACING_12/main_window.rs75,animated/floatingassistant/media.rs62 (`production-strict-boundary.log`). Not suppressed/fixedoutofscope |
| Scopedrustfmt(check,skip_children) alltaskRustfiles | **PASS**; ownnetworkassertionformattedlocally |
| `cargo fmt --all -- --check` | **FAILexisting3networkfiles**:network_settings.rs1135/1184/1201,network_settings/blacklist.rs,page_rem_tests.rs. Not bulkformattedoverconcurrentwork |
| `cargo build -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs --locked --offline` | **PASS**,freshmatchingworker (`archive-worker-build12.log`) |
| `dotnet build ...NxTools.PlatformHost.Tests.csproj --no-restore` | **PASS0warnings/0errors** (`archive-csharp-build.log`) |
| ReviewedexactvirtualTree5filtersLIST->sameRUN withfreshworker | **LIST5/RUN5/PASS5/SKIP0/FAIL0/not_run0** (`archive-csharp-tree-five12.log`); controlledHGLOBAL/IStream/originalworker, noExplorer/OLEtarget |
| Additionalpureclassiccascade/OpenWithiconplan | **LIST1/RUN1/PASS1**,0skip; noRegistry |
| `python tests/architecture/approved-icons.py --desktop-pe target/debug/nxtools-desktop.exe` | **PASS**bothgroups/18exactdecodedframes,readonlyPEparser/noexecution/Shell |
| DesktopShell/DownloadUIlifecycle,archiveFspike/shell,Tree/icon/spec/catalog/release/editor gates | **PASS**; spec114docs/14owners,catalog79historical/78source,currentgraphunchanged,editor57files/16cases |
| Scoped`git diff --check` | **PASS** |

Recoverablebuildartifactfailures retained: sharedfingerprintdirectorydisappeared; then ppv-lite86 E0463zerocopy/352cascadeerrors. Exactlocked`cargo check -p ppv-lite86 --locked -vv` observedmissingzerocopy_deriveDLL, rebuiltoriginaldependencyPASS. No dependencyupgrade/sourcepatch/cachedeletion or unknownprocessstop.

Oneoptionalread-onlyreview was launched but interruptedbyuserprogressquestion and yieldedno settledreport; no claimreviewpassed/no secondreview. Main performedactualsource/crosslayerchecks andfixedconcretetestfailures.

## Remaining blockers / acceptance

- No completelygreenfullDesktop rerun afterthefourtargetedfixes; finalscoped208andfourcasesPASS do not rewrite1393/4history.
- Productionstrict stillFAILthetwooriginalwarnings above; globalfmt stillFAILthethreeexistingnetworkfiles. Scopedstrict/check/fmt aregreen; do not reportallworkspacegatesgreen.
- RealproductGUI/physicalpointer/nativeTreecapture+OLEforeigntarget/Explorer/minimize-maximize-drag-doubleclick/DPI100/125/150/200/mixedDPI/IME/UIA/userlook-and-feel acceptance **NOT RUN**. Offlineiconinspection/PEdecode/syntheticGPUI/controllednativefixtures are not thoseacceptances.
- No package/signing/installerexecution/realassociation/UserChoice/RegistryCOM/service/protecteddeployment/cacheclear/commit/push. Existinginstaller31/protectedruntimehaveNOTacquiredthesourcechanges. Rebuild/repackageanduseracceptance requireseparateapproval; originaltrustkey/ledger/resourcesremainowners.
- Installationerror1 remains **OPEN**, diagnosis/deliverychainunchanged. Allcanonicalearlierphase/native/userobligations preserved.
