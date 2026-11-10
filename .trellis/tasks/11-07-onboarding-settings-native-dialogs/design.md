# Design

## Scope and ownership

This task changes only the reported onboarding/settings/Shell/uninstall/picker boundaries. Rust retains domain state, settings drafts, operation generations and logical layout. GPUI retains the current rendered windows/entities and released styled components. C# retains Windows modal/STA/window/Shell operations. The indexer remains the existing Rust service and persistence architecture.

Source evidence and external reference limits are recorded in [research](research.md). Implementation and isolated verification are approved; native and release acceptance retain the operational boundaries below.

## Native picker lifecycle

### Behavior gap

Most original picker commands cannot identify their initiating window; several native dialogs have no owner. Human selection is included in the IPC router's ten-second deadline, while the archive picker additionally has a two-minute native wait. Timeout removes Rust ownership without necessarily closing the native dialog.

### Smallest owning change

1. Carry the original source HWND through each existing typed picker command, Rust call site, C# parser and dispatcher. Use the existing authenticated pipe-peer identity to validate that owner; do not substitute a worker-time foreground HWND.
2. Reuse the existing detached command route and bounded STA runner. Human-modal operations use no elapsed deadline in the router, native runner or feature-level timers. Retain admission through real modal return and cleanup. Keep machine-only commands' existing bounded policies.
3. Present each native dialog with the validated original owner. While it is open, suspend only the relevant application-owned Topmost presentation states that would obscure it, using the original desktop/Windows presentation owner. Restore the same live windows' recorded states once. Do not globally pin the dialog above unrelated applications or restore stale/replacement windows.
4. Separate browser export destination selection from actual package generation. A typed destination-selection request returns a path/cancel result without a human deadline; the existing ExportBrowserExtension command receives the chosen nonempty destination and retains bounded machine work. Preserve one original pending export operation and correlation across both steps; no second export controller.
5. Owner close, disconnect and shutdown explicitly retire/cancel the matching picker. Close only that operation's native dialog and keep its slot until actual cleanup. Normal user Cancel is not a failure toast. Late results cannot populate a new view or dispatch an export after retirement.
6. Retain the existing weak-view/window/operation/edit-revision validation and deferred cross-entity callbacks. Do not change clipboard capture, FFmpeg execution/trust, archive worker limits or unrelated launch/navigation timeouts.

### Coverage inventory

Download directory and FFmpeg/program selection; archive open/source files/source folders/destination/save/temp directory; clipboard archive import/export and storage directory; extension manifest/runtime/settings file/folder selection; browser extension export destination. Archive source-folder multiselect and existing result-vector limits remain unchanged.

### Diagnostics

One correlation ID across Rust/IPC/C#: accepted, native presentation started, selected/cancelled/retired/disconnected, machine export started and terminal success/failure. Include elapsed/native codes and bounded counts only. Do not log chosen paths, filenames, clipboard contents or package secrets. Do not add periodic wait logs.

## Settings and effective paths

### Layout and component choices

- Appearance retains two natural-height Light/Dark preview columns plus Follow System. Keep released Kit 0.8.0 Radio/RadioGroup gesture and keyboard owners. Remove the fixed 244-DIP inner viewport; SettingPage remains the one vertical scroll owner.
- Display Latte and Frappe once each, labeled as their default modes. Existing Light/CatppuccinLatte and Dark/CatppuccinFrappe saved values remain readable and render the same selected preview. Do not rewrite a saved theme on opening or cancelling settings. Other palettes and Follow System remain selectable.
- Replace the category master list's embedded scrolling with an official category selection field above the existing selected-category Form/Field. Let the form grow naturally inside SettingPage; remove the fixed 310-DIP wrapper and form scrollbar. The trade-off is no permanently expanded category master list, in exchange for one straightforward page scroll owner. Keep category ID, ordering, enabled state, rule fields and original draft/Apply/Cancel owner.
- Use actual released SettingField dropdown/Select APIs, default 32-DIP fields, and Small outlined action Buttons. Do not put outlined actions into Toolbar.child, which forces ghost/compact. No dependency/vendor patch or hand-painted controls.

### Effective temporary directory

Show a read-only, selectable effective absolute directory with default/custom status next to the current base-directory input. Derive the default from the existing application paths/native cache policy and prove equality with C# production resolution. Custom base B resolves to B/NxTools.Next/archive/sessions; default resolves to LocalApplicationData/NxTools/NxTools.Next/cache/archive/sessions. Keep blank stored base as default, and do not materialize an absolute default into config. Reuse existing open/copy actions if exposed; never move, delete or sweep existing cache contents as a display side effect.

## Shell integration and uninstall

Include Create in fresh recommended command defaults and label its ordinary-file/folder purpose explicitly. Existing ArchiveSelectionHandlerPlan already routes selected create to * and Directory through --archive-compress. Preserve explicit Apply/Revert, consciously saved custom command choices, grouped selection and exact-owned process/service guards; do not add per-file loops or a new archiver backend.

Uninstall cleanup concerns application-owned history, settings, caches and runtime state, not download outputs or arbitrary custom parents. Standard uninstall removes personal application data; an explicit retain choice preserves it. Upgrade/repair always preserves data. Stop the exact-owned desktop/background/helpers, revert owned Shell/native-messaging values and remove the installation-owned service registrations/protected deployment through the privileged owner. Respect existing update/deployment gates and failure recovery; the upgrade preparation transaction is not an uninstall mechanism.

Runtime cleanup removes exact config/database/sidecar/lock/log owners and payloads proven by existing clipboard rows, extension catalog, archive metadata or managed-tool configuration. Custom parents and unattributed payloads are retained, including files under built-in runtime subtrees whose download history and directory configuration were removed. Some runtime data may therefore remain after uninstall. No new download ledger or migration owner is introduced. Exact deletion/preservation boundaries are recorded in [uninstall outcomes](uninstall-outcomes.md).

## Indexer first-build latency

Retain MFT/USN correctness, last-good search availability and versioned mmap-base/persisted-delta restart. Measure the actual initial path by scan method and volume, record scan/record admission/snapshot construction/persistence/readiness stages, and distinguish first-search readiness from completion of all volumes. Do not replace native MFT scanning with recursive filesystem traversal or declare completion early.

The confirmed expensive path is initial enumeration followed by one native open/read/close per record; all volumes are currently aggregated before publication. Establish stage attribution, then replace this initial time-hydration path with sequential raw-MFT metadata extraction while preserving FILE identity, true LastWriteTime, journal cursor and replay. Persist/exactly reopen each volume before publishing it; final Ready still waits for all required volumes. Reuse the cache format and retain incremental time metadata handling. Do not introduce multi-volume concurrency in the same slice. Existing standalone PowerShell performance probes must retain an authenticated approved Desktop before their results can serve as current service acceptance. Neither source inspection nor this design promises Listary-native equivalence or seconds-level timing.

## Compatibility and operations

- The reported fixes do not authorize release packaging, product launch, live registry changes, real uninstall, full-volume scanning, service restart or deletion of current user data.
- Compile/fixtures are not physical keyboard/focus/DPI/Explorer or LocalSystem acceptance. Run the separately authorized live gates after source/behavioral regressions.
- Preserve all unrelated existing worktree changes. Revert only task-owned changes if necessary; no reset/clean or implicit commit.
- Existing installation trust, protected atomic release tuple, service ownership and signing material policies remain unchanged. No new hashes, compatibility framework or parallel runtime is introduced.
