# Authority, modules and runtime ownership

## Required architecture
Rust owns domain state, scheduling and logical layout. GPUI renders views and owns its entities/window handles; C# adapts Windows APIs through typed commands/results. Rust Indexer is a deliberate native/raw-volume boundary, not a reason to put window APIs in domain libraries. Browser extension owns browser capture/UI, not engine state. Evidence: `repo:Cargo.toml`, `repo:rust/apps/nxtools-desktop/src/runtime.rs#DesktopRuntime`, `repo:rust/crates/nxtools-protocol/src/lib.rs#PlatformCommand`, `repo:csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.cs`.

| Flow | Owning boundaries |
| --- | --- |
| Manual/browser/CLI download | Frontend validation -> typed DownloadCommand -> DownloadEngine/store -> HTTP/media writers -> durable artifact -> event/snapshot projection |
| Clipboard | C# native capture -> typed PlatformEvent -> serialized Rust worker/history -> generation-scoped GPUI page -> correlated native action result |
| Quick Find | GPUI query generation -> Finder catalog plus Indexer service -> matching batched results -> selection/preview -> typed Windows action |
| Configuration | Local view draft -> owning section validation -> lock/read/merge/sync/rename -> refresh cache -> native apply result; saved != applied |
| Native auxiliary surface | Rust target/generation -> typed pixel bounds/visibility/focus command -> C# readback/result or WM_SIZE -> GPUI actual viewport |

Reference notes: [download](../../../docs/reference-code/download.md), [clipboard](../../../docs/reference-code/clipboard.md), [Quick Launch](../../../docs/reference-code/quick-launch-research.md), [Shell](../../../docs/reference-code/system-shell.md), [GPUI](../../../docs/reference-code/gpui.md). For migrated interactions read note then recorded reference sources before proposing a change; trace constructors/base classes to show/hide and add a red-capable boundary regression. Never imply Listary native engine code was available.

## Module design and dependencies
- Keep public interfaces typed and narrow; callers should not reconstruct a hidden state machine or choose storage/native ordering. Look for existing owners before adding utilities.
- Group by real lifecycle/surface/controller/store, not arbitrary line limits. Existing large orchestration modules are observed debt; changes need a concrete behavior/testability benefit, not cosmetic splitting.
- Workspace dependencies and lint configuration live in `repo:Cargo.toml`; package manifests inherit. Add a dependency only for a demonstrated gap, check license/maintenance/platform behavior and lockfile impact, and require approved scope. User2026-10-21 explicitly upgrades the coherent Kit/Base/Component/Assets stack to exact Kit0.8.0/default gpui-pre0.3.9; earlier0.7.1pins are historical. Kit remains the desktop's only direct GPUI-stack dependency; no GPUI Fast. Preserve source-backed behavior through actual regressions. Current [release/API delta](../../../docs/reference-code/gpui-kit-080.md) precedes historical component inventory; narrow approved source exceptions only below.
- Extract shared constants only when they express one invariant; similar literals can be unrelated. Tests use production paths/observed results, not solely string-presence gates.
- Generated release binary manifests are build outputs with security significance. Do not hand-edit hashes or infer protected deployment approval from source compilation.

## State and lease boundaries
Evidence: `repo:rust/apps/nxtools-desktop/src/ui/mod.rs#schedule_entity_update`, `repo:rust/apps/nxtools-desktop/src/clipboard_worker.rs`, `repo:rust/apps/nxtools-desktop/src/download_window_lifecycle.rs`, `repo:rust/crates/nxtools-ipc/src/lib.rs#CommandRouter`.
- Entity callbacks change local state synchronously, then defer cross-entity effects with `cx.defer`/a later app turn when another owner can write back into the leased entity. Cache-hit/immediate-result paths obey the same rule.
- Capture IDs/generation/read-only bridge snapshots before async work; validate identity at application, not only dispatch. Weak handles prevent resurrecting retired entities.
- One passive clipboard preview; one generation-scoped Quick Launch preview controller; one keyed download task window. Close/hide is presentation-only unless an explicit domain action was accepted.
- Admission/command acceptance is not completion. Native API BOOL, process launch or stored settings are not proof of applied pixels, foreground or running configuration.

## Failure and operation boundaries
Bound queues, retries and workers in the owning module. Stop/timeout invalidates logical ownership but cannot pretend to interrupt an uncooperative Windows/COM call. Do not create unlimited replacement threads. Protected Desktop/PlatformHost/Indexer are an atomic release tuple; service/package changes follow AGENTS.md exact-path/hash and separate authorization. Pure documentation never needs a product/service launch.

Narrow authorized native exception (original2026-10-10, carried by explicit2026-10-21upgrade): `repo:vendor/gpui-pre-windows-039/NXTOOLS-PATCH.md` records published Apache-2.0 Windows0.3.9 with only WM_MOUSEACTIVATE interpreting C#-owned live WS_EX_NOACTIVATE. Primary activation/readback/nativeownership remain; no callback injection/focusstealing. Old0.3.8vendor is inactive historical evidence.

Narrow authorized component exception (original2026-10-20, carried by explicit2026-10-21upgrade): `repo:vendor/gpui-component-080/NXTOOLS-PATCH.md` retains published Apache-2.0 Component0.8.0 with only Notification::duration(Duration) forwarding into originalToastOptions. Official NotificationList/ToastManager sole clock/pause/stack/close owners; raw5seconds/product1-60default6 unchanged. Old0.7.1/0.5.1vendors inactive/unmodified. Perfileupstreamhashes and `repo:tests/architecture/gpui-release-contracts.py` restrict each newvendor to its one approved changed implementationfile. No other framework/registry edits authorized.
