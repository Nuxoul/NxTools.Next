use nxtools_protocol::archive_job::ArchiveJobRequest;
use serde_json::{Value, json};

fn request(mode: &str) -> Value {
    json!({
        "kind":"extract", "paths":["C:\\owned\\source.zip"],
        "destination":"C:\\owned\\output", "codepage":0,
        "extract": {"strategy":"here", "single_file_folder":false,
            "conflict":"ask", "copy_motw":true, "selection":null,
            "reclamation":{"mode":mode,"recovery_id":"47bd704d-466f-4161-a045-a225cb0d7a62"}},
        "create":null
    })
}
fn valid(value: Value) -> bool {
    serde_json::from_value::<ArchiveJobRequest>(value)
        .is_ok_and(|request| request.validate().is_ok())
}

#[test]
fn explicit_start_and_fresh_continue_are_admitted_without_changing_ordinary_default() {
    assert!(valid(request("start")));
    assert!(valid(request("continue")));
    let mut ordinary = request("start");
    ordinary["extract"]
        .as_object_mut()
        .expect("ordinary request")
        .remove("reclamation");
    let parsed: ArchiveJobRequest =
        serde_json::from_value(ordinary).expect("parse ordinary request");
    assert!(parsed.validate().is_ok());
    let encoded = serde_json::to_value(parsed).expect("encode ordinary request");
    assert!(encoded["extract"].get("reclamation").is_none());
}

#[test]
fn destructive_intent_cannot_enter_ordinary_readmission_or_conflict_strategy() {
    let mut mixed = request("start");
    mixed["readmission"] =
        json!({"logical_id":"47bd704d-466f-4161-a045-a225cb0d7a62", "expected_source":null});
    assert!(!valid(mixed));
    for strategy in ["name", "smart"] {
        let mut value = request("start");
        value["extract"]["strategy"] = json!(strategy);
        assert!(!valid(value));
    }
    for conflict in ["skip", "overwrite", "auto_rename"] {
        let mut value = request("start");
        value["extract"]["conflict"] = json!(conflict);
        assert!(!valid(value));
    }
    assert!(!valid(request("resume")));
    let mut nil = request("start");
    nil["extract"]["reclamation"]["recovery_id"] = json!("00000000-0000-0000-0000-000000000000");
    assert!(!valid(nil));
}
