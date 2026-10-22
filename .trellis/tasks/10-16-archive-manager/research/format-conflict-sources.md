# 7z/RAR/TAR existing-target conflicts — inspected boundary

2026-10-20. Phase A only; extend accepted ZIP policy, not a second transaction/framework.
Reference-first: docs/reference-code/archive.md lines80/167,299,320,365–377 and accepted
zip-conflict sources/acceptance. Legacy tracked ZipArchive/SevenZip/SharpCompress search again
finds only Services/BrowserExtensionPackageService.cs227 package ZIP. Archiver
D:\Code\Archiver-main\core\src\lib.rs620–630 7z callback uses File::create; TAR683 unpack_in.
SupaZip D:\Code\supazip-master\supazip\supazip-core\src\formats\sevenz.rs186 callback/413
File::create truncates. These are not reusable safe conflict/target-observation APIs. Retain Next
approved full partial -> target identity/version/hash recheck -> close only target check -> ONE
atomic-path publication, no delete/truncate/backup/fallback or strict ID-CAS claim.

Original schema2 worker source_acquire -> execute_attempt -> isolated try_execute or engine.open
and ExtractionPlan -> PrivateOutput -> original C#runner ReadFrames -> ArchiveWorkerOutputBridge
observe_target/begin_overwrite -> ArchiveOutputSession.BeginFile/Commit. C#one observation token
binds latest path/full target identity; Rust InteractionPort owns tickets/policy/apply-all/counts.
Native runner still owns suspended Spawn/AssignJob/Resume, source+helper/output handles, timeout,
cancel and actual exited/drain cleanup. No UI/layout/focus/native show/hide change.

Real output starts:
- worker_codecs.rs::run_sevenz for_each_entries callback currently begin_file before fixed-buffer
  read. Replace only that admission with existing begin_extracted_file; even None MUST read full
  member/update entry+total counters/CRC/size, so solid/next member remains valid. Keep first error,
  staged tokens until ALL block decode/source fingerprint -> source_precommit -> metadata/commit.
- worker_codecs/rar.rs::execute second pass currently begin_file before RARProcessFileW(RAR_TEST,
  null destination/name). None keeps token absent but MUST use RAR_TEST not RAR_SKIP; callbacks
  still count/check actual bytes. Native Handle closes before moving pending tokens. run_rar
  validates original first-pass paths before second pass and commits only after full decode/source.
- formats/tar.rs::extract planned-file loop currently begin_file -> bounded copy -> pending set;
  reuse admission, copy skipped payload to sink with same limits. Existing compound Gzip/BZip2/
  XZ/Zstd full outer-integrity spool stays BEFORE inner output. TAR has no payload checksum.

Necessary small changes: ZipConflict name -> ExtractionConflict; existing strategy/protocol stays.
PrivateOutput helper installs same policy/reserved entries and emits existing summary after attempt
(including failure). Isolated entry metadata may become Arc to avoid extra archive-name copies.
Keep actual decoded counters separate from published files/bytes when skipped; record members_skipped
from native skipped count, not decoder count. Existing engine already uses skipped_files eligibility.
Preserve ZIP capability for compatibility; additive extraction_conflict_v1 negotiates extended formats.
Only readmission retains ZIP signature guard; general conflict opts in only ZIP/7z/RAR/TAR-family,
not standalone streams/read contracts. No password/risk/readmission expansion or source deletion.

Tests: independent stock7Zip tiny solid3members and CPython TAR/TGZ; existing attributed RAR4/5
literal bytes. Original runner/Job/worker/native, old targets/foreign partial preserved, fresh/stale
original tickets, apply-all/Skip+following member, overwrite, numbered no-replace race, cancel,
internal unsafe/collision preprompt and summary/real commit count/qualification. Eligibility probe
uses existing fixture observer/never-called backend only, never Shell or source deletion.
