# IPC quality and lifecycle contracts

Evidence: `repo:rust/crates/nxtools-ipc/src/lib.rs#tests`, `repo:rust/crates/nxtools-ipc/src/local_pipe.rs#tests`.

Test bounded queues/frame reads, caller IDs, immediate results, generation replacement and timeout/disconnect at production router seam. Temporary pipes prove initial deadline and disarm; actual client identity remains service auth owner. Contract-test adapters feed the same queues, never fake successful native apply. Scoped tests/checks use [verification](../../shared/verification.md); retain both contracts below and [shared engineering](../../shared/engineering.md).

## Scenario: Caller-assigned platform command IDs

### 1. Scope / Trigger
- Trigger: a caller must install its pending UI/session state before a fast PlatformHost result can be dispatched.
- Purpose: ensure the command ID exists in consumer pending state before the envelope becomes observable.

### 2. Signatures
- `PlatformBridge::send_command_with_id(command: PlatformCommand, command_id: Uuid) -> Result<Uuid, PlatformBridgeError>`.
- Results are delivered as `PlatformNotification::CommandResult { correlation_id: command_id, result }`.

### 3. Contracts
- The sent `Envelope.id` and returned ID equal the caller-supplied ID; no wire schema change is made.
- The caller registers the ID before invoking the bridge. The active connection pending map owns timeout and disconnect terminal handling.
- An ID already pending on the active connection is rejected as `DuplicateCommandId`; it must not overwrite the existing pending record or enqueue another envelope.
- Generated-ID `send_command` and operation-correlated `send_command_correlated` retain their existing behavior.

### 4. Validation & Error Matrix
- No active connection -> `Disconnected`, with no pending entry.
- Full command queue -> `Busy`, with no leaked pending entry.
- Duplicate in-flight caller ID -> `DuplicateCommandId`, preserving the original command and pending entry.
- Accepted command -> its result/timeout is routed by envelope ID; disconnect reports it in `pending_command_ids`.

### 5. Good/Base/Bad Cases
- Good: a GPUI action stores the supplied ID, then sends; an immediate result finds the exact pending operation.
- Base: ordinary callers use generated IDs.
- Bad: overwrite `router.pending[id]` or send before the consumer installs matching pending state.

### 6. Tests Required
- `caller_supplied_command_id_is_used_for_result_routing`: sent envelope ID and result notification use the supplied ID.
- `caller_supplied_command_id_cannot_replace_an_inflight_command`: duplicate is rejected and exactly one original envelope remains queued.
- Retain existing full-queue, timeout, disconnect and generation-isolation tests.

### 7. Wrong vs Correct
#### Wrong
```rust
let command_id = bridge.send_command(command)?;
panel.pending_action = Some(command_id); // a fast result can arrive before this assignment
```

#### Correct
```rust
let command_id = Uuid::new_v4();
panel.pending_action = Some(command_id);
bridge.send_command_with_id(command, command_id)?;
```

## Scenario: Initial named-pipe frame deadline

### 1. Scope / Trigger
- Changes to LocalPipeReader native waits or untrusted initial newline-framed handshakes.

### 2. Signatures
- `LocalPipeReader::set_initial_frame_timeout(&mut self, timeout: Duration)` sets `initial_frame_deadline: Option<Instant>`.
- `Read::read` returns std::io::ErrorKind::TimedOut after expiry; existing connection identity fields are unchanged.

### 3. Contracts
- Set once before initial Hello; deadline is absolute and partial reads never reset it.
- First newline read disarms deadline. Healthy long-lived sessions are not subject to the initial-frame timeout.
- Preserve disconnect/native-error semantics and existing frame-size validation; deadline does not authorize clients or replace admission limits.

### 4. Validation & Error Matrix
- Empty buffer -> zero; BrokenPipe/NoData/NotConnected -> EOF; elapsed initial deadline -> TimedOut; other native IO failure -> original native error.
- Complete first newline -> deadline None; later idle normal session -> no handshake timeout.

### 5. Good/Base/Bad Cases
- Good: a trickled incomplete Hello expires within its original budget.
- Base: complete Hello followed by idle healthy session survives the former deadline.
- Bad: refresh deadline on every byte or enforce handshake timeout forever.

### 6. Tests Required
- `partial_initial_frame_expires_without_resetting_deadline` and `complete_initial_frame_disarms_deadline_for_normal_session` use temporary pipes; assert exact timeout/disarm behavior.
- Keep service admission release and existing IPC routing tests.

