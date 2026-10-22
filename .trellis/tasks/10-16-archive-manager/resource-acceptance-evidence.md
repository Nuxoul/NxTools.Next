# Phase A — bounded resource acceptance evidence

Date:2026-10-19. Canonical **Phase A — Engine contracts and safety foundation**.
**Partial ENG08/ENG09/SAFE04 evidence, not full scenario/Phase A/A–F acceptance.**
Pre-run [resource contract](resource-acceptance-contract.md); machine-readable
[all21attempts/environment/exact names](resource-acceptance-results.json). Historical independent
reports remain unchanged. Pre-slice checkpoint copied byte-for-byte to
[historical checkpoint](research/phase-a-checkpoint-before-resource-slice.md).

## Outcome and boundaries

- Default public engine and actual legacy private worker each enumerate EXACT100000 entries.
  Streamed independent manifest checks ordinal/name/type/size/packed size; engine also checks
  every EntryId.session against snapshot session. Legacy private frames expose no snapshot ID.
- Original C# runner/Job does NOT complete100k transport: unchanged cumulative8MiB stdout stops
  at46710entries with stdout_total_limit, no successful truncation. No public transport escalation.
- Current product4096entries/4MiB metadata/4MiB retention rejects100k input as ResourceLimit,
  entry_count0/batch_count0, cleanuptrue. Explicit max_entries100000 -> InvalidRequest before
  dispatch; there is NO product finite escalation above4096. Engine with_limits is separate.
-1GiB actual Stored ZIP data-stage cancellation: process exit observed28ms, actualcleanup42ms
  without injected hold; held-cleanup variant30ms/66ms. Actual partial data65536/131072bytes
  proves native writes while decoding, not a sleep. Source/slot remain held at drained Job
  pre-disposal barrier and release only after actualcleanup. ExitCode0, no live hidden worker.
- Small independent5-entry credit/queue cancellations separately measured48–59ms cleanup, original
  connection retained and subsequent actual worker admission succeeds. These are NOT1GiB credit
  measurements.1GiB product open hit30s before first batch; no credit wait was reached.
- One concrete production defect found and minimally fixed: original runner timeout was relabeled
  user Cancelled by the worker's later cancellation cleanup frame. Real session/runner red→green
  regression preserves failed/io_failure (existing wire has no timeout enum); cleanupfalse still
  wins. No deadline/default/codec/dependency/writer/ZIP64/output-publication change.

## Environment, provenance and budgets

Windows11Pro10.0.22631, i5-12400F6cores/12logical, physical34095611904bytes. Initial observed
available16251736064bytes, committed29534126080/limit55570448384, headroom26036322304.
C:NTFS free27698446336bytes;D:NTFS free44719546368bytes. Exact per-run available/headroom plus
saved environment in JSON. Rust1.96.0(ac68faa20),Cargo1.96.0(30a34c682),.NET8.0.424,CPython3.13.2.
All THIS task's builds were Debug/unoptimized+debuginfo; every Cargo inherited jobs1,
incremental0,`RUSTFLAGS=-C codegen-units=256`,serial,offline. No Release/package operation here.

Locked codec versions unchanged:zip8.6.0,flate2=1.1.9,zlib-rs,bzip2=0.6.1,lzma-rust2=0.21.0,
zstd-safe=7.2.4,sevenz-rust2=0.23.0,unrar_sys=0.5.8. Cargo.lock SHA256 in environment JSON.
Stock7Zip26.00 oracle SHA2564bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f;
no download/install/PATH change. Unity persistent .NET/Burst/compiler observed, not stopped.
Other project's `D:\Code\wallhaven` Cargo builds/tests caused preflight stops and bounded waits;
no competing THIS-task build/measurement was started during those observed builds. Later global
checks encountered live unrelated Clipboard/Desktop source edits; original failures retained.

