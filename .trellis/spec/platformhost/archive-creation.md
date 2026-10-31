# Archive creation / held sources and partial read-back

## Scenario: Gzip/XZ worker creation over native output capabilities
### 1. Scope / Trigger
Required Phase A single regular source→Gzip or XZ; bounded ZIP source-tree creation is described below. Other required create formats remain task work. Evidence: `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerOutputBridge.cs`, `repo:rust/crates/nxtools-archive/src/worker_create.rs`, `repo:rust/crates/nxtools-archive/src/formats/xz.rs`.
### 2. Signatures
ArchiveWorkerRequest operation=create_gzip|create_xz adds optional CreateName/CompressionLevel; schema2start carries create_name(single component) and compression_level=fast|normal|maximum. Existing open/test/extract omit them. Worker accepted must contain source_lease_v1,budget_reservations_v1,partial_read_v1. Native read call uses operation/call/token,offset:u64,length:1..65536; result read_data is base64<=87384chars (decoded<=requested length). One outstanding acknowledged call.
### 3. Contracts
C# validates name/unsupported options and rejects canonical source/output equality before launch; native source lease protects actual namespace/object through cleanup. Output root stays C#. Only create_gzip/create_xz bridge allows read; current own active Preparing token,nonnegative offset<=written count,<=64KiB. ReadPartial flushes managed writer then reads same held handle,no path reopen/write-position change. Published/uncertain/unknown token cannot read. Rust fixed-chunk encode→finish→full partial read-back→CRC/size/SHA256 vs source→source recheck→native flush/single publish. XZ read-back uses bounded anonymous delete-on-close disk spool for preventive footer/index/block/dictionary validation before decode, never whole output Vec. Gzip1/6/9;XZ1/6/7 semantic mappings; XZ reserves16×dictionary+32MiB before construction because locked upstream estimate mixes byte/KiB terms. No source delete,secrets/logs/upstream patch; existing overwrite/TOCTOU unchanged.

Compatibility: additive optional fields are absent from legacy requests. Old worker rejects new operation/fields rather than silently creating unverified data; new host additionally rejects accepted capabilities missing partial_read_v1. Schema1 create is rejected. Actual product PlatformCommand/public IPC is not integrated by this private protocol. No helper/package filename mapping or million-entry/consumer-backpressure acceptance is inferred.

Diagnostics reuse worker accepted/started plus creation_encoding/creation_verifying/creation_committing once each and shared terminal projection. Preserve correlation/elapsed/native/count/state/cleanup; no per-read/chunk events. Sink failure never changes result. Creation payload bytes and readback remain private, never business event Frame.
### 4. Validation & Error Matrix
Unsafe/empty output component→invalid_component/invalid_create_request; password/Store/unknown level→invalid_create_request; same canonical source/output→source_output_collision; missing capability→worker_capability_mismatch. Invalid read range/state/token→read_limit/unknown_token, noncreate read→read_not_supported. Source change→SourceChanged; finite source/output/work→ResourceLimit; corrupted full readback→CorruptArchive and abort/no commit. Cancel before publication cleans own partial; publication/unknown states retain prior exact native policy. Native code only from owning boundary.
### 5. Good/Base/Bad Cases
Good: Gzip independently .NET-decoded, XZ independently explicit CPython lzma-decoded with exact length/SHA256 at three levels; cancellation preserves prior target/source. Base: one regular leased file,no recursion. Bad: encoder.finish alone→publish,mutable-path partial read,schema1 create,unlimited reads or using upstream mixed-unit estimate as a trustworthy cap.
### 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveStreamWorkerTests.cs`: ActualGzipCreation/ActualXzCreation independent decode/three levels/count/state/cleanup; CreationSafetyAndPartialRead covers source-write refusal,cancel/existing/budgets,read bounds/published token and invalid options/self-output. `repo:rust/crates/nxtools-archive/tests/creation_worker_contract.rs` checks actual normal+corrupt readback; worker_wire checks legacy omission/schema1 refusal. Public XZ/TAR.XZ read was subsequently verified separately; see owning current checkpoint. No broad/native-user/release acceptance.
### 7. Wrong vs Correct
Wrong: Rust receives native destination path and calls gzip writer directly; host logs raw native read bytes. Correct: source lease→Rust codec over token writer→same held-partial bounded reader→complete verification→C# flush/publication→one shared log/event terminal.

