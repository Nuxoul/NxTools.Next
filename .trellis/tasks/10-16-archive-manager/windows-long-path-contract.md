# Windows long paths — bounded real-chain contract

Canonical **Phase A — Engine contracts and safety foundation**. Implementation authorized
2026-10-20; regressions implemented and verified, pending independent acceptance. Only the
named local Windows long-path conditions are covered, not ENG08/Phase A/A–F, UNC, UI or deployment.

## Change boundary

The current gap is missing real >260-character regression evidence across product source reads,
native extraction publication and native source-tree ZIP creation. Existing source/output code
already uses held no-reparse component opens and internal volume-GUID namespaces; do not refactor
or change policy merely to produce a production diff. Add bounded actual-boundary tests first.
Only a demonstrated failing behavior may justify a minimal owning-boundary repair with the same
red test rerun. Do not change dependencies, resource tools, budgets, deadlines or system policy.

Expected files: a dedicated C# long-path fixture plus exact Catalog/TestEffects registrations;
existing independent Python ZIP oracle reused through the ZIP fixture helper; a new case in the
existing Rust product_read_chain integration test; this contract, additive evidence/results,
reference navigation and the unique current checkpoint/context. Production paths remain
unchanged unless one of those real tests fails with a concrete defect.

## Reference and call boundaries

Read the matching [reference note](../../../docs/reference-code/archive.md) before Next source.
Follow its recorded legacy/package/FFmpeg and Archiver/SupaZip sources; newly verified details
must be added to that note before test implementation. Source inspection is not runtime proof
that a reference handles long Windows paths. No Bandizip reverse engineering or runtime required.

- Rust `ArchiveReadCommand::validate` -> `ArchiveReadClient::start/poll` -> original typed platform
  pipe -> C# `ArchiveReadSession.Dispatch` -> original `ArchiveWorkerRunner` -> suspended child /
  Job admission -> schema2 source acquire -> `ArchiveSourceLease.Open` -> actual Rust worker
  engine open/test -> original process/pump/Job cleanup -> one terminal.
- Extraction: Rust original-record ZIP policy/plan/integrity -> existing native output rendezvous
  -> C# `ArchiveOutputSession` held-parent relative `NtCreateFile`, exclusive partial, metadata /
  native flush -> target recheck -> one native rename -> post-ID/content observation.
- Creation: original runner/source acquire -> `ArchiveCreationSources` bounded held root/lineage,
  `GetFinalPathNameByHandleW(VOLUME_NAME_GUID)` enumeration with reopened identity check -> opaque
  ID reads -> Rust `creation`/`worker_zip_create` plan/encode/full held-partial readback/source check
  -> same native output publication. Output root never becomes Rust path-write authority.

## Path and authority invariants

1. Public requests remain ordinary local drive-absolute DOS paths with legal components, finite
   total length/depth and no truncation. Test CJK plus nested ASCII components and record both
   UTF-16 code units and UTF-8 bytes; every component stays <=255 UTF-16 units.
2. Internal C# `\\?\Volume{...}` / NT names derive only from continuously held authorized handles.
   They are not accepted public source/output prefixes and are never member names/permission.
3. Untrusted members remain relative: reject absolute, drive-relative, UNC/device, ADS, traversal,
   reserved devices and ambiguous components. Long-path support cannot sanitize or relax them.
4. Source full identity/version and ancestor no-delete/no-reparse leases survive actual worker,
   pumps and Job cleanup. Source-tree enumeration remains bounded and optimistic, not a snapshot.
5. Existing metadata/work/output/count/depth, one-call rendezvous, Job admission and timeout
   bounds stay unchanged. No 100k/1GiB/4GiB+17/ZIP64 performance or memory observation.

## Publication, cancellation and failure invariants

- New output: exclusive same-directory owned partial; full integrity and source checks before
  flush and no-replace publication. Verify expected bytes AND complete directory/file structure.
- Existing output: capture original approved identity, immediate held-relative fullID/version/hash
  recheck, close only that target check, then one atomic path replacement. Accepted close->publish
  TOCTOU remains explicit; no target-ID CAS, delete-first, backup-first, retry or fallback.
- Deterministic precommit cancellation/failure: prove barrier reached at the original commit;
  zero publication, original target unchanged, exact owned partial gone, external partial intact.
- Cancellation during CommitInFlight and postpublication verification failure cannot erase actual
  native success or reset FilesCommitted. Published/unverified/unknown output is never partial-
  deleted or rolled back. Assert count at native_published as well as final terminal where used.
- CleanupComplete means leases/process/Job/pumps/slots closed, not recursive directory rollback.
  Retain new small fixture inputs/outputs/evidence for independent inspection. Remove only the
  exact newly copied helper after actual cleanup; no source recycle/delete API, unknown sweep,
  system setting/LongPathsEnabled/registry/drive mapping, formal Desktop/installed host or packaging.

## Diagnostics and regression acceptance

Reuse operation correlation and production accepted/started/stage/terminal events. Assert
terminal/log category/native code/count/state/elapsed agree. Fixed fields only; no path, member,
content, password or raw exception in diagnostics, no per-chunk/per-entry events. Injected native
codes are explicitly fault injection, not claimed observed OS failures. Real platform errors
retain exact call/category/native code and fail the case; no SKIP as PASS or policy relaxation.

Minimum real cases:
- long source ZIP open and test via actual Rust product owner/typed IPC/production C# session;
  exact metadata/counts/bytes and cleanup/source release; reject public internal prefixes.
- long extraction root plus long CJK member path, independent literal bytes/empty directory/file
  structure and metadata; new output, confirmed overwrite, no-approval retention, precommit cancel
  and injected failure, during-commit cancellation/postcommit failure with truthful publication.
- long source tree plus long ZIP output, native enumeration/readback/publication, independent
  CPython zipfile exact members/content; same bounded publication scenarios where applicable.
- unsafe members at a long destination still refuse before publication, no outside writes.

Build isolated C# test apphost first, then matching isolated-codecs worker. One machine-wide Cargo
lane; serialize actual worker execution against relinking; all Cargo inherits BUILD_JOBS=1,
INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256. C# LIST exact safe filters before RUN and compare
names/selected/run/pass/skip/fail/not_run. Execute affected Rust/C# regressions, strict Clippy,
fmt, necessary shared ZIP gates; compilation is not workspace tests. Record exact commands/exits,
path lengths, oracle and cleanup facts. Finish docs/context/selector/diff/hash gates, update only
the current checkpoint and stop for independent acceptance.

## Preserved accepted/historical facts

[Post-status independent100k acceptance](product-100k-post-status-acceptance.md) is PASS for finite
100k correctness, Debug observation and same-original-connection postcleanup reuse. Do not rerun
or rewrite its evidence. Old Failed/Stopped attempt remains historical with unknown contender and
unverified followup IN THAT OLD ATTEMPT; no claim all writer races fixed. NativeRecycle remains
**native_protection_unproven/NotAttempted**. No Phase B or next feature.