Independent `tests/performance/archive-resources/fixtures.py` uses CPython zipfile, not product
writer.100k ZIP12161983bytes,100directories,100Unicode files,9081empty files,total1362285payload
bytes. JSONL manifest8470828bytes,streamed no duplicate100k Vec. SHA256 ZIP
34a9f9fe2e1d669b825d3557e523cadf00aeabf1ebfe187c99fcd4475cda956d;
manifest4fd4bdca3f5ceb0f388206c1c2a12cf827be97856313f3be85c5efb31e2768dc.
1GiB ZIP1073741948bytes,exact1073741824payload,Stored ratio1;single1MiB deterministic block,
no separate raw1GiB copy. ZIP SHA256b7f213cbbd6f34d993b08f8baad8823adb07c191e182e77b60d32ab251bcda71;
payload2c06ade942ee3f17a048dd1064b2fab046a4bb95386d8bb41b68dc6711ac2af3.
Small credit.zip has5×3byte entries; independently produced, not a compressed bomb.

Engine defaults1M/256MiB metadata/512MiB work accepted100k; explicit4096entry and4MiB metadata
variants typed ResourceLimit. Runner used100k/256MiB metadata/64MiB work,Job384MiB
(work+metadata+64MiB overhead). Product uses its unchanged4096/4MiB/work64MiB,Job132MiB.
These model/Job ceilings are NOT observed memory consumption. No escalation/default increase.

## Actual measurements — report every ordinary repetition

All values **Debug**, ordinary reads/repeats, possibly cache-warm but NOT strict cold/warm.
No cache clearing/system pressure. Time measured monotonically; first batch is NOT first paint.

| Scenario / attempt | Operation result / timing | Peak WorkingSet / PrivateBytes MiB |
| --- | --- | --- |
| Engine correctness |100000exact;fullopen2248.739ms;lower budgets ResourceLimit |caller60.90/67.03 |
| Engine repeat |100000exact;fullopen2268.197ms;same assertions |caller60.84/67.23 |
| Actual legacy worker correctness |100000/50000batches;first2264.276ms;terminal3472.296ms |worker63.79/67.59 |
| Actual legacy worker repeat |100000/50000;first2271.717ms;terminal3490.191ms |worker63.81/67.64 |
| Original runner successful TOOL-repair operation |46710then stdout_total_limit;first2456ms;cleanup3485ms;exit1 |internal host48.93/24.60;worker63.88/67.58 |
| Product100k correctness |ResourceLimit,0batches,cleanup354.563ms |Rust7.05/1.06;C#37.02/11.46;worker7.79/1.39 |
| Product100k repeat |ResourceLimit,0batches,cleanup383.600ms;postcleanup reuse succeeded |Rust7.18/1.08;C#39.44/12.14;worker8.32/1.48 |
|1GiB data cancel,no hold |data65536;local at73664ms;exit+28ms;actualcleanup+42ms;exit0 |C#46.84/22.11;worker8.66/1.92 |
|1GiB data cancel,hold variant |data131072;local72755ms;exit+30ms;predisposal+40ms;final+66ms;exit0 |C#46.18/20.68;worker8.65/1.92 |
| Small credit correctness/repeat |cleanup+56.966/+48.374ms;samepipe;actualreuse |fullPID metrics JSON |
| Small full32queue Drop correctness/repeat |cleanup+58.591/+56.093ms;33rdBusy;samepipe;actualreuse |fullPID metrics JSON |
| FINAL fixed C#100k/credit/queue directed |367.520ms ResourceLimit;cancelcleanup54.941/53.221ms;reuse |separate tool-repair JSON,not statistical repeats |

1GiB extraction spent~72–74s before first actual native data write. Source revision currently
hashes full input at multiple ownership/verification stages; this Debug observation is not a
Release target miss or demonstrated algorithm defect. No profiling/optimization was authorized
from Debug timing alone.≤1.5s100k and≤200mscooperation remain TARGETS, not presets. These Debug
100kopen values exceed1.5s, with no Release conclusion. No UI: first paint NOT MEASURED.

Sampling: Windows per-process WorkingSet/PrivateBytes, exact PIDs/images/parent chain in JSON;
C# runner samples same native process handle. Requested20ms,actual max interval recorded;
observed~30mstypical,max~53msworker/~133mshost in long runs. External child discovery50ms can
miss short processes. Lifetimememory high-water counters supplement phase samples,not substitute.
External runner sampler stopped on a conhost classification bug; its FAILED sampler record stays,
only internal same-handle worker/host samples above are used. Sampling may miss subinterval peaks.
Machine memory deltas are never worker RSS. Host sampler/streamedmanifest overhead included.

