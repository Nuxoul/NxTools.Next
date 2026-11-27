# Quick Launch and Clipboard exterior corners

## Ordinary windows in Acrylic

Acrylic sets the Component `window_radius` to 8 DIP. The contour is GPUI-drawn UnderBorder (not the system `DWMWCP_ROUND` corner): the Root surface is extended by 1 DIP over the frame ring, carries fill and a 1 DIP border as one element, and the official frame border is made transparent so the arc is covered once. C# adds an HRGN only to cut the native blur out of the square corners. See [native corners](corner-native.md).
## Stage four surface contract

Quick Launch and the resizable Clipboard popup use an 8 DIP exterior in Solid and Acrylic. This is per-window presentation, not a change to the global Component `Theme.window_radius` or other windows. The existing UnderBorder composition draws the background and solid border under one contour. Palette, mode, fonts, controls, IDs, search/IME, selection, geometry, saved bounds and the original material/prepare/show owners remain unchanged.

Quick Launch keeps its official Large Input (44 DIP) in the 58 DIP header, 620 DIP width and 60/530 DIP compact/search sizes. Clipboard retains its original official InputGroup, controls and inset body/editor/footer. No Toolbar adaptation or control variant/size changes are needed. Layout divs only adapt the exterior; no replacement ordinary controls or vendor changes.

## Verified references

- `D:\Code\Listary.Decompiled\6.3.5.94\managed\Listary\Main\Views\LauncherSearchWindow.xaml.cs:24-32,50-69,91-96`: constructor/base and SourceInitialized placement; outer HWND positioning uses NOACTIVATE/NOSIZE/NOZORDER. `SearchWindowBase.cs:318-336,373-398` binds show/hide/close separately from foreground activation and search focus. This is managed boundary evidence; Listary native engine/renderer source is not available. Legacy NxTools has no Quick Launch implementation.
- `D:\Code\NxTools\Views\ClipboardHistoryPopupWindow.axaml:7-17,67-71` declares a resizable undecorated popup and inset square legacy border. Its constructor and Opened/Activated/Deactivated/Closed chain is in `ClipboardHistoryPopupWindow.axaml.cs:61-167`; `PrepareForPopupShow/HidePopup:206-260` reuses the same window and restores search focus. The approved 8 DIP target changes the exterior, not that lifecycle or the legacy palette.
- Existing reference notes: `docs/reference-code/quick-launch-research.md` (Windows10 outer corners), `docs/reference-code/clipboard.md` (popup lifecycle). Current public API authority is `docs/reference-code/gpui-kit-0100.md` plus the mandatory component selection/table/index.

## Verified render chain before implementation

1. `vendor/gpui-base-0100/src/root.rs:175-189,241-275`: one Root retains its application view; plugin styles run before public per-instance `Styled` refinement, and decoration follows. The instance corner slots can override only geometry while leaving the material plugin's background intact.
2. `vendor/gpui-component-0100/src/root.rs:729-757`: WindowState applies tokens.background and `window_corner_radii`, then decorates through WindowBorder.
3. `vendor/gpui-component-0100/src/window_border.rs:19-42,165-198`: the public helper reads global window_radius (default square) and client tiling. Overflow masks are rectangular, so rounded Root alone does not clip a square descendant fill. Windows and TestWindow use the platform's Server decoration reporting; the product popup radius must not rely on a global theme radius or require Client reporting. Maximize/fullscreen and client tiled edges remain square.
4. `rust/apps/nxtools-desktop/src/ui/material/paint.rs:67-100`: MaterialRoot owns the existing accepted-result projection. Pending/invalid Acrylic and AcrylicOpaque use alpha 1; confirmed Acrylic uses 0.62 dark/0.72 light; Solid retains official opaque tokens. Instance corner slots must not override this background.
5. `quick_launch.rs::open_panel` constructs one Root for QuickLaunchPanel. `quick_launch/panel.rs::render` paints a full-client canvas fill and 1 DIP canvas border, plus a canvas search header inside that border. Both the exterior and touching header need their own corners. The inner header contour is inset by the existing 1 DIP border: 7 DIP at the top and, in compact mode, bottom. Expanded result/action content has 8 DIP internal padding; the status footer has 12 DIP side and 8 DIP bottom margins.
6. `clipboard.rs::open_panel` constructs one Root for ClipboardPanel. `ui/mod.rs::panel_window_frame` paints the full-client canvas and 1 DIP border. Header, search, history and editor/footer fills are inset from the exterior; only the shell fill/border touches all four corners. The resize hitbox has no background. No new body/footer fill or clip owner is required.

## Public integration

Use `Root::new` plus public `Styled::{rounded_tl,rounded_tr,rounded_bl,rounded_br}` on only these two popup instances. A narrow `ui/popup_surface.rs` layout helper supplies 8 DIP corners, squares maximized/fullscreen/tiled edges, and reapplies the Root's corner slots from its own `Context<Root>::observe_window_bounds` callback. This is local Root state and does not update the application or leased child entity. Child fills read the same current geometry during rendering. No global plugin, manager, HWND call, background setter, style stripping or palette branch is added.

The existing Quick Launch native bounds request already carries 8 DIP converted to device pixels. Clipboard's native request and native HRGN maintenance belong to the separate C# / parent slice; Rust exterior rendering is not native hit-test evidence.

## Regression boundary and limits

The explicit TestWindow regressions inspect actual Root and production popup Quads, including exterior radii, pending/confirmed/fallback/Solid alpha, compact/expanded Quick Launch touching fills, and retained Root/view/input/selection/focus across theme changes. A plain Root control window stays square. Corner policy cases cover partial/all tiling, maximize and fullscreen without changing TestWindow or vendor code. Assertions inspect each painted quad reaching an exterior corner, not just a rounded ancestor or token equality.

## Verified behavior

- `corner_surface_`: four compiled tests pass on the current Kit0.10.0/pre0.3.12 graph. Root8DIP radii and alpha are exact; selected RGB channels use the public solid-color readback. Device-space bounds follow official pixel snapping, and inset children allow only half a device pixel of rounding difference.
- Original `panel_shell_fills_the_native_window_without_a_uniform_transparent_gutter` and `inset_sections_leave_safe_edges_and_resize_stays_on_the_native_corner` both pass. Main Quick Launch presentation10, Clipboard material6 and auxiliary surfaces2 pass.
- Release/editor provenance, spec navigation and scoped whitespace checks pass. Only new files are formatted; existing large files receive parser-only checks.
- Clipboard's main Root uses the per-instance helper and its original first bounds command supplies8DIP at the captured monitor scale. Preview and link-panel Roots retain their original ownership and geometry.

These are TestWindow scene and source results. Actual antialiased GPU pixels, native hit testing, physical resize/mixed DPI and user visual acceptance remain separate; [native region evidence](corner-native.md) records the self-owned HWND fixtures. No ordinary product, real clipboard/history, package, installation or service operation is included.
