# gpui-component theme / window-material survey (8 repos)

Root: `$PI_SCRATCH_DIR\research\repos\`. Paths below are relative to each repo. Read-only; nothing built.

## 1. zerx-lab__FluxDown (download manager)
- **Construction**: own token engine (`crates/theme`), not ThemeRegistry JSON. Builtins are Rust palettes (`crates/theme/src/builtin.rs:298-385`): DefaultDark, DefaultLight, MidnightBlue, Nord, WarmLight, Graphite, GraphiteLight (7, plus a "Default" light/dark pair). User theme documents (`ThemeDocument`, schema at `website-v2/public/schemas/gpui-theme.v2.json`) can `extends: "builtin:*"`. Install: `ComponentTheme::change(mode, None, cx)`, then `project_onto_kit(...)`, then `ComponentTheme::sync_base(cx)` (`crates/theme/src/manager.rs:415-417`).
- **Mode and selection**: `appearance.theme_mode` is `system|light|dark` (`native/protocol/src/settings.rs:315`). Separate `dark_theme`/`light_theme`, `color_scheme`, `custom_color` (accent) preferences (`manager.rs:284-286`). System mode uses `cx.window_appearance()` (`manager.rs:38`), re-synced through `observe_window_appearance` (`crates/app/src/windows/mod.rs:161`).
- **Default palettes** (`builtin.rs:300-323`):
  - Dark: bg `0x1C1C1E`, surface1 `0x2C2C2E`, surface2 `0x3A3A3C`, text `0xF5F5F7`, secondary text `0xA1A1A6`, border `0x48484A`, accent `0x3B82F6`.
  - Light: bg `0xF8F9FA`, surface1 `0xFFFFFF`, surface2 `0xF1F3F5`, text `0x09090B`, secondary text `0x71717A`, border `0xE4E4E7`.
- **Role overrides** (`manager.rs:480-535`, via a `paint!` macro that writes both the legacy field and `tokens.*`):
  - background, sidebar, table, table_even and table_head all = `surface`.
  - title_bar = `extended.chrome`; title_bar_border = `hairline`; sidebar_border = `border`.
  - list_active and table_active = `accent`; list_active_border = `primary`; table_hover = `row_hover`; table_head_foreground = `text_tertiary`.
  - table_row_border = `surface.opacity(0.)`, i.e. the download table draws no grid lines (comment at `:477`).
  - `focus_ring = false`; font_size = `typography.sm` (13 px).
- **Alpha**:
  - accent = `accent.alpha(26|38|46 / 255)` (`builtin.rs:19-21,415`).
  - selection = `accent.alpha(0.3)` (`:422`).
  - dragBorder `RefAlpha(primary, 0.65)`, dropTarget `0.2`, progressTrack `RefAlpha(mutedForeground, 0.16)` (`registry.rs:393-427`).
  - Graphite rowHover is RGBA `0xECEEF20A` (dark) / `0x1E1F240A` (light) (`builtin.rs:161,172`).
- **Derived chrome tokens** (`registry.rs:345-374`):
  - chrome: dark = mix(background, surface, 0.35); light = mix(background, muted, 0.35).
  - hairline = mix(border, surface, 0.35/0.4).
  - rowHover = mix(muted, surface, 0.45/0.4).
  - navSelected: dark = mix(muted, surface, 0.15); light = mix(muted, border, 0.65).
  - navHover = mix(chrome, navSelected, 0.55).
- **Window**:
  - `TitleBar::window_options()` plus `window_decorations = Some(WindowDecorations::Client)` (`crates/shell/src/lib.rs:73-78`); `app_owns_titlebar_drag` is asserted in tests (`:110`).
  - gpui-component `TitleBar::new()` (`crates/shell/src/view.rs:351,621`).
  - Custom `FixedSizeTitleBar` (minimize and close only) for non-resizable windows (`crates/shell/src/window_controls.rs:106`).
  - Popups use `WindowKind::PopUp`/`Floating` (`crates/app/src/windows/progress.rs:93`).
  - No `WindowBackgroundAppearance`, no `window_border`, no DWM calls.
- **Transparency/blur/mica**: not found. The only blur is shadow blur radius (`builtin.rs:236-242`: "persistent surfaces have no shadow; only floating layers lift").
- **Layering**: chrome-tinted frame vs white/`surface` content. Title bar, activity rail, sidebar and status bar use `chrome` with a 1 px `hairline` edge (`crates/shell/src/chrome.rs:773-775,811-813`; `crates/downloads/src/components/sidebar.rs:1035`). Content and table use `surface`. Rows have no borders and use `row_hover` on hover (`task_table.rs:3200`). The resizable handle is also hairline (`manager.rs:422`).
- **Legibility**:
  - In light mode the primary is lifted to 4.5:1 (`resolve.rs:126-127`, `extended.rs:14`: `MIN_PRIMARY_CONTRAST = 4.5`).
  - accentText = contrast(primary, surface, 4.5); navSelectedIcon is 3:1 against navSelected (`registry.rs:382,388`).
  - textTertiary = mix(mutedForeground, surface, 0.38).
  - Primary foreground is chosen by WCAG luminance (`builtin.rs:455-474`).
- **Motion**: only a pulse dot (`crates/settings/src/sections/cloud_push.rs:1461`) and a dialog form fade (`cloud_channel_dialog.rs:883-885`, `ease_in_out`). No page/route transitions. Main window can open unfocused (`crates/app/src/windows/main.rs:39-50`). No reveal-after-first-frame.

## 2. vicanso__zedis (Redis GUI)
- **Construction**: ThemeRegistry plus 6 embedded JSONs (`assets/themes/{ayu,catppuccin,flexoki,gruvbox,hybrid,tokyonight}.json`, `#[include = "themes/*.json"]` at `src/assets.rs:17`). Named themes go through `Theme::global_mut(cx).apply_config(&config)` (`src/window_setup.rs:139`). Otherwise `restore_default_themes` then `Theme::change(mode, None, cx)` (`:164-165`).
- **Selection**: Light/Dark/System plus any registry theme from a title-bar submenu (`src/views/title_bar.rs:162-179`). System mapping: Light|VibrantLight → Light, anything else → Dark (`window_setup.rs:220-224`). Applied before the first window to avoid a flash (`:160-163`).
- **Overrides**: only on the stock default configs (`window_setup.rs:193-198`):
  ```
  cfg.colors.primary = Some("#1f6feb".into());
  cfg.colors.primary_foreground = Some("#ffffff".into());
  cfg.colors.primary_hover = Some("#1a5ec8".into());
  cfg.colors.primary_active = Some("#1753b0".into());
  ```
  All other roles are stock. Notifications move to BottomRight (`:45-49`).
