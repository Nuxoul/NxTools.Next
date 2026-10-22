# Bounded Windows archive resource tools (Debug only)

Canonical **Phase A — Engine contracts and safety foundation**. This is opt-in test tooling,
not product/Release acceptance. Read the original resource-acceptance-contract/evidence and
resource-acceptance-review plus the additive `resource-tool-repair-contract.md` in the owning
Trellis task. Historical measurements/failures are immutable. No codec, budget, product timeout,
publication/recycle/wire permission change, user archive, installed app or system pressure.

## Small correction regression (this slice)

```powershell
$env:PYTHONDONTWRITEBYTECODE='1'
python -m unittest discover -s tests/performance/archive-resources -p 'test_*.py' -v
# Coordinate one machine-wide Cargo lane; NEVER overlap actual worker tests and relinking.
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -m:1 -p:UseSharedCompilation=false
cargo test -p nxtools-archive --example resource_probe --features isolated-codecs --offline -- --test-threads=1
cargo build -p nxtools-archive --features isolated-codecs --offline --example resource_probe --bin nxtools-archive-worker
# LIST new C# fixture names first, then execute exact audited names; see test catalog.
$root=Join-Path $env:PI_SCRATCH_DIR ('archive-resources-'+[guid]::NewGuid().ToString())
python tests/performance/archive-resources/fixtures.py $root --small
python tests/performance/archive-resources/run.py $root tool-stop-small correctness
```

`--small` generates only a 256KiB Stored ZIP/manifest and five-entry credit ZIP, not 100k/1GiB.
`tool-stop-small` injects a tool threshold after a controlled real owned partial barrier, requests
original runner cancellation, and records **Stopped**, not resource PASS. Driver CLI therefore
exits nonzero for this expected stopped scenario; evaluate the saved stop/measurement/cleanup
facts as a functional test. One bounded real sampling smoke maximum for this correction; no
performance repeat/automatic retry. Other failures also preserve their attempt JSON.

Preflight available RAM>=4GiB, commit headroom>=6GiB, fixture disk>=4GiB; build disk>=8GiB.
Stop at available<2GiB, commit headroom<3GiB, disk<2GiB or finite sampler/driver deadline. Inspect
machine-wide Cargo/rustc before every build/run; bounded wait, never kill unrelated builds.

## Test-only ownership and stop

Python owns the exact Popen handle and a fresh `control-<UUID>` directory within the marked
fixture root. `NXTOOLS_RESOURCE_RUN_ID` and `NXTOOLS_RESOURCE_CONTROL` bind one probe. First-wins
`stop.json {run_id,reason}` is atomically published once; main/sampler concurrent or late requests
are idempotent. C#/Rust publish bounded `status.json` with schema2, original identities/expected
images, state and actual cleanup facts. This is NOT a PlatformCommand or remote kill/delete API.

State: Running -> StopRequested -> Cancelling -> Draining -> Cleaned; failures may be
Forced/CleanupIncomplete/OutcomeUnknown. Probe cancels ORIGINAL OperationControl/product operation
or C# runner, waits actual worker/pumps/Job/source/helper/partial/slot cleanup. Local Cancelled,
receipt, force receipt or root disappearance alone is not cleanup. Partial initialization must
retain owners through finally/guard. No follow-up operation/reuse starts after tool stop.

Python uses bounded nonblocking reads of its own inherited pipes, not blocked reader threads.
After ten seconds without cooperation it may terminate only exact retained/verified owned
instances, recording targets/reason/native errors; after bounded final observation unconfirmed
cleanup remains Unknown. No process-name kills, no recursive scratch sweep, no uncertain partial
deletion. Original C# runner remains the only native Job/process runtime. The controlled Python
uncooperative fixture is separate tool evidence, never actual codec cancellation evidence.

## Measurement schema2

Operation, measurement, stop and cleanup are separate fields. Overall `resource_verdict=PASS`
requires expected operation success, every required PID identity verified with valid samples,
no stop/force/tool failure, and confirmed cleanup. Functional exit0 cannot certify resources.

Required roles: engine=probe_owner; direct worker=root probe_owner+actual_worker; runner=owner+
actual_worker; product=owner+isolated_csharp_host+actual_worker. Exact PID/image and original
parent/operation provenance, never name aggregation. conhost is not a business worker. Retain
acquisition/image/counter attempts/success/failure/native errors, including zero-success records.
Finite startup retry may recover; missing/short-lived/unverified/all-failed PID is Incomplete or
Unavailable, with null unavailable memory values, never invented zero MiB. C# successful count and
peaks advance only after BOTH WorkingSet and PrivateBytes reads succeed.

20ms sampling/50ms discovery are requested intervals. Actual valid-sample gaps are retained;
short processes/subinterval peaks can be missed. PrivateBytes is process commit, not machine RAM.
Process-lifetime high-water counters are supplemental, not phase peaks. Probe/observer/setup
memory is included. No first paint: first business batch is NOT UI presentation.

C# setup_ms is separate; operation clock starts immediately before runner.Start. First batch,
full terminal, cancel, exit observation and cleanup stamps share `timing_origin=runner_dispatch`.
Cancel differences are one domain. Legacy C# runner/cancel stamps remain setup-inclusive; no
subtraction/recomputed historical score. `summarize.py ROOT OUTPUT` reads all attempts without
modifying inputs, preserves new status/reasons and marks old/missing schema Legacy. Exact current
LIST/RUN/PASS counts are derived from saved selection/logs, never fixed historical numbers.

