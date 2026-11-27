# GPUI theme and translucent window material research (read-only)

Root: `research/repos/` under the session scratch directory. Line numbers come from the current shallow checkouts.

## 1. longbridge__gpui-kit (gpui-component 0.10.0, gpui-pre =0.3.12)
- **Material variants used:** only `Opaque` and `Transparent`, and only on Linux.
  - `crates/story/src/lib.rs:149-152,186-191` sets `window_background: Opaque` plus `WindowDecorations::Client` under `#[cfg(target_os="linux")]`. The comment says an alpha surface "lets compositors show the desktop through light themes".
  - `examples/dock/src/main.rs:433-436` sets Linux `Transparent` plus Client decorations.
  - No Blurred, Mica, or MicaAlt anywhere. gpui-kit makes no native calls: no SetWindowCompositionAttribute, ACCENT, or DWMWA in crates. It relies on gpui-pre's Windows backend.
- **Theme translucency model:** not found. There is no appearance, opacity, or blur field.
  - Window-related schema keys are `"window.radius"` (`theme/schema.rs:75-76`, applied at 1121-1122, default `px(0.)` in `theme/mod.rs:802`) and `"window.border"` (`schema.rs:657-658`, falls back to `self.border` at 1039; `default-theme.json:100` `#e5e5e5` light, `:303` `#262626` dark).
  - The root surface always paints opaque `cx.theme().tokens.background`, rounded with the window radii (`component/src/root.rs:735-747`).
  - The title bar paints a gradient from a mix of 55% title_bar + 45% background to title_bar, at 180° (`title_bar.rs:26-41`).
  - None of the 21 files in `themes/` uses an 8-digit hex for background, title_bar, popover, or window.border.
- **Legibility under translucency:** not found.
- **Corners, border, shadow** (`component/src/window_border.rs`):
  - `SHADOW_SIZE` = 0 px outside Linux and 20 px on Linux (11-14). `BORDER_SIZE` 1 px, resize hit band 4 px (15-17).
  - Radii come from `window_radius` only when `Decorations::Client`, and only for corners not next to a tiled edge (28-43).
  - The outer `window-backdrop` div is `transparent_black` with shadow padding. The inner div draws the border only on untiled sides, plus two shadows: `a: 0.18 * (1.0 active | 0.7 inactive)` with blur 10 / spread -1 / offset y2, and blur 3 / offset y1 (203-293).
  - Corners: `window.set_client_corner_radii(radii)` "lets the compositor treat the rounded corners as transparent" (196-198).
  - Duplicate-frame guard: the platform inset stays at the full shadow size even when fully tiled ("makes the first resize after restore double-count the shadow", 184-195).
  - With Server decorations it returns a plain div that keeps the same `window-backdrop` id so keyed element state survives (169-179).
  - On Windows there is no CSD: `TitleBar::window_options()` uses `appears_transparent: true` and `app_owns_titlebar_drag: true` (`title_bar.rs:63-93`).
  - `crates/webview/src/windows.rs:43-63` uses `SetWindowRgn` only to clip the WebView child HWND.
- **First-frame flash:** not found. `story-web/src/lib.rs:132` covers web appearance only.
- **Bundled translucent themes:** none. Docs and website mentions of blur, Mica, Acrylic, or WindowBackgroundAppearance: not found.

## 2. longbridge__nsh (Native Shell)
nsh has two Windows paths: a native Direct2D/DirectComposition renderer (`crates/nsh-platform-windows`) and a GPUI backend (`crates/gpui/nsh-platform-gpui`).

- **Selection:** per-window attributes, not the theme. `transparent: bool` and `backdrop: "none"|"window"|"sidebar"|"popover"|"hud"` (`website/modules/nsh-desktop.md:213-215`).
- **GPUI mapping** (`nsh-platform-gpui/src/window.rs:1281-1295`, applied with `set_background_appearance` at 1225-1239, test 2401-2409):
  ```rust
  Backdrop::None if transparent => B::Transparent,
  Backdrop::None => B::Opaque,
  Backdrop::Window if cfg!(windows) => B::MicaBackdrop,
  Backdrop::Sidebar if cfg!(windows) => B::MicaAltBackdrop,
  Backdrop::Window | Backdrop::Sidebar | Backdrop::Popover | Backdrop::Hud => B::Blurred,
  ```
  So on Windows, Popover and Hud become GPUI `Blurred`.
