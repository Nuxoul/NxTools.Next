# Phase A — Engine contracts and safety foundation: partial checkpoint

Date:2026-10-19. **Phase A in progress, NOT passed. Only this file owns current state.**
Canonical phase names/order stay in [implement.md](implement.md). No Phase B/A–Fcompletion.

## Current slice — bounded resources measured, one timeout projection repaired

[Pre-run contract](resource-acceptance-contract.md), [complete evidence/limits](resource-acceptance-evidence.md),
[raw21attempts/environment/PIDs/names](resource-acceptance-results.json). Results are **Debug only**.
All historical independent reports and failures remain unchanged. The complete pre-slice checkpoint
was preserved byte-for-byte in [historical snapshot](research/phase-a-checkpoint-before-resource-slice.md);
its internal current/historical wording is old evidence, not this current state.

- Independent CPython100kZIP:100000exact,100directories/100Unicode/9081emptyfiles; bounded1.30MiB
  payload and sequential independentmanifest. Publicengine default admits100k,allnames/kinds/sizes/
  ordinals/session verified. Two fullopen2248.739/2268.197ms;sampledWS60.90/60.84MiB,Private67.03/
  67.23MiB. Explicitlower4096entries/4MiBmetadata -> ResourceLimit. No defaultincrease.
- Actuallegacyworker outputs100000exact/50000batches: first2264.276/2271.717ms,terminal3472.296/
  3490.191ms,WS63.79/63.81MiB,Private67.59/67.64MiB. Legacyframes lack snapshotID;engine verifies
  that identity separately. Directschema1transport is NOT nativeJob/sourcelease acceptance.
- OriginalC#runner/Job unchanged8MiBstdout rejects at46710entries:stdout_total_limit,cleanuptrue,
  slot0/sourcewritable/helperremoved. NOT completed100kworker-through-runner listing.
- Product4096/4MiBread-only defaults reject100kinput typed ResourceLimit,0entries/0batches,
  cleanuptrue;explicitmax_entries100000 -> InvalidRequest. NO product finite100kescalation exists.
  FinalfixedC#directed367.520ms;earlier354.563/383.600ms retained. Sameconnection reuse aftercleanup.
- Actual1GiBStoredZIP native data-cancel after65536bytes:exitobserved+28ms,actualcleanup+42ms,exit0;
  held-cleanup variant131072bytes:+30/+66ms. PredisposalJobempty/processpumpsdrained butsource/slot
  STILLheld;only actualcleanup releases. Workerreceive/observe timestamps NOTexposed/NOTmeasured.
-1GiBproductopen hitunchanged30sdeadline beforefirstbatch;NOT1GiBcredit-wait acceptance. Separate
  independent5entrycredit/full32queueDrop cases cleanup48–59ms,originalpipe/reuse verified;
  finalfixedC#directed54.941/53.221ms. No fake≤200ms arbitrarynative-call guarantee.
- Foundrealdefect:timeout shutdown's lateworkerCancelled replaced originaltimeout as usercancel.
  Sharedsession/originalrunner/real30sdeadline red1fail -> green1pass. One-linepriority correction
  preserves failed/io_failure(existingwire,no timeoutenum),cleanupfalse stillwins. No budget/
  timeout/codec/output/creation/ZIP64/publication/dependency change. Earlierreds/toolerrors retained.

NoUI:firstpaint NOTMEASURED.≤1.5s/≤200ms aretargets,notReleaseverdicts. Ordinaryrepeat/possibly
warmcache only;no systemcacheflush.20mssampling actualintervals/limits andallPIDs inrawresults.
Debug1GiBpredata~72–74s includes repeatedfull-sourcefingerprints;no algorithmoptimizationclaim.

## Final stable verification this slice

| Gate | Actual result |
| --- | --- |
| Complete Archive isolated/offline/serial |138pass/0fail/0ignored,26groups/24nonempty |
| Cancellation/consumer/realtypedchain INCLUDED in138 |10/5/11pass |
| Protocol / IPC full --lib |50/29pass,0fail/ignored |
| Archive/Protocol/IPC strictClippy tests+examples default+isolated |exit0 |
| Archive scopedfmt / worker+probe samefeatureDebugbuild |exit0 |
| Extensions archive_import / Clipboard archive_ |7/7 and5/5pass,0fail/ignored,Clipboard43filtered |
| C#Debugbuild |0warnings/0errors |
| Exactoriginal127 minus ONLYlargeZIP64 + newtimeoutfixture |LIST127/RUN0;RUN127/PASS127/SKIP0/FAIL0/not_run0;names/orderexact |
| Actual4GiB+17ZIP64 |NOTRUNhere;independenthistoricalfinal-codepass retained |
| Finalglobalfmt |FAIL1 from unrelated concurrent Desktop/Clipboard formatting |
| Finalworkspacecheck--tests--offline |FAIL101 from unrelated Desktop GPUI tests/imports/KeyDownEvent fields |

Earlierglobalgreen andtransientClipboardcompilefailure retained;not currentglobalPASS. No full
workspace tests/Desktop suite/Release/nativeuser verification. Exactcommands/timestamps/exitcodes/
selected/run/pass/skip/fail/not_run andcleanup inresourceevidence. Finalstaticgates appendedthere.

## Cleanup / remaining boundaries / stop

At18:49:22:0ownedarchiveworker/testhost/outputdirs/resourcepackages/archive-testpipes;bothfixed
helpercopiesabsent. Large100k/1GiBfixtures hashcheckedthenremoved;independentmanifest/rawlogs/
samples retainedinscratch. Earlyfailedpackage manuallyremoved ONLYafter originalrecordedhelper
hashmatch;test-toolguard now proves paniccleanup. Fourindependentacceptancereporthashesunchanged.
Unrelateddirtyfiles preserved;externalwallhavenCargo/Unityprocessesnotstopped.

RemainingA:product100kfiniteescalation+streamtransport design,broadformats/dictionary/resources/
performance/millionentries/longpath/UNC/nativeuser;stockworker cancelreceive/observetime;1GiBcredit
waiting notmeasured. NativeRecycleBinoriginalidentitycontract/directorycoverage stillunapproved/
unproven,production **native_protection_unproven/NotAttempted**. Signed/protecteddeploymentworker
mapping/FIFO/conflicts/fullA exits remainopen. NoUI/FIFO/productwrite/deploymentmapping,formal
Desktop/installedPlatformHost,Release/package/sign/install,registry/service/drivechange,userarchive/
realrecycle/permanentsourcedelete,commit/push. Stopthisslice;noPhaseB/PhaseA/A–Fcompletion.

## Preserved independent and historical navigation

- [Independent product P2 acceptance](product-read-p2-acceptance.md):historical138/50/29/127with
  actuallargeZIP64once;notthisslice's127(which excludeslarge andadds timeout).
- [Original product findings](product-read-acceptance.md),[P2 repair](product-read-p2-remediation.md),
  [integration](product-read-evidence.md):originalcancellation/overflow/cleanupcontractsunchanged.
- [Original source-delete findings](source-deletion-acceptance.md),[repair](source-deletion-remediation.md),
  [independent repair acceptance](source-deletion-remediation-acceptance.md):nativeidentitystillpending.
- [TAR acceptance](tar-time-acceptance.md),[tree create](tree-create-evidence.md),
  [ZIP preparation](zip-create-output-preparation.md),[matrix](phase-a-capability-matrix.md).
- [Fullpreslicestatus/historicalcheckpointarchive](research/phase-a-checkpoint-before-resource-slice.md)
  retains all prior source-backed implementation and failure narrative without rewriting it.