- **Alpha**: in views only, e.g. `muted.opacity(0.3)` cards (`views/workbench/results.rs:100`), `warning.opacity(0.1)`, sidebar `pressed_bg: selected_bg.opacity(0.8)` (`views/sidebar.rs:794`), scrim `hsla(0,0,0,0.4)` (`helpers/color.rs:105-107`). The theme JSONs carry 8-digit hex, e.g. `"list.active.background": "#7aa2f722"` (`tokyonight.json:21`).
- **Window**:
  - Mac/Windows: `TitlebarOptions{appears_transparent: true, traffic_light_position: (9,9)}`, `app_owns_titlebar_drag: true` (`src/lib.rs:519-545`).
  - Linux keeps server decorations (`:515-518`).
  - Own `ZedisTitleBar` wraps gpui-component `TitleBar::new()` with `.bg(cx.theme().tokens.title_bar)` (`views/title_bar.rs:622-629`).
  - No WindowBackgroundAppearance or DWM.
- **Transparency/blur**: not found.
- **Layering**: card = background `lighten(1.0)` in dark / `darken(0.02)` in light (`helpers/color.rs:21-49`). Tab-bar trough adds `darken(0.22)`/`darken(0.04)` when `tab_bar` is within 0.03 L of bg (`:54-69`). Sidebar uses `theme.sidebar`/`sidebar_accent`/`list_hover`/`sidebar_border` (`views/sidebar.rs:786-802`).
- **Legibility**: hand-tuned per-mode colors "for 4.5:1+ contrast in both themes", e.g. healthy `0x69b083`/`0x1a7f37` and sidebar accent `0x6b95c4`/`0x2563eb` (`helpers/color.rs:71-79`). muted_foreground is stock.
- **Motion**: tokens 120/180/240/360 ms with reduce-motion collapse (`helpers/motion.rs:24-41`), but no `with_animation` callers found. Reveal: on macOS the window is created hidden (`show: cfg!(not(target_os="macos"))`) and `on_next_frame(... activate_window())` runs after the first themed frame (`lib.rs:534,561`). Windows shows immediately because hidden windows get no WM_PAINT (`:524-531`).

