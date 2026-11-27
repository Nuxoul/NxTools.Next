# GPUI window material / translucency research (5 repos)

Paths below are relative to `research/repos/<repo>/`.

## 1. maddada__Ghostex (custom GPUI fork: `BehindWindowBlur`, `set_background_blur_style`, `set_background_corner_radius`; the fork itself is not in the clone)
**Material**: a user setting. `refresh_window_glass` in `apps/desktop/src/app/helpers/window_glass.rs:280-296` reads `windowGlass` = `"frosted"|"opaque"|auto`. Auto means frosted in dark mode and opaque in light mode (comment :274). It resolves to `Blurred` or `Opaque` (:713-720):
```rust
if window_glass_active() && !window_art::window_art_active() { WindowBackgroundAppearance::Blurred } else { WindowBackgroundAppearance::Opaque }
```
- Windows: a window that should be Opaque is created as `Transparent` so it stays on DirectComposition and glass can be switched later without a restart (:560-577; comment :559). The Windows blur choice `windowGlassWindowsBlur` maps to `BehindWindowBlur::{System(Acrylic), Plain, Off}` (:39-50). A comment names `DWMSBT_TRANSIENTWINDOW` as the backing (:863).
- Blur radius slider `windowGlassBlurRadius` 0-100, default 4 (:54, :71). Menu blur default `FROSTED_MENU_BLUR_RADIUS = 17.0` (:911). Surfaces other than tooltips use x`1.25` (:947).
- Popups/menus are their own windows, opened `Blurred` when glass is on, else `Transparent` (`app/titlebar/popup_lifecycle.rs:153-157`, `app/window/main_hub_card.rs:102-105`).
**Surface layering ("tint once")**:
- Root shell is `transparent_black` under glass (:1038-1044). The body row, nested work-area fills and pane fills are also cleared (:1095-1111, :1178-1184).
- The sidebar and the work area each paint their own tint straight over the blur. Defaults: sidebar dark `0.61` / light `0.64`, work area dark `0.67` / light `0.64` (:200-203). These are user sliders clamped 0-100% (:237-240, :256-265).
- Terminal default background alpha is `0.0` under glass. Cells with an explicit background stay opaque (:1197-1199).
- Menus, popovers, dialogs and tooltips share one fill, `frosted_panel_fill = color.opacity(a)`. The alpha is `0.6` dark / `0.75` light on macOS, `0.86` (dark) / `0.9` (light) on Windows, and `1.0` on Linux (:1010-1028, :978-982).
- Surfaces over content inside a window are lifted toward white: `GLASS_LIFT_DARK 0.18`, `GLASS_LIFT_LIGHT 0.6`, at alpha `GLASS_LIFTED_ALPHA 0.96` (`helpers/chrome_palette.rs:74-94`). Veil `0.6` (:1151).
**Legibility**:
- The 18% lift is the highest step that keeps `#e5e5e5` body text at 4.5:1 or better (chrome_palette.rs:69-73).
- Dividers under glass are chrome ink at `0.08` (:1167-1174). Ink wash is `white 0.06` dark / `black 0.04` light (chrome_palette.rs:104-109). `glass_card` uses base `0.06`/`0.04` x strength (:1117-1127).
- Dark tooltips blend toward black `0.55` (:883-888).
- With an animated backdrop, alpha becomes `alpha.powf(2.0 dark / 2.5 light)` (:1050-1055).
**Corners/border**:
- Windows popups set the corner radius to DWM's fixed 8px/4px (`DWMWCP_ROUND`/`ROUNDSMALL`) and `DWMWA_BORDER_COLOR = 0xFFFFFFFE` (no border), so the card draws its own border (`app/window/popup_frame.rs:451-506`).
- App modals use `DWMWCP_ROUND` plus a theme COLORREF border (:510-541). `modal_window_frame.rs:465-489` uses `DWMWCP_DONOTROUND`.
- macOS strips the popup frame natively (`popup_frame.rs:442`).
**Reveal**:
- Popovers that size themselves open `Transparent` and turn glass on as they fade in (`popup_lifecycle.rs:150-153`).
- The workspace colour is read before the window opens so the first paint is correct (`main.rs:355-358`).
- No hide-until-first-frame for the main window was found.
**Fallbacks**:
- Glass is turned off when the system reduces transparency: macOS `AccessibilityDisplayShouldReduceTransparency`, Windows registry `Personalize\EnableTransparency == 0` (:465-492).
- Linux: off on GNOME, and on Hyprland when blur is disabled (:506-516). Hyprland is watched live (:526-549).
- Art themes force Opaque. Light/dark is tracked by `CHROME_LIGHT_APPEARANCE`.

