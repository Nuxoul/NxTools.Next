use gpui_kit::{App, Rgba};

pub(crate) mod material;
#[cfg(test)]
mod material_tests;

#[cfg(test)]
use crate::ui::typography::TEST_UI_FONT_FAMILY;
pub(crate) use crate::ui::typography::{UI_FONT_FAMILY, UI_MONO_FONT_FAMILY};

#[derive(Clone, Copy)]
pub struct Theme {
    pub canvas: Rgba,
    pub surface: Rgba,
    pub surface_raised: Rgba,
    pub surface_hover: Rgba,
    pub surface_pressed: Rgba,
    pub border_subtle: Rgba,
    pub border: Rgba,
    pub border_active: Rgba,
    pub text: Rgba,
    pub text_secondary: Rgba,
    pub text_muted: Rgba,
    #[allow(dead_code)] // Later module actions have not all migrated disabled presentation.
    pub text_disabled: Rgba,
    pub accent: Rgba,
    #[cfg(test)]
    pub accent_hover: Rgba,
    pub accent_soft: Rgba,
    pub accent_text: Rgba,
    pub success: Rgba,
    #[allow(dead_code)] // Warning surfaces retain their existing owner-specific presentation.
    pub warning: Rgba,
    pub danger: Rgba,
    pub danger_soft: Rgba,
    pub image_well: Rgba,
    pub window_border: Rgba,
    pub element_selected: Rgba,
    pub icon: Rgba,
    pub icon_muted: Rgba,
    pub focus_ring: Rgba,
}

const GRAPHITE_DARK: &str = include_str!("../assets/themes/nxtools-graphite-dark.json");
const GRAPHITE_LIGHT: &str = include_str!("../assets/themes/nxtools-graphite-light.json");
/// NxTools schemes: the defaults plus Mist, Pine and Sand, each light and dark.
const NXTOOLS_SCHEMES: &str = include_str!("../assets/themes/nxtools.json");
const DEFAULT_LIGHT: &str = "默认浅色";
const DEFAULT_DARK: &str = "默认深色";

fn default_colors(dark: bool) -> gpui_kit::component::ThemeColor {
    use std::sync::LazyLock;
    static COLORS: LazyLock<[gpui_kit::component::ThemeColor; 2]> = LazyLock::new(|| {
        let set: gpui_kit::component::ThemeSet =
            serde_json::from_str(NXTOOLS_SCHEMES).expect("bundled NxTools schemes");
        [DEFAULT_LIGHT, DEFAULT_DARK].map(|name| {
            let config = set
                .themes
                .iter()
                .find(|config| config.name == name)
                .expect("requested theme")
                .clone();
            let mut theme = gpui_kit::component::Theme::default();
            theme.apply_config(&std::rc::Rc::new(config));
            theme.colors
        })
    });
    COLORS[usize::from(dark)]
}

impl Theme {
    // Initial-view compatibility constructors share the requested bundled defaults.
    // Runtime rendering always projects the active public Kit theme.
    pub fn zed_dark() -> Self {
        Self::from_colors(&default_colors(true))
    }

    #[cfg(test)]
    pub fn light() -> Self {
        Self::from_colors(&default_colors(false))
    }

    fn from_colors(colors: &gpui_kit::component::ThemeColor) -> Self {
        Self {
            canvas: colors.background.to_rgb(),
            surface: colors.sidebar.to_rgb(),
            surface_raised: colors.popover.to_rgb(),
            surface_hover: colors.list_hover.to_rgb(),
            surface_pressed: colors.secondary_active.to_rgb(),
            border_subtle: colors.sidebar_border.to_rgb(),
            border: colors.border.to_rgb(),
            border_active: colors.list_active_border.to_rgb(),
            text: colors.foreground.to_rgb(),
            text_secondary: colors.tab_foreground.to_rgb(),
            text_muted: colors.muted_foreground.to_rgb(),
            text_disabled: colors.muted_foreground.to_rgb(),
            accent: colors.primary.to_rgb(),
            #[cfg(test)]
            accent_hover: colors.primary_hover.to_rgb(),
            accent_soft: colors.accent.to_rgb(),
            accent_text: colors.accent_foreground.to_rgb(),
            success: colors.green.to_rgb(),
            warning: colors.yellow.to_rgb(),
            danger: colors.red.to_rgb(),
            danger_soft: colors.danger.opacity(0.10).to_rgb(),
            image_well: colors.primary_foreground.to_rgb(),
            window_border: colors.window_border.to_rgb(),
            element_selected: colors.list_active.to_rgb(),
            icon: colors.foreground.to_rgb(),
            icon_muted: colors.tab_foreground.to_rgb(),
            focus_ring: colors.ring.to_rgb(),
        }
    }

