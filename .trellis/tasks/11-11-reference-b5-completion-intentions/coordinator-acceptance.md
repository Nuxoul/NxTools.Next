# B5 coordinator acceptance

## Verdict

**Accepted at source, production TestPlatform and owned HTTP/SQLite/writer/pipe fixture boundaries.**

Acceptance date:2026-11-11. B5-01, B5-02 and B5-03 are closed after the [scoped repair](acceptance-repair.md) and independent reacceptance below. No open B5 finding remains in the reviewed repair. B1–B4 retain their acceptance; canonical Archive remains **Phase E — Format expansion and ZIP editing / in_progress**.

The user-added [PlatformHost UIA memory repair](../../../docs/reference-code/platformhost-uia-memory-repair.md) is a separate prerequisite in the next B6 task, not a repaired B5 feature. No next-batch implementation was dispatched.

## Closed findings

| Item | Verified boundary |
| --- | --- |
| B5-01 — Coalesced membership | Original engine scheduler ownership keeps one destructive membership cutoff per queue and each task's accepted revision. The [snapshot](../../../rust/crates/nxtools-download/src/engine.rs:741) carries both under original locking. [Coordinator evaluation](../../../rust/apps/nxtools-desktop/src/completion_actions.rs:274) retains invalidation even if intermediate membership disappeared. Initial nonterminal membership/cutoff seeds its baseline; same-queue first attempt0→1 may advance only while cutoff is unchanged. Accept→remove and out→back real-writer cases hold the original pump, then prove no action and a later genuine successful round. |
| B5-02 — Post-terminal arming | The original [domain command](../../../rust/crates/nxtools-download/src/engine.rs:1151) rejects Completed/Failed/Cancelled before SQLite, memory or events. [Production-pump regression](../../../rust/apps/nxtools-desktop/src/downloads/settings_task_rem_tests.rs:4748) completes the real writer before public command/observation, refuses retroactive authority and preserves legitimately armed future success. |
| B5-03 — Stopped retry | Newly admitted retry [publishes after durability](../../../rust/crates/nxtools-download/src/engine.rs:2607), while duplicate admission does not restamp attempt/revision. Original completion snapshot projects stored eligible Failed retry as Waiting without changing raw scheduling/stopped state. [Original pump regression](../../../rust/apps/nxtools-desktop/src/downloads/settings_task_rem_tests.rs:4849) proves prompt countdown cancellation, no stopped-queue HTTP worker, duplicate idempotency, existing queue-start real success and once-only controlled action/stale callback isolation. |

The retained [four-case original RED](repair-evidence/b5-repair-prerepair-regressions.log) was read; its four real pump cases fail at unauthorized/persisting countdown assertions. Current independent B5 selection26 includes all four and passes. Tests hold observation by not driving TestApp, confirm unchanged event sequence, and let actual HTTP/writer/SQLite work proceed independently; they do not evaluate after each intermediate mutation or substitute a manual refresh.

The owner chain remains original DownloadEngine/task/queue/store/event pump/coordinator. No schema, wire version, dependency/vendor, alternate scheduler or unbounded mutation history was added in this repair. Existing single action slot, priority, directory bounds, suspend terminal wait and actual normal-exit joins remain.

## Independent reacceptance

All33Rust selections were nonzero LIST then identical package/target/filter RUN. All top-level selections and nested Download child summaries pass with0failed/ignored. Counts overlap and are not an additive distinct total.

