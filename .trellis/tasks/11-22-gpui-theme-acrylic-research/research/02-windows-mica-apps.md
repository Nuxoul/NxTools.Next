# Windows material survey: six GPUI apps

Repos live under `research\repos\`. Paths are relative to each repo. Attribute numbers: 20 = DWMWA_USE_IMMERSIVE_DARK_MODE, 33 = DWMWA_WINDOW_CORNER_PREFERENCE, 34 = DWMWA_BORDER_COLOR, 35 = DWMWA_CAPTION_COLOR, 38 = DWMWA_SYSTEMBACKDROP_TYPE.

## 1. Ray-lea__Argus (task manager)
**Material**
- Main window: `MicaBackdrop` if `supports_backdrop()`, otherwise `Opaque` (src-gpui/src/frontend.rs:830-834). The value is fixed; there is no user setting.
- Gate: `build.is_some_and(|build| build >= 22621)`, with the comment "On build 22000, requesting Mica would leave the transparent surface unfilled" (src-gpui/src/shell.rs:716-720). The build comes from ntdll `RtlGetVersion` (shell.rs:698-714). Mica is also off when `windows_opaque_window_requested()`: "An HWND swap chain ignores alpha" (shell.rs:669-673). `ARGUS_TEST_BACKDROP_BUILD` overrides the build in dev builds only (shell.rs:677-693). tools/window-probe/steps/backdrop-fallback.json expects `backgroundAlpha` 1 at 22000.
- Vendored gpui (vendor/gpui-pre-windows/src/window.rs:985-1009): Opaque maps to SWCA accent 0, Transparent to 2, Blurred to 4, Mica to attr 38 = 2 and MicaAlt to attr 38 = 4. Mica returns early if build < 22621 (:1696). SWCA needs >= 17763 (:1718) and uses attrib `0x13` (:1752).
- Tray flyout and taskbar monitor windows: `Opaque` (frontend/tray.rs:1378, frontend/monitors.rs:1494). DWMWA_CLOAK is never set; DWMWA_CLOAKED is only read (activation.rs:79,127). DwmExtendFrameIntoClientArea: not found.

**Surfaces** (tokens in resources/theme.json, light/dark)
- Alpha 0: the root (a RootPlugin sets the bg to `transparent_black()` under Mica, frontend/theme.rs:319-324), `Palette.background = colors.background.opacity(0.)` (theme.rs:158-162), title_bar and title_bar_border (:280-281), table, table_head, table_even and row_border (:258-262, :279), and the component overlay (:290).
- Semi-transparent: `layer` (cards) `rgb(255 255 255 / 0.5)` / `/ 0.035`; layer-strong 0.78 / 0.06; control 0.72 / 0.06; control-hover 0.55 / 0.085; flyout 0.9 / 0.92; tip 0.96 (theme.json:59-69, 94, 109-119, 144).
- Forced opaque: the menu `surface` is `Hsla { a: 1., ..color("flyout") }`, because "menus paint their shadow under the whole surface, where it would darken a translucent one" (theme.rs:225-230). `face` and `raised` use the opaque `on_card()` (:135-138).
- Mica on and off share one palette. Only the background alpha changes (:158).

**Legibility**
- stroke 0.075 / 0.07, stroke-strong 0.14 / 0.12, hover 0.04 / 0.045, hover-strong 0.06 / 0.08, press 0.07 / 0.03 (theme.json:61-65, 111-115). Selected rows: `primary.opacity(0.15)` (theme.rs:264).
- Flatten helpers:
  - `on_card()` blends window-bg, then layer, then the token, and sets `a: 1.` (theme.rs:72-77).
  - `remainder()` solves for the alpha that, stacked over `under`, gives `over` (:80-85).
  - `widgets::solid()` forces `a: 1.` (frontend/widgets.rs:413-415).
- WCAG solver for the UI: not found. The only contrast code is for taskbar GDI text (shell/taskbar_draw.rs:1290-1296).
- Tooltips use `.shadow_none()` to avoid a "screen-only halo reported on NVIDIA composition over Mica" (widgets.rs:570-572).

**Corners/border**
- Main window: gpui_kit `TitleBar::window_options()` (frontend.rs:835). No corner or border attribute is set on it.
- Popups only: `DWMWCP_ROUND` via attr 33 on the tray flyout (tray.rs:301-307) and the monitor windows (monitors.rs:1229-1235).
- Control border: a 1px stroke plus an absolute `control_edge` div that makes the bottom edge stronger (widgets.rs:417-421; `control_edge = remainder(stroke, stroke-strong)`, theme.rs:142).

**Startup/reveal**
- The flyout starts layered at alpha 0 ("Transparent from the start: the first show lays it out unseen", tray.rs:309-310), then fades in with ease_out (:567).
- When the main window starts hidden it is sized with `SetWindowPos(SWP_NOACTIVATE|SWP_NOZORDER|SWP_NOMOVE)` before it is shown (native_runtime.rs:184-204).
- DWMWA_TRANSITIONS_FORCEDISABLED is toggled by window_motion (window_motion.rs:337-347).

**Light/dark and fallback**
- Attr 20 is set from the setting or `AppsUseLightTheme` (shell.rs:722-757) and reapplied on WM_SETTINGCHANGE (shell.rs:1701-1702). The NSIS installer sets attr 20 and attr 35 (packaging/windows/installer-ui.nsh:313-314).
- Below 22621, when the build is unreadable, or when an opaque swap chain is requested: `Opaque` window and opaque background (docs/design/window.md:3).

## 2. Koimsurai__Arke (file explorer)
**Material**
- User setting `Material { Solid, Mica, MicaAlt }` (crates/app/src/settings.rs:217-223). It appears in Settings only when Mica is available (workspace_settings.rs:1113-1122).
- At open: `MicaBackdrop` if `theme::mica()`, otherwise `Opaque` (main.rs:1975-1981; properties.rs:253-256). At runtime, `apply_theme` picks Opaque, MicaAlt or Mica (workspace.rs:597-605).
- Gate: `info.dwMajorVersion >= 10 && info.dwBuildNumber >= 22000` (win.rs:447-457). `ARKE_NO_MICA` or `ARKE_THEME` turns Mica off (main.rs:1275-1281).
- The vendored gpui skips attr 38 below 22621 (third_party/gpui-pre-windows/src/window.rs:1918-1920). On builds 22000-22620 the app therefore requests Mica, and its translucent palette, while gpui does nothing. Argus notes this case at shell.rs:717-718.
- Menus: `WindowBackgroundAppearance::Transparent`, `show: false`, AnchoredPopup (popup.rs:497-515).
- DWMWA_CLOAK = 1 only on a hidden "open in folder" stand-in window (win_openinfolder.rs:611-620). SWCA and DwmExtendFrameIntoClientArea calls in the app: not found.

**Surfaces** (crates/app/src/theme.rs)
- `frame()` (sidebar and top bar) under Mica: alpha 0.35 (light); dark uses frame_over_mica 0.5 for the black shade or 0.35 for gray; tinted 0.78 (:956-974, :523-533).
- `paper()` (list, preview): `0xffffff` @0.7 (light), `0x1e1f22` or `0x3a3a3a` @0.45 (dark) (:982-995, :767).
- `card()`: white @0.8 (light) / @0.045 (dark) (:1032-1038).
- Opaque: `frame_solid()` for the collapsed sidebar opened on hover (:976-980); `overlay()` for menus, tooltips and dialogs: `0xffffff` / `0x222326` (:1043-1046, :435, :468), "always opaque" (:1236-1237).
- Mica-off palette: `frame()` and `paper()` return the solid ground and surface (:960-962, :984-986).

**Legibility**
- LIGHT ink alphas: muted 0.78, subtle 0.63, disabled 0.36, line 0.10, line_strong 0.22, wash 0.045, wash_strong 0.08, signal_wash 0.12 (:448-455).
- DARK ink alphas: 0.77 / 0.58 / 0.34 / 0.08 / 0.16 / 0.045 / 0.08 / 0.10 (:480-488).
- Selection: ink @0.10 (light), 0.11 or 0.13 (dark) (:999-1007, :527, :532).
- WCAG: `contrast()`, where "a translucent `fg` is composited onto `bg` first" (:718-722). `ACCENT_CONTRAST = 4.5` is enforced against both paper and frame (:568-569, :638-644). Subtle text on overlays gets +0.05 (:1083-1092).
- Under Mica the dark outline is dropped, because "drop shadows are painted as full rectangles under the element" (:1221-1235).

**Corners/border**
- `appears_transparent: true`, with the comment "the system keeps only the border and snap layouts" (main.rs:1965-1971). Corner attribute: not found.
- The 1px border is a `hairline(line())` BoxShadow (theme.rs:1216, :1233). Preview sheets use `border_1().border_color(line())` (:1024-1030).

**Startup/reveal:** popups are created with `show: false` and refreshed once their handle is known (popup.rs:501, :532-533). Main window: not found.

**Light/dark and fallback**
- `set_dark_frame` sets attr 20, but only when Mica is on (workspace.rs:618-622; win.rs:415-430). The reason given: "otherwise Mica keeps the system color and looks washed out".
- Below 22000 or with an env override: Solid and Opaque.

## 3. gelenmedya__rustyfiles (file manager)
**Material**
- `BackdropPref` (rf-app/src/workspace.rs:4890-4899):
  - Mica, MicaAlt.
  - Acrylic maps to `Blurred`.
  - `Solid` maps to `MicaBackdrop` with an opaque palette: "keep Mica under it so a switch back is instant".
  - `Glass` (the app's own DirectComposition engine) stays on Mica until the engine runs, then switches to `Transparent` (:4755-4761).
- Gate: `SYSTEM_BACKDROP_BUILD = 22621`, read from the registry value `CurrentBuild`. An unreadable build (0) counts as supported (rf-win/src/osver.rs:12-32). `RUSTYFILES_NO_BACKDROP` forces it off (rf-app/src/settings.rs:1133-1136).
- `window_material()` maps every non-transparent choice to `Opaque` when materials are unavailable (theme.rs:229-235). The main window may keep `Blurred` on Windows 10 (:239-244; settings.rs:1122-1128).
- Popups: `popup_material` gives `Transparent` (glass) or `Blurred` (theme.rs:248-250). Explorer-style menus use `Opaque` (fluent_menu.rs:504, flyout.rs:84).
- Before leaving glass the app sets `Opaque`, because "Transparent turns on a window accent policy that the Mica calls do not turn off" (workspace.rs:4804-4805).
- Win32 calls:
  - attr 20 (rf-win/src/window.rs:125-131).
  - attr 33 with ROUNDSMALL or ROUND (:133-151).
  - Dock and snip overlays: DONOTROUND, `DWMWA_BORDER_COLOR = DWMWA_COLOR_NONE`, NCRENDERING_POLICY disabled (dock/island.rs:136-139; snip/place.rs:153-157).
  - SWCA and DwmExtendFrameIntoClientArea: not found.

**Surfaces** (rf-app/src/theme.rs)
- Light: window_bg `F3F3F3` @0.30; titlebar, sidebar, address, toolbar and bar @0.0; content_bg `FFFFFF` @0.50; tile and tab_active `FFFFFF` @0.70 (:72-93).
- Dark: window_bg `202020` @0.40; content `141414` @0.45; tile `FFFFFF` @0.0512; tab_active @0.06 (:132-156).
- Opaque: `card_bg` `F9F9F9` / `2C2C2C` ("menus, dialogs and inputs", :17-18, :78, :140). `popup_fill` uses the same colors when there are no materials (:254-256). Solid and matte windows use window_bg `F3F3F3` / `202020` (:185-188, :220-223). The Black theme is fully opaque (:101-124).
- Over glass, washes come from a setting; the default is half the Mica strength (:198-210, :258-260).

**Legibility**
- Light: text `000` @0.90 / 0.62 / 0.45; border and divider 0.07; hover 0.04; pressed 0.07; selection 0.06; selection_border accent @0.55 (:82-92).
- Dark: text `FFF` @0.94 / 0.70 / 0.46; border 0.08; divider 0.07; hover 0.06; pressed 0.10; selection 0.085 (:144-155).
- WCAG or flatten helpers for window surfaces: not found. Contrast code exists only in shell/desktop/label_shadow.rs.

**Corners/border:** main window uses `TitleBar::window_options()` (lib.rs:376-382). Glass popups get small corners and panels get round corners (glass_watch.rs:175-177). The 1px border is the palette `border` (0.07 / 0.08).

**Startup/reveal:** main window: not found. Auxiliary windows open with `show: false` (snip/mod.rs:226, shell/switcher/mod.rs:119, shell/cover.rs:568).

**Light/dark and fallback**
- `set_dark_mode` runs when `(p.dark, window.appearance())` changes, because "GPUI resets it on a system theme switch" (workspace.rs:4850-4851, :4927-4936).
- Below 22621: solid palette, and popups paint their own fill. Issue 24, rf-win/src/osver.rs:3-8: legacy blur-behind "draws over the WHOLE window rectangle".

## 4. dstrml__wu (code editor)
**Material**
- Theme field `background.appearance` (crates/settings_content/src/theme.rs:565, :1344-1346), mapped in settings/src/content_into_gpui.rs:50-54. Default `Opaque` (theme/src/fallback_themes.rs:112).
- assets/themes/wu/wu.json: "Wu Dark" is `opaque` (:10), "Wu Dark Translucent" is `blurred` (:429), "Wu Light" (:845) has no key and uses the default. No bundled theme uses Mica.
- Applied at open (wu/src/wu.rs:367; settings_ui/src/settings_ui.rs:844) and to every window on every settings change (wu/src/main.rs:674-683).
- gpui_windows/src/window.rs:891 uses the upstream mapping: Mica needs 22621 (:1571-1573), SWCA needs 17763 (:1595). The renderer clears to `[1.0;4]` for Opaque and `[0.0;4]` otherwise (directx_renderer.rs:373-376). App-level DWM calls: not found.
- The renderer has its own backdrop-blur pass (gpui_windows/src/directx_renderer/backdrop.rs:244), used once: `window.paint_backdrop_blur(bounds, Corners::all(px(11.)), px(16.))` (agent_chat/src/chat_view/completion.rs:797).

**Surfaces** ("Wu Dark Translucent")
- Alpha 0 (`#00000000`): title_bar, tab_bar, tab.inactive, panel, status_bar, editor.gutter, terminal (wu.json:462-497).
- Semi-transparent: background `#0d0d0dcc` (0.80), editor and toolbar `#0a0a0a73` (0.45), tab.active `#ebebeb1a` (0.10) (:430, :465, :468, :482).
- Near-opaque: surface `#1b1b1deb` (0.92), elevated_surface `#1b1b1df5` (0.96) (:437-438).
- The opaque palette is a separate theme entry ("Wu Dark", `#181a21` throughout, :11-78).
- The buffer header paints its background only if the window or the color is opaque (editor/src/element/header.rs:699-704). `theme_is_transparent()` (ui/src/styles/appearance.rs:14-21) has no callers.

