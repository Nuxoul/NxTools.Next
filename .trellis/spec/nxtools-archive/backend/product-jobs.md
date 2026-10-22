# Product archive jobs and finite output readmission

## 1. Scope / Trigger
Phase D typed create/extract/test dispatch over the original worker. Public ArchiveRead remains
read-only. This source contract does not certify live Desktop/Shell or full Phase A–F acceptance.
Evidence: `repo:rust/crates/nxtools-archive/src/product_job.rs`,
`repo:rust/crates/nxtools-archive/src/worker_job.rs`,
`repo:rust/crates/nxtools-archive/src/worker_output.rs`,
`repo:rust/apps/nxtools-desktop/src/archive/jobs.rs`,
`repo:csharp/NxTools.PlatformHost/Protocol/ArchiveJobSession.cs`.

## 2. Signatures
`product_job::execute_with_output_readmission(command, control, interaction, run)` adapts typed
jobs to original `output_readmission::execute`; run returns native terminal facts only after exited
cleanup. Logical ID, native attempt ID, transport receipt, session/generation and prompt ticket are
distinct. `PrivateOutput::preflight_output` preserves source binding and reports finite shortages;
`destination_preflight` carries optional known planned bytes to the original held-output owner.

## 3. Contracts
Desktop is admission/presentation, not codec execution: strict FIFO max2running/max32queued,
aggregate2GiB/per-worker1GiB, overlapping source/output serialization and cleanup-held slots.
All extraction formats enter the same finite coordinator; only fully known original planned
regular-file sizes can produce a shortfall. Preserve original selected ordinals, source digest/native
identity, conflict/MOTW/password and work/metadata limits. Retry is at most one fresh attempt and
changes only entry/output byte limits after matching zero-commit/actual-cleanup evidence. Unknown
stream overflow remains terminal ResourceLimit, never a guessed shortage. Compressed-TAR header
discovery uses the existing verified anonymous spool bounded by max(output,metadata); an overflow
before known inner sizes is terminal. Extraction uses the original target limits, not spool limits.

ZIP and meaningful independent-block7z/non-solid nonsplitRAR member ratios reuse ZipRisk and the
original InteractionPort: >64MiB AND >1000x known positive packed bytes. Shared solid/split/unknown
ratios stay unknown. Approval equals the existing admitted entry ceiling; it never raises budgets.
C# observes writability/free space from the held namespace/volume before directory effects. Missing
root/known low space requires a current ticket; unknown output remains unknown. No probe file,
elevation or automatic approval. Rust supplies layout/policy, C# native handles/API/results.

Native publication facts survive cancellation/error and post-create test failure. Post-create Test
is a fresh original attempt under the same logical slot, with one final result/notification. Actual
cleanup owns retirement; disconnection is not zero-output proof. Pending state precedes dispatch;
old attempts/receipts/tickets cannot affect current work. Cross-entity callbacks defer, including
immediate results. Progress<=10Hz, bounded lossless-or-fail-stop prompt/terminal, existing taskbar/
NotificationCenter owners. Source deletion is visibly unavailable; strict recycling remains unaccepted.

## 4. Validation & Error Matrix
Invalid hidden format options/single-stream multiple roots -> reject before execution. Declared
shortage -> ResourceLimit+bound facts before target effects; native failure -> original code, not
shortage. Changed source/target -> typed refusal/no unsafe retry. Actual overflow -> terminal and
owned-partial cleanup. Commit-bearing/unclean/mismatched shortfall -> no new attempt. Stale reply ->
ignored. Cancel preserves native committed journal; cleanup unknown retains the running slot.

## 5. Good/Base/Bad Cases
Good: tiny known TAR shortage -> actual exited cleanup -> explicit finite approval -> one bound
native attempt. Base: ordinary ZIP/no shortage completes once. Bad: suffix proves capability,
solid block copied into every member ratio, UI retry loop, private complete releases a Job slot,
late cancellation zeroes commits, or source path deletion substitutes for strict recycling.

## 6. Tests Required
`repo:rust/crates/nxtools-archive/tests/worker_extraction_contract.rs` drives original workers for
non-ZIP shortage/source/risk/unknown overflow. `output_readmission_contract.rs` and
`job_execution_contract.rs` guard finite admission and projection. Production Desktop regression:
`repo:rust/apps/nxtools-desktop/src/archive/job_tests.rs`. Actual native boundaries:
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveJobTests.cs`,
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveCreateOverwriteTests.cs`,
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveDestinationPreflightTests.cs`.
Exact LIST->sameRUN with matching isolated worker is required; TestWindow/native fixtures are not
live GUI/user acceptance. Keep full-suite failures and earlier-phase obligations separately visible.

## 7. Wrong vs Correct
Wrong: publish form submission as success or enlarge an already admitted Job. Correct: immutable
validated draft -> original bounded FIFO/worker/native transaction -> actual exited cleanup ->
truthful once-only terminal; explicit finite output readmission is a separate fresh admission.
