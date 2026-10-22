# Phase A — bounded resource acceptance contract

Date: 2026-10-19. Canonical **Phase A — Engine contracts and safety foundation**.
This contract is saved BEFORE fixtures/builds/resource runs. ENG08/ENG09/SAFE04 evidence
only; no complete scenario, Phase A, Phase B or A–F acceptance. Historical independent
138 Archive / 50 Protocol / 29 IPC / 127 C# results are NOT this run's results.

## Change boundary

Add independent fixture generator/streamed manifest, opt-in resource tests/measurement wrapper,
raw bounded results and evidence. Reuse actual engine, worker, typed pipe, shared C# session,
runner/Job/source/helper owners. No production change unless a concrete reproducible in-scope
defect has a real-boundary red first. No dependencies/upgrades/default-limit changes, UI/FIFO,
product write feature, native recycle, deployment mapping, Release/package/install/sign/commit.
Preserve unrelated dirty files and all historical acceptance reports/failures.

## Scenarios and oracle

1. CPython zipfile creates exactly100000 Stored ZIP entries, including explicit directories,
   Unicode and empty files; small payload total. Independent JSONL manifest determines ordinal,
   name, kind, size and packed size. Compare sequentially, not duplicate100k metadata Vecs.
   Engine default ResourceLimits (1million entries/256MiB metadata/512MiB work) first; an explicit
   lower finite metadata/entry budget must reject ResourceLimit. Existing with_limits/WorkerLimits
   are legal finite engine interfaces, NOT product-budget permission.
2. Actual worker executable enumerates the same ZIP through its existing bounded private frames;
   validate each frame against streamed manifest. Direct private protocol observation does not
   claim native Job/source safety. Separately use original C# runner/Job and unchanged stdout8MiB
   ceiling; if transport rejects, record stdout_total_limit and partial count, NEVER success or
   full100k worker-through-runner listing. No limit increase to make this pass.
3. Real Rust product owner -> typed original pipe -> isolated C# TEST apphost -> shared production
   session -> original Job/worker:100k input with4096/4MiB product defaults must give typed
   ResourceLimit (or actual MetadataBudget at the boundary), never unsupported/corrupt/truncation.
   Explicit max_entries100000 remains invalid_request: no product finite escalation above4096
   exists. Record this precise future gap, not completed100k product listing.
4. CPython independently writes one Stored ZIP containing exactly1073741824 synthetic payload
   bytes, repeated bounded deterministic blocks (no separate raw1GiB copy, no compression bomb).
   Use real supported ZIP extraction only as an isolated test path, NOT new product write wiring.
   Observe an owned exclusive partial with0<size<1GiB as the active native-write/data barrier,
   then cancel original runner. Compare cancel call, observed worker Cancelled frame when exposed,
   process exit, pre-disposal cleanup barrier and actual final resource release separately.
   If a frame is not observable, mark it unavailable, not receipt-as-worker-ack.
5. Same1GiB input: paused/metadata-credit wait cancellation via real product chain; use received
   first batch without ACK as a verified wait. Original-connection command-queue-pressure Drop
   scenarios reuse the production dequeue barrier, not queue growth/retry/new connection.
   Existing small forced-termination fixture remains distinct from cooperative evidence.

No UI: first paint NOT MEASURED/not applicable. First business batch latency is not first paint.

## Repetition, timing and memory

Each new scenario: one correctness run, then at most ONE predeclared measurement repeat (two
runs total). Repeat is ordinary second access / possibly cache-warm; no strict cold/warm claim.
All attempts/failures and any justified targeted repair rerun stay in ledger. Regression suites
run once after stable tests. No repeated best-score hunting or timing assertion in normal tests.

Use monotonic clocks, operation UUID plus exact PID and executable path. Identify C# runner's
owned process from its native process handle/test-side observation; never aggregate/kill by name.
Roles: Rust test/product caller, isolated C# test host, actual worker, sampler. Sample exact
processes every20ms where feasible; log actual intervals, baseline, peak WorkingSet and Private
Bytes, terminal/end state. Windows process lifetime PeakWorkingSet/PeakPagedMemory where available
is supplemental, not a precise sampled phase peak. PrivateBytes is process commit, not machine
commit. Sampler may miss subinterval peaks and observer overhead is included. Machine-wide free/
commit figures are preflight/load context only. Model reservation/Job ceiling are not measured RSS.
Counters/metadata comparisons are streamed; no per-byte/block/batch diagnostic logging.

