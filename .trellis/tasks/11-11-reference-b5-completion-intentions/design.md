# B5 — Completion intentions

## Scope

REF-07 and REF-11 only. Preserve accepted B1–B4 and the original download, browser, limiter, Server and archive owners. Finish `ready_for_acceptance / not_accepted`; B6/B7 are outside this batch.

## Verified owners

| Boundary | Existing owner and call chain |
| --- | --- |
| Task/domain | `nxtools-download/src/lib.rs::DownloadTask`, custom Deserialize/restore; task identity, request, status and successful completion timestamp remain authoritative. |
| SQLite | `storage.rs::SqliteDownloadStore`; `StoredResolvedRequest` in original request_json; `save_task_row` and original task/transfer/admission transactions. Queues and membership are `download_queues`/`download_queue_items`, read through `load_queues`/`load_queue_order`. |
| Admission | `engine.rs::submit_with_resolution` acceptance under scheduler→tasks→store; duplicate receipts return before fresh admission. `resume_correlated`, retry/auto-rename and original queue admission remain owners. `engine/queue.rs::schedule_queue_locked` persists before publishing and spawning. |
| Success | Ordinary/media final publication reaches `engine.rs::commit_completed_state`: clone task, Completed + final_committed, SQLite persist, replace memory, unlock, TaskChanged. Failure is a distinct `record_failure` path. |
| Presentation/events | `downloads.rs::DownloadApplication::new` seeds old terminal notifications; original event receiver→handle_engine_event→refresh→publish_terminal_notifications. `task_windows_impl.rs` retains keyed task ID/window/generation and same-window transformation. Native close retires presentation/draft only. |
| Coordination | Application-wide `completion_actions.rs::CompletionActionCoordinator`; original terminal batch,400ms coalescing, case-insensitive directory dedup/max3, one30s countdown. `downloads/completion_shutdown_impl.rs` owns creation/tick/cancel/execute/results/timeouts. |
| Native | Original PlatformBridge correlated command→PlatformPipeClient dispatch/parser→WindowsPlatformAdapter→ICompletionSystemActions/WindowsCompletionSystemActions. Existing ShutdownSystem calls bounded shutdown.exe, retaining native exit/error code. |
| Exit | Normal Close uses `cx.quit`; Application.run returns to main. `DesktopRuntime::shutdown_downloads_and_wait` drains original engine work, then retires and joins Download IPC before the existing ExtensionService/ClipboardCaptureWorker cleanup and runtime/supervisor release. Original engine futures share one retained JoinSet; transfer pause, probe/resolver cancellation and actual joins precede successful drain. Indexer/Platform process-lifetime readers remain separate. |

External behavioral references were verified ABDM→legacy NxTools→Mister; exact paths and limitations are in reference-review.md. ABDM Windows Sleep/Hibernate TODO is not support evidence; Mister's failed-or-empty queue predicate and window-owned intention are not adopted.

## State and persistence

- One shared typed `DownloadCompletionIntention`: optional system/application action (Shutdown, Sleep, Hibernate, ExitApplication) plus explicit open-directory feedback. Default is no action/false. Existing completion window/sound/global directory settings remain independent.
- Task intention belongs to DownloadTask and is persisted in its original stored request metadata; missing fields decode to defaults. The original command persists a cloned current task before replacing memory/publishing. It does not rewrite accepted request headers, source identity, artifacts or admission.
- Queue intention belongs to DownloadQueue, stored in an additive original SQLite queue column with default empty intention; no second state database. Queue settings update only their owned column under original store/scheduler ownership.
- Durable intention, task execution attempt/current queue round and ephemeral pending action are distinct. Accepted task attempts carry a stable revision; engine's accepted-work/membership revision changes only after actual accepted mutations, not draft/probe/duplicate receipts. The coordinator consumes successful attempt identities once.
- At coordinator initialization/reload, existing terminal records are observed, not replayed. Nonterminal accepted members form current rounds. No countdown is persisted. Restart restores intention but not one-shot authority from an old terminal record.
- A queue round is nonempty current accepted membership, including waiting, paused and scheduled work. Every member must actually reach Completed. Failed/exhausted/cancelled members do not satisfy it. Removal or moving a member aborts its old round rather than silently turning absence into success. Retrying starts a new attempt. A new round contains fresh attempts and current nonterminal members, retaining completed members only while their existing round continues. Historical terminal rows neither block a later round nor authorize new success; restart seeds only current nonterminal membership.

## Single coordinator and arbitration

