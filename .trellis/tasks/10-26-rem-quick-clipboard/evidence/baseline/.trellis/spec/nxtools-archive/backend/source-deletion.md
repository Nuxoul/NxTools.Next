# Lifecycle-derived source-delete qualification

Evidence: `repo:rust/crates/nxtools-archive/src/source_deletion.rs`,
`repo:rust/crates/nxtools-archive/src/control.rs`, `repo:rust/crates/nxtools-archive/src/engine.rs`,
`repo:rust/crates/nxtools-archive/src/creation.rs`, `repo:rust/crates/nxtools-archive/src/worker.rs`.

## Scenario: Original-job success facts, not path deletion authority

### 1. Scope / Trigger
Phase A foundation only. No UI/product queue/public PlatformCommand or native recycle execution.
Read [native owner](../../platformhost/archive-source-deletion.md), task source-deletion-contract.md
and research/source-recycle-native-capability.md before altering qualification or source sharing.

### 2. Signatures
`OperationControl::source_delete_eligibility(consent: bool) -> SourceDeleteEligibility` consumes
private lifecycle facts once; output has operation_id, fixed reason and finite original source_ids.
Fields cannot be constructed or deserialized as in-process authority. WorkerRequest adds optional
`delete_source: Option<bool>`; omitted legacy fields remain omitted. Schema2 negotiated capability
source_delete_eligibility_v1; true extract also requires extract_options metadata/MOTW authority.
Only extract/create operations may carry the new field. Terminal source_delete is private output,
not public product IPC, delete paths, arbitrary parents or recursion authority.
Private `source_delete_begin() -> Lifecycle` creates a non-cloneable Arc identity; every finish
requires that exact original marker. Marker is not wire/public authority or operation UUID.

### 3. Contracts
- Engine open/test/extract each begin before validation/I/O,invalidating earlier facts even on
  failure. Begin returns a fresh private lifecycle identity;finish only changes facts if still
  current,then retires marker. A later begin wins,including reentrant callback/concurrent clone;
  mutex-only writes do not establish lifecycle ownership. Successful extraction records only
  after full file/source/publication/directory-finalization;selected/skipped/empty/count refuse.
  Older full extraction cannot restore eligibility after later open/test/failed extract;older
  selected completion cannot overwrite a newer full-success fact. Begin never resets consumed.
- Isolated RAR/7z record after full decode, all pending publication, directory finalization and
  final source fingerprint. No extraction qualification from open/test or a partial file count.
- ZIP/7z/TAR/TGZ/TXZ and single Gzip/XZ record only after full held-partial readback/structure/
  decode/hash verification, source revalidation and successful native publication acknowledgement.
  Finish alone, verification/commit failure and unknown/postcommit outcomes cannot grant.
- Worker dispatch invalidates before source_acquire. Normal engine open -> extract/test is legal
  sequential lifecycle ownership,not a shared outer grant. Each isolated RAR/7z and each creation
  implementation (including7zcreate_with_reader) begins at entry before options/budget/I/O and
  passes its own marker to the final fact point. CreationPlan only projects original roots;
  record_published_verification receives the caller lifecycle,never creates a new one at finish.
- CreationPlan retains bounded original roots separately from stripped archive names. Any actual
  exclusion -> members_excluded. Any directory root -> directory_coverage_unproven, including
  empty/stripped trees; do not silently delete archived descendants or unarchived directory members.
  Optimistic traversal is not a snapshot. Only exact original regular-file root IDs can qualify.
- Absence/false consent -> consent_missing, zero candidates. One real grant per shared operation
  control, concurrent consumers serialize and observe already_consumed. C# independently binds
  original admitted operation/source objects/version and native execution; Rust invokes no Windows
  API and deletes nothing. Engine-only eligibility is not object-safe native authorization.

### 4. Validation & Error Matrix
Incomplete/failed lifecycle -> lifecycle_incomplete; cancel -> cancelled; selected-only ->
selected_only; links/skips -> members_skipped; empty -> empty_source_set; count -> coverage_incomplete;
creation exclusion/directory -> members_excluded/directory_coverage_unproven; reused grant ->
already_consumed. Existing codec/metadata/I/O failures retain original archive error/state.

### 5. Good/Base/Bad Cases
Good: actual full extraction facts + original-job consent -> finite candidate IDs -> independent
C# validation and separate result. Base: old request succeeds with no recycling. Bad: structural
permits_source_delete_request alone, test success, encoder.finish or plan-entry completion deletes
an entire original directory. Default Thumbs.db/desktop.ini exclusions remain user data.

### 6. Tests Required
`repo:rust/crates/nxtools-archive/tests/source_deletion_contract.rs`: actual decode/metadata/commit/
directory success and failure, default-off/once-only/concurrent, selected-all IDs still selected,
CRC partial commits,cancel,source mutation,TARlink skips,test-after-extract invalidation;real
directory-finalization callback interleaves later test/open success/open failure/extract plan or
source-identity failure deterministically,no sleeps. Assert incomplete inside callback AND after
old finish,three actual commits,newer valid extract wins,operation consumption/cancel unchanged.
Worker wire tests reject legacy/unnegotiated/test-only authority; real ZIP creation worker checks
full readback/finish/source failure and exact regular-file IDs versus directory refusal. Actual
C# Start/Job/Rust/native tests cover original object binding and injected backend outcomes. Run
full isolated suite, strict default+isolated Clippy, fmt/check and shared ZIP regressions; current
counts only in phase-a-checkpoint.md. Fake backend is never real Recycle Bin acceptance.

### 7. Wrong vs Correct
Wrong: finish under mutex unconditionally restores old eligible facts,or test/finish-only counts
become native delete authority. Correct: fresh private lifecycle -> same-marker-only completion
-> operation-wide once-consumption -> original C#Run/session authorization,independent result.
