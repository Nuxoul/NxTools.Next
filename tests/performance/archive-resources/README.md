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

## OPT-00 completed format baseline

`prepare-opt-formats.py` creates a new marked root below `PI_SCRATCH_DIR`. It copies the
attributed solid7z fixture, downloads the already-indexed solidRAR5 data from pinned libarchive
commit `79c571f845dbdd250d69bff9d93ae034f674135e`, retains attribution/license, and creates two
deterministic USTAR members in TAR.GZ/TAR.XZ. System `tar.exe` independently decodes expected
payloads before measurement. It never executes archive payloads or changes existing inputs.

Use a previously built Debug `isolated-codecs` worker and the original compiled C# test host:

```powershell
$env:TEMP=$env:PI_SCRATCH_DIR
$env:TMP=$env:PI_SCRATCH_DIR
python tests/performance/archive-resources/prepare-opt-formats.py
$env:NXTOOLS_ARCHIVE_OPT_FORMAT_ROOT=(Get-Content (Join-Path $env:PI_SCRATCH_DIR 'opt-format-root.txt') -Raw).Trim()
$env:NXTOOLS_ARCHIVE_WORKER_FIXTURE='<absolute frozen worker path>'
dotnet run --project csharp/NxTools.PlatformHost.Tests --no-build --no-restore -- --fixtures --list 'archive OPT00 completed format baseline'
dotnet run --project csharp/NxTools.PlatformHost.Tests --no-build --no-restore -- --fixtures 'archive OPT00 completed format baseline'
```

The identical selection must LIST1/RUN1/PASS1/SKIP0. Each side completes three open/Test/
last-member-materialize/all-extract/last-member-extract trials for four formats. Reuse the same
root and host for both frozen worker versions; do not regenerate between comparison sides.
Outputs are compared with independent bytes, original ordinals are retained, source leases
must be released, and native outputs/helper/cache leases are cleaned through their original owners.
Inputs, independent expectations and provenance remain explicitly retained for comparison.

The existing Samples collector uses the runner's native process handle, not process-name lookup.
Requested20ms samples may miss peaks. JSON pipe bytes are not source reads; runner notification
intervals are not exclusive codec phase costs. Missing source/block counters remain unavailable,
not zero. The task's `opt00-opt01-results.md` and native JSONL record the acceptance boundary.

## OPT-04 retained drag fixtures

The separately authorized Phase E/OPT-04 controlled-COM measurements use the original read/entry/
runner/output/cache path, not the resource sampler or a real Explorer target. `Opt04DragTests` has
exact fixture registrations for same-path lifecycle, formats, split ZIP, full-block integrity,
receiver/work/Seal lifetime, ordinary preview adoption and solid encrypted RAR4 authorization.
Effects are in the original Program/TestEffects catalog; LIST the exact selection before RUN.

Required test inputs: explicit freshly built `NXTOOLS_ARCHIVE_WORKER_FIXTURE`, marked independent
format root `NXTOOLS_ARCHIVE_OPT_FORMAT_ROOT`, and session-owned `PI_SCRATCH_DIR`. Set TEMP/TMP to
that scratch root for the existing owned-root guard. The comparison lifecycle additionally needs
`NXTOOLS_ARCHIVE_OPT04_ROOT` pointing at its completed immutable before input/report; do not set it
for fixtures that create fresh roots. Keep the stable worker/host/test execution paths and Debug/
Release mode identical across before/after. First ZIP-green and final all-format evidence remain
separate. Never overwrite an existing measurements-before.json or measurements-retained.json.

`prepare-opt-formats.py` supplies solid7z/RAR5/TGZ/TXZ and independent expected bytes. For the added
solid RAR4 case, set the existing `NXTOOLS_ARCHIVE_7ZIP_ORACLE` to the explicitly approved decoder,
then run `python tests/performance/archive-resources/prepare-opt04-rar4.py`. It downloads one pinned
public libarchive fixture and attribution/test source, verifies RAR4/solid and independently decodes
four fixed payloads into a new marked GUID root. Set `NXTOOLS_ARCHIVE_OPT04_RAR4_ROOT` to the printed
root; no historical input is modified. Fixture password is public test data and never enters argv
of the product worker, logs or diagnostics.

Report worker/source/catalog counts, actual format invocations/replays, independent cache reads,
member publication/Seal-ready and actual receiver/Run cleanup separately. Missing internal native
reads/blocks/codec time and physical gesture/DragOver/target-disk clocks stay unavailable. One7z
public consuming block pass per distinct demand and RAR backward reopen/replay are real work.
Split ZIP has two admission directory passes and retains the normalized source thereafter.
Results, three-trial ranges and the scoped stop point:
[OPT-04 evidence](../../../.trellis/tasks/11-02-archive-workflow-optimization/opt04-retained-materialization.md).

## OPT-05 operation-local format and update plans

