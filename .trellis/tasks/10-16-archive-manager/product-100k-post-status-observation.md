# Post-status-repair product100k observation — pending independent acceptance

Date: 2026-10-20. Canonical **Phase A — Engine contracts and safety foundation** remains incomplete.
**New observation complete; pending independent acceptance.** This is a new authorized execution
record, not self-signoff of the whole slice/ENG08/PhaseA/A–F/UI/native-user/deployment gates.
[New result ledger](product-100k-post-status-observation-results.json);
[finite contract](product-100k-contract.md); [accepted status repair](status-read-p2-acceptance.md).

## Authorization and unchanged historical facts

This turn explicitly authorized ONE fresh catalog-only100kfixture, ONE product100k-v2 correctness
and Debug resource observation, including successful postcleanup readmission on the same original
connection. No automaticretry, performance repeat, other large case, implementation/tool change or
broad review. Actual product100k primary correctness already independently passed in
[original acceptance](product-100k-acceptance.md), but that old full observation remains
**Failed/Stopped(driver_exception)** and its follow-up reuse remains unverified in THAT attempt.
The status-read P2 was independently accepted; the old contender/cause remains unproved. This new
success does not overwrite, explain or retroactively reclassify any prior failure or writer race.

## Preflight, binary source and executed commands

Opening scoped SHA256baseline:386files (task/history, Python tools/tests, Archive/Protocol/IPC and
C#production/test sources, relevant specs and actual Debug binaries), stored in session scratch
post-status-100k-baseline.json SHA256
`9c2fd98d2f11d1f71782337ab93d8adfcadb0b459baf2610900075f2509f7199`.
All source/binary hashes checked again before dispatch and after completion. Source/layout is the
current approved implementation; no code or dependency change this turn. Existing worker/probe
fingerprints explicitly record isolated-codecs and `-C codegen-units=256`; relevant executable/DLL
outputs are newer than their Rust/C#source. No stale/missing output or other need to rebuild was
found, so **Cargo/dotnetbuilds RUN0, extra small actual-chain validation RUN0**. No broad suites were
rerun or historical test counts represented as current checks.

| Executed binary | Actual SHA256 |
| --- | --- |
| target/debug/nxtools-archive-worker.exe |b3adcdfa7a963132f963c23b46e233072accb4358e6b9364023df1d770f49b63 |
| target/debug/examples/resource_probe.exe |9150fb002231f0511ce45bf7d799905a5eb69d875d522287886442feabb328cf |
| isolated TEST apphost NxTools.PlatformHost.Tests.exe |c4c3fb04e085b2ca48261c7a8def7df60e6236410bc9493ba54ec3b958be244e |
| same output's tests DLL /product DLL |727358361e3aba4ccea225be42b51eb7a2556bc5a1885fdf2d08a1a037d1f7a2 /e91dfc141663263a9486b82de818e07036c4707e8a00ae5f194905ea3803d6bc |

Machinewide Cargo/rustc and archiveprocesses0, archiveproduct/resourcepipes0, fixedhelper0 before
start. Available memory20470222848bytes/commitheadroom26443866112; Cfree20683292672/Dfree44753063936
bytes, all above unchanged toolpreflight thresholds. No memorypressure/cache/pagefile manipulation.

All Python used PYTHONDONTWRITEBYTECODE=1, cwd D:\Code\NxTools.Next-main.

| Actual command | Exit / result |
| --- | --- |
| `python -m unittest discover -s tests/performance/archive-resources -p 'test_*.py' -v` |exit0,42pass/0fail/error/skip,0.538s |
| `python tests/performance/archive-resources/fixtures.py ROOT --catalog-only` |exit0, freshGUID-ownedroot, no1GiB |
| `python tests/performance/archive-resources/run.py ROOT product100k-v2 correctness` |**ONEexecution**,exit0/probeexit0;10:29:09.616035->10:29:22.147588+08:00 |
| scratch `post-status-100k-assert.py` |exit0,42postrun assertions PASS, no product execution |

A separate create-new startuprecord and subprocessscript enforce one observation/no overwrite.
No automaticretry/repeat/suffix performance run. Original driver retains its existing150sbounded
parentwait and original30sproductdeadline/ACK/quotas/PASSpredicate; no setting changed. The only
actualchain is Rustowner->typedIPC->C#productionArchiveReadSession in isolatedTESTapphost->original
runner/Job/source/helper lease->actualRustworker, not engine/directworker/runner-only. No ordinary
WindowsPlatformAdapter/globalhook/Desktop/installedPlatformHost was constructed.

## Fresh independent fixture and manifest checks

Ownedroot under this session scratch:
`archive-resources-90ea1e79-dcf7-4d8b-9eda-87bbd4318e72`.
CPythonZIP_STORED generated exactly100000members:100directories,100Unicodefiles,9081emptyfiles,
1362285payloadbytes. entries.jsonl is streamed while generating. No1GiBfile or largeZIP64case.

| Retained input | Bytes / SHA256 |
| --- | --- |
|100k.zip |12161983 /34a9f9fe2e1d669b825d3557e523cadf00aeabf1ebfe187c99fcd4475cda956d |
|entries.jsonl |8470828 /4fd4bdca3f5ceb0f388206c1c2a12cf827be97856313f3be85c5efb31e2768dc |
|credit.zip |537 /2e12e154c10fbcb8b35883ab8375c3acd101322156752e3a4ff3a3b1b036e571 |

Product Manifest.batch compared every ordinal/name/kind/size/packed_size/encrypted/provenance/
codepage and same real snapshot/formatZip/contiguous sequence. Manifest.finish checked100000,
EOF/count/batch totals/budget. Manifest_verified was emitted only after owner and captured original
remote terminal exactly matched. No full businessmetadata JSON or second100kproductVec was built.
A separate postrun CPython centralrecord->streamingmanifest comparison checked all100000fields,
Unicode/empty/directory/payload totals again. That read-only ZIPinspection is not another productrun.
Rawbusinessbatchframes are intentionally not logged/copied; this proof combines actual bounded
production assertions plus preserved event/count/terminal facts, not a second fullbatch capture.

Run55bde2b1-9f51-4954-964f-1dfa74c643ac; primaryoperation
5a4b55b1-294a-4c4a-9fb0-fce3e8e282c6; actualsnapshot
9ce8cd0d-5a79-4a9e-a99f-b016274c9ddc. Primary terminal succeeded/codeNull,100000entries,
files_verified(openmembercount)100000,391batches,cleanup_complete=true. No successfultruncation.

## Actual budgets/timing/cleanup and same-connection reuse

| Fact | This observation |
| --- | --- |
| Default read |ResourceLimit and cleanup confirmed before explicit request |
| Explicit v2limits |100000entries/128MiBmodel/64MiBwork;64MiBprivate cumulative and64MiBconsumer cumulative |
| Job reservation |268435456bytes=work+metadata+64MiBoverhead, not sampled memory |
| Batch/inflight |max256entries/42817bytes,391batches,inflight1; ceiling256KiBunchanged |
| Private cumulative framed transport |16745255bytes |
| C#serialized received businessbytes |16708854bytes |
| Rustserialized received businessbytes |16707854bytes |
| First businessmetadata from originaldispatch |3273.9116ms |
| Full original productterminal/cleanup observation |11399.6498ms;runner11392ms;unchanged30stimeout |
| Entire probe inclsetup/defaultrefusal/reuse |12429.1772ms;externalcommand12530.8843ms |
| Independent owner vs remote terminal |equal, succeeded/fullcounts/actualcleanup |
| Original source/helper release |source openedwritable and helper removed after terminal |
| Follow-up readmission and success |same_router_reused=true after all existing success/cleanup assertions |
| Final resources |all3operations remote cleanuptrue, source/helper/pumps/Job/slot/host/pipe cleanupConfirmed;originalguard removedpackage |

Serialized totals differ because C#Unicodeescaping is distinct, not missingmembers. ACKdoesnot
refund cumulative receipt: source reducer keeps monotonically increasing counters, noquotareset.
All exacttransferlimits checked postrun. No30s/budget/ACK/cancel/Job/source/helper changes.

### Reuse proof and generation limits (not inferred merely from a later worker)

Existing product.rs captures original bridge.connection_generation(), asserts it after100k,
checks originalcleanup/source/helperrelease, then uses the SAME client/bridge/host to start a fresh
credit.zip operation. start must return admitted; observe must return succeeded; cleanup_complete
must be true; helper is removed again and guard rereads remote cleanup for each of the three
operation IDs. same_router_reused is written only after those assertions and v1explicit100k
InvalidRequest assertion complete. This observation preserves that exacttrue fact and finalguard
remote_cleanup_complete=true, with no stop/error. TEST ArchiveProductHost connects exactly once,
never reconnects; one host1496and original pipe lifetime covers all three operations. A disconnect
would make oldclient/reuse admission fail, not silently swap to another connection.

Follow-up operation2966702d-60cf-4faa-bb24-f4399ff9c105, actualworker2296; source/image/parent
provenance and validsamples are recorded separately from primaryworker. **Numeric generation and
full follow-up terminal are not serialized by the existing scenario. No numeric value or full
terminal record is invented.** Successful readmission/terminal/cleanup are recorded by the completed
explicitassertions +same_routerfact+allremote cleanupguard, not solely the laterworker. The original
ownedpackage/terminal files were removed by the unchanged guard only after verifiedcleanup.

## Verdict dimensions, fresh status and exact PID samples

operationSucceeded; measurementComplete(allrequiredrolesvalid); stopNotRequested/forcedfalse,
cleanupConfirmed, failureNull/stderr0, **resource_verdict=PASS** under unchanged existing predicate.
This is the new observed result, still awaiting independentacceptance, not signoff of allPhaseA.

status_reads retained main1051reads/attempts, sampler188, final1. Retry/recovery/exhaustion/failure
counts0; nativeerror detailrows0/error_count0/truncatedfalse. Initialstatus unavailable{} occurred
2driverpolls/1samplerpoll before publication; finalread present, matchingcurrentrun/schema2/Cleaned
and actualcleanup, nooldstatusfallback. This run encountered noidentified32/33conflict; zeroerrors
does not re-exercise recovery or prove everywriter race eliminated. Prioraccepted14/42/nativeprobe
is historical evidence for recovery, not restated as a freshlarge-race demonstration.

| Exact PID/image role (parent) | Successful samplepairs / countererrors | Sampled WS /Private bytes |
| --- | --- | --- |
|23352 Rust resource_probe.exe caller(parent8476) |359/0 |9105408 /2723840 |
|1496 isolated C#testapphost(parent23352) |355/0 |63897600 /32841728 |
|14956 actual100kRustworker(parent1496,primaryoperation) |326/0 |67133440 /70897664 |
|7664 separatedefaultrefusalworker(parent1496) |11/0 |kept separate from100kpeak |
|2296 separate5memberreuseworker(parent1496,follow-upoperation) |3/0 |kept separate from100kpeak |

Allfiveexactimages/expectedimages/parents verified; allalive_at_end=false;0acquisition/image/counter
errors. AllJSONLsamplecounts reconcile perPID with records, monotonicactualtimes/positivecounters;
no missingPID treated as0MiB. Expectedworkerimage is exactselfownedpackagecopy of hashedworker,
not conhost/process-nameaggregation. Caller/host entirelifetimes include setup/default/reuse, not
pureopenphasepeak. Requested20mssampling/50msdiscovery, maximumactualvalidgap68.23ms for100kworker.
Sampled peaks are NOT absolutepeaks;PrivateBytes is processcommit, not systemRAM or reservedbudget.
Firstbatch is NOTfirstpaint. Debugordinarycacheaccess/noflush, noRelease1.5s or performancepromise.

## Evidence files, preservation and stop

Newfixture ZIP/manifest/credit/status/attempt/samplefiles are all retained unchanged for independent
inspection. No cleanup of newinputs/unknownfiles. Nine rawfiles were hashed; successfulpackage
removedbyoriginalguard. Postrun0host/worker/probe and0archiveproduct/resourcepipes; fixedtesthelper
copiesabsent. No unknownprocess terminated. Older12rawfiles in THISsession's resource roots hashed
against openingbaseline unchanged. Old independent Failedattempt in another session was not opened,
rewritten or cleaned; its report/status-repairacceptanceopeninghashes are recorded in newledger.
Current product/tool sources and binaryhashes match openingbaseline afterrun. Unrelateddirtychanges
preserved. Only additive evidence/results and necessarycurrentcheckpoint/contextnavigation change.

Session scratch: C:\Users\Administrator\.pi-desktop\scratch\30608b1f-4d13-4908-94fc-6b4d37c1bb7d.
- post-status-100k-command-result.json exactcommand/start/end/exit; SHA256
  0db78d459ce4ab9f13f4ad4c6f653644258d999e97ae25724b5a91e3e1036d88.
- post-status-100k-observation.log SHA256
  922b10b3b416b769f11821fd4b404a432699f41fade3ce6e2baa8683759ad356.
- post-status-100k-assertions.json (42PASS) SHA256
  14fe3ef08fba80512353d7130c755d173ca36cda14253f2ebba2924f5554936d.
- Freshattempt SHA25609f0eff71e8abfaddf148b6259f7575a06a7b5840ccb82675374ac32814dee16;
  samplesSHA256a055d18f1fef2218f204e1eb261733f704db9ae7b87811f428ebbc5c43a117a0.

No codefix, review, rebuild, extraactualchain, measurementrepeat,1GiB/4GiB+17ZIP64,formalDesktop/
installedPlatformHost,Release/package/sign/install/commit/push,userarchive/source delete/recycle,
registry/service/drivemapping/cache/pagefile/OOMpressure. NativeRecycle stays unproven/NotAttempted.
**New observation complete; pending independent acceptance.** OldFailed remainsoldFailed; original
contender/cause stillunknown. BroaderENG08/PhaseA/A–F/UI/native/deployment gates remain incomplete.
Stop after finaldocs/context/diff/preservation checks; no nextfeature/PhaseB.

## Final documentation gates

| Final gate | Actual result |
| --- | --- |
| `python tests/architecture/spec-contracts.py` |PASS106documents/14owners,exit0 |
| `python .trellis/scripts/task.py validate .trellis/tasks/10-16-archive-manager` |PASS73implement/70checkentries,exit0 |
| `tests/architecture/platformhost-test-selection.ps1` |PASSfixture/native selection sourcegate |
| `git diff --check` /scopedwhitespace/conflict/temporary-marker checks |exit0/PASS |
| Opening386file baseline |only4authorizednavigation/checkpointfiles changed,382unchanged;82historicaltaskfiles unchanged |
| New/old rawinputpreservation |9newfiles/12olderTHISsessionfiles hash-identical,allretained |
| Product/tool sources,binaries,dependencies /tracked dirtydiff |unchanged against openingbaseline |
| Finalownedresources |0host/worker/probe,0archiveproduct/resourcepipes,0fixedhelpers |

Existingnative-contracts.md35497bytes>32768contextwarning remains unchanged; unrelatedCRLF
normalizationwarnings were not cleaned or hidden. Finalhash/scope details are in session scratch
post-status-100k-final-hygiene.json. No additionalreview, implementationchange or productrun.
**New observation complete; pending independent acceptance.** All evidence saved; stop here.
