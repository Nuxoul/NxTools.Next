use nxtools_archive::worker::{WorkerFrame, WorkerRequest, read_frame, write_frame};
use std::io::Cursor;
use uuid::Uuid;

#[test]
fn private_frames_are_bounded_strict_and_preserve_operation_identity() {
    let id = Uuid::new_v4();
    let request = WorkerRequest::open(id, "synthetic.zip".into());
    let mut wire = Vec::new();
    write_frame(&mut wire, &request).expect("write request");
    let decoded: WorkerRequest = read_frame(&mut Cursor::new(&wire))
        .expect("decode request")
        .expect("one request");
    assert_eq!(decoded.operation_id, id);
    assert_eq!(decoded.path, "synthetic.zip");
    assert!(read_frame::<_, WorkerRequest>(&mut Cursor::new([0xff, 0xff, 0xff, 0x7f])).is_err());
    assert!(read_frame::<_, WorkerRequest>(&mut Cursor::new([4, 0, 0, 0, b'{'])).is_err());
    assert!(read_frame::<_, WorkerRequest>(&mut Cursor::new([2, 0, 0, 0, 0xff, 0xfe])).is_err());
    let frame = WorkerFrame::complete(id, 3, 275);
    assert!(
        serde_json::to_value(&frame)
            .expect("legacy success wire")
            .get("cancellation")
            .is_none()
    );
    let mut wire = Vec::new();
    write_frame(&mut wire, &frame).expect("write complete");
    let result: WorkerFrame = read_frame(&mut Cursor::new(wire))
        .expect("complete frame")
        .expect("one complete");
    assert_eq!(result.operation_id, id);
    assert_eq!(result.op, "complete");
    assert_eq!(result.bytes_verified, Some(275));
}

#[test]
fn secrets_and_paths_have_no_debug_or_argv_contract() {
    let json = serde_json::json!({"schema":1,"operation_id":Uuid::new_v4(),"op":"start","operation":"open","path":"synthetic.zip","password":"sentinel-secret","codepage":0});
    let request: WorkerRequest = serde_json::from_value(json).expect("private request");
    assert_eq!(request.password.as_deref(), Some("sentinel-secret"));
    assert!(request.validate().is_ok());
}

