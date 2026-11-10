# Same-stage protected archive observations

## 1. Scope / Trigger

Changes to source/target full reads in the original ZIP edit lease/output chain. Owners: `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveSourceLease.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveNativeReadObservation.cs`, and `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs`. Rust format reuse is separately specified in [operation-local plans](../nxtools-archive/backend/reusable-plans.md). No global proof table, hash file or new native authority is introduced.

## 2. Signatures

- `ArchiveSourceLease.OpenForEdit` captures original native source/target facts under no-write/no-delete protection.
- `VerifyEditSource(string expectedTargetIdentity, ArchiveNativeReadStage stage, CancellationToken cancellationToken, ...)` performs a fresh complete source observation and may compare a currently held target in that same call.
- `CaptureEditSource(ArchiveNativeReadStage, CancellationToken) -> (object Version, byte[] Hash)` brackets the actual source hash with full version capture; the temporary hash is zeroed when retired.
- `ArchiveOutputSession.RecheckTarget(..., stage, ..., editSource, ...)` passes its live target to source verification. `CommitVerifiedEdit` / `CommitCore` retain read-completion, FILE-only handoff and final native publication.
- `ArchiveNativeReadObservation.Hash(Stream, stage, kind, CancellationToken) -> byte[]`; `Snapshot() -> Counts[]`. Counts are worker-owned, fixed 9-stage by 3-role counters, never reusable source evidence.
- `ArchiveWorkerRunner.DiagnosticTerminal(Run, ArchiveWorkerEvent)` projects the resolved terminal and existing `Run.Source` / primary output observations after cleanup. It does not read source/output files.

## 3. Contracts

Reuse requires full `ArchiveRecycleVersion` equality of the continuously protected live source and target, including complete native identity, size/mtime, change time and attributes. Path, same size/content, mtime or a caller Boolean is insufficient. A newly captured source version must equal the before and after versions of its complete hash. Equality permits reuse only of that current call's observation, with no intervening release/write/delete protection loss.

The five fresh stages remain distinct: `EditOpen`, `EditVerification`, `EditReadsComplete`, `EditHandoff`, `Prepublication`. Do not carry a digest between them. `TargetAdmission`/`TargetBegin` remain separate target observations; `OutputFingerprint` and `OutputPostpublication` remain separate full output reads. Exact held-partial readback is a different role and is not credited as a source observation.

Original read-completion is single-use and bound to the lease. Worker/source readers must actually close before the FILE-only source release; parent/candidate protection remains held. Final target checks, flush/fingerprint, one atomic native publication and postpublication verification still run. Committed or unknown facts take precedence over late cancellation; postpublication verification completes to establish truthful output facts rather than aborting into a false unpublished result.

Hashing checks the original cancellation token between 64KiB reads. Acquisition, edit verification, read completion, target admission/begin, handoff, fingerprint and prepublication propagate that token. A full read is credited only after actual EOF and digest completion. An interrupted attempt retains actual returned bytes but no completed-read credit. The fixture chunk callback is instance-only and never installed by production callers.

Diagnostics reuse the original operation correlation and bounded sinks. The existing `archive.worker.completed/cancelled/failed` terminal carries `native_source_*`, `native_target_*` and `native_output_*`, each with four signed 64-bit counters: `attempts`, `full_reads`, `read_bytes`, `completed_bytes`. `read_bytes` includes bytes returned by interrupted attempts; only EOF plus digest completion contributes to `full_reads` and `completed_bytes`. `partial_read_calls` / `partial_read_bytes` independently count held-partial readback, including its EOF call.

The terminal aggregates numeric observations retained by the original Run/source/output owners after disposal; source verification interrupted before bridge binding still uses `Run.Source`. It reuses the resolved event's category, correlation, elapsed time, native code, commit and actual cleanup facts. A throwing sink or projection failure cannot alter that event or file result. The older `archive.output.*` fields `native_full_reads` / `native_read_bytes` are output-owner totals, not source totals or role-separated output payload reads. No paths, identities, hashes, payloads or per-chunk events are exported. Counters cover observed owner reads, not complete process/disk I/O.

## 4. Validation & Error Matrix

| Condition | Result |
| --- | --- |
| Same live full identity/version under uninterrupted protection | One full source/target observation in that stage |
| Different ID with identical size/content, or changed attributes/change time | Original changed-source/target refusal; no reuse |
| Wrong lease/read completion, reuse of consumed completion, or live readers | Original handoff refusal; no native replacement |
| Cancellation before publication | Original cancellation and owned partial cleanup, no fabricated commit |
| Native publication failure/unknown | Preserve native code and failed/unknown journal; no hidden retry |
| Late cancellation after commit | Preserve committed/published facts and finish postpublication verification |

## 5. Good/Base/Bad Cases

Good: EditHandoff hashes the protected source once, compares the concurrently held target's full version, and shares only that stage's observation. Base: Prepublication independently repeats the source observation; output fingerprint and postpublication both remain. Bad: compare same-size paths, reuse EditOpen's hash at handoff, convert a diagnostic counter into a proof, or release parents/candidate together with the source FILE handle.

## 6. Tests Required

`repo:csharp/NxTools.PlatformHost.Tests/Opt05NativeObservationTests.cs` covers same-object equivalence, identical-byte different identity, version/Hidden changes, eight interrupted chunk paths, cancelled acquisition, FILE-only and protected parents/candidate, single publication, late cancellation and actual exclusive-reopen cleanup. On the retained 8,388,730-byte source, old-object full reads are 14 before and 7 after; source/target bytes are 117,442,220 and 58,721,110. Output full reads remain two; held-partial readback remains 8,388,728 bytes. Keep all three completed native trials.

`repo:csharp/NxTools.PlatformHost.Tests/Opt05NativeObservationTests.Terminal.cs` runs the actual browser/worker/Run/native same-path edit and asserts the production terminal sink, not test-only snapshots. Success requires source5/target2/output2 full reads, exact role bytes, separate held-partial calls/bytes, one publication and exclusive reopen after actual cleanup. Cancellation in established-lease `EditVerification` requires source attempts2/full1, actual bytes `source_length + 65536`, completed bytes `source_length`, unchanged source and the original cancelled terminal. A rejecting sink retains successful publication and cleanup. Removing any role export must fail its assertion.

Original ZIP edit native tests retain changed revision, source reader close, wrong/consumed completion, failed/unknown/no-retry and late commit. Review effects, exact LIST then identical RUN against the actual apphost; inspect selected/run/pass/skip/not_run. Zero selection or skip never accepts native behavior. Real Explorer, user sources and Release remain separate gates.

## 7. Wrong vs Correct

Wrong: held source hash plus independent equivalent-target hash at the same stage, or a cross-stage cached digest treated as freshness.

Correct: fresh before/full-read/after source observation -> compare complete currently held target version -> reuse only within that protected call -> retire temporary hash -> retain the next fresh stage and original publication/cleanup owner.
