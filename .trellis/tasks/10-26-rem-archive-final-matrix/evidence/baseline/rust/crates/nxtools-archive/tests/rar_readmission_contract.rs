#![cfg(all(windows, feature = "isolated-codecs"))]
use nxtools_archive::{
    output_readmission::ReadmissionOptions,
    worker::{WorkerRequest, read_frame, write_frame},
};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};
use uuid::Uuid;

#[test]
fn rar4_rar5_tiny_total_reaches_original_declaration_before_output() {
    let mut missing = Vec::new();
    for (fixture, expected) in [("rar4.rar", 11), ("rar5.rar", 7)] {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/upstream")
            .join(fixture);
        let id = Uuid::new_v4();
        let mut request = WorkerRequest::open(id, source.to_string_lossy().into_owned());
        request.version = 2;
        request.operation = "extract".into();
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
            .expect("owned original worker");
        write_frame(child.stdin.as_mut().expect("input"), &request).expect("start");
        let mut shortage = None;
        let mut source_verified = false;
        loop {
            let frame: Value = read_frame(child.stdout.as_mut().expect("output"))
                .expect("bounded frame")
                .expect("terminal before EOF");
            assert_eq!(frame["operation_id"], id.to_string());
            if frame["op"] == "native_call" {
                let call = &frame["native"];
                let action = call["action"].as_str().expect("action");
                assert!(
                    !matches!(
                        action,
                        "output_ready"
                            | "directory"
                            | "directory_new"
                            | "begin"
                            | "write"
                            | "commit"
                            | "abort"
                    ),
                    "declaration precedes all output"
                );
                source_verified |= action == "source_precommit";
                let mut reply = json!({"version":1,"operation_id":id,"op":"native_result","call_id":call["call_id"],"success":true});
                if action == "source_stamp" {
                    reply["read_data"] = json!("held-native-source");
                }
                write_frame(child.stdin.as_mut().expect("input"), &reply).expect("original reply");
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
        let result = child.wait_with_output().expect("actual worker exit");
        assert!(result.status.success() && result.stderr.is_empty());
        if let Some(shortage) = shortage {
            assert_eq!(shortage["current"]["output_bytes"], 1);
            assert_eq!(shortage["required"]["entry_bytes"], expected);
            assert_eq!(shortage["required"]["output_bytes"], expected);
            assert_eq!(shortage["source"]["native"], "held-native-source");
            assert!(source_verified);
        } else {
            missing.push(fixture);
        }
    }
    assert!(
        missing.is_empty(),
        "RAR low output must reach declaration, not encoded-source rejection: {missing:?}"
    );
}
