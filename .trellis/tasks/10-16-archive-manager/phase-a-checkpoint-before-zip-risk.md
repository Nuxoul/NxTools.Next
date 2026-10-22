# Phase A — Engine contracts and safety foundation: partial checkpoint

Date:2026-10-20. **Phase A in progress, NOT passed. Only this file owns current state.**
Canonical names/order stay in [implement.md](implement.md). No Phase B/A–Fcompletion.

## Current checkpoint — B2 ZIP conflicts

**ZIP conflict slice implemented; scoped checks PASS; independent acceptance pending.**
[Evidence](zip-conflict-evidence.md) / [results](zip-conflict-results.json).
Ask/Overwrite/Skip/AutoRename/task-only apply-all use original port/runner/native publication;
changed-target/cancel/stale tickets and no-replace remain safe. Rust29PASS; C#LIST=RUN=PASS4,
0skip/fail; strictClippy/scopedfmt/docs pass. Otherformats,B3/fullB2/PhaseA/A–F NOT complete.
[Password independent PASS](password-operation-acceptance.md). Historical evidence below retained.
Stop for independent acceptance; no next feature.

## Prior checkpoint — B2 password portion

**B2 password portion complete at scoped implementation/tests; conflict/apply-all remains
incomplete. Pending independent acceptance. NOT full B2,ENG03/ENG09,Phase A or A–F.**
[Evidence](password-operation-evidence.md), [results](password-operation-results.json),
[sources](research/password-operation-sources.md). B1/deliveredC4 are
[independently accepted PASS](zip-encoding-acceptance.md), not reopened.

- Rust original worker owns opt-in schema2 open/test password interaction via existing
  InteractionPort; one original source/Job/worker, fresh actualattemptIDs/originallogicalID,
  max3actualattempts. Original path/codepage/op/limits retained; no ArchiveRead v1/v2 change.
- PasswordRequired/explicitWrongPassword prompt; ambiguousAuthenticationOrDataError terminal.
  Installedticket+secretresponse, staleanswer ignored; typedCancel/originalCancel end without
  laterattempt and actualsource/image/slotcleanup. Open/test cannot publish/overwrite outputs.
- Realpending and outerCancel mapping reds->green. FinalC#LIST=RUN=PASS4,0skip/fail/not_run;
  seven tiny RAR5scenarios plus originalcodec/error/readprivacy contracts. Rust16+1+1pass;
  strictClippyisolated/default,archivefmt,C#build0warnings/errors,matchingworker PASS.
- Globalfmt NOTpassed: unrelated unedited Desktop main.rs283 formatting. Preserved,notfixed.
  Prior fixture wait race fixed: await actualexited before checkingDrain; failedrun retained.
- Historicalreports/results retained. No100k/longpathmatrix/largefile/resourcesampling/UI/FIFO/
  risk/PhaseB/UNC/recycle/system/package/install/sign/commit/push. Stop for independent acceptance.

## Prior checkpoint — B1 ZIP Auto candidates and safe reprojection

**B1 and C4 implemented; targeted checks passed; pending independent acceptance.**
[Implementation evidence](zip-encoding-evidence.md), [results](zip-encoding-results.json),
[reference/source boundary](research/zip-encoding-sources.md). Phase A NOT passed; no Phase B.

- Unicode authority -> manual page -> bounded ZIP UTF8/GBK/ShiftJIS candidates. Script/length
  evidence, no decode-success-only selection; short/competing candidates remain uncertain/manual.
  Heuristic+codepage uses existing metadata fields and still carries UncertainEncoding.
- Reprojection checks original revision, keeps session/raw ordinal and reuses original path/
  collision checks. New unsafe/case/file-prefix plans refuse before writes; old projection
  plans refuse at extract with0output calls. Big5/CP949/1250/1251/1252 literal assertions pass.
- Actual domain/product/native Auto CP437 red -> green; actual product metadata and native
  exact name/29bytecontent verified. Stale-plan boundary red3pass/1fail ->4/4green.
- Final small archive40pass/0fail,7targets/100k1filtered; latest encoding4/4; targeted actual
  product1/1; protocol54/IPC30; C#exactLIST=RUN=PASS5,0skip/fail/not_run. C#apphost0warnings/errors
  then matchingisolatedworker; singleCargo channel. StrictClippydefault/isolated and globalfmt pass.