### 7. Wrong vs Correct
```text
Wrong: received bytes -> deadline = now + timeout.
Correct: initial deadline unchanged until first newline -> disarm.
```

## Cancelled partial-frame read regression

`repo:rust/crates/nxtools-ipc/src/lib.rs#read_bounded_frame_into` stores consumed prefix in the
original connection, not an interruptible future-local Vec. Deterministic duplex prefix8192bytes
then timeout/select interruption then suffix must decode exact original+next frame; old behavior
returned tail alone. Preserve existing preallocation1MiB, oversized/EOF/UTF8/nextframe tests,
ordinary-notice saturation real EOF, original cancel fairness and overflow epoch. Never fix a
partial read by raising product deadlines or creating a second reader runtime.

## Scenario: Retiring service endpoint I/O
### 1. Scope / Trigger
Authenticated Indexer endpoint replacement while a peer stops reading or remains idle.
### 2. Signatures
`LocalPipeConnection::into_retiring(retired,write_timeout) -> io::Result<(BufReader<LocalPipeReader>,impl Write+Send,PipeRetirement)>`; `PipeRetirement::retire()` sets the original lease flag and signals a kernel event. `LocalPipeWriter` owns explicit overlapped writes. `SharedIndexerRuntime::serve_admitted_native` passes that signal through the original endpoint owner; IPC owns native transport only.
### 3. Contracts
Native service frame writes have a500ms absolute deadline and wait on independent completion/retirement events. Idle readers block in event-backed ReadFile, never PeekNamedPipe polling. CancelIoEx targets the pending operation; GetOverlappedResult drains completion before OVERLAPPED/buffer/event storage is released. Both ends are FILE_FLAG_OVERLAPPED; read/write events are independent, so the duplicated writer remains duplex. Never toggle shared PIPE_NOWAIT or call std::File::write on these handles. The original coordinator retains16frame nonblocking output, four admissions, actual identity and3second initial Hello/1MiB input. Signal every retirement path, join both I/O workers before slot release. Ordinary and bounded desktop writers share the same explicit completion implementation.
### 4. Validation & Error Matrix
Stopped reader -> frame deadline expires; retirement -> bounded reader/writer exit; native error -> original typed IO error and fixed terminal diagnostic. Old endpoint EOF/errors/requests/results cannot affect the new service lease/generation.
### 5. Good / Base / Bad Cases
Good: B receives HelloAck/status while A's output and scan are held. Base: healthy endpoint remains duplex. Bad: coordinator performs blocking write/flush or holds a service mutex across a session.
### 6. Tests Required
`m4_nonreading_pipe_write_expires_and_releases_the_endpoint`, `m4_retirement_interrupts_a_nonreading_write_before_its_deadline`, `m4_retirement_releases_an_idle_pipe_reader_while_peer_remains_connected`; retain existing native pipe deadline/duplex and service admission/lease regressions. Temporary pipes do not certify installed-service acceptance.
### 7. Wrong vs Correct
Wrong: wait for old peer/scan before admitting a replacement. Correct: authorize/register a new lease, retire old transport, preserve service-owned work and reject old-lease effects.

