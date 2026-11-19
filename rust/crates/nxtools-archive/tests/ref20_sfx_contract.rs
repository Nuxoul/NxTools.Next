use nxtools_archive::worker::{WorkerRequest, ZipCreateOptions};
use nxtools_protocol::archive_job::{ArchiveJobLimits, ArchiveJobRequest};
use serde_json::{Value, json};
use uuid::Uuid;

fn public_request() -> Value {
    json!({
        "kind": "create", "paths": ["C:\\owned\\source"], "destination": "C:\\owned\\output",
        "codepage": 0, "limits": ArchiveJobLimits::default(), "extract": null,
        "modify": null, "readmission": null,
        "create": {
            "format": "zip", "level": "normal", "method": "deflate", "password": null,
            "solid": null, "encrypt_header": null, "strip_single_root": true,
            "exclude_patterns": [], "test_after_create": true, "name": "created.exe"
        }
    })
}

#[test]
fn public_intent_is_explicit_default_false_and_zip_only() {
    let legacy: ArchiveJobRequest =
        serde_json::from_value(public_request()).expect("legacy request");
    legacy.validate().expect("legacy validation");
    assert!(
        serde_json::to_value(legacy).expect("legacy wire")["create"]
            .get("self_extracting")
            .is_none()
    );
    let mut value = public_request();
    value["create"]["self_extracting"] = json!(true);
    let captured: ArchiveJobRequest =
        serde_json::from_value(value.clone()).expect("explicit SFX intent");
    captured.validate().expect("SFX validation");
    assert_eq!(
        serde_json::to_value(captured).expect("captured wire")["create"]["self_extracting"],
        true
    );
    for (field, invalid) in [
        ("name", json!("created.zip")),
        ("name", json!(".exe")),
        ("password", json!(b"private-test".to_vec())),
        ("format", json!("seven_z")),
        ("volume_bytes", json!(4_294_967_296u64)),
    ] {
        let mut request = value.clone();
        request["create"][field] = invalid;
        let request: ArchiveJobRequest =
            serde_json::from_value(request).expect("typed incompatible intent");
        assert!(request.validate().is_err(), "incompatible SFX {field}");
    }
}

#[test]
fn private_intent_survives_snapshot_and_rejects_incompatible_options() {
    let legacy: ZipCreateOptions = serde_json::from_value(json!({
        "method": "deflate", "encryption": "none", "strip_single_root": true, "exclude_patterns": []
    }))
    .expect("legacy private options");
    assert!(
        serde_json::to_value(legacy)
            .expect("legacy private wire")
            .get("self_extracting")
            .is_none()
    );
    let mut request = WorkerRequest::open(Uuid::new_v4(), "C:\\owned\\source".into());
    request.version = 2;
    request.operation = "create_zip".into();
    request.bulk_transfer = Some(1);
    request.create_name = Some("created.exe".into());
    request.compression_level = Some("normal".into());
    let mut value = serde_json::to_value(request).expect("private request");
    value["zip_options"] = json!({
        "method": "deflate", "encryption": "none", "strip_single_root": true,
        "exclude_patterns": [], "self_extracting": true
    });
    let captured: WorkerRequest =
        serde_json::from_value(value.clone()).expect("private SFX intent");
    captured.validate().expect("private SFX validation");
    assert_eq!(
        serde_json::to_value(captured.clone()).expect("private snapshot")["zip_options"]["self_extracting"],
        true
    );
    for (field, invalid) in [
        ("create_name", json!("created.zip")),
        ("create_name", json!(".exe")),
        ("password", json!("private-test")),
        ("volume_bytes", json!(4_294_967_296u64)),
    ] {
        let mut request = value.clone();
        request[field] = invalid;
        let request: WorkerRequest =
            serde_json::from_value(request).expect("typed private incompatible intent");
        assert!(
            request.validate().is_err(),
            "incompatible private SFX {field}"
        );
    }
}
