# Independent acceptance — resource-tool safe stop, samples and timing

Date:2026-10-19. Canonical **Phase A — Engine contracts and safety foundation**.
Scope: the three findings in [original resource review](resource-acceptance-review.md), plus the
new tool implementation's evidence-retention path. Product/project-test source unchanged by
acceptance. No100k/1GiB/largeZIP64performance sample or formal product launch was executed.

## Verdict

**Original P1 safe stop and both P2 sample/timing defects pass bounded independent acceptance.**
One additional P2 remains in summarization of malformed retained output, independently reproduced
below. Thus the complete tool slice is not yet unconditionally accepted. No product corruption or
unsafe source deletion was shown. No PhaseA/ENG08/ENG09/A–F/release completion claim.

The implementer's historical configured132collection exited early without SUMMARY. This acceptance
ran ONE current exact132collection to normal completion:132/132,zero skips/fails. That establishes
current full regression evidence;it does not explain/erase the historical process exit or prove
its cause was fixed. No broad retry-to-green loop was used.

## P2 — retained malformed RESOURCE line prevents all-attempt summary

Location: `tests/performance/archive-resources/summarize.py:21`.
collect unconditionally json.loads every line beginning RESOURCE. The driver explicitly handles
malformed probe output by requesting driver_exception stop and retaining the original stdout;
test_tools.py:241–246 demonstrates RESOURCE not-json as supported failure input. But summarizing
that valid attempt JSON reparses its stdout without error capture and raises JSONDecodeError.
No output summary is produced,and later well-formed attempts are not represented in that view.
Original attempt files remain intact;this is not deletion of raw evidence.

Parent independently reproduced with two small synthetic attempt JSONs and valid synthetic
selection/log/environment inputs, invoking ACTUAL summarize.collect. First failed attempt had
stdout='RESOURCE not-json\n',operationStopped/measurementIncomplete/cleanupConfirmed and fixed
JSONDecodeError failure;second had valid RESOURCE JSON. Observed:
```text
SUMMARY_PRODUCED=false ERROR=JSONDecodeError FAILED_ATTEMPT_AND_LATER_RECORD_NOT_SUMMARIZED=true
ORIGINAL_ATTEMPT_BYTES_PRESERVED=true
```
Scratch resource-summary-independent-probe.py/log records this. No process or archive operation.
One bounded read-only reviewer proposed this source finding;parent confirmed and ran the probe.
No other concrete finding in its safe-stop/grace/verified-force/sampling/guard review;no repeated
review. Its no-tests status is separate from parent runtime checks.

Required repair: bounded per-line parse result/error retention without discarding the attempt or
later records;invalid payload must never become a successful measurement. Preserve raw stdout,
original failure/stop/cleanup and valid events where possible,mark summary parse completeness
explicitly and continue bounded collection. Add malformed/truncated RESOURCE among valid lines and
multiple attempts regression. Do not silently skip broken records,label them Legacy/PASS,or modify
original attempt bytes. No real smoke/large/full codec rerun is needed for this small correction.

## Original findings — independent assessment

### Safe stop

Read Python control/driver/sampler, C#Control/Measure finally, Rust resource guard and tests.
First-wins runUUIDstop routes to original runner/control/product operation;parent finite grace
begins on internal and external stop. Exact verified owned process force is explicit fallback,
never cleanup=true by root death. Driver keeps bounded pipe reads/records and native owners drain.
Returned failures/threshold/deadline/initialbarrier/ownerexit/control races covered by unit tests;
controlled separate uncooperative small process remains distinct from actual codec cooperation.
No arbitrary forever-blocked Win32call interruption is claimed or independently tested.

### Valid samples

Mandatory roles/PID/image/parent provenance and zero-valid records now survive acquisition/image/
counter failures. Missing samples -> Incomplete,not zeroMiB/PASS. C#commits both counters then
Count/baseline/peaks,so partial reads cannot increase success count. Validity is separate from
operation and cleanup. Transient retry/errors and unknown exit observation are retained. Python
actual24unique tests and C#counter tests pass;no real access denial/system pressure manufactured.

### Dispatch timing

C#DispatchClock now separates setup and resets immediately before original runner.Start. Its
virtual clock test varies setup10vs9000 while operation stays7;actual new smoke says
runner_dispatch. Historical summary labels Legacy/setup-inclusive without recalculation. First
batch remains not firstpaint;sampled memory is not absolute peak/privatebytes not machine RAM.

## Independent small actual sampling smoke

Only ONE external smoke this acceptance,256KiBStored synthetic source. New owned root
archive-resources-30a882ae-2ecf-4fb6-9f54-4c88679b2fdc. Pythondriver ->C#tool ->original runner/
Job/actualworker,real65,536bytepartial held before cancellation. CLI exit1 EXPECTED,then independent
assertion script exit0 confirmed all schema2facts:
- resource_verdictStopped;operationStopped;reason injected_partial_threshold;cancel accepted once;
  forcedfalse/failurenull,not resourcePASS.
