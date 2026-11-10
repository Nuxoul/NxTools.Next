# Reference improvement coordination

## Current state

- Implementation session: `0ffe6ffa-9e52-420b-af61-a29fdd0f37b4`.
- Coordinator: the conversation that dispatched this work; completion notifications return here through SessionTask.
- Current batch: **B3 — Split-file usability**.
- Delivery state: B3 is **accepted at source, production TestPlatform and owned-native fixture boundaries** after repair verification. Further implementation is paused, including B4–B7.
- Delivery message ID: `a2fad500-827e-40aa-a117-7186d0df09a7`.
- Delivery turn ID: `5c029aec-3329-40ee-ac97-7db7266f1317`.
- Delivery idempotency key: `reference-b3-split-file-usability-20261110-0ffe6ffa-from-ede3964f-01`.
- Implementation is performed only in `D:\Code\NxTools.Next-main`; preserve unrelated changes.
- This file is coordinator-owned. The implementation session owns its batch source/task artifacts and does not edit this coordination file.
- B2 verdict and retained evidence: [coordinator acceptance](../.trellis/tasks/11-10-reference-b2-intake-control/coordinator-acceptance.md). No open B2 product finding remains at the stated level; whole-workspace/native/installed limits remain separate.
- Latest user directive: stop after B3 acceptance/report. The subsequent explicit repair acceptance request does not authorize B4–B7 or further implementation; the pause remains in effect.

## Approved sequence

The [taskbook](reference-improvement-taskbook.md) remains scope authority. Delivery batches do not rename the archive parent's canonical phases.

| Batch | IDs | State |
| --- | --- | --- |
| B1 — Existing correctness | REF-01, REF-02, REF-03 | Accepted at source/owned-fixture level after corrections |
| B2 — Intake and control | REF-04, REF-05, REF-06, REF-12 | Accepted for source/owned fixtures and executed isolated browser; selection adopted, original cadence retained |
| B3 — Split-file usability | REF-17, REF-18 | Accepted at source/production TestPlatform/owned-native boundaries; four acceptance items closed; user-requested pause in effect |
| B4 — Task operation and visibility | REF-08, REF-09, REF-10 | Paused by user; scope retained, no automatic dispatch |
| B5 — Completion intentions | REF-07, REF-11 | Paused by user; scope retained, no automatic dispatch |
| B6 — Connection experiments | REF-13, REF-14, REF-15, REF-16 | Paused by user; scope retained, no automatic dispatch |
| B7 — Format expansion | REF-19, REF-20 | Paused by user; scope retained, no automatic dispatch |

## Dispatch and acceptance protocol

1. Send one bounded batch to the existing implementation session, retaining its model/project/context. Use SessionTask `send`, `kind=task`, a unique delivery idempotency key and `notifyOnCompletion=true`.
2. Rely on the host completion notification for that delivery. Do not poll the same callback-enabled delivery, enqueue its later batch early, or treat a worker's self-assessment as coordinator acceptance.
3. The implementation session completes source changes and relevant checks, records requirements/design/implementation/outcomes, retires owned build processes and returns a structured handoff. It stops at the batch boundary. A genuine blocker is reported without weakening tests or expanding scope.
4. Coordinator acceptance is scoped to the delivered batch or explicitly requested repair, with relevant independent checks and at most one optional directed read-only review. Acceptance does not authorize further implementation.
5. If a supported in-scope defect is confirmed, report it and keep B3 unaccepted. Do not send an automatic repair delivery; await explicit user resumption.
6. If B3 passes, record the completed delivery's acceptance through SessionTask `accept` and update the verdict. Regardless of outcome, report the result and pause; do not send B4.
7. B4–B7 remain approved taskbook scope but automatic continuation is suspended by the latest user directive. Resume only on an explicit user instruction.

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