- Test-selection mistake: earlier inaccurate skipnames ran old domain100k and productlocal-long
  source tests once (both passed); disclosed,not authorized/necessary B1resource acceptance.
  No repeat,1GiB/largeZIP64/performance. Optionalreview stopped without report,NOTreviewPASS.
- Remaining B2/B3,C1/C2/C3/C5,D1–D3 unchanged. Original ledger/report/results remain historical;
  no Phase A/A–Fcompletion,UI/prompt/FIFO/UNC/recycle/system/package/install/sign/commit/push.
  NativeRecycle stays native_protection_unproven/NotAttempted. Stop for independent acceptance.

## Prior checkpoint — Phase A exit convergence, documentation only

[Exit ledger](phase-a-exit-ledger.md) maps every canonical ENG01–09,SAFE01–08,P201/touchedIPC01
subcondition to production/test evidence and A/B/C/D/E status. **Phase A NOT passed; no Phase B.**

- Latest independent PASS: [finite100k correctness/Debugresource/originalconnectionreuse](product-100k-post-status-acceptance.md),
  [localZIP Windows long paths](windows-long-path-acceptance.md),
  [actualworker cooperative receive/observe/credit/pause/nativecleanup](worker-cancel-acceptance.md).
  These conditions are closed at their bounded scope,not complete ENG08/ENG09. No repeat this turn.
- Remaining FUNCTION gaps: B1 boundedAutoencoding detectablecandidates+safetyreprojection;
  B2 actualoperation typedpassword/conflict/retry/apply-all/cancel orchestration;
  B3 ratio/risk finiteapproval core. Existing InteractionPort standalone tests are not integration.
- Named EVIDENCE gaps: C1emptyGzip/XZcreate;C2real7zBCJcoder/exactbytes;C3signature/ZIPalias assertions;
  C4othermanualpages/changed-decoding collision;C5solidpackedunknown/optionalmetadata assertions.
  No generic broaderformat/resource/method×size obligation added;see ledger for exact exits.
- DECISIONS: D1 Aexit versus Cfirstpaint/DpublicUI and originalmeasurement wording;
  D2 strictoriginalobject/neverpermanent nativeRecycle and directorycoverage;
  D3 trustedUNC first-release scope;count ceilings clarified by layer,not a millionbenchmark blocker.
  FIFO/UI/Shell/deployment are not silently pulled into A;unresolved originalexit dependencies stay open.
- Sole recommended next slice: **ZIP bounded Auto filename candidates and safety re-projection**
  (B1,ENG05/SAFE01/02),with tinyindependentrawname actualproduct/native red and ambiguity/Unicode/
  manualcollision controls. No code/UI/dependency/systemauthority or largecase run thisturn.
  Await a separate implementation instruction;do NOT begin it from this document-only checkpoint.
- Staticcloseoutonly:spec106/14,context82/79,gitdiff--check exit0;19IDs/40links valid.
  Opening234files:5authorizedcurrentdocs changed/229unchanged,93historicaltaskfiles unchanged;
  unrelateddirtydiff and originalhistoricalcheckpoint content preserved. No product/build execution.

Current [capability table](phase-a-capability-matrix.md) updated;historicalsections/reports/results,
oldFailed/Stopped and unknowncauses preserved. NativeRecycle remains
**native_protection_unproven/NotAttempted**;outputoverwriteTOCTOU is NOT sourceRecycle approval.

## Prior slice — real worker cancellation (subsequently independently accepted)

The following submitted pending status is historical; the later [independent acceptance](worker-cancel-acceptance.md)
is PASS for named cooperative boundaries only. Original evidence/results remain byte-identical.

**Bounded cancellation regressions complete; pending independent acceptance.**
[Contract](worker-cancel-contract.md), [evidence](worker-cancel-evidence.md), [results](worker-cancel-results.json).
Only named cooperative ENG09/SAFE05/07conditions; full ENG09/PhaseA/A–F remain incomplete.

- Reference-first legacy/Archiver/SupaZip cancellation sources inspected; no blocked-I/O interruption
  or reference nativecleanup inferred. Original cancel/queue/reader/Job200msgrace/publication unchanged.
- Added one private Cancelled-terminal observation object: actual original-input received_us and
  first work-thread observed_us/fixed checkpoint/paused_checkpoint/native_reply/metadata_credit enum.
  Same worker monotonic origin only; no crossprocess subtraction or perchunk logging/public pause.