The original owner counters measure ZIP local/descriptor admission/record seeks and source/addition/
candidate payload decodes, TAR outer bytes/scans/ordinal locators, shared selection visits and native
source/target observations. They are not total source I/O, all seeks, exclusive codec time or peak
memory. Aggregate at the original operation terminal, including failures, without member/path/chunk
logging. Exact contracts: [format plans](../../../.trellis/spec/nxtools-archive/backend/reusable-plans.md)
and [native observations](../../../.trellis/spec/platformhost/archive-read-observation.md).

Kernel tests: `opt05_zip_plan_contract`, `opt05_tar_plan_contract` and library `opt05` selection tests.
Set the required memory environment in the same invocation, TEMP/TMP to PI_SCRATCH_DIR and keep one
Cargo lane. ZIP comparison uses original marked `NXTOOLS_OPT05_ZIP_ROOT` and create-new
`NXTOOLS_OPT05_ZIP_REPORT`. TAR uses original `NXTOOLS_OPT05_TAR_FIXTURES` and a fresh
`NXTOOLS_OPT05_TAR_REPORT_LABEL`; explicitly execute `opt05_tar_baseline_three_completed_trials`
with `--ignored --exact --nocapture`, not an ignored acceptance count. Preserve every prior label.

Native exact fixtures: `archive OPT05 completed compound format baseline`, `archive OPT05 compound
format plan reuse`, `archive OPT05 native equivalent observations`, `archive OPT05 native identity
version and cancellation`. Review original effects, call the actual freshly built apphost with
`--fixtures --list` plus exact names, then the identical selection without `--list`; no dotnet run,
blanket native suite, zero selection or skip acceptance. Supply matching frozen
`NXTOOLS_ARCHIVE_WORKER_FIXTURE`, marked two-format `NXTOOLS_ARCHIVE_OPT_FORMAT_ROOT` and create-new
`NXTOOLS_OPT05_FORMAT_REPORT`; retained four-format preservation uses its original four-format root,
not the two-format comparison root. Direct native observation uses the retained
`NXTOOLS_OPT05_NATIVE_ROOT`; baseline-only `NXTOOLS_OPT05_NATIVE_BASELINE=1` is test tooling, never
product policy. Remove it for actual equivalence assertions. Preserve the resulting unique reports.

Use the same input/build mode/disk/route/stable execution paths, three completed principal trials,
no system-cache flush and no best-run selection. Report work, encoded readback, decode, publication
and actual cleanup separately. Reduced counts with overlapping timings are work reduction, not a
general speedup. [OPT-05 result](../../../.trellis/tasks/11-02-archive-workflow-optimization/opt05-reusable-plans.md)
and [all comparable measurements](../../../.trellis/tasks/11-02-archive-workflow-optimization/opt05-measurements.md)
retain raw three-trial data and open G-00/overall/physical/Release gates.

## OPT-06 private bulk comparison

`Opt06BulkTransferTests.Measurements` measures the original browser/Job/Run/native routes for ZIP
Store creation, ZIP extraction and ZIP Add. Each side has three completed trials, deterministic
1MiB source/addition inputs, independently produced ZIP, fixed per-route/trial source/output paths
and the same owned worker execution path. Independent decode and fixture verification are outside
the operation timer; actual Run drain is inside it. No cache flush or best-run selection.

Freeze the observed before source/worker/Host/tests before changing transport. The exact fixture
selection is `archive OPT06 binary transfer measurements`; inspect `--fixtures --list` then use the
identical filter without `--list`. Set explicit `NXTOOLS_ARCHIVE_WORKER_FIXTURE` and the existing
`NXTOOLS_EDIT_INTERACTION_WORKER_SHA256` approval marker to the matching frozen Debug worker.
`PI_SCRATCH_DIR` and `NXTOOLS_OPT06_ROOT` must be absolute; the root is a direct GUID-owned
`archive-product-opt06-<GUID>` child. Baseline-only `NXTOOLS_OPT06_BASELINE=1` creates marked inputs
once. Clear it for after; inputs are byte-verified, never regenerated. `NXTOOLS_OPT06_REPORT` names
a new file directly inside the same root's `reports` directory; never overwrite before or earlier
attempts. Do not run under Cargo/dotnet/native fixture build load.

The fixture requires actual source/output/held-partial payload counters, one publication and
Committed journal, source preservation, exact independently decoded bytes, zero active Runs,
exclusive reopen and no owned partial. Current binary assertions require wire bytes equal payload
plus80bytes per frame, source/partial EOF included in frame counts, zero modern conversion input,
one source/partial envelope copy, <=2 host payload buffers and <=131072payload bytes high-water,
with a1MiB reservation inside existing64MiB overhead. JSON bytes/frames remain separate.

Sampled owner/worker memory and worker process CPU interval are not exact peaks or exclusive codec
time. Operation wall includes real cleanup; separately timed cleanup, complete/internal source I/O,
and unobserved before conversion/copy/buffer metrics remain null/unavailable. Compare native role
and actual payload work separately from wire reduction. The [frame contract and results](../../../.trellis/tasks/11-02-archive-workflow-optimization/opt06-binary-transfer.md)
retain source attribution, all samples and scoped handoff; no uniform speedup/reference equivalence
or independent acceptance follows from these measurements.
