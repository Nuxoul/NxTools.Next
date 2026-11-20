# Window refresh and picker presentation

## Requested behavior

- Diagnose full-surface flicker on updates/page changes, with About caption-close as the clearest example. Establish a red-capable regression at the actual invalidation/render/native lifecycle boundary before a cause-specific repair; do not infer visual acceptance from imports or source tokens.
- About's existing separate dialog has only a Close caption action, no Minimize/Maximize. Preserve fixed geometry, open reuse, update cancellation, copy/licenses and original native close lifecycle.
- Starting blacklist window selection hides currently visible windows of the originating Desktop process, excluding the passive picker overlay. Selection/cancellation/failure/disconnect/timeout restores only the windows captured as visible and still owned, preserving placement/maximized/minimized state. Keep original source/view/session and deferred process-name refill, source activation/input focus and Add/Apply/Cancel authorities. No source UI remains in front during target selection.
- Remove only the two preset scope checkboxes beside the process input/selection button. Preserve rule-row scope controls and existing add/edit defaults and save transaction unless evidence requires a documented adjustment.

## Boundaries

Original MainWindow/About and GPUI invalidation owners; original NetworkSettingsApplication/View editor; original ProcessWindowPickController/Coordinator/Native input/style/cleanup owner. Rust owns domain/layout, GPUI rendering, C# Windows visibility/input facts. Reuse the existing command/session and native interface seam; no parallel picker, window manager, downloader, new config policy or vendor patch. Exact released TitleBar/Settings/Input/Button/Checkbox APIs and reference constructors/native boundaries are reviewed before code changes.

Preserve all pre-existing committed work and the untracked gpui-kit task directory. No commit, Release packaging, installation, real config/Registry/service mutation or live global-hook/product acceptance. Controlled fixture effects are reviewed before execution; serial Cargo uses workstation memory settings. Record source, compiled, native and user-visible evidence separately.
