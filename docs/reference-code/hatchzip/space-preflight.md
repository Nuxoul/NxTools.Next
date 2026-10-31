# HZ1 — Pre-extraction disk-space planning

[Research authority / evidence limits](../hatchzip.md). External coordinates below are relative to `D:\Code\hatchzip-main`; [manifest](source-manifest.json) fixes file hashes. Source inspected; no execution or measured local performance.

## Entry map and call chain

| Source | Verified responsibility |
| --- | --- |
| `crates/core/src/session.rs:306` SpacePlan | free/needed/keep_free/fits/free_after; normal_needs/in_place_needs; either/reclaims/solid_block_bound/archive_kept_whole |
| `crates/core/src/session.rs:356` plan | current destination free bytes + presumed stale output reclaim; same-filesystem check; mode-specific peak and terminal net |
| `crates/core/src/engine.rs:438` Job::space_needed | replay physical units in reverse offset order; accumulate peak extra allocation and net end cost |
| `engine.rs:486` stream_cost / `:527` solid_cost | large Stored/Deflate and compressed-tar peak estimates, resumed bytes and punch alignment/buffer allowance |
| `engine.rs:1295` ensure_room / `:1319` next_batch | remeasure capacity at runtime, subtract reserved margin; refuse next step/batch when short |
| `crates/core/src/fsutil.rs:10` available_space | nearest existing ancestor + fs4; debug-only HATCHZIP_FREE override |
| `fsutil.rs:69` can_punch / `:111` punch_probe | create1MiB noisy scratch file beside source; flush, compare allocated bytes before/after actual punch |
| `crates/gui/src/work.rs:228` extract | open -> plan -> NoSpace or Planned -> optional verify -> begin journal -> Run |
| `crates/gui/src/text.rs:71` plan_view | shortage/verdict/format-specific explanations, not just a red percentage |
| `crates/gui/ui/parts.slint:78` SpacePanel | available, both mode costs, reserved, free-after and verdict |
| `crates/cli/src/main.rs:268` report | CLI presentation of the same shared plan |

Fresh GUI: Request::Extract -> work::open -> session::open_encrypted -> State::Fresh -> plan -> optional Job::verify -> session::begin. Resume follows the old journal's output/mode, plans remaining work, then clear_uncommitted. CLI shares core plan, not an independent calculator.

## Actual calculation, not total-minus-archive shorthand

Units represent physically releasable extents (file/solid block). Engine build sorts them by decreasing source offset. Normal extraction does not credit source reclamation. For ordinary units, space_needed begins with saved-tail cost when fresh, then for each unit:

1. peak=max(peak,current_extra+rounded_output_cost);
2. current_extra+=output_cost;
3. only when reclaims, current_extra-=source_span;
4. final accounting includes initial stub/tail removal.

Streaming paths substitute stream_cost/solid_cost. Reference output allocation estimate uses4KiB rounding rather than discovering every filesystem's actual cluster/compression/quota behavior. Source-data gaps, largest solid block, decoder resume boundary, retained buffers and granules matter. `fits = free >= needed + keep_free`; default GUI reserve is2,000,000,000 decimal bytes in gui/settings.rs, a reference choice **not an approved NxTools default**.

Cross-volume output cannot spend source-space savings on the target. A single huge7zsolidblock needs full block output before reclaim; FAT/exFAT may truncate but cannot mid-file punch. Capacity can change after planning, so runtime rechecks still matter. Unsupported/error/unknown volume observations are not zero or proof of fit.

## Side effects and limits to preserve explicitly

- `check` is advertised as archive-read-only and tests assert archive/sidecars unchanged. Capability estimation may nevertheless create/delete a probe file. Do not call it a wholly no-write operation.
- plan adds sizes from uncommitted outputs to available space on the assumption they are disposable. NxTools must not count foreign/replaced files as reclaimable, nor delete them based on paths from an untrusted journal.
- available-space observation is not a reservation. Other processes, concurrent jobs, filesystem allocation granularity, quotas, sparse/compressed files and snapshots can invalidate forecasts.
- Prefix-based Windows same_filesystem is reference behavior, not sufficient volume identity proof for mount points/aliases/reparse/UNC. Use our native held-directory/volume identity.
- Compressed-tar planning scans the stream; GUI can scan once for browsing and again before extraction. TODO explicitly identifies this latency; do not inherit the duplicate scan as an optimization.
- Arithmetic must be bounded/checked across byte totals, reserve, peak/net signed values and entry limits; do not copy unchecked additions as contract.