- Real ZIP extract held first successful65536byte native write before reply; originalCancel precedes
  reply FIFO. Actualworker Cancelled/ExitCode0,1write/0commits,oldtarget+foreignpartial retained.
  Process/Job/pumps drained BEFORE native resource barrier: source/ownpartial/slot stillheld,noexited.
  Two originalcleanup-held slots refuse third concurrency_limit; release->cleanuptrue/slot0/source
  writable/ownpartialremoved,then realfresh test succeeds. No source delete/recycle.
- First red LIST1/RUN1/FAIL1exit1: missing actualworker receive/observe facts only; prior lifecycle
  assertions passed. Same case green1/1, final precise C#LIST=RUN=PASS6/6,0skip/fail/not_run.
- Actualworker metadata-credit cancel with noACK returns original Cancelled and observedcredit;
  realtypedIPC productexplicitcancel has0laterbatches,owner=remote cleanuptrue,source/helperreleased,
  samegeneration freshTest2files/12bytes succeeds. Productcase doesnotinvent worker timestamps.
- Paused OperationControl Condvar wait confirmed, cancel wakes withoutresume/Cancelled;
  receive timestampabsent forlocalcancel. Existingpausedengineopen stillpasses.
- Directed/final worker native-reply receive->observe26/54us,credit14/13us; localpaused79us.
  Productcaller->remote cleanup44.134/42.138ms. Boundedtarget only,notarbitrarycodec/Win32/cleanup promise.
- FinalRust77pass/0fail/ignored,8groups,100k+acceptedlongsource2filtered. StrictClippytests/examples
  isolated/default,globalfmt,workspacecheck--tests--offline exit0 (compileonly,NOTworkspacetests).
- Closinggates:offline40PASS/7roots26rawfiles retained;90historicaltaskfiles,priorlongpath186raw and
  100k9new+12olderrawunchanged;unrelateddirtydiffpreserved. Docs106/14,context80/77,selector/diffPASS;
  reference32766bytes belowlimit,existingnativewarningretained. Final0archiveprocess/pipe/helpers.
  C#apphost0warnings/errors;matchingisolatedworkerbuiltlastbeforeC#execution,singleCargo lane.

The [localZIP long-path slice](windows-long-path-acceptance.md) and [finite100k observation/reuse](product-100k-post-status-acceptance.md)
are independently PASS; no fullmatrix/largecase repeat. HistoricalFailed/unknowncauses unchanged.
NativeRecycle **native_protection_unproven/NotAttempted**. NoUI/PhaseB/package/systemchange.

## Prior slice — Windows local long paths (subsequently independently accepted)

The submitted pending state below is historical; later [independent acceptance](windows-long-path-acceptance.md)
is PASS for the bounded localZIP slice only. Original evidence/results remain byte-identical.

**Long-path regressions complete; pending independent acceptance.**
[Contract](windows-long-path-contract.md), [evidence](windows-long-path-evidence.md),
[results](windows-long-path-results.json). Phase A/ENG08/A–F remain incomplete.

- Reference-first legacy/Archiver/SupaZip source follow-up recorded before test code. No reference
  runtime long-path support inferred. Production Rust/C# native/policy/IPC and dependencies unchanged;
  added actual-boundary regressions only, no needless implementation refactor.
- Original Rust ArchiveReadClient -> typed IPC -> production C# Session -> runner/Job/source lease ->
  actual isolated-codecs worker open/test succeeds at446UTF16 CJK source path.3exact members/2batches,
  test2files/12bytes, owner=originalremote terminals, same connection and true cleanup/source/helper
  release, original endpoint retirement then normal host exit. Internal public prefixes still reject.
- ZIP native extraction source432/root424–439/final767–782UTF16; member342units, legal components.
  Independent exact payload/emptyfile/emptydirectory/complete structure and DOSmtime verified.
- ZIP creation source root423/payload766/output436–447UTF16. Original held-volume-GUID enumeration
  and source reads, full held-partial verification, CPython exact10member/hash oracle pass.
  External Python requires oracle-only extended path; public requests remain ordinary local DOS.
