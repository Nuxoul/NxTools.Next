# Product read100k — explicit finite read v2 contract

Canonical **Phase A — Engine contracts and safety foundation**, in progress. User-authorized
headless open/test only. No UI/FIFO/write/deployment/recycle, dependency or third-party changes.
Historical read/resource reports and results are immutable; current evidence is recorded separately.

## Smallest gap and implementation owners

Current product owner rejects budgets above4096entries/4MiB. Private/runner v1 emits2entries per
batch:100k requires50000business ACK roundtrips, and legacy runner cumulative stdout8MiB stops
before complete enumeration. Change existing Protocol DTO validation, Archive product reducer and
worker metadata emitter, C# read session/metadata/runner only. Reuse original IPC router, credit,
Job/source/helper lease and cleanup-bound reservations; no new queue/process runtime. Tests drive
actual ArchiveReadClient -> typed pipe -> production ArchiveReadSession -> original runner/Job ->
actual Rust worker. No layout/focus/window lifecycle is introduced.

## Budget table (bytes, not observed RSS)

| Layer | Legacy/default read v1 | Explicit read v2 maximum / observation |
| --- | --- | --- |
| Engine members/model metadata |4096 /4MiB |100000 /128MiB; exact explicit finite request, no auto escalation |
| Engine working bytes |64MiB default,512MiB max |unchanged64MiB minimum/512MiB maximum |
| Worker/Job process |work+metadata+64MiB |same formula, <=1GiB/process and original max2/aggregate2GiB;100k fixture64+128+64=256MiB |
| Private single frame |1MiB |unchanged1MiB, checked before allocation |
| Metadata batch |2entries /512KiB |<=256entries AND <=256KiB actual serialized private frame and business batch |
| Private cumulative bytes |8MiB |explicit max_transport_bytes1..64MiB, default8MiB; includes every u32LE header+JSON frame; larger limit activated ONLY after exact v2 negotiation |
| Product cumulative received/retained serialized metadata |<=4MiB |explicit max_retained_bytes<=64MiB; separate monotonically cumulative counter, never decremented by ACK |
| In-flight/transport queues |one unacknowledged batch; IPC events8; C# events8 |unchanged; slow consumer receives no next batch before exact ACK |
| Commands/cancel/concurrency |existing finite command queue;2read reservations/cancel records; wake1 |unchanged start-before-cancel/coalescing/fairness; reservations until actual cleanup/connection retirement |
| Deadlines |runner30s; ACK200..10000ms |unchanged; no timeout increase to hide transport slowness |

Cumulative transfer is not simultaneous memory retention. Engine keeps its bounded immutable
snapshot; emitter builds only one bounded batch (not another full Vec/100k JSON); C# and Rust
move bounded batches and retain counters, not a second full archive. Caller retention remains
charged cumulatively even after take_batch/ACK; ACK is not a claim of historical memory release.
Batch size256 reduces short-name100k to391batches rather than50000; byte limit may yield more.
Actual manifest/batch/cumulative measurements must be recorded, not inferred from this estimate.

## New/old negotiation matrix

- Outer envelope remains1. Existing ArchiveRead version1 and absent old fields keep their exact
  limits/serialization/2entry batches/8MiB runner path. Default request above limits fails typed.
- Version2 is explicit per operation, requires explicit limits, accepts only open/test, never
  extract/create/password/delete. Limits additionally expose max_transport_bytes (default8MiB,
  omitted in legacy serialization); v1 forbids a nondefault new transport allowance.
- Production starts with private schema2/read_contract1 and conservative legacy-shaped limits,
  without read_transport, so strict old workers can parse before reporting their actual capabilities.
  New workers additionally advertise product_read_v2/bounded_metadata_v2 in accepted. Only after
  verifying those plus all v1read/credit/source/budget capabilities does C# send ONE read_upgrade
  on the original control pipe, with finite five engine limits and transport{max_total_bytes}.
  FIFO delivery precedes original source_acquire acknowledgement; worker applies/seals upgrade
  after source lease acknowledgement and BEFORE engine/codec source reads. Duplicate/late or
  invalid upgrades cancel; never fallback to large reading with v1budgets or old worker semantics.
  Legacy omission/read_contract1 without upgrade remains unchanged; direct private read_contract2
  is bounded/validated but production does not require an old worker to decode that DTO.
