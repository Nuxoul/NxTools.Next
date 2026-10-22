# Independent review — bounded Debug archive resource acceptance

Date:2026-10-19. Canonical **Phase A — Engine contracts and safety foundation**.
Reviewed resource contract/evidence/results,Python driver/generator/summary,Rust resource example,
C#resource probe and timeout terminal correction. Product/project-test sources unchanged by review.

## Verdict

**Partial acceptance only.** Normal bounded resource assertions were independently reproduced;
the timeout-to-user-cancel projection repair passes actual session regression. The resource
harness needs one P1 safe-abort correction and two P2 measurement-validity corrections below.
No data corruption,user-file loss or actual low-memory abort residue was demonstrated. Existing
Debug/performance/transport/product-budget limitations are honestly documented and not themselves
new defects. No full ENG08/ENG09/SAFE04,PhaseA/B/A–F/release acceptance.

## P1 — resource-stop path kills host instead of requesting owned cancellation/cleanup

Locations: tests/performance/archive-resources/run.py:144–147 and239–244.
Sampler threshold/deadline calls child.terminate directly. For cancel1gib child is the C#testhost,
so Windows termination bypasses ArchiveResourceProbe.Measure finally at178–183. Job close may
kill its worker,but cannot execute owned partial/helper/output-directory cleanup. Exception/
communicate timeout path similarly terminates root first. Product scenarios also cannot execute
Rust fixture Drop after root termination. This violates the pre-run 'cancel only owned operation,
await actual cleanup,forced result separate' contract at exactly the resource pressure boundary.

Parent source-confirmed and used a fake-child threshold injection against actual Sampler.run:
RESOURCE_THRESHOLD_CALLED_TERMINATE_DIRECTLY=true. No real resource pressure or host termination
was manufactured;actual leftover partial/Job behavior is NOT runtime reproduced. A controlled
small fixture can test this without OOM.

Required: a bounded test-tool cancellation/stop channel to the exact owned operation/host,await
actual source/Job/partial/helper cleanup,then explicitly classified forced fallback only if bounded
cooperation fails. Reap/observe owned descendants and record abort/cleanup completeness. Never
kill unrelated processes or declare root death equal to cleanup. Add deterministic injected
threshold/deadline/failure tests while a tiny owned partial exists. No production API expansion
or memory-pressure experiment is needed.

## P2 — missing required PID samples can still yield green resource verdict

Locations: run.py:81–89,129–130,149–156,249–254;
ArchiveResourceProbe.cs:48–56.
Failed process open/image lookup silently omits required root/child;failed memory query silently
skips. Driver success only checks failure string,not required role coverage/valid sample count.
C#Count increments before WorkingSet/Private reads and suppresses InvalidOperationException,
so its count can include failed reads. Transient child startup misses may legitimately be retried,
but persistent absence cannot certify a resource result.

Controlled fake-child/tool-only injection made every Sampler.add return no observation,while
operation stub returned exit0. Actual driver completed with failure:null,processes:[] and no
assertion failure: TOOL_VERDICT_ACCEPTED_WITH_ZERO_REQUIRED_SAMPLES=true. This proves verdict gap,
not archive correctness or actual counter API failure on this workstation. Real independent
normal runs below did contain valid PID samples and remain usable at their documented scope.

Required: retain acquisition/query errors and successful/failed counts;bind required roles to
expected exact PID/image and classify resource measurement INCOMPLETE/FAILED if required live
process has no valid samples. A short missed child must be explicitly unavailable,not invented0.
C#increment successful sample count after both values read. Separate operation success from
measurement validity;test deterministic denied/zero-sample/transient-retry cases without real
privilege changes. Do not reject harmless conhost or silently accept missing actualworker.

## P2 — C#listing operation times include setup before dispatch

Location: ArchiveResourceProbe.cs:83,92–96,125.
clock starts before manifest opening,caller sampler initialization,helper copy and runner creation.
first_batch_ms/full_terminal_ms are therefore setup-inclusive,contrary to contract's immediately
before Start origin. Cancel-to-exit/cleanup differences use same origin and are unaffected.

Source-confirmed only;no injected setup delay or retiming runtime test performed. Values should
be relabeled setup-inclusive if retained historically. Move/reset operation clock at dispatch and
record setup separately;add a controlled setup-delay test proving operation timing does not grow.
Do not rewrite historical timings or call firstbatch firstpaint. Engine open and directworker
request clocks have distinct valid origins;not all timestamps are affected.

One bounded measurement read-only reviewer proposed these findings;parent independently read
call sites and ran the two fake-child injections. No repeated review. Probe initially had an
unstarted-thread join error and later hit real build guard;both scratch failures retained,then
corrected fake-only environment/thread execution produced the controlling results. No product
or committed harness code was patched for the injections.

## Independent resource rerun — one run per scenario,no best-score repetitions

Debug builds only;no Release,system cache flush or synthetic pressure. New owned scratch root:
archive-resources-c52767d9-67ec-4f9a-a99a-fe8216d46f23. Driver logs/JSONL/rawresults and independent
manifest retained there. Script resource-independent-run.ps1;ledger19:33:26->19:37:03(+08:00),
all8scenarios exit0/failure:null. Available precheck~17.3GiB,commit headroom~19.4GiB;exact system
before/after and PID/path memory rows in each JSON. Sampling20ms requested,actual gaps recorded;
one internal host gap250ms,so no exact peak guarantee. Per-process data,not system deltas.

