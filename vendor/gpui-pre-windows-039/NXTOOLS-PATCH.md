# NxTools Windows patch on0.3.9

Published gpui-pre-windows0.3.9, archiveSHA256 `513d4f0fc7d596d3bdef2a73a524ffb83dabc95bf484f2855a698e7f431290a9`, Apache-2.0 LICENSE-APACHE retained. `NXTOOLS-UPSTREAM.json` records every published file hash. Matched core remains gpui-pre0.3.9, Kit/Base/Component/Assets0.8.0, no Fast. Old0.3.8vendor remains inactive historical evidence.

User-approved2026-11-21 material integration permits exactly these production files:

- `src/directx_renderer.rs`: ordinary and path-sprite source-over use destination alpha `INV_SRC_ALPHA`; external project regression modules are test-only.
- `src/shaders.hlsl`: UnderBorder2 compensates background coverage for the later solid border. Foreground, child order, masks and SDF footprint are unchanged.
- `src/window.rs`: actual left/top/right/bottom nonclient margins use ClientToScreen, with inverse client/window conversions. The cache represents restored bounds and does not absorb maximized/minimized/fullscreen geometry. Target-display scale remains the placement owner.
- `src/events.rs`: the real WINDOWPOSCHANGED boundary refreshes restored margins before default move/size callbacks. Custom normal/fullscreen frames use all client edges; maximized frames retain native sizing insets. The original resize owner handles all eight edges. DPI handling reads the final client size after actual SetWindowPos. Native-caption handling remains the default path.

The prior no-activate patch is retained in `events.rs`: WM_MOUSEACTIVATE reads live GWL_EXSTYLE; WS_EX_NOACTIVATE yields MA_NOACTIVATE, otherwise MA_ACTIVATE. C# owns Windows policy, Rust owns staging/generation and GPUI owns rendering. No titlebar/resize-style stripping, fake DPI/settings messages, registry changes, injection or primary-window nonactivation.

Source/lock/provenance: `python tests/architecture/gpui-release-contracts.py`; optional `--upstream` compares independently extracted published files. GPU: `cargo test -p gpui-pre-windows --lib source_over_tests -- --test-threads=1`. Owned native frame/cache: `cargo test -p gpui-pre-windows --lib gpui_window_frame -- --test-threads=1`. Use the required low-memory Cargo environment. These filtered tests exclude unrelated system clipboard/credential tests. Native frame tests exercise the production boundary with owned HWNDs; they do not certify actual GPUI renderer lifecycle, physical resize/snap, mixed-DPI, native Acrylic or visual acceptance.
