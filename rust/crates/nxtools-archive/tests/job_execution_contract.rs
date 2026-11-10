#[path = "common/bulk_peer.rs"]
mod bulk_peer;
use bulk_peer::{read_frame, write_frame};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    process::{Command, Stdio},
};
use uuid::Uuid;

// Actual private worker entry; no alternative codec/extractor. Only native handle replies are synthetic.
#[test]
fn general_extract_smart_uses_original_plan_and_exact_native_transaction_paths() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
        .arg("--archive-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("owned tiny worker");
    let id = Uuid::new_v4();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/deflated.zip");
    write_frame(child.stdin.as_mut().expect("stdin"), &json!({
        "schema":2,"bulk_transfer":1,"operation_id":id,"op":"start","operation":"extract","path":path,
        "job_extract":{"strategy":"smart","single_file_folder":false,"conflict":"skip","copy_motw":false,"selection":null}
    })).expect("start");
    let mut paths = BTreeMap::new();
    let mut active = BTreeMap::<Uuid, String>::new();
    let mut committed = 0;
    loop {
        let frame: Option<Value> =
            read_frame(child.stdout.as_mut().expect("stdout")).expect("bounded frame");
        let Some(frame) = frame else {
            let result = child.wait_with_output().expect("owned cleanup");
            panic!(
                "general execution must reach native transaction, not reject new plan: exit={:?}",
                result.status.code()
            );
        };
        if frame["op"] == "complete" {
            assert_eq!(frame["files_verified"], 3);
            break;
        }
        assert_ne!(
            frame["op"], "failure",
            "typed worker failure: {}",
            frame["error_code"]
        );
        if frame["op"] != "native_call" {
            continue;
        }
        let call = &frame["native"];
        let mut reply = json!({"version":1,"operation_id":id,"op":"native_result","call_id":call["call_id"],"success":true});
        match call["action"].as_str().expect("action") {
            "begin" => {
                let token = Uuid::new_v4();
                let path = call["path"].as_str().expect("relative").to_owned();
                paths.insert(path.clone(), Vec::<u8>::new());
                active.insert(token, path);
                reply["token"] = json!(token);
            }
            "write" => {
                let token: Uuid = serde_json::from_value(call["token"].clone()).expect("token");
                paths
                    .get_mut(&active[&token])
                    .expect("held partial")
                    .extend(
                        call["data"]
                            .as_array()
                            .expect("bytes")
                            .iter()
                            .map(|v| v.as_u64().expect("byte") as u8),
                    );
            }
            "commit" => committed += 1,
            "directory" | "directory_new" | "source_acquire" | "source_validated"
            | "source_codec_open" | "source_complete" | "source_precommit" | "file_metadata"
            | "directory_metadata" => {}
            action => panic!("unexpected native action {action}"),
        }
        write_frame(child.stdin.as_mut().expect("stdin"), &reply).expect("native result");
    }
    drop(child.stdin.take());
    let result = child.wait_with_output().expect("cleanup");
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    assert_eq!(committed, 3);
    assert_eq!(paths["deflated/资料/说明.txt"], "你好，压缩包\n".as_bytes());
    assert_eq!(paths["deflated/bytes.bin"], (0u8..=255).collect::<Vec<_>>());
}

#[test]
fn product_readmission_adapter_preserves_original_intent_and_native_terminal() {
    use nxtools_archive::{
        OperationControl,
        interaction::{InteractionPort, Prompt, PromptResponse},
    };
    use nxtools_protocol::archive_job::*;
    let id = Uuid::new_v4();
    let control = OperationControl::with_operation_id(id);
    let interaction = InteractionPort::new(id, 1);
    let port = interaction.clone();
    let answer = std::thread::spawn(move || {
        let deadline = std::time::Instant::now();
        loop {
            if let Some(pending) = port.pending() {
                let Prompt::OutputBudget { required, .. } = pending.prompt else {
                    panic!("budget only")
                };
                assert!(
                    port.respond(
                        pending.ticket,
                        PromptResponse::OutputBudget { approved: required }
                    )
                    .expect("original reply")
                );
                break;
            }
            assert!(deadline.elapsed().as_secs() < 5);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    });
    let command:ArchiveJobCommand=serde_json::from_value(json!({"version":1,"operation_id":id,"session_id":Uuid::new_v4(),"generation":8,"action":"start","request":{"kind":"extract","paths":["C:\\owned\\tiny.zip"],"destination":"C:\\owned\\output","limits":{"max_entry_bytes":1,"max_output_bytes":1},"extract":{"strategy":"name","single_file_folder":false,"conflict":"skip","copy_motw":true,"selection":null},"create":null}})).expect("typed request");
    let mut attempts = 0;
    let terminal = nxtools_archive::product_job::execute_with_output_readmission(
        command,
        &control,
        &interaction,
        |command, _| {
            attempts += 1;
            let request = command.request.expect("immutable attempt");
            assert_eq!(request.destination.as_deref(), Some("C:\\owned\\output"));
            assert_eq!(
                request.extract.expect("extract").strategy,
                ArchiveJobStrategy::Name
            );
            let shortfall = if attempts == 1 {
                Some(ArchiveJobOutputShortfall {
                    current: ArchiveJobOutputBudget {
                        entry_bytes: 1,
                        output_bytes: 1,
                    },
                    required: ArchiveJobOutputBudget {
                        entry_bytes: 8,
                        output_bytes: 8,
                    },
                    source: ArchiveJobOutputSource {
                        native: "v1:original".into(),
                        digest: [7; 32],
                    },
                })
            } else {
                assert_ne!(command.operation_id, id);
                assert_eq!(
                    request
                        .readmission
                        .expect("readmitted")
                        .expected_source
                        .expect("binding")
                        .digest,
                    [7; 32]
                );
                assert_eq!(request.limits.max_output_bytes, 8);
                None
            };
            Ok(ArchiveJobTerminal {
                outcome: if attempts == 1 {
                    ArchiveJobOutcome::Failed
                } else {
                    ArchiveJobOutcome::Succeeded
                },
                code: if attempts == 1 {
                    Some("resource_limit".into())
                } else {
                    None
                },
                files_committed: if attempts == 1 { 0 } else { 1 },
                bytes_committed: if attempts == 1 { 0 } else { 8 },
                files_verified: 0,
                bytes_verified: 0,
                files_skipped: 0,
                cleanup_complete: true,
                commit_state: None,
                elapsed_ms: 1,
                native_error_code: None,
                journal: vec![],
                output_shortfall: shortfall,
                result_location: None,
                output_published: None,
                add_result: None,
                journal_summary: None,
            })
        },
    )
    .expect("original finite readmission");
    answer.join().expect("answer");
    assert_eq!(attempts, 2);
    assert_eq!(terminal.bytes_committed, 8);
}