## Scenario: Idle native waits and transient pipe admission
### 1. Scope / Trigger
Local pipe connect/read/write/accept or Tokio platform acceptance changes.
### 2. Signatures
`connect_local_pipe(name,timeout)`; `accept_current_user_pipe`; `accept_local_indexer_pipe`; `transient_accept_error`; `run_platform_server`. Wire versions/frame limits/security descriptors are unchanged.
### 3. Contracts
Missing-pipe WaitNamedPipe NotFound sleeps25ms capped by remaining absolute deadline. Idle reader waits indefinitely without a handshake deadline, or until its absolute initial deadline/retirement event; newline disarms only the initial deadline. Accept NoData/NotConnected/BrokenPipe/OperationAborted closes the affected instance, recreates it and backs off10→100ms; fixed native-code diagnostics at most1/second. Unrecoverable CreateNamedPipe/native accept errors retain caller failure behavior. Tokio uses the same classification and bounded asynchronous delay.
### 4. Validation & Error Matrix
Client connect-close before ConnectNamedPipe -> recreate and accept the next client. Missing endpoint -> bounded retries/timeout, not spin. EOF -> zero. Incomplete handshake -> TimedOut. Peer not reading -> original absolute write timeout; retire -> direct event wake.1MiB framing validation remains at original frame reader.
### 5. Good/Base/Bad Cases
Good:ten-second idle wait has0periodic wakeups and a healthy second client is accepted. Base: simultaneous waiting read and response write remain duplex. Bad: synchronous ReadFile on a duplicated synchronous file object, overlapped handle with stdFile writes, or cancel without draining completion.
### 6. Tests Required
Temporary-pipe `idle_reader_has_no_periodic_wakeups`, `measure_ten_seconds_of_idle_pipe_reader_wakeups`, `missing_pipe_attempts_are_bounded_by_remaining_deadline`, `disconnected_before_connect_named_pipe_does_not_end_admission`, `tokio_accept_recreates_after_a_closed_preconnected_client`; retain initial/trickle/newline/EOF/duplex/1MiB and all M4 native retirement/deadline fixtures. Measure before production repair and after using the same10second seam; counters are test-only. Keep Indexer safe-library/admission and Desktop bridge tests; installed LocalSystem release acceptance remains separate.
### 7. Wrong vs Correct
Wrong: Peek/sleep every8ms or retry absent pipe immediately; accept232 returns permanently. Correct: completion/deadline/retirement event wait,25ms remaining-budget backoff and bounded transient instance recreation.

## Floating assistant original-connection cleanup
Evidence: `repo:rust/crates/nxtools-ipc/src/floating_cleanup.rs`, `repo:rust/crates/nxtools-ipc/src/lib.rs#PlatformCommandReceiver`. Capture `floating_assistant_connection` before renderer preparation and pass that origin to `send_floating_assistant`; original generation is checked under the same router lock as pending/enqueue. Ordinary commands retain Busy/rollback. Full cleanup admission stores at most one exact envelope in the original ActiveConnection, not a retry task or unbounded queue. Same-token release coalesces; same-lease retire covers release; other token/session/generation/HWND cannot replace it.

Original command dequeue immediately re-admits the retained envelope through CommandRouter::enqueue after capacity returns, registering pending before visibility. Old consumer cannot flush into a replacement connection; EOF/replacement retires the slot and original C# pipe owner owns physical cleanup. Retire has no ordinary pending deadline and waits for exact terminal/disconnect. Floating Retire and human-driven PickImage keep PendingCommand timeout=None; other floating operations retain the ordinary10second deadline. Archive receipt exemptions stay unchanged outside the separate raw-join contract below. Deferred/resumed diagnostics retain original operation correlation without serialized payloads.

Required tests: capacity1 full/duplicate release/competing token+generation/retire coalescing, consumer progress after bridge destruction, replacement origin refusal/no replay, and original Retire terminal after60second sweep. Retain all ordinary queue/pending/timeout/disconnect and archive fair-cancel tests. This is transport ownership evidence, not actual native or release acceptance.

Ordinary numbered-file joining follows [the physical-operation contract](../../platformhost/raw-numbered-join.md): origin-bound pending-before-send, no ordinary Inspect/Start timeout, exact coalesced reserved Cancel with original fairness, and release only on the original operation terminal or connection retirement. Cancel ACK does not free the operation.

## Scenario: Floating image picker human wait
### 1. Scope / Trigger
`FloatingAssistantOperation::PickImage` enters the shared platform command router through the Desktop caller-assigned command ID.
### 2. Signatures
`PlatformBridge::send_command_with_id(PlatformCommand::FloatingAssistant(command), id)` -> `CommandRouter::enqueue` -> original-generation `PlatformNotification::CommandResult { correlation_id: id, result }`.
### 3. Contracts
Register pending before enqueue with timeout=None. Keep the request until the original native selection/cancel result or connection retirement; do not apply a machine deadline to human wait. Preserve bounded admission, caller ID, generation checks and exactly-once completion. Other floating operations keep their existing timeout policy. Native dialog ownership, placement, focus and cancellation remain unchanged.
### 4. Validation & Error Matrix
Waiting beyond ordinary deadline -> pending, no failure notification. Matching delayed selection/cancel -> forward unchanged once. Disconnect -> remove pending and report its ID. Old-generation result -> reject without consuming a replacement request. Ordinary machine command -> original timeout failure.
### 5. Good/Base/Bad Cases
Good: image selection after11/121/601seconds still completes. Base: cancel returns successful selected_path=null without changing the configured image. Bad: report a timeout while the chooser is open, then discard its eventual successful result.
### 6. Tests Required
`repo:rust/crates/nxtools-ipc/src/floating_cleanup.rs#tests` includes `floating_image_picker_waits_for_selection_or_cancel_without_machine_timeout` and `floating_image_picker_disconnect_retires_pending_and_rejects_late_results`. Exercise the original bridge/enqueue/sweep/result path, exact caller ID and data, duplicate completion, disconnect notification and retired-generation rejection. Retain ordinary timeout and other human-picker tests.
### 7. Wrong vs Correct
Wrong: remove native/Desktop wait limits while leaving PickImage on the IPC machine deadline. Correct: exempt this typed operation in CommandRouter while retaining ordinary command deadlines and original-connection retirement.