**Legibility:** border `#ffffff1a` (0.10), border.variant `#ffffff0f` (0.06), hover `#ffffff1c` (0.11), selected `#8b7cf62e` (0.18); text `#e8e8ea` is opaque (wu.json:431-453). Contrast solver: not found.

**Corners/border, startup/reveal:** app-level code: not found.

**Light/dark and fallback:** attr 20 comes only from upstream gpui_windows/src/util.rs:142. A Mica theme does nothing below 22621 (window.rs:1573); app fallback: not found.

## 5. lassejlv__termy (terminal)
**Material**
- Derived from config: `background_opacity: 1.0` and `background_blur: false` by default (crates/core/src/config_core/types.rs:507-509). `resolve_background_appearance` then decides (desktop_app/src/terminal_view/appearance.rs:81-131):
  - opacity >= 1: `Opaque`.
  - blur off: `Transparent`.
  - Windows >= 22621: `MicaAltBackdrop`, "smoother than the legacy acrylic accent (which lags while the window is dragged)" (:106-112).
  - older Windows: `Blurred`.
- Gate: `WINDOWS_11_SYSTEM_BACKDROP_BUILD: u32 = 22621` (native_sdk/win32_window.rs:25-27; appearance.rs:40-47).
- Applied only when it changes (terminal_view/mod.rs:3084-3094; settings_view/style.rs:50-58).
- Win32 calls: attr 20 (win32_window.rs:174-187) and attr 35 (:191-201). SWCA, attr 33 and DwmExtendFrameIntoClientArea: not found.