No stock worker cancel-received timestamp/pause ACK is exposed. Cancelled business terminal,
private frame,process exit and actualcleanup are not interchangeable. Native data-cancel worker
Cancelled frame was not observed (runner pump stops after cancellation);receive/observe latency
is explicitly UNMEASURED. Exit0 before drained cleanup is compatible with cooperative termination,
not evidence that every native call is interruptible. Existing forced/hung-worker fixtures are
separate evidence;no memory-bomb/unknown codec pressure was run. Hold variant deliberately kept
source+slot after Job/process/pumps drained; injected hold is not unheld response performance.

## Defect and preserved failed attempts

Actual1GiB product-credit attempt:source hashing still running before firstbatch;automatic30s
runner stop,terminal at30070ms reported cancelled/cleanuptrue although no caller Cancel was sent.
This failed expected firstbatch barrier (exit101),not a passed1GiB credit/cancel test. Saved actual
terminal and all PIDs. Source/helper/process cleanup completed;pre-guard package residue was
manually cleaned only after original build hash verification. No second large product attempt.

Correct seam:`ArchiveProductSessionTests.TimeoutIsNotUserCancellation`:shared production session
->original runner/real30sdeadline->controllednegative worker emits failureCancelled on shutdown.
First LIST1/RUN0;RED selected1/run1/pass0/skip0/fail1/not_run0 exit1,exact cancelled misclassification.
A second old-code red occurred because editor unseen-line protection had prevented the intended
change; retained,NOT green. Then minimal OnRunnerEvent gives original timeout cause precedence
below cleanupfalse;GREEN1/1 exit0;finalfull127also includes it. No fake worker performance claim.
Current wire maps timeout to failed/io_failure; dedicated timeout enum remains absent, unchanged.
Rust helper guard now owns testhost/server/package across panic;controlledtool-guard-failure
exit101 intentionally verifies ownedpackage gone,not productPASS. Final small realchain confirms.

Other retained TOOL failures:early C#sampler MainModule null/suspended startup;external image
query/conhost discovery;helper residue after prematurely terminated testhost;first1GiBhold
InvalidCast(Job indexed before outputbridge). Fixed TESTtools only;failed attempts stay in21record
JSON. InlinePowerShell quoting/duplicateenvironmentkey/JSONarraynesting and source-edit/ASTerrors
were pre-execution tooling failures,not resource passes. First probe strictClippy missingelse lint
was fixed without suppression. No timing-threshold functional tests or repeated best-score hunting.

## Final stable verification, commands and limits

Exact commands/start/end/exit ledger is embedded in results JSON and scratch
resource-verification-ledger.txt. C#apphost was built before Rust realchain;actualworker and
Cargo relink commands never intentionally overlapped. Every build used required memory env.

| Final stable gate | Actual result |
| --- | --- |
| `cargo test -p nxtools-archive --features isolated-codecs --offline --no-fail-fast -- --test-threads=1` |138pass/0fail/0ignored,26groups/24nonempty;cancel10/consumer5/realchain11 INCLUDED |
| Protocol/IPC `--lib --offline` |50/29pass,0fail/ignored |
| Archive/Protocol/IPC strict Clippy `--tests --examples` default+isolated |exit0 |
| archive scoped fmt |exit0 |
| Extensions archive_import / Clipboard --lib archive_ |7/7 and5/5,0fail/ignored;currentClipboard43filtered |
| samefeature worker/probe Debug build |exit0 |
| C#build |0warnings/0errors;exit0 |
| Original127 minus ONLY largeZIP64 plus newtimeout |LIST127/RUN0;RUN127/PASS127/SKIP0/FAIL0/not_run0;names/order exact |
| Actual4GiB+17largeZIP64 |NOT RUN here;historical independent final-code pass retained |
| FINAL `cargo fmt --all -- --check` |FAIL1:unrelated concurrent Desktop/Clipboard formatting;no edits by thisslice |
| FINAL `cargo check --workspace --tests --offline` |FAIL101:unrelated Desktop GPUI test imports/KeyDownEvent fields;earlier transient Clipboard errors also retained |

