# OPT-03 — directed repair independent acceptance

## Decision

**Accept O3-AC01, O3-AC02 and O3-AC03 at the affected automated boundaries.** The three findings in the [original independent report](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt03-independent-acceptance.md) are closed by the reviewed repair and current independent runs. Combined with the retained G-21/G-22 and worker/owned-native evidence, **OPT-03 — Complete Add intake and results is accepted at its authorized automated/owned-native stop point**: G-12/G-13/G-21/G-22/G-23 and touched G-18/G-19 within that scope.

The original report remains the historical partial verdict; [repair evidence](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt03-ac01-ac03-repair.md) and this directed acceptance supplement it. No further in-scope repair blocker was confirmed. The optimization task remains `in_progress`, the parent remains **Phase E — Format expansion and ZIP editing**, and OPT-04 through OPT-07 are not started.

## Review boundary

HEAD remains `0940eec23286c1a750b3aff4284f32e76da68164`; reviewed changes are uncommitted. The directed review covered the five affected Desktop files: `jobs.rs`, `clipboard_requests.rs`, `intake_tests.rs`, `opt03_add_selection_tests.rs`, and `opt03_result_tests.rs`. It did not restart a broad review or alter product/test source. Previously accepted writers, output naming, protocol, queue and native publication mechanisms were not changed for this repair and were not remeasured.

## Findings closed

| Finding | Independently confirmed closure |
| --- | --- |
| O3-AC01 — original connection | [EditFormOrigin](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:134) retains browser origin and connection. [Form creation](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:2168) captures the actual connected generation; [final submission](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:427) and [intake](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/clipboard_requests.rs:23) require that same generation, preserving all source/snapshot/folder checks. |
| O3-AC02 — valid terminal path | [No-refresh cases](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/opt03_add_selection_tests.rs:100) use valid facts without contradictory Add display results. [Validity and installation assertions](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/opt03_add_selection_tests.rs:171) prove the matching terminal reaches the original business owner before checking zero refresh/unchanged selection. Malformed rejection is a distinct test, not business evidence. |
| O3-AC03 — Test cleanup failure | [post_test_case](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/opt03_result_tests.rs:12) consumes real C# Create exports, observes Test Start and validates the Test terminal. [Cleanup-false assertions](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/opt03_result_tests.rs:138) preserve Create facts/Test outcome, hold the original running reservation and refuse actual Locate/Retry calls and reclamation continuation. |

The connection regression enters through the rendered incoming-drop Add action and original form controls. [The actual router replacement](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/intake_tests.rs:455) changes connection generation while its old Disconnected notification remains queued. The test invokes submit, Paste and both pickers without consuming that notification, verifies no job/input command and retained source/browser/draft, and requires same-connection controls to dispatch once. This directly covers the original counterexample rather than relying on a synthetic disconnect invalidating the browser.

The six valid no-refresh shapes cover allSkip, failed-before-commit, unknown publication, CommitOutcomeUnknown, published-but-unclean and precommit cancellation. Matching job terminal equality is asserted after original IPC/application notification handling. Old attempt/generation isolation, malformed frame refusal and published late-cancel refresh remain separate cases.

Test-side cleanup failure covers ordinary ZIP, split ZIP `.zip` and split7z `.001`, each with success/failure/cancel result shapes: nine combinations. All enter post-Test after successful clean creation; the Test terminal itself has cleanup=false. Assertions retain committed/verified/skipped counts, journal, commit state, output publication and native location, while keeping Test outcome/code and final cleanup=false. No new ArchiveJob, ArchiveSpace or OpenShellTarget command is allowed after Locate/Retry calls, and the running slot stays occupied. This closes an automated lifecycle gap, not a newly exercised native cleanup-failure mechanism.

## Independent runs

All Rust commands inherited `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256` in the same invocation and used one Cargo lane. Native execution was limited to the existing self-owned Create-location export fixture, exact LIST then identical RUN, with TEMP/TMP in this session scratch.

| Check | Observed result |
| --- | --- |
| C# test-host build: `--no-restore -m:1 -p:UseSharedCompilation=false` | PASS;0 warnings/errors |
| `OPT03 create actual native result locations` | LIST1/RUN1/PASS1/SKIP0; fresh ordinary/split ZIP/split7z terminals exported |
| Desktop `intake_tests` | 8/8 |
| Desktop `opt03_add_selection_tests` | 6/6 |
| Desktop `opt03_result_tests` | 4/4, including nine Test-side cleanup-false combinations |
| Desktop `archive::application::jobs::tests` | 111/111 |
| Desktop `step4_` | 11/11 |
| Desktop all-target strict Clippy, offline, `-D warnings` | PASS |

Test groups overlap and are not summed. Unrelated integration targets with zero selected tests are not counted. The post-Test group used this run's freshly generated `ordinary.zip.json`, `split.zip.json` and `split.7z.json` through `NXTOOLS_OPT03_CREATE_WIRE`; missing files do not synthesize facts. Existing actual C# progress exports were supplied to shared Job regressions. No writer, full Create/Add native matrix, performance baseline or full-package test suite was rerun.

The frozen worker still matches `E3DFF478BA9CBED906314E345685EECDAA4C1A4852F746D7019061D62F5487AE`, the previously accepted OPT-03 final worker. Its unchanged source/publishing behavior is not claimed as a new optimization result. Current independent logs are `opt03-repair-acceptance-*` under session scratch `D:\.pi-desktop\data\scratch\aab2f188-352d-4f1c-b7a3-5375fe0df2d8`.

## Status and limits

O3-AC01 was a production connection-isolation defect; O3-AC02/03 were test evidence gaps. This acceptance closes those specific findings without rewriting them as three production publication failures. Existing successful full OPT-03 independent runs remain attributed to [the original report](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt03-independent-acceptance.md); the original delivery and repair records remain intact.

G-00, OPT-00/01 overall closeout, complete G-04, Phase E and Release acceptance remain open. Real clipboard/picker/Explorer/product GUI, IME/DPI/accessibility and user visual acceptance remain unexecuted and separately authorized. The overall task is not complete. No package/sign/install, user archive mutation, source recycling/HZ5, service/registry action, commit or push was performed. Stop at OPT-03; proceeding to OPT-04 requires a new implementation instruction.