- **Native Windows mapping** (`nsh-platform-windows/src/shell.rs`):
  - `backdrop_type` sets DWMWA_SYSTEMBACKDROP_TYPE: None→1, Window→2 (Mica), Sidebar→4 (Mica Alt), Popover|Hud→3 (Acrylic) (256-265).
  - `set_material` (791-837):
    - Margins are `-1` with a backdrop, `1` for a frameless shadowed opaque window, else `0` (`DwmExtendFrameIntoClientArea`).
    - `bare = frameless && (!shadow || transparent)`. When bare or zoomed, it sets `DWMWA_NCRENDERING_POLICY=1` and `DWMWA_WINDOW_CORNER_PREFERENCE=1` (do not round); otherwise both are `0`.
    - Zoom changes re-run it (`zoom_changed`, 842-850). The reasons are documented at 770-786: frame border and DWM caption buttons would otherwise show on a zoomed backdrop window.
- **Translucent roles:**
  - The window base becomes `Color::TRANSPARENT` when transparent or backdropped; otherwise `0x0a0a0aff` dark or `0xffffffff` light (`nsh-platform-windows/src/system.rs:224-232`; `surface.rs:866-877`).
  - The GPUI root paints the host theme background quad only when not see-through (`nsh-platform-gpui/src/window.rs:2205-2215`).
  - Per-view blur: `backdrop: { material: "blur" }` uses `CreateHostBackdropBrush` plus `DWMWA_USE_HOSTBACKDROPBRUSH` sprites under the DComp tree (`docs/dev/windows.md:84-94`).
  - Gallery glass tokens (`examples/gallery-rs/native/src/ui/theme.rs`):
    - `GLASS` (432-448): fill `0xfafaf8@0.66` / `0x16171a@0.62`, border `INK@0.07` / `WHITE@0.07`, inner rim `WHITE@0.9` / `WHITE@0.09`, blur 20.
    - `MENU` (450-469, "denser for reading", 260): fill `WHITE@0.84` / `0x1e2024@0.82`, border alpha 0.08, blur 24.
- **Legibility:** no automatic text or border adjustment found. Readability comes from denser alpha on reading surfaces, plus rims and hairline borders.
- **Corners and border:** set through DWM attribute 33 (corner preference) as above, not SetWindowRgn. `native.rs:1093-1110` uses `SetWindowRgn`/`CreateRoundRectRgn` only for hosted native child HWNDs.
- **First-frame flash:**
  - `app.rs:797-807` `show_main` does `set_cloaked(true)`, then `fill_background` (GDI FillRect), then `paint()`, then show, then `set_cloaked(false)`.
  - `set_cloaked` uses `DWMWA_CLOAK` (`window.rs:968-982`). `fill_background` is skipped for transparent windows (868-878).
  - The fill also runs during resize (`window.rs:1436`).
- **Bundled files:** `examples/nsh-gallery-ui/finishes.json:89,187` `"backdropFilter": "blur(20px)"`. No JSON sets a window `backdrop`.

