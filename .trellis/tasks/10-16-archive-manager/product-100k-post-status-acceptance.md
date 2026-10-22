# Post-status-repair product100k — independent observation acceptance

Date: 2026-10-20. Canonical **Phase A — Engine contracts and safety foundation** remains incomplete.
Reviewed [new observation](product-100k-post-status-observation.md), its
[result ledger](product-100k-post-status-observation-results.json), [finite contract](product-100k-contract.md),
[prior independent acceptance](product-100k-acceptance.md) and [accepted status repair](status-read-p2-acceptance.md).

## Verdict

**PASS for this newly authorized, single explicit finite100k product read correctness + Debug
resource observation + original-connection postcleanup reuse/cleanup. No concrete in-scope blocker
was found.** This completes independent acceptance of that bounded observation, not all formats,
ENG08, PhaseA, A–F, UI, native-user, Release performance or signed deployment.

Acceptance did NOT run another100k/product observation, build, modify implementation/tests, or
rewrite the submitted evidence/results. It independently inspected preserved raw data, executed
an offline66check verifier and reran the current42Python tool tests. No delegate/review loop.

Historical independent attempt remains **Failed/Stopped(driver_exception)** and its follow-up
reuse remains unverified IN THAT OLD ATTEMPT. New success neither erases nor explains it. Its exact
contender remains unknown; this newrun saw no status conflict and cannot prove all writer races fixed.

## Raw evidence independently checked

Original execution scratch:
`C:\Users\Administrator\.pi-desktop\scratch\30608b1f-4d13-4908-94fc-6b4d37c1bb7d`.
Fresh fixture root `archive-resources-90ea1e79-dcf7-4d8b-9eda-87bbd4318e72`.
Run `55bde2b1-9f51-4954-964f-1dfa74c643ac`; primary operation
`5a4b55b1-294a-4c4a-9fb0-fce3e8e282c6`; snapshot `9ce8cd0d-5a79-4a9e-a99f-b016274c9ddc`.

- Rehashed attempt, samples, command ledger, observation log, original assertion result and final
  status against submitted SHA256values; all match. Command driverexit0 and probeexit0 agree.
- Rehashed the386file opening source/binary baseline: exactly4declared checkpoint/navigation files
  changed,382unchanged. Thus current product/tool sources, actual worker/probe/apphost/DLL outputs
  and reviewed assertion code match that observation's baseline. No rebuild was needed/performed.
- Offline CPython ZipFile central records were compared to the retained streaming manifest for
  all100000ordinal/name/kind/size/packed_size values, checking Store/plain flags. Counts reconcile:
  100directories,100Unicodeentries,9081emptyfiles,1362285payloadbytes. No extra product execution.
- Input ZIP/manifest/credit hashes match the ledger. Product Manifest code independently checks
  encrypted/provenance/codepage, snapshot/format, sequences and terminal totals while streaming;
  raw manifest_verified/fullterminal facts show that boundary completed successfully. Business
  batch payloads were not recorded; offline ZIP comparison is not a claim to replay those batches.
- Primary owner terminal equals captured remote terminal and recorded business event. Fresh final
  status equals retained control/status.json, correctrun/schema2/Cleaned; cleanup is not oldfallback.
- Recomputed all1054raw valid samplepairs grouped by exactPID: counts, positive counters,
  chronological timestamps, maxima and maximumgap match process records. Exact expected images,
  parent/operation provenance, nonzero valid counts and ended states checked for all5processes.

| Observed dimension | Accepted result |
| --- | --- |
| Operation /measurement /stop /cleanup /resourceverdict |Succeeded /Complete /NotRequested /Confirmed /PASS |
| Failure /stderr /force |null /empty /false; no forcedtargets or stoperrors |
| Original primary terminal |succeeded/codeNull/cleanuptrue/100000members/391batches |
| Default request |ResourceLimit with cleanup, before explicitv2 |
| First business batch /full primary terminal-cleanup |3273.9116ms /11399.6498ms;runner11392ms<unchanged30s |
| Private framed cumulative /C#business /Rustbusiness |16745255 /16708854 /16707854bytes, independent64MiBquotas |
| Maxbatch /inflight /Jobreservation |256entries,42817bytes /1 /268435456bytes |
| Fresh statusread diagnostics |driver1051/sampler188/final1reads;0retry/recovery/exhaustion/failure/detailerrors |
| Initial unavailable status |2driver/1sampler polls; final present, not fallback |

ACK cumulative-quota behavior checked in unchanged product reducer: p.retained only increases,
terminal quotas remain checked; no ACKrefund. Actual v1explicit100k rejection is asserted before
same_router_reused is written. Entry/batch/transfer limits and30sdeadline were not changed.

## Original connection reuse and real cleanup proof

Reviewed the baseline-identical `examples/resource_probe_support/product.rs`, `mod.rs`,
C#`ArchiveProductHost.cs`, and product reducer. This is not inferred merely from a later worker:

