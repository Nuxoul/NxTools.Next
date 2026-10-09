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
- Changes to LocalPipeReader polling or untrusted initial newline-framed handshakes.

### 2. Signatures
- `LocalPipeReader::set_initial_frame_timeout(&mut self, timeout: Duration)` sets `initial_frame_deadline: Option<Instant>`.
- `Read::read` returns std::io::ErrorKind::TimedOut after expiry; existing connection identity fields are unchanged.

### 3. Contracts
- Set once before initial Hello; deadline is absolute and partial reads never reset it.
- First newline read disarms deadline. Healthy long-lived sessions are not subject to the initial-frame timeout.
- Preserve disconnect/native-error semantics and existing frame-size validation; deadline does not authorize clients or replace admission limits.

### 4. Validation & Error Matrix
- Empty buffer -> zero; broken/closed pipe -> EOF; elapsed initial deadline -> TimedOut; native peek failure -> native IO error.
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