## 3. feigeCode__navop (DB/server workspace)
- **Construction**: ThemeRegistry with bundled JSON (`themes/*.json`: 23 files incl. custom `themes/navop.json`) via `load_themes_from_str` (`crates/core/src/themes.rs:34-41`). User import goes into the config dir `themes/` (`:43-90`). Install: set `light_theme`/`dark_theme`, `Theme::change(mode, None, cx)`, `focus_ring=false`, custom accent, font size, window opacity, `refresh_windows` (`:146-165`).
- **Selection**: separate light/dark theme names (defaults "Navop Light"/"Navop Dark"), a mode setting plus `auto_switch_theme` (`:119-143`), and an optional custom accent (`:207-238`: primary, accent, ring, list/table_active_border, sidebar_primary, `selection.a = 0.25`).
- **Navop Light/Dark values** (`themes/navop.json`):
  | Role | Light | Dark |
  |---|---|---|
  | background | `#ffffff` | `#171717` |
  | sidebar.background | `#f3f4f6` | `#1e1e1e` |
  | title_bar | `#ffffff` | `#171717` |
  | list.background | `#ffffff` | `#0a0a0a` |
  | table.background | `#ffffff` | `#0a0a0a` |
  | popover | `#ffffff` | `#262626` |
  | border | `#e5e7eb` | `#404040` |
  | muted.foreground | `#6b7280` | `#a3a3a3` |
  | primary | `#3b82f6` | `#3b82f6` |
  | list.active | `#3b82f610` | `#3b82f620` |
  | table.active | `#bfdbfe33` | `#1d4ed833` |
  | table.row.border | `#e5e5e5b3` | `#262626b3` |
  | window.border | `#e5e5e5` | `#262626` |

  Lines: Light `:11-68`, Dark `:97-154`.
- **Window**: `window_background: WindowBackgroundAppearance::Transparent`, `TitleBar::title_bar_options()`, Linux `WindowDecorations::Client`, `app_owns_titlebar_drag` (`crates/navop_app/src/lib.rs:245-259`). Popup windows are the same (`crates/core/src/popup_window.rs:631-643`). Popups use gpui-component `TitleBar::new()` (`:807`).
- **Transparency (yes, plain alpha, no blur)**: a user setting `window_opacity` in [0.5, 1.0], default 1.0 (`crates/core/src/settings.rs:1650-1652,2103-2104`). Applied only to `theme.background` (`themes.rs:167-185`):
  ```
  Theme::update(cx, |theme| {
      theme.background = theme.background.alpha(opacity);
  });
  ```
  The doc comment explains why element `opacity` is avoided: it would fade text. The terminal canvas follows the same rule (`crates/terminal_view/src/theme.rs:201-211`). The docked sidebar deliberately uses `cx.theme().background`, so it goes translucent with the content (test `persistent_connection_sidebar/mod.rs:381-389`). No Blurred/Mica/DWM.
- **Layering**: color steps from the JSON (sidebar `#f3f4f6` vs bg `#fff`; dark `#1e1e1e` vs `#171717`). The docked tree takes the bg and a single resize-handle divider, with no `border_r` (test above).
- **Legibility**: muted fg `#6b7280` on `#fff`. Custom accent picks its foreground by L > 0.62 (`themes.rs:14,214-218`).
- **Motion**: AI chat only (panel slide `ease_out_quint`, message entrance: `crates/ai_chat_view/src/motion.rs:34-36`, `message_view.rs:325-327`), skeleton pulse, marquee. No page transitions found.

## 4. 0xErwin1__dbflux (DB client)
- **Construction**: in-code palettes in its own `bolt` crate. `Theme::change(palette.mode, window, cx)`, then `apply_palette`, `sync_component_tokens`, `Theme::sync_base` (`crates/bolt/src/theme.rs:77-85`). Two palettes, Dark and Light (`:428-513`). The accent comes from an installed `Brand` (`crates/bolt/src/brand.rs`). No theme JSON.
- **Mode**: System/Dark/Light; System maps Dark|VibrantDark → Dark (`theme.rs:93-99`), re-applied on `observe_window_appearance` (`crates/dbflux/src/main.rs:576-595`).
- **Palette values** (`theme.rs:434-508`):
  | Role | Dark | Light |
  |---|---|---|
  | background | `0x09090B` | `0xF6F4F7` |
  | panel | `0x100F13` | `0xFFFFFF` |
  | raised | `0x1A181E` | `0xEEEAF0` |
  | line | `0x232128` | `0xE3DEE6` |
  | line_strong | `0x37333D` | `0xCBC4D1` |
  | row_divider | `0x18161B` | `0xF0ECF2` |
  | text strong/body/muted | `F7F4F7`/`C6C3CC`/`8E8996` | `141118`/`3B3740`/`6B6572` |
  | hover_wash | `FFFFFF @0.04` | `141118 @0.04` |
  | alternating row | `@0.012` | `@0.015` |
  | overlay | `050507 @0.62` | `1E1423 @0.28` |