## Scenario: Bounded source-tree ZIP creation
### 1. Scope / Trigger
ZIP Store/Deflate and AES256 through existing Job/worker/native output,not UI/product IPC. Evidence:`repo:rust/crates/nxtools-archive/src/creation.rs`,`repo:rust/crates/nxtools-archive/src/worker_zip_create.rs`,`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveCreationSources.cs`.
### 2. Signatures
ArchiveWorkerRequest create_zip adds ZipOptions(Method,Encryption,StripSingleRoot,ExcludePatterns),optional host-only SourcePaths. Worker schema2zip_options; requires source_tree_v1/zip_create_v1 plus existing source/budget/partial_read. SupportsCapabilities is the production bounded validator. Source native actions roots/next/open/read/close/check use operation-scoped opaque IDs; metadata/read_data stays private.
create_zip alone accepts an explicit TimeoutMs<=600000;default30000 and other operations<=120000 stay unchanged. This bounded extension followed an actual4GiBfixture120s timeout;no replacement worker or cancellation weakening.
### 3. Contracts
Rust owns traversal/globs/root/names/collision/budgets, C# owns authorized roots/relative no-reparse handles/fullID/size/mtime/ChangeTime/actual reads. <=32root chains/64depth/native65536records,one directory enumerator OR active file plus transient chain; no all-file handles/content Vec/path-read-then-hash. Fixed64KiB enum buffer reserved before enumeration; retained record count charged only for actual members,not EOF. Read<=64KiB from same protected handle,close/final versions rechecked; observed mutation/access/reparse fails. This is not a filesystem snapshot. Reject output inside source directory; exact CreateName-only begin and overwrite authorization. Same held partial readback→bounded anonymous spool→existing original-record central/local/descriptor/extents preflight→exact plan+CRC/AES/SHA256→final sources/cancel→existing flush/publish. Sequential public ZIP writer needs no Seek extension. For directory records use public start_file with trailing slash+Store: new_stream/add_directory omitted descriptor in locked8.6.0. No reader relaxation/upstream patch.
### 4. Validation & Error Matrix
Changed identity/member→SourceChanged; count/metadata/input/output→ResourceLimit; unsafe/reparse→UnsafePath; self/name collisions→OutputCollision; missing capability→worker_capability_mismatch. Incorrect password ambiguous auth remains AuthenticationOrDataError. Publication/unknown/cancel states retain approved atomic-path contract and truthful terminal projection;only owned unpublished partial cleanup.
### 5. Good/Base/Bad Cases
Good: actual Job→native published ZIP→Python/stock7Zip exact list/length/hash. Base: files/directories/multiple roots,UTF8/empty/default exclusions/strip-only-single. Bad: finish-only publication,all-source handles,ordinary path read or self-reader without independent tool;fixed writer directory omission tolerated by external readers.
### 6. Tests Required
Creation plan/worker wire+finish/readback/sourceIO/ZIP64header red-capable boundaries; actual8ZIP creation fixtures including mutations/reparse/precise EOF/count/cancel/overwrite/privacy/output ownership/missing capabilities and4GiB+17FastZIP64 independent streaming hash. Original54C# retained. ZIP64large-count/broad resources remain unverified;exact results only in checkpoint/evidence.
### 7. Wrong vs Correct
Wrong: reserve nonexistent EOF record,encode direct source path,publish without global descriptor preflight. Correct: bounded ID-source plan→held reads/hash→token write/finish→full structure+content readback→source/cancel→existing native publish.

