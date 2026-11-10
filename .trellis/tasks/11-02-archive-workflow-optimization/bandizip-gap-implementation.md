# Archive extraction work reduction — 2026-11-05

Parent: **Phase E — Format expansion and ZIP editing / in_progress**.

## Result

Two operation-local optimizations are implemented and verified at the automated/owned-native boundary:

- Default protected, unencrypted single-volume ZIP Test/extract jobs retain the catalog already opened for password admission. One catalog and four full source revisions replace two catalogs and six revisions. Execution-before/final source checks, source/browser binding, selection, complete payload integrity and native publication remain.
- Cache retirement keeps its durable Retired stamp and the original verified deletion, but does not persist OutputRemoved immediately before destroying that manifest. Restart accepts the pending retired record and rechecks the original output or its absence, including when the owned root is already gone. Root/ancestor replacement, unknown files, edited/locked content and active consumers remain protected.

The supplied ZIP comparison measures lower ordinary extraction, controlled Root completion and receiver-end cleanup times. It does not establish Bandizip parity or real file-manager visual acceptance.

## Product changes

`repo:rust/crates/nxtools-archive/src/worker.rs#OpenedAttempt` privately owns the original native source-lease borrow and optional admitted catalog. `job_password_format` transfers only an unencrypted catalog protected by that lease. `execute_opened_attempt` enters the original binding, planning, execution and terminal path with it. `execute_attempt` retains the independent entry used by password preparation. Encrypted, unleased, split, modification and reclamation paths keep their existing authority and checks. No opened state enters a public request/snapshot field or persistent/global cache.

`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs#Release` preserves the pre-deletion Retired Persist/Flush, identity/metadata/MOTW/content validation and exact-object deletion. Its OutputRemoved transition is in-memory. The original fixed-parent retirement record remains valid with Retired=true and OutputRemoved=false; recovery must verify the original object or observe its absence before directory retirement. Root identity checks and target-owned stream/consumer retention remain unchanged.

The product delta is confined to those two files. Public/private wire formats, one-call output acknowledgements, buffer sizes, concurrency, timeouts, CRC/MAC, native publication and real cursor checks are unchanged.

## Same-source Release comparison

Input: 137737166-byte ZIP, 215 catalog records, 208 regular files and 206572163 decoded bytes. Each side completes three ordinary Extract Here operations and three Root operations through the same controlled COM receiver. Every operation is independently byte-compared against streamed .NET ZIP contents outside its timed interval. All 12 operations pass output/count/cleanup checks; no member is executed and the source is unchanged.

Both sides use the same stable test-apphost and worker execution paths, fresh owned outputs, Release mode and the original Read/Job/Entry/worker/native-output pipeline. The baseline freezes the pre-change Release worker and pre-change cache assembly; the candidate contains the changes above. Builds are absent at measurement start/end. No system cache flush is performed; all observations, including each first trial, are retained.

### Operation wall time

All values are milliseconds.

| Route / side | Trial 1 | Trial 2 | Trial 3 | Median |
| --- | ---: | ---: | ---: | ---: |
| Ordinary Extract, before | 3400.1158 | 2981.6114 | 2965.1757 | 2981.6114 |
| Ordinary Extract, after | 3298.9682 | 2752.4745 | 2798.5288 | 2798.5288 |
| Controlled Root, before | 7756.0336 | 7608.1170 | 7936.9318 | 7756.0336 |
| Controlled Root, after | 7589.0674 | 7482.8169 | 7505.0565 | 7505.0565 |

Observed median reduction: ordinary Extract **6.14%**, controlled Root **3.24%**.

### Receiver End to complete cleanup

| Side | Trial 1 ms | Trial 2 ms | Trial 3 ms | Median ms |
| --- | ---: | ---: | ---: | ---: |
| Before | 1331.8161 | 1350.5045 | 1297.2440 | 1331.8161 |
| After | 1079.8778 | 1122.2533 | 1102.2282 | 1102.2282 |

Observed median reduction: **17.24%**. Root's principal materialization/verification/copy route remains; the retirement change primarily reduces its final cleanup work.

### Work and integrity