- **Role map** (`theme.rs:558-648`):
  - background, sidebar, list, title_bar, tab_bar, table_head = `background`.
  - popover, table, tab_active, list_head = `panel`.
  - border, window_border, sidebar_border, title_bar_border = `line`.
  - muted = `line_strong`; muted_foreground = `text_muted`.
  - accent = `hover_wash`; list_active = `selected_item_wash`; `list_active_border = transparent_black()`.
  - selection = `text_selection`; scrollbar thumb = `text_muted @0.30/0.50`.
- **Alpha washes** (`brand.rs:31-40`): selected_row `0.07`, selected_item `0.12` dark / `0.14` light, text_selection `0.25`, drop_target `0.10`.
- **Window**: `TitlebarOptions{title, ..Default}` (native frame on Win/mac). Client decorations on Linux only (`crates/dbflux_ui_base/src/platform.rs:65-97`; `main.rs:480-492`). No gpui-component `TitleBar`, no WindowBackgroundAppearance, no DWM.
- **Transparency/blur**: not found.
- **Layering**: background → panel → raised steps plus 1 px `line` borders. Selected tree row = tint wash plus a 2 px tint bar (`theme.rs:640-643`).
- **Legibility**: text_muted `0x6B6572` on light bg `0xF6F4F7`. On light surfaces "the accent doubles as the text accent" (`theme.rs:491-492`).
- **Motion**: only loading-state/chamfer primitives (`crates/bolt/src/primitives/{loading_state,chamfer}.rs`). No page transitions or reveal handling found.

## 5. tahayvr__omarchist (Linux)
- **Construction**: one embedded ThemeSet `ui_themes/theme.json` ("Omarchist Light"/"Omarchist Dark", `radius: 0`) loaded into `light_theme`/`dark_theme`, then `Theme::change` (`src/system/ui_theme_watcher.rs:399-463`). A third "omarchy" mode builds a ThemeConfig from the desktop's `colors.toml` (`:127-396`), polled every 1 s (`:16`).
- **Selection**: setting `theme_mode` = `light|dark|omarchy` (`:437-462`). No OS-appearance observer found.
- **Values**:
  - Light (`ui_themes/theme.json:17-48`): bg `#eaeaea`, border `#d4d4d4`, muted.bg `#f5f5f5`, muted.fg `#737373`, popover `#fafafa`, primary `#171717`, title_bar `#eaeaea`, list.active `#e5e5e522`.
  - Dark (`:251-282`): bg `#0a0a0a`, border `#262626`, muted.fg `#a3a3a3`, popover `#0a0a0a`, title_bar `#0a0a0a`, list.active `#26262622`.
  - No sidebar keys.
- **Omarchy derivation** (`ui_theme_watcher.rs:204-276`): muted bg ±0.05 L, popover ±0.02, border ±0.10/0.12, secondary ±0.08, muted_fg = fg darken/lighten 0.30. Alpha: `list.active = secondary+"22"`, `list.even = muted+"99"`, `scrollbar.thumb = fg+"4c"`, `tab.background = bg+"00"`.
- **Window**: `TitleBar::title_bar_options()`, `show: true`, `focus: true` (`src/main.rs:274-285`). gpui-component `TitleBar::new()` (`src/ui/menu/title_bar.rs:45`). No WindowBackgroundAppearance or DWM.
- **Transparency/blur**: not in the app window. The `blur` hits are Hyprland config pages (`src/ui/config_page/pages.rs:463-667`).
- **Layering**: lightness steps from bg (above), zero radius.
- **Legibility**: muted fg `#737373` on `#eaeaea` (light).
- **Motion**: a recording-dot pulse (`src/ui/keybinds_page/keystroke_input.rs:250-255`). No page transitions found.