## Scenario: Immutable admission and held-directory namespace
### 1. Scope / Trigger
Every ArchiveWorkerRunner.Start operation, including legacy extract/Gzip/XZ and bounded ZIP. Evidence: `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerAdmission.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerBudget.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveCreationSources.cs`.
### 2. Signatures
ArchiveWorkerAdmission.Capture(request) returns a request with private collection storage and ArchiveAdmittedLimits(max_entries,max_metadata_bytes,max_working_bytes,max_entry_bytes,max_output_bytes). OutputNative.StableDirectoryPath(handle) derives VOLUME_NAME_GUID; NamespacePathProbe is an internal per-run test seam only, never wire authority.
### 3. Contracts
Capture before observable accepted, detailed validation, wire serialization and Run retention. Bound counts/strings/cumulative bytes before copying; read only five scalar budgets, not unlimited arbitrary serialization. Copy SourcePaths, nested exclusions and path/identity confirmations into private owned storage; a read-only wrapper around caller storage is not a snapshot. Validate/serialize/acquire/output/Job budgets use the same immutable request. Reject observed inconsistent captures before acceptance. Keep absent-field defaults, finite memory categories and exact CreateName-only authorization; caller mutation after accepted cannot add,remove or update permission.
ZIP source enumeration derives a volume-GUID path from the held directory and verifies the actual reopened enumeration directory against retained fullID/version. Root/child kind and attributes use protected relative opens/basic info, not original DOS File.GetAttributes. Retain one enumerator OR active file, bounded lineage/depth/metadata and optimistic source checks; no atomic filesystem/source-set snapshot claim. Reject output root at/below any input directory and final-file identity aliases/reparse, but allow source-child/output-parent with final sibling ZIP. Preserve multi-root overlap/name/case/prefix policy and approved publication/TOCTOU.
### 4. Validation & Error Matrix
Observed capture mutation -> request_changed/rejected before accepted. Count/string/budget violations -> existing invalid_create_request/invalid_request/invalid_budget/session_entry_limit or resource_limit at owning bound. Target replacement after accepted -> target_changed/FailedBeforeCommit/zero commits; new target retained. Namespace identity mismatch -> SourceChanged/no publish; unsupported stable path -> typed source refusal, never fall back to original DOS enumeration. Dangerous output placement -> OutputCollision; safe sibling output -> success.
### 5. Good/Base/Bad Cases
Good: accepted callback mutates all original collections/budgets and output still uses original authorization; substitute real enumeration directory with subset names and refuse on real identity. Base: omitted options and ordinary source/confirmed overwrite unchanged. Bad: record-with shallow copy, bridge copies mutable authorization asynchronously, serialize limits before checking bounds, reverse-direction parent collision, or DOS enumerate while relative reads use another held object.
### 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveAdmissionTests.cs`: real Start/accepted/Job/native source+output; original source exact independent bytes/names, list replace/add/clear, exclusion/budgets, confirmation replace/add/remove, legacy extract/Gzip/XZ, unstable/bounded capture rejected without accepted. `repo:csharp/NxTools.PlatformHost.Tests/ArchiveSourceBindingTests.cs`: parent output plus normal confirmed overwrite exact independent manifest/bytes, inside/equal/multi/same-file/hardlink/reparse refusal; root/nested namespace substitution retains real identity checks, zero commits/no final/partial. No actual SUBST/DefineDosDevice/system mapping operation is authorized or inferred by the seam. Preserve original62cases; exact execution counts in remediation/checkpoint,not inferred from registration.
### 7. Wrong vs Correct
Wrong: validate caller IReadOnlyList -> accepted -> asynchronously reread List/dictionary/Limits; enumerate reconstructed DOS path. Correct: bounded owned request capture -> validate/serialize/retain same snapshot -> held directory-derived stable namespace + real identity -> same source/readback/native publication.

