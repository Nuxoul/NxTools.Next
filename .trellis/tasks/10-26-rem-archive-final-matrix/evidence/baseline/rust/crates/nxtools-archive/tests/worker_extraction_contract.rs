use nxtools_archive::worker::{WorkerRequest, read_frame, write_frame};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    process::{Command, Stdio},
};
use uuid::Uuid;

#[test]
fn actual_worker_waits_for_native_output_acknowledgements_and_never_commits_bad_crc() {
    for (fixture, expected_files, expected_error) in [
        ("deflated.zip", 3, None),
        ("bad-crc.zip", 2, Some("CorruptArchive")),
    ] {
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
        request.operation = "extract".into();
        write_frame(child.stdin.as_mut().expect("stdin"), &request).expect("private request");
        let mut partials = BTreeMap::<Uuid, (String, Vec<u8>)>::new();
        let mut committed = BTreeMap::new();
        let mut aborted = 0;
        let terminal = loop {
            let frame: Value = read_frame(child.stdout.as_mut().expect("stdout"))
                .expect("bounded frame")
                .expect("terminal before EOF");
            assert_eq!(frame["operation_id"], id.to_string());
            if frame["op"] == "native_call" {
                let call = &frame["native"];
                let mut token = None;
                match call["action"].as_str().expect("action") {
                    "directory" => {}
                    "begin" => {
                        let key = Uuid::new_v4();
                        partials.insert(
                            key,
                            (call["path"].as_str().expect("path").into(), Vec::new()),
                        );
                        token = Some(key);
                    }
                    "write" => {
                        let key =
                            Uuid::parse_str(call["token"].as_str().expect("token")).expect("UUID");
                        let data = call["data"].as_array().expect("bounded private bytes");
                        assert!(data.len() <= 65536);
                        partials
                            .get_mut(&key)
                            .expect("owned partial")
                            .1
                            .extend(data.iter().map(|value| value.as_u64().expect("byte") as u8));
                    }
                    "commit" => {
                        let key =
                            Uuid::parse_str(call["token"].as_str().expect("token")).expect("UUID");
                        let (path, data) = partials.remove(&key).expect("partial");
                        committed.insert(path, data);
                    }
                    "abort" => {
                        let key =
                            Uuid::parse_str(call["token"].as_str().expect("token")).expect("UUID");
                        partials.remove(&key);
                        aborted += 1;
                    }
                    _ => panic!("unknown native action"),
                }
                write_frame(child.stdin.as_mut().expect("stdin"),&json!({"version":1,"operation_id":id,"op":"native_result","call_id":call["call_id"],"success":true,"token":token})).expect("native acknowledgement");
            } else if frame["op"] == "complete" || frame["op"] == "failure" {
                break frame;
            }
        };
        assert_eq!(committed.len(), expected_files);
        assert!(partials.is_empty());
        if let Some(error) = expected_error {
            assert_eq!(terminal["error_code"], error);
            assert_eq!(aborted, 1);
        } else {
            assert_eq!(terminal["op"], "complete");
            assert_eq!(committed["资料/说明.txt"], "你好，压缩包\n".as_bytes());
        }
        drop(child.stdin.take());
        let result = child.wait_with_output().expect("wait owned worker");
        assert!(result.status.success());
        assert!(result.stderr.is_empty());
    }
}

