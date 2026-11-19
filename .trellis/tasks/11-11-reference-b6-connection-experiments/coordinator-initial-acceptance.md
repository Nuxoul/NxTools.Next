# B6 coordinator acceptance

Date: 2026-11-12

Verdict: **not_accepted — B6-01 requires repair**.

Independent verification passes all37selected behavior groups,841successful executions, nine-package all-target check, affected-owner strict lint, scoped format and five source/lifecycle gates. One confirmed REF-13 control-flow defect remains. No product source was changed during acceptance.

## B6-01 — Terminal native refusal bypasses learning

Priority: **P2**. Owner: REF-13 original segment worker.

- `rust/crates/nxtools-download/src/engine/segmented_runner.rs:763` places `connection_ceiling::observe_refusal` inside `Err(error) if should_retry_transfer(attempt, &error)`.
- The same file's `should_retry_transfer` at2291requires `failed_attempt < MAX_TRANSFER_ATTEMPTS`; `engine.rs:3312` fixes that budget at3.
- Attempt3therefore takes `terminal => break terminal` at803without offering its error to the observation owner.

A supported transfer sequence is one receiving same-key peer plus an additional worker receiving HTTP503on attempt1and genuine pre-header native ConnectionRefused on attempts2and3. The peer can remain receiving throughout on an ordinary sufficiently long segment. The two connection refusals qualify under the frozen REF-13 contract, but the current branch records only one:

| Attempt | Actual error | Retry allowed | Refusal observation |
| --- | --- | --- | --- |
|1|HTTP503|Yes|Nonqualifying; correct|
|2|Native ConnectionRefused with receiving peer|Yes|First qualifying refusal|
|3|Native ConnectionRefused with receiving peer|No|Bypassed|

The observation remains unlearned and later same-authority/route tasks repeat the excess connection. Retry exhaustion must determine the worker's terminal result, not whether valid connection evidence exists.

**Required repair:** offer every failed segment attempt to the existing typed refusal observer before deciding retry versus terminal return. Keep the original three-attempt budget, returned error, task failure priority and physical drain unchanged. Do not treat503itself as refusal evidence or add another controller.

**Required regression:** at the original HTTP/segment-worker boundary, retain a real receiving peer across `503 → native refusal → native refusal`; verify the second refusal learns even when it is the terminal attempt, no fourth request occurs, the original failure remains terminal and workers/socket/writer owners actually drain. Retain no-peer, nonrefusal, lower-cap, route/expiry and same-run cache-eviction coverage.

Evidence classification: **confirmed source control-flow defect**, independently checked against the current worker, budget and observation implementation. This acceptance did not add or claim a new dynamically reproduced HTTP RED. Existing status-exclusion, refusal-only and retry-budget tests pass but do not cover this combined sequence. One directed read-only review was completed; no further review or repair was dispatched.

## Experiment decisions

| REF | Coordinator decision | Audited evidence |
| --- | --- | --- |
|REF-13|Proposed adoption held by B6-01|Refusals `[2,2,2,2] → [2,0,0,0]`, TCP attempts32→26, HTTP requests24unchanged. Median+4.7208%and slowest+4.5378%meet the pre-existing5%bound. Throughput1.436145→1.371404MiB/s: fewer refusals, not throughput improvement.|
|REF-14|Retain-current supported|Immediate fallback reuse reduces requests7→3, but all three formal10second delayed-confirmation candidates fail the body requirement. These exit101records remain failures, not completion timings.|
|REF-15|Retain-current supported|Candidate retirements0in formal trials and warmups. Original released-slot splitting acts first; persistent median improvement0.2959%does not meet15%. No activated candidate lifecycle is certified.|
|REF-16|Retain-current supported|Same-port gain is real; authoritative port-swapped median regresses98.9693%, with TCP attempts6→8. Original connector/DNS order and300mscross-family fallback remain.|

The raw audit covers138recorded formal observations:132authoritative selections and6additional retained REF-16observations, plus92warmups. It directly checks original LIST/RUN logs, raw/aggregate result equality, actual expected exits, AB/BA/AB order, serial timestamps, byte-result/resource assertions, memory sample counts/peaks and recomputed median/slowest statistics. All5319data/source consistency checks pass. The three formal REF-14candidate failures are preserved explicitly.

