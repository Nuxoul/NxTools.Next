# Volume and ZIP-edit seams — Phase E production ownership

## 1. Scope / Trigger
Approved project-owned ZIP container/record adaptation with existing payload codecs/crypto unchanged.
Evidence: `repo:rust/crates/nxtools-archive/src/zip_container.rs`,
`repo:rust/crates/nxtools-archive/src/zip_edit.rs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVolumeSources.cs`.
Current results and unfinished native/user acceptance live only in the archive task checkpoint.

## 2. Signatures
`ArchiveEngine::with_zip_volumes(self)` enables exact ZIP source-family routing; default engines stay
OFF. `zip_volume_candidates` returns anonymous unpublished encoded files. Production worker enables
routing only after source_acquire returns the C#-protected complete family. WorkerRequest.volume_bytes
and job_modify are additive negotiated private fields; public ArchiveJob Modify binds browser source,
revision and original ordinals. `zip_edit_candidate` returns a verified anonymous candidate, not a
final-path writer. ArchiveRead remains read-only.

## 3. Contracts
ZIP max1024 exact same-directory .zip/.zNN members. Parsed canonical ordinal/family and exact
canonicalized member identity follow platform path semantics: Windows .Z01 aliases .z01; absent
uppercase paths on case-sensitive platforms do not. No unconditional path-folding or recursive scan.
Checked EOCD/ZIP64 disk coordinates map to bounded encoded spool ranges; original local/descriptor/
CRC/authentication checks remain. SourceRevision includes all member digests; native fullIDs remain
C#. Native source owner acquires every exact ZIP/7z/RAR member before codec reads and retains leases
through actual worker/Job cleanup. 7z uses encoded-length-bound continuous volumes; RAR4/5 transitions
must stay within the native protected set. No RAR creation.

Continuous ZIP `.zip.001` or signature-bound `.001` input is a distinct read adapter, not a
disk-coordinate split writer. The original native owner acquires the complete consecutive family,
checks the final ZIP extent and returns its exact ordered paths. Worker `with_held_zip_volumes`
and snapshot reopen never rescan that production set. Public opt-in discovery remains available.
Encoded normalization uses its own finite source allowance for isolated browser entries, not the
selected text/image output quota; actual output and all other budgets stay unchanged. See
[reader compatibility](reader-compatibility.md) for concrete seams and tests.

ZIP split writer regenerates per-disk local/central/end coordinates, not ordinary byte chopping;
7z splits the fully verified continuous encoded stream. NEW target families only: bounded first/
final/surplus numeric-member preflight before effects, then authoritative native no-replace per member.
Write/readback/flush/validate ALL candidates before first publication. No volume-set atomicity or
existing-family overwrite/backup framework. Existing output journal retains native identity,
published/unpublished/uncertain and actual cleanup facts after disposal, excluding encoder spool.

Ordinary ZIP edits use immutable source/revision/entry IDs, not visible row indices. Separate local/
central extras, attributes, comments, encryption headers/data/descriptors and unchanged raw names
remain exact. Rename rewrites name/UTF8/Unicode-CRC and relocation rewrites disk/offset/ZIP64 fields.
Known safe extras have a finite allowlist; unknown potentially dependent structures, SFX, application
containers, damaged and split archives reject before native edits. Add uses original plain Deflate
writer (Stored directory records), explicitly disclosed in the existing form; no automatic new-entry
encryption. Existing encrypted entries retain original encoded records. Source MOTW/ordinary DOS
bits use the existing native owner; no arbitrary ADS/ACL/filesystem-flag preservation claim.

Edit-only source FILE handoff follows approved contract: complete verified candidate -> all worker
source readers actually closed -> acknowledged read-complete -> still-held native source fullID/
version/hash/target check -> final cancel -> release ONLY FILE -> one original atomic replacement.
Parents/candidate retained, no global sharing change. Accepted final-check/close/publish TOCTOU is not
ID-CAS. Real publication survives late cancel; failure/unknown never blindly retries.

## 4. Validation & Error Matrix
Missing member -> MissingVolume; replaced source/set -> SourceChanged; malformed disk/record/extent ->
CorruptArchive; finite metadata/work/encoded/count limits -> ResourceLimit. Unknown edit structure ->
UnsupportedOperation before publication; unsafe/colliding names -> original typed policy. Candidate
readback damage aborts before handoff. Native target race never overwrites an unapproved final; earlier
published members remain truthful journal facts. Unknown publication retains evidence, not partial-delete.

## 5. Good/Base/Bad Cases
Good: real Job/worker/native split outputs independently extracted by hash-verified stock7Zip; real
RAR4/5 volume fixtures match independent hashes. Base: ordinary source/read/create protection stays.
Bad: expose source siblings before native leasing, count encoder spool as a volume, lose cleaned
unpublished facts, infer atomicity from last.zip, or claim GUI acceptance from TestWindow/compilation.

## 6. Tests Required
Small zip_volume_contract covers Windows case semantics/exact ordinal, missing/replaced/order/count/
budget/default-off. zip_edit_contract covers all actions, separate metadata/unknown refusal, AES and
ZipCrypto record preservation, tiny forced ZIP64. zip_edit_worker_contract proves reader closure at
actual handoff and corrupt-readback abort. ArchiveVolumeSourceTests protects all members/native .Z01
routing; ArchiveZipEditCommitTests requires actual same-path success/cancel/revision/gates/fail/unknown.
ArchivePhaseEDeliveryTests exercises production edits, ZIP/7z split browse/Test/independent extract,
old-family refusal, partial publication race and genuine RAR4/5 extraction/missing member.
Explicitly exclude known100ktest for engine verification; no large/performance matrix here. Current
exact counts belong to outcomes/checkpoint, not inferred from registrations.

## 7. Wrong vs Correct
Wrong: source path/hash alone grants sibling access; finish alone grants publication; cancelled means
unchanged. Correct: native complete source set -> original bounded codec/candidate verification ->
existing per-output publication/cleanup journal -> once-only truthful terminal and browser refresh.
