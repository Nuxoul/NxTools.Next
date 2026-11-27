# Protocol quality

Evidence: `repo:rust/crates/nxtools-protocol/src/lib.rs#tests`, `repo:csharp/NxTools.PlatformHost/Protocol/ProtocolEnvelope.cs`.

Roundtrip real enum/DTO payloads, version rejection, exact frame-size boundary and serde default/unknown-field compatibility. For a changed native command verify C# parser and actual owning callback/result, not only encode equality. Bounds/size units (logical/device pixels, bytes/ms, HWND/PID) are explicit in field contract; no name-only authorization. Keep optional source HWND/native result metadata coherent. `cargo test -p nxtools-protocol --lib` and affected IPC/native tests follow [verification](../../shared/verification.md); no new warning suppression.

IndexSearchContext search extensions retain_kinds_apart/allow_homophone_fallback/recent_modified default false when absent and omit false serialization. Test legacy context and opt-in roundtrip. Caller explicitly permits approximation only after local ordinary sources are empty; recent mode is launcher-only opt-in, not inferred for file-dialog callers. SearchFailed carries generation/fixed reason and preserves request-envelope correlation at the runtime owner. Indexer/desktop own semantics; no C# native DTO changed. Evidence: `repo:rust/crates/nxtools-protocol/src/lib.rs#search_retention_tests`; executable cross-layer contracts in `.trellis/tasks/10-14-winshun-w2-w5/code-spec.md`.

ArchiveRead command/event definitions live in archive_read.rs and preserve outer protocol1.
Require explicit archive version1 or opt-in2, bounded read-only fields and symmetric defaults/rejects;
unknown capability grants no authority. Test real owner->pipe->C#session->Job->worker, not just
JSON equality; exact lifecycle/budget/error cases are in
[product read](../../nxtools-archive/backend/product-read.md).
Readv2 requires explicit limits; v1defaults4096/4MiB and omitted default transport field stay
compatible. Version precedes budget rejection. Batch bytes/count are independent, v2terminal
stats are optional/omitted for oldv1, required for v2success. Exact budget/negotiation matrix:
`repo:.trellis/tasks/10-16-archive-manager/product-100k-contract.md`.

Quick Launch keyboard health retains outer PlatformHost protocol version 1. The typed
`quick_launch_keyboard_health_changed` event carries one UUID in both its envelope and payload,
`install_failed | thread_exited | reinstalled`, and a native i32 error code. It reports only
observed install/owned-thread transitions; no heartbeat or silent-removal inference. Rust rejects
unknown event variants during envelope decode and closes that pipe session; PlatformHost must emit
the additive event only when HelloAck accepts `quick_launch_keyboard_health_v1`. The existing
`quick_launch_panel_hotkey` Hello availability remains authoritative on reconnect. Evidence:
`repo:rust/crates/nxtools-protocol/src/lib.rs#QuickLaunchKeyboardHealthChanged`,
`repo:rust/crates/nxtools-ipc/src/lib.rs#handle_connection`.

## Scenario: Optional per-volume index maintenance
### 1. Scope / Trigger
Indexer-to-Desktop per-volume initial-build/rebuild/ready feedback under outer protocol1.
### 2. Signatures
`IndexDriveStatus.maintenance: Option<IndexDriveMaintenance>` has serde default and skips None. Internally tagged snake_case states: InitialBuilding, Rebuilding { reason: IndexRebuildReason }, Ready. Reasons: JournalWrapped/BulkChanges/CursorInvalid/Requested, serialized journal_wrapped/bulk_changes/cursor_invalid/requested.
### 3. Contracts
Omitted legacy fields decode to None and remain omitted on serialization. Old DTOs ignore this added field; no new outer event variant or protocol bump. Maintenance grants no service/search authority and never substitutes for last_error. C# DTOs are unchanged.
### 4. Validation & Error Matrix
Missing field -> legacy projection; known state/reason -> exact typed roundtrip; malformed state/reason -> ordinary decode failure. Maintenance is not a ProtocolFault.
### 5. Good / Base / Bad Cases
Good: empty successfully indexed volume reports Ready. Base: old status without maintenance retains count/availability copy. Bad: infer Ready from nonzero count or place a recoverable rebuild reason in last_error.
### 6. Tests Required
`index_maintenance_tests::u4_drive_maintenance_roundtrip_and_legacy_default` asserts all states, omitted-field behavior and an old DTO ignoring the added field. Service and production Settings tests validate consumers.
### 7. Wrong vs Correct
Wrong: add a mandatory field or unnegotiated outer event. Correct: defaulted optional field on the existing drive DTO with unchanged outer protocol1.

## Request-specific Indexer cancellation

`repo:rust/crates/nxtools-protocol/src/lib.rs#IndexSearchCancel` adds the typed
`IndexerCommand::CancelSearch` payload with required request_id:UUID and generation:u64 under
the unchanged outer protocol1. request_id is the original Search envelope ID, not the cancellation
envelope's own ID; cancellation correlation carries that original ID. No query, path, result or
native payload is transmitted. `search_cancel_tests` asserts the exact cancel_search/data shape
and complete envelope roundtrip. Existing Search/context/default/frame/version contracts remain;
C# native DTOs and generic IPC routing are unchanged. Only the Indexer coordinator's service lease
plus exact request/generation match authorizes retirement; decoding alone is not cancellation.
See the owning [Indexer lifecycle contract](../../nxtools-indexer/backend/quality-guidelines.md).

Ordinary numbered-file joining follows [the cross-layer contract](../../platformhost/raw-numbered-join.md): strict action-specific fields and reply identities, bounded numeric preview, native-private source versions and unchanged outer protocol1. Actual owner/reader/stream/UI tests accompany the wire test.

Installed Windows applications use the parameterless `query_installed_applications` command and
typed `installed_applications::InstalledApplications` snapshot inside original command-result
data JSON string. Outer protocol1, envelope/request correlation and existing frame limits remain.
See [installed application contract](../../nxtools-finder/backend/quality-guidelines.md#scenario-installed-windows-applications)
for native ownership, snapshot replacement, target validation and required Rust/C# boundary tests.

Blacklist `process_window_pick` and `process_window_pick_changed` retain outer protocol1 and opt-in
session events. Action-specific fields, request/source/connection identity, device-pixel coordinates
and original editor publication follow [the window selection contract](../../nxtools-desktop/backend/quality-guidelines.md#scenario-blacklist-process-window-selection).
Roundtrip actual typed envelopes and check the C# parser/controller path, not DTO equality alone.

## Native window material facts and receipts

`window_material_v1` is explicitly negotiated under outer protocol1 before emitting the additive
`system_material_capabilities_changed` event. `window_material.rs` separates requested Solid/Acrylic,
backdrop/transparent host policy, session-qualified monotonic facts and effective setter/reset receipt.
`apply_window_material` carries the current session, facts revision, HWND and nonzero generation;
the native owner uses the actual pipe-peer PID rather than a caller-provided process claim.
Original command-result data remains a JSON string. Receipt acceptance matches every request identity,
style/surface/dark field plus coherent effective/fallback/native-error combinations. A successful
setter or opaque reset is not compositor-pixel acceptance. Unknown enum values/extra command fields
reject; Win10 does not require DWM38. Roundtrip/invalid-effect tests, original Hello negotiation,
actual C# parser/pipe route and Desktop first-show barrier tests protect the cross-layer contract.
Exact shapes and fallback order: [material contract](../../../tasks/11-21-desktop-motion-acrylic-style/material-contract.md).