- Extract/create each8publication cases: new+approved overwrite, unapproved retention, target change,
  precommit cancel, injected flush5, in-flight cancel, injected postcheck5. Count at native_published
  and terminal preserve actual1commit; old targets preserved before commit, published outputs retained
  afterward, foreignpartial untouched, ownpartials/resources clean. AcceptedTOCTOU unchanged/notID-CAS.
  8unsafe-member real extractions refuse UnsafePath/0commits/nooutsidewrite;4internal prefixes reject.
- Final C# LIST=RUN=PASS exact23/23,0skip/fail/not_run; new4fixture subset4/4. Rust affected62pass,
  9groups/0fail/ignored, existing100kfiltered; productchain22/22 includes newlongcase. Protocol53/IPC30,
  ExtensionsZIP7/ClipboardZIP5pass. StrictClippy isolated/default tests+examples, globalfmt and
  workspacecheck--tests--offline exit0 (compilation only,NOTworkspace tests).
- Preserve test-infrastructure first failures:4MiBnewtest budget, externalPython ordinarypath oracle,
  newfield compile/adjacentdeclaration error and listenertask-only teardown; fixed tests only.
  Interruptedbackgroundcheck has no exit ledger/notPASS; completedforeground checks separately retained.
  All small inputs/outputs/failed attempts retained; no source recycle/delete acceptance.
- Closing offline95checks PASS:17retained roots/186rawfiles,85historicaltaskfiles and accepted100k
  9new/12oldrawinputs unchanged. Unrelateddirtydiffpreserved;0ownedarchiveprocess/pipe/fixedhelpers.
  Docs106/14,context77/74,selector/diff gates PASS; referenceindex32654bytes,existingnativewarning.
  Optional review stopped without completed report,notreviewPASS; independentacceptance pending.

The separately authorized finite100k correctness/Debug observation/originalconnection reuse is
now [independently accepted PASS](product-100k-post-status-acceptance.md), no repeat thisslice.
Old independentFailed/Stopped and unknowncontender remain historical. NativeRecycle remains
**native_protection_unproven/NotAttempted**. No1GiB/largeZIP64/performance/UI/PhaseB/packaging/systemchange.

## Prior slice — post-status-repair product100k observation (subsequently independently accepted)

The following preserves the submitted observation facts. Its then-pending status is historical;
the later [independent acceptance](product-100k-post-status-acceptance.md) is PASS for this bounded
observation only. Original evidence/results remain byte-identical; full PhaseA/ENG08 is not passed.

**New observation complete; pending independent acceptance.** This turn expressly authorized ONE
freshcatalog-only100kZIP/streamingmanifest and ONEproduct100k-v2correctness+Debugresourceobservation
with originalconnection postcleanupreuse. No implementation/toolchange, rebuild, extrachain orretry.
[New evidence](product-100k-post-status-observation.md), [new ledger](product-100k-post-status-observation-results.json).

- CurrentPython tools independently42/42pass/exit0. ExistingfreshDebugworker/probe fingerprints
  isolated-codecs;scope386file/source/binaryhashbaseline preserved. NoCargo/dotnetbuildneeded/run.
- Newrun55bde2b1-9f51-4954-964f-1dfa74c643ac: **operationSucceeded/measurementComplete/
  stopNotRequested/cleanupConfirmed/resource_verdictPASS**,driver/probeexit0, noforce/failure.
  ActualRustowner->typedpipe->C#productionSession->originalrunner/Job/sourcelease->actualworker.
-100000manifestexactmembers/391batches/snapshot9ce8cd0d-5a79-4a9e-a99f-b016274c9ddc;
  contiguousordinal/sequence/Zip/encodingverified;owner/remoteterminalequal/fullcounts.
  DefaultResourceLimit and v1explicit100kInvalidRequest remain. Original30s/64MiBquotasunchanged.
- Debugfirstbatch3273.9116ms/fullproduct11399.6498ms,runner11392ms;private16745255bytes,
  C#receipt16708854/Rust16707854bytes,maxbatch42817bytes/256entries,inflight1;Jobreservation256MiB.
  Sameclient/bridge/singleone-shotpipehost1496 successfullyreadmitted creditZIPafteroriginalcleanup;
  followupsuccess+actualcleanupassertionscompleted and **same_router_reused=true** written afterward.
  Numericgeneration/fullfollowupterminalnotserialized;explicitassertions/samehostpipe/guardproof,
  NOT merelya laterworker. Source/helper/Job/pumps/slotsclean,packageoriginalguardremoved.
