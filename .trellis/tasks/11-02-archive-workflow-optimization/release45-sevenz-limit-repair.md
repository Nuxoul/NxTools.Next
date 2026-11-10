# Release 45 — native 7z resource-limit preservation

Parent: **Phase E — Format expansion and ZIP editing / in_progress**. Scope: the remaining native7z preservation failure in [directed independent acceptance](release45-ac01-independent-acceptance.md). R45-AC01's accepted abort behavior and the earlier UI/name repairs remain unchanged.

## Change boundary

The original non-readmission fixture requests extraction with `max_output_bytes=1`. Native `ArchiveWorkerRunner` uses that finite bound for encoded-source admission; `ArchiveVolumeSources` holds the source then refuses its larger encoded size with `resource_limit`. Runner emits the same typed failure in the matching `source_acquire` native reply. `PrivateOutput::call_payload` must preserve it as `ArchiveError::ResourceLimit`, not its fallback IoFailure.

Production scope is this typed error mapping in `worker_output.rs`. The actual-process regression in `worker_extraction_contract.rs` rejects original source acquisition and asserts terminal category plus zero later effects/shortfall; a source-unavailable control keeps genuine I/O refusal distinct. The original C#7z fixture retains all assertions and reports each completed case, including its low-budget, collision and damaged-content cleanup cases. No source/actual-output budget, readmission eligibility, accepted capability, cancellation, CRC/size/native publication, abort, protocol schema or resource owner changes.

## Result — 2026-11-05

The remaining native7z preservation blocker is repaired and the complete original fixture passes. The production delta adds `resource_limit` to the existing ResourceLimit match arm in `worker_output.rs`; no other native or worker production path changes. `source_unavailable` still maps to IoFailure. The non-readmission source limit remains1 in the failing scenario; the repair neither enables output nor synthesizes a readmission shortfall.

### Original-boundary RED/GREEN

`worker_extraction_contract::sevenz_native_source_admission_preserves_resource_limit_before_output` runs the actual worker and delivers the original matching native `source_acquire` refusal. Before repair it fails with IoFailure instead of ResourceLimit. After repair it passes both resource_limit and source_unavailable control cases, checks source/binary accepted capabilities, one terminal, only source_acquire in native actions, and no decode/output/shortfall/risk event. The original C# fixture independently fails at case7 before repair and passes unchanged expectations after repair.

### Executed checks

| Boundary | Result |
| --- | --- |
| Output unit boundary, including R45-AC01's3 cases |10/10 pass |
| Actual-process binary transfer |8/8 pass |
| Actual-worker extraction, including new native-admission regression |11/11 pass |
| Original opt-in7z readmission shortage boundary |1/1 pass |
| Native7z handle-session fixture |LIST1/RUN1/PASS1/SKIP0/FAIL0/not_run0 |
| Native subcases |10/10 completed with original assertions |
| Final isolated-codecs Debug worker / C# Debug host |Build pass; C#0 warnings/errors |
| Archive isolated-codecs/tests strict Clippy |Pass, `-D warnings` |
| Scoped rustfmt/diff whitespace; spec/GPUI/editor gates |Pass |

Thirty distinct Rust cases pass. R45-AC01's extra-write, omitted-abort and sticky-first-failure regressions remain green without changing its implementation.

### Complete native7z cases

The exact selection is `archive extraction actual Rust 7z handle-session fixture`. Its effects remain attributed read-only repository inputs, fresh owned output/sentinels and damaged copies, one matching helper copied only into the test apphost slot, original Job/stdio/native handles and actual cleanup. Exact LIST precedes identical RUN. The slot is removed in the fixture's original finally block; no installation is touched.

| Case | Expected / observed | Preserved assertions |
| --- | --- | --- |
|1 lzma.7z|success|Independent15-byte literal and cleanup|
|2 solid.7z|success|Independent two-file hashes/count and no partial|
|3 non-solid.7z|success|Independent two-file hashes/count and no partial|
|4 encrypted.7z|success|Independent exact names/sizes/hashes|
|5 missing password|PasswordRequired|Only original sentinel remains; cleanup|
|6 wrong password|AuthenticationOrDataError|Only original sentinel remains; cleanup|
|7 lzma.7z max_output_bytes=1|ResourceLimit|Only original sentinel remains; no leaked output; cleanup|
|8 existing target|OutputCollision|Existing file and unrelated sentinel unchanged; no partial; cleanup|
|9 damaged.7z|CorruptArchive|No invalid output/partial; sentinel unchanged; cleanup|
|10 damaged-encrypted.7z|AuthenticationOrDataError|No invalid output/partial; sentinel unchanged; cleanup|

Every Verify call also checks private native calls do not escape the original output pump and requires `DrainAsync` success. `ARCHIVE_7Z_CASE` is emitted only after those terminal/output/sentinel/drain assertions; `ARCHIVE_7Z_PRESERVATION cases=10 ...` is emitted after all payload oracles and all ten cases complete. No password or member payload is logged.

## Evidence

- [Worker RED](../../../.diagnostics/archive-root-drag/20261105-r45-sevenz-limit/r45-sevenz-limit-worker-red.log)
- [Native RED, including build and exact LIST](../../../.diagnostics/archive-root-drag/20261105-r45-sevenz-limit/r45-sevenz-limit-native-red.log)
- [Worker GREEN and final build](../../../.diagnostics/archive-root-drag/20261105-r45-sevenz-limit/r45-sevenz-limit-worker-green.log)
- [Native GREEN, exact LIST/RUN and all ten cases](../../../.diagnostics/archive-root-drag/20261105-r45-sevenz-limit/r45-sevenz-limit-native-green.log)
- [Lint/format/spec/dependency gates](../../../.diagnostics/archive-root-drag/20261105-r45-sevenz-limit/r45-sevenz-limit-gates.log)

Cargo used offline mode, one build job, incremental disabled and codegen-units256 in each invocation. C# execution used the real test apphost and explicit final worker; temporary outputs stayed under session scratch. Original failed evidence and independent decisions are unchanged.

## Acceptance boundary

No remaining failure was observed in this directed scope. This is delivery verification for independent reacceptance, not a rewrite of the prior independent verdict. Physical DOpus/GUI/Bandizip, new reverse engineering, Release packaging/install, service/registry and commit/push were not performed. No performance trials were added; prior sequential/ABBA observations keep their original limits. Phase E remains in_progress.

## Directed independent acceptance

[Independent verdict](release45-sevenz-limit-independent-acceptance.md): accepted in the automated/owned-native scope. Fresh30 Rust cases and the exact native7z selection pass; all ten internal cases complete with original expectations and preservation assertions. The low-budget category blocker is closed. Historical failures, prior performance variability and physical/installed acceptance boundaries remain unchanged.