## 6. pandaligx__RTX-FG-Manager (Windows tool)
- **Construction**: ThemeRegistry plus 21 embedded stock gpui-kit JSONs (`rust/assets/themes/*.json`; list at `rust/src/ui_themes.rs:123-166`). Applied with `apply_config_with_defaults`, which resets fonts, radii, shadow and highlight from the Kit default first and then calls `apply_config` (`:259-283`). The font is forced to `"Microsoft YaHei UI"` (`:9`).
- **Selection**: Choice = System or Named{mode,name}, with persisted `ui_theme_light`/`ui_theme_dark` (defaults "Default Light"/"Default Dark") (`:49-108`). System dark comes from the registry `AppsUseLightTheme` (`rust/src/win.rs:494`). Supports preview without persisting.
- **Overrides**: none to roles. Only `focus_ring=false` and `font_size*8/7` (`rust/src/ui.rs:319-321`). The brand color `0x76b900` is kept as an identity token, not a role (`ui_themes.rs:12-14`).
- **Window**: `TitlebarOptions{appears_transparent: true}` (`ui.rs:4740-4753`) with gpui-component `TitleBar::new()` (`ui.rs:4554`, `ui_settings.rs:26`). The app sets no WindowBackgroundAppearance. The vendored `vendor/gpui_fast_windows` implements Blurred as Win11 acrylic (`DWMSBT_TRANSIENTWINDOW`, `src/fast/backdrop.rs:1-31`) and Mica via `DWMWA_SYSTEMBACKDROP_TYPE` (`src/window.rs:927-951,1644-1653`), but the app never requests it.
- **Transparency/blur in app**: not found.
- **Layering/legibility**: stock theme roles.
- **Motion**: a 4 s `repeat_synced` flowing-accent border on one entry button (`ui.rs:1040-1046`; B-spline palette blend `ui_themes.rs:24-45`). No page transitions found.

## 7. ELY-Open-Source__ElyOffice
- **Not longbridge gpui-component**: it uses its own fork `ely-gpui-component` (git `ELY-Open-Source/Ely-GPUI-Components`, `Cargo.toml:23`), which is not cloned, so stock palette values are not available here.
- **Construction**: `Theme::set_palette(mode, Some(palette))` and `Theme::set_mode`/`set_mode_now` (`crates/ely-ui/src/theme/mod.rs:134-182`). Appearance is System/Light/Dark (env `ELY_APPEARANCE` overrides) plus optional Omarchy sync. System follows `observe_window_appearance` (`:128-131`).
- **Omarchy palette builder** (`crates/ely-ui/src/theme/omarchy/palette.rs:12-120`): surfaces are `bg.mix(fg, t)` with surface 0.04, overlay 0.06, sunken 0.08, hover 0.08, active 0.14, border 0.16, border_strong 0.32. Text is lifted until it reaches 4.5:1 on every surface and focus marks reach 3:1 (`lift()`, `:37-46`). fg_muted = fg.mix(bg, 0.25), fg_subtle = mix 0.45, both contrast-lifted.
- **Window**: `appears_transparent: true`, `WindowDecorations::Client`, `inactive_frame_interval: 250 ms` (`crates/ely-ui/src/run.rs:217-231`). Own `shell::TitleBar` (`crates/ely-ui/src/shell/mod.rs:153`). The DWM call is `DWMWA_CLOAK` only, used for a hidden-start/reveal (`shell/scene.rs:146-177`).
- **Transparency/blur**: not found.
- **Motion**: a startup "scene" (icon scale+fade, name fade, repeating bar sweep, leave fade, `scene.rs:74-136`). Reduced motion honors settings and the software renderer (`theme/mod.rs:94-119`). The window stays cloaked until the document is ready or a timeout fires (`run.rs:201-213`).

## 8. vicanso__cyberapi (API tool)
- **Construction**: stock Kit default themes only. No theme JSON or registry use found. `Theme::sync_system_appearance(Some(window))` or `Theme::change(Light|Dark, Some(window))` (`src/root.rs:376-381`), re-applied via `observe_window_appearance` (`:97-99`). Mode setting is System/Light/Dark (`src/views/settings.rs:51`).
- **Overrides** (`root.rs:388-400`):
  ```
  let accent = try_parse_color(if dark { "neutral-600" } else { "neutral-200" })...;
  theme.accent = accent;
  if dark { theme.overlay = hsla(0., 0., 0., 0.55); }
  ```
  The reason given: the stock dark ghost-hover halves the accent alpha, so the wash vanished. Notifications go BottomRight.