Held-output amendment: Runner supplies its actual ArchiveOutputSession to source admission. CheckCreationCollisions compares source identities to that session's already-held ancestor chain/root and opens the final component relative to it. Never re-resolve outputRoot via DOS just for collision checks after publishing authority is fixed. HeldOutputNamespace regression substitutes only the check path with a harmless owned directory while actual output aliases the protected source: OutputCollision/zero commits/source+target bytes retained. ErrorCompatibility additionally preserves invalid_component for long names, invalid_create_request for long creation passwords,and request_changed for mid-copy counts. Final original62+new13 integration is75/75,0skip;source result lives in current checkpoint.

## Scenario: Read-only ZIP output preparation before directory effects
### 1. Scope / Trigger
ZIP source/output admission with an absent nested output root. Evidence: `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs` and `repo:.trellis/tasks/10-16-archive-manager/zip-create-output-preparation.md`. Zero commits is insufficient when rejected requests already created source members.
### 2. Signatures
ArchiveOutputSession.Prepare(root,id,log) -> session holding actual existing ancestor handles + bounded pending suffix; CreatePreparedRoot(Action? checkpoint) completes that SAME session. Open reuses both for legacy callers. ArchiveCreationSources.Open checks original eligibility/collisions against prepared heldOutput; RevalidateRoots and RejectOutputCollisions are reused at actual creation boundaries. No wire/API authority change.
### 3. Contracts
ZIP bridge Prepare cannot create directories. Actual source_acquire owns original bounded no-reparse source chains then checks lexical dangerous direction plus real held output ancestor fullID before any create. Create pending components relative to continuously held actual ancestors,checking cancellation/source versions before/after each component; repeat same collision check against completed output root/final. Missing external root is allowed. Preserve immutable preaccepted request,source/enum namespace/identity/final commit authorization. Created directories are deliberately retained on later failure/cancel; only exact owned unpublished partials are cleaned and leases closed. CleanupComplete is resource cleanup,not global directory rollback;never recursive-delete preexisting/raced/external content.
### 4. Validation & Error Matrix
Internal absent/existing root or held ancestor/final identity alias -> OutputCollision before create; reparse/wrong-kind component -> native typed refusal; source version change -> SourceChanged; injected access5 -> source/native failure without source modification; precreate cancel -> cancelled/no create. Later lawful create+cancel may retain created directory/external additions,but no commit/final/owned partial. Existing atomic-path target results are unchanged.
### 5. Good/Base/Bad Cases
Good: absent input/new-output/nested rejects with exact unchanged source list/hash; legal sibling missing root creates independently readable ZIP. Base: existing legal output+confirmed overwrite remains unchanged. Bad: construct eager output session -> reject source later; strings-only alias proof; failure Directory.Delete(path,true); CleanupComplete implies global rollback.
### 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveSourceBindingTests.cs`: MissingInternalOutput real Job/worker/native red first; MissingInternalMultiAndIdentity bypasses lexical path yet real ancestor check rejects; ExistingInternalUnchanged includes foreign content; MissingExternalOutput independent ZIP bytes; PreparationFailures injects real-boundary cancel/denial/source member change/raced file and asserts zero commits,owned cleanup and retained external additions. Preserve original75cases; actual current counts only in checkpoint/evidence.
### 7. Wrong vs Correct
Wrong: eager create output -> source OutputCollision -> claim cleanup complete despite changed source tree. Correct: held existing namespace -> source eligibility/identity collision -> relative create with cancel/version checkpoints -> same held-session collision -> original verified publication.

