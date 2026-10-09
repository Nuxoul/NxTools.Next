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
- Workspace dependencies and lint configuration live in `repo:Cargo.toml`; package manifests inherit. Add a dependency only for a demonstrated gap, check license/maintenance/platform behavior and lockfile impact, and require approved scope. Vendored gpui-component is patched; do not silently treat it as upstream API or rewrite it during surface work.
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
