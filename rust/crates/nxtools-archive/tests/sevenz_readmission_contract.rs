#![cfg(feature = "isolated-codecs")]
#[path = "common/bulk_peer.rs"]
mod bulk_peer;
use bulk_peer::{read_frame, write_frame};
use nxtools_archive::{output_readmission::ReadmissionOptions, worker::WorkerRequest};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};
use uuid::Uuid;

#[test]
fn sevenz_tiny_total_shortage_reaches_original_declaration_before_any_output() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upstream/lzma.7z");
    assert!(
        std::fs::metadata(&source)
            .expect("independent fixture")
            .len()
            > 1
    );
    let id = Uuid::new_v4();
    let mut request = WorkerRequest::open(id, source.to_string_lossy().into_owned());
    request.version = 2;
    request.operation = "extract".into();
    request.bulk_transfer = Some(1);
    request.limits.max_entry_bytes = 1024;
    request.limits.max_output_bytes = 1;
    request.output_readmission = Some(ReadmissionOptions {
        logical_id: id,
        expected_source: None,
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
        .arg("--archive-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("original isolated worker");
    write_frame(child.stdin.as_mut().expect("input"), &request).expect("original request");
    let mut shortage = None;
    let mut actions = Vec::new();
    loop {
        let frame: Value = read_frame(child.stdout.as_mut().expect("output"))
            .expect("bounded frame")
            .expect("terminal");
        if frame["op"] == "native_call" {
            let native = &frame["native"];
            let action = native["action"].as_str().expect("action");
            actions.push(action.to_owned());
            assert!(
                !matches!(
                    action,
                    "output_ready" | "directory" | "begin" | "write" | "commit"
                ),
                "shortage must precede output"
            );
            let mut reply = json!({"version":1,"operation_id":id,"op":"native_result","call_id":native["call_id"],"success":true});
            if action == "source_stamp" {
                reply["read_data"] = json!("held-native-source");
            }
            write_frame(child.stdin.as_mut().expect("input"), &reply).expect("same native control");
        }
        if frame["op"] == "output_budget_shortfall" {
            shortage = Some(frame["output_budget_shortfall"].clone());
        }
        if matches!(frame["op"].as_str(), Some("failure" | "complete")) {
            assert_eq!(frame["error_code"], "ResourceLimit");
            break;
        }
    }
    drop(child.stdin.take());
    let status = child.wait_with_output().expect("worker exit");
    assert!(status.status.success() && status.stderr.is_empty());
    let shortage = shortage.expect(
        "7z must report reliable declared total shortage, not encoded-source ResourceLimit",
    );
    assert_eq!(shortage["current"]["output_bytes"], 1);
    assert_eq!(shortage["required"]["entry_bytes"], 15);
    assert_eq!(shortage["required"]["output_bytes"], 15);
    assert_eq!(shortage["source"]["native"], "held-native-source");
    assert!(actions.iter().any(|a| a == "source_precommit"));
}