## Scenario: Verified 7z and TAR-family source-tree creation
### 1. Scope / Trigger
Private Phase A create_7z/create_tar/create_tgz/create_txz, not Desktop/public product IPC.
Evidence: `repo:rust/crates/nxtools-archive/src/worker_sevenz_create.rs`,
`repo:rust/crates/nxtools-archive/src/worker_tar_create.rs`,
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveTreeCreationTests.cs`.
### 2. Signatures
ArchiveWorkerRequest adds optional TreeOptions(StripSingleRoot,ExcludePatterns) and
SevenzOptions(Solid,Encryption=none|aes256,EncryptHeader). Schema2 uses tree_options and
sevenz_options; old ZIP/extract/Gzip/XZ omit these fields. Require source_tree_v1 plus matching
sevenz_create_v1/tar_create_v1/tgz_create_v1/txz_create_v1 and existing lease/budget/partial-read.
TAR has no level/password;7z1/6/7,TGZ1/6/9,TXZ1/6/7 map fast/normal/maximum; no Store/threads API.
### 3. Contracts
Capture nested tree exclusions and SourcePaths privately before accepted. Every tree operation
reuses Prepare -> held source admission/collisions -> relative create/cancel/version checkpoints
-> same held-session collision check. Existing bounded native ID source table and exact output
name/confirmation remain authoritative. Rust worker_sources shares only ZIP's native ID adapter;
ZIP writer/crypto/reader behavior is unchanged. Lazy codec readers hold one real source at a time,
check exact length/EOF and close before next; retain first typed source/output/cancel failure.
7z single-thread public Write+Seek targets a finite anonymous exclusive delete-on-close encoded
spool, never plaintext source pre-spool or whole archive Vec. Explicit header-encryption choice;
preventive work includes preset6 default LZMA header encoder, not only data dictionary. Reserve
metadata before backend construction. TAR explicit plan/header/Read, no path-source helpers;
raw TAR bytes and encoded output independently <=max_output_bytes, including headers/end blocks.
Stage encoded spool into native partial, read same held partial fully in<=64KiB, compare full
encoded hash/length, then complete bounded structure/decode/exact names/kinds/sizes/SHA256 against
original plan. TAR has no format payload checksum; create-time source hashes do not invent one.
Sources/cancel recheck precedes unchanged native flush/recheck/single atomic path publication.
PlannedSource.size is archive payload length:0for directories even if native directory physical
size is nonzero. C# retains original physical size/fullID/ChangeTime for source revalidation.
### 4. Validation & Error Matrix
Unsupported password/level/cross-format option -> invalid_create_request before acceptance;
missing negotiated capability -> worker_capability_mismatch. Finite work/output/header overhead
-> ResourceLimit, including ArchiveWriter constructor seek; preserve first typed error before
backend wrappers. Readback damage/truncation/plan mismatch -> CorruptArchive/no commit; changed
source -> SourceChanged; cancellation -> original native outcome projection/owned cleanup.
### 5. Good/Base/Bad Cases
Good: independent stock7Zip exact list+materialized kinds/hash and Python tarfile UTF8/longname/
empty/multiple-root data; accepted callback cannot mutate original source/exclusion snapshot.
Base: plain nonsolid7z or TAR tree. Bad: directory allocation size as payload, finish-only commit,
ordinary final-path write, header encryption inherited by accident, plaintext source spool.
### 6. Tests Required
Actual Start/Job/worker/native per-format red then green; ArchiveTreeCreationTests levels/solid/
AES/header/empty/longname/multi/admission/cancel/source/budget/no-replace/overwrite/diagnostics.
`repo:rust/crates/nxtools-archive/tests/tree_creation_worker_contract.rs` injects corrupted and
truncated held readback, typed source errors/write failures; no commit, exact abort/source close.
`repo:rust/crates/nxtools-archive/tests/creation_plan_contract.rs`4096physical directory red→0payload.
Actual integration counts and remaining broad/native/product gates live only in current checkpoint.
### 7. Wrong vs Correct
Wrong: native directory Size4096 -> TAR header4096 or7z expected4096 -> reject valid archive.
Correct: retain native physical snapshot for version checks; archive plan directory payload0;
complete exact source/hash verification -> existing protected native publication.

## Scenario: ZIP edit-only source FILE handoff and volume terminal facts
### 1. Scope / Trigger
Phase E same-path ZIP editing under the explicit user-approved handoff/TOCTOU contract.
Evidence: `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveSourceLease.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs`,
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveZipEditCommitTests.cs`.
### 2. Signatures
OpenForEdit/CompleteEditReads/VerifyEditSource/ReleaseEditFile; MarkCandidateVerified and
CommitVerifiedEdit reuse original output CommitCore, not a second commit framework. Ordinary Open
retains original lightweight source protection. CommitJournal adds optional native_identity,
published and cleanup fields; absent old fields stay compatible.
### 3. Contracts
Candidate fully verified while protected; actual worker closes every source reader before original
acknowledged completion. C# still-held original fullID/version/hash/target recheck, final cancel,
release ONLY source FILE, retain source/output parents/candidate, one existing atomic rename.
Accepted final-check/FILE-close/publication TOCTOU is not ID-CAS. Never global write/delete sharing,
early release, source Dispose, delete/truncate/backup move, new privilege or blind retry.
Edit-only source MOTW uses existing bounded fixed-stream owner; ordinary DOS bits0x27 retained,
no arbitrary ADS/ACL/filesystem-managed flags or last-write-time preservation claim.
Volume NEW-family preflight precedes effects; all candidates verify/flush before first no-replace
publish. Bounded output transaction history survives exact-token cleanup, freezes after Dispose,
and excludes original encoder spool. Native identity and per-volume actual publication/cleanup
remain truthful across later failure; never set-wide atomicity or rollback claim.
### 4. Validation & Error Matrix
Precommit cancel/changed source or target -> no replacement/owned candidate cleanup. Native success
immediately Committed before close/postcheck/latecancel. Denial or busy32 -> FailedBeforeCommit;
network uncertainty -> CommitOutcomeUnknown/evidence retained/no retry. Published false for cleaned
unpublished candidates, true for actual publication, absent for uncertain. Cleanup failure explicit.
### 5. Good/Base/Bad Cases
Good: same-path independently readable ZIP after actual handoff, parents remain protected. Base:
ordinary extract/create/Test leases unchanged. Bad: expected32 masquerading as success, disposing
parents at handoff, lost unpublished journal or count encoder spool as real volume.
### 6. Tests Required
Original three real success/cancel/revision tests plus gate/ownership and failure/unknown/no-retry
fixtures; actual worker reader-closure/corrupt-readback regression separately. Native all-volume
leases and production Job ZIP/7z/RAR fixtures assert full cleanup/independent bytes/journal identity.
Exact same-set LIST→RUN,matching worker only for actual worker tests. Historical1pass/2fail32 remains
unchanged in prior outcomes; current exact results belong to sole checkpoint. No GUI/release claim.
### 7. Wrong vs Correct
Wrong: POSIX flag implies live source FILE cannot block replace. Correct: observed32 -> approved
edit-only ownership handoff -> real native success and truthful cancellation/failure/unknown facts.

