# HatchZip reference: five archive capabilities

## Scope and evidence

Reviewed 2026-10-27 at user request for **research and future-task handoff only**. No product implementation, external build/test execution, archive extraction, source truncation/punching, service/registry operation or packaging occurred. Existing NxTools UI/installation acceptance is unaffected.

Reference root: `D:\Code\hatchzip-main`. This supplied directory has **no Git metadata** (`git rev-parse HEAD` refused); do not invent a revision. Workspace package version is0.1.3, not a commit identity. [Source manifest](hatchzip/source-manifest.json) records reviewed file SHA256, byte/line counts and key-symbol coordinates. Recheck hashes before reuse; line numbers are navigation hints, symbols plus hashes are authoritative.

This is an original source-backed summary, not copied upstream documentation. Root code is MIT OR Apache-2.0; inspected LICENSE-MIT attributes Yash Bavadiya2026. Preserve applicable notices for substantial code reuse. RAR support compiles UnRAR under LICENSE-UNRAR, with its independent restriction against implementing RAR-compatible compression. Inspect exact dependencies/features before any adoption; no dependency change is approved by this research.

## Read in this order

| ID / capability | Focused note | Inspected source entry |
| --- | --- | --- |
| HZ1 Pre-extraction disk-space plan | [Space preflight](hatchzip/space-preflight.md) | core/session.rs::SpacePlan/plan; engine.rs::space_needed |
| HZ2 Clear progress / speed / ETA | [Progress presentation](hatchzip/progress.md) | gui/work.rs::Counters; runner.rs::tick; task.rs::tick; text.rs::Rate |
| HZ3 Batch task summary | [Batch summary](hatchzip/batch-summary.md) | cli/main.rs::extract_all/summary; gui/runner.rs::start_queue/step |
| HZ4 Crash/recovery testing | [Crash tests](hatchzip/crash-recovery-tests.md) | engine.rs::crash_point; cli/tests/{extract,keep,streaming,targz,restore,rar}.rs |
| HZ5 Extraction while reclaiming archive space | [Destructive in-place extraction](hatchzip/in-place-reclamation.md) | engine.rs::Run/commit/save_point; archive/storage.rs; journal.rs; restore.rs |

External source paths in the notes are relative to the reference root, not NxTools files. Only navigation and source existence were checked here; upstream tests/CI/performance statements are **not independently reproduced results**. README, TODO and implementation differ in places (e.g. RAR, NTFS punch coverage); source/tests are recorded separately from marketing/historical TODO status. Even docs/how-it-works.md has stale 'encrypted entries unsupported' and 'non-empty output refused' bullets: current session::open_encrypted and check_collisions support password flow and noncolliding pre-existing output. Do not copy those historical bullets into current requirements.

## Verified architecture and useful separation

HatchZip workspace is core/cli/shell/gui; GUI is **Slint**, not GPUI. CLI and GUI share session opening/space planning and core execution, but format numbers and handle human prompts separately. Work counters are sampled, not one event per byte. Core Unit represents a removable physical extent, which may contain several files; it is not our UI job, file counter or output budget.

Do not import Slint, the whole engine, a second job queue or the reference's defaults merely to reproduce these features. In NxTools: Rust owns planning/state/scheduling, C# owns Windows handles/space/native mutation, GPUI owns presentation. Original worker/source/output/lease/Job/cancellation owners remain authoritative.

## NxTools re-entry map (snapshot, not frozen workspace)

| Area | Existing owner to reread before changes |
| --- | --- |
| General task/progress wire | `rust/crates/nxtools-protocol/src/archive_job.rs` |
| Destination capacity / output identity | `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs::CheckDestination`, `ArchiveOutputSession.cs::InspectDestination`, `OutputNative.AvailableSpace` |
| Progress window | `rust/apps/nxtools-desktop/src/archive/progress.rs::JobProgressView` |
| Logical jobs, attempts, batch submission | `rust/apps/nxtools-desktop/src/archive/jobs.rs::submit_batch/pump_jobs/job_terminal`, `readmission.rs` |
| Admission only | `rust/apps/nxtools-desktop/src/archive/queue.rs::JobQueue` (observed2running/32queued/2GiBaggregate memory) |
| Source protection | `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveSourceLease.cs`, `ArchiveVolumeSources.cs` |
| Controlled output/publication | `ArchiveOutputSession.cs`, `ArchiveWorkerOutputBridge.cs` |
| Post-success recycle, NOT in-place reclamation | [Rust contract](../../.trellis/spec/nxtools-archive/backend/source-deletion.md), [native contract](../../.trellis/spec/platformhost/archive-source-deletion.md) |
| Existing real child lifecycle test | `tests/integration/archive-cli-lifecycle.py`, `ArchiveCliLifecycleFixture.cs` |

The main workspace has concurrent work. Do not infer a missing implementation solely from this dated note; read the latest [single checkpoint](../../.trellis/tasks/10-16-archive-manager/phase-a-checkpoint.md) and actual source. Do not relabel existing successful lifecycle fixtures as crash/restart or power-loss acceptance.

## Suggested future work, not an execution approval

1. HZ1/HZ2/HZ3: implement through existing owners; preserve unknown-size honesty, logical-vs-attempt IDs and current concurrency. Can be scoped independently after shared wire decisions.
2. HZ4: add real-process boundary tests alongside each durable feature; safe failure without source mutation is the starting baseline. Tests do not create recovery capability by themselves.
3. HZ5: separate explicit destructive mode with new capability/consent/recovery design; ordinary extraction must remain source-preserving. First prove smallest disposable fixture and Windows native ownership/durability, then expand formats. Do not start by loosening ArchiveSourceLease sharing or reusing DeleteSource/recycle authority.

User requested all five as subjects for later work, including HZ5. This round grants research/docs only, not execution on real archives, not approval of destructive defaults or a particular decoder/library. Product choices still requiring future approval are listed in each note. Preserve canonical A–F phase names/order; these HZ IDs are reference topics, not replacement phase numbers.

## Non-negotiable cross-topic distinctions

- Disk capacity estimate != output bomb limit != memory reservation != disk reservation.
- Processed/decompressed bytes != durably committed output != physically freed allocation.
- Pause in a live process != restart recovery != power-loss durability.
- Reconstructing a remaining archive != restoring the original complete archive.
- Recycle-after-full-success != truncate/punch-during-extraction; no permanent-delete fallback.
- Test injection, source review and upstream CI definitions != local compiled/native/user acceptance.

## Research delivery

[Task handoff](../../.trellis/tasks/10-16-archive-manager/hatchzip-research-handoff.md) lists future task boundaries and verification. The source manifest is reproducible evidence, not a security approval or embedded dependency. Documentation checks validate links/contracts only. Do not run any command in the reference on a user archive merely to 'verify' this note.
