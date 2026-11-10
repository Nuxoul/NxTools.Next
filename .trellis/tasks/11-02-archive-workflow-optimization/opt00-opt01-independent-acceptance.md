# OPT-00/01 — independent acceptance

## Decision

**Overall: not accepted for closeout; retain `in_progress`.** Shared ZIP admission/revision and drag sealed-size accounting are accepted within the tested source/controlled-native scope. Task interaction and progress presentation need three focused repairs. G-00 remains incomplete independently of those repairs. OPT-02 through OPT-07 remain not started.

Scope: the uncommitted archive changes against HEAD `0940eec23286c1a750b3aff4284f32e76da68164`, the new archive optimization fixtures, and the retained measurement JSONL. Existing unrelated extension/keyboard/browser/download changes are not part of this verdict. The canonical parent phase and OPT package order are unchanged.

Requirements: [implementation](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/implement.md:13), [acceptance](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/acceptance.md:23). Delivery evidence: [results](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt00-opt01-results.md), [retained measurements](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt00-opt01-native-metrics.jsonl).

## Standards

**No confirmed architecture/data-protection violation in the reviewed shared-read/native-cache slice.**

- `NativeReadLease` is private, constructed after successful original native acquisition and borrowed by the operation. Public/unleased calls, volume sets and reclamation do not receive the per-member revision shortcut. Source revision checks still enclose the protected operation; original ordinal, structural, codec/CRC/MAC/size, cancellation and native publication checks remain.
- Browser ZIP listing and edit qualification reuse the same opened source/records. Optional edit-record budget exhaustion preserves catalog readability without permitting editing; an actual catalog limit still refuses.
- Seal's verified actual size travels through an internal completion object, not the public DTO. Removing the accounting cursor does not remove any real consumer cursor's object/content check or independent cursor lifetime.
- The requested-volume correction is present in the scoped diff. `ArchiveVolumeSources.Requested` points to the already held requested member for activity-reservation comparison; decoder `Primary` and complete family validation remain distinct. This is an in-scope baseline prerequisite, not new volume-format support. Its implementation was inspected; split native acceptance was not rerun in this review.

Source anchors: [lease construction](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker.rs:1025), [opened ZIP admission](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/engine.rs:232), [ZIP protected extraction](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/formats/zip.rs:700), [internal materialization completion](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Protocol/ArchiveEntrySession.cs:205), [real cursor verification](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs:236), [requested volume](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVolumeSources.cs:109), [reservation comparison](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs:784).

## Spec

### AC-01 — P1: closing/backgrounding can still strand an existing prompt

**Trigger:** a Password/Conflict or another current prompt is installed while the progress window exists, followed by the accepted title-bar close/background action. The same state is reachable when the UI has deferred a background action and the prompt arrives before that action runs.

`restore_prompt_surface` returns immediately while `record.window` exists. `background_job` subsequently takes/removes that window and retains only the expiring background notification; it never rechecks the already retained prompt. Once that notification expires, the current ticket has no durable actionable surface. Duplicate prompt delivery cannot repair this reliably because prompt IDs are deliberately deduplicated.

Evidence: [restore filter](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:152), [native window close callback](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:1895), [window removal and transient recovery notification](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:1912). The existing [background regression](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/job_tests.rs:4670) backgrounds the job before delivering the prompt and does not cover this opposite ordering. This violates FLOW-01/G-15's close/prompt and durable-decision requirements.

**Repair:** after the original background owner clears the window, recheck a retained current prompt and schedule the existing identity-checked restoration, or retain an equally durable original decision entry. Keep the same job/ticket/window owner and deferred cross-entity updates; do not add another notification/prompt manager.

**Acceptance:** enter through actual background/title-close callbacks; cover prompt-before-close and prompt-between-dispatch-and-defer, all five prompt kinds, expired toast, duplicate ticket, terminal/disconnect and replacement attempt. Exactly one current decision surface and one response remain possible. No implicit cancellation or revival of retired work.

### AC-02 — P2: late coalesced progress can replace a newer edit phase