- **Alpha in views**: tree border `foreground.opacity(0.12)`, guide `0.1` (`views/workspace/tree_panel.rs:464,627`); inactive tab label `muted.opacity(0.88)`, close icon `0.45` (`tabs.rs:291,363`).
- **Window**: `..TitleBar::window_options()` plus `TitleBar::title_bar_options()`, `show: cfg!(not(target_os="macos"))` (`src/lib.rs:365-384`). gpui-component `TitleBar::new()` (`root.rs:694`). No WindowBackgroundAppearance or DWM.
- **Transparency/blur**: not found.
- **Layering**: stock roles. Tree panel `popover` bg (`tree_panel.rs:111`), rows `list_active`/`list_hover`, `muted` blocks.
- **Motion**: no `with_animation` found. Hidden-until-ready only on macOS.

## Patterns across these repos
1. **Opaque windows.** None of the 8 apps requests Blurred, Mica or acrylic.
   - Only navop changes `WindowBackgroundAppearance` (to `Transparent`). It fakes "window transparency" by setting the alpha of the single `theme.background` role from a 0.5–1.0 user slider (default 1.0). It keeps text, controls and popovers opaque and avoids element `opacity()` because that fades content.
   - RTX vendors a Windows backend that maps Blurred to acrylic/Mica, but the app leaves it unused.
2. **Theme construction.** Two schools:
   - (a) ThemeRegistry plus stock or forked JSON themes, with the user picking separate light and dark names (zedis, navop, RTX, omarchist).
   - (b) Hand-written in-code palettes, with `Theme::change(mode)` first and then direct role writes, plus a token resync (`sync_base` / `sync_component_tokens`) (FluxDown, dbflux).
   - Every app keeps System/Light/Dark, resolved via `window_appearance` plus `observe_window_appearance`, except RTX (registry) and omarchist (setting only).
3. **Surface layering is lightness steps plus hairlines, not shadows.**
   - FluxDown: chrome frame (bg mixed 35 % toward surface/muted) with white or `surface` content, 1 px hairline edges, and no table row lines.
   - dbflux: background → panel → raised.
   - zedis: card = lighten/darken by about 0.02–0.04 L.
   - ElyOffice: bg mixed with fg by 0.04/0.06/0.08/0.14.
   - Shadows are reserved for floating layers (FluxDown says so explicitly).
4. **Alpha is used for washes, not surfaces.**
   - Selection/active: accent at 0.07–0.30 (dbflux 0.07/0.12/0.14/0.25; FluxDown 0.10/0.15/0.18 accent bg and 0.3 selection; navop 0.25 custom selection).
   - Hover: white or black at 0.04.
   - Scrims: 0.28–0.62.
   - Stock JSONs use 8-digit hex for list.active, muted and scrollbar.
5. **Brand primary over neutral defaults.** zedis, cyberapi and FluxDown all note that the stock primary/accent is neutral black/white. They override primary with its hover/active shades, and in cyberapi's case the accent hover wash, because the stock dark ghost-hover nearly disappears.
6. **Contrast handling exists only in the bespoke engines.**
   - FluxDown lifts primary to 4.5:1 in light mode, requires accentText ≥ 4.5 and nav icons ≥ 3:1.
   - ElyOffice lifts every text color to ≥ 4.5 on all surfaces.
   - zedis hand-tunes per-mode status colors.
   - The others rely on theme JSON muted foregrounds: `#6b7280`, `#737373`, `#6B6572`.
7. **Motion is sparse.**
   - No app has page or module transition animations.
   - Animation covers pulses, skeletons, chat entrance and ElyOffice's startup scene, all gated by `reduce_motion` where handled (zedis, ElyOffice, navop).
   - Reveal: hidden-until-first-frame on macOS only (zedis, cyberapi), because Windows hidden windows never get WM_PAINT. ElyOffice instead uses `DWMWA_CLOAK` on Windows.

## Limitations
- ElyOffice's stock palettes live in the external `Ely-GPUI-Components` fork, which is not cloned. Only its Omarchy palette builder was read.
- The 21 RTX theme JSONs, and zedis's 6, are stock gpui-kit themes. Their per-role values were not quoted except for samples.
