# Quick Launch GPUI surface preview

Run from the repository root:

```powershell
.\scripts\preview-quick-launch.ps1
```

After the first build, reopen without Cargo:

```powershell
.\scripts\preview-quick-launch.ps1 -NoBuild
```

- Type/select text in the real official Kit InputState. No search is executed.
- **F2** switches the existing Catppuccin Latte/Frappe theme pair without replacing the input.
- **Escape** closes the window and exits the preview process.
- `-Light` starts with the light theme; it can be combined with `-NoBuild`.
- `-BuildOnly` compiles without opening a window.
- `-TestOnly` runs the adjacent real-component layout/input/close regressions without opening a native window.

The runner applies the preview's C# native-frame policy as well as starting the executable; use it for the intended desktop appearance. It requires no package, install or administrator elevation.

## Surface

The official0.8.0 InputGroup owns one620×60DIP frame with8DIP corners and20/28DIP Sarasa UI SC Regular type. Its public background slot uses the current opaque popover token. The official Search addon shares the input's focus boundary. One public Base Root is transparent; the652×92DIP native popup reserves16DIP around the visible bar for its soft shadow. The gutter is specific to the preview, not a change to production window placement.

The C# helper targets only the newly launched PID and exact preview title. It applies the same DWM non-client-rendering-disabled policy as PlatformHost's existing SetWindowShadow(false), then reads back the enabled state. It does not apply a native region or change GPUI's input/composition/bounds owner.

Source: [example](../../rust/apps/nxtools-desktop/examples/quick_launch_surface.rs), [tests](../../rust/apps/nxtools-desktop/examples/quick_launch_surface/tests.rs), [runner](../../scripts/preview-quick-launch.ps1), [native helper](../../scripts/preview-quick-launch-window.cs).

## Verification

The two real-component tests pass, together with the example build and strict example Clippy. They check actual frame geometry/radii, opaque themed fill, control containment, transparent outer fill, stable input/focus/text through theme changes and Escape retirement.

Actual Windows11/100% white-backdrop captures: [dark](../../.trellis/tasks/11-21-launcher-surface-clipboard-flicker/preview-dark-white.png), [light with retained input](../../.trellis/tasks/11-21-launcher-surface-clipboard-flicker/preview-light-white.png). Native input and F2 theme switching were observed, and DWM non-client rendering was read back disabled.

## Upstream source-over comparison

The comparison uses Zed's merged [PR65571](https://github.com/zed-industries/zed/pull/65571), commit[e18de65b](https://github.com/zed-industries/zed/commit/e18de65b75901f8720ff9c9320bb013395de54e5). The fix changes two Windows destination-alpha blend factors from ONE to INV_SRC_ALPHA, correcting accumulated alpha for transparent windows. It changes no shader, radius, font, palette, RGB factor or draw-pass count.

Build and open the isolated candidate:

```powershell
.\scripts\preview-quick-launch-source-over.ps1
```

Reopen the already built candidate:

```powershell
.\scripts\preview-quick-launch-source-over.ps1 -NoBuild
```

Close the current preview before switching versions. F2/Escape and Light/BuildOnly/TestOnly have the same meaning. The source-over TestOnly command also runs a real Direct3D blend-state GetDesc regression for both affected pipelines; the candidate passes that test and the two original component tests.

The comparison runner copies the current Windows backend to a temporary directory and applies exactly the upstream's two alpha assignments through a command-local Cargo override. The ordinary example executable is preserved; the candidate lives separately under target/debug/preview-source-over. The checked-in vendor, Cargo manifest and lock remain unchanged. This is a preview experiment, not a product backport.

Actual Windows11/100% [candidate on white](../../.trellis/tasks/11-21-launcher-surface-clipboard-flicker/source-over-dark-white.png) and [nearest-neighbor corner comparison](../../.trellis/tasks/11-21-launcher-surface-clipboard-flicker/source-over-corner-comparison.png). The candidate lightens the dark edge fringe and retains the same silhouette. [Research and verification boundary](../../.trellis/tasks/11-21-launcher-surface-clipboard-flicker/source-over-research.md).

## Direction-aware pixel coverage

The pixel-coverage candidate retains the upstream alpha correction and adapts the rounded SDF's edge transition to its screen-pixel direction. Straight edges retain1px coverage; diagonal arcs use the analytic L1 gradient footprint, up to sqrt(2)px. Inner/outer border coverage is consistent. Radius, dimensions, fonts, palette and shadow are unchanged; no blur layer or extra rendering pass is added.

