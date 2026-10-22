# Resource-tool correction evidence

Date:2026-10-19. Canonical **Phase A — Engine contracts and safety foundation**.
Tool-only correction of the independent resource review's P1 safe stop, P2 required-sample
validity and P2 setup-inclusive C# operation timing. [Repair contract](resource-tool-repair-contract.md),
[new machine-readable results](resource-tool-repair-results.json). Historical review, resource
results/evidence/contracts and independent reports are unchanged. No codec/budget/product timeout/
wire/native publication/recycle change. **Not full tool-integration acceptance, ENG08/ENG09,
Phase A or A–F acceptance**: one exact C# collection anomaly remains below.

## Root causes and owners

1. Python Sampler threshold/deadline and parent exception directly terminated the probe, bypassing
   C# finally/Rust Drop. New first-wins run-UUID control file requests original engine/product/
   runner cancellation; actual process/pump/Job/source/helper/partial/slot owners drain before
   cleanup confirmation. Parent permits explicit exact-identity force only after finite cooperation,
   recording reason/targets/native errors and cleanup Unknown. No name kills/second Job runtime.
   Rust drops its original pipe runtime BEFORE its synchronous fixture guard waits for the host;
   aborting only the listener is not proof that detached connection tasks closed. Guard removes
   only exact known package files after native terminal/source/host proof, never unknown evidence.
2. Missing process opens/images/counter reads were omitted and exit0 was used as resource verdict.
   Required roles now bind PID/image/original parent/operation provenance, retain acquisition/image/
   counter success/failure/native codes and zero-success rows. Null means unavailable, not0MiB.
   C# commits counter pair/count/baseline/peaks only after BOTH reads succeed. Operation, measurement,
   stop and cleanup are independent schema2 fields; Incomplete/Forced/Legacy can never resource PASS.
3. C# clock started before helper copy/manifest/sampler/runner setup. Setup is now separate;
   operation starts immediately before runner.Start. First batch/full terminal/cancel/exit/cleanup
   share runner_dispatch origin; cancel differences remain same-domain. First batch is not first
   paint. Old runner/cancel measurements are marked setup-inclusive in new summary views only;
   historical timing values are not subtracted/recomputed/replaced.

One joinable Rust engine monitor uses bounded file reads and original OperationControl; Python
polls only its inherited pipes without unattended reader threads. Sample/process/error counts,
control size, startup retries, stop dispatch and waits are bounded. Wrong-run/duplicate/late stop
cannot dispatch again. Source/helper/partial ownership starts before partial initialization can
fail. After stop, no later product reuse/escalation or measurement is started.

## Behavioral red -> green (all preserved)

Logs are under session scratch `bf4f40e2-11aa-482c-90a4-a0d1b194908f`; compact controlling logs
and command/exit/count/PID data are embedded in the new results JSON. Project tests do not depend
on old scratch probes.

| Symptom / actual seam | Red | Green |
| --- | --- | --- |
| Old Python Sampler.run threshold directly terminates fake owned root; old run accepts exit0+processes=[] | resource-tool-python-red.log:2tests/2fail,exit1 | final unique Python24pass,exit0 |
| C# probe init failure before old finally loses owned status | csharp-resource-tool/red-control-run.log:1run/0pass/1fail,exit1 | new initialization fixture PASS |
| C# private counter fails after working read; Count advanced | red-counter-run.log:1run/0pass/1fail,exit1 | counter-pair/failed peaks/null unavailable fixture PASS |
| C# controlled virtual setup advance leaks into operation stamps | red-time-run.log:1run/0pass/1fail,exit1 | setup10 vs9000; operation7 vs7, no wallclock threshold |
| Summary old fixed127 / missing Legacy label | resource-tool-summary-red.log:3tests/2fail/1pass,exit1 | dynamic exact counts/Legacy original-bytes tests PASS |
| Stopped Rust probe stranded before initial go barrier | resource-tool-startup-red.log:1fail,exit1 | stop written, go releases setup only, probe checks before dispatch |
| Summary interrupted regression log aborts and omits output view | resource-tool-summary-interrupted-red.log:1fail,exit1 | actual counts and missing-SUMMARY Incomplete preserved |
| Final native exit query throws, losing record/handle cleanup | resource-tool-exit-query-red.log:1error,exit1 | Unknown exit/error row retained, handle closed, no resource green |

