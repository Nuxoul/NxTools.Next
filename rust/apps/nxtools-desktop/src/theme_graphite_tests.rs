//! Applying a theme must not recolor its bundled configuration.
#[gpui_kit::test]
fn bundled_themes_apply_exactly_as_configured(cx: &mut gpui_kit::TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    for preference in nxtools_core::ThemePreference::ALL
        .into_iter()
        .filter(|preference| *preference != nxtools_core::ThemePreference::System)
    {
        let expected = cx.read(|cx| {
            let config = gpui_kit::component::ThemeRegistry::global(cx)
                .themes()
                .get(preference.label())
                .expect("retained official theme")
                .clone();
            let mut theme = gpui_kit::component::Theme::default();
            theme.apply_config(&config);
            theme.colors
        });
        cx.update(|cx| crate::theme::set_theme_preference(preference, cx));
        cx.read(|cx| {
            assert_eq!(
                gpui_kit::component::Theme::global(cx).colors,
                expected,
                "optional official theme must not be recolored"
            )
        });
    }
}

#[gpui_kit::test]
fn density_theme_defaults_are_exact_nxtools_schemes_and_system_pair(
    cx: &mut gpui_kit::TestAppContext,
) {
    use nxtools_core::ThemePreference;
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    for (preference, system_dark, expected_name) in [
        (ThemePreference::Light, true, "默认浅色"),
        (ThemePreference::Dark, false, "默认深色"),
        (ThemePreference::System, false, "默认浅色"),
        (ThemePreference::System, true, "默认深色"),
    ] {
        cx.update(|cx| {
            crate::theme::apply_system_theme(system_dark, cx);
            crate::theme::set_theme_preference(preference, cx);
        });
        cx.read(|cx| {
            let config = gpui_kit::component::ThemeRegistry::global(cx)
                .themes()
                .get(expected_name)
                .expect("bundled requested theme")
                .clone();
            let mut expected = gpui_kit::component::Theme::default();
            expected.apply_config(&config);
            let active = gpui_kit::component::Theme::global(cx);
            assert_eq!(
                active.colors, expected.colors,
                "exact alias {preference:?}/{system_dark}"
            );
            assert_eq!(active.tokens.button_primary.color, expected.colors.primary);
            assert_eq!(
                crate::theme::Theme::active(cx).canvas,
                expected.colors.background.to_rgb()
            );
        });
    }
}

#[gpui_kit::test]
fn runtime_feedback_outer_halo_is_disabled_without_recoloring_focus_or_selection(
    cx: &mut gpui_kit::TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    for preference in nxtools_core::ThemePreference::ALL {
        cx.update(|cx| crate::theme::set_theme_preference(preference, cx));
        cx.read(|cx| {
            let active = gpui_kit::component::Theme::global(cx);
            assert!(
                !active.focus_ring,
                "only outside halo should be disabled in {preference:?}"
            );
            assert!(
                active.colors.ring.a > 0.0,
                "thin official focus edge remains available"
            );
            assert!(
                active.colors.selection.a > 0.0,
                "selected content must remain distinct"
            );
        });
    }
}