## Scenario: Human-driven Shell, authorization and native drag
### 1. Scope / Trigger
Platform requests whose native completion waits for a person's choice, elevation authorization or held drag gesture.
### 2. Signatures
`PlatformBridge::send_command_with_id` and ordinary generated-ID sends share `CommandRouter::enqueue`; terminals use the original connection generation and `PlatformNotification::CommandResult { correlation_id, result }`.
### 3. Contracts
`OpenShellTarget` exempts only OpenWith and RunElevated; `LaunchProcess` exempts only run_as_admin=true. InvokeShellAction, EnsureIndexerService and BeginFileDrag also retain timeout=None until real terminal or original connection retirement. Preserve pending-before-enqueue, bounded admission, exact caller ID, duplicate rejection and once-only results. Ordinary Open/Reveal, non-elevated launch and Shell menu/submenu queries retain their10-second IPC deadline. Native Shell query/startup and authorized installer execution keep their separate finite machine budgets. No wire fields, workers, queues or retry policies are added.
### 4. Validation & Error Matrix
Human wait after11/121/601/3601seconds -> no synthetic timeout. Matching success/cancel/failure -> original result forwarded once. Disconnect -> clear pending and publish its ID; old-generation result -> cannot consume same-ID replacement. Ordinary machine wait -> original correlated timeout failure.
### 5. Good/Base/Bad Cases
Good: UAC or Open With returns after the former deadline and completes the same request. Base: long file drag retains one native operation until release/Escape. Bad: report failure while native interaction continues, discard its later success, or exempt every Shell operation indiscriminately.
### 6. Tests Required
`repo:rust/crates/nxtools-ipc/src/lib.rs#tests`: `human_shell_commands_outlive_machine_deadlines_and_route_terminal_once`, `human_shell_commands_retire_on_disconnect_without_consuming_replacement_results`, `machine_shell_commands_keep_their_original_timeout`. Exercise every typed exemption, caller ID, delayed terminal data, duplicate completion, disconnect and same-ID replacement. Keep picker, cleanup, queue pressure and ordinary timeout tests.
### 7. Wrong vs Correct
Wrong: a single short completion deadline for both human dialogs and machine requests. Correct: typed human-operation exemptions in the original router, finite machine-stage limits in their owning adapters, original connection-scoped terminal handling.

## Window material single-flight submission

Evidence: `repo:rust/crates/nxtools-ipc/src/window_material.rs`. ApplyWindowMaterial uses a capacity-bounded FIFO in the original ActiveConnection and original PlatformCommandReceiver. Register caller ID, generation and ordinary10second deadline before admission; ordinary commands and retained cancellations remain independently deliverable. Owner supersession/close removes only its exact unsent ID through `retire_queued_window_material`; dispatched work keeps its native slot until the actual terminal or connection retirement.

The original result route releases only the exact current-generation in-flight ID and wakes the same consumer. Unknown/duplicate/old-generation results cannot release another slot. Native `window_material_unavailable` or an in-flight IPC deadline quarantines material output for that connection and fails queued owners; it never creates a replacement worker while the old native call may remain active. A queued-only timeout removes that command without quarantining a healthy in-flight call. Disconnect/replacement retires all original pending IDs and never replays waiting HWND actions.

Required tests cover original terminal routing/wake, ordinary-command responsiveness, bounded admission, owner retirement/coalescing, exact generation and duplicate results, native unavailable, in-flight timeout, queued timeout and disconnect. Synthetic bridge fixtures never execute a real indexer/service or mutate Windows material settings.