## 2. s1dashu__quiet-md (vendored zed gpui with "quiet.md:" patches)
**Material**: fixed per platform. `src/theme.rs:22-39`:
```rust
#[cfg(target_os = "windows")] { ... windows_build().is_some_and(|b| b >= 22621) }  // Win10/11 21H2 acrylic drag lag (#32)
#[cfg(any(target_os = "linux", target_family = "wasm"))] { false }
... if glass() { WindowBackgroundAppearance::Blurred } else { WindowBackgroundAppearance::Opaque }
```
- Used for the main window and quick note (`src/main.rs:129`, `src/quick_note.rs:68`).
- On Windows, vendored gpui maps `Blurred` to `SetWindowCompositionAttribute` accent state `4` (ACRYLICBLURBEHIND) with color `(0,0,0,0)`, `Transparent` to state `2`, and Mica/MicaAlt to `DWMWA_SYSTEMBACKDROP_TYPE` 2/4 (`vendor/zed/crates/gpui_windows/src/window.rs:1614-1636`).
- macOS 26: a native `NSGlassEffectView` sits under the GPUI view, sized to the sidebar width + 12px underlap, `setCornerRadius:0`, applied with `present_next_frame_in_transaction` (`src/native_header.rs:9-57`).
**Surfaces**: only the sidebar/window ground is translucent. `Theme::window()` (`src/theme.rs:414-419`):
- alpha `0.0` with NSGlassEffectView
- alpha `0.84` of `#eff0f1` with glass
- `1.0` when the user preference `opaque_window` is set (`settings.rs:124, 1567`) or the window is fullscreen

The note page is opaque. Popovers are `#fafafa` (raised `#2c2c2e` in dark) and opaque (:157, :458).
**Legibility**:
- Sidebar text uses `N900 @0.85`, hover `black 0.045`, selected `0.072`, drop `0.05`/`0.08` (:422-433).
- Dark mode: lightness is mirrored in OKLab (`to_dark` :79-88). Highlights are made opaque in dark mode (`mark` :94-99).
- Shadows get x3 alpha, capped at `0.6`, in dark mode (:272-274).
**Corners**: Linux uses `WindowDecorations::Client`. macOS and Windows use `appears_transparent` titlebars (`main.rs:131-134`). No DWM corner calls in `src/`.
**Reveal/recovery**:
- No hide-until-ready found.
- After a GPU device loss, the vendored gpui re-applies the accent as Opaque and then the original appearance, because setting the same accent again may be ignored (`gpui_windows/src/events.rs:1349-1356`).
**Light/dark**: an `AtomicBool DARK` global with an adapt mapping. No reduce-transparency check found.

## 3. nyakang__nyaterm (gpui-kit)
**Material**: a user setting `appearance.window_material` = `opaque|transparent|blurred|<unknown>`, default Opaque (`crates/nyaterm-core/src/models/window_material.rs:8-14`; `nyaterm-store/src/storage/app_settings.rs:81-89`). It resolves against capabilities (`crates/nyaterm-desktop/src/features/shell/window_material.rs:9-20`):
```rust
WindowMaterial::Transparent if capabilities.transparent => Transparent,
WindowMaterial::Blurred if capabilities.blurred => Blurred,
_ => Opaque,
```
- Capabilities (`nyaterm-desktop/src/platform/windows.rs:11-35`, `platform/mod.rs:66-89`):
  - Windows: build >= 17763, not `GPUI_DISABLE_DIRECT_COMPOSITION`, and `user32!SetWindowCompositionAttribute` exists.
  - macOS: always supported.
  - Others: false.
