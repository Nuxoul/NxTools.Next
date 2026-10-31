# Archive source authorization and isolated recycle boundary

Evidence: `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveSourceDeletion.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveSourceLease.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveCreationSources.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs`.
Private session evidence: `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.SourceDeletion.cs`.

## Scenario: One original operation, one finite source capability, independent result

### 1. Scope / Trigger
Phase A internal foundation. Production native backend is deliberately absent: strict original
object/version binding through public Shell recycle is unproved. No real recycling is authorized
or executed. Read [Rust owner](../nxtools-archive/backend/source-deletion.md) and the task native
capability research before changing source lease sharing, binding or adding a Shell adapter.

### 2. Signatures
`ArchiveWorkerRequest` adds host-only `bool? DeleteSource=null`, `bool ActiveTemporarySource=false`.
Admission preserves immutable scalar/source snapshots. Private delete_source is packed only when
specified; old requests never acquire default delete authority. `ArchiveWorkerEvent.SourceDeletion`
is separate from archive Category/FilesCommitted/CommitState. The private nested RecycleSession
is captured only by original Run at source_acquire, binding original source capabilities/backend.
Only that Run's real terminal owner may Complete/ExecuteOwnedAsync/Retire it. The observable
IArchiveRecycleAttempt accepts only operation/token and cannot start a backend (owner_required).

### 3. Contracts
- Source files keep original READ-only sharing and no-delete-sharing ancestors. Never release or
  widen protection just to make Shell work. Single source remembers held full identity/basic
  version; tree roots retain full128ID/volume/size/mtime/ChangeTime. Only original regular roots
  become capabilities; directory whole-root deletion remains refused, not descendant semantics.
- Resolve only after actual worker/pipe drain, before source resources dispose. Require full archive
  success, explicit consent, expected operation/source IDs and source-version recheck. Reject
  active temporary source provenance. No new worker-supplied arbitrary path/parent authority.
- Captured -> Qualified -> InFlight -> Completed -> Retired;or Captured -> Refused -> Retired.
  All transitions are session-locked;qualification once,execution once,retire irreversible.Capture fixed original
  source objects/version before acknowledging source acquisition; terminal never reopens/replaces
  its source set. No static Qualify(request,evidence,true,newSources), weak request-key table,
  public factory, request clone or global unbounded spent-ID set can mint authority. Original
  private Run identity, not request reference equality, owns completion. The observer cannot
  requalify/start execution or replace backend/sources; wrong job/token never consumes grant.
  Await actual isolated execution, then retire before lease disposal/slot retirement; a retained
  observer after drain always refuses. No timeout replacement or early resource release.
- Dedicated STA isolated execution max2 global slots, no queued replacement workers. Runner retains
  original operation slot/leases until actual backend terminal. Cancellation before a source
  starts -> NotAttempted; native/backend cancellation result -> Cancelled. In-flight timeout or
  lost acknowledgement cannot be treated as interruption; exception after dispatch -> OutcomeUnknown,
  stop remaining sources and never retry. No permanent-delete or fallback API exists in this seam.
- Recycled/Cancelled/Unsupported/Failed/OutcomeUnknown/NotAttempted are per-source outcomes. Partial
  results and native errors never overwrite successful archive category/count/state or delete/
  roll back the output. Production null backend -> native_protection_unproven + NotAttempted.
- ForRecycleFixture is internal and requires current test apphost; ForPackage has no native backend.
  This is an injection seam for isolation, not installed Shell support or a public delete command.
- Logs reuse operation correlation, qualification/accepted/start/terminal/elapsed, fixed reasons,
  bounded counts and native code. No path/name/content/secret/token/raw exception. Sink failures
  cannot affect archive/recycle result. No byte/member/pointer event stream.

### 4. Validation & Error Matrix
Absent/false -> consent_missing/zero backend calls; actual lifecycle failure ->
archive_not_fully_succeeded; absent worker fact -> worker_evidence_missing; wrong operation/shape ->
invalid_evidence; arbitrary/missing/duplicate IDs -> source_set_mismatch; changed/unavailable ->
source_changed/source_unavailable; temp -> active_temporary_source; repeated completion ->
already_qualified; observer before completion -> not_qualified,after qualification -> owner_required;
wrong token/job -> authorization_mismatch; consumed -> already_consumed; retired -> retired;
no native contract -> native_protection_unproven; occupied STA slots -> execution_busy/NotAttempted;
backend exception -> OutcomeUnknown, no retry. Real native codes only from their owning boundary.

### 5. Good/Base/Bad Cases
Good: completed native output remains successful when recording backend returns failure5; sources
remain leased until actual isolated terminal. Base: old request never dispatches. Bad: final-ID check
-> close -> Shell path/PIDL recycle asserted identity-safe, or FOF_ALLOWUNDO treated never-permanent.

### 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveSourceDeletionTests.cs` uses actual Start/Job/Rust/
native extraction and creation before production qualification. Keep consent/default, directory/
exclusion/strip-empty, active temp, RAR4/5+7z, metadata/MOTW/directory/postcommit/unknown/cancel,
exact mixed per-source outcomes,once/concurrent/late/cross-job/token/ID rejection,source lease
write refusal during backend,success-output preservation and diagnostic privacy/throwing sink.
OnceOnlyUnknownAndBinding must use the actual captured original production session,never a new
lease plus caller-supplied true. RetiredEvidenceCannotBindReplacement performs real Start/drain,
preserves original aside,replaces source,opens new lease,replays old JSON/request clone against
original retired observer;zero extra backend calls,all original/replacement/output/foreignpartial
bytes retained. OriginalSessionRejectsChangedEvidence corrupts only real worker evidence at the
test-apphost input seam;original session refuses job/source/duplicate/extra-field tampering.
No-Shell sharing32/name-resolution test is capability evidence,not recycling. Preserve original
117cases union addedfixtures,LIST/RUN/PASSnames/counts exact,largeZIP64once final;current counts
only in unique checkpoint. Removed old static interface compilingFAIL is not behavior acceptance.

### 7. Wrong vs Correct
Wrong: recycling failed -> permanent delete, or ack timeout -> repeat same source.
Correct: preserve strict lease/native contract -> retain source with typed independent warning;
authorized isolation consumes once, records exact outcomes and preserves successful archive output.
