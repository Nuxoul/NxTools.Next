//! Actual worker rendezvous: schema2 alone does not grant an old host new native actions.
use nxtools_archive::worker::{ExtractOptions, WorkerRequest, read_frame, write_frame};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    path::PathBuf,
    process::{Command, Stdio},
};
use uuid::Uuid;

#[test]
fn actual_worker_preserves_old_schema2_native_actions_and_explicit_metadata_negotiation() {
    #[cfg(all(windows, feature = "isolated-codecs"))]
    let fixtures = [
        "deflated.zip",
        "upstream/lzma.7z",
        "upstream/rar4.rar",
        "upstream/rar5.rar",
    ];
    #[cfg(not(all(windows, feature = "isolated-codecs")))]
    let fixtures = ["deflated.zip"];
    for fixture in fixtures {
        for copy in [None, Some(false), Some(true)] {
            let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
                .arg("--archive-worker")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("owned worker");
            let id = Uuid::new_v4();
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(fixture);
            let mut request = WorkerRequest::open(id, path.to_string_lossy().into_owned());
            request.version = 2;
            request.operation = "extract".into();
            request.extract_options = copy.map(|copy_motw| ExtractOptions { copy_motw, reclamation: None });
            write_frame(child.stdin.as_mut().expect("stdin"), &request).expect("request");
            let mut active = HashSet::new();
            let mut staged = HashSet::new();
            let mut commits = 0;
            let terminal = loop {
                let frame: Value = read_frame(child.stdout.as_mut().expect("stdout"))
                    .expect("frame")
                    .expect("terminal before EOF");
                assert_eq!(frame["operation_id"], id.to_string());
                if frame["op"] == "native_call" {
                    let call = &frame["native"];
                    let mut token = None;
                    match call["action"].as_str().expect("action") {
                        "source_acquire" | "source_validated" | "codec_opened"
                        | "source_decoding" | "source_precommit" | "directory" => {}
                        "begin" => {
                            let key = Uuid::new_v4();
                            active.insert(key);
                            token = Some(key);
                        }
                        "write" => {
                            assert!(
                                active.contains(
                                    &Uuid::parse_str(call["token"].as_str().expect("token"))
                                        .expect("uuid")
                                )
                            );
                        }
                        "file_metadata" => {
                            assert!(
                                copy.is_some(),
                                "old host must never receive an unknown metadata action"
                            );
                            let key = Uuid::parse_str(call["token"].as_str().expect("token"))
                                .expect("uuid");
                            assert!(active.contains(&key) && staged.insert(key));
                            serde_json::from_value::<nxtools_archive::metadata::OutputMetadata>(
                                call["metadata"].clone(),
                            )
                            .expect("typed metadata")
                            .validate()
                            .expect("valid source metadata");
                        }
                        "directory_metadata" => {
                            assert!(copy.is_some());
                        }
                        "commit" => {
                            let key = Uuid::parse_str(call["token"].as_str().expect("token"))
                                .expect("uuid");
                            assert!(active.remove(&key));
                            assert_eq!(staged.remove(&key), copy.is_some());
                            commits += 1;
                        }
                        action => panic!("unexpected action {action}"),
                    }
                    write_frame(child.stdin.as_mut().expect("stdin"), &json!({"version":1,"operation_id":id,"op":"native_result","call_id":call["call_id"],"success":true,"token":token})).expect("ack");
                } else if frame["op"] == "complete" || frame["op"] == "failure" {
                    break frame;
                }
            };
            assert_eq!(terminal["op"], "complete", "{fixture}/{copy:?}: {terminal}");
            assert!(commits > 0 && active.is_empty() && staged.is_empty());
            drop(child.stdin.take());
            let result = child.wait_with_output().expect("reaped worker");
            assert!(result.status.success() && result.stderr.is_empty());
        }
    }
}