### Retained-output summary correction (Python only)

For valid attempt JSON, malformed/truncated `RESOURCE ` lines no longer abort collection.
Valid events retain their order and later attempts remain present. Each attempt has independent
`summary_parse`: `Complete` with no parse errors, otherwise `Incomplete`, even if its preserved
original `resource_verdict` is `PASS`. An original PASS does not certify summary completeness.
Diagnostics use one-based stdout line numbers and fixed `invalid_resource_json` categories;
at most16 details per attempt, plus total `error_count` and `errors_truncated`. They do not copy
payloads or exception text. Original stdout/failure/operation/measurement/stop/cleanup/verdict
remain unchanged for schema2; existing Legacy annotation and dynamic test counting are unchanged.
This is not a general corrupt-file recovery framework; invalid outer attempt JSON still fails.

Run only the Python unittest command above for this correction; no Cargo/C# build, worker smoke,
sampling or large/performance repeat. Exact red/green, immutable-input hashes and limits:
[summary repair evidence](../../../.trellis/tasks/10-16-archive-manager/resource-summary-p2-repair.md).

## Historical performance scenarios (not rerun by this correction)

Original `engine100k`, `direct-worker100k`, `runner100k`, `cancel1gib[-hold]`, `product100k` and
small product-credit/pressure scenarios remain available. Full generator without --small still
creates historical-sized independent CPython fixtures; do not run it routinely. No 1GiB product
first-batch repeat, ZIP64 large test, cache flushing or best-score hunting. Direct legacy schema1
has no native Job/source lease/snapshot-ID claim; engine checks snapshot/ordinal separately.
Runner unchanged8MiB stdout refusal is not full100k transport. Product4096/4MiB ResourceLimit is
not product100k support; no budget increase. Held cleanup is injected, not unheld latency.

Remove only exact self-owned fixtures after hashes and no-live-owner/cleanup checks; preserve
manifest/attempts/logs, including failures. Never sweep unknown roots. Current results and
remaining gates live only in the task's unique phase-a-checkpoint.md; this README grants no
full ENG08/ENG09/SAFE04/Phase A/A–F/native/release acceptance.

Native observation limit: a permanently blocked Win32 counter/query call is not proved interruptible
by the finite sampler join. Returned native failures and normal deadlines are tested; no arbitrary
kernel-call interruption guarantee or replacement sampler runtime is implied. Keep this limitation
separate from confirmed cooperative operation cleanup. Probe-internal stops start the same parent
ten-second grace immediately; owned empty-output-directory deletion is part of cleanup confirmation.

## Explicit finite product100k v2 (separately authorized)

`fixtures.py ROOT --catalog-only` generates exactly100000members with a streaming entries.jsonl
manifest and credit.zip; it does NOT generate1GiB. `run.py ROOT product100k-v2 correctness` uses
the unchanged accepted sampler/stop/cleanup infrastructure and actual typed product chain.
It first proves default ResourceLimit, then requests explicit v2finite limits, streams exact
manifest comparisons, verifies owner/remote terminal and original connection reuse/cleanup.
Original product100k remains the default-refusal observation, not silently rewritten success.
Firstbatch is not firstpaint; Debug sampled WS/private is not absolute peak or budget usage.
One failed attempt and one declared repeat after a deterministic partial-frame transport repair
are retained for this slice. Do not routinely rerun/optimize: commands/counts/PIDs/limits and all
attempts are in the owning product-100k-evidence.md/results.json; phase-a-checkpoint.md alone
owns current status. No1GiB/largeZIP64/Release/deployment/user-archive authority.

## Bounded Windows status-read repair (Python tools only)

Independent product100k acceptance passed the original100000entry/391batch read, but its full
observation is still Failed/Stopped after status PermissionError; follow-up same-router reuse
was NOT verified. Do not infer overall resource PASS from its successful inner operation.

Status reads preserve actual Win32code (CRT PermissionError/errno13 alone loses the distinction).
Only native32sharing/native33lock open/read conflicts retry: max4attempts,10ms between attempts.
Native5AccessDenied, absentnativecode, otherI/O, invalidrun/schema/JSON/oversize remain failures.
Main/sampler/final use the same reader and per-run `status_reads` counters plus<=16fixed details
(caller/stage/type/errno/nativecode/attempt). No payload/path/errorprose or per-poll log stream.
No last-good status is returned after error/missingread; finalread cannot fallback to oldCleaned.
PASS requirements for operation/measurement/stop/cleanup are unchanged; retry exhaustion fails.

Reader grants SHARE_READ|WRITE|DELETE and closes exacthandle before return. That does not prove
MoveFileExoverwrite succeeds while a targethandle staysopen: native5was observed, then success
afterclose. Publishers keep original temp-close-replace/failclosed policy; no blanket5retry.
Run Python unittest/AST and doc/selector checks only; this correction authorizes no largefixture
repeat. [Repair contract](../../../.trellis/tasks/10-16-archive-manager/status-read-p2-contract.md)
and [repair evidence](../../../.trellis/tasks/10-16-archive-manager/status-read-p2-evidence.md)
are separate from immutable product-100k-acceptance.md and old attempts/results.
