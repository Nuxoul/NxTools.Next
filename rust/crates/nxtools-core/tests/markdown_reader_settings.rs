use nxtools_core::{AppConfig, MarkdownReaderSettings, ThemePreference};

#[test]
fn markdown_reader_legacy_defaults_and_owned_preferences_merge() {
    let folder = tempfile::tempdir().expect("owned Reader settings directory");
    let path = folder.path().join("config.json");
    let mut legacy =
        serde_json::to_value(AppConfig::default()).expect("serialize default configuration");
    legacy
        .as_object_mut()
        .expect("configuration object")
        .remove("markdown_reader");
    let bytes = serde_json::to_vec(&legacy).expect("serialize legacy configuration");
    std::fs::write(&path, &bytes).expect("write owned legacy configuration");
    let initial = AppConfig::load_from(&path).expect("load legacy Reader defaults");
    assert_eq!(initial.markdown_reader, MarkdownReaderSettings::default());
    assert!(initial.markdown_reader.enabled);
    assert_eq!(
        std::fs::read(&path).expect("read unchanged legacy bytes"),
        bytes
    );

    AppConfig::update_section(&path, |config| config.theme = ThemePreference::Light)
        .expect("save independent theme preference");
    let saved = AppConfig::update_section(&path, |config| {
        config.markdown_reader.enabled = false;
        config.markdown_reader.show_outline = false;
        config.markdown_reader.font_size = 20;
        config.markdown_reader.wide_body = true;
    })
    .expect("save owned Reader preferences");
    assert_eq!(saved.theme, ThemePreference::Light);
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("read persisted Reader preferences")
            .markdown_reader,
        MarkdownReaderSettings {
            enabled: false,
            show_outline: false,
            font_size: 20,
            wide_body: true,
        }
    );
}
