use nxtools_core::{AppConfig, SystemSettings, ThemePreference};

#[test]
fn new_global_appearance_follows_system() {
    assert_eq!(ThemePreference::default(), ThemePreference::System);
    assert_eq!(SystemSettings::default().theme, ThemePreference::System);
    assert_eq!(AppConfig::default().system.theme, ThemePreference::System);
    assert_eq!(AppConfig::default().theme, ThemePreference::System);
}

#[test]
fn absent_appearance_follows_system_without_overwriting_explicit_choices() {
    let absent: AppConfig = serde_json::from_value(serde_json::json!({
        "theme": "dark", "platform_pipe_name": "synthetic", "system": {}
    }))
    .expect("legacy configuration");
    assert_eq!(absent.system.theme, ThemePreference::System);
    for preference in ThemePreference::ALL {
        let restored: SystemSettings = serde_json::from_value(serde_json::json!({
            "theme": preference
        }))
        .expect("explicit appearance");
        assert_eq!(restored.theme, preference);
    }
}