- Old private request omission/read_contract1/native output operations remain compatible. New
  controls retain existing exact sequence/operation identity and private metadata_ack version1;
  public ACK/cancel carries its originating archive version. No producer-enqueue ACK.
- Old worker/read contract => IncompatibleVersion; missing required capability => MissingCapability;
  unknown public version => IncompatibleVersion; malformed/zero/overmax/unauthorized budgets =>
  InvalidRequest. Actual engine admission => ResourceLimit; received/transport quota => MetadataBudget.
- Successful terminal verifies original snapshot/format/contiguous ordinal+sequence, exact count/
  batch totals and bounded cumulative bytes. No successful truncation. v2 complete counters are
  required; legacy optional private counters retain compatibility.

## Preserved lifecycle and diagnostics

Pending-before-send and original connection generation, reliable Drop/local-error cancel intent,
start-before-cancel/duplicate coalescing/fairness, synchronous overflow epoch and ordinary-notification
saturated real EOF remain unchanged. C# process-wide max2roots original Runs until actualcleanup;
source/helper/Job/pumps/slots are not released by receipt/private complete/ACK. Late/duplicate/
retired messages and old generations cannot mutate replacements. Source-delete original run/session
and Rust lifecycle epochs unchanged; native recycle remains native_protection_unproven/NotAttempted.
Production helper stays trusted fixed path, no PATH/env/request override. Low-noise correlated
accepted/start/negotiated/terminal events contain only stage/count/elapsed/real native code, never
member/path/content. No per-batch diagnostic stream.

## Vertical validation order and stop

1. Small actual typed-chain v2 request goes red at current admission before implementation; default
   overlimit remains ResourceLimit. Implement smallest v2 negotiation then same assertion green.
2. Byte/entry/retained/transport edge fixtures and old/missing capability failclosed; original
   credit/cancel/Drop/EOF/generation/overflow and source/helper/Job/slot lifecycle regressions.
3. Independent CPython exact100000entry ZIP+streaming manifest (dirs/Unicode/empty/nonempty); ONE
   successful actual product correctness run merged with ONE bounded Debug resource observation.
   Stream comparisons, no full business Vec. Record exactPIDs/samples/firstbatch/fullterminal/
   batches/cumulative bytes/inflight/cleanup; firstbatch not firstpaint, no1.5s/Release claim.
4. User-requested related integrated Rust/C#/shared ZIP/doc/selector checks serially under required
   jobs1/incremental0/codegen256 and one machine Cargo lane. Build isolated C#apphost first. One
   optional bounded readonly review only; no broad retry-to-green or unrelated cleanup.
5. Save additive evidence, owner/reference/context and unique current checkpoint, exact owned
   resource cleanup/history hashes. Stop; no next feature or completeENG08/PhaseA/A–F claim.

## Verified source follow-up and single review corrections

Legacy tracked ZipArchive/SevenZip/SharpCompress grep again found only package ZIP, not a general
archive listing/IPC owner. Archiver core/src/lib.rs140Entry/494list returns a full Vec, not bounded
credit transport. Next source boundaries above were traced to suspended Job creation, source_acquire
and actual process/pump/Job drainage. No proprietary native engine source is claimed.
One bounded readonly review found DEL JSON escaping, old worker preaccepted strict parsing and
C#unknown-version budget-validation precedence. All received behavior reds before correction;
details/actual final results belong to additive evidence, not older reports. Receiver byte charging
includes C#Unicode/HTML-sensitive/DEL escaping; final C#and Rust serializers still enforce exact
single-batch ceilings. No second review round is authorized or needed for this slice.

## One declared correctness/measurement repeat after concrete transport failure

First actual product100k-v2 attempt stopped, no success claim; all attempt/sample bytes retained.
Source investigation then reproduced a pre-existing original platform-pipe bug deterministically:
read_bounded_frame consumed8192prefix bytes into a future-local Vec; an interrupting select branch
dropped them, and resumed read returned only tail. Red1failed/exit101 -> connection-owned finite
prefix buffer -> IPC30/30pass/exit0. No queue/ACK/process runtime or1MiB limit change.
One repeat ONLY is declared before execution, to verify this concrete repair against the SAME
100kmanifest with unchanged30s product timeout/budgets. Not a best-score or optimization repeat.
The previous stopped attempt remains Incomplete; success cannot retroactively rewrite it.
