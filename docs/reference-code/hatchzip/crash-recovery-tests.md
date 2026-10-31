# HZ4 — Real-process crash and recovery test design

[Authority](../hatchzip.md); [snapshot hashes](source-manifest.json). Test source and CI configuration reviewed only. **No HatchZip tests, real process crashes, sparse-file mutation or power-loss simulation ran in this research.**

## Source entry and test mechanism

`crates/core/src/engine.rs:1712::crash_point` checks debug_assertions plus HATCHZIP_CRASH_AT; a process-local atomic visit count and HATCHZIP_CRASH_AFTER trigger `std::process::abort()`. Release ignores these controls. `test_knob` similarly gates small batches/stream steps. `crates/cli/tests/common/mod.rs:52::hatchzip` launches the compiled real CLI with only fixture-specific env; TempDir fixtures build actual archives and compare output trees/sidecars. This is failure at production commit seams, not a fake successful state machine.

Core boundaries inspected:

| Injection | Real boundary | Recovery expectation to inspect |
| --- | --- | --- |
| after-extract | Run::commit after output flush, before new journal | old journal is authoritative, uncommitted output redone; no further source truncation |
| after-journal | new committed_len durable, before Storage::set_len | source may still be longer; restart completes pending shrink then continues |
| stream-after-sync | output prefix flushed, old resume point still authoritative | trim output beyond old checkpoint only under owned recovery authority |
| stream-after-journal | checkpoint persisted before punch | new decoder point valid whether reclaim happened or not |
| stream-after-punch | source prefix may now be zeros | retain exact output prefix/decoder window; never restart at0 or delete indispensable partial |
| restore-appended | new index appended after restoring journal | retry undo-appended bytes or continue restore using original intent |
| restore-written |7zheader patches written, intent still present | restore originals before reattempt; preserve remaining payload |

No source mutation may lead its durable output/journal proof. Conversely, a journal declaring success before its output is durable is a safety bug even if normal-path tests pass.

## Exact suites and useful symbols

- `crates/cli/tests/extract.rs:87::resumes_after_a_crash_at_any_point`:2batchpoints x multiple visit counts, real child non-success, sidecars exist, rerun byte-equal output. `resuming_twice_in_a_row_is_harmless`; `extracts_zip64_archives_and_resumes_them`.
- `formats.rs::{resumes_7z_after_a_crash,resumes_tar_after_a_crash,split_zip_resumes_across_volume_boundaries}`.
- `keep.rs:99::an_interrupted_run_resumes_and_still_keeps_the_archive`: restart WITHOUT keep flag, old journal still forces keep; original volume bytes unchanged. `every_format_keeps_its_archive_byte_for_byte` is essential non-destructive baseline.
- `streaming.rs:133::crash_inside_the_entry`:3streampoints x multiple visits; output equality, archive length retained mid-entry, allocated-space observation when supported. StoredZIP/TAR/DeflateZIP; repeated crashes, final commit, damage retaining indispensable partial and restore refusal.
- `targz.rs::{a_tar_gz_survives_a_crash_at_every_step_of_a_resume_point,crashing_again_and_again_inside_one_tar_gz_entry_is_harmless,multi_member_gzip_extracts_and_resumes,a_split_tar_gz_resumes_across_volumes,damage_found_on_resume_loses_no_progress}`.
- `restore.rs:617::an_interrupted_restore_can_be_run_again_or_extracted`: ZIP/TAR/7z,2restorepoints, then either restore or extract. Reopened remaining archive + extracted files partition expected content. Native tool oracle helpers `check_with_unzip/check_with_7z/check_with_tar` exist; inspect availability/skip conditions before claiming oracle coverage.
- `rar.rs::{crash_at_every_commit_rar5_set,crash_at_every_commit_rar4_set,crash_at_every_commit_rar5_single_volume,crash_at_every_commit_rar4_single_volume,a_kept_solid_archive_resumes_after_a_crash,an_interrupted_restore_can_be_run_again}`; feature/license/build prerequisites apply.
- `large.rs::entries_past_4_gib_extract_and_resume`: sparse >4GiB offsets, not a measured huge-disk throughput benchmark.
- `core/src/fsutil.rs::punched_ranges_free_space_and_nothing_else`: punch success plus allocation drop and bytes outside interval unchanged.
- `.github/workflows/ci.yml`: Linux/macOS/Windows matrix, RAR feature, HATCHZIP_EXPECT_PUNCH=1. This is workflow intent, not inspected CI execution evidence.

## What these tests do NOT prove

Abort leaves OS/filesystem caches intact; it is not power removal. A child abort alone does not prove any Windows directory/rename durability claim. Source platform/windows.rs has sync_dir/sync_batch no-op functions with per-file sync; do not relabel those as verified power-loss transactions. Some allocation checks are conditional/skip; requiring expected filesystem support avoids 'green because never exercised'. Test counts in README are not this session's results.

CRC32 supports corruption detection where format supplies it; it is not authentication or proof that an arbitrary sidecar is authorized. TAR has no payload checksum. Fault injection must be Release-inert and separately checked for refusal; no production env switch that can terminate normal user work.