- ExactPIDs caller23352/host1496/100kworker14956 with359/355/326validsamplepairs,0errors/allended.
  WorkerWS67133440/private70897664bytes;actualsamplegap68.23ms. DebugfirstbatchNOTfirstpaint,
  sampledpeakNOTabsolutepeak,cumulativebytesNOTRSS;noReleaseperformancepromise.
- status_reads driver1051/sampler188/final1attempts;retry/recovery/exhaustion/failure/nativeerrors0;
  initialmissingstatus2driver+1sampler,finalfreshCleanedreadpresent. Nooldfallback/nohiddenerrors;
  thisrunhadnosharingconflict,notproofallwriterracesfixed. Newinputs/attempt/JSONLretainedunchanged.
- Closinggates:spec106docs/14owners,context73/70,selector/diff/scopedhygienePASS;
  existingnative-contractssizewarningunchanged.382baselinefiles/82historicaltaskfiles and
  9newraw+12oldrawfiles hashunchanged;source/binary/trackeddirtydiffunchanged.
  Final0ownedarchiveprocess/pipe/fixedhelpers;inputs/manifest/attempt/JSONLretainedforindependentcheck.

Original [independent100kacceptance](product-100k-acceptance.md) **Failed/Stopped** remainsimmutable,
with followupreuseunverified in THATattempt. [StatusreadP2](status-read-p2-acceptance.md) is prior
independentlyacceptedtoolrepair. Newsuccesscannotrewrite/explainoldfailure;contenderstillunproved.
Onlynewobservationdone, NOTfullslice/ENG08/PhaseA/A–F/native/deploymentsignoff. No1GiB/largeZIP64,
Desktop/installedhost/package/install/sign/commit/push/source recycle/systemchange. Stopthisslice.

## Prior slice — independently accepted bounded status-read observation-tool P2

[Independent product100k acceptance](product-100k-acceptance.md) governs current evidence:
primaryactual100k correctness PASS(100000manifestexact/391batches/originalsuccess+cleanupconfirmed),
but its ONEfullobservation remains **Failed /Stopped(driver_exception)** after statusPermissionError.
Follow-up **sameconnectionreuse was NOTverified**. CompletePIDmeasurement/originalterminal cannot
turn thatattemptPASS. ExclusiveWin32handleprobe reproduced errno13/recoveryafterclose, but the
originalattempt's exactcontender/cause remainsunproved. NoPhaseA/ENG08/A–Fcompletion.

[Repaircontract](status-read-p2-contract.md), [Python-only repair evidence](status-read-p2-evidence.md).
Main/sampler/finalshare nativecode-aware boundedreader: retryONLY32/33open/read conflicts,
max4attempts/10msbetween; native5/unknowncode/invalidstatus stayfailclosed. Diagnosticcounters and
<=16fixeddetails retainattempt/recovery/exhaustion/nativefacts, no payload/perpollprose. No lastgood
return/finalfallback; staleCleaned cannot certify currentterminal/cleanup or resourcePASS.
Actualred2tests(1error1failure)/exit1 -> samegreen2/exit0; finalstatus14/14,Pythonfull42/42,
0failure/error/skip/exit0,PythonAST4PASS. Tinyrealfile/sharetests only; noarchiveworkersmoke.
Rust/C#publishers/products unchanged; noCargo/dotnet/100k/1GiB/largeZIP64run thisrepair.
Actualclosinggates:spec106docs/14owners,context71/68entries,selector/diff/AST/scopedhygienePASS;
existingnative-contractssizewarningretained.79historicaltaskfiles/publishershashunchanged;
wholetrackeddirtydiffunchanged;0ownedprobe/host/worker,0testpipes/fixedhelpers. No fullresourcePASS.

This toolimplementation fix was [independently accepted](status-read-p2-acceptance.md), NOT complete
resource/lifecycleacceptance. The old independentFailed/reuse-unverified remain original. The later
separatelyauthorized newobservation above completed once and is pending itsownindependentacceptance.
No broadreview/optimization/nextfeature/PhaseB; NativeRecycleunproven.

## Historical implementation — explicit finite product100k readonly transport