## NxTools adaptation boundary

Existing `ArchiveWorkerRunner.cs::CheckDestination` calls `ArchiveOutputSession.cs::InspectDestination` and native GetDiskFreeSpaceExW through a stable directory. Missing destination and insufficient capacity currently feed original Destination prompt. `progress.rs` renders it. Extend that owner with a typed bounded plan observation rather than GPUI filesystem queries or a new execution service.

Rust computes bounded/honest plans from the same selected members/options/source generation; C# supplies target capacity/allocation/native observations; GPUI renders and permits refresh. Bind plan to target/options/source revision and revalidate before execution. Unknown member sizes stay unknown or a clearly labeled lower bound. Header declarations alone are not actual decoded output or trustworthy bomb admission.

Keep disk reserve, working-memory admission and max_entry/max_output separate. User override of a capacity warning must not enlarge output budget or enable source mutation. Concurrent original JobQueue jobs require per-volume advisory accounting/recheck; not a silent global serialization redesign. Missing target inspection should not create the target merely to show a plan. Decide future UX for reserve default, hard refusal versus explicit warning, and whether an explicit temporary capability probe is acceptable.

## Source-backed tests to follow (NOT RUN here)

- `crates/core/src/session.rs` tests: normal/kept plan and output naming.
- `crates/cli/tests/extract.rs::refuses_to_start_without_enough_space`: archive unchanged and no output root.
- `crates/cli/tests/keep.rs::the_space_plan_for_keeping_counts_every_file`.
- `crates/cli/tests/streaming.rs::the_space_check_counts_freeing_inside_the_entry_only_where_it_works`.
- `crates/cli/tests/targz.rs::the_space_check_for_a_tar_gz_counts_freeing_only_where_it_works`, `without_hole_punching_a_tar_gz_is_kept_whole_and_still_resumes`.
- `crates/core/src/fsutil.rs::punched_ranges_free_space_and_nothing_else` measures allocation, not merely successful return.

## Future acceptance checklist

HZ1-A: known/unknown/selected members, empty/huge/split/solid archives; bytes match original calculation inputs, no invented zero totals. HZ1-B: missing/denied/replaced target, true same/different volumes, mount aliases; plan invalidated on changes. HZ1-C: insufficient/zero space and reserve boundaries, overflow, concurrent jobs/disk pressure; original budgets unchanged. HZ1-D: no archive/output mutation during advisory preflight; any probe disclosed/isolated/exact-owned cleanup. HZ1-E: actual UI labels and compact layout distinguish available/needed/reserved/estimated-after, loading/error/unknown. Estimate versus actual peak measured on disposable fixtures, with discrepancy reported rather than hidden.

## NxTools implementation follow-up (2026-10-28, PARTIAL)

The separate authorized implementation keeps reserve0/no source credit and original runtime
destination confirmation. Actual `ArchiveOutputSession::Prepare/ObserveSpace` holds/rechecks the
nearest local no-reparse ancestor and reports GetDiskFreeSpaceExW/GetDiskFreeSpaceW plus native
volume/full directory IDs and exact missing-suffix hash without output creation. Original detached
dispatcher/boundedSTA owns at most2native observations; timeout never replaces hung work. Rust
estimates only a bounded retained browser snapshot/all-or-selected request, including allocation
rounding/directory allowance; unknown metadata and compressed-TAR spool remain explicit unknown.
No second scan, source truncation/punch/recycle, enlarged budget or new hard refusal was added.

Actual core/native tests are not full acceptance: final related273PASS/1FAIL and strictlintFAIL1;
closed-form UI pending retirement and borrowed-path lint remain OPEN under the session edit stop.
Completed-plan disconnect invalidation was reproduced/fixed. See exact commands/RED/GREEN/limits:
[HZ1 outcomes](../../../.trellis/tasks/10-16-archive-manager/hz1-space-preflight-outcomes.md),
[native observation](../../../.trellis/tasks/10-16-archive-manager/hz1-native-observation-outcomes.md).
The original research/acceptance checklist above is retained, not relabeled as executed upstream.
