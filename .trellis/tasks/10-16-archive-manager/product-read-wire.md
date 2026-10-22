# Product read v1 exact wire implementation ledger

Implementation supplement to product-read-contract.md. No acceptance claim.

Platform command payload: `{"command":"archive_read","data":ArchiveReadCommand}`.
Command fields (snake_case, unknown fields rejected):
- version:u32 required1, operation_id:UUID, session_id:UUID, generation:u64>0
- action:string open|test|ack|cancel
- path:optional string (required only open/test, absolute local Windows source)
- codepage:u32 default0; allowed0/65001/936/950/932/949/437/1250/1251/1252
- limits:optional ArchiveReadLimits, defaults when absent only for open/test
- sequence:optional u64, required only ack
No extra fields. Controls cannot carry path/limits/nonzero codepage. No password support here.
Envelope.correlation_id=operation_id; distinct caller-supplied command envelope.id.

ArchiveReadLimits fields all default individually only when absent; reject unknown:
max_entries:u64=4096 (1..4096), max_metadata_bytes:u64=4194304 (1..4194304),
max_working_bytes:u64=67108864 (67108864..536870912),
max_entry_bytes:u64=34359738368 (>0), max_output_bytes:u64=107374182400 (>0),
max_retained_bytes:u64=4194304 (1..4194304), ack_timeout_ms:u32=2000 (200..10000).
Existing worker limits receives first five fields only. Existing runner30s timeout unchanged.
Max serialized command256KiB. All keys/versions required, no default version1.

Platform event payload: `{"event":"archive_read","data":ArchiveReadEvent}`.
ArchiveReadEvent common required version:u32, operation_id,session_id,generation;
kind:string accepted|negotiated|batch|terminal; capabilities:Vec<string> default[],
batch:optional ArchiveReadBatch, terminal:optional ArchiveReadTerminal.
Negotiated capabilities must contain product_read_v1,metadata_credit_v1,read_open_v1,
read_test_v1. Unknown capabilities grant nothing and known list projected only.

ArchiveReadBatch: snapshot_id:UUID (actual engine/codec session),sequence:u64 starts0,
format:string (existing worker Pascal names Zip/SevenZ/Rar4/... supported formats),
entries:Vec<ArchiveReadEntry> (1..2). Batch serialized<=524288 bytes.
ArchiveReadEntry: ordinal:u64, name:string, kind:string File|Directory|Link|Special,
size:optional u64,packed_size:optional u64,encrypted:optional bool,
warning_codes:Vec<string>,provenance:string Utf8Flag|UnicodeExtra|FormatUnicode|Explicit|
UncertainCp437,codepage:optional u32 (Explicit uses selected encoding, UncertainCp437437;
Unicode provenance65001). No raw content/native token/ADS/ordinary handle.
Total retained serialized batch bytes<=max_retained_bytes, entries<=max_entries.
Consumer verifies contiguous original ordinals starting0 and same snapshot/format, ack only
exact current sequence after bounded retention. Terminal verifies total counts/batches.

ArchiveReadTerminal: status:string succeeded|rejected|failed|cancelled|disconnected;
code:optional ArchiveReadError, files_verified:u64,bytes_verified:u64,
entry_count:u64,batch_count:u64,elapsed_ms:u64,native_error_code:optional i32,
cleanup_complete:bool.
ArchiveReadError snake_case exact:
invalid_request,helper_missing,helper_untrusted,incompatible_version,missing_capability,
unsupported_operation,unsupported_format,unsupported_method,unsupported_encryption,
password_required,wrong_password,authentication_or_data_error,corrupt_archive,missing_volume,
unsafe_path,output_collision,resource_limit,source_unavailable,source_changed,busy,cancelled,
disconnected,worker_crashed,protocol_violation,consumer_too_slow,consumer_queue_full,
metadata_budget,io_failure,cleanup_incomplete.
Accepted event follows actual runner admission, not success. Rejected before runner start
sends one terminal with status rejected and cleanup_complete true. Duplicate start/control
returns standard command result rejection but MUST NOT send another operation terminal.
Standard fast platform_command_result uses success admission boolean,error fixed code or null,
data JSON-encoded ArchiveReadReceipt{accepted:bool,code:optional ArchiveReadError}; request ID
correlation remains standard. Every valid command gets a receipt. Terminal alone closes owner.