Global gates were green earlier thissession before unrelated edits;do NOT claim they are currently
green. Latest error filenames/lines are in resource-closeout-workspace-current.log and fmt log.
No fullworkspace tests/Desktop suite or Release/native-user acceptance. No optional broad review
loop;one bounded read-only harness exploration only,not an independent acceptance run.

## Evidence retention / cleanup / authorization

Session scratch root:`archive-resources-fa3927c9-28d7-4e0a-aaf2-fa046fb57263` under PI_SCRATCH_DIR.
Retained:JSONL independentmanifest,fixture-summary,allattemptJSON,sampledJSONL,environment,cleanup,
originalterminal;parent resource-*.log/ps1/selection/ledger/red/green files. Results JSON persists
compact raw metadata/commands/PIDmetrics/exactnames in task;generator reproduces fullmanifest.
Large100k.zip/1gib.zip removed after recordedhash and no-live-ownedprocess checks;no ownedoutput
dirs,resourcepackage roots,fixedtest/producthelpers or archive testpipes remain atcleanup18:49:22.
Normal/failure operations observed actualprocess/pumps/Job drainage;sourceexclusivewrite/helper
removal aftercleanup. Credit/queue product sameconnection readmission verifies reservationreuse.
Originalsource/useroutputs untouched. Independent4report SHA256s rechecked unchanged.

No formal Desktop/installedPlatformHost launch,Release/package/sign/install,commit/push,UI/FIFO,
productwrite/deploymentmapping,registry/service/drivechange,cacheflush/pagefile/OOMpressure,
userarchive/recycle/permanent sourcedelete. Native recycle remains
**native_protection_unproven/NotAttempted**. RemainingA:nativeidentity/directorycoverage,
signeddeployment,broadformats/resources/UNC/longpaths/millionentries,UI/nativeuser acceptance.
Stop at thisslice;no PhaseB or PhaseA/A–Fcompletion. Final document/hygiene gate ledger follows.

## Final static/document gates and exact external blockers

Actual final requested gates:spec-contractsPASS106documents/14owners;GPUIcatalogPASS79pages/
79rows/78locked;taskvalidatePASS61implement/58check;PlatformHostfixtureselectionsourcegatePASS.
All exit0. Initialtaskvalidation warned archive reference note exceeded32KiB;thisslice's new
sourcefollow-up moved to research/resource-boundaries.md with shortindexlink,then validation
rerun without that warning. `git diff --check` exit0;PythonAST andscopedconflict/temporaryprobe/
whitespace/preservation checks recorded in resource-final-hygiene.json. Staticchecks are NOT
native/UI/resource acceptance. Fullrawsamples/manifest remain scratch,not shipped binary assets.

Latest workspacecheck failures (unrelated files,not modified here):
`rust/apps/nxtools-desktop/src/clipboard_worker/thumbnails.rs` missing `image` import/crate,
and `rust/apps/nxtools-desktop/src/ui/efficiency_tests.rs:67` uses nonexistent
`gpui::KeyDownEvent.input_preference` (available field prefer_character_input). Eighterrors,
exit101. Latestglobalfmt includes those Desktopfiles/ui/mod.rs andClipboardlib.rs/query.rs.
Preserved earliertransientClipboardquerycompileerrors andallglobalgatefailures;no further
retestloop or unrelatedfix. These remain genuine finalglobalverificationblockers.

Finalscoped C#productiondiff is only timeoutcausepriority in ArchiveReadSession.OnRunnerEvent;
original runner/Job/source/output/recycle/codec/defaults/wire/dependency files unchanged bythisslice.
The controllednegativeworker proves classification order,not actualcodecperformance. Actual1GiB
data cancellation measurements preceded that C#projection-onlyfix;finalactualsmallchain and
full127verify cancellation/cleanup afterfix. No1GiBperformance rerun or actuallargeZIP64 rerun.
Source/native cancellation receive/observe timestamp remains UNMEASURED;1GiBcredit andfirstpaint
remain NOTPASSED/NOTMEASURED. Evidence is partial,not a waiver of ENG08/ENG09/SAFE04.
