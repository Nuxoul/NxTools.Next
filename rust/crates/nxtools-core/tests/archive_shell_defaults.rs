use nxtools_core::archive::{ArchiveMenuCommand, ArchiveShellIntegrationSettings};
use serde_json::json;

#[test]
fn archive_polish_fresh_and_missing_recommend_three_creation_commands() {
    let fresh = ArchiveShellIntegrationSettings::default();
    assert!(!fresh.classic_cascade && !fresh.open_with && fresh.extensions.is_empty());
    fresh.validate().expect("valid recommendation");
    let old =
        json!({"classic_cascade":false,"open_with":false,"extensions":[],"desired_revision":0});
    let missing: ArchiveShellIntegrationSettings = serde_json::from_value(old).expect("old fields");
    for settings in [fresh, missing] {
        let commands =
            serde_json::to_value(settings).expect("encoded settings")["menu_commands"].clone();
        for command in ["create_zip", "create_7z", "create"] {
            assert!(
                commands
                    .as_array()
                    .expect("commands")
                    .contains(&json!(command)),
                "recommended {command}"
            );
        }
    }
}

#[test]
fn archive_polish_explicit_empty_and_custom_choices_remain_authoritative() {
    for choices in [
        json!([]),
        json!(["test"]),
        json!(["create"]),
        json!(["create_zip"]),
        json!([
            "open",
            "extract_smart",
            "extract_here",
            "extract_name",
            "test"
        ]),
    ] {
        let settings: ArchiveShellIntegrationSettings = serde_json::from_value(json!({
            "classic_cascade":false,"open_with":false,"extensions":[],"desired_revision":1,
            "menu_commands":choices
        }))
        .expect("explicit saved choices");
        assert_eq!(
            serde_json::to_value(settings).expect("encoded")["menu_commands"],
            choices
        );
    }
    assert_eq!(ArchiveMenuCommand::Create.wire_name(), "create");
}
