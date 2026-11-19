use nxtools_core::AppConfig;
use serde_json::Value;
use std::fs;

#[test]
fn integration_setup_dismissal_defaults_for_legacy_config_and_roundtrips() {
    let directory = tempfile::tempdir().expect("create config directory");
    let path = directory.path().join("config.json");

    let mut legacy = serde_json::to_value(AppConfig::default()).expect("serialize defaults");
    legacy
        .as_object_mut()
        .expect("config object")
        .remove("integration_setup_dismissed");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&legacy).expect("serialize legacy config"),
    )
    .expect("write legacy config");

    let loaded = AppConfig::load_from(&path).expect("load legacy config");
    assert!(!loaded.integration_setup_dismissed);

    let saved = AppConfig::update_section(&path, |config| {
        config.integration_setup_dismissed = true;
    })
    .expect("save integration setup dismissal");
    assert!(saved.integration_setup_dismissed);
    assert!(
        AppConfig::load_from(&path)
            .expect("reload config")
            .integration_setup_dismissed
    );

    let encoded = serde_json::to_value(AppConfig::load_from(&path).expect("load saved config"))
        .expect("serialize saved config");
    assert_eq!(
        encoded
            .get("integration_setup_dismissed")
            .and_then(Value::as_bool),
        Some(true)
    );
}