[Contract/budget/compatibility matrix](product-100k-contract.md), [actual implementation evidence](product-100k-evidence.md),
[durable results/all attempts](product-100k-results.json). These are historical implementation
claims/results, not a current independent full-resource signoff; current acceptance/toolstatus above.

- Default/v1 stays4096entries/4MiB,2entrybatches/legacy8MiBstdout. Explicit per-operationv2
  permits<=100000/128MiBmodel,64MiBtransport/64MiBcumulative receipt,<=256entries/256KiBbatch,
  oneinflight. ACKdoesnotrefund historicalreceipt. Jobwork+metadata+64MiB/max1GiB/aggregate2GiB,
  processwide max2actualcleanup reservations and original30stimeout unchanged.
- Legacy-compatible bootstrap observes realv2capabilities, then oneoriginalpipe upgrade before
  source reads; strictoldworker/missingcapability/unknownversion failtyped. No extra process/queue
  runtime/write/recycle/helperoverride. Fullsnapshot/ordinal/sequence/format/encoding/counts retained.
- Firstsampledattempt Stopped/measurementComplete/cleanupIncomplete retained unchanged. Deterministic
  originalplatformpipe partial-frame cancellation red returnedtail after8192consumedbytes; fixed
  connection-owned boundedprefix, no secondreader/queue. One declaredrepeat ONLY thenPASS.
- SAMEindependentCPython100kmanifest:100000exactentries,391batches,samesnapshot,fullterminal.
  Debugfirstbatch2275.9759ms,fullproduct7006.9153ms,private16745255bytes,C#receipt16708854bytes;
  maxbatch42817bytes/256entries,inflight1. DefaultResourceLimit and sameconnectionreuse were
  implementer evidence; later independent fullobservation did NOTverify follow-upreuse.
- ExactPIDs caller25592/host2556/100kworker18788 sampled245/243/223validpairs,0counterfailures,
  noalive_at_end. Worker sampledWS67010560/private70864896bytes vs256MiBJobreservation;
  samplednotabsolutepeak,firstbatchnotfirstpaint,notcontrolledcold/warmcache or1.5sReleaseclaim.
- Actualsuccesssource/helper/Job/pumps/slot/packagecleanupConfirmed. Failedattempthelper later
  hash/exit/unlockchecked thenremovedseparately, originalIncompleteunchanged. OwnZIPshashverified
  thenremoved; manifest/attempts/status/JSONL/logs/faileddefaultterminal retained. Historyhashesunchanged.

### Historical implementation verification (not this Python tool-repair execution)

| Gate | Actual final result |
| --- | --- |
| Archive complete isolated/offline/no-fail-fast/serial |150pass/0fail/0ignored,26groups,exit0 |
| Protocol / IPC --lib |53/30pass,0fail/ignored,exit0 |
| Strict Clippy tests+examples isolated/default |both exit0 |
| Globalfmt /workspace check --tests --offline |both exit0; check is NOTworkspace tests |
| Extensions archive_import /Clipboard --lib archive_ |7/5pass,0fail/ignored,exit0 |
| resource example /Python tool tests |11/28pass,0fail/skip,exit0;PythonAST2PASS |
| C#Debug build /same exactLIST->RUN |0warnings/errors;134/134,0skip/fail,exit0;old132+new2names retained |
|100kcorrectness+sampling |1Stopped+1declaredrepeatPASS;no further100k/1GiB/largeZIP64 |
| Docs/context/catalog/selector/diff/hygiene |PASS106docs/14owners;68/65context;79/79/78catalog;selector/diff/scopedchecksPASS |
| Preservation/cleanup |74historicaltaskfiles+bothattemptsamplehashesunchanged;0ownedprocess/pipes/fixedhelpers |

One boundedreadonlyreview's3concretefindings red->green; nosecondreview. Additional realpipe
partialreadred->green afterfirst100kattempt. OriginalC#wholeprocessexitcausestillunknown,not
diagnosedbythisrepair. Finaldoc/selector/hygiene results are recorded in additiveevidence.
No Desktopsource/UI/FIFO/write,Release/package/sign/install/commit/push,systemconfiguration,
userarchive/nativeRecycle/permanentsourcedelete. NativeRecycle staysnative_protection_unproven/NotAttempted.

## Prior slice — independently accepted retained-output summary P2

