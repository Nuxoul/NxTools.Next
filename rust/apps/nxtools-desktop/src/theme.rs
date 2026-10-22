use gpui_kit::{App, Rgba};

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

fn graphite_colors(dark: bool) -> gpui_kit::component::ThemeColor {
    use std::sync::LazyLock;
    static COLORS: LazyLock<[gpui_kit::component::ThemeColor; 2]> = LazyLock::new(|| {
        [GRAPHITE_LIGHT, GRAPHITE_DARK].map(|source| {
            let set: gpui_kit::component::ThemeSet =
                serde_json::from_str(source).expect("bundled project theme");
            let mut theme = gpui_kit::component::Theme::default();
            theme.apply_config(&std::rc::Rc::new(
                set.themes.into_iter().next().expect("project theme"),
            ));
            theme.colors
        })
    });
    COLORS[usize::from(dark)]
}

impl Theme {
    // Initial-view compatibility constructors share the bundled project config.
    // Runtime rendering always projects the active public Kit theme.
    pub fn zed_dark() -> Self {
        Self::from_colors(&graphite_colors(true))
    }

    #[cfg(test)]
    pub fn light() -> Self {
        Self::from_colors(&graphite_colors(false))
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

    pub fn active(cx: &App) -> Self {
        Self::from_colors(&gpui_kit::component::Theme::global(cx).colors)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct UiThemeState {
    pub preference: nxtools_core::ThemePreference,
    pub system_dark: bool,
}
impl Default for UiThemeState {
    fn default() -> Self {
        Self {
            preference: nxtools_core::ThemePreference::Dark,
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

pub(crate) fn set_theme_preference(preference: nxtools_core::ThemePreference, cx: &mut App) {
    let mut state = cx.try_global::<UiThemeState>().copied().unwrap_or_default();
    state.preference = preference;
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
    crate::ui::components::project_menu::init(cx);
    for source in [
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
                .get("NxTools Graphite Light")
                .cloned()
                .unwrap_or_else(|| registry.default_light_theme().clone())
        });
    let dark_theme = selected
        .filter(|theme| theme.mode.is_dark())
        .unwrap_or_else(|| {
            registry
                .themes()
                .get("NxTools Graphite Dark")
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
    // Keep installed CJK fonts. The selected public Kit config owns every
    // color, renderable token and Base projection, including optional themes.
    ComponentTheme::update(cx, |component| {
        component.font_family = UI_FONT_FAMILY.into();
        component.mono_font_family = UI_MONO_FONT_FAMILY.into();
    });
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
    fn graphite_default_roles_are_applied_through_the_real_kit_theme(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(apply_component_theme);
        for (preference, canvas, surface, raised, border, text, accent) in [
            (
                nxtools_core::ThemePreference::Dark,
                0x181a1f,
                0x202329,
                0x292d35,
                0x3b404b,
                0xe7e9ee,
                0xa5b4fc,
            ),
            (
                nxtools_core::ThemePreference::Light,
                0xf4f5f7,
                0xffffff,
                0xffffff,
                0xd4d8e0,
                0x20242d,
                0x5654c5,
            ),
        ] {
            cx.update(|cx| set_theme_preference(preference, cx));
            cx.read(|cx| {
                let theme = Theme::active(cx);
                for (actual, expected) in [
                    (theme.canvas, canvas),
                    (theme.surface, surface),
                    (theme.surface_raised, raised),
                    (theme.border, border),
                    (theme.text, text),
                    (theme.accent, accent),
                ] {
                    assert_color_close(actual, gpui_kit::rgb(expected));
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
            assert_eq!(theme.colors.button_primary, visual.accent.into());
            assert_eq!(
                theme.colors.button_primary_foreground,
                visual.image_well.into()
            );
            assert_eq!(
                theme.tokens.button_primary.color,
                theme.colors.button_primary
            );
            assert_eq!(
                theme.semantic_tokens().colors.selection,
                theme.colors.selection
            );
            assert_eq!(theme.colors.background, visual.canvas.into());
            assert_eq!(theme.colors.group_box, graphite_colors(true).group_box);
            assert_eq!(theme.tokens.group_box, theme.colors.group_box.into());
        });
    }

    #[test]
    fn approved_graphite_actions_and_focus_share_the_public_theme_roles() {
        let theme = Theme::zed_dark();
        let official = graphite_colors(true);
        assert_eq!(theme.accent, official.primary.to_rgb());
        assert_eq!(theme.accent_hover, official.primary_hover.to_rgb());
        assert_eq!(theme.focus_ring, official.ring.to_rgb());
        assert_ne!(theme.element_selected, theme.accent);
    }

    #[test]
    fn foundation_palette_matches_registered_project_configs() {
        for (theme, colors) in [
            (Theme::zed_dark(), graphite_colors(true)),
            (Theme::light(), graphite_colors(false)),
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
            assert_ne!(
                component.colors.primary_active, component.colors.primary_hover,
                "primary pressed state must remain distinct from hover"
            );
            assert_eq!(
                component.colors.primary_foreground,
                visual.image_well.into()
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
        let official = graphite_colors(false);
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
    fn recurrence_both_themes_pair_readable_roles_and_component_tokens(
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
                let visual = Theme::active(cx);
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
                        "{preference:?} {role} must have readable text contrast"
                    );
                }
                let component = gpui_kit::component::Theme::global(cx);
                assert_eq!(visual.success, component.colors.green.to_rgb());
                assert_eq!(visual.warning, component.colors.yellow.to_rgb());
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