## 3. zed-industries__zed (sparse checkout)
- **Enum** (`crates/gpui/src/platform.rs:2986-3009`): `Opaque` (default) | `Transparent` | `Blurred` | `MicaBackdrop` | `MicaAltBackdrop`.
- **Windows calls** (`crates/gpui_windows/src/window.rs:918-943`):
  ```rust
  Opaque => set_window_composition_attribute(hwnd, None, 0),
  Transparent => set_window_composition_attribute(hwnd, None, 2),
  Blurred => set_window_composition_attribute(hwnd, Some((0,0,0,0)), 4),
  MicaBackdrop => dwm_set_window_composition_attribute(hwnd, 2),   // DWMSBT_MAINWINDOW
  MicaAltBackdrop => dwm_set_window_composition_attribute(hwnd, 4),// DWMSBT_TABBEDWINDOW
  ```
  - SWCA requires build ≥17763 (1634). It is loaded from user32 by name. `attrib: 0x13`. ACCENT flags are `0` for acrylic (state 4) and `2` otherwise. Acrylic alpha 0 is forced to 1 (1630-1675).
  - Mica uses `DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE, n)` only on build ≥22621 and silently returns below that (1606-1628).
  - The Opaque, Transparent, and Blurred branches never write DWMWA_SYSTEMBACKDROP_TYPE.
- **Renderer:**
  - Clear color is `[1.0;4]` for Opaque and `[0.0;4]` for anything else (`directx_renderer.rs:375-379`).
  - The DComp swap chain uses `DXGI_ALPHA_MODE_PREMULTIPLIED` (1362). The HWND fallback swap chain uses `ALPHA_MODE_IGNORE` (1390). `WS_EX_NOREDIRECTIONBITMAP` is set when DComp is on (`window.rs:515-517`).
- **Theme field:** `ThemeStyles.window_background_appearance` (`theme/src/styles/colors.rs:623-626`), with getter `Theme::window_background_appearance()` (`theme.rs:291-295`).
  - The fallback theme is Opaque (`fallback_themes.rs:112`). The VS Code importer writes `WindowBackgroundContent::Opaque` (`theme_importer/src/vscode/converter.rs:59`).
  - The JSON `background.appearance` → `WindowBackgroundContent` conversion and the `set_background_appearance` call site are not in this sparse checkout (no match in theme, workspace, or title_bar).
  - Applied at creation from `WindowOptions.window_background` (`gpui/src/window.rs:1753`) or at runtime with `Window::set_background_appearance` (3462).
- **Translucent roles:** no automatic alpha. Colors are used exactly as the theme gives them. Built-in defaults only add alpha to derived colors: status `*_background = fg.opacity(0.25)` (`fallback_themes.rs:29-44`) and selection `a = 0.25` (51-57).
- **Legibility:** subpixel text AA is disabled whenever the appearance is not Opaque (`gpui/src/window.rs:5376-5378`). No other adjustment found.
- **Corners, border, shadow:**
  - CSD lives in `workspace/src/workspace.rs:12590-12748`, only for `Decorations::Client`.
  - `CLIENT_SIDE_DECORATION_ROUNDING = 10 px` and `CLIENT_SIDE_DECORATION_SHADOW = 10 px` (`theme.rs:53,55`).
  - It calls `set_client_inset` and `set_client_corner_radii`. The outer backdrop div is `transparent_black`. The inner div draws the border on untiled sides only, with shadow `a: 0.4` and blur `SHADOW/2`.
  - The status bar rounds its bottom corners itself (`status_bar.rs:160,165`).
  - `gpui_windows` has no Decorations or corner-preference code. Windows corners are DWM defaults.
- **First-frame flash:** no cloak. The window class `hbrBackground` is `CreateSolidBrush(COLORREF(0x00000000))` (black) (`gpui_windows/src/window.rs:1437`).
- **Bundled themes:** `assets/themes/{one,ayu,gruvbox}.json` set only `"appearance": "dark"|"light"`. None sets `background.appearance`.

## 4. ZacharyZhang-NY__Ely-GPUI-Components (gpui-pre =0.3.8)
- **Variants used:**
  - `Blurred` for the gallery "Frosted" window (`examples/gallery/pages/theme/platform.rs:129-148`, `appears_transparent: true`).
  - `Transparent` for the quick launcher `WindowKind::PopUp` (`src/navigation/palette/launcher.rs:42-50`).
  - Mica is still open: "[ ] Mica — prove ... from Windows 11 build 22621" (`tasks/ch31-43.md:272`).
  - Ely makes no native Win32 calls. It relies on gpui-pre.
