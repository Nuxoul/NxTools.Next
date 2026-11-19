# Reference improvement coordination

## Current state

- Implementation session: `0ffe6ffa-9e52-420b-af61-a29fdd0f37b4`.
- Coordinator: the conversation that dispatched this work; completion notifications return here through SessionTask.
- Current slice: **B7 — Format expansion**, REF-19/REF-20.
- Delivery state: **accepted for the declared capability matrices and recorded source/worker/native/TestPlatform levels**, with audited own-recipient receipts; no confirmed in-scope finding.
- Current handoff: user-supplied B7 delivery; [coordinator acceptance](../.trellis/tasks/11-12-reference-b7-format-expansion/coordinator-acceptance.md). UIA/native-memory and formal packaging remain separately owned; archive parent Phase E stays open.
- Previous accepted B3 delivery message: `a2fad500-827e-40aa-a117-7186d0df09a7`, turn `5c029aec-3329-40ee-ac97-7db7266f1317`.
- Previous B3 delivery key: `reference-b3-split-file-usability-20261110-0ffe6ffa-from-ede3964f-01`.
- Implementation is performed only in `D:\Code\NxTools.Next-main`; preserve unrelated changes.
- This file is coordinator-owned. The implementation session owns its batch source/task artifacts and does not edit this coordination file.
- B2 verdict and retained evidence: [coordinator acceptance](../.trellis/tasks/11-10-reference-b2-intake-control/coordinator-acceptance.md). No open B2 product finding remains at the stated level; whole-workspace/native/installed limits remain separate.
- Latest user request: independently accept B7. Acceptance is complete; no follow-up implementation, UIA/package work, commit/push or automatic dispatch follows.

## Approved sequence

The [taskbook](reference-improvement-taskbook.md) remains scope authority. Delivery batches do not rename the archive parent's canonical phases.

| Batch | IDs | State |
| --- | --- | --- |
| B1 — Existing correctness | REF-01, REF-02, REF-03 | Accepted at source/owned-fixture level after corrections |
| B2 — Intake and control | REF-04, REF-05, REF-06, REF-12 | Accepted for source/owned fixtures and executed isolated browser; selection adopted, original cadence retained |
| B3 — Split-file usability | REF-17, REF-18 | Accepted at source/production TestPlatform/owned-native boundaries; four acceptance items closed; user-requested pause in effect |
| B4 — Task operation and visibility | REF-08, REF-09, REF-10 plus Header Reset | Accepted at source/TestPlatform/owned HTTP-SQLite-writer-pipe boundaries; four acceptance items closed |
| B5 — Completion intentions | REF-07, REF-11 | Accepted at source/TestPlatform/owned HTTP-SQLite-writer-pipe boundaries; three findings closed |
| B6 — Connection experiments | REF-13, REF-14, REF-15, REF-16 | Accepted at source/owned HTTP-TLS-SQLite-writer-pipe/TestPlatform boundaries; B6-01 closed; REF-13 adopt, REF-14/15/16 retain-current |
| B7 — Format expansion | REF-19, REF-20 | Accepted for declared ALZ/EGG rows and Windows x64 clear Store/Deflate ZIP-SFX profile; source/worker/native/TestPlatform plus audited recipient receipts; parent Phase E remains open |

## Dispatch and acceptance protocol

1. Send one bounded batch to the existing implementation session, retaining its model/project/context. Use SessionTask `send`, `kind=task`, a unique delivery idempotency key and `notifyOnCompletion=true`.
2. Rely on the host completion notification for that delivery. Do not poll the same callback-enabled delivery, enqueue its later batch early, or treat a worker's self-assessment as coordinator acceptance.
3. The implementation session completes source changes and relevant checks, records requirements/design/implementation/outcomes, retires owned build processes and returns a structured handoff. It stops at the batch boundary. A genuine blocker is reported without weakening tests or expanding scope.
4. Coordinator acceptance is scoped to the delivered batch or explicitly requested repair, with relevant independent checks and at most one optional directed read-only review. Acceptance does not authorize further implementation.
5. If a supported in-scope defect is confirmed, report it and keep the current batch unaccepted. Do not send an automatic repair delivery; await explicit user resumption.
6. Record the current verdict and exact evidence. SessionTask acceptance applies only to a matching completed delivery receipt; a pasted handoff is recorded in its task artifacts. Regardless of outcome, report and pause.
7. B5–B7 remain approved taskbook scope but automatic continuation is suspended. Resume implementation only on an explicit user instruction.