- measurementComplete;exact hostPID53460:15validexternal samples;workerPID43992:6;identityverified,
  both alive_at_endfalse. Internal caller6/worker4 valid counter pairs. Sampled-not-absolute caveat.
- cleanupConfirmed:actual native terminal/workerexit/Job+pumpsdrained,ActiveCount0,sourcewritable,
  helperremoved,partialabsent,emptyoutputdirectoryremoved.
- setup/dispatch domains distinct;operationterminal~299.356ms,cancel~251.849ms,difference~47.507ms.
  This held tinytool fixture is NOT universal native cancellation/performance guarantee.

No second sampling smoke or100k/1GiB/4GiBrepeat. Smallactualstop fixture in fullC#collection is a
functional regression,not additional sampled resource measurement. FullJSON/JSONL/status/manifest
retained;small/credit ZIPs hashverified then removed after confirmedcleanup/no liveownedprocess.

## Independent verification — actual final source

Python unittest discover:24/24,exit0. Foreground serial script
resource-tool-independent-verify.ps1/ledger/logs in scratch. START21:37:09;bounded73sbuild-lane wait;
firstbuild21:38:22,complete21:42:38(+08:00). Externalwallhavenbuild untouched,no duplicateCargo.
EveryCargo set jobs1/incremental0/RUSTFLAGS=-C codegen-units=256 and checked machine-wide lane.
C#testapphost built first;actualworker rebuilt before smoke/C#execution. All steps exit0 except
expectedStoppedCLIexit1,whose independent contract assertion passed.

| Gate | Actual independent result |
| --- | --- |
| Python unique tool/summary tests |24pass,exit0 |
| Rustresource example tests |11pass/0fail/0ignored |
| FullArchive isolated/offline/no-fail-fast/serial |138pass/0fail/0ignored |
| Protocol/IPC --lib |51/29pass,0fail/ignored |
| Archive/Protocol/IPC strictClippy tests+examples default+isolated |both exit0 |
| Globalfmt --all --check |CURRENT exit0 |
| Workspacecheck --tests --offline |CURRENT exit0;compilation NOT workspace tests |
| Extensions --test archive_import/Clipboardarchive_ |7/5pass,0fail/ignored |
| DebugC#product/testbuild |0warnings/0errors |
| SamefeatureDebugworker/probebuild |exit0 |
| Exactoriginalnonlarge127+new5LIST |selected132/run0/pass0/skip0/fail0/not_run132 |
| SAMEcurrentexecution |selected132/run132/pass132/skip0/fail0/not_run0 |
| LIST/RUN/PASSnames |0differences |
| LargeZIP64/100k/1GiBrepeats |RUN0thisacceptance;historicalonly |
| Spec/catalog/context/selector |PASS106docs/14owners;79/79/78;63/60;selectorPASS |
| gitdiff--check |exit0 |

Contextvalidation still warns unrelated platformhost/native-contracts.md35497bytes exceeds32768;
not rewritten here. Applicable tool-specific contracts read directly. Globalgates nowpass;older
failures remain historical. Desktoptests not rerun;no fullworkspace/unrestrictedC#claim.
Existing7-Zip26.00hashmatched4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f;
no install/download/PATHchange. Exactfixturepaths supplied,zero skipvs implementer's configfailure.

## Preservation, cleanup and limits

Historical resource review/evidence/results hashes observed unchanged:
BA8AEFBD75AB516631F49CDDBEBAC7D289050266194149CED738895691CC54CA;
6888B1BB3C4035D124BDCC2D546539743C4BA82FAB3817D5DF7D3068FD138DF9;
FE53DEA74EF68C1F9AFEEA1AD728A625E2C3BB3DCCF53086FCD4DD5F7C1247CA.
No task sourcefixed byacceptance;unrelated dirtychanges preserved. Scratchtests/logs retained.
Observed0Cargo/rustc/archiveworker/testhost/7z,0testpipes,0fixedhelper/productpackage roots.
No unknowncleanup,manual residue sweep,realresourcepressure/cacheflush/pagefilechange.

No formalDesktop/installedPlatformHost,Release/package/sign/install/commit/push,userarchive,
nativeRecycle/permanentsourcedelete,registry/service/drivemapping/UI/FIFO/productwrite.
Original three corrections are verified;fix the one summaryP2 before fulltoolclosure. Product100k
finiteescalation,largeformat/RSS/resources/longpath/UNC/nativeuser,1GiBcredit,cancelreceivedtiming,
firstpaint,nativeRecycleidentity/directorycoverage/signeddeployment/fullPhaseA remainunfinished.