| Per operation | Before | After |
| --- | ---: | ---: |
| Ordinary catalog loads | 2 | 1 |
| Ordinary full source revisions | 6 | 4 |
| Ordinary revision bytes | 826422996 | 550948664 |
| Root catalog loads / source revisions | 1 / 4 | 1 / 4 |
| Binary output records | 6436 | 6436 |
| Decoded output bytes | 206572163 | 206572163 |
| Native output full readbacks | 416 | 416 |
| Native output readback bytes | 413144326 | 413144326 |
| Root Seal checks / verified cursors | 208 / 208 | 208 / 208 |
| Root Seal hash bytes | 206572163 | 206572163 |
| Root cursor hash bytes | 206572163 | 206572163 |

Ordinary execution avoids 275474332 source-revision bytes per operation. Successful first cache release has one fewer manifest Persist/Flush call; the sample owns 208 such leases. Full native publication, Seal, each actual cursor and deletion-time content verification remain in place. Stage clocks overlap and are not additive.

## Regression and verification

- `ArchiveOptimizationTests.SharedReadWork`: original native Read/Job/Entry routes; strict one-catalog/four-revision assertions for Test/all/1/2/16 selected files, independent exact bytes and actual cleanup. The preservation assertion that selected-member count does not multiply archive-sized source work remains.
- `ArchiveCacheRetirementTests.AncestryAndMalformedRecovery`: actual persisted pending flags, obstructed nested retirement and restart with both existing and already removed roots. Existing expiry, edited/locked/live-owner, substituted-root/ancestor, malformed-record and fair-retirement tests remain.
- R45 paired measurement accepts either the original two-catalog ordinary chain or the one-catalog chain, while retaining exact payload/record/native bounds. It is a measurement probe; the strict current-work test above is the regression gate.
- Release C# build: zero warnings/errors. Four cache regressions, one native work regression, ten source/password/selection/cursor/preview/native-outcome regressions and four async lifecycle regressions pass: **19 selected/run/pass, zero skip/fail/not_run**. Performance probes are reported separately.
- Routine Rust archive verification: **359 passed, 1 existing ignored**, across 53 distinct completed unit/integration targets. Results are grouped by target, not summed across repeated executions; this is not a green unfiltered full-suite invocation. Four explicit-fixture targets below are deferred.
- `cargo clippy --release -p nxtools-archive --all-targets --features isolated-codecs --offline -- -D warnings` passes; scoped Rust formatting passes.
- `spec-contracts.py`, `gpui-release-contracts.py` and `markdown-editor-contracts.py` pass.

### Standards

One bounded review of the nine-file implementation/test/reference/spec delta against its frozen pre-change worktree: zero hard violations and zero actionable judgement findings.

### Spec

One bounded review of the same delta: zero missing requirements, scope expansion or reachable behavior defects within the two-optimization boundary.

## Evidence

`repo:.diagnostics/archive-gap/20261105-optimization/` retains both complete native reports, before/after run logs, the original work/cache regression evidence, native preservation/async logs, per-target routine Rust summary and raw execution logs, strict Clippy output and the reviewed delta.

Key records:

- `repo:.diagnostics/archive-gap/20261105-optimization/baseline-measurements.json`
- `repo:.diagnostics/archive-gap/20261105-optimization/candidate-measurements.json`
- `repo:.diagnostics/archive-gap/20261105-optimization/rust-routine-summary.json`
- `repo:.diagnostics/archive-gap/20261105-optimization/review.diff`

## Limitations and remaining acceptance

The measured reduction is scoped to the identified Release binaries, supplied ZIP and sequential three-trial comparison. No new Bandizip runtime comparison, universal speed ratio or decoder speedup is asserted. The current drag delivery still materializes verified private cache files and supplies independent IStreams; it does not directly decode into a receiver-granted final directory.

Real Directory Opus/Explorer gestures and the visible scatter/collect transition are not accepted by controlled COM tests. Wrapper-first placement is unchanged. The prior real destination trace remains separate evidence, not a new visual replay.

`rar_readmission_chain`, `sevenz_readmission_chain`, `tar_stream_readmission_chain` and `tar_stream_readmission_contract` require dedicated explicit fixture/host environments and were not executed in the routine verification group. The existing opt-in TAR performance test remains ignored. Full unfiltered suite and those additional fixture gates are not claimed green.

No package, installer, installed runtime, protected deployment, service or registry operation is part of this change. Overall Phase E/G-00 and physical/Release acceptance remain open.