## Scenario: Ordinary 7z edit adapter over the same native transaction
### 1. Scope / Trigger
Private `modify_7z` for the separately authorized PhaseE7z slice, not a second writer/commit manager.
### 2. Signatures
IsModification includes modify_zip/modify_7z. C# ArchiveJobModify adds optional SourcePrefix; SevenZ
replace_content requires exactly one ordinary protected input and one old ordinal, never ZIP Add.
### 3. Contracts
Original OpenForEdit/source/addition leases/partial-read/edit handoff/CommitVerifiedEdit own native
authority. Match actual SevenZ intent and negotiated sevenz_modify_v1/source_tree_v1/partial_read_v1.
Only the exact current edit commit call may freeze its native failure/category/code/state; later
worker IoFailure or cleanup cannot replace it. Existing unknown/published precedence stays.
Original task progress adds fixed edit planning/reading/reencoding/verifying/committing phases;
only changed fixed7z phases use the existing bounded business queue, byte samples stay coalesced.
Password remains private array/stdio only. No member/path/content/secret diagnostic fields.
### 4. Validation & Error Matrix
Malformed action/format/source-prefix -> invalid_edit_request; stale source -> source_changed;
readback/fidelity error -> no handoff/publication; native5 -> FailedBeforeCommit/code5; network64 ->
CommitOutcomeUnknown/evidence/no retry; late cancel after native success retains published journal.
### 5. Good/Base/Bad Cases
Good: real7z candidate same-path replacement with source FILE readers gone and parents held.
Base: original ZIP/extract/Test permissions unchanged. Bad: general source sharing widened for edit,
plain output from encrypted input, foreign argv password, or mutable last-native-error recovery.
### 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/SevenzEditTests.cs` uses hash-frozen current worker, exact
LIST/RUN, independent7Zip known metadata/content, source/handoff/cancel/fail/unknown/latecancel.
Retain ArchiveZipEditCommitTests and original task/OLE/space collections; counts in checkpoint.
### 7. Wrong vs Correct
Wrong: feed SevenZ into ZIP editor or release native source early. Correct: Rust7z full candidate
verification -> same read-complete boundary -> original protected one-shot native publication.
See [Rust format contract](../nxtools-archive/backend/sevenz-edit.md) for fidelity/refusal limits.

