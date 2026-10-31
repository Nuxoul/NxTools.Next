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
forwarded to file materialization. Bounds128descriptors/32files/256KiBcommand; prefix/casing/name/source
consistency fail closed. Rust receiver-escaped conservative size bound precedes native admission.
P208 password extension uses optional opaque authorization_id (notsecret) on formal drag, omitted
for plain files/cancel. Entry authorize/revoke retain original source/session/connection ownership;
public descriptors never include password or request passwordinteraction. Private inherited worker
browser_authorize TEST and browser_password bytes are narrowly mutually exclusive; no ArchiveRead
secret authority or P209 store. Original PrivatePassword remains redacted/drop-cleared.

Read-only `ReadSystemProxy` is an additive no-payload command. `system_proxy.rs` contains only typed bounded snapshot/endpoints (<=8, host<=253, nonzeroport, fixedprotocol), no request-route policy, Windows API or persistence. C# SystemProxySnapshotReader uses WinHttpGetIEProxyConfigForCurrentUser and frees each native allocation; auto-detect/PAC are booleans, no PAC URL/credentials/bypasslist is returned or logged. Old hosts reject unsupported command; UI keeps System route and independent manual draft with actionable read failure.

`main_window.rs` defines additive `PrepareMainWindowCommand` (logical DIP and authenticated sourcePID/HWND) and actualdeviceclient+DPIreadback. DTO contains no persistedwindowpreference or Windows API. Exactwidth/height scaling/readbackmatches rejects capturedoversized1904x1021at96DPI; fixedDPI48..960 and explicitdimensionranges. Existingouterprotocol1/1MiB unchanged; nativeparser/peer tests and Rustroundtrip cover bothsides. Oldhostunsupported failsclosed duringhiddenstartup, neverassumedready.

Extension settings path selection is additive PickExtensionSettingPath with explicit sourceHWND/sourcePID, file|folder kind and bounded currentPath. Rust/C# parser/pipe peer/native picker/current draft form one correlated path; it never changes trust or saves settings. Evidence: `repo:rust/crates/nxtools-protocol/src/lib.rs#PickExtensionSettingPathCommand`, `repo:csharp/NxTools.PlatformHost/Protocol/PlatformCommandParser.cs#ParsePickExtensionSettingPath`, `repo:csharp/NxTools.PlatformHost/Windows/ExtensionSettingPathPicker.cs`.
