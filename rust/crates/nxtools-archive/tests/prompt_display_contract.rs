//! Audit #4: real worker/InteractionPort with a controlled no-filesystem output adapter.
use nxtools_archive::worker::{WorkerRequest, read_frame, write_frame};
use serde_json::{Value, json};
use std::process::{Command, Stdio};
use uuid::Uuid;

fn run(fixture: &str, named: bool) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture);
    let id = Uuid::new_v4();
    let mut request = WorkerRequest::open(id, path.to_string_lossy().into_owned());
    request.version = 2;
    request.operation = "extract".into();
    request.conflict_policy = Some(serde_json::from_value(json!("ask")).expect("typed Ask policy"));
    if named {
        request.job_execution = Some(true);
        request.job_extract = Some(serde_json::from_value(json!({"strategy":"name","single_file_folder":false,"conflict":"ask","copy_motw":false,"selection":null})).expect("typed Name plan"));
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
        .arg("--archive-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("owned worker");
    write_frame(child.stdin.as_mut().expect("owned stdin"), &request)
        .expect("worker admission frame");
    let mut observed = String::new();
    let mut prefix = None;
    let mut tickets = vec![];
    loop {
        let frame: Value = read_frame(child.stdout.as_mut().expect("owned stdout"))
            .expect("bounded frame")
            .expect("terminal before EOF");
        match frame["op"].as_str().expect("typed frame kind") {
            "native_call" => {
                let call = &frame["native"];
                let mut reply = json!({"version":1,"operation_id":id,"op":"native_result","call_id":call["call_id"],"success":true});
                match call["action"].as_str().expect("typed native action") {
                    "observe_target" => {
                        observed = call["path"]
                            .as_str()
                            .expect("current observed path")
                            .to_owned();
                        reply["token"] = json!(Uuid::new_v4());
                        reply["existing_bytes"] = json!(10);
                    }
                    "directory_new" => {
                        prefix = Some(format!(
                            "{}/",
                            call["path"].as_str().expect("assigned subfolder")
                        ))
                    }
                    "source_acquire"
                    | "source_validated"
                    | "codec_opened"
                    | "source_decoding"
                    | "source_precommit"
                    | "destination_preflight"
                    | "directory"
                    | "directory_metadata" => {}
                    other => panic!("Skip must never allocate/write/publish: {other}"),
                }
                write_frame(child.stdin.as_mut().expect("owned stdin"), &reply)
                    .expect("fake native acknowledgement");
            }
            "conflict_prompt" => {
                let prompt = &frame["conflict_prompt"];
                let display: nxtools_protocol::archive_job::ArchiveJobPromptDisplay =
                    serde_json::from_value(prompt["display"].clone())
                        .expect("real producer display");
                assert_eq!(
                    display.target, observed,
                    "current native observation, not last progress"
                );
                let member = display
                    .incoming_member
                    .as_deref()
                    .expect("current source member");
                let expected = prefix
                    .as_deref()
                    .and_then(|prefix| observed.strip_prefix(prefix))
                    .unwrap_or(&observed);
                assert_eq!(member.replace('\\', "/").trim_end_matches('/'), expected);
                assert!(!format!("{display:?}").contains(member));
                assert_eq!(prompt["existing_bytes"], 10);
                assert!(!tickets.contains(&prompt["ticket"]));
                tickets.push(prompt["ticket"].clone());
                write_frame(child.stdin.as_mut().expect("owned stdin"), &json!({"version":1,"operation_id":id,"op":"conflict_reply","conflict_reply":{"ticket":prompt["ticket"],"decision":"skip","apply_to_all":false}})).expect("original ticket reply");
            }
            "complete" => {
                assert_eq!(frame["files_verified"], 0);
                break;
            }
            "failure" => panic!(
                "unexpected fixed worker category: {} ({fixture})",
                frame["error_code"]
            ),
            _ => {}
        }
    }
    assert!(
        !tickets.is_empty(),
        "existing supported producer must prompt: {fixture}"
    );
    if named {
        assert!(prefix.is_some(), "actual plan prefix assigned");
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("actual worker cleanup");
    assert!(output.status.success() && output.stderr.is_empty());
}
#[test]
fn audit04_original_engine_format_prompts_carry_exact_objects() {
    for fixture in [
        "stored.zip",
        "independent.tar",
        "independent.tar.gz",
        "independent.tar.bz2",
        "independent.tar.xz",
        "checked.tar.zst",
        "independent.gz",
        "independent.bz2",
        "independent.xz",
        "checked.zst",
    ] {
        run(fixture, false);
    }
    run("stored.zip", true);
    run("independent.tar", true);
}
#[cfg(feature = "isolated-codecs")]
#[test]
fn audit04_original_isolated_codec_prompts_carry_exact_objects() {
    for fixture in [
        "upstream/non-solid.7z",
        "upstream/solid.7z",
        "upstream/rar4.rar",
        "upstream/rar5.rar",
    ] {
        run(fixture, false);
    }
    run("upstream/non-solid.7z", true);
    run("upstream/rar5.rar", true);
}