The frozen5%REF-13gate is present in the starting design. Baseline/candidate input differences for the rejected prototypes were inspected separately; only REF-13changes the retained production algorithm. No performance trials or600second expiry wait were rerun by the coordinator. The retained actual expiry log records600000msand restored receiving width4; genuine refusal and cache-eviction RED/GREEN logs were inspected. Controlled local timing is not Internet performance.

## Independent execution

[Independent results](evidence/coordinator/results.json) records fresh commands, exact filters, preflights, logs and counts. [Inventory](evidence/coordinator/files.txt) contains95verification materials. [Data audit](evidence/coordinator/data-audit.json) retains raw checks and recomputed values.

- **37nonzero LIST→identical-filter RUN groups,841successful executions, FAIL0, ignored/SKIP0, not_run0.** Overlapping selectors are not841distinct tests.
- Download library142, domain12, messages16, original engine105, HTTP-client26, resolution15, artifact4, storage7/capacity3, HTTP-fixture7, Server11, B4pausedURL11/Header3/coverage1, B5completion9, rate2/runtime8and remaining original targets pass.
- DownloadIPC9, Network3, Core98, IPC62, Protocol116, CLI15, BrowserHost48and Core B4settings2pass.
- Ten Desktop TestPlatform selections pass: B5 UI26, original coordinator5, shutdown12, B4 UI30, exact speed-cap11, Server7, window handoff2, deferred render1, REF-01four and link intake6.
- Nine-package locked/offline `cargo check --keep-going --all-targets`: **exit0**.
- Independent strict `cargo clippy --all-targets -- -D warnings`: Download, Network and DownloadIPC **exit0**.
- Ten-file nonrecursive `rustfmt --check`: **exit0**.
- Spec contracts, component catalog, locked GPUI release, Markdown editor provenance and download UI lifecycle gates: **exit0**.

All Rust commands used one lane with jobs1, incremental0, codegen-units256and coordinator scratch TEMP/TMP in the originating execution. Each Cargo invocation checked build-lane availability and at least8GiBcommit headroom. The verification wrapper exited0. No Cargo/rustc or owned B6test/fixture/driver process, or temporary B6TLSdirectory remained at the final resource observation.

## Source and ownership

The20final-source/attribution snapshots and21protected files match the delivered byte copies at the pre-recording audit. This includes UIA, prior acceptance records, Cargo.lock, packaging material and the coordinator-owned coordination document before this acceptance update. No whole-HEAD diff was attributed to B6. Product source remains unchanged by the coordinator; only acceptance evidence/task/coordination records are written.

The adopted candidate uses the original engine table, operation client/policy snapshot, segmented admission/adaptive/worker owners and typed pre-header IO classification. One run's Arc crosses all four spawn sites and survives table eviction. Rejected response-holder, slow-retirement and address-winner mechanisms are absent from production. Original artifact/checkpoint and B4/B5ownership remain covered by the passing selections. Production learned-state diagnostics remain bounded and omit request secrets.

## Known issues and acceptance limits

The full strict suite is not all green. Delivered logs and unchanged source attribution retain three historical diagnostics: Protocol `archive_browser.rs:563` unnecessary_map_or, Desktop `browser_handoff_impl.rs:436` nonminimal_bool and `settings_task_rem_tests.rs:614` unwrap_used. The coordinator inspected those raw diagnostics and byte-identical source; these two failing package selections were not rerun. No lint suppression or unrelated repair was applied.

The verified level is source, owned local HTTP/TLS/SQLite/writer/pipe and production TestPlatform. Physical GUI/IME/DPI/provider behavior, PlatformHost native-memory effectiveness, installed service and Release/package acceptance remain separate. Kernel TCP retransmission is unobserved; application served-byte overlap is reported separately. Rejected prototypes do not establish their unimplemented product integration lifecycles.

B1–B5remain accepted at their recorded boundaries. UIA/native-memory and formal packaging belong to other tasks and are not B6prerequisites. B7remains unstarted. No product repair, package/install, real power/current Desktop exit, system mutation, commit/push or next-batch dispatch was performed. Await explicit repair authorization for B6-01.