## Scenario: TAR-family edit adapter over the same native transaction
### 1. Scope / Trigger
Step3 TAR/TGZ/TXZ editing only; same PhaseE transaction, no new commit/share-mode policy.
### 2. Signatures
`modify_tar` matches actual Tar/TarGz/TarXz and negotiates tar_modify_v1/source_tree_v1/partial_read_v1. Optional browser TarEdit facts are completed-worker source qualification, not write authority.
### 3. Contracts
Existing Modify ordinals/source-prefix/actions, protected additions, OpenForEdit, held readback, edit_reads_complete and CommitVerifiedEdit remain the owners. Rust verifies the complete outer+TAR candidate and closes all readers before FILE-only handoff. Fixed phases/no-password/correlated terminal native codes reuse the original Job. Final TOCTOU/unknown retention remains unchanged.
### 4. Validation & Error Matrix
Stale source/prefix -> source_changed; unsupported structure/readback failure -> no publication; native5 -> FailedBeforeCommit,64 -> unknown/no retry; latecancel keeps published journal. Disk-full112 injected before_flush must preserve source and remove only owned unpublished candidate.
### 5. Good/Base/Bad Cases
Good: independently verified same-path candidate after old reader closure. Base: unchanged original ZIP/7z/native lease owners. Bad: suffix grants edit, source FILE release before verification, or injected112 called real disk-fill evidence.
### 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/TarEditTests.cs`: effects -> exact LIST -> same RUN, frozen matching worker, CPython/bsdtar operations/metadata/empty/source/cancel/native5/64/latecancel/conflict and injected112. [Rust contract](../nxtools-archive/backend/tar-edit.md) defines exact fidelity/limits.
### 7. Wrong vs Correct
Wrong: new native replacement algorithm or global sharing relaxation. Correct: separate TAR codec candidate -> original held-source/read-complete/one-publication chain. Physical GUI/installed/release acceptance remains separate.

Step4 interaction amendment: original Job emits Ask for every admitted Modify, including ZIP;
Runner admits the same existing InteractionPort on modify_zip/modify_7z/modify_tar. Existing optional
private display remains archive-relative for Modify, never destination-prefixed or native authority.
No source/output/FILE-only handoff/share/commit policy changes. [Cross-owner edit interaction contract](../nxtools-archive/backend/edit-interactions.md)
requires exact native ZIP Ask decisions and original TAR/7z/ZIP/preview/control compatibility using
one explicit current hash-frozen worker; retain initial failures/skips and corrected reruns.