**Surfaces**
- Tab-strip chrome: `chrome.a = 0.0` over Blurred, Mica or MicaAlt; otherwise the background is mixed 0.06 toward the foreground at the surface alpha (tab_strip/style.rs:113, :175-192, :240-257).
- Every surface alpha is multiplied by the opacity (appearance.rs:137-143). Overlay panels keep at least 0.72 × base (`OVERLAY_PANEL_ALPHA_FLOOR_RATIO`, terminal_view/constants.rs:84; appearance.rs:145-163; the settings panel also uses 0.72, settings_view/mod.rs:80).
- Strokes and dividers are forced to `a: 1.0` (appearance.rs:180-200), with `TAB_STROKE_FOREGROUND_MIX = 0.16` (tab_strip/constants.rs:56).

**Legibility:** WCAG `contrast_ratio` composites onto the backdrop first (`composite_over`), and `contrasting_text_for_fill` picks the readable text color (settings_view/style.rs:408-449). There is an increased-contrast option (ui/native.rs:191).

**Corners/border:** tabs use `rounded_t(8)` (tab_strip/style.rs:91). Window corners: not found.

**Startup/reveal:** not found.

**Light/dark and fallback:** `native_titlebar::sync` polls whether the system is dark, then sets attr 20 and attr 35 = terminal background (native_titlebar.rs:263-288). On Windows 10 with blur on, the window uses SWCA `Blurred`.

