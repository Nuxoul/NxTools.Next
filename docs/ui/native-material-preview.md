# Native GPUI material preview

This preview opens actual GPUI/DirectComposition windows with native Windows Acrylic. It is independent of the browser theme study and does not start the NxTools business runtime, capture clipboard contents, query the indexer, persist settings or replace installed binaries.

## Run

From the repository root in Windows PowerShell:

```powershell
.\scripts\preview-native-material.ps1 -Surface Workbench -Material Acrylic
.\scripts\preview-native-material.ps1 -Surface Clipboard -Material Acrylic -NoBuild
.\scripts\preview-native-material.ps1 -Surface Search -Material Acrylic -NoBuild
.\scripts\preview-native-material.ps1 -Surface Workbench -Material Solid -Light -NoBuild
```

The first command runs the native boundary fixture and builds the isolated candidate. Later commands may use `-NoBuild`. Ordinary invocation owns the process-stream session until the preview exits. F2 changes the GPUI palette and synchronizes native dark policy through the same C# owner. The titlebar X closes the normal preview; the GPUI Escape handler is separately tested. There is no search execution or real clipboard access.

The candidate is `target/debug/preview-native-material/quick_launch_surface.exe`. It uses the existing private source-over/UnderBorder fixture and exact Kit0.8.0/pre0.3.9 graph. Production vendor sources, root Cargo.lock and the ordinary corner-preview executable are not overwritten.

## Native boundary

[MaterialSurface](../../rust/apps/nxtools-desktop/examples/quick_launch_surface/material.rs) owns local state, actual released controls, rendering and stable InputState. [The C# helper](../../scripts/preview-native-material-window.cs) owns native operations and receives only its newly launched PID/exact preview title.

- Acrylic uses the released Windows backend's WCA Accent policy: state4, flags0, gradient0x01000000, animation0. Solid clears Accent with state0. No whole-window opacity is used.
- DWM system-backdrop state is explicitly None in both modes, leaving one blur mechanism.
- Fixed preview clients remove the unused native caption/window-edge while retaining SysMenu and other style bits. The real client screen origin/dimensions are preserved with one normal frame refresh, without activation or Z-order changes.
- Visible window bounds must match the client within1px. A resizable THICKFRAME target is rejected before native style writes; this is not a production resizable-window migration.
- C# session stdout parsing accepts only the same correlation ID and fixed dark booleans. Native failures are retained; normal exit drains readers and disposes the session.

The local proof requires Windows11 build22621+ and x64. Current visual evidence is build22631 at100% DPI. Actual production fallback policy and other Windows/DPI configurations remain separate work.

## Reproduce desktop-pixel checks

The owned static background and capture tools are checked in:

```powershell
$background = Join-Path $env:PI_SCRATCH_DIR 'native-material-backdrop.exe'
Add-Type -Path .\tests\graphics\native-material-backdrop.cs `
  -ReferencedAssemblies System.Windows.Forms,System.Drawing `
  -OutputAssembly $background -OutputType WindowsApplication
Start-Process $background
# Launch the Workbench preview above; keep both fixtures at their initial positions.
.\tests\graphics\native-material-frame.ps1 -PreviewProcessId <reported-pid> -AssertClientOnly
.\tests\graphics\capture-native-material.ps1 -WindowHandle <reported-hwnd> `
  -ExpectedProcessId <reported-pid> -Destination $env:PI_SCRATCH_DIR\native-workbench.png
python .\tests\graphics\check-native-material-blur.py $env:PI_SCRATCH_DIR\native-workbench.png
```

The capture refuses another PID/title or a nonforeground target. It records the real desktop composition, not `render_to_image`, so native backdrop pixels are included. The pixel check is intentionally tied to the client-only900x600 workbench over this known100%-DPI static background. It requires fine stripes to be blurred and broad background color to remain visible; neither ordinary transparency nor an opaque fill qualifies.

## Evidence and limitations

[Native verification](../../.trellis/tasks/11-21-desktop-motion-acrylic-style/native-verification.md) records actual frame/pixel/input and close results. Independent browser verification does not certify these native results.

The preview is a representative rendering fixture, not the complete product UI or a replacement settings system. The isolated30-run resource comparison is recorded in [performance results](../../.trellis/tasks/11-21-desktop-motion-acrylic-style/performance-results.md); [performance tools](native-material-performance.md) preserve raw CPU/memory/DWM/GPU/frame evidence. Production resizable windows, persisted window-bounds behavior, passive auxiliary windows, mixed DPI, Windows10, physical IME, full-product/Release performance and power acceptance remain outstanding. Do not lift the fixed-frame helper unchanged into the product: the framework's saved-window-bounds border cache is outside this proof's lifecycle.
