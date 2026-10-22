# Extraction metadata / MOTW — executable slice contract

Canonical phase: **Phase A — Engine contracts and safety foundation**. Status: implementation
in progress; not slice/Phase A acceptance. This does not change publication/TOCTOU approval,
source deletion eligibility, UI, product IPC, queues or package mapping.

## Boundary and approved seams

Gap: archive mtime/Windows readonly+hidden are not carried to native output; source MOTW is not
copied. Rust parses bounded immutable metadata; C# owns Windows handles and actual mutation.
Tests use the approved actual Start -> Job -> Rust worker -> native output and independent
Windows file-info/ADS reads, plus public format and private-wire regression boundaries.
Expected changes: archive metadata/parser/output/worker owners; C# source lease/output/session/
runner; dedicated synthetic fixtures, effects registration, reference/owner/task evidence.
No unrelated refactor or changes to user dirty files outside this behavior.

## Time and attribute contract

Keep source semantics in a typed value: UTC FILETIME (100ns since1601), signed Unix seconds plus
subseconds, or DOS local calendar (2s). Do not relabel DOS as UTC. Native application uses the
Windows local time-zone rules only for an unambiguous valid local calendar; ambiguous/nonexistent
wall clocks fail explicitly rather than choose an undocumented offset. UTC formats never use
local-zone conversion. Preserve declared precision; resolution below100ns cannot be represented
by Windows and is truncated toward the earlier instant, with tolerance<100ns in the oracle.
Missing/format-not-provided is explicit absence, not now/source path metadata. Supplied malformed
or out-of-range metadata is a typed failure, not silent fallback to now. Exact format/API matrix
and verified source chain are recorded in the reference-linked research before each adapter.
Only readonly/hidden booleans are authorized; raw attribute masks, reparse/device/system/archive/
Unix permissions are not worker authority. Do not infer hidden from dot names or readonly from
Unix mode bits. Known absence and unsupported Windows attributes are distinguished in metadata.

## File lifecycle

Decode -> full existing integrity/plan checks -> original source/cancel recheck -> original native
commit. Metadata is staged against the current operation's unpublished token, not an arbitrary
path. Commit flushes pending managed writes, copies MOTW, restores time, applies readonly/hidden
last, flushes/observes the same handle, then performs original target recheck and one publication.
Record native success immediately before any later observation/cancellation. Refresh cleanup
credentials from actual restored metadata; no final-path metadata write. A precommit failure
preserves old target and removes only exact owned partial. Readonly cleanup clears only readonly
on the exact owned handle after ownership checks, then existing handle deletion; no new path
delete/truncate/backup-first/fallback/retry. Published/unknown outputs retain existing nonrollback
rules. Directory metadata failure after file publication must retain real FilesCommitted.

## Directory lifecycle

Track actual create disposition per held directory identity; an existing/raced-existing directory
is layout-only. Never explicitly rewrite its mtime/attributes. Explicit archive directories created
by this operation finalize deepest-first only after all children and partial cleanup complete.
Implicit directories have no fabricated metadata. Retain directories on failure; no recursive
rollback of external content. A successful extraction terminal follows successful directory
finalization, not merely file commits.

## MOTW / compatibility

Only fixed `:Zone.Identifier` is supported. Read once after no-reparse original ArchiveSourceLease
acquisition, bound to that same held source object; compound origin is original archive, never
anonymous decoded TAR spool. Maximum64KiB, with length/extra-byte bounds; no unbounded ADS read.
No ADS bytes/URL/path/name/token in diagnostic events. Do not expose arbitrary ADS names/paths.
Default enabled at the C# request boundary; absent legacy C# option means enabled and the new
host explicitly packs schema2 extract_options. Omitted raw private-wire options denote an old
host with no metadata-action authority,not a new host's default. Schema2 alone predates this
slice and is not two-way negotiation. New host requires extraction_metadata_v1/motw_v1;old
host/new worker gets no unknown metadata actions. Options in schema1/nonextract reject.
No UI/config/product IPC added.
Source without mark is a successful no-mark state. Missing metadata/unsupported-by-format are
not errors. ADS filesystem unsupported, over-budget source ADS, source ADS denied, native time/
attribute/ADS failure are distinct fixed categories and fail the operation (no full-success
claim). If files already published, preserve them and report failure with true committed count.
Explicit MOTW false performs no source ADS read and no target ADS write.

## Diagnostics

Reuse operation correlation: accepted/start, source-MOTW captured/absent/disabled, first metadata
application, directory finalization, terminal with elapsed/native code. Coalesce per-file metadata
success into stage/count, not byte/per-entry spam. First exact failing native call owns terminal
category/code. Sink failure is observational only. No raw backend exceptions/payload logging.

## Initial actual evidence (not product acceptance)

Scratch metadata-native-probe.ps1/log, self-owned NTFS files: NtCreateFile RootDirectory=held
source + fixed :Zone.Identifier and :Zone.Identifier:$DATA both succeed native0; same relative
stream CREATE_NEW on exclusive partial succeeds native0. No DOS reopen required. FILE_BASIC_INFO
readonly set0 -> single NtSetInformationFile rename0 -> direct readonly deletion native5 -> clear
readonly on same held handle0 -> delete same handle0. Windows SDK10.0.26100.0 winbase.h9077 and
minwinbase.h179 confirm FILE_BASIC_INFO layout and FileBasicInfo0; winternl.h475 NtCreateFile.
These are capability probes, not full product fixtures or FAT/SMB acceptance.
First actual ZIP DOS test: list selected1/run0, then selected1/run1/pass0/skip0/fail1/not_run0,
exit1. Expected2020-02-02T20:05:06Z (local2020-02-03 04:05:06), observed extraction-time2026 date.
C# build0warnings/0errors,exit0; product implementation unchanged at red. Logs metadata-time-red*.