## NxTools testing seam

Keep existing worker/PlatformHost/CLI lifecycle and finite readmission gates. `tests/integration/archive-cli-lifecycle.py` already drives exact owned children/pipe cancellation/cleanup checkpoints; use its ownership pattern rather than process-name kills. It currently proves cleanup/lifecycle cases, not an already-existing persisted restart feature. Inspect C# ArchiveOutputSession, worker protocol/terminal and disk journal format before choosing durable hooks. An in-memory CommitJournal or serialized terminal report is not automatically a restart log.

Start from source-preserving ordinary extraction: crash must not alter source; output partially committed state remains truthful; no arbitrary cleanup. Add recovery implementation before asserting resume succeeds. If recovery is unsupported, test a safe explicit refusal with intact evidence, not fake success. HZ5 requires its own new durable mutation protocol and disposable fixtures after approval.

## Future test matrix / acceptance

HZ4-A: use GUID-owned scratch archives/targets/keys; exact Popen PID/handle, bounded checkpoint waits, capture exit+stdout/stderr/evidence without secrets; never broad kill. HZ4-B: first prove fault hook actually fired and original boundary was reached; wrong failure elsewhere is not red. HZ4-C: restart a fresh process with journal/source/output fingerprints, not reused in-memory state. HZ4-D: compare exact file set/content/metadata and source/sidecar state, no stray partials and unrelated files unchanged; inject user replacement/reparse/hardlink conflicts, malformed/missing/newer/older journal and tail. HZ4-E: crash before/after output publish, journal atomic replacement, source reclaim and final cleanup; repeat crashes, source moved/modified, missing volume, disk full, write/flush failure, password cancel/wrong, IPC loss and child exit. HZ4-F: keep-mode and ordinary extraction unchanged by every destructive hook. HZ4-G: verify test controls disabled/refused in real Release. HZ4-H: separately authorized disposable-VM power-cut/filesystem/native acceptance, explicitly distinct from process tests.

Every result should retain exact case name/format/point/visit, input hashes, observed durable state, launched child identity, restart outcome, final output hashes and actual PASS/FAIL/SKIP. No broad 'crash safe' claim from one happy restart. Diagnostics remain bounded transition events; test-only checkpoint data never enters user-facing logs.

## HZ4 implementation discovery — 2026-10-29

Rechecked the nine recorded HatchZip files against `source-manifest.json`; no recorded hash drift.
Read actual engine1712/debug gate, commit827/flush845/journal855/keep857, CLI common52/61,
extract87/124, formats107/154, keep63/99, streaming133, restore617 and CI59–85. No reference
test or destructive source mutation was executed; checkout still has no claimed Git revision.
The useful transplant is real-child/seam/visit/fresh-process/independent-oracle evidence, NOT
HatchZip's truncation, punch, decoder journal or restore authority. Windows sync_dir/sync_batch
remain no-ops in the inspected reference; process termination does not prove power-loss durability.

Newly verified NxTools chain: original `ArchiveJobSession.Dispatch`77 -> `ArchiveWorkerRunner`
488/output bridge +504/suspended owned worker +821/source checkpoints -> format worker:
ZIP383 original fully validated candidate,386/edit_verifying,409/native staging,431/hash readback,
436–440/actual reader drops then edit_reads_complete;7z575/encoded verify,599/stage/readback,
606/full verify,616/drop,626/completion;TAR393/source drop,394/inner verify,403/outer encode,
414/staging,421–427/outer+inner verify/drop,439/completion. Those are different codec seams;
none is a persisted restart checkpoint. `ArchiveWorkerOutputBridge.PhaseE.HandleExtendedCall`
53–58 marks candidate verified and obtains original single-use read completion. The existing
`CommitVerifiedEdit`411–422 rechecks held source/target and releases only source FILE inside
original `CommitCore`;472/commit_in_flight is after that release, before the one native rename;
483/native_published is after native success and publication tracking, before output close.
Runner575/cleanup probe is after actual worker/pump drainage but before resource disposal;
Job terminal follows587/in-memory journal freeze and actual cleanup. Reuse these owners/probes.

Restart capability finding: `ArchiveOutputSession.transactions/entries` and JobSession.active/
retired are process-local collections; CommitJournal81 is a projection, not a disk write.
Runner587 and JobSession156 serialize terminal facts only after cleanup. No archive-job durable
recovery record/parser/replay or trusted restart cleanup authority was found in these owners.
`ArchiveJobSession.Dispatch`60–66 rejects old process control/resume as `stale_operation`;
fresh-process tests must prove that exact refusal and unchanged disk facts, not feed an old
terminal back as authority. EntryCache temp-lease recovery is a separate owner and cannot grant
ordinary extraction/edit cleanup permission. No legacy NxTools general archive/crash owner was
identified (see `../archive.md`16); unrelated update/cache recovery is not reused as authority.
Private fixture envelopes/checkpoint receipts are not authenticated recovery journals.
Tests may retain owned partials after abrupt Host death; no recursive/path-based cleanup is allowed.

