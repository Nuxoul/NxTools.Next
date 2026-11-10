# Release 45 — native 7z limit directed independent acceptance

## Decision

**Accepted in the automated and owned-native scope.** The native 7z low-budget error-category blocker is closed. The original complete 7z preservation selection now passes all ten internal cases, including the collision/corruption assertions previously hidden by its fail-fast stop. R45-AC01's three cleanup regressions remain green.

This accepts [the directed resource-limit repair](release45-sevenz-limit-repair.md) and closes the remaining native verification blocker in [AC01 independent acceptance](release45-ac01-independent-acceptance.md). Earlier UI/name acceptance and performance measurements retain their separate scope. Parent remains **Phase E — Format expansion and ZIP editing / in_progress**.

## Source and regression boundary

Compared with the 77-file AC01 archive snapshot, the only production-source change is [worker_output.rs](../../../rust/crates/nxtools-archive/src/worker_output.rs:665): explicit native `resource_limit` joins the existing ResourceLimit match arm. No budget, abort behavior, source/output ownership, capability or integrity policy changes.

[ArchiveVolumeSources::CheckEncoded](../../../csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVolumeSources.cs:193) returns that existing typed refusal when encoded source size exceeds the admitted bound. The former Rust fallback converted it to IoFailure. The new [actual-process regression](../../../rust/crates/nxtools-archive/tests/worker_extraction_contract.rs:514) feeds the original matching source-acquire reply and checks ResourceLimit, source_unavailable→IoFailure, one terminal, zero later native output and no fabricated readmission shortfall/risk event.

The [C# fixture](../../../csharp/NxTools.PlatformHost.Tests/ArchiveExtractionWorkerTests.cs:40) retains the original expectations and file/sentinel/payload/drain assertions. Only bounded completed-case observations and the final ten-case completeness assertion were added. The low-budget value remains `max_output_bytes=1`.

## Fresh independent verification

| Check | Result |
| --- | --- |
| Output unit boundary, including AC01's three cases | 10 pass |
| Actual-process binary transfer | 8 pass |
| Actual-worker extraction, including source-refusal regression | 11 pass |
| Original opt-in 7z readmission shortage | 1 pass |
| Exact C# native selection | LIST1 / RUN1 / PASS1 / SKIP0 / FAIL0 / not_run0 |
| Internal native cases | 10 complete; expected and actual results agree |
| C# Debug build | 0 warnings / 0 errors |
| Archive isolated-codecs/tests strict Clippy; scoped rustfmt | Pass |
| Spec contracts | Pass, 125 documents / 14 owners |

Thirty distinct Rust tests ran; no zero-selection or overlapping counts contribute. Fresh worker and C# host were built with the approved configuration. Every Cargo invocation used offline mode, one build job, incremental disabled and codegen-units256. Exact effects-reviewed native LIST preceded the identical RUN; only attributed repository fixtures and owned temporary outputs were used. The test helper slot was removed afterward.

Native cases 1–4 verify plain, solid, non-solid and encrypted output with the original independent byte/hash oracles. Cases 5–10 verify missing/wrong password, the one-byte output limit, existing-target collision, plain corruption and encrypted corruption. All retain actual cleanup and applicable original-file/sentinel/no-partial assertions. The final completeness record occurs after every payload oracle and all ten Verify calls.

[Verification summary](../../../.diagnostics/archive-root-drag/20261105-r45-sevenz-limit-independent/verification.json), [Rust](../../../.diagnostics/archive-root-drag/20261105-r45-sevenz-limit-independent/r45-sevenz-limit-independent-rust.log), [C# build](../../../.diagnostics/archive-root-drag/20261105-r45-sevenz-limit-independent/r45-sevenz-limit-independent-csharp-build.log), [native LIST](../../../.diagnostics/archive-root-drag/20261105-r45-sevenz-limit-independent/r45-sevenz-limit-independent-native-list.log), [native RUN](../../../.diagnostics/archive-root-drag/20261105-r45-sevenz-limit-independent/r45-sevenz-limit-independent-native-run.log), [lint/format/spec/readmission](../../../.diagnostics/archive-root-drag/20261105-r45-sevenz-limit-independent/r45-sevenz-limit-independent-gates.log).

## Handoff and limits

No remaining directed blocker was found. The user's conditional authorization now permits sending `帮我打包一下正式包` to session `ceebe351-d540-4ca3-a460-dadc71f3a7df`. `PACKAGING.md` was reread; the packaging session owns preflight, source freeze, actual Release/signing/installer generation and artifact verification. This acceptance itself did not package or install.

Physical DOpus visible staging, actual Bandizip branch/performance, user visual/DPI acceptance and installed-release/service gates remain separate. No new performance trials were run, and prior sequential/ABBA variability remains disclosed. The later direct incoming-Add/unsafe_path/notification request is not accepted by this directed repair. No parent Phase E, G-00/G-04/G-18 or overall Release closeout is implied. Product and checked-in test sources were not modified by the acceptance session.
