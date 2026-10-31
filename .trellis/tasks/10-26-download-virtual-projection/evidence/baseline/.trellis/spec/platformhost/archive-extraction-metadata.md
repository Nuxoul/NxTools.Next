# Archive extraction metadata native boundary

Evidence: `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputMetadata.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveMotw.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerOutputBridge.cs`, `repo:csharp/NxTools.PlatformHost.Tests/ArchiveMetadataTests.cs`.

## Scenario: Bound mtime/attributes and fixed source MOTW

### 1. Scope / Trigger
Extraction-only native metadata restoration, not general path/ADS tools. Rust owns format/time semantics; C# owns Windows calls, protected original source and exact unpublished output. Read [Rust contract](../nxtools-archive/backend/extraction-metadata.md) and the task source evidence first.

### 2. Signatures
`ArchiveExtractOptions(bool CopyMotw = true)` is admission-snapshotted/extract-only. Private native actions `file_metadata{token,metadata}` and `directory_metadata{path,metadata}` use existing operation authority. `ArchiveOutputMetadata.Parse(JsonElement)` rejects unknown/duplicate/missing fields and raw masks. `ArchiveSourceLease.CaptureZoneIdentifier()` reads once; `ArchiveOutputSession.BindZoneIdentifier(ReadOnlyMemory<byte>?)` stays host-only. Only fixed `:Zone.Identifier`, maximum65536bytes.

### 3. Contracts
- File order: flush managed writes -> bounded fixed ADS -> SetFileTime -> read/merge readonly+hidden via FILE_BASIC_INFO -> native flush/held identity/content fingerprint -> existing target recheck -> original one publication. Exact observed mtime must match requested native precision; API success alone is not restored metadata.
- DOS calendar uses local historical zone only when valid/unambiguous. FILETIME/Unix UTC never converts through local. No final-path metadata mutation.
- ADS opens relative to held base handle and verifies same volume/full128-bit ID. Source read share-read denies concurrent ADS writers during capture; target/cleanup ADS share-delete only to coexist with held base DELETE access. Base/parent sharing protection does not widen. Read exact length plus extra byte/recheck; bytes zeroized on lease disposal. Never transmit ADS bytes to Rust/business frames/logs.
- Query GetVolumeInformationByHandleW on the held object; FILE_NAMED_STREAMS0x40000 absence is motw_not_supported, not missing ADS. Do not invent native error50 for a successful capability query. Explicit false does no source ADS read/target ADS write/capability query. Compound provenance is original compressed archive, not spool.
- Track actual exclusive directory creation; preexisting/raced-existing directories are layout-only. New explicit directories finalize deepest-first after files, no recursive rollback. Directory failure after publication retains true FilesCommitted and Committed state; cancellation gives cancelled_with_committed_output when appropriate.
- Record publication immediately. Owned partial cleanup clears readonly only on that exact handle. Guarded new-output cleanup requires original fullID/size/mtime/hash plus actual attributes and fixed MOTW digest/absence before clearing readonly/deletion. External metadata changes are retained. Replaced/published-uncertain/nonrollback rules stay unchanged; no retry/backup/delete-first/fallback.
- Diagnostics use same operation ID, elapsed/native code, source motw_captured/absent/disabled and coalesced metadata/directory stages. First exact failing native call owns terminal code/category. Throwing sink cannot affect output. Never log names/paths/ADS/URLs/raw exceptions.

### 4. Validation & Error Matrix
Absent ADS -> successful no-mark; empty ADS -> exact empty stream. Over65536 -> motw_budget. Capability unsupported -> motw_not_supported; native errors1/50/120 preserve actual code. Read/write rejection -> motw_read_failed/motw_write_failed. Time/attrs rejection -> metadata_time_failed/metadata_attributes_failed with original native code. SetFileTime success/no-op or precision mismatch -> metadata_time_not_supported, no fabricated error code. Invalid typed JSON/local ambiguity -> metadata_invalid/metadata_local_time_ambiguous. Precommit failure removes own partial and preserves old target; postcommit failure retains publication/count.

### 5. Good/Base/Bad Cases
Good: independent file-info/ADS oracle checks the published object and source preservation. Base: original source without mark, legacy caller default-on, explicit disabled. Bad: mutable DOS source reopen, arbitrary ADS name/raw attribute worker authority, readonly before ADS writes, or directory failure claiming FailedBeforeCommit after files published.

### 6. Tests Required
`ArchiveMetadataTests` uses only GUID-owned synthetic files/ADS/helper/Jobs and independent Windows reads. Keep per-format time/attrs, nested/preexisting dirs, default/options/budget/absence, capability flag injection without volume changes, source lease, native denied/unsupported, metadata-stage cancel, overwrite/no-replace, readonly partial cleanup, external metadata retention, postcommit facts and throwing-log/privacy regressions. Run original precise C# fixture selection union new tests with zero skips; test effects reviewed explicitly. Source gates/compilation are not native/Phase A/release acceptance.

### 7. Wrong vs Correct
Wrong: missing stream on a non-ADS filesystem means no MOTW, or cleanup credentials ignore external ADS/attrs. Correct: held-volume capability -> fixed identity-bound bounded capture -> exact partial mutation -> native publication facts -> metadata-aware guarded cleanup/retention.
