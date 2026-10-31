# Protocol structure

Evidence: `repo:rust/crates/nxtools-protocol/src/lib.rs#Envelope`, `repo:rust/crates/nxtools-protocol/src/lib.rs#Message`, `repo:rust/crates/nxtools-protocol/Cargo.toml`.

lib.rs defines Envelope, tagged snake_case message/command/event enums, native DTOs and encode/decode validation. Keep transport scheduling, Windows APIs and live feature state out. One typed field has one canonical shape across Rust and C# ProtocolEnvelope/parser; don't cast raw data independently in multiple consumers. Optional/default additions require compatibility tests; breaking field behavior requires approved version contract. [Shared architecture](../../shared/architecture.md).

P208 formal `ArchiveVirtualDrag` and legacy `ArchiveVirtualDragSpike` serialize distinct snake_case
command names but share the exact version1 bounded payload/validator and original C# session.
Outer receipt ID differs from operation correlation; cancel retains original operation/session/
generation with hwnd0/itemsNone, no new materialization authority. See
[production contract](../../../tasks/10-16-archive-manager/phase-f-p208-production-contract.md).
Directory/unknownsize amendment: nullable item.size (missing/null unknown, notzero), defaultfalse
directory. All descriptors retain source entry; only descriptor directories may omit ordinal, never
forwarded to file materialization. Compact serialization shares frozen source context once and
preserves per-item original ordinals; full and compact formal/alias forms validate against the
actual1MiB related-envelope frame. Prefix/casing/name/source consistency remains mandatory.
P208 password extension uses optional opaque authorization_id (notsecret) on formal drag, omitted
for plain files/cancel. Entry authorize/revoke retain original source/session/connection ownership;
public descriptors never include password or request passwordinteraction. Private inherited worker
browser_authorize TEST and browser_password bytes are narrowly mutually exclusive; no ArchiveRead
secret authority or P209 store. Original PrivatePassword remains redacted/drop-cleared.

Read-only `ReadSystemProxy` is an additive no-payload command. `system_proxy.rs` contains only typed bounded snapshot/endpoints (<=8, host<=253, nonzeroport, fixedprotocol), no request-route policy, Windows API or persistence. C# SystemProxySnapshotReader uses WinHttpGetIEProxyConfigForCurrentUser and frees each native allocation; auto-detect/PAC are booleans, no PAC URL/credentials/bypasslist is returned or logged. Old hosts reject unsupported command; UI keeps System route and independent manual draft with actionable read failure.

`main_window.rs` defines additive `PrepareMainWindowCommand` (logical DIP and authenticated sourcePID/HWND) and actualdeviceclient+DPIreadback. DTO contains no persistedwindowpreference or Windows API. Exactwidth/height scaling/readbackmatches rejects capturedoversized1904x1021at96DPI; fixedDPI48..960 and explicitdimensionranges. Existingouterprotocol1/1MiB unchanged; nativeparser/peer tests and Rustroundtrip cover bothsides. Oldhostunsupported failsclosed duringhiddenstartup, neverassumedready.

Extension settings path selection is additive PickExtensionSettingPath with explicit sourceHWND/sourcePID, file|folder kind and bounded currentPath. Rust/C# parser/pipe peer/native picker/current draft form one correlated path; it never changes trust or saves settings. Evidence: `repo:rust/crates/nxtools-protocol/src/lib.rs#PickExtensionSettingPathCommand`, `repo:csharp/NxTools.PlatformHost/Protocol/PlatformCommandParser.cs#ParsePickExtensionSettingPath`, `repo:csharp/NxTools.PlatformHost/Windows/ExtensionSettingPathPicker.cs`.

## Scenario: Complete virtual directory wire and content lifetime
### 1. Scope / Trigger
ArchiveBrowser freezes a complete directory or multi-root selection for native virtual-file drag.
Evidence: `repo:rust/crates/nxtools-protocol/src/archive_virtual_drag.rs`,
`repo:csharp/NxTools.PlatformHost/Protocol/ArchiveVirtualDragContract.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualFileDataObject.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualContent.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs`.
### 2. Signatures
`ArchiveVirtualDragCommand` keeps the in-memory item/entry shape and custom command serialization.
The command carries shared path/codepage/browser/depth/MOTW once; items retain operation/ordinal/
descriptor facts. C# restores original entry commands before native admission. Formal and legacy
names retain the original session; cancel grants no content authority.
### 3. Contracts
No separate32-root/128-descriptor/32-file/256-request/512MiB drag gates. Preserve every selected
ordinal or reject the whole selection with its actual reason; never truncate or substitute zero.
Full and compact forms accept the same frozen source and reject member overrides. Validate the
real related-envelope1MiB frame, native relative names, checked HGLOBAL and signed64-bit stream size.
Known size uses full high/low bits; unknown omits FD_FILESIZE and progress totals stay unknown.
Materialize reuses general32GiB-per-member/100GiB-output allowances and charges actual bytes;
preview8MiB/text64KiB and active-worker2 remain independent. Logical files/live leases/repeated
GetData calls are not active workers. Serialize first materialization, retain exact per-lease
identity/integrity, and keep transferred streams readable until their own final release.
Original HWND/held-button/OLE STA/async completion/cancel/disconnect/cleanup owners and correlated
low-noise lifecycle diagnostics remain; no paths, content or credentials in logs.
### 4. Validation & Error Matrix
| Condition | Result |
| --- | --- |
| Ordinary256-file directory /64 selected roots | Complete descriptors and original ordinals |
| Shared escaped archive comment within actual frame | Accepted once, not repeated per member |
| Frame overflow / source override / unrepresentable native path | Whole rejection, no OLE start |
| Unknown or overflowing aggregate estimate | Unknown total; actual output remains accountable |
| Repeated or34th distinct member demand | Original content flow, not logical-count rejection |
| Cancel/retire with transferred stream | Stop new work; held stream survives until final release |
### 5. Good/Base/Bad Cases
Good:256 descriptors are addressable and a257th legal repeated request remains readable.
Base:34 distinct real ZIP members cross the old32-lease boundary with independent ownership.
Bad: replace128 with another small cap, split/drop selected roots, or count active workers as files.
### 6. Tests Required
Rust protocol/IPC wire, connection and reserved-cancel regressions; Desktop complete-freeze and
held-pointer dispatch. `repo:csharp/NxTools.PlatformHost.Tests/ArchiveVirtualTreeTests.cs` covers
actual shared frames, native256-stream descriptors,8GiB+17 metadata,34 distinct real-worker reads,
repeat and held-stream retirement; cache/async/password fixtures retain original lifecycle checks.
Metadata size coverage is not GiB payload coverage. Controlled native fixtures do not accept
physical Explorer/foreign targets, installed products, mixed DPI or user visuals.
### 7. Wrong vs Correct
Wrong: a directory has more items than the worker/cache concurrency so refuse it. Correct: complete
immutable descriptor tree -> actual frame/native representation -> on-demand original worker ->
independent lease/stream -> actual cleanup.
