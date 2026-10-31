# Archive CLI — bounded lifecycle validation outcomes

Date: 2026-10-24. Same **Phase F — Explorer integration and genuine native drag**.
**Delivery status: archive CLI lifecycle slice pending independent acceptance.**
Public ordinary list/test/extract acceptance remains CLOSED at its recorded level in
[archive-cli-acceptance.md](archive-cli-acceptance.md); that report and historical failures were not
rewritten. [Original contract](archive-cli-contract.md#bounded-lifecycle-evidence-contract--2026-10-24),
[approved plan](implement.md), [sole current checkpoint](phase-a-checkpoint.md).

## Scope / change outcome

No product defect was confirmed. **No product Rust CLI, C# Program/ArchiveCliHost/ReadSession/
JobSession/runner/logging implementation, generated approval, dependency or vendor was changed.**
New code is test infrastructure plus exact harness registration: one owner-only C# cleanup fixture,
one independent-console Python lifecycle driver, one scratch-only C# checkpoint observer and its
preparation script. Contract/quality/README/plan/current-checkpoint documentation is updated.
No new command, password input, runtime cleanup framework, protocol narrowing or logging rewrite.

The internal-host extra request shapes and inherited DisposeAsync ignoring DrainAsync(false) remain
observations, not automatically promoted to public defects. The tested original short-drain gate
held ownership correctly. No speculative fix was made to manufacture a code change.

## One evidence root and exact build level

`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\archive-cli-lifecycle-20261024-104628`.
Preparation/build and per-case logs01–24, baseline/source/delivery preservation, selected-lifecycle-
events.json, case-summary-final.json and per-case result/checkpoints/stdout/stderr are retained.
All samples are independently generated two-file Store ZIPs (6-byte first,7-byte second), or one
<1KiB literal-payload ZIP in the owner fixture. No user archive/target or performance workload.

- Real CLI image SHA256 **DF24070DED3AA927703F614CAEBCEC46CD881A3DBD1762D852BAFA2DD9BBDA3E**,
  same as independently accepted CLI. Real Windows CTRL_C_EVENT reaches that unchanged process.
- Fresh matching isolated-codecs worker SHA256
  **7C81BF94489D878C41F18B46857EF7851DE24ED693CF9E5388DE5442CF9BB8D1**.
  Original feature tree then rebuild and original manifest generator produced scratch approval.
- **Controlled host, not an uninstrumented production-host claim:** preparation copies current
  host sources into scratch. Exactly the early log factory and original session composition/Publish
  are wired to `tests/integration/ArchiveCliLifecycleFixture.cs`; original session/runner/codec/native
  output implementations are unchanged. Existing write/native_published/control-input/cleanup
  probes only record/wait with8-second bounds. No product env/argv checkpoint override was added.
- Source-copy host versions differ as observer assertions were enhanced; per-case bin is retained,
  so prior signal/partial runs are not falsely attributed to the last observer build. The initial
  controlled host DLL was E99E1406...C028F10; final per-case identities are independently hashable.
- Host initial, controlled host and owner-test builds passed with0warnings/errors. Cargo feature/
  worker commands each checked the shared lane and used jobs1/incremental0/codegen256. No duplicate
  Cargo, timeout, full Desktop/workspace execution or ordinary15-command rerun. No optional review.

## Actual boundary results

| Boundary | Actual result and evidence level |
| --- | --- |
| Native Ctrl+C before publication | **PASS controlled process/signal evidence.** NEW console contained exactly driver8284+CLI29700; one CTRL_C_EVENT only to that console. Original control write observed; CLI remained pending after cancel write/receipt and while real cleanup checkpoint held. Final single JSON exit7/code cancelled, committed0/bytes0/cleanuptrue; journal retains unpublished first partial with partial_removed. No postcancel file publication. `23-final-signal-observation.log`, signal-33eb40d6-bc5a-4629-8817-8155ec9d1129/result.json. Earlier07/signal-dcd4... evidence retained separately. |
| Ctrl+C after first native publication | **PASS controlled process/signal evidence.** Console exactly driver29260+CLI28540, one signal at original native_published checkpoint. Exit7/code **cancelled_with_committed_output**, files1/bytes6/commit_stateCommitted, one published journal entry/cleanuptrue. first.txt literal bytes preserved, second absent, no own partial. No conflict/zero-output flattening. `08-partial-console-signal.log`, partial_signal-428a9b01-2e4f-436c-af62-65f16589f257. |
| Parent CLI terminated after publication | **PASS real owned-parent termination with controlled checkpoint.** Exact Popen-owned CLI26516 terminated only after first native publish; EOF/Stop led to original cancel write, worker/host exit and lease release. No JSON because the parent is dead, exit1 is forced termination, not business success. First6bytes retained, second absent/no partial. `21-parent-exit-after-commit.log`, parent_exit-6cd98047-0678-4b7b-a410-a0f87e597b78. Initial09 precommit parent-exit observation remains separate. |
| Private connection interrupted | **PASS controlled connection/process evidence.** Only own NamedPipeClientStream closed at first native write; original cancel write observed BEFORE releasing work. Living CLI30136 returns exactly one non-success JSON exit7/disconnected with cleanup_complete=false/unknown local facts, never fabricated remote success. Host's actual later operation terminal records cleanuptrue; worker/host exit and exclusive reopening independently confirm release. `22-private-disconnect-ordered.log`, eof-7af0b26d-bca3-4dfe-b917-11483428abd7. Earlier10/eof-12b... retained. |
| Cleanup short timeout | **PASS original owner-only fixture, not public CLI timeout.** C# LIST1=RUN1=PASS1,skip0/fail0/not_run0, exact name/order; original DrainAsync(150) returnsfalse at151ms while2slots active, no exited/cleanup credit, source/helper exclusive denied native32; third request rejected concurrency_limit. After explicit finite checkpoint release: draintrue, exactly2success/exited/cleanuptrue, active0, source/helper exclusiveopen succeeds and hashes stable. `14-cleanup-list.log`, `15-cleanup-run.log`, archive-cli-cleanup-*/evidence.json. |
| Isolated lifecycle diagnostics | **PASS actual sink capture.**20valid own records; start/connected/stopped under one session correlation plus original job terminal under operation correlation, elapsed/native fields and cleanuptrue, no payload/path/password fields. `19-isolated-sink-assertions.log`, diagnostics-e8596775-c8c4-4292-89e2-2581a6abadc6. |
| Default production sink capture | **PASS for observed own calls, not shared-sink reliability.** Final complete own-PID/date capture20validrecords, start/connected/stopped+original terminal, matching-malformed0, safe fields. `24-final-default-sink.log`, diagnostics-b7ff2e11-30f9-41e4-80e9-776772bfc791. Earlier18-default-sink-red.log was named for a planned red assertion but actually EXIT0/PASS, NOT red evidence. |

### Exact terminal / correlation examples

- Final precommit signal operation **dc319ab4-1696-4b85-88a3-d6250b2e43ed**, session
  **cdd895a1-5f34-4261-8f90-7282c58439c5**: original job terminal199ms/cancelled/cleanuptrue;
  host stopped344ms. No cross-process clock subtraction is used.
- Partial signal operation **a8277bb9-5c55-456c-8464-8065f5cca652**, session
  **4528a347-6902-40fe-a4c3-97229fa22518**: terminal181ms/cancelled_with_committed_output/
  cleanuptrue, host stopped311ms. Source/helper were exclusive-denied before AND at cleanup barrier.
- Ordered disconnect operation **8ae9e7cc-64fb-42d8-b5b2-3e86db8c4f93**, session
  **fc20cc7f-3e08-4137-8238-2532db49141c**: host actual terminal209ms/disconnected/cleanuptrue,
  stopped354ms; public CLI correctly retained its unavailable remote-cleanup facts, not that terminal.
- Final default diagnostics session **d6bcf1e1-256b-49b4-be79-82307d9c4685**, operation
  **e486350a-be0b-4077-b586-c29e68dda24d**: start0ms/connected61ms, original terminal164ms,
  stopped303ms/native_error_code=null. Null means no native error supplied, not a fabricated code0.

### Diagnostics observation correction — no product repair

Initial tail-only/default-sink own-PID capture found0records. A fixture-only stderr observation
recorded0sink-write failures; it did NOT establish a write exception or causal shared-file corruption.
Complete bounded own-PID/date assertions then captured20validrecords on two actual default calls.
The earlier conversational statement calling this a reproduced logging defect was retracted before
any product edit. Initial no-capture observations12/17 and successful18/24 remain, no green-hunting
full product suite. Shared historical malformed/contended logs were not removed or repaired. Do not
claim universal concurrent-sink reliability or that an absent tail record proved sink failure.

## Ownership / cleanup final facts

Every measured signal case checked the real process tree and exact scratch image before signal.
Before cleanup, source/helper exclusive opening fails native32; while cleanup checkpoint held the
CLI still has no JSON. After release, real terminal and original child exits precede exclusive
read-write/source/helper reopening. Precommit partial is removed by original output owner; committed
output is retained. Source hashes remain equal. Final observed helper before/after hashes match the
approved worker, and all retained case helper images match it. No user/foreign partial is removed.

The timeout fixture uses original production ArchiveReadHelper/held-image lease via original
ForBrowserEntry runner construction, NOT the historical unleased test-apphost ForPackage shortcut.
It runs original legacy TEST on real worker/native Jobs; no public CLI is involved. Both original
process/Job/pumps reached their cleanup checkpoint while leases/slots remained rooted. The original
runner's JobEmpty predicate is part of that reached boundary; no arbitrary external Job proof is
claimed. After release2checkpoints return,0timeouts/0failedassertions,11originalevents,active0.
It removes only its own unchanged staged helper after actual drain and retains all sample/evidence.

Final process observation found0executables under this round and0remaining recorded own process IDs.
No same-name process termination, global signal, current terminal detach/attach, installed Desktop,
service/bootstrap/Registry/COM/GUI/Explorer action or user-data/recursive cleanup was performed.
The isolated console driver receives the same confined event but its own handler suppresses it;
only the real CLI performs product cancellation. Parent/worker/host killing is exact owned-process
fixture cleanup only, never claimed as cooperative product success.

## Remaining NOT RUN / unaccepted boundaries

- User physical Ctrl+C keypress: **NOT RUN**. Native CTRL_C_EVENT evidence is not physical input.
- Uninstrumented production host under the same exact checkpoint timings: **NOT RUN**; controlled
  host production sessions/runner/worker are stronger than a helper unit test but not the same claim.
- Parent MonitorAsync branch independently winning: **NOT RUN**. Actual terminated-parent logs
  show EOF won and stopped the originals. Do not claim both paths independently proved.
- Public CLI prolonged cancellation-drain timeout / inherited session DisposeAsync Drain(false)
  false-success reproduction: **NOT RUN / not reproduced**. The requested short owner gate passed;
  no reason to modify whole protocol/disposal. Irrecoverable OS/pump/Job non-drain is not synthesized.
- Diagnostic throwing/full/blocked sink and broad default shared-sink reliability: not exercised
  here; scoped real sink capture passed without changing feature result. No failure event was
  fabricated when actual host ended normally via EOF/stopped and operation terminal.
- Real Explorer/OLE/foreign COM/DPI/UIA and existing partial drag content confirmation remain as
  recorded. P206/P207live/ProductApplyAuthorized=false and priorA–E/JOB07/SAFE08 debts stay open.

## Checks, preservation and stop

C# host/test builds0warnings/errors; exact new cleanup LIST/RUN/PASS1 verified. Python AST and
PowerShell parser checks PASS. Existing source architecture16assertions and spec-contracts111docs/
14owners PASS, scoped diff whitespace PASS. No Rust product edits, so no repeated CLI/ordinary tests
or strict/workspace matrix; matching-worker feature/build was the only Cargo activity.
Product source baselines for CLI execution, host/Program, Read/JobSession, runner, PlatformLog and
checked-in Generated outputs are identical. Historical archive-cli-acceptance/outcomes were retained.
80/80 observed packaged files+setup hashes unchanged; exact installed26 preservation is checked by
read-only identity only, not maintenance or runtime launch. Concurrent existing changes not attributed
to this round; no commit/push/package/sign/install/dependency/vendor changes.

**STOP: archive CLI lifecycle slice pending independent acceptance.** No fullP208/PhaseF/first-release
completion, no automatic packaging, ordinary-command rerun or next feature progression.
