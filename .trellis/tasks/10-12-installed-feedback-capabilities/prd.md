# Installed feedback and download capabilities

## Goal and approval
Deliver the user's 2026-10-12 installed screenshot corrections, functional fixes and three explicitly selected ABDM capabilities. User approved the final reference-based suggestions with “都按照你建议的来” and selected task count+size, Open With, limit speed while using other apps. Implementation approved; no further approval pause for the settled behavior.

## Confirmed evidence
- Current source checkpoint is626 desktop tests; broad dirty work includes prior UI/extension/installer fixes and must be preserved. Scratch approved-feedback-baseline records status/diff.
- Installed Desktop/PlatformHost are current, protected service tuple is older and service5324 predates latest package. Indexer validates approved desktop identity, while EnsureAsync reuses an existing service rather than upgrading mismatched tuple. Do not bypass authentication.
- Download filename/category concatenated in download_page.rs; clipboard source label uses text_sm; notification host lacks opaque background; Quick input prefix adaptation and prior installer race fixes remain.
- ABDM downloaded source lacks.git. CHANGELOG has1.11.0; monitor uses1-second samples,5-sample average(default app true),500ms active view update. Progress Info: name/status/size/downloaded-percent/speed/ETA/resume; settings:thread/limit; separate part table(number/status/downloaded/total). Windows uses nativefolderselect thenexplorer fallback; file/incompletefile/parent fallback. Do not copy immediate completed-window dismissal or indefinite marquee.
- Kit0.7.1 bundled default Light/Dark plus theme loading API; not a bundled many-theme library.

## Requirements
R1 UI: neutral tool card surface/subtle hover/status dot; Clipboard source12px subordinate; titlebar global action vertically centered includinghover; classification separate column(notfilename); ordinary confirmed new-download content removes wasted gap using measured client/content allocation; category popup header/list/two-line row bounds; notification opaquebackground; taskbar true native transparent externalclient.
R2 monitor: same keyed native task window; completefilename wrapping/fulltext access; noURL/savepath inprogress; downloaded/total,speed,ETA,status,resumesupport,limitstate; unfold parts number/status/downloaded/total; speeddisplay1s withrecent5s smoothing derived fromtimestamped byte counters (avoid double averaging),ETAusingdisplayrate. Paused/failed/completed immediatelyaccurate. All currentactions retained; unknowndata explicit.
R3 directory: nativefolderselect/open first,explorer fallback, correct existing/temp/parent target, restore/foregroundreadback. Keepcompletedwindow untilcorrelatedsuccess; failuresretainretry. Open With secondarymenu using Windows chooser throughC#; cancellationnotfailure, no earlyclose.
R4 Indexer: detectmatching/mismatchedprotectedtuple; explicitupgrade/UAC throughformalinstaller withallDesktop/Host/Indexer identities coordinated. Show actionableversion/authorizationstatus, boundedretry; not autoinstall/restartrealservice duringtests.
R5 additions: selectedcount + knownselectedtotalbytes, unknownsizeexplicit; Open With context/completionentry; backgroundspeedlimit enable/config value basedonforegroundotherapp, combinewithmanualglobal/per-taskcaps usingminimumactivecap withoutmutatingoriginalcaps; disable/restorenativepolicy immediately; persistssectiononly.
R6 Link: removecopy_onlysetting/implicitcopy-on-open; capturetriggersidentificationonly; clickwebopensbrowser,fileSelectInExplorer,directoryOpen; secondaryexplicitcopylink/path; noautomaticopenaftercapture; preservegeneration/close-before-deferredroute/six-secondhide/passiveonewindow.
R7 themes: persistedSystem/Light/Dark inGlobal; allcustombusinessregions deriveactivepalette,notjustKitcontrols; followOSchanges viaexistingC#adapter; nodependencyupgrade/themefileimport thisslice.
R8 search: Finderprecomputename/pinyin/initialismmaterials oncatalogrefresh; unique-base-name matchreuse before rowfiltering/ranking; selective3gram auxiliaryindex foreligiblequery withcompletefallbackforwide/short/CJK/OR/exclusion/path/fuzzy semantics; noresult/usage/rankingchange, boundedmemory,cancel; syntheticbenchmarkcomparisonsandgoldenresults.

## Acceptance
AC1 eachR1/R2 revisedproductionbounds/paint tests andactualcapturedvisualinspection(default/min/long/error/disabled) separatelyrecorded; nocontrolremovedtofit exceptrequestedURL/path/copyonly.
AC2 R3 nativefixtureprovesdispatch/result/fallback/foreground/retry once; no realShell withoutseparatepermission. R4 syntheticupgrade/approvalchecks, actualLocalSystemreleasegate deferreduntilauthorized.
AC3 R5 configdefault/legacy/atomicmergetests, originalcapsrestoredonforegroundtransition; singlechangeevents/no movementorbyteticklogs; selectionfulltotalmatchreference.
AC4 R6 clickrouteandexplicitcopytests, noimplicitclipboardwrite; R7 persist/cancel/reopen/livepalettefixture; no realclipboard/themeOSmutation.
AC5 R8 old/newresultsidentical acrossquerymatrix;base+delta/hardfilter/cancelcovered; recordmeasuredsyntheticcost/memory,notclaimcurrentinstalledCPUrelease.
AC6 scopedowner tests, fullDesktop, strictprod/testsClippy,workspace--tests/fmt,affectedgates,ownednativefixtures. Preserveunrelatedchanges/noignoredtests;reportgenuineblockers.

## Out of scope
ABDMdatecolumns/trayspeedtoggle/minpartsetting,newtools,framework/dependencyupgrades,themeimports,realURLs/Clipboard/diskvolume/Shell/serviceoperations,Releasepackaging/installation,commit/push. Userapprovaldoesnotremoveprotectedrelease/nativegates.