// Real worker/private rendezvous; protected Windows handles/Job cleanup are separately native-tested.
fn format_worker(request: WorkerRequest, approve_risk: bool) -> (Value, Vec<String>, Vec<Value>) {
    format_worker_fault(request, approve_risk, None)
}
fn format_worker_fault(
    mut request: WorkerRequest,
    approve_risk: bool,
    failure: Option<(&str, &str)>,
) -> (Value, Vec<String>, Vec<Value>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
        .arg("--archive-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("original worker");
    request.version = 2;
    request.operation = "extract".into();
    let id = request.operation_id;
    write_frame(child.stdin.as_mut().expect("stdin"), &request).expect("start");
    let mut actions = Vec::<String>::new();
    let mut frames = Vec::new();
    let terminal = loop {
        let frame: Value = read_frame(child.stdout.as_mut().expect("stdout"))
            .expect("private frame")
            .expect("terminal before EOF");
        assert_eq!(frame["operation_id"], id.to_string());
        if frame["op"] == "native_call" {
            let call = &frame["native"];
            let action = call["action"].as_str().expect("native action");
            actions.push(action.into());
            let mut reply = json!({"version":1,"operation_id":id,"op":"native_result","call_id":call["call_id"],"success":true});
            match action {
                "source_stamp" => reply["read_data"] = json!("held-native-source"),
                "begin" => reply["token"] = json!(Uuid::new_v4()),
                "source_acquire" | "source_validated" | "codec_opened" | "source_decoding"
                | "source_precommit" | "output_ready" | "directory" | "directory_new" | "write"
                | "commit" | "abort" => {}
                other => panic!("unexpected native action {other}"),
            }
            if let Some((stage, code)) = failure
                && stage == action
            {
                reply["success"] = json!(false);
                reply["error_code"] = json!(code);
            }
            write_frame(child.stdin.as_mut().expect("stdin"), &reply).expect("native reply");
        } else if frame["op"] == "risk_prompt" {
            // All codec confirmations precede any current output admission.
            assert!(
                !actions
                    .iter()
                    .any(|a| matches!(a.as_str(), "begin" | "write" | "commit"))
            );
            let prompt = &frame["risk_prompt"];
            if approve_risk {
                // A larger answer must not satisfy the original admitted-ceiling ticket.
                write_frame(child.stdin.as_mut().expect("stdin"), &json!({"version":1,"operation_id":id,"op":"risk_reply","risk_reply":{"ticket":prompt["ticket"],"approved_limit":request.limits.max_entry_bytes + 1}})).expect("invalid answer");
            }
            write_frame(child.stdin.as_mut().expect("stdin"), &json!({"version":1,"operation_id":id,"op":"risk_reply","risk_reply":{"ticket":prompt["ticket"],"approved_limit":if approve_risk {Some(request.limits.max_entry_bytes)} else {None}}})).expect("risk reply");
        }
        let done = frame["op"] == "complete" || frame["op"] == "failure";
        frames.push(frame.clone());
        if done {
            break frame;
        }
    };
    drop(child.stdin.take());
    let result = child.wait_with_output().expect("actual exited worker");
    assert!(result.status.success() && result.stderr.is_empty());
    (terminal, actions, frames)
}
fn format_request(fixture: &str) -> WorkerRequest {
    WorkerRequest::open(
        Uuid::new_v4(),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(fixture)
            .to_string_lossy()
            .into_owned(),
    )
}
#[test]
fn non_zip_declared_shortfall_has_source_binding_and_zero_output_calls() {
    let mut fixtures = vec![
        "independent.tar",
        "independent.tar.gz",
        "independent.tar.bz2",
        "independent.tar.xz",
        "independent.tar.zst",
    ];
    if cfg!(feature = "isolated-codecs") {
        fixtures.push("upstream/lzma.7z");
    }
    if cfg!(all(feature = "isolated-codecs", windows)) {
        fixtures.extend(["upstream/rar4.rar", "upstream/rar5.rar"]);
    }
    for fixture in fixtures {
        let mut request = format_request(fixture);
        request.job_execution = Some(true);
        request.job_extract = Some(serde_json::from_value(json!({"strategy":"name","single_file_folder":false,"conflict":"ask","copy_motw":false,"selection":null})).expect("original name plan"));
        request.limits.max_entry_bytes = 1;
        request.output_readmission =
            Some(nxtools_archive::output_readmission::ReadmissionOptions {
                logical_id: request.operation_id,
                expected_source: None,
            });
        let (terminal, actions, frames) = format_worker(request, false);
        assert_eq!(terminal["error_code"], "ResourceLimit", "{fixture}");
        assert!(
            !actions.iter().any(|a| matches!(
                a.as_str(),
                "output_ready"
                    | "directory"
                    | "directory_new"
                    | "begin"
                    | "write"
                    | "commit"
                    | "abort"
            )),
            "{fixture}: {actions:?}"
        );
        let shortages: Vec<_> = frames
            .iter()
            .filter(|f| f["op"] == "output_budget_shortfall")
            .collect();
        assert_eq!(shortages.len(), 1, "{fixture}");
        assert!(
            shortages[0]["output_budget_shortfall"]["required"]["entry_bytes"]
                .as_u64()
                .expect("declared")
                > 1
        );
        assert_eq!(
            shortages[0]["output_budget_shortfall"]["source"]["native"],
            "held-native-source"
        );
        assert_eq!(
            shortages[0]["output_budget_shortfall"]["source"]["digest"]
                .as_array()
                .expect("digest")
                .len(),
            32
        );
    }
}
#[test]
fn unknown_stream_actual_overflow_never_emits_declared_shortfall() {
    let mut request = format_request("independent.gz");
    request.limits.max_entry_bytes = 1;
    request.output_readmission = Some(nxtools_archive::output_readmission::ReadmissionOptions {
        logical_id: request.operation_id,
        expected_source: None,
    });
    let (terminal, actions, frames) = format_worker(request, false);
    assert_eq!(terminal["error_code"], "ResourceLimit");
    assert!(actions.iter().any(|a| a == "begin"));
    assert!(actions.iter().any(|a| a == "abort"));
    assert!(!actions.iter().any(|a| a == "commit"));
    assert!(
        !frames
            .iter()
            .any(|f| f["op"] == "output_budget_shortfall" || f["op"] == "risk_prompt")
    );
}
#[test]
#[cfg(all(feature = "isolated-codecs", debug_assertions))]
fn sevenz_known_member_risk_confirms_original_ceiling_but_solid_has_no_member_ratio() {
    for (fixture, risk) in [
        ("interop/x86-bcj-lzma2.7z", true),
        ("upstream/solid.7z", false),
    ] {
        let mut request = format_request(fixture);
        request.zip_risk_fixture = Some(
            serde_json::from_value(json!({"bytes":1,"ratio":1})).expect("lower debug thresholds"),
        );
        let (terminal, actions, frames) = format_worker(request, true);
        assert_eq!(terminal["op"], "complete", "{fixture}");
        if risk {
            assert_eq!(terminal["bytes_verified"], 4096);
        }
        assert_eq!(
            frames.iter().any(|f| f["op"] == "risk_prompt"),
            risk,
            "{fixture}"
        );
        assert_eq!(
            frames.iter().any(|f| f["op"] == "risk_confirmed"),
            risk,
            "{fixture}"
        );
        assert!(actions.iter().any(|a| a == "commit"));
    }
}
#[test]
fn non_zip_invalid_paths_precede_output_shortfall() {
    for (fixture, code) in [
        ("unsafe.tar", "UnsafePath"),
        ("collision.tar", "OutputCollision"),
    ] {
        let mut request = format_request(fixture);
        request.limits.max_entry_bytes = 1;
        request.output_readmission =
            Some(nxtools_archive::output_readmission::ReadmissionOptions {
                logical_id: request.operation_id,
                expected_source: None,
            });
        let (terminal, actions, frames) = format_worker(request, false);
        assert_eq!(terminal["error_code"], code);
        assert_eq!(actions, ["source_acquire"]);
        assert!(!frames.iter().any(|f| f["op"] == "output_budget_shortfall"));
    }
}

#[test]
fn non_zip_readmission_reenters_original_worker_once_after_actual_exit() {
    use nxtools_archive::{
        OperationControl,
        interaction::{InteractionPort, Prompt, PromptResponse},
        output_readmission::{self, AttemptResult, OutputBudget, OutputShortfall},
    };
    let control = OperationControl::default();
    let port = InteractionPort::new(control.operation_id(), 1);
    let worker_control = control.clone();
    let worker_port = port.clone();
    let thread = std::thread::spawn(move || {
        let mut request = format_request("independent.tar.gz");
        request.operation_id = worker_control.operation_id();
        request.version = 2;
        request.operation = "extract".into();
        request.password_interaction = Some(true); // Non-password formats bypass password preparation.
        request.limits.max_entry_bytes = 1;
        request.limits.max_output_bytes = 1;
        let original_work = request.limits.max_working_bytes;
        let mut attempts = 0;
        let result =
            output_readmission::execute(request, &worker_control, &worker_port, |attempt, _| {
                attempts += 1;
                assert!(attempts <= 2);
                assert_eq!(attempt.limits.max_working_bytes, original_work);
                assert_eq!(attempt.password_interaction, Some(true));
                let (terminal, actions, frames) = format_worker(attempt.clone(), false);
                let shortfall = frames
                    .iter()
                    .find(|f| f["op"] == "output_budget_shortfall")
                    .map(|f| {
                        serde_json::from_value::<OutputShortfall>(
                            f["output_budget_shortfall"].clone(),
                        )
                        .expect("original shortfall")
                    });
                if attempts == 1 {
                    assert!(!actions.iter().any(|a| a == "output_ready"));
                    assert!(shortfall.is_some());
                } else {
                    assert!(
                        attempt
                            .output_readmission
                            .as_ref()
                            .expect("readmission")
                            .expected_source
                            .is_some()
                    );
                    assert!(actions.iter().any(|a| a == "commit"));
                }
                Ok(AttemptResult {
                    operation_id: attempt.operation_id,
                    cleanup_complete: true,
                    files_committed: u64::from(terminal["op"] == "complete"),
                    error_code: terminal["error_code"].as_str().map(str::to_owned),
                    shortfall,
                })
            })
            .expect("finite coordinator");
        assert_eq!(attempts, 2);
        assert!(result.error_code.is_none());
    });
    let started = std::time::Instant::now();
    let pending = loop {
        if let Some(pending) = port.pending() {
            break pending;
        }
        assert!(started.elapsed() < std::time::Duration::from_secs(15));
        std::thread::sleep(std::time::Duration::from_millis(2));
    };
    let Prompt::OutputBudget { current, required } = pending.prompt else {
        panic!("output approval");
    };
    assert!(
        port.respond(
            pending.ticket,
            PromptResponse::OutputBudget {
                approved: OutputBudget {
                    entry_bytes: current.entry_bytes.max(required.entry_bytes),
                    output_bytes: current.output_bytes.max(required.output_bytes),
                }
            }
        )
        .expect("finite explicit approval")
    );
    thread.join().expect("coordinator exited");
}

#[test]
fn non_zip_readmission_changed_digest_or_native_identity_has_zero_output() {
    use sha2::{Digest, Sha256};
    for native_changed in [false, true] {
        let mut request = format_request("independent.tar");
        let digest: [u8; 32] =
            Sha256::digest(std::fs::read(&request.path).expect("tiny source")).into();
        request.output_readmission =
            Some(nxtools_archive::output_readmission::ReadmissionOptions {
                logical_id: request.operation_id,
                expected_source: Some(nxtools_archive::output_readmission::SourceBinding {
                    native: if native_changed {
                        "different-held-object"
                    } else {
                        "held-native-source"
                    }
                    .into(),
                    digest: if native_changed { digest } else { [0; 32] },
                }),
            });
        let (terminal, actions, frames) = format_worker(request, false);
        assert_eq!(terminal["error_code"], "SourceChanged");
        assert!(!actions.iter().any(|a| matches!(
            a.as_str(),
            "output_ready" | "directory" | "begin" | "write" | "commit"
        )));
        assert!(!frames.iter().any(|f| f["op"] == "output_budget_shortfall"));
    }
}

#[cfg(all(feature = "isolated-codecs", windows, debug_assertions))]
fn tiny_rar4(declared: u32, solid: bool) -> Vec<u8> {
    // Independent stored RAR records, no RAR compressor. Rejection tests never decode the
    // intentionally over-declared known header. Unknown-size member carries one real byte.
    fn header(kind: u8, flags: u16, body: &[u8]) -> Vec<u8> {
        let mut protected = vec![kind];
        protected.extend(flags.to_le_bytes());
        protected.extend(((7 + body.len()) as u16).to_le_bytes());
        protected.extend(body);
        let mut bytes = (crc32fast::hash(&protected) as u16).to_le_bytes().to_vec();
        bytes.extend(protected);
        bytes
    }
    let mut bytes = b"Rar!\x1a\x07\0".to_vec();
    bytes.extend(header(0x73, if solid { 8 } else { 0 }, &[0; 6]));
    let mut body = Vec::new();
    body.extend(1u32.to_le_bytes());
    body.extend(declared.to_le_bytes());
    body.push(2); // Windows host.
    body.extend(crc32fast::hash(b"x").to_le_bytes());
    body.extend(0u32.to_le_bytes());
    body.extend([20, 0x30]); // Stored method.
    body.extend(5u16.to_le_bytes());
    body.extend(0u32.to_le_bytes());
    body.extend(b"a.txt");
    bytes.extend(header(0x74, 0x8000, &body));
    bytes.push(b'x');
    bytes.extend(header(0x7b, 0, &[]));
    bytes
}
#[test]
#[cfg(all(feature = "isolated-codecs", windows, debug_assertions))]
fn rar_known_risk_rejects_before_output_unknown_size_and_solid_never_fake_a_ratio() {
    let directory = tempfile::tempdir().expect("owned tiny fixture");
    for (name, declared, solid, expected_risk) in [
        ("known", 4096, false, true),
        ("unknown", u32::MAX, false, false),
        ("solid", 4096, true, false),
    ] {
        let path = directory.path().join(format!("{name}.rar"));
        std::fs::write(&path, tiny_rar4(declared, solid)).expect("tiny records");
        let mut request = WorkerRequest::open(Uuid::new_v4(), path.to_string_lossy().into_owned());
        request.zip_risk_fixture = Some(
            serde_json::from_value(json!({"bytes":1,"ratio":1})).expect("lower debug thresholds"),
        );
        request.output_readmission =
            Some(nxtools_archive::output_readmission::ReadmissionOptions {
                logical_id: request.operation_id,
                expected_source: None,
            });
        let (terminal, actions, frames) = format_worker(request, false);
        assert_eq!(
            frames.iter().any(|f| f["op"] == "risk_prompt"),
            expected_risk,
            "{name}"
        );
        assert!(
            !frames.iter().any(|f| f["op"] == "output_budget_shortfall"),
            "{name}"
        );
        if expected_risk {
            assert_eq!(terminal["error_code"], "Cancelled");
            assert!(
                !actions
                    .iter()
                    .any(|a| matches!(a.as_str(), "begin" | "write" | "commit"))
            );
        } else {
            assert_ne!(terminal["error_code"], "UnsupportedOperation");
            assert!(
                actions.iter().any(|a| a == "begin"),
                "{name}: must pass metadata preflight"
            );
        }
    }
}

#[test]
fn non_zip_preflight_preserves_native_source_failure_without_shortfall() {
    for code in ["source_changed", "source_unavailable"] {
        let mut request = format_request("independent.tar");
        request.limits.max_entry_bytes = 1;
        request.output_readmission =
            Some(nxtools_archive::output_readmission::ReadmissionOptions {
                logical_id: request.operation_id,
                expected_source: None,
            });
        let (terminal, actions, frames) =
            format_worker_fault(request, false, Some(("source_precommit", code)));
        assert_eq!(
            terminal["error_code"],
            if code == "source_changed" {
                "SourceChanged"
            } else {
                "IoFailure"
            }
        );
        assert!(!actions.iter().any(|a| matches!(
            a.as_str(),
            "output_ready" | "directory" | "begin" | "commit"
        )));
        assert!(!frames.iter().any(|f| f["op"] == "output_budget_shortfall"));
    }
}