[Additive repair evidence](resource-summary-p2-repair.md) records the actual red/green and input
hashes. Valid attempt JSON containing malformed/truncated RESOURCE lines now retains valid events
and later attempts. Independent `summary_parse=Incomplete` exposes incomplete parsing without
rewriting original schema2 facts/verdict, including PASS. Error details are fixed-category/line
only, capped at16 per attempt with total count/truncation flag; no copied payload. Legacy and
dynamic test-count behavior are unchanged. Scope: summarize.py/test_summary.py plus necessary docs.

Prior summary repair: directed red1error/exit1 -> green1pass/exit0; summary8/8 and Python28/28, no skips,
AST2files pass. Actual collect+CLI two-attempt probe exit0; both attempts and all13input bytes/
hashes preserved. Spec106documents/14owners and context65/62pass,exit0; existing unrelated native-
contracts context-size warning retained. Diff/whitespace/conflict/temporary-marker checks pass;
all73historical task files/hash unchanged. Parallel Clipboard edits were observed, not touched.
No Cargo/C#/worker smoke/sampling/100k/1GiB/largeZIP64/package/install/commit/push in that prior repair.

[Prior independent acceptance](resource-tool-repair-acceptance.md) passed original safe-stop,
required-sample and dispatch-timing repairs, and ran one integrated C#132/132 with zero skips/fails.
These are prior independent results, NOT this run. The historical early-exit cause remains unknown;
the newer132/132 does not erase or explain it. Only the summary P2 is closed by this repair;
Phase A/ENG08/ENG09/A–F are not complete and no next feature is started.

## Prior tool correction — independently accepted baseline

[Minimal repair contract](resource-tool-repair-contract.md), [independent additive evidence](resource-tool-repair-evidence.md),
[new raw results/commands/PIDs/all attempts](resource-tool-repair-results.json). Original
[resource review](resource-acceptance-review.md), [resource evidence](resource-acceptance-evidence.md),
[historical results](resource-acceptance-results.json) and independent reports remain unchanged.
Previous current checkpoint preserved byte-for-byte as [historical snapshot](research/phase-a-checkpoint-before-tool-repair.md).

- Tool-only P1: threshold/deadline/sampler/driver exception requests unique run-scoped stop;
  original engine control/product cancellation/C# runner drains its actual resources. Exact-owned
  force is finite explicit fallback, never cleanup=true by root death. Init/duplicate/late/broken
  channel paths retain facts. No new product wire command or second native Job/process runtime.
- Tool-only P2 validity: operation, measurement, stop, cleanup separate. Required exact PID/image/
  original-parent roles retain failed acquisition/image/counter counts and zero-sample records;
  incomplete/unavailable never green, no invented0MiB. C# count/peaks commit only after both reads.
- Tool-only P2 time: setup separately saved; operation origin immediately before runner.Start;
  firstbatch/fullterminal/cancel/exit/cleanup same dispatch domain. Virtual setup delay red->green.
  Legacy fields remain original/setup-inclusive in summary views, not recalculated historical scores.
- Actual original runner/Job/worker tiny256KiB data barrier produced65,536byte partial, original
  cancellation before release, cleanup/source/helper/partial/slot convergence. Fixture LIST/RUN/PASS.
- Exactly one external real sampling smoke: Stopped, not resource PASS; noforce,measurementComplete,
  cleanupConfirmed. Exact hostPID48348:15valid samples;workerPID47400:7;both exited. Source/helper/
  Job+pumps/partial/slot all released. Requested20ms/50msdiscovery,actual gaps/missedpeak caveats retained.
- Rust functional-only small original-product stop and controlled guardfailure each clean original
  pipe/host/source/helper/package, noforce; stopped original operation never reused. guard exit101
  is failure with confirmed cleanup, not product success. No further memory sampling/performance repeat.

No codec, budgets, product timeouts, timeout-to-cancel product fix, native publication/recycle or
Desktop/Clipboard/UI change. No100k/1GiB/4GiBZIP64 rerun, systempressure/cache/pagefile manipulation.
Firstbatch remains not firstpaint. No full ENG08/ENG09/SAFE04/Phase A claim.

## Historical pre-summary implementation verification (not this slice)