- **Theme model:** no window-appearance field. Translucency is a `glass` color token used by `shell::Vibrancy` (`src/shell/vibrancy.rs:8,38-41`: `self.base.bg(cx.theme().colors.glass)`).
  - `glass` is `rgba(0xfcfaf7b8)` light and `rgba(0x252525b8)` dark, alpha 0xb8 ≈ 0.72 (`src/theme/palette.rs:208,280`). The opaque `bg` values are `0xfcfaf7` and `0x252525` (178, 250).
  - Scrims: `backdrop` `rgba(0x18161352)` light / `rgba(0x00000080)` dark; `media_backdrop` `0x0e0d0bf8` / `0x000000f8` (205-206, 277-278).
  - In the demo only the 180 px sidebar is glass. The content pane stays opaque `colors.bg` with `border_l_1` `colors.border` (`platform.rs:157-185`).
- **Legibility:**
  - High contrast makes glass opaque and strengthens text and borders (`palette.rs:237-243`, dark 309-314). Example values: `fg_muted 0x44423f`, `border 0xa6a4a2`, `glass = bg`. A test asserts `strong.glass == strong.bg` (`theme/tests.rs:86-90`).
  - The contrast devtool counts translucent text as it blends in (`src/devtools/contrast.rs:61,195`; `src/tooling/a11y.rs:268`).
- **Corners and border:** `shell::ResizeBorder` (`src/shell/frame.rs:54-114`) runs only on `Decorations::Client`.
  - It uses `set_client_inset(theme.window_inset())`.
  - The pane is opaque `colors.bg` with `rounded(Radius::Lg)`, `border_1 border_strong`, and `shadow(Elevation::Modal)` only when not tiled.
  - It does not call `set_client_corner_radii`.
- **First-frame flash:** not found.
- **Bundled theme files with translucent appearance:** none.

## Patterns across these repos
1. **Material lives on the window, not in the colour roles.** Only Zed stores a `WindowBackgroundAppearance` in the theme (`ThemeStyles`). nsh uses per-window `transparent` and `backdrop` attributes. gpui-kit and Ely pass `WindowOptions.window_background` directly. None of them converts theme colours to translucent ones automatically.
2. **Translucency is opt-in per surface.** gpui-kit's root, Zed's colours, and Ely's content pane all stay opaque. nsh and Ely make specific panes see-through instead: an opaque base is skipped when see-through (nsh), or a dedicated glass token is used (Ely `0xb8`, about 0.72; nsh glass 0.62–0.66, menus 0.82–0.84).
3. **Legibility comes from stronger tokens, not automatic contrast.** Reading surfaces get denser alpha. Rims and hairline borders sit at alpha 0.07–0.08. High contrast makes glass opaque (Ely). GPUI turns off subpixel AA for any non-Opaque window (Zed `window.rs:5376`).
4. **Windows calls:**
   - GPUI (Zed): Blurred is SWCA ACCENT 4 (acrylic), Transparent is ACCENT 2, Opaque is ACCENT 0; Mica is DWMWA 38 = 2 and MicaAlt is 38 = 4, with a build gate at 22621.
   - nsh native: DWMWA 38 (1, 2, 3, 4), `DwmExtendFrameIntoClientArea(-1)`, and DWMWA 33 = 1 (do not round) or 0 (default) for bare or zoomed windows.
5. **Corners and frame:** client-side decorations (Linux) use a transparent outer `window-backdrop` and an inner rounded div with per-side borders on untiled sides only, plus `set_client_inset` and `set_client_corner_radii`. Windows corners come from DWM. Duplicate-frame guards are explicit in two places: gpui-kit keeps the inset stable when fully tiled, and nsh disables NC rendering and rounding when zoomed.
6. **First frame:** only nsh handles the flash explicitly, with `DWMWA_CLOAK` on, then a GDI fill, a paint, show, and cloak off. Zed and GPUI register a black class brush and use `WS_EX_NOREDIRECTIONBITMAP` with DirectComposition.