## Shared build and write ownership

- One implementation session; no recursive implementation delegation or competing source writers.
- The implementation session holds the Cargo lane during its batch. The coordinator does not run Cargo until the delivery settles and existing Cargo/rustc processes have actually exited.
- While the coordinator verifies, the implementation session does not start another build or batch. A repair/next-batch dispatch transfers the lane back after current coordinator verification settles.
- Every Rust invocation sets `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, and TEMP/TMP to that session's scratch in the same execution. Use full-operation timeouts and resumable background waits; never kill unrelated processes or start duplicate builds.
- Capture a scoped dirty baseline before each batch. Never attribute the entire working-tree diff to one delivery or broadly format/reset unrelated work.
- Discovery/connection performance comparisons run without concurrent compilation or competing measurements, with equivalent complete outputs and the taskbook's baseline/trial rules.

## Retained acceptance boundary

B1's three previous findings are accepted after independent verification: originating Cookie-store survival, manual confirmation-pending semantics and exact-once explicit Copy diagnostics. Current source verification included extension170/170, BrowserHost47/47, REF-01 GPUI4/4 and adjacent polish11/11; counts overlap. The original two independent witnesses now assert correct behavior. Supplementary popup DOM84checks passed; media DOM remains unperformed because its browser runtime space disappeared between calls. Do not recreate test spaces to bypass lost ownership; do not claim native/physical/visual acceptance from these fixtures.

[Current B1 evidence and limits](../.trellis/tasks/11-09-reference-b1-existing-correctness/outcomes.md) retain the message-version, Desktop strict-lint and Clipboard source-gate baselines. Reassess a blocker if it affects the new batch, but do not weaken assertions or repair unrelated areas merely to obtain a whole-workspace green result.

## Authorization limits

The approved workflow covers implementation, local deterministic fixtures, relevant compilation and coordinator acceptance for the taskbook. It does not authorize packaging/installing or replacing the deployed product, launching reference programs, extension registration/installation, service/registry mutation, real power actions, commit or push. Source/fixture, isolated DOM, native/physical/DPI and packaged/installed acceptance are reported separately. Windows/native operations remain in the original C# adaptation boundaries.

## Required handoff

Return the batch/repair ID, task directory, exact source boundary and changed files, real RED/GREEN assertion evidence, executed commands/counts/log paths, diagnostic/privacy observations, relevant regressions and remaining acceptance limits. Identify whether the batch is ready for coordinator acceptance or genuinely blocked. Confirm the Cargo lane and owned test resources are released. Update task metadata to implementation-complete/awaiting-acceptance without self-approving the batch or advancing to the next one.

## Authorization handoff

The first delivery `8b23224a-f75d-4c73-868a-7f059f289c7e` settled without implementation after the target applied its earlier B1-only instruction. The target subsequently reported that its user corrected that interpretation and that the blocker was withdrawn, in message `6c6574ba-f736-4fa0-ace9-dcee215d3cff`. Existing authorization limits remain unchanged.

The B2 authorization handoff is resolved and B2 is accepted at its recorded level. The user-directed pause remains in effect after B3 repair acceptance; earlier standing dispatch permission does not override that pause. Settled delivery keys are not reused and native/release permissions remain unchanged.

## B2 outcome

The B2 task is `.trellis/tasks/11-10-reference-b2-intake-control`. Coordinator verification passed Node255, Host48, Desktop REF-06 eleven/REF-04 six/intake eleven/B1 four/polish eleven, Popup DOM22 and78browser/conformance assertions. Full raw trials and current logs are retained under that task's `evidence/` directory.

Six serial AB/BA/AB trials retained all required complete/manifest/Blob/hook/CSS sources. Candidate presentation fell91to10 while raw observations remained91. Active task-time medians were1995.342/1959.447ms, idle262.074/258.909ms and median discovery latency1813.100/1817.950ms for baseline/candidate. Accept source-based candidate denoising; retain7s/1s/1s/5s timer responsibilities. These data do not establish stable CPU or overall-latency speedup.

The successful coordinator-owned B2 TaskSpace completed in one script and was finished. User pages were not claimed. The B2-only permission for iterative isolated spaces does not expand future native, profile, install or release permissions.

## Acceptance closure

- Exact applied-rate bytes survive untouched global Settings saves and reopen; explicit edits retain original KiB/draft semantics.
- Current authoritative task cap drives compact-refresh summaries and clean editors, including external typed changes; dirty/error drafts remain protected.
- Named bounded browser stages completed, all six measured trials and raw metric endpoints are retained, and no missing metric or unperformed native acceptance is labelled passed.

## B3 outcome

[B3 coordinator verdict and evidence](../.trellis/tasks/11-10-reference-b3-split-file-usability/coordinator-acceptance.md) record scoped acceptance. B3-01/B3-02 and E1/E2 are closed: real invalid-size reasons, first Enter/Space Confirm, actual preset/Step controls and immutable form-to-Job/wire bytes. Independent repair verification passed REF17 8, REF18 11, forms48, draft12, Job129, quick3, tools11, Core2, Protocol1 and IPC62; scoped format and four documentation/API/provenance gates passed. Counts overlap.

Original coordinator native split3/raw4 remain applicable to the unchanged native/writer boundaries; this repair round did not rerun them. Strict Desktop Clippy was independently rerun and retains only the two unchanged download baselines. Physical/IME/DPI/provider, human visual, installed/Release and whole-workspace limits remain separate. Current verification process scan is0.

SessionTask accepted original B3 delivery `a2fad500-827e-40aa-a117-7186d0df09a7` at `2026-11-10T07:35:24.203Z`, incorporating the user-supplied repair handoff. No further implementation task was dispatched. B4–B7 remain documented scope only and paused until an explicit user instruction. Archive stays **Phase E — Format expansion and ZIP editing / in_progress**.

## B4 outcome

[B4 coordinator verdict](../.trellis/tasks/11-10-reference-b4-task-operation-visibility/coordinator-acceptance.md) records acceptance after30independent LIST/RUN selections and70reacceptance logs. Shared validator selection, bounded task revision, real draining-pipe cancellation and held-probe native-close/queued-result isolation close B4-01/B4-02/B4-E1/B4-E2. Production UI30, replacement11, messages16, Download-IPC8, Download127/HTTP-client26/Core98 and original Header/template/coverage/rate/Server/window/caller checks pass. Counts overlap.

Affected compile/format/Core-Download-DownloadIPC lint and four source gates pass. Desktop strict lint retains only two unchanged baselines. The verification wrapper exited0and process scans are0; fixture candidate/socket/lease/listener/client drain is asserted. Physical Windows OS-close/IME/DPI, human visual, installed/Release and whole-workspace limits remain separate.

B4 is accepted at the stated scope. The user's conditional request now calls for a B5 — Completion intentions prompt, REF-07/REF-11, not automatic dispatch. No product source repair, next-batch execution, package/install, system mutation, commit or push followed reacceptance. B6/B7 remain unstarted.

## B5 outcome

[B5 coordinator verdict](../.trellis/tasks/11-11-reference-b5-completion-intentions/coordinator-acceptance.md) records acceptance after33independent repair/preservation LIST/RUN selections. B5 UI26, completion9, engine105, B4 UI30 and Archive129pass;81newlogs retain exact commands/results. B5-01/B5-02/B5-03 are closed. Previous independent safe C#2evidence applies to unchanged adapters, not rerun in this repair. Counts overlap.

Scoped format, eight-package check, six package strict lint and five source gates pass. Protocol/Desktop retain only three historical diagnostics. Owned verification processes are0and the wrapper exited; no real power/current-user Desktop exit was performed. Physical/native/visual/installed/Release layers remain separate.

The next prompt includes the user-added [PlatformHost UIA memory root repair](reference-code/platformhost-uia-memory-repair.md) before B6 connection experiments. It requires native COM/target-demand lifecycle and actual controlled memory evidence, not startup WarmUp removal alone. The source allocation diagnosis is reported evidence, not a new measurement by B5 acceptance. No next-batch implementation, package/install, system mutation, commit or push followed reacceptance; B7 remains unstarted.

## A source acceptance

[UIA coordinator report](../.trellis/tasks/11-11-platformhost-uia-memory/coordinator-acceptance.md) records matching C#Debug build0warnings/errors, reviewed synthetic Fixture LIST9/RUN9/PASS9/SKIP0, spec and scoped whitespace PASS. Those existing seams do not execute the new native COM client or prove the activity-dependent memory improvement.

Native ABI/lifetime, ordinary/helper no-target module behavior, active/split/WPF provider behavior and memory effectiveness remain unverified by this coordinator. No dynamically reproduced remaining leak or fixed-memory result is asserted. A remains not_accepted at those layers; formal packaging/physical testing belong elsewhere. The current B6 verdict is recorded separately below. No A product repair, global UIA/product/UAC, system mutation, commit or push occurred.

## B6 outcome

[B6 coordinator verdict](../.trellis/tasks/11-11-reference-b6-connection-experiments/coordinator-acceptance.md) is **accepted** with B6-01 closed. The original worker observes every failure once before retry eligibility. Identical-test/fixture RED→GREEN and independent repaired execution prove `503 → refusal → terminal refusal` learns width1while preserving native10061, exactly three attempts, continued peer reception, exact4MiBpeer range and actual joins.

Independent37nonzero LIST/RUN groups pass842executions, FAIL/SKIP/not_run0, including all10Desktop groups. Nine-package all-target check, Download/Network/DownloadIPC strict lint, eleven-file format and five source/lifecycle gates pass. The original138formal observations/132authoritative selections/92warmups and statistics remain frozen. Independent pre-recording audit matches2177relevant protected files; seven repair source snapshots stay unchanged during execution. Owned resources are0; wrapper exit0. The independent archive retains99materials.

REF-13 adoption is accepted for fewer refused connections, with primary median+4.7208%inside the frozen5%allowance; it is not a throughput improvement. REF-14/15/16 retain-current decisions are accepted. Historical Protocol/Desktop strict-lint diagnostics remain separately attributed. B1–B6 stay accepted at their boundaries; B7 stays unstarted. UIA/native memory, physical GUI and installed/Release acceptance are separate. The coordinator changed no product source and performed no automatic dispatch, package/install/system mutation, commit or push.

## B7 outcome

[B7 coordinator acceptance](../.trellis/tasks/11-12-reference-b7-format-expansion/coordinator-acceptance.md) separately accepts REF-19's declared reader matrix and REF-20's declared ZIP-SFX profile. Independent29groups/376executions pass with FAIL/SKIP/not_run0: Worker74, recipient22, layout15, intent2, Rust preservation138, Desktop73and native fixtures52. Matching Debug builds, ten-package check, Archive/Core/IPC strict lint, Archive default lib lint,48-file format and five source/lifecycle gates pass.

Fresh actual Create outputs independently verify4profiles/8physical files/20entries/3,623,188decoded bytes. Static PE/source/notices match the reviewed own recipient. Thirty retained recipient outcomes and four loader observations were audited, not reexecuted. The4250-check audit passes;3097original B1–B6records remain byte-identical,68tested source/asset snapshots stay unchanged and owned processes are0.

No confirmed in-scope product finding remains. Unqualified method/encryption/name/solid/split combinations, physical HWND/IME/DPI/user visual, native-memory and installed/Release acceptance remain separate; historical Protocol/Desktop lint is not reported green. Parent **Phase E — Format expansion and ZIP editing** remains open. The coordinator changed no product source and performed no package/install/system mutation, commit/push or follow-up dispatch.