C# fake/control/time final selected4/run4/pass4/skip0/fail0; five new fixture names LIST5/RUN0.
One intermediate CS0649 compile failure is retained but is NOT behavioral red. Final C# Debug
build exit0,0warnings/0errors. Rust example11tests pass includes original-control paused cancel,
monitor join, EOF-before-wait-before-force, noncooperator classification, init/unknown-file retention,
missing remote terminal, wrong/late/duplicate stop and bounded control. Rust-specific original Drop
baseline was saved with reproduction instructions, but no separate compiled old-Rust red was run;
P1's old actual Python boundary red, C# init red and new real native-stop proof are the controlling
behavior evidence. No claim of a nonexistent old-Rust runtime red.

Controlled Python noncooperative child is a separate tiny test process: cooperative branch removes
its exact partial; uncooperative branch records explicit force and retains partial/Unknown until
test-owned teardown. It is NOT real archive codec cancellation or resource-pressure evidence.

## Actual small partial / one real sampling smoke

`archive resource tool actual small stop convergence`: LIST1/RUN0 then RUN1/PASS1,exit0.
Original runner/Job/actual Rust worker writes the first65,536bytes of independent256KiB Stored ZIP.
Probe-only FileStream barrier is installed at existing BeginFile diagnostic boundary; actual native
write/flush completes before hold. Product writer/codec/runtime unchanged. Original Cancel/Stop
happens BEFORE pump release, then original terminal/drain confirms worker exit/Job+pumps drainage,
source writable/helper removable/partial absent/ActiveCount0. Empty owned output removed only
nonrecursively; unknown outputs are retained.

Exactly ONE external memory-sampling smoke was performed:
`run.py <owned-small-root> tool-stop-small correctness`, CLI exit1 **by design**, independently
asserted expected stopped contract exit0. Run ID `b0cd5493-215d-4e97-9adf-c2c7d1eca427`.

- resource_verdict=Stopped, operation=Stopped; original worker category cancelled/exit0.
- stop reason injected_partial_threshold, cancel dispatched/accepted once, forced=false, failure=null.
- measurement Complete. Exact owner PID48348/parent27916/apphost path:15valid samples; actual worker
  PID47400/parent48348/exact helper path:7valid samples. Both identity_verified=true, failures0,
  alive_at_end=false. C# internal same-handle counters8host/4worker valid pairs.
- Native partial65,536bytes while source and slot held. Actual cleanup Confirmed: worker exited,
  native terminal/Job+pumps drained, active_count0, source writable, helper removed, partial absent,
  empty output removed. Exit observation is polling upper bound, not native cancel-receive time.
- operation full terminal304.4278ms from dispatch; cancel273.9597ms, cleanup difference30.4681ms;
  values are observation of this tiny held tool fixture, NOT general codec cancellation target.
- Sampling20ms requested/50ms discovery; actual gaps and sampled peaks in raw JSON/JSONL. Short
  peaks/processes can be missed; PrivateBytes is processcommit, not machine deltas/absolute peak.
  No first paint, no100k/1GiB/ZIP64performance repeat or cache control.

Two further Rust tiny lifecycle checks were **functional only, no memory sampling**:
product-credit-small external driver_exception stop => original typed remote cancelled/cleanuptrue,
exactly one operation, no reuse, original pipe/host/source/helper/package released, noforce, exit0;
controlled tool-guard-failure => original-pipe EOF/host graceful cleanup, exit101 retained as failure,
no force/package residue. Full status/stdout/stderr in new results. These do not refresh scores.

## Current verification (actual counts, not historical constants)

All Cargo commands inherited jobs1/incremental0/RUSTFLAGS codegen-units256 and machine-lane checks.
C# apphost built first; actual-worker tests and worker relinking serialized. External wallhaven
Cargo activity was observed, never stopped; one bounded wait released after100s. Every actual
command/start/end/exit is retained in results; no release command was run by this task.

| Gate | Current result |
| --- | --- |
| Python unique tool tests / AST |24pass,exit0 /8scripts PASS |
| Rust resource example tool tests |11pass/0fail/0ignored,exit0 |
| Full Archive isolated-codecs/offline/serial |138pass/0fail/0ignored,26groups,exit0 |
| Protocol / IPC --lib |51/29pass,0fail/ignored,exit0 |
| Archive/Protocol/IPC strict Clippy tests+examples default+isolated |both exit0 |
| Scoped Rustfmt / global cargo fmt --all --check |PASS / exit0 (current; historical fmt failures not erased) |
| cargo check --workspace --tests --offline |exit0; compilation only, NOT workspace tests |
| samefeature Debug worker/probe build |exit0 |
| C# final Debug build |exit0,0warnings/0errors |
| Shared Extensions --test archive_import / Clipboard archive_ |7/5pass,0fail/ignored,exit0 |
| First Extensions invocation |wrong name filter selected0tests; retained, NOT used as PASS |
| C# exact collection |see anomaly below; no integrated132/132 claim |
| Large4GiB+17 ZIP64 /100k/1GiB repeats |RUN0; historical only |

