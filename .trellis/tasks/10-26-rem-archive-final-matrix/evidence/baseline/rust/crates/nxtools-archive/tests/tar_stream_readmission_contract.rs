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

fn run(name: &str, entry: u64, total: u64) -> (Value, Vec<Value>, Vec<String>) {
    let root = PathBuf::from(
        std::env::var_os("NXTOOLS_TAR_STREAM_FIXTURES").expect("independent owned fixtures"),
    );
    let id = Uuid::new_v4();
    let mut request = WorkerRequest::open(id, root.join(name).to_string_lossy().into_owned());
    request.version = 2;
    request.operation = "extract".into();
    request.limits.max_entry_bytes = entry;
    request.limits.max_output_bytes = total;
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
        .expect("original worker");
    write_frame(child.stdin.as_mut().expect("input"), &request).expect("request");
    let mut frames = Vec::new();
    let mut actions = Vec::new();
    let terminal = loop {
        let frame: Value = read_frame(child.stdout.as_mut().expect("output"))
            .expect("frame")
            .expect("terminal before EOF");
        assert_eq!(frame["operation_id"], id.to_string());
        if frame["op"] == "native_call" {
            let call = &frame["native"];
            let action = call["action"].as_str().expect("action");
            actions.push(action.to_owned());
            let mut reply = json!({"version":1,"operation_id":id,"op":"native_result","call_id":call["call_id"],"success":true});
            if action == "source_stamp" {
                reply["read_data"] = json!("original-held-source");
            }
            if action == "begin" {
                reply["token"] = json!(Uuid::new_v4());
            }
            write_frame(child.stdin.as_mut().expect("input"), &reply).expect("original reply");
        }
        let done = matches!(frame["op"].as_str(), Some("failure" | "complete"));
        frames.push(frame.clone());
        if done {
            break frame;
        }
    };
    drop(child.stdin.take());
    let result = child.wait_with_output().expect("worker exit");
    assert!(result.status.success() && result.stderr.is_empty());
    (terminal, frames, actions)
}
#[test]
fn known_xz_and_complete_zstd_declarations_reach_shortfall_before_output() {
    let mut missing = Vec::new();
    for (name, required) in [
        ("known.xz", 4096),
        ("known.zst", 4096),
        ("two-known.zst", 8192),
    ] {
        let (terminal, frames, actions) = run(name, 1, 1);
        assert_eq!(terminal["error_code"], "ResourceLimit");
        assert!(
            !actions.iter().any(|action| matches!(
                action.as_str(),
                "output_ready" | "begin" | "write" | "commit"
            )),
            "declaration before effects"
        );
        if let Some(frame) = frames
            .iter()
            .find(|frame| frame["op"] == "output_budget_shortfall")
        {
            let shortage = &frame["output_budget_shortfall"];
            assert_eq!(shortage["required"]["entry_bytes"], required);
            assert_eq!(shortage["required"]["output_bytes"], required);
            assert_eq!(shortage["source"]["native"], "original-held-source");
        } else {
            missing.push(name);
        }
    }
    assert!(
        missing.is_empty(),
        "complete original stream declarations must reach finite preflight: {missing:?}"
    );
}
#[test]
fn unknown_stream_actual_failure_never_becomes_declaration_shortage() {
    for name in ["single.gz", "single.bz2", "unknown.zst", "mixed.zst"] {
        let (terminal, frames, actions) =
            run(name, if name == "mixed.zst" { 6144 } else { 3072 }, 1 << 20);
        assert_eq!(terminal["error_code"], "ResourceLimit", "{name}");
        assert!(
            !frames
                .iter()
                .any(|frame| frame["op"] == "output_budget_shortfall"),
            "{name}"
        );
        assert!(!actions.iter().any(|action| action == "commit"), "{name}");
    }
}