| Selection | Result | Evidence |
| --- | --- | --- |
| Desktop `b5_` |26/26|[B5 production](evidence/b5-reaccept-b5-ui.log)|
| Download completion contract |9/9|[Completion](evidence/b5-reaccept-completion.log)|
| Download library / engine integration |128/128;105/105|[Library](evidence/b5-reaccept-download-lib.log), [Engine](evidence/b5-reaccept-engine.log)|
| Download-IPC / IPC / Protocol / Core |9/9;62/62;116/116;98/98|[Download-IPC](evidence/b5-reaccept-download-ipc.log), [IPC](evidence/b5-reaccept-ipc.log), [Protocol](evidence/b5-reaccept-protocol.log), [Core](evidence/b5-reaccept-core-lib.log)|
| B4 UI / original Shutdown / coordinator |30/30;12/12;5/5|[B4](evidence/b5-reaccept-b4-ui.log), [Shutdown](evidence/b5-reaccept-shutdown-original.log), [Coordinator](evidence/b5-reaccept-coordinator-original.log)|
| Archive draft / Job |12/12;129/129|[Draft](evidence/b5-reaccept-archive-draft.log), [Job](evidence/b5-reaccept-archive-job.log)|
| CLI / BrowserHost |15/15;48/48|[CLI](evidence/b5-reaccept-cli.log), [BrowserHost](evidence/b5-reaccept-browser-host.log)|

Other green selections retain domain12/messages16/HTTP26/paused11/Header3/coverage1/settings2, rate2/runtime-speed8/REF06 eleven/Server7/theme10, handoff2 and original render/phase/detail/settings rollback/failure. [Exact commands, exits, summaries and timing](evidence/coordinator-reacceptance-results.json) plus matching LIST logs retain all33selections.

Archive Job129 uses the retained real C# progress/create exports, not synthetic DTOs or a newly launched archive worker. The missing-export122/7result in the initial implementation remains historical evidence; it is not the prerequisite environment used for this current run.

Scoped original/repair Rust format, eight-package all-target check, strict all-target Clippy in Core/Download/Download-IPC/IPC/CLI/BrowserHost and the five source/provenance/lifecycle gates pass. [Format](evidence/b5-reaccept-format.log), [check](evidence/b5-reaccept-all-target-check.log), [specs](evidence/b5-reaccept-spec-contracts.log), [catalog](evidence/b5-reaccept-gpui-component-catalog.log), [Kit](evidence/b5-reaccept-gpui-release-contracts.log), [editor](evidence/b5-reaccept-markdown-editor-contracts.log), [Download UI source](evidence/b5-reaccept-download-ui-lifecycle.log).

All Rust verification inherited jobs1/incremental0/codegen-units256 and coordinator scratch TEMP/TMP in the same PowerShell execution, using one coordinator Cargo lane. The wrapper completed,81new execution logs are retained and final process scans are0.

## Retained native evidence and limits

C# native adapters and safe fixtures are unchanged by this repair. Previous independent [LIST/effects](evidence/b5-coordinator-safe-native-list.log) and [RUN2/PASS2/SKIP0](evidence/b5-coordinator-safe-native.log) remain applicable; this repair acceptance did not rerun them. They inject the typed native boundary and held delegate, never real power/token/capability calls. Current Rust tests retain controlled action dispatch, native-slot/suspend wait/disconnect-unconfirmed and actual Download worker/IPC retirement.

Strict Protocol/Desktop Clippy still returns101 for the three historical diagnostics: archive_browser.rs:563 unnecessary_map_or, browser_handoff_impl.rs:436 nonminimal_bool and settings_task_rem_tests.rs:614 unwrap_used. [Protocol log](evidence/b5-reaccept-clippy-nxtools-protocol.log), [Desktop log](evidence/b5-reaccept-clippy-nxtools-desktop.log). No suppression, unrelated repair or whole-workspace green claim is made.

Physical Shutdown/Sleep/Hibernate/current-user Desktop Exit, native OS-close/IME/DPI/Shell, human visual and installed/Release acceptance remain unperformed. Quit request, actual Download joins and physical process exit are distinct. The separate PlatformHost memory report's native allocation attribution is not newly measured by these B5 tests.

## Coordination

B5 is accepted and completed at these boundaries. The next task includes the user-requested UIA memory root repair before B6 connection experiments; its [scope](../../../docs/reference-code/platformhost-uia-memory-repair.md) requires native ownership and memory evidence, not just startup WarmUp removal or forced GC. B7 remains unstarted. The coordinator changed acceptance/task/coordination records and planning evidence only, not product source. No repair/next-batch implementation dispatch, package/install, system mutation, commit or push occurred.