| Gate | Prior implementation result |
| --- | --- |
| Python unique tools / AST |24pass,exit0 /8scripts PASS |
| Rust resource example tool tests |11pass/0fail/0ignored |
| Full Archive isolated/offline/serial |138pass/0fail/0ignored,26groups |
| Protocol / IPC --lib |51/29pass,0fail/ignored |
| Strict Clippy Archive/Protocol/IPC tests+examples default+isolated |both exit0 |
| Scoped fmt / global fmt --all --check |PASS /exit0 |
| workspace check --tests --offline |exit0; compilation only,NOT workspace tests |
| Shared ZIP Extensions --test archive_import /Clipboard archive_ |7/5pass,0fail/ignored |
| C# final Debug build |exit0,0warnings/0errors |
| New C# tool group |all5 names current PASS coverage,includes actual tiny partial |
| LargeZIP64/100k/1GiB measurements |RUN0thisslice;historical only |

### Historical integration exit — cause still unproven

Exact original127 (unique largeZIP64 excluded, prior timeout retained)+5new =132 names LISTed.
Firstrun with missing explicit fixture/oracle config:132run/77pass/55skip/0fail,exit0;not fullPASS.
Configuredrun: **process exit1,RUN100/PASS99/SKIP0,missingFAIL/SUMMARY**,lastRUN ZIP Zstd native reads.
No live ownedprocess left; exact leftoverhelper hashmatched then removed after no-ownercheck.
One targeted interruptedcase1/1PASS and uncompleted suffix32/32PASS; all132exact names have current
PASS coverage. At that time it was **not one integrated132/132 run**. Cause of the configured
whole-process exit remains unproven; evidence/results retain the anomaly unchanged. Subsequent
independent acceptance ran ONE current integrated132/132 to normal completion, zero skips/fails;
that closes the missing-current-full-run evidence, not the historical cause. No C# execution
was part of that prior Python summary repair; this slice's fresh134/134 is listed above.

Historical globalfmt/workspace failures and the prior implementation's later passes remain in
their original reports. Current compiled/native verification is separately listed above.
Initial wrong Extensions namefilter selected0tests; corrected explicit --test target7pass retained
separately. Exact historical logs/commands/exits/names remain in the prior results.
One bounded review's actual internal-stop-grace/output-directory-confirmation findings were red
then fixed;Python24/C#fake4+actualsmall1pass. No second sampling/broad-suite/review pass. A permanently
hung Win32 counter call was not reproduced or claimed interruptible; documented in tool evidence.

## Cleanup / restrictions / remaining Phase A

Final exact ownedprocess/pipe/helper/fixture/history/diff/static observations live in
[tool hygiene](resource-tool-final-hygiene.json) and additive evidence. SmallZIPs removed only after
recorded hashes+confirmedcleanup;manifest/status/rawsamples/logs retained. Unrelateddirtychanges
and externalwallhavenbuilds preserved. No commit/push,Release/package/sign/install,formalDesktop/
installedPlatformHost,registry/service/drivechange,userarchive/nativeRecycle/permanentsourcedelete.

Current remaining PhaseA conditions are ONLY the exit ledger's B1–B3 function gaps,C1–C5 named
evidence gaps and D1–D3 explicit dependency/native decisions. Accepted finite100k,localZIP longpaths
and workerreceive/observe/cleanup are not reopened. Million-entry PERFORMANCE is not a new gate;
operation-specific finite ceilings remain explicit. UI/FIFO/Release/native-user stay with original
owners,not generic A blockers. NativeRecycle remains **native_protection_unproven/NotAttempted**.
No PhaseB or nextimplementation begins in this documentation-only convergence.

## Preserved historical navigation

- [Resource independent review](resource-acceptance-review.md),[old measurements](resource-acceptance-evidence.md),
  [pre-tool checkpoint](research/phase-a-checkpoint-before-tool-repair.md): historical numbers/failures unchanged.
- [Product independent acceptance](product-read-p2-acceptance.md),[original findings](product-read-acceptance.md),
  [P2 repair](product-read-p2-remediation.md),[integration](product-read-evidence.md).
- [Source-delete findings](source-deletion-acceptance.md),[repair](source-deletion-remediation.md),
  [independent acceptance](source-deletion-remediation-acceptance.md);nativeidentity remainspending.
- [TAR](tar-time-acceptance.md),[treecreate](tree-create-evidence.md),[ZIP preparation](zip-create-output-preparation.md),
  [formatmatrix](phase-a-capability-matrix.md),[earliercheckpointarchive](research/phase-a-checkpoint-before-resource-slice.md).
