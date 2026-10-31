# HZ3-AC01–03 bounded repair route

2026-10-29. Remains **Phase E — Format expansion and ZIP editing**, Step5 HZ3 only. Parent acceptance is withheld in `hz3-acceptance.md`; its three counterexamples and all earlier evidence remain immutable. No HZ4/HZ5, new scheduler/history/recovery manager, review/delegation or release/system action.

## Opening and authority

Actual full checkpoint saved before edits:202215bytes, SHA256 `41DEF2F0EA5091141D6FC4AE668E8EFB0C381AAAADCB91E87CBF0AFF4434EC27`, scratch `hz3-ac01-ac03/checkpoint-before.bin`. Only top prepend after final reread; prove this entire opening exact suffix. Opening status/HEAD/source and earlier reports are saved separately. Existing dirty changes are not reset/staged/committed.

Reread HatchZip `batch-summary.md` and its original inspected call-chain map; borrow presentation, not its serial/stop policy. New native evidence follows actual `ArchiveOutputSession::{CreatePreparedRoot,TrackDirectory,EnsureDirectory,CommitCore,Dispose}` -> `ArchiveWorkerRunner::{ExecuteAsync,ResolveTerminal}` -> `ArchiveJobSession.Worker` -> Rust `job_notification` -> finite readmission `Delivery::Finished` -> `job_terminal` -> `batch_snapshot_for/retry_job`. Source references and evidence limits remain in the reference note.

## AC01 vertical slice

Original output owner must expose an optional/default-compatible read-only `output_published: bool?` terminal fact, symmetric Rust/C#. True means a published file or surviving native-created directory/completed directory output; false is affirmative complete zero-output evidence; absent/null is unknown, never zero. Include prepared root/wrapper/ancestor directories retained on failure, not merely explicit directory requests. Record directory publication immediately at original native creation boundary, before later verification/metadata can fail. File CommitJournal remains file-only; uncertainty retains null, never manufacture false. Resource cleanup is not rollback. Preserve completed-directory-only result_location and do not delete directories.

Regression seam approved by the task: actual owned Job+matching worker+native metadata-failure fixture exports measured terminal/source-target baseline; actual production app+PlatformBridge consumes that terminal and checks published summary/no retry. A genuine failed file-before-publication fixture supplies the eligible zero-output control. Missing legacy facts also refuse. Tests first, classify fixture/compile failures separately from safety RED.

## AC02 vertical slice

Bind each job observation to logical ID, concrete native operation ID and retry purpose. Original attempt dispatch/terminal/local-failure owners maintain first-attempt observation eligibility; a native terminal closes first-baseline acquisition BEFORE delivering to finite coordinator. Attempt transition invalidates pending old observations and cannot acquire a replacement first baseline if the initial context was missed. Preserve an already valid immutable baseline. Retry recheck belongs to the completed attempt and cannot dispatch twice. Receipt/connection/session/generation and native-attempt lifecycle all gate results. Fail closed in logical-terminal gaps.

Regression seam: real app/original bridge delivers native terminal and delayed ArchiveSpace in the SAME application turn (logical Finished not processed), then finite transition before old observation. Replacement-source observations cannot create baseline or replay. Valid current pre-native-terminal observation positive; finite logical member/terminal once. No physical timed source-swap claim.

## AC03 vertical slice

Repair original `Jobs.retired` owner: bounded one-pass sweep requeues protected IDs, removes only reclaimable closed completed records and preserves oldest reclaimable64 policy. Do not spin if all protected. Keep active/cleanup-held/retry-pending/pending-confirmation protections and recover tracking after they end. Invoke original sweep on relevant protection-release lifecycle if needed; no separate history manager.

Regression seam: actual app submit/completion notifications through original bridge, closed A plus active/late B, >64 other completions, B done then further pressure. Assert no orphan untracked completed records and eventual bounded reclaim; positive active/held/pending protection and original two slots. Never enlarge channels to hide overflow.

## Preserved contracts / diagnostics / verification

Queue2running/32queued/2GiB, FIFO/head-overlap, source/output serialization, cleanup-held slots; immutable source/ordinal/attempt/budgets, reader closure/edit_reads_complete, FILE-only native handoff/single publication and accepted EIC/codecs unchanged. No ordinary UI redesign: original official Kit0.8.0 Table/Label/Alert/outlinedMedium32, parent/deferred lifecycle/focus/secrets/footer unchanged. New fields are facts, not permissions; terminal precedence unchanged. Existing correlated batch/job/attempt/receipt/terminal/elapsed/native-code diagnostics remain; no payload logs/probes/per-tick events or new clock/logger.

Every Cargo invocation checks idle then same-process jobs1/incremental0/codegen-units256, locked/offline/serial and full timeout; no duplicate process/name-kill. C# effects audit -> exact LIST -> identical RUN, explicit matching frozen worker/hash/oracles, skip not pass. Precise RED/GREEN, affected protocol/IPC/desktop/native compatibility, strict/workspace/fmt/spec gates, protected drift and unrun layers recorded in independent repair-outcomes. Original SourceChanged-vs-Cancelled/source_unavailable/helper_untrusted/FB-AC05 and physical/UI/user/installed acceptance stay OPEN. Stop for host callback and narrow parent acceptance, not automatic next step.
