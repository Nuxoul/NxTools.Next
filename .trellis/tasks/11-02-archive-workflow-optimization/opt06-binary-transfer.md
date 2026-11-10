# OPT-06 — Bounded binary bulk transfer

## Scope and status

Implementation and scoped automated/owned-native verification pass under **Phase E — Format expansion and ZIP editing**; [independent acceptance](opt06-independent-acceptance.md) now accepts this slice. OPT-02/03/04/05 retain independent scoped acceptance; O5-AC01/02 remain closed. This slice covers Create/Add ID-source reads, Extract/retained output writes and held-partial readback through the original worker pipes/Run/native owners. No format, public IPC, Job runtime, scheduler, source authority or UI change is introduced. G-00 remains incomplete. Stop at OPT-06; OPT-07 is not started.

Reference chain: [archive reference](../../../docs/reference-code/archive.md#opt-06-bounded-binary-bulk-transfer). Requirements: PERF-07 / MAP-10; principal exit G-14, preserving touched G-01/G-02/G-04/G-07/G-19/G-23/G-25.

## Private frame contract — version 1

### Negotiation

Modern schema2 native-data operations (`extract`, `create_*`, `modify_*`) request `bulk_transfer:1` in the original start JSON. The accepted capability array must contain the exact string `binary_bulk_v1`. Missing, duplicate/malformed capability or an unsupported transfer version is rejected before source payload reads or output mutation; there is no automatic JSON fallback. The same request/capability supplies mutual version agreement, and the original `source_acquire` acknowledgement remains the source-authority boundary.

Schema1 ZIP, read-only open/test and explicitly separate HZ5 reclamation retain their original JSON contracts. They do not negotiate bulk or gain ordinary-source/edit/volume/reclamation authority from it. Existing legacy direct fixture peers must be migrated explicitly when testing modern data operations, not silently downgraded by production.

### Envelope and discriminator

All records retain the original four-byte little-endian prefix. A clear high bit is the existing UTF-8 typed JSON length `1..1,048,576`. A set high bit denotes a binary record; its low31bits are exactly `76 + payload_length`. Binary body size is `76..65,612`, admitted before allocation. No public PlatformHost framing changes.

Binary fixed header (offsets are body-relative):

| Offset | Size | Field |
| ---: | ---: | --- |
| 0 | 1 | Version, exactly1 |
| 1 | 1 | Kind:1 output write,2 source-read reply,3 held-partial reply |
| 2 | 2 | Reserved, zero |
| 4 | 16 | Original operation UUID, RFC4122/network byte order |
| 20 | 16 | Exact call UUID, RFC4122/network byte order |
| 36 | 16 | Exact source/partial token UUID, RFC4122/network byte order |
| 52 | 8 | Positive bulk sequence, little-endian u64 |
| 60 | 8 | Byte offset, little-endian u64 |
| 68 | 4 | Requested length, little-endian u32,1..65,536 |
| 72 | 4 | Actual payload length, little-endian u32,0..requested length |
| 76 | actual length | Raw bytes |

Operation/call/token must be nonempty and match their original owner. No filename/path/secret appears in the binary header. Write records travel worker -> host, kind1, actual length equals requested length and is positive. Read replies travel host -> worker, kind2/3; short reads are valid and zero bytes is EOF. Actual source-size and codec/candidate checks retain their original refusal behavior.

### Requests, acknowledgements and ordering

Source/partial read requests remain typed `native_call` JSON with original call ID/action/token/offset/length plus positive `sequence`. A successful reply is exactly one matching binary record; errors remain typed `native_result` JSON with exact call/token/sequence/offset/length and original error/native code. Output writes are one binary record; their acknowledgement remains typed `native_result` JSON with the same binding. A short native write is failure, never a successful acknowledgement.

One original operation-wide bulk sequence starts at1 and increases for every source read, output write or partial read. It never restarts for a new member, token, format attempt or retained demand. Repeated/old/future/mismatched sequence, wrong direction/kind, operation/call/token/offset/requested length, duplicate reply or extra data is refused. Only the current pending binding can complete the one existing rendezvous. Native token admission and per-token written offset remain original-authority checks.

The transfer window remains one acknowledged call. This intentionally avoids speculative pipelining and an extra request map while removing data conversion. The producer cannot issue metadata/source close/finish/commit/member-ready before its prior data acknowledgement; the original native reader/writer executes those actions in order. Transport drain is not codec integrity, `FlushFileBuffers`, held-partial validation, publication, Seal or Run cleanup.

### Buffers, budgets and fairness

Each payload is at most64KiB. Each active direction retains at most two transport payload buffers, including any queued envelope copy; fixed headers are separate. A fixed1MiB transport reservation (four64KiB buffers plus bounded header/token state) is charged within the existing64MiB runtime/IPC overhead, not added to codec/work/metadata/global process ceilings. No file-sized Vec, plaintext-source spool, global cache, unbounded map or concurrency/deadline increase is introduced. Existing codec/output buffers keep their own reservations.

Original Rust control reader and C# stdout/input owners remain unique. The original bounded host input channel may queue controls, but at most one payload reply can be outstanding. At every acknowledged64KiB boundary Rust checks the original cancellation/pause state; control can be processed while the work thread waits. Progress may be interleaved only at complete record boundaries, never inside a binary header/payload. The existing RAR callback retains its buffer until acknowledged output, or a bounded owned copy has taken responsibility.

### Teardown and failure precedence

EOF, malformed frame, cancellation, native failure and transport loss retire the exact pending binding and wake waiters. Owned uncommitted buffers are zeroed/retired. Host input cancellation stops queued/blocked writes through the original token; uncooperative synchronous pipe/native work stays subject to the original cooperative grace/owned-Job termination and actual pump/process drain. No replacement process or released reservation is inferred from logical cancellation.

Preserve the first true failure and original native error. Native committed/unknown/postpublication facts outrank late cancellation. The existing source freshness, CRC/MAC/actual size/codec end, metadata/MOTW, candidate readback, reader completion, FILE-only handoff, postpublication verification, retained member-ready/Seal and actual cleanup boundaries are unchanged. Transferred streams and preview consumer shares retain OPT-04 lifetimes. O5-AC01 role-separated terminal counters remain intact.

## Diagnostics and measurement contract

Use original operation correlation and low-frequency terminal observations only. Record actual source/output/partial payload bytes/calls, complete JSON/binary wire bytes/frames, conversion/copy work whose owner is observed, transport buffer high-water and operation/actual cleanup timing where measurable. Missing process/internal I/O, exclusive codec time and exact memory peaks remain null/unavailable. No paths, members, passwords, tokens or raw bytes enter production logs.

Freeze current source/worker/Host before changing transport. Principal comparable routes use three completed trials, identical deterministic inputs, Debug build, same disk/route/stable execution paths and no concurrent build/cache flush/best-run choice. Separate transport savings from already accepted source/plan savings. No universal speedup or reference equivalence is inferred.

## Verification seams

1. Actual Rust worker/private pipe peer: negotiation/refusal and exact raw frames, lengths, identity/sequence/range, short reads/EOF, duplicate/stale replies, backpressure/cancel/control fairness and ordered metadata/commit.
2. C# original parser/Run/source/output: independent byte-literal frame contract, truncated/short IO, original native token/range refusal, bounded buffers and queued control/data order; effects-reviewed actual Job/native outputs and actual cleanup.
3. Actual Create/Add source reads, extraction/retained writes and candidate readback: independently decoded exact output, source preservation, complete integrity, one publication/journal, actual Run cleanup/exclusive reopen and current terminal diagnostics. Native execution is exact LIST then identical RUN; zero selection/skip is not acceptance.

RED/GREEN evidence, exact selections/counts, frozen pair attribution and three-trial results are recorded as executed, not inferred from compilation or source checks. Implementation verification is not independent acceptance.

## Executed implementation result

The [verification/independent handoff](opt06-acceptance.md) records effective Rust/C# RED/GREEN, exact selected/run/pass/skip/not_run, native lifecycle preservation and frozen source/pair attribution. Current Rust worker8groups/35process scenarios, worker-related units28, directed integration95, C# wire/legacy13 and affected actual-native13 all pass at their stated boundaries. Final C#build0warnings/0errors, strict archive all-target Clippy, scoped rustfmt37 and applicable source/document gates pass. One bounded correctness review found no actionable defect.

[Comparable measurements](opt06-measurements.md) retain18complete samples with exact original native/payload work and actual cleanup. ZIP Store Create/Extract/Add complete-operation medians are691.335/542.544/1260.297ms before and183.681/246.609/495.964ms after. Bidirectional wire medians fall6,789,553->3,172,163;3,755,971->1,059,473;12,054,485->5,280,000bytes. Host high-water is2buffers/131072bytes for Create/Add and1/65536for Extract; modern conversion input is0. Full ranges/raw samples and null unavailable metrics remain explicit. These results do not close G-00, complete G-04/Phase E or physical/Release gates, and do not assert independent OPT-06 acceptance.
