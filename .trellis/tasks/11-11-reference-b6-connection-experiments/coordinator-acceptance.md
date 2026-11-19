# B6 coordinator acceptance

Date: 2026-11-12

Verdict: **accepted at source, owned local HTTP/TLS/SQLite/writer/pipe and production TestPlatform boundaries**. **B6-01 is closed.** No confirmed B6 product finding remains at these levels.

## B6-01 closure

The original `spawn_segment_transfer` offers each failed attempt exactly once to the typed connection-ceiling observer before retry eligibility. An exhausted or nonretryable error then returns unchanged. The production file differs from the repair before-image only by this observation/retry ordering; the three-attempt budget, retry delay, error priority and physical worker/checkpoint owners are unchanged.

The regression `engine::connection_experiments::b601::terminal_refusal_after_http_503_learns_without_an_extra_attempt` crosses the original HTTP client, real probe, SQLite queued identity, segment workers and artifact writer. One peer actually receives while the target encounters `HTTP503 → ConnectionRefused → terminal ConnectionRefused`. Scheduling gates verify no learning after503and no duplicate learning after one refusal. The listener closes without closing the peer socket; native refusal is not synthesized from an HTTP status.

| Boundary | Delivered RED | Delivered GREEN | Independent repaired run |
| --- | ---: | ---: | ---: |
| Target attempts |3|3|3|
| Typed native refusals |2|2|2|
| Terminal WinSock code |10061|10061|10061|
| Effective width queried with4 |4|1|1|
| Exact completed peer bytes |4194304|4194304|4194304|
| Server accepted / HTTP requests |4/4|4/4|4/4|
| Final output published |No|No|No|
| Actual worker/writer/engine/server/reader joins |Pass|Pass|Pass|

RED and GREEN use byte-identical test/fixture inputs. RED production equals the repair before-copy and fails only the final expected-width assertion after the error/attempt/byte/drain assertions. GREEN changes only the approved production ordering. The coordinator inspected these raw records and independently ran the regression inside the143-test Download library selection. The4MiBpeer range is byte-exact; the8MiBpartial artifact remains incomplete and unpublished.

[Repair evidence](b6-01-repair.md), [independent regression result](evidence/coordinator-b601-result.json) and [input/source audit](evidence/coordinator-reacceptance/evidence-audit.json) retain the exact facts. The [initial verdict](coordinator-initial-acceptance.md) and [initial structured result](evidence/coordinator-initial-verification.json) remain historical evidence, not the current verdict.

## Independent repair verification

[Fresh results](evidence/coordinator-reacceptance/results.json) contain exact commands, filters, counts, exits, elapsed time and preflights. [Inventory](evidence/coordinator-reacceptance/files.txt) lists99verification materials, including raw logs, drivers and seven source snapshots.

- **37nonzero LIST→identical-filter RUN groups,842successful executions; FAIL0, ignored/SKIP0, not_run0.** Overlapping selectors are not842distinct tests.
- Download lib143; domain12; messages16; original engine105; HTTP-client26; resolution15; original artifact/storage/capacity/queue/HTTP-fixture/Server/B4/B5/rate/runtime selections pass.
- DownloadIPC9, Network3, Core98, IPC62, Protocol116, CLI15, BrowserHost48and Core B4settings2pass.
- All ten Desktop TestPlatform groups pass: B5 UI26, original completion coordinator5, shutdown12, B4 UI30, exact speed-cap11, Server7, window handoff2, deferred render1, REF-01four and link intake6.
- Nine-package locked/offline `cargo check --keep-going --all-targets`: **exit0** on the independently tested source.
- Download, Network and DownloadIPC strict all-target Clippy with `-D warnings`: **exit0**.
- Eleven-file nonrecursive `rustfmt --check`: **exit0**.
- Spec contracts, component catalog, locked GPUI release, Markdown editor provenance and download UI lifecycle gates: **exit0**.

All Rust execution used one lane, jobs1, incremental0, codegen-units256and coordinator scratch TEMP/TMP in the originating environment. Every Cargo invocation checked available build lane and at least8GiBcommit headroom. The verification wrapper exited0. Seven relevant source/fixture snapshots remained byte-identical throughout independent execution. No owned test/fixture/compiler process or temporary B6TLSdirectory remained at the final observation.

The repair evidence audit passes100checks: exact production difference, identical RED/GREEN/current inputs, raw RED/GREEN outcomes, all37settled-source delivery LIST/RUN records and lint attribution. Direct byte comparison of2177relevant frozen experiment/prior acceptance/source files passes before coordinator record updates. Concurrent Archive, Quick Launch, Desktop, C#and shared reference-document work is not attributed to B6-01 or overwritten.

## Accepted experiment decisions

| REF | Decision | Basis and tradeoff |
| --- | --- | --- |
|REF-13|**adopt**|Refusals `[2,2,2,2] → [2,0,0,0]`, TCP attempts32→26, HTTP requests24unchanged. Completion median+4.7208%and slowest+4.5378%meet the frozen5%bound. Throughput1.436145→1.371404MiB/s; the benefit is fewer refusals, not faster transfer. Terminal-refusal learning now passes the added worker regression.|
|REF-14|**retain-current**|Immediate fallback reuse saves requests, but all three formal10second delayed-confirmation candidates fail the body requirement. Those failures remain explicit; no response holder is shipped.|
|REF-15|**retain-current**|Candidate retirements remain0in formal trials/warmups; the original released-slot split acts first. Persistent median improvement0.2959%does not meet15%. No retirement mechanism is shipped.|
|REF-16|**retain-current**|Same-port gains do not offset port-swapped median regression98.9693%and TCP attempts6→8. Original DNS order and300mscross-family fallback remain; no winner cache is shipped.|

The initial coordinator [raw-data audit](evidence/coordinator/data-audit.json) covers138recorded formal observations,132authoritative selections and92warmups. Its5319consistency checks and recomputed statistics remain applicable to the unchanged frozen evidence. Repair acceptance did not rerun performance trials or the600second expiry observation and makes no new performance claim. Kernel TCP retransmission remains unobserved; application served-byte overlap is separate.

## Known issues and acceptance limits

The full strict suite is not all green. Settled-source delivered strict logs retain only three historical diagnostics: Protocol `archive_browser.rs:563` unnecessary_map_or, Desktop `browser_handoff_impl.rs:436` nonminimal_bool and `settings_task_rem_tests.rs:614` unwrap_used. These logs and attribution were inspected; the independent strict rerun covers the three affected owner packages above, not the two historically failing package selections. No suppression or unrelated repair was applied.

Physical GUI/IME/DPI/provider behavior, PlatformHost native-memory effectiveness, Internet performance, installed service and Release/package acceptance remain separate. Rejected prototypes do not certify unimplemented product integration lifecycles. Controlled fixtures and TestPlatform do not establish those levels.

B1–B6are accepted at their recorded boundaries. UIA/native-memory and formal packaging are independently owned, not B6prerequisites. B7remains unstarted. The coordinator changed no product source and performed no package/install, real power/current Desktop exit, system mutation, commit/push or next-batch dispatch. No B6 blocker remains at the accepted levels.