    pub(crate) fn surface_on_canvas(self, surface: Rgba) -> Rgba {
        if self.canvas.a == 0.0 {
            surface
        } else {
            self.canvas.blend(surface)
        }
    }

    pub fn active(cx: &App) -> Self {
        let colors = &gpui_kit::component::Theme::global(cx).colors;
        let mut theme = Self::from_colors(colors);
        if cx
            .try_global::<UiThemeState>()
            .is_some_and(|state| state.material_style.is_frosted())
        {
            // Frosted palettes clear the sidebar roles for the system material; product cards
            // and their edges use the card and divider roles instead.
            theme.surface = colors.group_box.to_rgb();
            theme.border_subtle = colors.border.to_rgb();
        }
        theme
    }
}

pub(crate) fn overlay_background(cx: &App) -> gpui_kit::Background {
    let theme = gpui_kit::component::Theme::global(cx);
    if cx
        .try_global::<UiThemeState>()
        .copied()
        .unwrap_or_default()
        .material_style
        .is_frosted()
    {
        theme.tokens.popover.background
    } else {
        theme.tokens.background.background
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct UiThemeState {
    pub preference: nxtools_core::ThemePreference,
    pub material_style: nxtools_core::MaterialStyle,
    pub tuning: nxtools_core::MaterialTuning,
    pub system_dark: bool,
}
impl Default for UiThemeState {
    fn default() -> Self {
        Self {
            preference: nxtools_core::ThemePreference::System,
            material_style: nxtools_core::MaterialStyle::Solid,
            tuning: nxtools_core::MaterialTuning::default(),
            system_dark: true,
        }
    }
}
impl gpui_kit::Global for UiThemeState {}
impl UiThemeState {
    pub fn is_dark(self) -> bool {
        self.preference.is_dark(self.system_dark)
    }
}

#[derive(Clone)]
struct UiFontState(gpui_kit::SharedString);
impl gpui_kit::Global for UiFontState {}

/// Call only at startup or on the deferred, durably saved settings turn.
/// Returns false when saved intent needs the default presentation fallback.
pub(crate) fn set_ui_font_family(family: &str, cx: &mut App) -> bool {
    let installed = crate::ui::typography::installed_ui_fonts(cx);
    set_ui_font_from_catalog(family, &installed, cx)
}

fn set_ui_font_from_catalog(family: &str, installed: &[String], cx: &mut App) -> bool {
    let available = nxtools_core::validate_ui_font_family(family).is_ok()
        && crate::ui::typography::ui_font_available(family, installed);
    let resolved = crate::ui::typography::resolved_ui_font(family, installed);
    cx.set_global(UiFontState(resolved.clone()));
    gpui_kit::component::Theme::update(cx, |theme| {
        theme.font_family = resolved;
        theme.mono_font_family = UI_MONO_FONT_FAMILY.into();
    });
    cx.refresh_windows();
    available
}

pub(crate) fn set_theme_preference(preference: nxtools_core::ThemePreference, cx: &mut App) {
    let style = cx
        .try_global::<UiThemeState>()
        .copied()
        .unwrap_or_default()
        .material_style;
    set_appearance_preferences(preference, style, cx);
}

/// Saved intent only; native material effectiveness is confirmed separately by Windows.
pub(crate) fn set_appearance_preferences(
    preference: nxtools_core::ThemePreference,
    material_style: nxtools_core::MaterialStyle,
    cx: &mut App,
) {
    let tuning = cx
        .try_global::<UiThemeState>()
        .copied()
        .unwrap_or_default()
        .tuning;
    set_appearance(preference, material_style, tuning, cx);
}

pub(crate) fn set_appearance(
    preference: nxtools_core::ThemePreference,
    material_style: nxtools_core::MaterialStyle,
    tuning: nxtools_core::MaterialTuning,
    cx: &mut App,
) {
    let mut state = cx.try_global::<UiThemeState>().copied().unwrap_or_default();
    state.preference = preference;
    state.material_style = material_style;
    state.tuning = tuning;
    cx.set_global(state);
    apply_active_theme(cx);
    cx.refresh_windows();
}

pub(crate) fn apply_system_theme(dark: bool, cx: &mut App) {
    let mut state = cx.try_global::<UiThemeState>().copied().unwrap_or_default();
    if state.system_dark == dark {
        return;
    }
    state.system_dark = dark;
    cx.set_global(state);
    if state.preference == nxtools_core::ThemePreference::System {
        apply_active_theme(cx);
        cx.refresh_windows();
    }
}

pub(crate) fn observe_theme<T: 'static>(
    window: &mut gpui_kit::Window,
    cx: &mut gpui_kit::Context<T>,
) {
    let view = cx.entity().downgrade();
    let id =
        gpui_kit::SharedString::from(format!("ui-theme-observer-{:?}", cx.entity().entity_id()));
    let _ = window.use_keyed_state(id, cx, move |_, cx| {
        cx.observe_global::<UiThemeState>(move |_, cx| {
            let view = view.clone();
            cx.defer(move |cx| {
                let _ = view.update(cx, |_, cx| cx.notify());
            });
        })
    });
}

/// Production startup and visual fixtures use one component palette mapping.
/// Transparent window backgrounds preserve the existing native alpha contract.
pub(crate) fn apply_component_theme(cx: &mut App) {
    crate::ui::material::paint::init(cx);
    crate::ui::components::project_menu::init(cx);
    for source in [
        NXTOOLS_SCHEMES,
        GRAPHITE_LIGHT,
        GRAPHITE_DARK,
        include_str!("../assets/themes/catppuccin.json"),
        include_str!("../assets/themes/gruvbox.json"),
        include_str!("../assets/themes/solarized.json"),
    ] {
        gpui_kit::component::ThemeRegistry::global_mut(cx)
            .load_themes_from_str(source)
            .expect("bundled GPUI Kit theme configs must be valid");
    }
    cx.set_global(UiThemeState::default());
    apply_active_theme(cx);
    observe_window_shape(cx);
}

fn apply_active_theme(cx: &mut App) {
    use gpui_kit::component::{Theme as ComponentTheme, ThemeMode, ThemeRegistry};
    let state = cx.try_global::<UiThemeState>().copied().unwrap_or_default();
    let dark = state.is_dark();
    let registry = ThemeRegistry::global(cx);
    let selected = registry.themes().get(state.preference.label()).cloned();
    let light_theme = selected
        .clone()
        .filter(|theme| !theme.mode.is_dark())
        .unwrap_or_else(|| {
            registry
                .themes()
                .get(DEFAULT_LIGHT)
                .cloned()
                .unwrap_or_else(|| registry.default_light_theme().clone())
        });
    let dark_theme = selected
        .filter(|theme| theme.mode.is_dark())
        .unwrap_or_else(|| {
            registry
                .themes()
                .get(DEFAULT_DARK)
                .cloned()
                .unwrap_or_else(|| registry.default_dark_theme().clone())
        });
    ComponentTheme::update(cx, |theme| {
        theme.light_theme = light_theme;
        theme.dark_theme = dark_theme;
    });
    ComponentTheme::change(
        if dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        None,
        cx,
    );
    // The selected config owns colors and sizes; saved UI typography survives
    // theme changes and the original diagnostic monospace remains unchanged.
    let family = cx
        .try_global::<UiFontState>()
        .map(|state| state.0.clone())
        .unwrap_or_else(|| UI_FONT_FAMILY.into());
    let rounded = crate::ui::material::rounded_corners(cx);
    let mut contour = gpui_kit::Hsla::default();
    let mut tint = None;
    ComponentTheme::update(cx, |component| {
        component.font_family = family;
        component.mono_font_family = UI_MONO_FONT_FAMILY.into();
        // Keep the official thin focus edge; omit the redundant outside halo.
        component.focus_ring = false;
        // Every window is rounded once facts are known: DWM on Windows 11, a region on Windows 10.
        component.window_radius = gpui_kit::px(if rounded {
            material::WINDOW_RADIUS
        } else {
            0.0
        });
        contour = if state.material_style.is_frosted() {
            let (frosted_contour, frosted_tint) = material::apply(&mut component.colors, dark);
            tint = Some(frosted_tint);
            frosted_contour
        } else {
            component.colors.window_border
        };
    });
    cx.set_global(material::Contour(contour));
    if let Some(tint) = tint {
        cx.set_global(tint);
    }
}

/// Re-themes when the window-corner capability becomes known, and repaints on every capability
/// change: which acrylic a frosted window uses (system or host backdrop) is read at paint time.
fn observe_window_shape(cx: &mut App) {
    cx.observe_global::<crate::ui::material::UiMaterialCapabilities>(|cx| {
        let rounded = crate::ui::material::rounded_corners(cx);
        let current = gpui_kit::component::Theme::global(cx).window_radius > gpui_kit::px(0.0);
        if rounded != current {
            apply_active_theme(cx);
        }
        cx.refresh_windows();
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;
    fn assert_color_close(actual: Rgba, expected: Rgba) {
        for (actual, expected) in [
            (actual.r, expected.r),
            (actual.g, expected.g),
            (actual.b, expected.b),
            (actual.a, expected.a),
        ] {
            assert!(
                (actual - expected).abs() < 0.00001,
                "color channel differs: {actual} vs {expected}"
            );
        }
    }

    #[gpui_kit::test]
    fn default_scheme_roles_are_applied_through_the_real_kit_theme(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(apply_component_theme);
        for preference in [
            nxtools_core::ThemePreference::Dark,
            nxtools_core::ThemePreference::Light,
        ] {
            let expected = default_colors(preference.is_dark(true));
            cx.update(|cx| set_theme_preference(preference, cx));
            cx.read(|cx| {
                let theme = Theme::active(cx);
                for (actual, expected) in [
                    (theme.canvas, expected.background.to_rgb()),
                    (theme.surface, expected.sidebar.to_rgb()),
                    (theme.surface_raised, expected.popover.to_rgb()),
                    (theme.border, expected.border.to_rgb()),
                    (theme.text, expected.foreground.to_rgb()),
                    (theme.accent, expected.primary.to_rgb()),
                ] {
                    assert_color_close(actual, expected);
                }
                let kit = gpui_kit::component::Theme::global(cx);
                assert_eq!(kit.tokens.button_primary.color, kit.colors.primary);
                assert_eq!(kit.semantic_tokens().colors.ring, kit.colors.ring);
            });
        }
    }

    #[gpui_kit::test]
    fn kit_theme_update_synchronizes_button_tokens_and_base_projection(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(apply_component_theme);
        cx.update(|cx| {
            let theme = gpui_kit::component::Theme::global(cx);
            let visual = Theme::zed_dark();
            assert_eq!(
                theme.colors.button_primary,
                default_colors(true).button_primary
            );
            assert_eq!(
                theme.colors.button_primary_foreground,
                default_colors(true).button_primary_foreground
            );
            assert_eq!(
                theme.tokens.button_primary.color,
                theme.colors.button_primary
            );
            assert_eq!(
                theme.semantic_tokens().colors.selection,
                theme.colors.selection
            );
            assert_eq!(theme.colors.background.to_rgb(), visual.canvas);
            assert_eq!(theme.colors.group_box, default_colors(true).group_box);
            assert_eq!(theme.tokens.group_box, theme.colors.group_box.into());
        });
    }

    #[test]
    fn approved_graphite_actions_and_focus_share_the_public_theme_roles() {
        let theme = Theme::zed_dark();
        let official = default_colors(true);
        assert_eq!(theme.accent, official.primary.to_rgb());
        assert_eq!(theme.accent_hover, official.primary_hover.to_rgb());
        assert_eq!(theme.focus_ring, official.ring.to_rgb());
        assert_ne!(theme.element_selected, theme.accent);
    }

    #[test]
    fn foundation_palette_matches_registered_project_configs() {
        for (theme, colors) in [
            (Theme::zed_dark(), default_colors(true)),
            (Theme::light(), default_colors(false)),
        ] {
            assert_eq!(theme.canvas, colors.background.to_rgb());
            assert_eq!(theme.surface, colors.sidebar.to_rgb());
            assert_eq!(theme.surface_raised, colors.popover.to_rgb());
            assert_eq!(theme.text, colors.foreground.to_rgb());
            assert_eq!(theme.text_secondary, colors.tab_foreground.to_rgb());
            assert_eq!(theme.text_muted, colors.muted_foreground.to_rgb());
            assert_eq!(theme.element_selected, colors.list_active.to_rgb());
        }
    }

    #[test]
    fn selection_and_pressed_roles_remain_distinct_from_neutral_hover() {
        let theme = Theme::zed_dark();
        assert_ne!(theme.element_selected, theme.accent_soft);
        assert_ne!(theme.element_selected, theme.surface_hover);
        assert_ne!(theme.surface_pressed, theme.surface_hover);
        assert_eq!(theme.icon_muted, theme.text_secondary);
    }

    #[gpui_kit::test]
    fn foundation_component_theme_uses_the_same_palette(cx: &mut gpui_kit::TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(apply_component_theme);
        cx.read(|cx| {
            let component = gpui_kit::component::Theme::global(cx);
            let visual = Theme::zed_dark();
            assert_eq!(component.colors.ring, visual.focus_ring.into());
            assert_eq!(component.colors.primary, visual.accent.into());
            assert_eq!(
                component.colors.primary_active,
                default_colors(true).primary_active
            );
            assert_eq!(
                component.colors.primary_hover,
                default_colors(true).primary_hover
            );
            assert_eq!(
                component.colors.primary_foreground.to_rgb(),
                visual.image_well
            );
            assert_eq!(component.font_family, UI_FONT_FAMILY);
            assert_eq!(component.radius, gpui_kit::px(6.0));
        });
    }

    #[test]
    fn popup_typography_uses_one_compact_scale() {
        assert_eq!(super::super::ui::typography::TEXT_BODY, 12.0);
        assert_eq!(super::super::ui::typography::TEXT_META, 10.0);
    }
}

#[cfg(test)]
mod preference_tests {
    use super::*;
    use nxtools_core::ThemePreference;

    #[gpui_kit::test]
    fn selected_theme_and_system_changes_share_component_and_business_palette(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(apply_component_theme);
        cx.update(|cx| set_theme_preference(ThemePreference::Light, cx));
        cx.read(|cx| {
            assert_eq!(Theme::active(cx).canvas, Theme::light().canvas);
            assert_eq!(
                gpui_kit::component::Theme::global(cx).colors.background,
                Theme::light().canvas.into()
            );
        });
        cx.update(|cx| apply_system_theme(true, cx));
        cx.read(|cx| assert_eq!(Theme::active(cx).canvas, Theme::light().canvas));
        cx.update(|cx| set_theme_preference(ThemePreference::System, cx));
        cx.read(|cx| assert_eq!(Theme::active(cx).canvas, Theme::zed_dark().canvas));
        cx.update(|cx| apply_system_theme(false, cx));
        cx.read(|cx| assert_eq!(Theme::active(cx).canvas, Theme::light().canvas));
        cx.update(|cx| set_theme_preference(ThemePreference::Dark, cx));
        cx.read(|cx| assert_eq!(Theme::active(cx).canvas, Theme::zed_dark().canvas));
    }
}

#[cfg(test)]
mod recurrence_tests {
    use super::*;
    use gpui_kit::rgb;

    #[test]
    fn recurrence_light_business_roles_come_from_official_kit_theme() {
        let official = default_colors(false);
        let visual = Theme::light();
        assert_eq!(visual.canvas, official.background.to_rgb());
        assert_eq!(visual.surface, official.sidebar.to_rgb());
        assert_eq!(visual.text, official.foreground.to_rgb());
        assert_eq!(visual.element_selected, official.list_active.to_rgb());
        assert_eq!(visual.accent, official.primary.to_rgb());
        assert_eq!(visual.focus_ring, official.ring.to_rgb());
    }

    fn contrast(a: Rgba, b: Rgba) -> f32 {
        fn luminance(color: Rgba) -> f32 {
            fn linear(value: f32) -> f32 {
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            }
            0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
        }
        let first = luminance(a);
        let second = luminance(b);
        (first.max(second) + 0.05) / (first.min(second) + 0.05)
    }

    #[gpui_kit::test]
    fn historical_graphite_numeric_roles_and_current_component_tokens(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(apply_component_theme);
        for preference in [
            nxtools_core::ThemePreference::Light,
            nxtools_core::ThemePreference::Dark,
        ] {
            cx.update(|cx| set_theme_preference(preference, cx));
            cx.read(|cx| {
                let retained: gpui_kit::component::ThemeSet = serde_json::from_str(
                    if preference.is_dark(true) { GRAPHITE_DARK } else { GRAPHITE_LIGHT }
                ).expect("historical Graphite asset");
                let mut historical = gpui_kit::component::Theme::default();
                historical.apply_config(&std::rc::Rc::new(retained.themes.into_iter().next().expect("historical config")));
                let visual = Theme::from_colors(&historical.colors);
                for (role, foreground, background) in [
                    ("body", visual.text, visual.canvas),
                    ("supporting", visual.text_secondary, visual.surface),
                    (
                        "selected",
                        visual.text,
                        Rgba {
                            r: visual.element_selected.r * visual.element_selected.a
                                + visual.canvas.r * (1.0 - visual.element_selected.a),
                            g: visual.element_selected.g * visual.element_selected.a
                                + visual.canvas.g * (1.0 - visual.element_selected.a),
                            b: visual.element_selected.b * visual.element_selected.a
                                + visual.canvas.b * (1.0 - visual.element_selected.a),
                            a: 1.0,
                        },
                    ),
                    ("primary", visual.image_well, visual.accent),
                    ("primary hover", visual.image_well, visual.accent_hover),
                    // Status colors are official semantic indicators, not body text.
                ] {
                    assert!(
                        contrast(foreground, background) >= 4.5,
                        "historical Graphite {preference:?} {role} must retain readable text contrast"
                    );
                }
                let component = gpui_kit::component::Theme::global(cx);
                assert_eq!(Theme::active(cx).success, component.colors.green.to_rgb());
                assert_eq!(Theme::active(cx).warning, component.colors.yellow.to_rgb());
                assert_eq!(component.colors.table_active, component.colors.list_active);
                assert_eq!(
                    component.tokens.tab_active.color,
                    component.colors.tab_active
                );
                assert_eq!(
                    component.tokens.button_primary.color,
                    component.colors.primary
                );
            });
        }
    }

    #[gpui_kit::test]
    fn official_theme_switch_keeps_all_kit_colors_and_projects_current_palette(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        for (preference, mode) in [
            (
                nxtools_core::ThemePreference::Light,
                gpui_kit::component::ThemeMode::Light,
            ),
            (
                nxtools_core::ThemePreference::Dark,
                gpui_kit::component::ThemeMode::Dark,
            ),
        ] {
            cx.update(|cx| gpui_kit::component::Theme::change(mode, None, cx));
            let expected = cx.read(|cx| gpui_kit::component::Theme::global(cx).colors);
            cx.update(|cx| set_theme_preference(preference, cx));
            cx.read(|cx| {
                let component = gpui_kit::component::Theme::global(cx);
                assert_eq!(
                    component.colors, expected,
                    "application must not recolor Kit defaults"
                );
                assert_eq!(Theme::active(cx).canvas, expected.background.to_rgb());
                assert_eq!(
                    Theme::active(cx).element_selected,
                    expected.list_active.to_rgb()
                );
            });
            cx.update(|cx| {
                gpui_kit::component::Theme::update(cx, |kit| {
                    kit.colors.background = rgb(0x123456).into()
                })
            });
            cx.read(|cx| {
                assert_eq!(
                    Theme::active(cx).canvas,
                    gpui_kit::component::Theme::global(cx)
                        .colors
                        .background
                        .to_rgb(),
                    "custom views must project current Kit, not an independent static palette"
                )
            });
        }
    }
}

#[cfg(test)]
mod ui_font_tests {
    use super::*;

    #[gpui_kit::test]
    fn ui_font_application_survives_theme_switch_and_keeps_monospace_and_sizes(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(apply_component_theme);
        let before = cx.read(|cx| {
            let theme = gpui_kit::component::Theme::global(cx);
            (theme.font_size, theme.mono_font_size, theme.colors)
        });
        let mono_before = cx.read(|cx| {
            gpui_kit::component::Theme::global(cx)
                .mono_font_family
                .clone()
        });
        cx.update(|cx| {
            assert!(set_ui_font_from_catalog(
                "思源黑体 CN",
                &["思源黑体 CN".into()],
                cx
            ));
        });
        cx.read(|cx| {
            let theme = gpui_kit::component::Theme::global(cx);
            assert_eq!(theme.font_family, "思源黑体 CN");
            assert_eq!(
                (theme.font_size, theme.mono_font_size, theme.colors),
                before
            );
        });
        for preference in [
            nxtools_core::ThemePreference::Light,
            nxtools_core::ThemePreference::Dark,
        ] {
            cx.update(|cx| set_theme_preference(preference, cx));
            cx.read(|cx| {
                let theme = gpui_kit::component::Theme::global(cx);
                assert_eq!(theme.font_family, "思源黑体 CN");
                assert_eq!(theme.semantic_tokens().typography.sans, "思源黑体 CN");
                assert_eq!(theme.mono_font_family, mono_before);
                assert_eq!(theme.semantic_tokens().typography.mono, mono_before);
                assert_eq!(
                    gpui_kit::base::Theme::global(cx).tokens,
                    theme.semantic_tokens()
                );
                assert_eq!(
                    (theme.font_size, theme.mono_font_size),
                    (before.0, before.1)
                );
            });
        }
        cx.update(|cx| assert!(!set_ui_font_family("NxTools missing test family", cx)));
        cx.read(|cx| {
            assert_eq!(
                crate::ui::typography::active_ui_font_family(cx),
                UI_FONT_FAMILY
            )
        });
    }
}

#[cfg(test)]
mod ui_font_paint_tests {
    use super::*;
    use gpui_kit::{AppContext as _, IntoElement, ParentElement as _, Styled as _, div, px};
    use std::{cell::RefCell, rc::Rc};

    struct FontPaintFixture(Rc<RefCell<Vec<gpui_kit::SharedString>>>);

    impl gpui_kit::Render for FontPaintFixture {
        fn render(
            &mut self,
            _: &mut gpui_kit::Window,
            cx: &mut gpui_kit::Context<Self>,
        ) -> impl IntoElement {
            let normal = self.0.clone();
            let mono = self.0.clone();
            let body = div()
                .child(
                    gpui_kit::canvas(
                        |_, _, _| (),
                        move |_, (), window, _| {
                            normal
                                .borrow_mut()
                                .push(window.text_style().font_family.clone());
                        },
                    )
                    .size(px(1.0)),
                )
                .child(
                    div().font_family(UI_MONO_FONT_FAMILY).child(
                        gpui_kit::canvas(
                            |_, _, _| (),
                            move |_, (), window, _| {
                                mono.borrow_mut()
                                    .push(window.text_style().font_family.clone());
                            },
                        )
                        .size(px(1.0)),
                    ),
                );
            crate::ui::components::settings::settings_shell(
                Theme::active(cx),
                crate::ui::components::settings::SettingsShellIds {
                    root: "font-paint-shell",
                    content: "font-paint-body",
                    footer: "font-paint-footer",
                },
                div().into_any_element(),
                body.into_any_element(),
                div().into_any_element(),
            )
        }
    }

    #[gpui_kit::test]
    fn ui_font_reaches_production_settings_shell_paint_without_changing_monospace(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(apply_component_theme);
        let fonts = Rc::new(RefCell::new(Vec::new()));
        let recorded = fonts.clone();
        let (_, window) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|_| FontPaintFixture(recorded));
            gpui_kit::component::Root::new(view, window, cx)
        });
        window.refresh().expect("default font paint");
        assert!(fonts.borrow().iter().any(|name| name == UI_FONT_FAMILY));
        fonts.borrow_mut().clear();
        window.update(|_, cx| {
            assert!(set_ui_font_from_catalog(
                TEST_UI_FONT_FAMILY,
                &[TEST_UI_FONT_FAMILY.into()],
                cx
            ));
        });
        window.refresh().expect("saved font paint");
        let painted = fonts.borrow();
        assert!(painted.iter().any(|name| name == TEST_UI_FONT_FAMILY));
        assert!(painted.iter().any(|name| name == UI_MONO_FONT_FAMILY));
        assert!(!painted.iter().any(|name| name == UI_FONT_FAMILY));
    }

    #[gpui_kit::test]
    fn material_style_intent_remains_independent_across_palette_font_and_system_updates(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use nxtools_core::{MaterialStyle, ThemePreference};
        cx.update(gpui_kit::init);
        cx.update(apply_component_theme);
        cx.update(|cx| {
            assert!(set_ui_font_from_catalog(
                TEST_UI_FONT_FAMILY,
                &[TEST_UI_FONT_FAMILY.into()],
                cx
            ));
            let resolved_mono = gpui_kit::component::Theme::global(cx)
                .mono_font_family
                .clone();
            for preference in ThemePreference::ALL {
                set_appearance_preferences(preference, MaterialStyle::Solid, cx);
                let solid = Theme::active(cx);
                let radius = gpui_kit::component::Theme::global(cx).radius;
                set_appearance_preferences(preference, MaterialStyle::Acrylic, cx);
                assert_eq!(cx.global::<UiThemeState>().preference, preference);
                assert_eq!(
                    cx.global::<UiThemeState>().material_style,
                    MaterialStyle::Acrylic
                );
                // The palette keeps its accent; Acrylic may only deepen text for legibility.
                assert_eq!(Theme::active(cx).accent, solid.accent);
                assert_eq!(gpui_kit::component::Theme::global(cx).radius, radius);
                assert_eq!(
                    gpui_kit::component::Theme::global(cx).font_family,
                    TEST_UI_FONT_FAMILY
                );
                assert_eq!(
                    gpui_kit::component::Theme::global(cx).mono_font_family,
                    resolved_mono
                );
                for dark in [false, true] {
                    apply_system_theme(dark, cx);
                    assert_eq!(
                        cx.global::<UiThemeState>().material_style,
                        MaterialStyle::Acrylic
                    );
                    assert_eq!(
                        cx.global::<UiThemeState>().is_dark(),
                        preference.is_dark(dark)
                    );
                }
                set_theme_preference(ThemePreference::SolarizedLight, cx);
                assert_eq!(
                    cx.global::<UiThemeState>().material_style,
                    MaterialStyle::Acrylic
                );
                set_appearance_preferences(preference, MaterialStyle::Solid, cx);
                assert_eq!(
                    cx.global::<UiThemeState>().material_style,
                    MaterialStyle::Solid
                );
            }
        });
    }
}