The original C# session sends an edit-phase transition through lossless `Events`, while ordinary progress and same-phase updates can use coalesced `Progress`. The two streams are forwarded concurrently. A pending older `running` or earlier edit-phase update can therefore arrive after a newer edit phase.

Rust only rejects decreasing counters when `old.phase == next.phase`. It accepts an older different phase and overwrites `record.progress`, resetting phase-specific presentation/metrics. The production view can regress from verifying/committing to reading/running. This violates FLOW-09/G-25's truthful phase presentation even though every individual frame is schema-valid.

Evidence: [C# routing](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Protocol/ArchiveJobSession.cs:178), [concurrent channel forwarding](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Protocol/PlatformPipeClient.cs:218), [Rust application](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:1137). The [six-frame test](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/job_tests.rs:4592) consumes a fixed exported array, not the concurrent channel merge, and permits `running` after edit phases.

**Confirmed impact is phase/metric regression, not established illegal Pause admission.** Native checkpoints emit separate Verifying/Committing state before the corresponding edit phase ([runner](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs:846)); [pause_allowed](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:87) also checks that state. This protection must remain.

**Repair:** retain authoritative progress ordering within the original operation/attempt and reject late superseded phase observations at the original publication/application boundary. Preserve legitimate password-retry phase restarts; an unconditional global phase ranking must not hide a new valid internal attempt. No second transport/runtime is needed.

**Acceptance:** deterministically hold an earlier coalesced update, deliver a later lossless edit phase, then release the older update through production routing/`job_notification`. The displayed phase and metric epoch must not regress; native verification/commit controls remain disabled. Cover replacement attempts and legitimate retry restart separately.

### AC-03 — P2: TAR-family edits display 7z/LZMA2 semantics

The progress view selects the edit label only by `ArchiveJobKind::Modify`, then renders `准备 7z 编辑` and `重编码为非固实 LZMA2`. The same phase vocabulary is published for Tar/TarGz/TarXz, so their actual edit path displays a false format/algorithm.

Evidence: [format-blind labels](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/progress.rs:674), [C# supported formats](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Protocol/ArchiveJobSession.cs:183). The [C# export fixture](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost.Tests/ArchiveJobTerminalTests.cs:94) constructs a Tar request, but the [Rust test](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/job_tests.rs:4543) binds its frames to a SevenZ task and expects the 7z text. Passing that cross-language test does not validate format-specific presentation. This violates FLOW-09's honest stage feedback.

**Repair:** project the frozen edit format through the existing progress snapshot and choose accurate SevenZ versus TAR-family labels, or use truthful shared wording where the format does not matter. Preserve original official controls and callbacks.

**Acceptance:** pair each SevenZ/Tar/TarGz/TarXz request with its own actual serialized edit phases. TAR-family rows must not claim 7z/LZMA2; verification/publication/processed-byte distinctions remain explicit.

### AC-04 — G-00: baseline remains incomplete

The retained data supports the reported bounded ZIP improvement but not complete G-00:

- Revision bytes omit ZIP directory/local/payload reads, compound input reads and isolated-codec fingerprint work. Internal format timings and block/decode observations are still missing.
- ZIP/TAR/7z measurement work that can be added at existing read/decode owners has not been completed. Locked UnRAR's unavailable internal read/block events are a different constraint; do not classify all missing work as an API impossibility.
- The retained comparable ZIP dataset identifies only the 20,973,276-byte generated fixture. The implementation plan additionally requires the original read-only ZIP/member baseline; the supplied 137,737,166-byte sample is not present in these current comparison records. Historical measurements are not a substitute.
- The selected paired workers are frozen before/after builds. The final code received later budget/context changes; current regressions pass, but the historical speed numbers remain measurements of the recorded frozen pair, not a new final-current or Release timing run.

**Completion:** measure observable fields at their original owners, preserving spool/source distinction, and retain comparable before/after evidence rather than backfilling old numbers. Resolve an explicit supported measurement/acceptance decision for unavailable UnRAR internals without changing codecs, weakening output guarantees or inventing zero counts. Add the required original-sample comparison when its source is available under read-only permission; otherwise retain that row as pending. Other repairs can proceed while the measurement prerequisite is settled.

## Independent checks performed

All Rust calls used jobs1/incremental0/codegen-units256 in the same invocation and one Cargo lane. Scratch was used for temporary fixtures and exported frames. No product source was changed by this acceptance review.

| Check | Observed result |
| --- | --- |
| `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -m:1 -p:UseSharedCompilation=false` | PASS; 0 warnings/errors |
| Exact C# LIST then RUN: `archive OPT01 CSharp production editing progress wire` | selected1/run1/pass1/skip0/fail0; six frames exported by original OnFrame/serializer |
| `cargo test -p nxtools-archive --features isolated-codecs --test read_work_contract --offline -- --test-threads=1` | 4 passed, 0 failed/ignored |
| `cargo test -p nxtools-protocol --lib opt01_progress_phase_wire_vocabulary_and_malformed_frames --offline -- --test-threads=1` | 1 passed, 0 failed/ignored |
| `cargo test -p nxtools-desktop --offline opt01_ -- --test-threads=1`, with `NXTOOLS_ARCHIVE_EDIT_PROGRESS_WIRE` set to this review's C# export | 15 passed, 0 failed/ignored; unrelated integration target selected0 and is not counted as coverage |
| Exact C# LIST then identical RUN: shared-read revision work; sealed-size accounting; first/repeated cursor tamper; cache cursor integrity/lifetime | selected4/run4/pass4/skip0/fail0 using current built worker and owned temporary sources |
| Retained measurement audit | 356 JSONL records; 216 completed operations; 72 three-trial groups; all exported medians consistent; cleanup flags, nonzero transport and sample counts valid |

The native run observed normal ZIP browser open at one catalog/two revisions; read-only operations had zero edit qualifications; selected1/2/16 had six revisions each. First demand had Seal hash262144, real-cursor hash262144 and one native cursor open. Repeated/known/empty requests preserved independent positions and actual-size charging; same-size/same-time tampering was rejected. Helpers and owned outputs were removed through the fixture owners; the generated baseline source and ownership marker remain in the session scratch root.

Retained data independently recomputes browser1777.9156→913.1176ms, materialize3183.5465→1717.4910ms and selected1/2/16 at4490.5385/4979.9115/11841.5587→2476.8149/2524.7045/3473.8228ms. All seven ZIP read ranges are disjoint in that recorded batch. Creation ranges overlap, and the earlier drag ranges overlap; no broader creation/drag latency speedup is accepted.

## Gate disposition

| Gate | Independent disposition |
| --- | --- |
| G-00 | Incomplete, AC-04 |
| G-01/G-02/G-06 | Accepted for the tested shared-read/native-cache slice; not full I/O/performance or real Explorer acceptance |
| G-15 | Partial; ordinary background-after-close restoration passes, AC-01 remains |
| G-25 | Partial; vocabulary, current tests and Stop acknowledgement ownership pass, AC-02/AC-03 remain |
| G-19/G-24/G-26 | Touched source/controlled-fixture preservation only; no blanket gate completion |
| G-18 and Release | Not run; no physical Explorer/GUI/DPI/user/release acceptance |

## Follow-up boundary

Repair AC-01 through AC-03 in the original owners, add their missing real-boundary regressions and rerun the affected narrow tests. Complete or explicitly resolve AC-04's measurement requirements without starting OPT-02/04/05/06 work. The overall task stays `in_progress`; the statement that only G-00 remains is insufficient for closeout.

The source-confirmed AC-01/02/03 counterexamples were not exercised by new failing tests or physical UI in this read-only review. Existing passing tests and the independent successful runs above are retained as positive evidence, not proof that those untested branches are correct. Historical full-suite/Clippy claims were not rerun here. No package, install, reference execution, service/registry action, user-source mutation, recycling, HZ5 mutation, commit or push was performed.
