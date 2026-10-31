# IPC structure

Evidence: `repo:rust/crates/nxtools-ipc/src/lib.rs#platform_bridge`, `repo:rust/crates/nxtools-ipc/src/local_pipe.rs#LocalPipeConnection`.

lib.rs owns bounded async notifications/commands, active connection generations, pending IDs, sweep/timeouts and bounded sync/async line helpers. Windows-only local_pipe owns ACL/native handles, actual client PID/session and initial-frame timeout. Domain/native feature actions stay in owning apps; no UI callback state or SQL here. Contract-test seams feed the same production queues, not a parallel router. [Shared architecture](../../shared/architecture.md).

CommandRouter::enqueue is the single private pending-registration/queue-admission implementation used by ordinary commands and summon replies. Caller-specific duplicate-ID and error mapping stay at their entry paths; generation, queue rollback and terminal ownership remain in the same router. Do not add a parallel queue manager or conflate summon reply delivery with ordinary notifications.

Archive read uses archive_read.rs to intercept typed archive events/receipts into one claimed
capacity8channel, never a competing UI receiver clone. Original connection generation is checked
under CommandRouter before enqueue. Original connection reserves2read lifecycle records at
start; each can retain/coalesce one cancellation independently of normal queue/receipt pressure.
PlatformCommandReceiver owns fair ordinary/cancel dequeue with start-before-cancel and one
coalesced wake. No retry task, queue expansion or replay across generations. Only exact remote
cleanup_complete or original connection retirement releases that read record; local consumer
retirement is not cleanup. The public contract-test command consumer is this production owner.
Overflow emit and flag/event8/wake1 drain/reset are one short synchronous epoch boundary, shared
by fast-path and waiting select; never hold it across await or clear later overflow blindly.
Normal UI traffic keeps its original bounded queue. See
[product read contract](../../nxtools-archive/backend/product-read.md).

Formal P208 archive_virtual_drag.rs uses the same original router/receiver: one admitted native drag
and one coalesced exact cancellation, formal+legacy alias. Start-before-cancel, fair ordinary/cancel
turn and original wake are reused. Full ordinary queue cannot discard source retirement cancellation.
Cancel ACK/local view retirement does not free identity; original start native terminal frees it unless
virtual_cleanup_incomplete, or original connection retirement clears it. Only bounded held start/cancel
receipts bypass ordinary10s sweep; native120s/cleanup owner unchanged. No new pipe/reader/retry task.
Formal `send_archive_virtual_drag(command,receipt_id,origin_connection)` compares ORIGINAL read
connection with active generation under that SAME router lock before pending/reservation/enqueue.
Generic unbound formal starts fail closed; legacy isolated spike alias remains compatible. Caller
browser generation/current is_connected does not authorize an old source after reconnect. Formal
cancel carries its admitted original connection too; no ordinary timeout or native owner expansion.

Ordinary notification saturation must never suspend the same pipe task that writes retained
archive cancellation. The owner uses nonblocking delivery and fails/physically closes that
ORIGINAL PlatformHost connection when its ordinary receiver is full/closed. Existing C# EOF
retires that session and drains all of its owned runs; this connection-wide fallback also
interrupts its other pending native commands, not unrelated host processes. Do not replace it
with local Disconnected or await a saturated disconnect-notification queue before closing pipe.
Real-chain regression fills ordinary notifications BEFORE Drop/receipt routing, checks actual
test-host EOF/exit and original source/helper release before business-credit timeout.

The original platform connection owns pending_frame across read/sweep/command select turns.
read_bounded_frame_into may consume a prefix before awaiting suffix; dropping that future must
not drop consumed bytes. Retain the finite <=1MiBprefix in the connection, clear only at complete
newline/connection retirement; no extra reader task/queue or enlarged frame. Handshake/summon
one-shot reads retain their ordinary wrapper. The read reservation matches archive version as
well as operation/session/generation, including actual cleanup release. Regression
`repo:rust/crates/nxtools-ipc/src/lib.rs#interrupted_large_frame_read_retains_consumed_prefix`
interrupts after8192consumed bytes, resumes exact frame and preserves following frame.