```powershell
.\scripts\preview-quick-launch-source-over.ps1 -PixelCoverage
# Reopen the already built candidate:
.\scripts\preview-quick-launch-source-over.ps1 -PixelCoverage -NoBuild
```

The candidate is separate under target/debug/preview-pixel-coverage. The original and source-over previews remain available. [GPU full-frame comparison](../../.trellis/tasks/11-21-launcher-surface-clipboard-flicker/pixel-coverage-comparison.png), [corner detail](../../.trellis/tasks/11-21-launcher-surface-clipboard-flicker/pixel-coverage-corner-comparison.png), [method and scope](../../.trellis/tasks/11-21-launcher-surface-clipboard-flicker/pixel-coverage.md).

### Reproduce the real GPU pixel regression

Window::render_to_image reads the actual Windows renderer's last scene. The capture mode draws on a deferred app turn, exports a premultiplied frame and an on-white frame, then exits. It does not need desktop focus or a screenshot backdrop. Example commands after both candidates have been built:

```powershell
$captures = Join-Path $env:TEMP 'nxtools-gpu-captures'
.\scripts\preview-quick-launch-source-over.ps1 -NoBuild -CaptureDirectory "$captures\baseline"
.\scripts\preview-quick-launch-source-over.ps1 -PixelCoverage -NoBuild -CaptureDirectory "$captures\candidate"
python .\tests\graphics\check-gpui-pixel-coverage.py "$captures\baseline" "$captures\candidate"
```

Add -Light to both capture commands for the light-theme comparison. The pixel fixture requires100%DPI. It bounds four arc samples to reject both clipped coverage and opaque squared-off corners; viewport, straight edge and interior samples must remain byte-exact. Both themes pass. The same native blend-state and two component/input/close regressions, Debug build and strict example Clippy pass. An export-error fixture confirms failure cannot reuse an earlier success.

## Background compensation candidate

The UnderBorder experiment keeps source-over and the original fixed SDF ramp. Its temporary core copy passes actual solid-border data to the background draw; the temporary Windows shader precompensates that background coverage before the existing foreground border. Child order and border-only strip masks remain. Fully covered opaque borders use a defined zero-background branch, and the outer coverage is not multiplied twice.

```powershell
.\scripts\preview-quick-launch-source-over.ps1 -UnderBorder
# Reopen the built candidate:
.\scripts\preview-quick-launch-source-over.ps1 -UnderBorder -NoBuild
```

The same example is built through a temporary standalone manifest with pinned Kit0.8.0/pre0.3.9 and temporary core/Windows overrides. Root Cargo files and checked-in vendor remain unchanged. The separate executable is under target/debug/preview-under-border. Native blend-state1/1, component/input/close2/2, Debug build and strict candidate Clippy pass.

### Single-coverage GPU oracle

After source-over and UnderBorder candidates have been built:

```powershell
$captures = Join-Path $env:TEMP 'nxtools-under-border-captures'
.\scripts\preview-quick-launch-source-over.ps1 -NoBuild -NoShadow -CaptureDirectory "$captures\baseline"
.\scripts\preview-quick-launch-source-over.ps1 -UnderBorder -NoBuild -NoShadow -CaptureDirectory "$captures\candidate"
python .\tests\graphics\check-gpui-under-border.py "$captures\baseline" "$captures\candidate"
```

NoShadow applies only to diagnostic capture; normal preview retains the original shadow. Add -Light to both commands for Light. At100%DPI the oracle compares all256corner pixels against one opaque fill/border coverage application, plus unchanged non-arc samples and viewport. Maximum RGBA channel error falls from61.452/255in the baseline to0.784Dark/0.641Light. This validates the current opaque1px/r8InputGroup, not every transparency/child/platform combination.

[Shadow-enabled full comparison](../../.trellis/tasks/11-21-launcher-surface-clipboard-flicker/under-border-comparison.png), [corner detail](../../.trellis/tasks/11-21-launcher-surface-clipboard-flicker/under-border-corner-comparison.png), [method and scope](../../.trellis/tasks/11-21-launcher-surface-clipboard-flicker/under-border.md).

## Known limits

All candidates still render on a finite pixel grid. The compensation result is verified for the current opaque solid InputGroup; translucent fills, group opacity, dashed borders, children covering the edge and other backends are not certified. Final dark/light smoothness remains subject to user visual acceptance. All framework changes remain in temporary experimental copies.

This example does not launch normal NxTools, connect to PlatformHost/Indexer, register hotkeys/clipboard capture, open configuration/history/download stores, install or replace packaged executables. It does not certify production first-show lifecycle, physical Windows IME, mixed DPI, screen readers or Win10 rendering. A Windows11 rendering result is not Win10 acceptance.
