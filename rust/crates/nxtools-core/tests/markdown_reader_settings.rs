use nxtools_core::{AppConfig, MarkdownReaderSettings, ThemePreference};

#[test]
fn markdown_reader_legacy_defaults_and_owned_preferences_merge() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("config.json");
    let mut legacy = serde_json::to_value(AppConfig::default()).unwrap();
    legacy.as_object_mut().unwrap().remove("markdown_reader");
    let bytes = serde_json::to_vec(&legacy).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let initial = AppConfig::load_from(&path).unwrap();
    assert_eq!(initial.markdown_reader, MarkdownReaderSettings::default());
    assert!(initial.markdown_reader.enabled);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    AppConfig::update_section(&path, |config| config.theme = ThemePreference::Light).unwrap();
    let saved = AppConfig::update_section(&path, |config| {
        config.markdown_reader.enabled = false;
        config.markdown_reader.show_outline = false;
        config.markdown_reader.font_size = 20;
        config.markdown_reader.wide_body = true;
    })
    .unwrap();
    assert_eq!(saved.theme, ThemePreference::Light);
    assert_eq!(
        AppConfig::load_from(&path).unwrap().markdown_reader,
        MarkdownReaderSettings {
            enabled: false,
            show_outline: false,
            font_size: 20,
            wide_body: true,
        }
    );
}
