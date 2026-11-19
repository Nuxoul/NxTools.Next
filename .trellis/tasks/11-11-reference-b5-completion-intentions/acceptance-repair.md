# B5 acceptance repair

Status: **accepted at the [coordinator's stated boundaries](coordinator-acceptance.md)**. Scope: B5-01, B5-02 and B5-03; all are closed after independent repair checks.

## Boundaries

Original DownloadEngine/task/queue/SQLite commit → bounded DownloadEvent stream → DownloadApplication event pump/authoritative snapshot → CompletionActionCoordinator → original countdown/typed platform endpoint. Rust retains domain and coordination; GPUI retains presentation; C# retains Windows effects. No additional coordinator, scheduler, transport or persistence store.

B1–B4 remain accepted. Preserve native action arbitration, priority, directory deduplication, suspend pending/disconnect behavior, and normal Download worker/IPC joins. Coordinator reports/evidence and `docs/reference-improvement-coordination.md` are read-only. B6/B7 remain paused; Archive remains **Phase E — Format expansion and ZIP editing / in_progress**.

## Repair sequence

1. B5-01: retain bounded authoritative queue membership invalidation facts across coalesced snapshots. A destructive membership change aborts its original round; a genuinely later acceptance can establish a new round. Test add/remove and out/back while the original UI event pump is not driven, with real HTTP/writer/SQLite completion and a later successful round.
2. B5-02: reject terminal intention updates at the original task command/domain boundary. Keep nonterminal persistence, rollback and presentation lifecycle. Test durable success before observation, public intention command, then original pump; also retain a legitimately armed successful attempt.
3. B5-03: publish durable retry acceptance even in a stopped queue; persist/project the new accepted attempt as waiting rather than a failed terminal. Keep stopped admission, artifact/checkpoint, retry and idempotency semantics. Test original pump cancellation, StartQueue real success and stale callbacks.

## Verification

Each target regression uses nonzero LIST followed by the identical RUN. Record actual behavioral RED/GREEN separately from compilation/fixture prerequisites and first-pass coverage. Holding observation means no TestApp scheduling or direct coordinator/refresh calls during the mutation interval; owned HTTP/download workers continue independently.

Run affected Download, message, IPC, B5/production-window and B1–B4 preservation selections, scoped format, applicable strict lint/check, the four source/provenance gates and Download UI lifecycle gate. Retained real Archive wire exports may be read only. Attribute unchanged historical lint diagnostics separately.

Power/Exit use typed fake/TestPlatform only. No physical power/current Desktop exit, product/reference GUI, packaging/install, system/service/registry changes, commit or push. Final delivery is `ready_for_acceptance / not_accepted`, pending independent retest.

## Implemented owner contracts

### B5-01

The original engine scheduler gate retains one destructive membership cutoff per existing queue. Successful removal and assignment out of a queue record the source queue cutoff after the original SQLite commit; same-queue no-op and failed transactions do not change it. Each original accepted task/retry retains a session-local accepted_revision. First dispatch and pause/resume do not restamp it. No schema, persistence owner, event history or additional lock/coordinator was added.

The original completion snapshot carries these facts under scheduler→tasks→store. QueueRound aborts the original round even when add/remove or out/back disappear from the next member snapshot. A genuinely later acceptance can establish a new nonempty successful round. Task intentions, native pending slot, priority and directory bounds retain their original owners.

The initial coordinator snapshot establishes current nonterminal membership and cutoff as its baseline, preserving original multiqueue success/priority. An already seeded member advances attempt 0→1 on first dispatch only with unchanged queue/cutoff, without restamping acceptance. Later destructive cutoff changes abort that live round independently of coalesced UI observation; terminal history does not seed authority.

### B5-02

The original set_task_completion_intention command/domain boundary rejects Completed, Failed and Cancelled before SQLite, memory or events. Nonterminal persistence and rollback remain unchanged. Terminal preferences are not stored for future attempts, so they cannot retroactively arm an old successful attempt.

### B5-03

resume_eligible_correlated publishes the original TaskChanged after durable newly admitted retry, even if no worker starts. Duplicate admission does not increment attempt/revision or publish another acceptance. completion_snapshot projects raw Failed plus stored eligible admission as Waiting for completion authority; the original scheduler retains raw failure/admission, stopped state and artifact/checkpoint ownership. Queue start uses the existing daily-schedule API; no StartQueue command, sampler or refresh loop was added.

## Behavioral regressions

All four tests use the original persisted DownloadApplication, public commands, real local HTTP, writer publication and SQLite. Observation is held by not driving TestApp while independent runtime/download workers continue. The original production pump resumes once the mutation interval ends; there is no per-mutation coordinator update or manual refresh.

| Selection / log stem | LIST / RUN / PASS / FAIL / SKIP / not_run | LIST / RUN exits | Boundary |
| --- | --- | --- | --- |
| b5_repair_0 / b5-repair-prerepair-regressions |4 /4 /0 /4 /0 /0|0 /101|Both coalesced membership cases, post-terminal command and stopped-retry notification expose their actual countdown failure.|
| b5_repair_01_ / b5-repair-01-green |2 /2 /2 /0 /0 /0|0 /0|Both aborted rounds reject authority; later real successful rounds remain usable.|
| b5_repair_0 / b5-repair-02-green-03-waiting-red |4 /4 /3 /1 /0 /0|0 /101|Terminal command guard passes; retry event cancels old countdown, but accepted attempt is still Failed instead of Waiting.|
| b5_repair_0 / b5-repair-all-green |4 /4 /4 /0 /0 /0|0 /0|All required pump-held sequences, stopped waiting/duplicate/start/success and stale callbacks pass.|

Positive once-only typed Sleep execution, controlled native failure50 and late callback checks are supplementary coverage; their first successful run is not claimed as a separate RED. Real Windows power and current Desktop exit were not invoked. Existing execute-time recheck already blocked old power requests; B5-03 corrects prompt cancellation and new-attempt eligibility, not a demonstrated physical misexecution.

Four test names in downloads/settings_task_rem_tests.rs:

- b5_repair_01_coalesced_accept_remove_aborts_original_round
- b5_repair_01_coalesced_out_back_aborts_original_round
- b5_repair_02_post_terminal_command_cannot_arm_completed_attempt
- b5_repair_03_stopped_retry_cancels_then_succeeds_through_original_pump

All successful repair fixtures await engine.shutdown_and_wait, HTTP connection drain and listener close. Full preservation, quality, resource and delivery records are tracked separately in repair-verification.md.

## Handoff

All33affected/preservation selections pass their nonzero LIST→identical RUN; final Desktop B5 is26/26 and retained real Archive Job replay is129/129. Scoped format, eight-package all-target check and all five source/lifecycle gates pass. Six strict Clippy packages pass; Protocol/Desktop retain only the three separately attributed historical diagnostics. Actual worker/HTTP/IPC joins and the final zero-resource scan are recorded in repair-verification.md and repair-verification.json. Exact repair paths and protected-byte attribution are in repair-changed-paths.txt and repair-audit.json.

Independent B5 reacceptance is complete with verdict accepted at the stated boundaries. Physical power/current Desktop exit, package/install/system mutation, commit or push remain unperformed. The next prompt adds PlatformHost UIA memory root repair before B6; it does not dispatch implementation.
