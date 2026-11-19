# Ordinary numbered-file join

Evidence: `repo:rust/apps/nxtools-desktop/src/tools/raw_join.rs`, `repo:rust/crates/nxtools-protocol/src/raw_numbered_join.rs`, `repo:rust/crates/nxtools-ipc/src/raw_numbered_join.rs`, `repo:csharp/NxTools.PlatformHost/Protocol/RawNumberedJoinSession.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/RawNumberedJoin.cs`. Reference: [volume boundaries](../../../docs/reference-code/archive-volumes.md).

## 1. Scope / Trigger

Explicit Archive Browser File/Open-menu ordinary `.001/.002/...` byte concatenation beside “新建压缩包”. This is not archive opening, extraction, repair, conversion or split-set editing. Signature-bound ZIP/7z entrances remain separate; `.z01` is not a raw numeric part. Generic raw sets cannot establish whether any part, including an unknown final part, is missing.

## 2. Signatures

- `RawNumberedJoinCommand { version, operation_id, session_id, generation, action, selected_path?, preview_id?, output_path? }`, outer `PlatformCommand::RawNumberedJoin`, wire `raw_numbered_join` under unchanged protocol1.
- `PlatformBridge::send_raw_numbered_join(command, command_id, origin_generation) -> Result<Uuid, PlatformBridgeError>`.
- Reply `{ version, operation_id, session_id, generation, preview?, terminal? }`. Preview `{ preview_id, suggested_output, parts:[{name,length}], total_bytes }`; terminal `{ outcome, code, bytes_written, cleanup_complete, native_error_code }`.
- Native `Inspect(selected, token) -> RawJoinPreview`; `Join(frozen_preview, fresh_output, operation, token, observation?) -> RawJoinResult`.

## 3. Contracts

Version1, nonempty UUIDs and positive generation are required; unknown command/reply fields reject. Inspect carries only selected_path; Start only frozen preview_id/output_path; Cancel neither. Envelope correlation is operation_id; result correlation is the original command envelope ID. Native source versions stay private to the original per-pipe session, never client-supplied.

The same-directory family has one basename, canonical minimum-three-digit ordinals1..1024, no gap or alias. Later-part selection discovers the entire observed set. Metadata is bounded to1024parts,262144 UTF-8 name bytes and a checked34359738368byte total. Preview is metadata-only and creates no output. Start re-admits the frozen names/lengths/native identities; it does not reread content for a digest.

Original `ArchiveSourceLease` holds read-only source/ancestor protection. Native opens all bounded source leases before output, retains them through actual transfer/finally, and streams once with one65536byte buffer. Original `ArchiveOutputSession.Prepare/BeginExtractedFile(overwrite:false)/Write/Commit/Cancel` owns the direct fresh final name. Existing parent required; output cannot be an existing part or any new numeric member of the same family. No content hash, candidate/rename/readback, source deletion or whole-family buffering is added.

The original pipe session owns one active operation/frozen preview. Synchronous admission precedes detached waiting on the original dispatcher. `BoundedStaOperationRunner` uses the existing named max1 slot and an infinite uncancelled outer wait; only the inner finite operation observes cancellation. Stop/DisposeAsync cancel and await actual work. No serial-reader/shared-event-thread IO or replacement runtime is introduced.

Original IPC installs pending before enqueue, binds origin_generation and reserves one physical operation. Inspect/Start have no ordinary10second timeout. Exact Cancel has coalesced reserved fair delivery even when the ordinary queue is full. Cancel ACK never releases pending/slot; only the original Inspect/Start CommandResult or connection retirement does. Generic send cannot bypass origin-bound admission.

Desktop owns only this slice's stable inputs and operation state. The original single raw-join window/global weak target, sole platform forwarder, Files/Save picker and native HWND remain. Archive PopupMenu callbacks and both first-open/existing-window activation defer beyond the menu/browser lease. Replies defer through weak window/entity and validate pending ID/session/generation/connection. Source edits invalidate preview; official Dialog requires explicit confirmation before Start. Close during work requests Cancel and waits for actual cancelled/cleanup-complete terminal. Failure/late success stays visible; disconnect reports unknown output rather than fabricated cleanup.

The actual official Medium32 confirmation Button dispatches public `dialog::Confirm { secondary:false }` only for `ClickEvent::is_keyboard()`. DialogAction retains pointer Confirm ownership. Both routes reach the original deferred generation-scoped on_ok, the sole Start continuation. No product KeyDown/KeyUp handler or vendor change is needed.

Diagnostics use original operation/session/generation IDs: accepted, inspecting/started, preview_ready/cancelling and terminal with elapsed/fixed code/bytes/cleanup/native error. C# owns native-code facts. No per-chunk events, paths/names/content/raw exceptions or secrets.

## 4. Validation & Error Matrix

| Condition | Result / effect |
| --- | --- |
| Wrong shape/version/identity | invalid_request or ignored stale UI reply; no new work |
| Active operation/full ordinary admission | busy; original operation retained |
| Stale preview/session/generation | stale_preview/stale_operation; no output |
| Gap / noncanonical or duplicate ordinal | missing_part / ambiguous_family; no output |
| Unsupported suffix such as .z01 | invalid_family; no output |
| Changed source/family | source_changed; no publication |
| Same numeric-family output | output_is_part before content reads or output creation |
| Existing target/missing parent/insufficient space | original native refusal; no replacement or parent creation |
| Cancel/write failure before Commit | cancel exact owned incomplete output; retain cleanup/native error |
| Cancel after Commit | succeeded and actual byte count retained |
| Disconnect | native cancellation/drain; UI output status unknown |

## 5. Good / Base / Bad Cases

Good: choose “合并分卷文件（.001）…” beside “新建压缩包”, select `.003`, review the numerically ordered observed `.001/.002/.003` names/count/output and the explicit inability to determine completeness, confirm, then stream to a new file while Ping/other commands remain responsive.
Base: cancel the confirmation dialog; no Start/output command exists.
Bad: infer completeness from a contiguous discovered suffix, concatenate `.z01`, replace an existing file, or release a worker/window on Cancel ACK.

## 6. Tests Required

- Protocol `ref18_strict_shapes_identity_and_metadata_bounds`: strict shapes, redacted Debug, native metadata, ordinal/generation rejection.
- IPC `ref18_original_router_reserves_cancel_and_waits_for_actual_terminal`: full queue, fair/coalesced Cancel, no long timeout, ACK preserves reservation, actual terminal releases, old origin rejects.
- Desktop `tools::raw_join::tests::ref18_`: actual Archive Browser PopupMenu entry and absent Other Tools tile, default menu semantics, inputs/buttons/Dialog/IME, first Enter and first Space after observed semantic Confirm focus, Escape/back/no-Start, repeat/no-duplicate, stale/deferred reply, picker/focus, minimum viewport/footer, caption close/cleanup/late success and real menu cache-hit/reopen.
- C# exact reviewed `--fixtures "raw numbered join"` LIST then identical RUN: later-part byte-exact readback; family/source/output refusals with zero preflight content IO; >buffer streaming/read-write maxima/one content pass; cancellation/write112/cleanup/handle reopen; original serial reader Ping/clipboard-parse response and physical drain.

Owned TestPlatform/native fixtures do not establish physical IME/DPI/provider, human visual, installed/Release or full-workspace acceptance. Never execute joined output.

## 7. Wrong vs Correct

Wrong: ordinary timed send → Cancel ACK → release operation/window → start replacement while old handles remain live.

Correct: origin-bound pending-before-send → reserved Cancel receipt → retain operation/window → actual native finally and original terminal → release; keep late success or cleanup failure visible.