- The window opens with the default (Opaque) background (`app_shell/controller/workspaces.rs:138-153`). The material is applied afterwards through `cx.defer`, which calls `window.set_background_appearance(..)` and sets the gpui-kit Root bg to `rgba(0)` (`window_material.rs:89-112`; `nyaterm-ui/src/root.rs:28-30`).
**Surfaces** (`ShellBackground`, window_material.rs:30-73):
- `root_color` = theme bg at `window_background_opacity`, default `20`%, clamped `min(100)` (`window_material.rs:44-46` core; store :90-98).
- `surface_color` (content surfaces) = `background_opacity`, default `82`, clamped 0-100 (store :161-173).
- `transparent_color` (structural containers) = alpha 0.
- The terminal uses `surface_color`. Legacy wallpaper shading `a += (1-a)*a` only applies to opaque windows (:43-51).
- A test asserts the tint is painted exactly once (:224-281).
**Legibility**:
- `text_on_fill` keeps white if contrast is >= `2.0`, otherwise uses the darker of bg/text (`nyaterm-ui/src/theme.rs:54-65`).
- `hover_fill` mixes 12% toward white or black (:71-84).
- Nothing translucency-specific found.
**Corners**: gpui-kit Root draws the Linux CSD frame (`root.rs:62-65`). Linux `window_decorations` is a user setting (`workspaces.rs:136-149`). No DWM corner calls.
**Reveal**: not found.
**Fallbacks**: unsupported or unknown materials go Opaque without rewriting the preference. The settings slider is disabled when not applicable (`pages/settings/workspace/window_background.rs:24, 181`). No reduce-transparency check.

## 4. lattice-writer__lattice (macOS only)
**Material**: user setting Settings > Appearance > Translucency `Off|Subtle|Strong` (`gpui/crates/lattice-gpui/src/window_material.rs:52-58`). On macOS it adds its own `NSVisualEffectView` below GPUI's view and sets GPUI `Transparent`; otherwise `Opaque` (:128-147, :173-215):
```rust
const UNDER_WINDOW_BACKGROUND: isize = 21; const BEHIND_WINDOW: isize = 0; const FOLLOWS_WINDOW_ACTIVE_STATE: isize = 0;
window.set_background_appearance(if clear { Transparent } else { Opaque });
```
- It is applied on open and again on every activation, because Reduce Transparency is only reported when asked (`lattice-gpui/src/shell.rs:404-414`).
- `NSAppearance` is pinned to the chosen theme with `NSAppearanceNameDarkAqua`/`Aqua` (:151-169).
- Non-macOS returns `Backing::Unsupported` (:245-260).
**Surfaces**: `Palette::with_glass` (`gpui/crates/lattice-ui/src/palette.rs:362-379, 528-545`):
- Window material is cleared.
- Shell (titlebar/gutters) keeps a `shell` share of chrome: Subtle `0.40`/`0.45` (light/dark), Strong `0.0`.
- Full-window pages keep `page`: Subtle `0.72`/`0.74`, Strong `0.52`/`0.62`.
- The titlebar adds a white veil `0.38` light / `0.045` dark over the shell.
- "Every panel, the side panels among them, stays opaque."
- Popover cards use `backdrop_blur(px(14.), palette.raised.alpha(1.))` (`lattice-editor/src/latex/cards.rs:74`), from the patched gpui-fast backdrop with an opaque fallback colour (`patches/gpui-fast/crates/gpui/src/fast/backdrop.rs:1-30`).
**Legibility**: nothing material-specific beyond the veils. Accent soft is `0.14` dark / `0.10` light. Tinted surfaces use `0.12`/`0.18`/`0.24` (palette.rs:422-473).
**Corners**: native macOS frame with `appears_transparent` and traffic lights at (14,13) (`shell/windows.rs:763-772`). Other platforms keep the system title bar (:775-778).
**Reveal**: not found.
**Fallbacks**: `Backing::{Translucent, Opaque, ReducedTransparency, Unsupported}` (window_material.rs:28-38). Glass is `Off` unless Translucent (:74-78).