1. Probe captures original bridge.connection_generation() before requests and asserts equality
   after primary100k. Primary successful owner and remote terminals must agree exactly.
2. After originalcleanup it opens originalsource writable and removes exacthelper, then uses the
   SAME client/bridge/endpoint/host to submit credit.zip with a freshoperation ID.
3. Follow-up start must succeed; observe must return succeeded and cleanup_complete; helper removal
   must succeed again. No-stop facts and processexit0 show these unconditional assertions completed.
4. `same_router_reused=true` is written only after those checks and v1invalid-budget assertion.
   Raw finalstatus/cleanup retain this fact. Follow-up operation
   `2966702d-60cf-4faa-bb24-f4399ff9c105` and worker2296provenance match the ledger.
5. TESThost connects exactly once and has no reconnect path. A disconnected replacement cannot
   transparently supply successful reuse on that host. Numericgeneration is NOTserialized, nor is
   the fullfollow-upterminal retained. These remain documented evidence limits, not invented values.
6. Guard rereads each of3original-operation remote terminals for actualcleanup, checks each source
   writable, drains the host/pipe, and removes only known packagefiles after proof. Raw cleanup
   remote_cleanup_complete/source_released/helper_released/host_exited/pipe_closed/package_removed
   are true; original package is currently absent.

These completed behavior assertions plus baseline provenance and rawfacts are sufficient for this
bounded same-connection reuse criterion. No need to expand logging or repeat100k solely to persist
numericgeneration/fullfollow-upterminal. This does not substitute for untested native/product flows.

| Exact process role | Valid samples /counter failures | Sampled WS /Private bytes |
| --- | --- | --- |
|23352caller |359 /0 |9105408 /2723840 |
|1496C#host,parent23352 |355 /0 |63897600 /32841728 |
|14956actual100kworker,parent1496 |326 /0 |67133440 /70897664 |
|7664defaultrefusalworker,parent1496 |11 /0 |separate from100kpeak |
|2296follow-upworker,parent1496 |3 /0 |separate from100kpeak |

Requested20mssampling/50msdiscovery; actual100kmaximumvalidgap68.23ms. Sampled peaks are not absolute
peaks. Caller/host lifetimes include setup/default/reuse. PrivateBytes is processcommit, not RAM or
reservedbudget; cumulativetransfer is not simultaneously retainedmemory. Firstbatch is not firstpaint;
Debugordinarycache11.4s is not a Release1.5s promise. No performance threshold was silently invented.

## Fresh independent verification and preservation

Python inherited PYTHONDONTWRITEBYTECODE=1. Current tool unittest discover test_*.py:
**42pass/0failure/error/skip,0.597s,exit0**. Independent scratch offlinecheck:
**66checks PASS/exit0**,100000manifest records,1054samplepairs,382unchangedbaselinefiles.
No historical Cargo/C#test counts represented as newly executed in this acceptance.

Spec106documents/14owners, catalog79pages/79rows/78locked, context73implement/70checkentries,
PlatformHost fixture/native selection sourcegate and gitdiff--check all exit0. Existing
native-contracts.md35497bytes>32768contextwarning remains. No source/test fix or scope expansion.
Closing observation:0archivefixture probe/host/worker,0archiveproduct/resourcepipes, fixedhelperabsent.

Acceptance scratch root:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df`.
- `post-status-100k-independent-check.py` and `post-status-100k-independent-results.json`;
  resultSHA256 `fc05da7ba5320b4980b0cd722289a281d8f32d113fddbb6182bbd95ba68dede8`.
- `post-status-100k-independent-python.log` SHA256
  `c4c3ae1bcac6bb8e6599f279481e0892b4dc69c6b3073b323437c0d6082b8f28`.
- `post-status-100k-independent-gates.json` and per-gate logs preserve actualexits.
- New originalattempt SHA256 `09f0eff71e8abfaddf148b6259f7575a06a7b5840ccb82675374ac32814dee16`;
  samples `a055d18f1fef2218f204e1eb261733f704db9ae7b87811f428ebbc5c43a117a0`.
- Old independentFailedattempt and samples rehashed unchanged:
  `ca165d886c837a78da137d0b387a4da0e44c946d37511155dbbd566812756f55` /
  `e8b291144acb16eb9041adc57cb73a06854eb6d20e25c65f4b9f91cc4078e8d3`.

No extra100k/1GiB/largeZIP64, Cargo/dotnet, actualproductchain, formalDesktop/installedhost,
Release/package/sign/install/commit/push, source deletion/recycle or systemconfiguration change.
Original evidence/results stay unchanged; this report alone records independent signoff.

**Accepted and stop. No remaining blocker for this bounded new observation. BroaderENG08/PhaseA/
A–F/UI/native-user/deployment work remains incomplete; no nextphase or feature is started here.**