EIC-AC01/02 regression amendment: original Modify ZIP logical implicit-directory merge/allSkip
must reach Job succeeded_with_warnings with empty publication journal and unchanged source;
existing native commit probes verify zero flush/publication. Exact native `cancelled` private reply
must remain worker Cancelled/Job cancelled without another cancel control frame. Rust owns this
reply mapping; do not change C# Job precedence or use cancellation request state to override real
published/unknown/sourcechanged/cleanupfailure facts. Original ZIP/TAR/7z actual control-cancel,
native5/64/latecancel gates remain. [Exact repair contract](../nxtools-archive/backend/edit-interactions.md).

HZ3 observation amendment: explicit ArchiveSpace v2 projects original ArchiveVolumeSources
identity/primary/members after the same held-source/output observation; v1 remains unchanged.
No decoder/output/source-write operation or new native lease policy. Strict bounded private
source_set validates the requested source and unique members, never payload diagnostics.
Original Job/native re-admission and final publication remain authoritative; Desktop batch
dedup/explicit retry uses [the cross-owner contract](../nxtools-desktop/backend/archive-batches.md).

## Scenario: HZ3 complete native publication evidence
### 1. Scope / Trigger
Generic batch retry must distinguish zero FILE commits from zero output; original native directory
creation/metadata failure can leave output while CommitJournal is empty. Evidence:
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs`,
`repo:csharp/NxTools.PlatformHost/Protocol/ArchiveJobSession.cs`.
### 2. Signatures
`ArchiveOutputSession.OutputPublished() -> bool?`; optional/default-null
`ArchiveWorkerEvent.OutputPublished` -> `ArchiveJobTerminal.OutputPublished` -> Rust
`ArchiveJobTerminal.output_published: Option<bool>` (serde default/omitNone). Existing versions and
file CommitJournal unchanged. This is read-only evidence, never execution/commit/cleanup permission.
### 3. Contracts
True: known published file or native-created/completed output directory. Original relative native
directory-create success records the fact BEFORE fallible inspection; root/wrapper/ancestry retained
on failure count, not just completed explicit empty-directory requests. Unknown directory creation,
live/undrained owner or uncertain file commit cannot yield false. False requires original cleanup
and affirmative zero publication. Runner freezes facts only after worker drain/resources; Job passes
them through without changing terminal/cancellation/native precedence. Preserve directory-only
ResultLocation; no recursive deletion/rollback to manufacture zero output. Missing legacy facts are
unknown. C# fixture env `NXTOOLS_HZ3_NATIVE_EVIDENCE` names its explicit owned export file.
### 4. Validation & Error Matrix
Metadata failure after ensured directory -> failed/io_failure, files0/journal[], publishedtrue,
directory+location retained. File failure before publication with no created directory ->
publishedfalse after cleanup. Root/ancestry created before later failure -> publishedtrue even when
no archive result_location is available. CommitOutcomeUnknown -> null, not false. Legacy omission
and explicitnull deserialize unknown; booleans roundtrip; numeric/string/object/array rejected.
### 5. Good/Base/Bad Cases
Good: native failed empty directory stays visible in summary and refuses blind retry; real zero
output may revalidate. Base: accepted one-shot FILE publication and old DTO omission. Bad: empty
file journal/commitstateNone/root identity unchanged treated as no output, or delete directory first.
### 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/Hz3BatchTests.cs` exact two fixtures: matching worker/native
directory metadata failure plus genuine preflush zero-output control exports exact terminals; wire
defaults/strictbool and original retained-root/ancestry/unknown-owner controls. Rust production app
consumes current export and validates summary/retry. Effects -> exact LIST -> same RUN; skips not
passes. Checked-in actual transcript fallback is replay evidence, not a fresh native/physical run.
### 7. Wrong vs Correct
Wrong: `files_committed == 0 && journal.is_empty()` -> replay. Correct: original native complete
publication fact + clean known precommit journal + immutable pre-native-terminal identity -> explicit
fresh recheck -> original queue, otherwise original form/reopen. Facts never weaken native permissions.
