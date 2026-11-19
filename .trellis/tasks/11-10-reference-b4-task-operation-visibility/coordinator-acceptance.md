# B4 coordinator acceptance

## Verdict

**Accepted at source, production TestPlatform, owned HTTP/SQLite/writer and original-pipe fixture boundaries.**

Acceptance date:2026-11-11. B4-01, B4-02, B4-E1 and B4-E2 are closed after the [repair handoff](acceptance-repair.md) and independent reacceptance below. No open B4 product finding remains in the reviewed repair. B1/B2/B3 retain their accepted boundaries. Canonical Archive remains **Phase E — Format expansion and ZIP editing / in_progress**.

The next taskbook batch is **B5 — Completion intentions**, REF-07/REF-11. The user requested its prompt after a passing B4 verdict; no B5 implementation was dispatched or started.

## Closed acceptance items

| Item | Verified production boundary |
| --- | --- |
| B4-01 — Effective validator | [ResourceValidator::if_range](../../../rust/crates/nxtools-download/src/lib.rs:398) is shared by replacement admission, [actual If-Range](../../../rust/crates/nxtools-download/src/http.rs:1362) and [response validation](../../../rust/crates/nxtools-download/src/http.rs:2298). Blank/whitespace/weak ETag uses valid Last-Modified; legal quoted-empty strong ETag remains usable. [Real HTTP/SQLite/writer regression](../../../rust/crates/nxtools-download/tests/support/b4_url_cases.rs:558) proves retained plan/prefix, exact outgoing validator, one suffix transfer from the checkpoint rather than zero, same task and exact final bytes for all four cases. |
| B4-02 — Retired attempt freshness | The original task owns one checked [revision](../../../rust/crates/nxtools-download/src/engine/paused_url.rs:100), consumed only on acquired admission and retained on retirement. [Exact A→B→A regression](../../../rust/crates/nxtools-download/tests/support/b4_url_cases.rs:650) proves cancelled A and failed B preserve old state, exact replayA performs no HTTP/mutation, and fresh C resumes exact bytes. No UUID history/store/schema is added. Required token is exposed only in original TaskDetail/command, with command11/Hello4 and shared callers. |
| B4-E1 — Draining pipe cancellation | [Original pipe/held-probe test](../../../rust/crates/nxtools-download-ipc/src/paused_url_tests.rs:11) starts actual written/paused state and a held candidate, enters Draining, rejects new work without HTTP and accepts precise Cancel. It separately awaits candidate terminal/caller join, socket drain, InvalidState after lease retirement and pipe-client drain, while asserting old request/probe/plan/checkpoints/artifact/queue facts. |
| B4-E2 — Close and late result | [Production TestPlatform witness](../../../rust/apps/nxtools-desktop/src/downloads/settings_task_rem_tests.rs:3556) activates original Button/Input/Confirm and runs its sole original background task on an owned worker. The old view remains strongly owned. Original simulate_close/native-close watch cancels the held probe; the worker joins while its foreground result remains queued. A new same-task window is created before dispatching that result. New owner/Input, fresh revision2 attempt, stale-close isolation and retained old task/plan/bytes all pass; sockets/listener drain is asserted. |

The supplied original [validator RED](evidence/b4-repair01-red.log) and [replay RED](evidence/b4-repair02-red.log) were read and retained with their LIST logs. They show the actual empty If-Range and accepted retired A respectively. Independent current-source runs below pass the same product regressions. E1/E2 are additional production-boundary evidence, not invented product REDs.

## Independently executed reacceptance

All30selections were nonzero LIST followed by identical package/target/filter RUN. Every top-level selection passed with0failure/ignored; nested Download child summaries also passed. Counts overlap and are not a distinct cumulative total.

