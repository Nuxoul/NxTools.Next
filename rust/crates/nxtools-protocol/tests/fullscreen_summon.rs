use nxtools_protocol::ConfigureClipboardCommand;
use serde_json::json;

#[test]
fn clipboard_fullscreen_summon_policy_defaults_on_and_roundtrips_explicit_values() {
    let legacy = json!({
        "hotkey_modifiers": ["control", "shift"],
        "hotkey_key": "V",
        "hotkey_display": "Ctrl+Shift+V",
        "excluded_processes": []
    });
    let decoded: ConfigureClipboardCommand =
        serde_json::from_value(legacy.clone()).expect("legacy clipboard configuration");
    assert!(decoded.suppress_in_fullscreen);
    for enabled in [false, true] {
        let mut value = legacy.clone();
        value["suppress_in_fullscreen"] = json!(enabled);
        let decoded: ConfigureClipboardCommand =
            serde_json::from_value(value).expect("fullscreen configuration");
        assert_eq!(decoded.suppress_in_fullscreen, enabled);
        let roundtrip = serde_json::to_value(decoded).expect("clipboard configure wire");
        assert_eq!(roundtrip["suppress_in_fullscreen"], json!(enabled));
    }
    let mut invalid = legacy;
    invalid["suppress_in_fullscreen"] = json!("false");
    assert!(serde_json::from_value::<ConfigureClipboardCommand>(invalid).is_err());
}