## 6. l0ng-ai__tty7 (terminal; sparse clone)
gpui comes from the `l0ng-ai/zed` fork, branch tty7 (Cargo.toml:494-497). It is not in the clone. `AcrylicBackdrop` is a variant of that fork.

**Material**
- Setting `WindowBackdrop {Auto, Blur, Mica, MicaAlt, Acrylic, Off}`, resolved in src/ui/theme.rs:327-378:
  - Mica, MicaAlt and Acrylic (DWMSBT_TRANSIENTWINDOW) need >= 22621; below that they use `Blurred` on >= 17763 and `Transparent` otherwise.
  - Blur needs >= 17763, otherwise `Transparent`.
- Build via RtlGetVersion; a failure counts as 0 (:415-435). The dropdown lists only what the build supports, plus the stored value (:388-407, :545-551).
- A fully opaque fill without a material is reported as `Opaque` so ClearType still works (:574-586). DWM materials are not routed through Opaque: "switching away from one through `Opaque` would not clear the system backdrop type" (:571-573).
- The applied appearance is cached per window, because gpui re-sets attr 38 and does `SWP_FRAMECHANGED` on every call (:609-639). It is applied at open (ui/windows.rs:867, ui/settings_window.rs:40) and to all windows (theme.rs:692-707).

**Surfaces**
- Window background alpha is `SYSTEM_MATERIAL_OPACITY = 0.82` when a material is explicitly active. Auto never counts as a material (:316-319, :471-499).
- Sidebar, right panel, SFTP panel and rail use `base.alpha(0.15)` under a material, because "its own alpha stacks (src-over)" (:501-537).
- Opaque: full-window overlays (settings, opened file, diff view) via `window_background_opaque` (:226-252); popover and button fills are opaque rgb (:762-776).