#[test]
fn actual_worker_exchanges_private_frames_and_exits_without_payload_stderr() {
    use std::process::{Command, Stdio};
    let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
        .arg("--archive-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("owned worker process");
    let id = Uuid::new_v4();
    let path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stored.zip");
    let request = WorkerRequest::open(id, path.to_string_lossy().into_owned());
    write_frame(child.stdin.as_mut().expect("private stdin"), &request)
        .expect("send private request");
    let output = child.stdout.as_mut().expect("private stdout");
    let mut entries = 0;
    loop {
        let frame: WorkerFrame = read_frame(output)
            .expect("valid bounded frame")
            .expect("terminal frame before EOF");
        assert_eq!(frame.operation_id, id);
        if frame.op == "entry_batch" {
            entries += frame.entries.len();
        }
        if frame.op == "failure" {
            panic!("worker must list valid ZIP");
        }
        if frame.op == "complete" {
            break;
        }
    }
    assert_eq!(entries, 4);
    drop(child.stdin.take());
    let result = child.wait_with_output().expect("wait for owned worker");
    assert!(result.status.success());
    assert!(
        result.stderr.is_empty(),
        "private payload must never reach diagnostic stderr"
    );
}

#[test]
#[cfg(feature = "isolated-codecs")]
fn codec_schema2_requires_native_source_ack_and_schema1_is_explicitly_rejected() {
    use serde_json::{Value, json};
    use std::process::{Command, Stdio};
    for schema in [1, 2] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
            .arg("--archive-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("owned worker");
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/upstream/lzma.7z");
        let mut request = WorkerRequest::open(Uuid::new_v4(), path.to_string_lossy().into_owned());
        request.version = schema;
        write_frame(child.stdin.as_mut().expect("stdin"), &request).expect("start");
        let accepted: Value = read_frame(child.stdout.as_mut().expect("stdout"))
            .expect("frame")
            .expect("accepted");
        assert_eq!(accepted["op"], "accepted");
        if schema == 2 {
            assert!(
                accepted["capabilities"]
                    .as_array()
                    .expect("negotiated capabilities")
                    .contains(&json!("source_lease_v1"))
            );
            let call: Value = read_frame(child.stdout.as_mut().expect("stdout"))
                .expect("frame")
                .expect("source call");
            assert_eq!(call["native"]["action"], "source_acquire");
            write_frame(child.stdin.as_mut().expect("stdin"), &json!({"version":1,"operation_id":request.operation_id,
                "op":"native_result","call_id":call["native"]["call_id"],"success":false,"error_code":"source_changed"})).expect("deny source lease");
        }
        let terminal: Value = read_frame(child.stdout.as_mut().expect("stdout"))
            .expect("frame")
            .expect("typed failure");
        assert_eq!(
            terminal["error_code"],
            if schema == 1 {
                "UnsupportedOperation"
            } else {
                "SourceChanged"
            }
        );
        drop(child.stdin.take());
        let result = child.wait_with_output().expect("owned cleanup");
        assert!(result.status.success());
        assert!(result.stderr.is_empty());
    }
}

#[test]
fn creation_fields_are_opt_in_schema2_and_roundtrip_without_changing_legacy_requests() {
    let mut request = WorkerRequest::open(Uuid::new_v4(), "source.bin".into());
    let legacy = serde_json::to_value(&request).expect("legacy wire");
    assert!(legacy.get("create_name").is_none());
    assert!(legacy.get("compression_level").is_none());
    request.operation = "create_gzip".into();
    request.create_name = Some("created.gz".into());
    request.compression_level = Some("normal".into());
    assert!(
        request.validate().is_err(),
        "schema1 cannot use new capability"
    );
    request.version = 2;
    let decoded: WorkerRequest =
        serde_json::from_value(serde_json::to_value(&request).expect("new wire"))
            .expect("new decode");
    assert!(decoded.validate().is_ok());
    assert_eq!(decoded.create_name.as_deref(), Some("created.gz"));
    for invalid in ["../bad", "C:bad", "NUL.gz", ""] {
        request.create_name = Some(invalid.into());
        assert!(request.validate().is_err());
    }
    request.create_name = Some("valid.gz".into());
    request.compression_level = Some("store".into());
    assert!(request.validate().is_err());
    request.operation = "create_xz".into();
    request.compression_level = Some("normal".into());
    assert!(request.validate().is_ok());
    let decoded: WorkerRequest =
        serde_json::from_slice(&serde_json::to_vec(&request).expect("XZ encode"))
            .expect("XZ decode");
    assert_eq!(decoded.operation, "create_xz");
    request.version = 1;
    assert!(request.validate().is_err());
}

#[test]
fn zip_create_options_are_explicit_bounded_and_do_not_expand_legacy_requests() {
    use serde_json::json;
    let legacy = WorkerRequest::open(Uuid::new_v4(), "source".into());
    assert!(
        serde_json::to_value(&legacy)
            .expect("legacy")
            .get("zip_options")
            .is_none()
    );
    let valid = json!({"schema":2,"operation_id":Uuid::new_v4(),"op":"start","operation":"create_zip","path":"authorized","create_name":"out.zip","compression_level":"normal","zip_options":{"method":"deflate","encryption":"none","strip_single_root":false,"exclude_patterns":[]}});
    let decoded: WorkerRequest = serde_json::from_value(valid.clone()).expect("ZIP request");
    assert!(decoded.validate().is_ok());
    for (field, value) in [
        ("schema", json!(1)),
        ("operation", json!("open")),
        ("create_name", json!("../escape")),
        ("compression_level", json!("store")),
    ] {
        let mut bad = valid.clone();
        bad[field] = value;
        assert!(
            serde_json::from_value::<WorkerRequest>(bad)
                .expect("typed fields")
                .validate()
                .is_err()
        );
    }
    for (field, value) in [
        ("method", json!("bzip2")),
        ("encryption", json!("aes128")),
        ("encryption", json!("aes256")),
        ("exclude_patterns", json!(["x".repeat(9000)])),
    ] {
        let mut bad = valid.clone();
        bad["zip_options"][field] = value;
        assert!(
            serde_json::from_value::<WorkerRequest>(bad)
                .expect("typed options")
                .validate()
                .is_err()
        );
    }
    let mut aes = valid.clone();
    aes["zip_options"]["encryption"] = json!("aes256");
    aes["password"] = json!("private synthetic");
    assert!(
        serde_json::from_value::<WorkerRequest>(aes)
            .expect("AES wire")
            .validate()
            .is_ok()
    );
    let mut unknown = valid;
    unknown["zip_options"]["unreviewed"] = json!(true);
    assert!(serde_json::from_value::<WorkerRequest>(unknown).is_err());
}

#[test]
fn sevenz_creation_is_explicit_and_legacy_wire_omits_tree_options() {
    use serde_json::json;
    let legacy =
        serde_json::to_value(WorkerRequest::open(Uuid::new_v4(), "source".into())).expect("legacy");
    assert!(legacy.get("tree_options").is_none() && legacy.get("sevenz_options").is_none());
    let valid = json!({"schema":2,"operation_id":Uuid::new_v4(),"op":"start","operation":"create_7z","path":"authorized","create_name":"out.7z","compression_level":"normal","tree_options":{"strip_single_root":false,"exclude_patterns":[]},"sevenz_options":{"solid":false,"encryption":"none","encrypt_header":false}});
    assert!(
        serde_json::from_value::<WorkerRequest>(valid.clone())
            .expect("new request")
            .validate()
            .is_ok()
    );
    for (field, value) in [
        ("schema", json!(1)),
        ("operation", json!("test")),
        ("compression_level", json!("store")),
        ("password", json!("unrequested")),
    ] {
        let mut bad = valid.clone();
        bad[field] = value;
        assert!(
            serde_json::from_value::<WorkerRequest>(bad)
                .expect("typed request")
                .validate()
                .is_err()
        );
    }
    for (field, value) in [
        ("encryption", json!("aes128")),
        ("encryption", json!("aes256")),
        ("encrypt_header", json!(true)),
    ] {
        let mut bad = valid.clone();
        bad["sevenz_options"][field] = value;
        assert!(
            serde_json::from_value::<WorkerRequest>(bad)
                .expect("typed options")
                .validate()
                .is_err()
        );
    }
}

#[test]
fn tar_creation_options_reject_unrequested_levels_passwords_and_codec_options() {
    use serde_json::json;
    for format in ["tar", "tgz", "txz"] {
        let mut valid = json!({"schema":2,"operation_id":Uuid::new_v4(),"op":"start","operation":format!("create_{format}"),"path":"authorized","create_name":format!("out.{format}"),"tree_options":{"strip_single_root":true,"exclude_patterns":[]}});
        if format != "tar" {
            valid["compression_level"] = json!("normal");
        }
        assert!(
            serde_json::from_value::<WorkerRequest>(valid.clone())
                .expect("new request")
                .validate()
                .is_ok()
        );
        for (field, value) in [
            ("schema", json!(1)),
            ("password", json!("not-supported")),
            ("compression_level", json!("store")),
            (
                "sevenz_options",
                json!({"solid":true,"encryption":"none","encrypt_header":false}),
            ),
        ] {
            let mut bad = valid.clone();
            bad[field] = value;
            assert!(
                serde_json::from_value::<WorkerRequest>(bad)
                    .expect("typed request")
                    .validate()
                    .is_err()
            );
        }
        if format == "tar" {
            valid["compression_level"] = json!("normal");
            assert!(
                serde_json::from_value::<WorkerRequest>(valid)
                    .expect("typed request")
                    .validate()
                    .is_err()
            );
        }
    }
}

#[test]
fn extraction_metadata_options_are_schema2_extract_only_and_legacy_omits_them() {
    let base = serde_json::json!({"schema":2,"operation_id":Uuid::new_v4(),"op":"start","operation":"extract","path":"synthetic.zip"});
    let legacy: WorkerRequest = serde_json::from_value(base.clone()).expect("old private request");
    assert!(legacy.validate().is_ok());
    assert!(
        serde_json::to_value(legacy)
            .expect("old wire")
            .get("extract_options")
            .is_none()
    );
    for copy in [true, false] {
        let mut json = base.clone();
        json["extract_options"] = serde_json::json!({"copy_motw":copy});
        let request: WorkerRequest =
            serde_json::from_value(json.clone()).expect("new bounded extract options");
        assert!(request.validate().is_ok());
        assert_eq!(
            serde_json::to_value(request).expect("roundtrip")["extract_options"]["copy_motw"],
            copy
        );
        for (field, value) in [
            ("schema", serde_json::json!(1)),
            ("operation", serde_json::json!("test")),
            ("operation", serde_json::json!("open")),
        ] {
            let mut invalid = json.clone();
            invalid[field] = value;
            assert!(
                serde_json::from_value::<WorkerRequest>(invalid)
                    .expect("typed invalid combination")
                    .validate()
                    .is_err()
            );
        }
        json["extract_options"]["stream_name"] = serde_json::json!("not-authorized");
        assert!(serde_json::from_value::<WorkerRequest>(json).is_err());
    }
}

#[test]
fn legacy_or_unnegotiated_extraction_cannot_request_source_deletion() {
    let mut request = WorkerRequest::open(Uuid::new_v4(), "synthetic.zip".into());
    assert!(
        serde_json::to_value(&request)
            .expect("legacy wire")
            .get("delete_source")
            .is_none()
    );
    request.delete_source = Some(true);
    request.operation = "extract".into();
    assert!(
        request.validate().is_err(),
        "schema1 has no deletion authority"
    );
    request.version = 2;
    assert!(
        request.validate().is_err(),
        "old raw host lacks extraction metadata/MOTW negotiation"
    );
    request.extract_options = Some(nxtools_archive::worker::ExtractOptions { copy_motw: true });
    assert!(request.validate().is_ok());
    request.operation = "test".into();
    assert!(request.validate().is_err(), "test-only never delete source");
}

#[test]
fn read_v2_requires_exact_transport_contract_and_never_grants_write_authority() {
    let mut request = WorkerRequest::open(Uuid::new_v4(), "synthetic.zip".into());
    assert!(
        serde_json::to_value(&request)
            .expect("legacy")
            .get("read_transport")
            .is_none()
    );
    request.version = 2;
    request.read_contract = Some(2);
    request.limits.max_entries = 100000;
    request.limits.max_metadata_bytes = 128 << 20;
    assert!(request.validate().is_err());
    request.read_transport = Some(nxtools_archive::worker::ReadTransport {
        max_total_bytes: 64 << 20,
        browser_context: false,
    });
    assert!(request.validate().is_ok());
    for operation in ["extract", "create_zip", "create_7z"] {
        request.operation = operation.into();
        assert!(request.validate().is_err());
    }
    request.operation = "test".into();
    assert!(request.validate().is_ok());
    request.read_contract = Some(1);
    assert!(request.validate().is_err());
    request.read_contract = Some(2);
    request
        .read_transport
        .as_mut()
        .expect("transport")
        .max_total_bytes = 0;
    assert!(request.validate().is_err());
    request
        .read_transport
        .as_mut()
        .expect("transport")
        .max_total_bytes = (64 << 20) + 1;
    assert!(request.validate().is_err());
}

#[test]
fn password_interaction_is_private_opt_in_open_test_extract_and_legacy_wire_stays_absent() {
    let mut request = WorkerRequest::open(Uuid::new_v4(), "synthetic.rar".into());
    assert!(
        serde_json::to_value(&request)
            .expect("old request")
            .get("password_interaction")
            .is_none()
    );
    request.password_interaction = Some(true);
    assert!(request.validate().is_err(), "schema1 cannot opt in");
    request.version = 2;
    for operation in ["open", "test", "extract"] {
        request.operation = operation.into();
        assert!(request.validate().is_ok());
    }
    request.read_contract = Some(1);
    assert!(
        request.validate().is_err(),
        "accepted read contract stays password-free"
    );
    request.read_contract = None;
    request.operation = "create_gzip".into();
    assert!(
        request.validate().is_err(),
        "creation does not gain password interaction"
    );
    assert!(
        serde_json::to_value(WorkerFrame::complete(request.operation_id, 1, 18))
            .expect("terminal")
            .get("password_state")
            .is_none()
    );
}

#[test]
fn zip_conflict_policy_is_opt_in_extract_only_and_old_terminal_omits_new_fields() {
    let request = WorkerRequest::open(Uuid::new_v4(), "synthetic.zip".into());
    let mut value = serde_json::to_value(&request).expect("legacy request");
    assert!(value.get("conflict_policy").is_none());
    value["conflict_policy"] = serde_json::json!("ask");
    for (schema, operation, valid) in [
        (1, "extract", false),
        (2, "open", false),
        (2, "test", false),
        (2, "extract", true),
    ] {
        value["schema"] = serde_json::json!(schema);
        value["operation"] = serde_json::json!(operation);
        let decoded: WorkerRequest =
            serde_json::from_value(value.clone()).expect("typed private conflict request");
        assert_eq!(decoded.validate().is_ok(), valid);
    }
    value["read_contract"] = serde_json::json!(1);
    assert!(
        serde_json::from_value::<WorkerRequest>(value)
            .expect("read request")
            .validate()
            .is_err()
    );
    let old = serde_json::to_value(WorkerFrame::complete(request.operation_id, 1, 0))
        .expect("legacy terminal");
    assert!(old.get("conflict_prompt").is_none() && old.get("extraction_summary").is_none());
}

#[test]
fn risk_test_seam_cannot_disable_raise_thresholds_or_mutate_read_contracts() {
    let request = WorkerRequest::open(Uuid::new_v4(), "synthetic.zip".into());
    let legacy = serde_json::to_value(&request).expect("legacy request");
    assert!(legacy.get("zip_risk_fixture").is_none());
    assert!(
        serde_json::to_value(WorkerFrame::complete(request.operation_id, 1, 0))
            .expect("terminal")
            .get("risk_prompt")
            .is_none()
    );
    let mut wire = legacy;
    wire["schema"] = serde_json::json!(2);
    wire["operation"] = serde_json::json!("extract");
    for (bytes, ratio, valid) in [
        (1024u64, 2u64, true),
        (0, 2, false),
        ((64 << 20) + 1, 2, false),
        (1024, 0, false),
        (1024, 1001, false),
    ] {
        wire["zip_risk_fixture"] = serde_json::json!({"bytes":bytes,"ratio":ratio});
        #[cfg(debug_assertions)]
        assert_eq!(
            serde_json::from_value::<WorkerRequest>(wire.clone())
                .expect("strict private seam")
                .validate()
                .is_ok(),
            valid
        );
        #[cfg(not(debug_assertions))]
        {
            let _ = valid;
            assert!(serde_json::from_value::<WorkerRequest>(wire.clone()).is_err());
        }
    }
    wire["zip_risk_fixture"] = serde_json::json!({"bytes":1024,"ratio":2});
    wire["operation"] = serde_json::json!("open");
    #[cfg(debug_assertions)]
    assert!(
        serde_json::from_value::<WorkerRequest>(wire.clone())
            .expect("typed open")
            .validate()
            .is_err()
    );
    wire["read_contract"] = serde_json::json!(1);
    #[cfg(debug_assertions)]
    assert!(
        serde_json::from_value::<WorkerRequest>(wire)
            .expect("typed read")
            .validate()
            .is_err()
    );
}

#[test]
fn output_readmission_is_strict_private_extract_only_and_absent_from_legacy_wire() {
    use serde_json::json;
    let legacy = WorkerRequest::open(Uuid::new_v4(), "synthetic.zip".into());
    let mut value = serde_json::to_value(legacy).expect("legacy");
    assert!(value.get("output_readmission").is_none());
    value["schema"] = json!(2);
    value["operation"] = json!("extract");
    value["output_readmission"] = json!({"logical_id": Uuid::new_v4(), "expected_source": null});
    assert!(
        serde_json::from_value::<WorkerRequest>(value.clone())
            .expect("opt in")
            .validate()
            .is_ok()
    );
    for (field, bad) in [
        ("schema", json!(1)),
        ("operation", json!("open")),
        ("operation", json!("test")),
        ("read_contract", json!(1)),
        ("delete_source", json!(true)),
    ] {
        let mut invalid = value.clone();
        invalid[field] = bad;
        assert!(
            serde_json::from_value::<WorkerRequest>(invalid)
                .expect("typed")
                .validate()
                .is_err()
        );
    }
    let mut unknown = value.clone();
    unknown["output_readmission"]["unknown"] = json!(true);
    assert!(serde_json::from_value::<WorkerRequest>(unknown).is_err());
    for native in ["".to_owned(), "x".repeat(257)] {
        let mut invalid = value.clone();
        invalid["output_readmission"]["expected_source"] =
            json!({"native":native,"digest":vec![0;32]});
        assert!(
            serde_json::from_value::<WorkerRequest>(invalid)
                .expect("source shape")
                .validate()
                .is_err()
        );
    }
}

#[test]
fn optional_member_metadata_missing_or_null_remains_unknown_not_zero() {
    for packed in [
        None,
        Some(serde_json::Value::Null),
        Some(serde_json::json!(1446)),
    ] {
        let mut value = serde_json::json!({"ordinal":0,"name":"member","kind":"File","size":3447,"warning_codes":[]});
        if let Some(packed) = packed {
            value["packed_size"] = packed;
        }
        let entry: nxtools_archive::worker::WorkerEntry =
            serde_json::from_value(value.clone()).expect("optional fields");
        assert_eq!(
            entry.packed_size,
            value.get("packed_size").and_then(serde_json::Value::as_u64)
        );
        assert_eq!(entry.encrypted, None);
        assert_eq!(entry.provenance, None);
        assert_eq!(entry.codepage, None);
        let encoded = serde_json::to_value(entry).expect("roundtrip unknown");
        assert_eq!(
            encoded["packed_size"],
            value
                .get("packed_size")
                .cloned()
                .unwrap_or(serde_json::Value::Null)
        );
        assert!(encoded.get("provenance").is_none() && encoded.get("codepage").is_none());
    }
}

#[test]
fn shared_conflict_capability_preserves_zip_marker_and_admits_standalone_streams() {
    use serde_json::{Value, json};
    use std::process::{Command, Stdio};
    for policy in [None, Some("skip")] {
        let mut request = WorkerRequest::open(
            Uuid::new_v4(),
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/independent.gz")
                .to_string_lossy()
                .into_owned(),
        );
        request.version = 2;
        request.operation = "extract".into();
        request.conflict_policy =
            policy.map(|value| serde_json::from_value(json!(value)).expect("typed policy"));
        let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
            .arg("--archive-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("owned worker");
        write_frame(child.stdin.as_mut().expect("stdin"), &request).expect("request");
        let accepted: Value = read_frame(child.stdout.as_mut().expect("stdout"))
            .expect("frame")
            .expect("accepted");
        let caps = accepted["capabilities"].as_array().expect("capabilities");
        assert_eq!(
            caps.contains(&json!("extraction_conflict_v1")),
            policy.is_some()
        );
        assert_eq!(caps.contains(&json!("zip_conflict_v1")), policy.is_some());
        let acquire: Value = read_frame(child.stdout.as_mut().expect("stdout"))
            .expect("source call")
            .expect("acquire");
        assert_eq!(acquire["native"]["action"], "source_acquire");
        if policy.is_some() {
            write_frame(child.stdin.as_mut().expect("stdin"), &json!({"version":1,"operation_id":request.operation_id,"op":"native_result","call_id":acquire["native"]["call_id"],"success":true})).expect("trusted fixture lease");
            let observe: Value = read_frame(child.stdout.as_mut().expect("stdout"))
                .expect("target observation")
                .expect("original native call");
            assert_eq!(observe["native"]["action"], "observe_target");
            write_frame(child.stdin.as_mut().expect("stdin"), &json!({"version":1,"operation_id":request.operation_id,"op":"native_result","call_id":observe["native"]["call_id"],"success":true,"token":Uuid::new_v4(),"existing_bytes":10})).expect("ordinary target");
            let summary: Value = read_frame(child.stdout.as_mut().expect("stdout"))
                .expect("summary frame")
                .expect("Skip summary");
            assert_eq!(summary["extraction_summary"]["files_skipped"], 1);
            assert_eq!(summary["extraction_summary"]["all_members_written"], false);
            let complete: Value = read_frame(child.stdout.as_mut().expect("stdout"))
                .expect("complete frame")
                .expect("verified Skip terminal");
            assert_eq!(complete["op"], "complete");
            assert_eq!(complete["files_verified"], 0);
            assert_eq!(complete["bytes_verified"], 0);
        }
        drop(child.stdin.take());
        let result = child.wait_with_output().expect("cleanup");
        assert!(result.status.success() && result.stderr.is_empty());
    }
}

#[test]
fn browser_password_private_authorization_has_no_read_output_or_prompt_authority() {
    use nxtools_protocol::archive_browser::*;
    let context = ArchiveBrowserContext {
        snapshot_id: Uuid::new_v4(),
        format: "Zip".into(),
        comment: String::new(),
        source: ArchiveSourceBinding {
            native: "v1:test".into(),
            digest: [1; 32],
            size: 1,
            modified_unix_ns: 1,
        },
    };
    let mut prepare = WorkerRequest::open(Uuid::new_v4(), r"C:\owned\source.zip".into());
    prepare.version = 2;
    prepare.operation = "test".into();
    prepare.password_interaction = Some(true);
    prepare.browser_authorize = Some(context.clone());
    prepare.limits.max_entry_bytes = 512 << 20;
    prepare.limits.max_output_bytes = 512 << 20;
    assert!(prepare.validate().is_ok());
    prepare.operation = "extract".into();
    assert!(prepare.validate().is_err());
    prepare.operation = "test".into();
    prepare.read_contract = Some(1);
    assert!(prepare.validate().is_err());
    let mut materialize = WorkerRequest::open(Uuid::new_v4(), r"C:\owned\source.zip".into());
    materialize.version = 2;
    materialize.operation = "extract".into();
    materialize.browser_entry = Some(nxtools_archive::worker_browser::BrowserEntry {
        ordinal: 0,
        expected: context,
    });
    materialize.browser_password = Some(b"private-fixture-password".to_vec());
    materialize.limits.max_entry_bytes = 512 << 20;
    materialize.limits.max_output_bytes = 512 << 20;
    materialize.limits.max_entries = 100_000;
    materialize.limits.max_metadata_bytes = 128 << 20;
    materialize.extract_options =
        Some(nxtools_archive::worker::ExtractOptions { copy_motw: false });
    assert!(materialize.validate().is_ok());
    materialize.password_interaction = Some(true);
    assert!(materialize.validate().is_err());
    materialize.password_interaction = None;
    materialize.browser_entry = None;
    assert!(materialize.validate().is_err());
}