Independent fixture hashes match prior small100k ZIP/manifest and1GiB ZIP/payload. Exactly100000
members:100directories,100Unicodefiles,9081emptyfiles,1362285payloadbytes;1GiBStored exact
1073741824payload without rawduplicate. No product writer oracle or compression bomb.

| Scenario | Independently observed |
| --- | --- |
| Engine100k |100000exact names/kinds/sizes/packed/ordinal/session;open2210.644ms;lower entry+metadata budgets ResourceLimit;sampledWS~60.55MiB/private~67.00MiB |
| Direct legacy worker100k |100000/50000batches;first2149.146ms/terminal3324.144ms;WS~63.78MiB/private~67.61MiB;no nativeJob/sourcelease/snapshotID claim |
| Original runner100k |46710then stdout_total_limit;first2398ms/terminal3391ms SETUP-INCLUSIVE;cleanuptrue/slot0/sourcewritable/helperremoved;not full100ktransport |
|1GiB data cancel,unheld |partial65536bytes;cancel at69290ms;exit observation+31ms;actualcleanup+44ms;exit0;no owned partial/sourcelease/slot left |
|1GiB data cancel,hold |partial65536bytes;exit+29ms;predisposal+39ms;finalcleanup+65ms;Jobempty+sourceheld+slotheld before release |
| Product100k |typed ResourceLimit,0entries/0batches,cleanuptrue,458.585ms;explicit100k escalation InvalidRequest;originalconnection/reuse |
| Smallcredit cancel |cleanup upper56.821ms,sameconnection/source/helperrelease/reuse |
| Smallfullqueue Drop |cleanup upper49.957ms,sameconnection/source/helperrelease/reuse |

WS/private sampled for entire probe lifetime,including bounded manifest/budget/reuse work,not
isolated pure-open phase peak. PrivateBytes is processcommit. Short worker samples can be1–4points;
those rows are not universal codec RSS evidence. Cancel receive/accept/observed internal timing
not exposed and NOT measured. Exit observations are polling upper bounds;exit0/cleanuptrue is
consistent with cooperation,not guaranteed arbitrarynative-call interruption. Hold is deliberate
and not unheld latency. No1GiBproductcredit repeat;firstpaint noUI/unmeasured. Debug2210ms cannot
judge Release1.5s target. Product4096/4MiB refusal is correct,not100kproductsupport.

## Independent stable regressions and current global gates

Two launch attempts stopped BEFORE any test due unrelated Cargo lanes;records retained. Bounded
wait then serialized commands with jobs1/incremental0/RUSTFLAGS codegen-units256 each invocation.
Observed external wallhaven and separate Desktop tests were not killed. Final regression ledger
19:41:24->19:52:42;tests started19:42:55. No duplicate owned build or timeout retry.

| Gate | Actual independent result |
| --- | --- |
| Archive isolated/offline/no-fail-fast/serial |138pass/0fail/0ignored |
| Protocol / IPC --lib |51/29pass,0fail/ignored;Protocol count reflects current concurrent repo state,not an extra resource-slice test |
| Archive/Protocol/IPC strictClippy tests+examples,default+isolated |all exit0 |
| workspace check --tests --offline |CURRENT PASS exit0;prior implementer Desktop compile failures historical,not reproduced current |
| globalfmt --all --check |CURRENT FAIL exit1;unrelated Desktop/Clipboard and Protocol formatting in captured log;not modified here |
| extensions archive_import / clipboard archive_ |7/5pass,0fail/ignored |
| same-feature Debugworker+probe build |exit0 |
| C#test/product Debugbuild |0warnings/0errors |
| Exact original127 minus unique largeZIP64 plus timeout |selected127/run127/pass127/skip0/fail0/not_run0;LIST/RUN/PASSsame |
| timeout actual session/runner regression |RUN/PASS;failed/io_failure+cleanuptrue,not userCancelled |
| large4GiB+17ZIP64 |RUN0;historical independent pass only |
| spec/catalog/context/selector |PASS106docs/14owners;79/79/78;61implement/58check;selectorPASS |
| gitdiff--check/PythonAST/scopedwhitespace-conflict |PASS |

Current workspace check is compilation,not workspace tests. No Desktop suite rerun. Globalfmt
failure remains a real out-of-scope gate and was not hidden or fixed. Timeout correction in
ArchiveReadSession prioritizes original timeout below cleanup failure;its behavioral regression
passes and normal cancellation suites remain green. No budget/deadline/wire/codec/publication
change. That repair can be accepted independently of measurement-tool corrections above.

## Cleanup and limits

No observed task Cargo/rustc/worker/testhost/resourceprobe/7z,0testpipes,0fixedhelper or product
package roots after runs. Self-owned100k.zip/1gib.zip/credit.zip were hash-verified then removed;
no output-*dirs remained. Rawmanifest/JSONL/results/logs retained. Fake-child injected JSON is in
separate resource-tool-validity-* roots,NOT merged with genuine resource runs. No real termination
or memory/disk threshold was exercised by that probe. Unrelateddirty changes preserved.

No actual Desktop/installedPlatformHost launch,Release/package/sign/install/commit/push,userarchive,
real recycle/permanent sourcedelete,registry/service/drive-map/systemcache/pagefile changes.
Remaining PhaseA/ENG08/ENG09 limits remain:product100kfiniteescalation/transport,broadformats/
dictionary/UNC/longpath/million/RSS,nativecancelreceive timestamps,1GiBcreditwait,UIfirstpaint,
nativeRecycleidentity/directorycoverage/signeddeployment. No fullphase/firstrelease claim.
