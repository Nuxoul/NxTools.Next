# Extraction metadata and MOTW handoff

Evidence: `repo:rust/crates/nxtools-archive/src/metadata.rs`, `repo:rust/crates/nxtools-archive/src/formats/zip.rs`, `repo:rust/crates/nxtools-archive/src/formats/tar.rs`, `repo:rust/crates/nxtools-archive/src/worker_codecs/rar.rs`, `repo:csharp/NxTools.PlatformHost.Tests/ArchiveMetadataTests.cs`.

## Scenario: Typed archive metadata through the original publication

### 1. Scope / Trigger
Phase A extraction mtime and supplied Windows readonly/hidden, plus default-on original-source MOTW. Not UI, source deletion, product IPC/queues or packaging. Read the task's [source matrix](../../../tasks/10-16-archive-manager/research/extraction-metadata-sources.md) before changing a format adapter.

### 2. Signatures
`OutputMetadata { modified: Option<ModifiedTime>, attributes: Option<WindowsAttributes> }` is immutable and Copy. `ModifiedTime::{DosLocal{date,time},FiletimeUtc{ticks},UnixUtc{seconds,nanoseconds,precision_ns},Invalid}`; attributes are two booleans, never a native mask. `ArchiveOutput::set_file_metadata(token, metadata)` stages against an unpublished token; `finalize_directory_metadata(relative,metadata)` uses previously registered layout authority. Unsupported adapters fail closed.

### 3. Contracts
- ZIP original central records: NTFS100ns UTC -> UT unsigned Unix seconds -> DOS local2s. All-zero DOS is absent. Central UT length5 may contain only mtime even when local-header flags include other times. Supplied malformed higher-priority time is not fallback. Windows-origin0/10/14 permits low1/2 only.
- 7z presence flags gate NtTime100ns since1601 and Windows attributes; raw zero is not absence when present. No upper POSIX/raw bits replay.
- RAR DLL mtime low/high is normalized UTC FILETIME; zero means unavailable in that API. Public Windows host2 gates attrs. Original encoding/precision/RAR4 zone ambiguity is not recoverable. Exact bundled `dll.hpp` is pack1; do not use mispacked upstream HeaderDataEx. Private packed codec FFI checks win64 size10244/CmtBuf6188/Mtime6268 and copies arrays before references.
- TAR mtime uses only its fixed12-byte checked octal/GNU signed base256 decoder; GNU markers0x80/0xff, full-field sign extension and checked i64 conversion. Never use unsigned Header::mtime or truncate to low64bits. All-NUL/space is absence; malformed/overflow is Invalid; valid signed values retain Unix semantics and existing native-range validation. Size/other numeric fields stay unchanged.
- PAX mtime is Unspecified/Set/Delete, not empty decimal. Local Set > global Set > header; global Delete removes persistent override and restores later member headers. Local Delete suppresses both global and header for the current member only; next member retains global context. Last same-key record wins; nonempty malformed values remain Invalid. Bounded64-byte signed decimal/fractions use no floats/exponents; negative discarded sub-ns digits preserve earlier100ns floor. TAR compounds share the parser and original compressed-source authority. See [source semantics](../../../tasks/10-16-archive-manager/research/extraction-metadata-sources.md#tar-time-p2-source-clarification).
- Gzip first genuine RFC1952 MTIME describes aggregate output; zero absent. BZ2/XZ/ZST have no fabricated mtime/Windows attrs.
- Schema2 extract-only `extract_options:{copy_motw:bool}`; omitted fields stay omitted for legacy wire. New C# caller omission means default-on, new host requires extraction_metadata_v1+motw_v1. Old raw wire gets no new metadata native actions.
- Preserve full integrity/source/cancel checks before original publication. TAR/7z/RAR retain metadata with pending token until whole stream integrity succeeds. Explicit directories finalize deepest-first; ordinal lookup is bounded, not repeated linear entry search. No Rust Windows APIs or ADS bytes.

### 4. Validation & Error Matrix
Missing/not-provided -> absence. Supplied invalid/out-of-range -> InvalidMetadata. DOS stays local; native ambiguous/nonexistent calendar fails. Native unrepresentable value or metadata/ADS failure cannot be full success. Valid FILETIME0 currently succeeds as a native no-op: C# verifies exact observed mtime and fails metadata_time_not_supported before publication, rather than treating it as absent. Committed/unknown output facts cannot roll back or become zero commits after later failure.

### 5. Good/Base/Bad Cases
Good: independent ZIP/Python/external7z/syntheticRAR archives plus native Windows file-info/ADS oracle. Base: no metadata still extracts ordinary bytes; default source MOTW applies through the common C# seam. Bad: source stat/current time/extension supplies fake metadata, DOS relabeled UTC, Unix mode becomes readonly/hidden, or public binding names substitute for verified native ABI.

### 6. Tests Required
Retain real Start -> Job -> Rust -> native tests for each supported metadata source and absence formats, directory ownership/deepest-first, signed precision/zero/invalid values, options negotiation, default/disabled/missing/empty/budget ADS, native failure/cancel/publication and privacy. TAR regressions: `repo:rust/crates/nxtools-archive/tests/tar_time_contract.rs` and `repo:csharp/NxTools.PlatformHost.Tests/ArchiveTarTimeTests.cs` assert raw GNU negatives/full-width overflow, global clear/local suppression/scope, exact compound content/nativeUTCtime and true postfailure commits. Do not assume staged token commit order; compare actual published files, plus deterministic post-file directory failure. Boundary-year oracle uses Windows GetFileTime UTC directly because .NET GetLastWriteTimeUtc can overflow via local conversion. Keep wire compatibility, full isolated archive tests, default+isolated strict Clippy, fmt/workspace check and original C# selection union new tests. List/count before run; large ZIP64 once in final integration. Current results belong to the [single checkpoint](../../../tasks/10-16-archive-manager/phase-a-checkpoint.md).

### 7. Wrong vs Correct
Wrong: `attributes = raw`, set final-path metadata after rename, or trust SetFileTime success without observation. Correct: typed values -> exact owned native partial -> MOTW/time/whitelisted attrs -> same-handle observation/flush -> original recheck/single publication -> owned directory finalization.
