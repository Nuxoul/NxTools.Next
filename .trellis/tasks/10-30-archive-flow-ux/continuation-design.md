# Object-scoped source-space reclamation continuation

## Product contract
The archive home has no unconditional continuation button. A cancelled/failed reclamation task exposes a contextual check/continue action after actual cleanup. Reopening a source with recovery records exposes recovery on that source's failed browser surface. Neither entry opens an unrelated blank form or asks for a UUID. No directory scan, new task history, automatic destructive continuation or new format support.

Continuation presents the exact source, original target when identified, concise irreversible warning and explicit Start/Cancel. A missing original target requires selecting that original folder, not inventing a destination. Restored recovery ID and original copy_motw are internal native facts. The actual Continue still uses a new original Job Start/session and the existing full source/root/companion/output/structure verification. Reading a record does not certify resumability or grant write authority.

## Smallest behavior gap and owners
Original ArchiveApplication/Jobs already retains source, destination and recovery intent for live task records. The existing home opens OperationForm with an empty source and empty recovery ID; restart continuation lacks a read-only native identity lookup. Keep ArchiveApplication as the single async/pending owner and OperationForm/ArchiveBrowser/JobProgressView as original views.

## Read-only native inspection wire
New additive PlatformCommand `archive_recovery_inspect`, ordinary correlated CommandResult response. No new worker/job action or changes to persistent schema.

`ArchiveRecoveryInspectCommand { version:1, operation_id:UUID, session_id:UUID, generation:u64, source:String, destination:Option<String> }`.
`ArchiveRecoveryInspection { version:1, operation_id:UUID, session_id:UUID, generation:u64, recovery_id:UUID, source_identity:String, destination:Option<String>, copy_motw:bool, completed:bool }`.

All IDs nonnil, generation nonzero, paths use original archive path grammar; destination is an observed matching original root, not an arbitrary journal-provided path. None means the original target is not identified and the UI must request it. With no candidate destination, the source parent may be returned only when its held native identity equals recorded root_identity. With a supplied candidate, return it only when its held identity equals recorded root_identity; otherwise destination=None. Never derive a new output location.

C# reads the source ADS marker and derived companion with original held read-only source/parent primitives. Marker<=1024bytes; existing journal<=32MiB, frames<=64KiB with strict framing/schema/sequence/known record kinds. Match source identity including logical length, marker/companion IDs and header recovery/source/parent/companion identities. Recognize a well-framed completed_consumed record as a recorded completed state, not independently verified current output. No payload scan, digest inventory, output creation, writable opens, append, repair, truncate, punch or change to existing recovery execution. Default ordinary source leases still refuse recovery-bearing sources. Native blocking work is bounded/detached from the pipe reader; a hung operation retains its native slot until actual return.

Fixed error reasons distinguish absent records, source/record mismatch or corruption, missing companion, busy/access denied and disconnected/timeout. Diagnostics `archive.recovery.inspect.accepted/finished` carry original operation correlation, fixed outcome/native error/elapsed only; no paths, contents or record data.

## Rust/UI integration
- Jobs keeps bounded pending inspections tied to original form/browser ID, generation, source/candidate target and connection; install before dispatch. Retire on close/supersede/disconnect/deadline and ignore old receipts.
- Contextual task action rechecks exact job/attempt/generation, reclamation kind and terminal cleanup before opening the source-bound continuation form.
- Top-level failed/rejected reads with actual cleanup may issue one bounded record inspection. Missing records retain the original read failure; cancelled/disconnected/nested reads do not probe. Current source/browser generation alone may accept a found record and show source-local recovery. Completed records have no Continue action.
- Continue form has no source chooser, UUID field, ordinary range/conflict/arrangement editing or ordinary capacity query. Source is fixed. Confirmed original target is readonly; unresolved target uses the original directory picker then reinspection. Original open-after/keep-open preferences remain optional; no remember-default operation. Pending/failure/completed/missing target states keep Cancel reachable and Start disabled when not ready.
- Kit0.8.0 Form/Field, readonly InputGroup, original Button Medium outline/default submit and Small retry, DescriptionList and concise warning Alert. Compose outside Toolbar.child to preserve outline. No new controller/Root/theme, no motion and all cross-owner writes deferred.

## Source/reference boundary
Rechecked original HZ5 design/wire/independent acceptance, reclamation owner contract, archive/reference and HatchZip in-place notes. Actual native chain is ArchiveSourceLease.OpenCore -> original held parent/file handles -> ArchiveReclamationNative.OpenMarker/Identity -> ArchiveReclamationSession header/framing. Existing execution remains ReserveActivity -> suspended original worker/Job -> native source/output acquisition -> OpenContinuation/Load -> Rust plan/output verification and original ready/pending/apply order. Native header stores root identity, not its path; no inferred full target path or persistent-schema migration is permitted. No legacy NxTools general archive owner exists in the verified reference.

## Verification
Add red-capable production-view and real application/IPC tests for absent global entry/UUID, exact-source task continuation/new Start, current/stale/closed/disconnect and no auto-submit, readonly target/missing-target picker, found/completed/absent recovery rendering. Native read-only fixtures create synthetic marker/journal data only: matching identity/root, wrong or missing authority, corrupt/torn/oversized records, completion, concurrency/terminal diagnostics and byte/identity preservation. Do not run existing destructive punch/process-kill matrices or use user originals. Scoped Rust/Protocol tests, strict lint, C# build/exact safe fixture filters, release/editor/spec gates and actual isolated production-window inspection are separate evidence levels.
