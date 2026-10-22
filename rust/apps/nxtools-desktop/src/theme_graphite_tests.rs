//! Project defaults must not recolor the retained official optional configurations.
#[gpui_kit::test]
fn graphite_defaults_preserve_every_optional_official_theme(cx: &mut gpui_kit::TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    for preference in nxtools_core::ThemePreference::ALL
        .into_iter()
        .filter(|preference| {
            !matches!(
                preference,
                nxtools_core::ThemePreference::System
                    | nxtools_core::ThemePreference::Light
                    | nxtools_core::ThemePreference::Dark
            )
        })
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