The original coordinator remains the only action/countdown owner. Eligible successful task/queue sources coalesce while other accepted work remains nonterminal. One stable action priority applies: **Shutdown > Hibernate > Sleep > ExitApplication**. Directory feedback is independent. This favors the explicit machine-level intention in conflicting sources, rather than running several power actions or allowing callback order to decide. No queue starts its own timer.

A countdown consumes its sources once; Cancel does not erase durable intention and cannot re-arm the same completed attempt. Actual new accepted work/retry/membership mutation cancels the pending countdown through the original state event and work revision. Countdown identity carries correlation, engine generation/work revision and chosen action. Old tick/execute/cancel/native-close/result callbacks must match before clearing anything. Execution rechecks original facts, then retires countdown authority before dispatch. The original pending native command occupies the single action slot: facts and ready sources still update, but no second countdown is created until its matching result or timeout closes the executing window and reevaluates. Enqueue/result failure is terminal for that one-shot, with no fallback or hidden retry.

## Native and Exit boundary

C# retains all Windows API ownership. Add typed Sleep/Hibernate support to the original completion-system adapter and parser/pipe dispatch. Native capability checks and SetSuspendState errors are truthful, including unsupported and privilege codes. Blocking suspend executes outside the serial pipe reader/event thread. Ordinary tests inject at the typed native boundary and never execute real power APIs.

Shutdown retains its existing bounded acknowledgement deadline. Sleep/Hibernate retain their in-flight action slot until the correlated native terminal, including time suspended and resumed. A PlatformHost disconnect retires the matching executing presentation and reports an unconfirmed outcome, not native rejection or success; a late result cannot replay consumed authority. No fallback or automatic retry is scheduled.

ExitApplication uses the original GPUI quit request and post-run runtime cleanup, not process termination. The same correlation enters original runtime/engine drain. Admission closes; transfer controls pause resumably; candidates, probes and resolver futures cancel; the original engine JoinSet awaits actual work and preserves join failures. Download IPC then retires and joins its clients/host even if engine drain failed. Main still attempts existing extension and clipboard cleanup before returning errors; only successful feature cleanup is published as drained. Runtime/supervisor release and physical process exit remain separate layers. No test quits the user's current Desktop process.

## UI component decision

Keep the original440×184 compact progress/completion window and expanded details budgets, original Input entities/focus/IME, keyed identity and presentation-only close. Extend the task's existing completion-control rail with a named official Small outlined Button and released menu/actions; parent `Toolbar.content` preserves boundaries (`Toolbar.child(Button)` would force ghost/compact). Retain applicable existing Shutdown behavior while making the selected action discoverable. Queue intention is edited through the original Settings/Form save owner, including explicit directory option, not the last visible task window.

Reviewed locked0.8.0 public sources: component button/button.rs (`Button`, variants/size), menu/dropdown_menu.rs (`DropdownMenu::dropdown_menu`, PopupMenu build/trigger focus), form/form.rs (`Form::vertical`, `child`, `footer`, `small`), dialog/footer.rs (`DialogFooter::child`), existing Root/titlebar/dialog_shell integration. Countdown remains450×200 fixed/minimum, official DialogContent/DialogFooter, named action heading, clear Small/Medium official Cancel and execute actions. Enter/Space use official activation; Escape cancels the current transient. Cross-entity effects, including immediate results, use the next application turn.

## Diagnostics

Use original task/queue/correlation/engine-generation/attempt identities. Emit bounded intention acceptance/failure, condition transitions, countdown start/cancel reason, execution request, native terminal and cleanup stages/elapsed/native code. No URLs, directory paths, names, headers, cookies, credentials or contents. No tick/progress/frame logs.

## Verification

Vertical behavioral TDD through real original task/queue/SQLite, coordinator and production TestPlatform. Start with the existing task control persistence across close/reopen/engine reload; its current process-only state is a behavioral RED. Add real successful local HTTP/writer completion and failed/cancelled distinctions, all-success queue rounds, concurrent sources/priority, waiting/scheduled/empty/insertion/retry/removal/membership/restart, explicit/new-work cancellation and stale callbacks. Directory tests cross original bounded feedback transport; native tests inject exact supported/unsupported/success/failure/error-code responses. Exit tests distinguish quit request from joined resources and post-run cleanup.

Every test selection uses nonzero LIST then identical RUN and records actual counts/exits/logs. Compiler/API/fixture prerequisites are not behavioral REDs. Run affected Download/Core/Download-IPC/IPC/Protocol/Desktop/PlatformHost checks, strict scoped lint/check/format, B4 preservation and four documentation/source gates. C# fixtures are inspected and effects-reviewed before exact LIST/RUN. No GUI launch, package/install/service/registry mutation, real power, current-process exit, commit or push.