Private worker opt-in field `read_contract:1`, schema2; only open/test permitted.
Worker accepted adds read_contract:1 and the four capabilities (in addition to existing).
Missing read_contract in worker accepted=IncompatibleVersion, missing flags=MissingCapability.
WorkerFrame adds optional snapshot_id:UUID,batch_sequence:u64; WorkerEntry adds optional
provenance/codepage (legacy default compatibility). Normal/isolated codec open use actual
session; every product batch supplies these. Metadata flow uses NEW control op metadata_ack,
version1,operation_id,batch_sequence. It is NOT native_result/output ack. Worker installs
one sequence wait BEFORE writing batch, wakes on exact ack/cancel/EOF. Existing ReplyPort may
carry an independent metadata-credit state internally but cannot confuse native call IDs with
metadata seq; tests must observe business consumer pause, not output-call rendezvous.
Runner product read frame forwarding must wait asynchronously on exact business ack before
reading/emitting next private batch, cancellable and timed; no serial reader wait. Runner
forwards metadata_ack to worker ONLY on business consumer exact ack. Existing stdout8MiB and
frame1MiB ceilings unchanged. Preserve old request omission and all119existing fixtures.
Opt-in complete now additionally emits confirmed entry_count/batch_count; absent legacy private
counters remain compatible while open files_verified and observed contiguous batches are checked.
The existing runner owns a process-wide max2read reservation across pipe-session replacement,
released only after real cleanup; unconfirmed originals/leases stay rooted and block replacement.

Isolation host entry in C# TEST apphost only:
`--archive-product-host <unique-pipe-name> <absolute-owned-package-root>`.
Connect NamedPipeClientStream to Rust run_platform_server, send ordinary Hello client_name
platform_host with archive_read_v1, get HelloAck; read bounded envelopes. Use SAME production
ArchiveReadSession command dispatch/result/event translation as PlatformPipeClient; no normal
WindowsPlatformAdapter construction/global hooks. A guarded constructor fixes only owned root's
NxTools.ArchiveWorker.exe, no env search. Host stops session/drains on EOF. Fixture can use
controlled copied current test apphost for old-version/missing-capability/crash ONLY via
separate guarded fixture factory; actual positive uses real Rust worker.
Before wiring: shared dispatcher stub returns missing_capability and rejected terminal.
Parent Rust integration test runs product owner -> pipe -> this stub, expected literal open
must FAIL. Save red exit/log; parent creates scratch archive-product-red-ready marker; then
C# implementation may replace stub. This preserves a real product call-path red, not compile red.

## Additive readv2 finite directory contract

The v1ledger above remains the legacy/default contract, not an assertion that its ceilings were
globally raised. [Readv2 exact budget/compatibility ledger](product-100k-contract.md) adds explicit
version2/required limits, max_transport_bytes and bounded256entry/256KiBmetadata. No UI/write.
Production uses legacy-compatible read_contract1bootstrap, actual v2capabilities, then one private
read_upgrade{version:1,operation_id,upgrade:{limits:<five engine fields>,transport:{max_total_bytes}}}
before source_acquire acknowledgement. Worker seals/applies at that native source boundary.
Public ACK/cancel/event version follows original request. Successful v2terminal additionally
requires transfer{private_bytes,received_bytes,max_batch_entries,max_batch_bytes,
max_inflight_batches,process_budget_bytes}; v1omits it. Counts and budgets must all reconcile.
Current evidence/results are additive; old refusals/reports remain original.