| Selection | Result | Retained run evidence |
| --- | --- | --- |
| Download paused URL contract, all |11/11|[Replacement](evidence/b4-reaccept-paused.log)|
| Download message contract, all |16/16|[Messages](evidence/b4-reaccept-messages.log)|
| Download-IPC library, including E1 |8/8|[Original pipe](evidence/b4-reaccept-download-ipc.log)|
| Desktop `b4_`, including E2 |30/30|[Production UI](evidence/b4-reaccept-ui.log)|
| Download library / HTTP client contract |127/127;26/26|[Library](evidence/b4-reaccept-download-lib.log), [HTTP](evidence/b4-reaccept-http-client.log)|
| Real Header / coverage / Core settings contracts |3/3;1/1;2/2|[Headers](evidence/b4-reaccept-headers-http.log), [Coverage](evidence/b4-reaccept-coverage-http.log), [Settings](evidence/b4-reaccept-core-settings.log)|
| Core library |98/98|[Core](evidence/b4-reaccept-core-lib.log)|
| REF-06 / Server / theme |11/11;7/7;10/10|[Limits](evidence/b4-reaccept-ref06.log), [Server](evidence/b4-reaccept-server.log), [Theme](evidence/b4-reaccept-theme.log)|
| Scroll / handoff / render owner / phase size / selected detail |1/1;2/2;1/1;1/1;1/1|[Scroll](evidence/b4-reaccept-task-scroll.log), [Handoff](evidence/b4-reaccept-handoff.log), [Render](evidence/b4-reaccept-render-owner.log), [Size](evidence/b4-reaccept-phase-size.log), [Detail](evidence/b4-reaccept-detail.log)|
| Settings rollback / failure |1/1 each|[Rollback](evidence/b4-reaccept-settings-rollback.log), [Failure](evidence/b4-reaccept-settings-failure.log)|
| Original batch / dynamic rebalance |3/3 each|[Batch](evidence/b4-reaccept-batch.log), [Rebalance](evidence/b4-reaccept-rebalance.log)|
| Parallel reopen / range reopen / request context / changed resource |1/1 each|[Parallel](evidence/b4-reaccept-parallel-reopen.log), [Resume](evidence/b4-reaccept-range-reopen.log), [Context](evidence/b4-reaccept-request-context.log), [Resource](evidence/b4-reaccept-changed-resource.log)|
| IPC / Protocol / CLI / BrowserHost |62/62;115/115;15/15;48/48|[IPC](evidence/b4-reaccept-ipc.log), [Protocol](evidence/b4-reaccept-protocol.log), [CLI](evidence/b4-reaccept-cli.log), [BrowserHost](evidence/b4-reaccept-browser-host.log)|

[Exact commands, exits, counts and timings](evidence/coordinator-reacceptance-results.json) and matching `*-list.log` files retain this round's execution. One Cargo lane was used; every Rust process inherited jobs1/incremental0/codegen-units256 and coordinator scratch TEMP/TMP from the same PowerShell execution.

Quality verification:

- Original/repair changed-Rust union scoped rustfmt: [PASS](evidence/b4-reaccept-format.log).
- Eight affected packages, all-target cargo check: [PASS](evidence/b4-reaccept-all-target-check.log).
- Core/Download/Download-IPC strict all-target Clippy: [Core](evidence/b4-reaccept-clippy-nxtools-core.log), [Download](evidence/b4-reaccept-clippy-nxtools-download.log), [Download-IPC](evidence/b4-reaccept-clippy-nxtools-download-ipc.log), all exit0.
- Four source/provenance gates: [Specs](evidence/b4-reaccept-spec-contracts.log), [Catalog](evidence/b4-reaccept-gpui-component-catalog.log), [Kit](evidence/b4-reaccept-gpui-release-contracts.log), [Editor](evidence/b4-reaccept-markdown-editor-contracts.log), all PASS.

The exact requested Trident default UA, editable Settings value and draft-only Reset retain their real HTTP and production input/Apply/Cancel evidence. Template preview/admission, actual written-range coverage, live rebalance/window chain, precise limits, Server and original window budgets remain green. The lockfile change only adds already-locked Tokio to IPC test dependencies; no dependency version, production dependency or vendor upgrade occurred.

## Limitations

[Desktop strict Clippy](evidence/b4-reaccept-clippy-nxtools-desktop.log) returns101 only for the two retained baseline findings: `downloads/browser_handoff_impl.rs:436 nonminimal_bool` and `downloads/settings_task_rem_tests.rs:612 unwrap_used`. Rules were not weakened; there is no whole-workspace strict-lint green claim.

The E2 evidence exercises original TestPlatform native-close, not physical Windows OS-close. Physical IME/DPI/Shell, human visual, installed/Release and full-workspace acceptance remain separate. The IPC accept thread retains its original test-executable lifetime; candidate/socket/lease/client drain is independently asserted before that executable exits.

## Coordination

B4 is accepted and completed at the stated boundaries. The independent wrapper exited0,70reacceptance logs and original RED evidence are retained, and final scans found0compiler/linker/test/archive-worker verification processes. No product source repair, package/install, product/reference GUI launch, system mutation, commit or push was performed by the coordinator. B5 prompt preparation is not implementation dispatch; B6/B7 remain unstarted.