## 5. nowledge-co__con-terminal (gpui-component)
**Material**:
- The main window is `Transparent` when supported (macOS >= 13, Windows, Linux CSD), else `Opaque` (`crates/con-app/src/main.rs:223-257, 616-624`). The comment says the macOS blur comes from Ghostty's NSWindow config, and that GPUI's NSVisualEffectView "would add a second material".
- Windows backdrop: `DWMWA_SYSTEMBACKDROP_TYPE`. With `terminal_blur` it tries `DWMSBT_TRANSIENTWINDOW` (Acrylic) first, then falls back to `DWMSBT_MAINWINDOW` (Mica). If both fail, the Root uses an opaque theme bg (:494-552, :822-833). The code explicitly does not call `DwmExtendFrameIntoClientArea`, because that made the DComp window opaque (:508-517).
- Linux always uses `Transparent` (:300-303).
- Settings defaults: `terminal_opacity 0.80`, `ui_opacity 0.90`, `terminal_blur true` (`con-core/src/config.rs:32-40`).
**Surfaces/opacity** (`con-app/src/workspace/chrome.rs:13-90`):
- Clamps: terminal `0.25..1.0`, UI `0.35..1.0`.
- Remap: terminal `remap(v, 0.25, 0.72, 1.55)` and UI `remap(v, 0.35, 0.84, 1.9)`, so effective floors are 0.72 / 0.84.
- Elevated = `ui + 0.02`, max `0.98`.
- Older macOS forces `1.0`.
- Top bar uses `theme.title_bar.opacity(ui)`. On macOS it first blends foreground `0.045*depth` (`workspace/render.rs:595-600`).
- Popups use `background.opacity(elevated)` with border `muted 0.16` (`render/popups.rs:245-247`).
- Seam covers are forced to `a: 1.0` (render.rs:580-590).
- Terminal default-bg cells are clear at `background_opacity`. Alternate-screen apps get floor `1.0` (`con-ghostty/src/windows/render/mod.rs:62, 1175-1187`).
**Legibility**:
- Tab selection is resolved against chrome, not the desktop. Dark: `backdrop.blend(fg 0.22)`. Light: `l+0.04`. Then `0.96` alpha.
- Resting tab `fg 0.045`/`0.035`, hover `0.075`/`0.06` (`render/top_bar.rs:602-623`).
- Popup hover `primary 0.08`, selected `0.10` (popups.rs:264-268).
- Tab accent alpha defaults `0.15`/`0.22`, clamped (config.rs:50-63).
**Corners**:
- Linux X11 uses a SHAPE bounding+clip mask built from per-row rounded-corner rectangles (`main.rs:314-492`). Wayland relies on the transparent surface plus CSD.
- No DWM corner/border calls found.
**Reveal**: not found for window open. The postmortems cover in-window reveal and seam issues (`postmortem/2026-10-10-macos-terminal-handoff-flash.md`, `2026-10-24-input-tray-material-and-reveal.md`). macOS chrome animation next to native Ghostty views is `Duration::ZERO` (chrome.rs:92-102).
**Fallbacks**: the DWM failure chain above, and the macOS < 13 opaque path. No reduce-transparency check found.

## Patterns across these repos
1. **Variants used**:
   - `Blurred`: Ghostex, quiet-md.
   - `Transparent` plus a native material: Lattice (NSVisualEffectView), con (DWM Acrylic/Mica, Ghostty blur), quiet-md macOS 26 (NSGlassEffectView).
   - User-selectable: nyaterm.
   - Mica variants: wired only in quiet-md's vendored gpui. con sets Mica directly through DWM.
2. **Clear root, tint once**: every repo clears the root or window and paints one tint per region. Ghostex and nyaterm explicitly avoid stacking tints and nyaterm tests it.
3. **Region translucency**: chrome and sidebar are translucent (0.4-0.84). Content, editor and note panes stay opaque or near-opaque (0.82-1.0). Menus and popovers stay dense: Ghostex 0.6-0.86, con UI+0.02, quiet-md and Lattice opaque.
4. **Opacity floors**: con clamps 0.25/0.35 and remaps to 0.72/0.84. nyaterm clamps 0-100. Ghostex clamps 0-100 and forces 1.0 on Linux.
5. **Selection and hover** resolve against the app's own chrome colour, not the desktop: con tab 0.96 alpha; Ghostex lift 0.18 at 0.96.
6. **Fallback gating**:
   - Reduce Transparency honored: Ghostex (macOS and Windows), Lattice (macOS).
   - Windows build gates: quiet-md >= 22621, nyaterm >= 17763.
   - Linux blur detection: Ghostex.
   - DWM failure falls back to an opaque theme bg: con.
7. **Windows corners via DWM** (`DWMWCP_ROUND`/`DONOTROUND` + `DWMWA_BORDER_COLOR`): Ghostex only. Linux client-side decorations: quiet-md, nyaterm, con. con also uses an X11 SHAPE mask.
8. **Startup flash**: no repo hides the main window until the first frame or until the material is ready. Ghostex delays glass on self-sizing popovers. quiet-md re-applies the accent after a device loss.
