# Desktop material reference boundary

## Scope

The approved Acrylic theme covers the entire application; palette/font selection is independent. Current implementation evidence is an isolated native rendering proof, not production theme adoption. The complete application retains its existing state, native placement, focus, input, workers and settings owners.

## Verified call chain

- `repo:rust/apps/nxtools-desktop/examples/quick_launch_surface.rs#main` selects the explicit native-material fixture before the ordinary example path; no DesktopRuntime is constructed.
- `repo:rust/apps/nxtools-desktop/examples/quick_launch_surface/material.rs#MaterialSurface` uses released TitleBar, Sidebar, GroupBox, ListItem, Button and InputGroup/Input. Solid and Acrylic share logical geometry/font/palette; Acrylic tint alpha is0.78. Base Root transparency and global palette updates are deferred.
- `repo:scripts/preview-native-material.ps1` builds the separately named candidate and starts one owning C# process-stream session.
- `repo:scripts/preview-native-material-window.cs#NativeMaterialPreviewWindow` validates PID/title, normalizes only fixed client frames, applies native policy and records scalar results.
- `repo:vendor/gpui-pre-windows-039/src/window.rs#set_window_composition_attribute` is the pinned framework reference for WCA19 Accent state4/flags0/gradient0x01000000. This proof adapts the native policy in C# without changing production vendor sources.
- `repo:scripts/preview-native-material-window.cs#NativeMaterialPreviewSession` accepts correlation-qualified deferred theme requests, applies native dark policy, retains errors, drains process readers and disposes on exit.

## Window-frame invariant

GPUI's Windows custom-titlebar boundary in `repo:vendor/gpui-pre-windows-039/src/events.rs#handle_calc_client_size` restores the top client edge after DefWindowProc. Caption/window-edge styles can still reserve side/bottom non-client pixels. Native Acrylic must not expose a second glass collar outside the GPUI surface.

For the fixed proof, client size/screen origin are preserved, Caption and WindowEdge alone are removed, SysMenu/other bits are retained, and FrameChanged uses NoActivate/NoZOrder. The helper checks the visible/client alignment and rejects THICKFRAME rather than removing resize behavior. No fake DPI/settings notification, root crop, opaque cover or second overlay is used.

The framework's live logical size/origin follow actual WM_SIZE/WM_MOVE. Its saved `window_bounds` uses a border-offset cache refreshed at construction/DPI/settings; this fixed fixture does not persist bounds. Production placement/resize integration must use its real native owner and must not assume the fixed helper certifies that lifecycle.

## Acceptance evidence

- Actual client/visible frame gate: `repo:tests/graphics/native-material-frame.ps1`.
- Real desktop composition: `repo:tests/graphics/capture-native-material.ps1`, using only the matching foreground preview HWND.
- Controlled native background: `repo:tests/graphics/native-material-backdrop.cs`.
- Blur/transmission gate: `repo:tests/graphics/check-native-material-blur.py`.
- Native setter acceptance is not visual acceptance. WCA has no effect getter; compositor pixels are mandatory evidence.
- GPU-only capture does not include the native desktop backdrop. Root/button geometry tests, browser CSS and successful property readback do not establish native Acrylic.

Run instructions: [native preview](../ui/native-material-preview.md). Exact local evidence and limits: [native verification](../../.trellis/tasks/11-21-desktop-motion-acrylic-style/native-verification.md).

## Isolated resource workloads

- `repo:rust/apps/nxtools-desktop/examples/quick_launch_surface/material.rs#Options` accepts explicit benchmark scenarios; ordinary previews keep their original path. One process creates either one Workbench or the three fixed, non-overlapping surfaces.
- `repo:rust/apps/nxtools-desktop/examples/quick_launch_surface/benchmark.rs#MaterialSurface` owns finite phase tasks, stable InputState and the actual1000-row uniform_list. Input writes occur outside the leased view; cancellation/generation retirement stops late work. Idle has no workload timer. Actual rendered-row counts establish scrolling.
- `repo:scripts/preview-native-material-window.cs#NativeMaterialPreviewWindow` adds exact-PID/title enumeration and an explicit-HWND Prepare overload; both overloads share the same native material/frame owner.
- `repo:tests/performance/NativeMaterialBenchmarkDriver.cs` owns test-only PDH/process observation and the synthetic STA background. `native-material-benchmark.ps1` runs paired processes sequentially; `native-material-frame-capture.ps1` is the explicitly authorized temporary elevated observer for candidate/same-session DWM.
- `repo:tests/performance/analyze-native-material.py` uses actual QPC intervals, explicit missing-value status, within-repetition deltas and separate swapchains. Frame wall time is not scheduled CPU time; target timer cadence is not achieved cadence.

The completed30-run Debug comparison and portable raw evidence are in [performance results](../../.trellis/tasks/11-21-desktop-motion-acrylic-style/performance-results.md). [Run guide](../ui/native-material-performance.md). Actual input≈16Hz/scroll≈34–35Hz does not certify60Hz or full-product performance; the whole-app theme scope is unchanged.