### Exact C# collection anomaly / bounded follow-up

Preserved original127 named scenarios (already excluding unique large ZIP64 and including prior
timeout regression), plus5new tool fixtures => LIST132/RUN0, exact ordered names reconciled.

1. First run missed explicit fixture/oracle env: selected132/run132/pass77/skip55/fail0/not_run0,
   process exit0; verification wrapper correctly rejected it. No skip was called PASS.
2. Configured run with existing independent ZIP/codec fixtures, CPython3.13.2 and stock7-Zip26.00
   (SHA2564bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f):
   **process exit1, RUN100/PASS99/SKIP0, no FAIL or SUMMARY**, last RUN is ZIP Zstd native reads.
   All logs retained. No live host/worker remained; its fixed helper was hash-equal to the exact
   built worker then removed only after no-live-owner check. This is NOT integrated PASS.
3. One targeted diagnostic at that exact existing ZIP Zstd case:LIST1/RUN1/PASS1,exit0. No product
   source changed; recurrence cause unproven. This does not erase whole-collection failure.
4. Only the previously uncompleted suffix32:LIST32/RUN32/PASS32/skip0/fail0/not_run0,exit0.
   Across configured99+targeted1+suffix32, all132exact identities have current PASS coverage.
   **Coverage is not one integrated132/132 run.** No more whole-suite green hunting; the single
   configured collection's unexplained process exit remains an integration verification blocker.

New tool fixture group including actual small partial is passing; product regression anomaly is
not attributed to the tool repair without evidence and no unrelated product/codec repair was made.

## History, cleanup and restrictions

Original resource-acceptance-review.md/evidence/results/contract and other baseline report hashes
were checked unchanged. New summaries label old records Legacy/missing-contract-fields and old
C# times setup-inclusive without touching original files/values. Unrelated dirty changes preserved;
no reset, Desktop/Clipboard formatting, commit or push. New owner guidance, README/context and the
single current checkpoint are updated separately; original historical checkpoint is not overwritten.

Final hygiene/results are recorded in resource-tool-final-hygiene.json: exact self-owned small
fixtures removed only after recorded hashes/confirmed cleanup; JSONL manifest/sample/status/logs
retained. No name-based killing/unknown partial deletion/recursive scratch sweep. No formal Desktop/
installedPlatformHost, Release/package/sign/install, registry/association/service/drive/cache/pagefile/
OOM pressure, userarchive, real recycle/permanent source deletion. Native recycle remains
native_protection_unproven/NotAttempted. Stop this slice; no Phase B/100k product escalation/UI/native
recycle/performance optimization. Final document gates and bounded-review details appended below.

## One bounded review / final targeted corrections

One read-only tool-lifecycle review, no second pass. Two concrete findings were reproduced before
fix: internally requested probe stop did not start the parent's ten-second grace (virtual161s
before force), and output-directory removal failure was omitted from C# cleanup confirmation.
`resource-tool-review-internal-stop-red.log`:1fail/exit1; corrected driver imports original owner
stop immediately, green within original10s. `resource-tool-review-csharp-red-run.log`:1run/1fail,
exit1; corrected Control.Finish and native confirmation require owned empty-output removal.
Final Python24/24, C#fakegroup4/4 and real small-partial functional1/1 pass, all exit0; no second
memory-sampling smoke or broad-suite rerun. New C# change only tightens cleanup classification.

Reviewer also noted a hypothetical permanently blocked Win32 counter call would outlive the
sampler's finite join and could keep its non-daemon thread alive. No blocked native API was
reproduced; documented fast local process-counter calls have no arbitrary-call interruption
guarantee. No replacement thread/process or second native runtime was added to hide that limit.
Current tests cover returned failures/deadlines/control races, not a permanently hung kernel API;
this is an explicit measurement-tool limitation, not claimed green proof of arbitrary interruption.

Final document/source gates: spec-contracts106docs/14owners, GPUIcatalog79/79/78, taskcontext63/60,
selector all exit0. Existing native-contracts35,497byte context-injection warning remains; applicable
content was read directly, no unrelated document split. Final history11hashes unchanged; exact
process/pipe/helper/output and task-bytecode cleanup recorded in resource-tool-final-hygiene.json.
No additional review or retry-to-green loop.
