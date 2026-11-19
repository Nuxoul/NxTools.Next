# PlatformHost UIA memory repair and B6 connection experiments

## Goal

Repair the reported OneCommander UIA native-memory lifecycle in ordinary PlatformHost and the elevated-directory helper, then evaluate B6 — Connection experiments independently for REF-13/14/15/16 without memory pressure contaminating timing. Deliver the two parts separately and stop at ready_for_acceptance / not_accepted for independent acceptance; B7 remains unstarted.

## Background

B1–B5 are accepted at their recorded source/production TestPlatform/owned fixture boundaries. B5 acceptance is recorded in ../11-11-reference-b5-completion-intentions/coordinator-acceptance.md. These accepted dirty changes are the baseline, not this batch's delta. Canonical Archive remains Phase E — Format expansion and ZIP editing / in_progress.

The reported incident had two PlatformHost instances near20GiB private memory with a small managed heap and OOM stacks through managed UIA ListView proxies/COM marshalling. Both startup roles reached WarmUp even without OneCommander. No native allocation stacks establish a complete CCW allocation breakdown; separate reported facts from inferred attribution and new controlled measurements.

## Deliverables and order

1. [A — Native UIA memory lifecycle](../11-11-platformhost-uia-memory/prd.md): native COM migration, target-demand ownership and actual retirement plus sustained controlled native-memory evidence. A is a required repair, not an optional algorithm experiment.
2. [B6 — Connection experiments](../11-11-reference-b6-connection-experiments/prd.md): REF-13 learned ceiling, REF-14 eligible first-GET reuse, REF-15 slow retirement and REF-16 multi-address selection, each with a measured adopt/retain-current decision.

B6 timing begins only after A passes its defined controlled lifecycle and memory criteria. Reference review and harness planning may precede that boundary; no B6 timed baseline or candidate run may do so.

## Shared requirements

- Read AGENTS, applicable specs and indexed real references before product changes. Download reference order is ABDM then legacy compatibility and Mister; OneCommander follows system-shell/SwiftList behavior.
- Rust owns download state/layout, C# Windows APIs, GPUI rendering. Reuse original task/queue/store/event pump/coordinator, native apartments/admission and route/connector owners. No replacement manager/scheduler/transport/settings store or dependency/vendor upgrade.
- Preserve B1–B5, durable-before-publication, checkpoint/artifact identity, stop/join/sync order, B5 action authority/normal cleanup, B4 Header/reset/validator/revision/template/coverage, rate limits, Server and focus/IME/geometry.
- Carry correlation and low-noise fixed-state/count/elapsed/native-code diagnostics. Never log paths, URLs, focus text, headers/cookies, credentials or per-event/per-byte streams.
- Preserve unrelated dirty changes. Coordinator acceptance reports/evidence and docs/reference-improvement-coordination.md are read-only. Maintain before-relative path attribution and exact protected bytes.
- Test first at actual public/native lifecycle boundaries; nonzero LIST followed by the identical RUN, with precise pass/fail/skip/not_run and exits. Distinguish behavioral RED, build/fixture prerequisites and first-pass supplementary coverage.
- One Cargo lane; inspect commit headroom and surviving builds before Rust execution. Every execution sets jobs1/incremental0/codegen-units256 and scratch TEMP/TMP. No compilation or competing benchmark during measurement.

## Acceptance

- A satisfies its native lifecycle and sustained no-target/active-target memory criteria without restart, forced GC, working-set trimming or a cosmetic memory cap.
- Each REF has separate raw serial interleaved baseline/candidate trials (at least three pairs), median/slowest/resource/request costs and an explicit measured decision. Equivalent final bytes are checked outside timing.
- A results, four REF results, precise changed paths/owners, commands/counts/raw logs, actual resource exits and remaining acceptance levels are recorded separately.
- Affected C#/Rust regressions, necessary B1–B5 preservation, strict lint/check, scoped formatting and source/lifecycle gates are executed. Historical unrelated lint stays separately attributed, not suppressed or repaired incidentally.
- Final handoff remains ready_for_acceptance / not_accepted pending independent coordinator review; no B7.

## Authorized effects and exclusions

Owned synthetic Win32 ListView/dialog windows and explicitly reviewed owned fixture subprocesses may be created and cleaned up. Do not operate user windows or launch real product/reference GUI, UAC or an elevated process. Physical OneCommander/elevation, OS/visual and installed/Release acceptance remain separate. No real power/current Desktop exit, packaging/install, service/registry changes, commit or push.