Operation start is immediately before open/start dispatch; engine full-open ends on returned
snapshot. Worker first batch includes engine full-open/source work; final end requires actual
process/pump/Job cleanup and source/helper release, not private complete/local Cancelled/receipt.
20ms process-exit observation is an upper-bound timestamp unless a native event supplies exact
ordering. A pre-disposal cleanup barrier may deliberately hold resources: record injected hold
separately and exclude it from unheld cooperation latency. Slots cannot be reported reusable there.

Per-operation deadline120s for listing/extract; product unchanged30s, credit up to10000ms;
cleanup deadline10s, runner own200ms cooperative grace then owned Job termination unchanged.
Sampler deadline180s and finite<=10000 samples/process; metadata/log records bounded. Long Debug
build wrappers have full-operation timeout and surviving-process inspection before any retry.
No claims that arbitrary codec/OS calls meet200ms. Debug target misses alone do not justify
algorithm optimization. Normal cancellation and forced termination reported separately.

## Resource preflight and abort

Before fixture generation, every Cargo/build and each resource scenario: machine-wide Cargo/
rustc inspection, no concurrent build or worker relink. Wait for unrelated builds (do not kill).
Required Cargo env EACH invocation: CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,
RUSTFLAGS=-C codegen-units=256. Build Debug C# test apphost before real Rust product chain.

Require available physical memory>=4GiB, commit headroom>=6GiB, fixture disk free>=4GiB and
build disk free>=8GiB. Fixture peak budget<=2.2GiB including one partial, small ZIP/manifest/logs.
At observed available<2GiB or commit headroom<3GiB or fixture free<2GiB during runs, stop launching,
cancel ONLY current owned operation, await real cleanup, preserve evidence and report blocked.
Never manufacture memory pressure, clear caches, modify pagefile/system configuration, mount/map
volumes, or stop unrelated processes. Background Unity/.NET activity recorded, not terminated.

## Evidence and preservation

Scripts in tests/performance/archive-resources/; opt-in Rust/C# tests in their existing owners.
Temporary fixtures/build scripts/raw logs in session PI_SCRATCH_DIR/archive-resources-<run-id>.
Persist compact raw measurements/commands/environment/hash manifests and final evidence in owning
Trellis task; large fixtures/outputs removed ONLY under unique self-owned root after verified
cleanup; retain scripts/manifest/hash/logs, including failures. Never operate user sources.
Record Windows/CPU/RAM/free/commit/disks/FS, Debug flags, exact versions/hashes/dependencies,
background builds and cache caveats. End checks include original source writable, helper image
removable, ActiveCount/reservations zero, owned process gone, test pipe closed and fixture roots.
Native recycle stays native_protection_unproven/NotAttempted; no actual recycle/permanent source
deletion, installed Desktop/PlatformHost/service, registry/association or signing operations.

Final regression scope if tests/scripts/docs only: new directed acceptance, complete Archive,
Protocol/IPC related suites, original C#127 precise selection minus ONLY
`archive ZIP creation fixture large ZIP64 streaming`, then additions; LIST/RUN/PASS exact names,
selected/run/pass/skip/fail/not_run and exit codes. Existing4GiB+17 final-code ZIP64 is historical
and is NOT rerun unless a production repair touches creation/ZIP64/publication. Shared ZIP
extensions/clipboard gates retained. Source/static checks do not replace resource measurements.
Final required document/selector gates and diff/conflict/temporaryDEBUG/owned cleanup checks.

## Recorded bounded amendment after first actual runs

1GiB product open reached the unchanged30s deadline before first metadata batch; capture shows
cancelled/cleanuptrue at30070ms WITHOUT local cancellation. This is NOT credit-wait acceptance,
and not a Release performance/algorithm verdict. Do not repeat this large product request or
increase timeout. Use independent5-entry/15-byte credit.zip for separately labeled credit/queue
measurements (one correctness+one repeat maximum).1GiB native extraction data-cancel and cleanup
remain distinct. Runner100k attempts include retained TEST-tool startup/sampler failures; one
tool-repair operation completed with stdout_total_limit and internal memory samples, external
sampler rejected Windows conhost. No additional runner performance repetitions are authorized
to chase green. Further TEST-only fixes do not erase those failed attempts. Direct legacy
schema1 worker frames omit snapshot ID: verify ordinal/name/type/size; engine verifies session.
Worker cancel-accept timestamp is not exposed; actual Cancelled frame may be unavailable when
runner pump stops after local cancellation. Report unavailable, normal exit0/actualcleanup and
forced termination separately, never invent worker-observed timing.