**Legibility:** `bisect_contrast` runs 12 steps toward a target ratio (presets.rs:502-522), using `relative_luminance` and `contrast` (:624, :642). Lane colors are precomputed once per theme (theme.rs:720-723).

**Corners/border:** `WindowDecorations::Client` (ui/windows.rs:866). DWM corner calls in the repo: not found.

**Startup/reveal:** not found.

**Light/dark and fallback:** attr 20 does not appear in the repo; `sync_native_appearance` is macOS-only (theme.rs:1075-1099). The follow-system preset switch is at :305-314. Below 22621: `Blurred`, or `Transparent` below 17763.

## Patterns across these repos
1. **One build threshold.** Every vendored or upstream gpui applies attr 38 only when `dwBuildNumber >= 22621`. Argus, rustyfiles, termy and tty7 gate Mica at 22621 in the app too. Arke gates at 22000, so on 22000-22620 it paints a translucent palette over a window with no material. The build is read via RtlGetVersion (Argus, Arke, tty7, gpui) or the registry `CurrentBuild` (rustyfiles).
2. **Two fallback styles below 22621.** Argus, Arke and rustyfiles fall back to `Opaque` plus a solid palette. termy, tty7 and rustyfiles (main window, opt-in) fall back to SWCA `Blurred` (needs >= 17763). rustyfiles keeps popups off legacy blur because it covers the whole frame.
3. **Clear chrome.** The title bar, tab strip and sidebar are alpha 0: Argus, rustyfiles, wu Translucent, termy. Arke instead keeps a frame wash of 0.35-0.78 for readability on bright wallpapers.
4. **One translucent content layer.** Values range 0.45-0.82: rustyfiles 0.45/0.50, Arke paper 0.45/0.7, Argus card 0.5 (light) / 0.035 (dark), wu editor 0.45, tty7 0.82.
5. **Opaque floating surfaces.** Menus, flyouts, dialogs and full-window overlays are made opaque: Argus `surface a=1`, Arke `overlay()`, rustyfiles `card_bg` / `popup_fill`, tty7 `overlay_background`. The stated reason, in Argus theme.rs:225-226 and Arke theme.rs:1221-1223: shadows are painted as full rectangles under the element and darken translucent surfaces.
6. **Explicit src-over stacking math.** tty7 uses a fixed 0.15 for panels. Argus has `on_card` and `remainder`. termy uses composite-first contrast.
7. **Contrast solving that composites translucent foregrounds first.** Arke (4.5:1 accent), termy, tty7 (bisection). Argus, rustyfiles and wu have none for UI surfaces.
8. **Apps re-apply attr 20 themselves.** Argus on WM_SETTINGCHANGE, Arke when Mica is on, rustyfiles on a `(dark, appearance)` change, termy by polling plus caption color (attr 35).
9. **Explicit corner preference only on popups and panels.** Argus `DWMWCP_ROUND`, rustyfiles ROUNDSMALL / ROUND. Main windows keep the DWM frame. Only rustyfiles sets `DWMWA_BORDER_COLOR`, to `COLOR_NONE` on overlay windows.
10. **Switching materials leaves residue.** Transparent to Mica keeps the SWCA accent unless Opaque is set first (rustyfiles). Leaving Mica through Opaque does not clear the backdrop type (tty7).
11. **No DwmExtendFrameIntoClientArea.** None of the six repos calls it.
12. **Little reveal-flash handling.** Argus lays the flyout out at alpha 0 and pre-sizes a hidden main window. Arke and rustyfiles create popups with `show: false`. No repo cloaks the main window for reveal.

## Limitations
- tty7 is a sparse clone, and its gpui fork (AcrylicBackdrop, Windows dark-mode handling) is not in it.
- `gpui_kit` (`TitleBar::window_options`, used by Argus and rustyfiles) is an external crate, so its frame and decoration internals were not inspected.
