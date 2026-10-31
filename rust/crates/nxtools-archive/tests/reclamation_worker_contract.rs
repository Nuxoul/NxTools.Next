//! Actual worker protocol tests; native allocation/durability is covered by Windows fixtures.
use nxtools_archive::{
    OperationControl,
    interaction::InteractionPort,
    worker::{ExtractOptions, WorkerRequest, read_frame, write_frame},
};
use nxtools_protocol::archive_job::*;
use serde_json::{Value, json};
use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};
use uuid::Uuid;

struct Fixture {
    root: PathBuf,
    source: PathBuf,
}
impl Fixture {
    fn new(method: zip::CompressionMethod) -> Self {
        let root = PathBuf::from(std::env::var_os("PI_SCRATCH_DIR").expect("session scratch"))
            .join(format!("hz5-{}", Uuid::new_v4()));
        std::fs::create_dir(&root).expect("create owned worker fixture");
        let source = root.join("owned.zip");
        let mut writer = zip::ZipWriter::new(File::create(&source).expect("create source ZIP"));
        writer
            .start_file(
                "payload.bin",
                zip::write::SimpleFileOptions::default().compression_method(method),
            )
            .expect("start owned stored file");
        writer
            .write_all(&vec![0x3d; 65536])
            .expect("write owned payload");
        writer.finish().expect("finish owned ZIP");
        Self { root, source }
    }
    fn request(&self) -> WorkerRequest {
        let mut request =
            WorkerRequest::open(Uuid::new_v4(), self.source.to_string_lossy().into_owned());
        request.version = 2;
        request.operation = "extract".into();
        request.job_execution = Some(true);
        let intent = ArchiveReclamationIntent {
            mode: ArchiveReclamationMode::Start,
            recovery_id: Uuid::new_v4(),
        };
        request.extract_options = Some(ExtractOptions {
            copy_motw: true,
            reclamation: Some(intent),
        });
        request.job_extract = Some(ArchiveJobExtract {
            strategy: ArchiveJobStrategy::Here,
            single_file_folder: false,
            conflict: ArchiveJobConflict::Ask,
            copy_motw: true,
            selection: None,
            reclamation: Some(intent),
        });
        request
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).expect("remove owned worker fixture");
    }
}

fn execute(request: &WorkerRequest, fail_at: Option<&str>) -> (Vec<String>, Value, Vec<u8>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
        .arg("--archive-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn owned worker");
    write_frame(child.stdin.as_mut().expect("worker stdin"), request).expect("send request");
    let mut calls = Vec::new();
    let mut written = Vec::new();
    let token = Uuid::new_v4();
    let terminal = loop {
        let frame: Value = read_frame(child.stdout.as_mut().expect("worker stdout"))
            .expect("read worker frame")
            .expect("terminal before EOF");
        assert_eq!(frame["operation_id"], request.operation_id.to_string());
        match frame["op"].as_str().expect("worker operation") {
            "accepted" => {
                assert!(
                    frame["capabilities"]
                        .as_array()
                        .expect("worker negotiated capabilities")
                        .contains(&json!("stored_zip_reclamation_v1"))
                );
            }
            "progress" => {}
            "complete" | "failure" => break frame,
            "native_call" => {
                let native = &frame["native"];
                let action = native["action"].as_str().expect("native action");
                calls.push(action.to_owned());
                let mut reply = json!({"version":1,"operation_id":request.operation_id,"op":"native_result",
                    "call_id":native["call_id"],"success":true});
                if fail_at == Some(action) {
                    reply["success"] = json!(false);
                    reply["error_code"] = json!("reclamation_journal_flush");
                } else {
                    match action {
                        "source_acquire" | "hz5_plan_entry" | "hz5_seal" | "hz5_select"
                        | "file_metadata" | "commit" | "hz5_pending" | "hz5_complete" => {}
                        "hz5_info" => {
                            reply["read_data"]=json!(json!({"schema":1,
                        "recovery_id":request.extract_options.expect("extract options").reclamation.expect("explicit reclamation").recovery_id,
                        "cluster_bytes":4096,"journal_bytes":0,"source_identity":"source-object","root_identity":"root-object"}).to_string());
                        }
                        "begin" => reply["token"] = json!(token),
                        "write" => written.extend(
                            native["data"]
                                .as_array()
                                .expect("native data bytes")
                                .iter()
                                .map(|value| value.as_u64().expect("native byte") as u8),
                        ),
                        "hz5_apply" => {
                            reply["read_data"] = json!(json!({"released_bytes":61440}).to_string())
                        }
                        other => panic!("unexpected native action {other}"),
                    }
                }
                write_frame(child.stdin.as_mut().expect("worker stdin"), &reply)
                    .expect("send native reply");
            }
            other => panic!("unexpected worker frame {other}"),
        }
    };
    drop(child.stdin.take());
    let result = child.wait_with_output().expect("wait for owned worker");
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    (calls, terminal, written)
}

#[test]
fn reclamation_worker_orders_publication_pending_and_apply_on_original_rendezvous() {
    let fixture = Fixture::new(zip::CompressionMethod::Stored);
    let (calls, terminal, written) = execute(&fixture.request(), None);
    assert_eq!(terminal["op"], "complete");
    assert_eq!(terminal["files_verified"], 1);
    assert_eq!(written, vec![0x3d; 65536]);
    let index = |action: &str| {
        calls
            .iter()
            .position(|call| call == action)
            .expect("required native action")
    };
    assert!(index("hz5_seal") < index("begin"));
    assert!(index("commit") < index("hz5_pending"));
    assert!(index("hz5_pending") < index("hz5_apply"));
    assert!(index("hz5_apply") < index("hz5_complete"));
    assert!(
        !calls
            .iter()
            .any(|call| call == "source_stamp" || call == "observe_target" || call == "abort")
    );
}
#[test]
fn reclamation_journal_failure_cannot_approve_source_release_or_generic_retry() {
    let fixture = Fixture::new(zip::CompressionMethod::Stored);
    let (calls, terminal, _) = execute(&fixture.request(), Some("hz5_pending"));
    assert_eq!(terminal["op"], "failure");
    assert!(!calls.iter().any(|call| call == "hz5_apply"));
    assert!(!calls.iter().any(|call| call == "abort"));
}
#[test]
fn reclamation_unsupported_method_is_refused_before_journal_or_output() {
    let fixture = Fixture::new(zip::CompressionMethod::Deflated);
    let (calls, terminal, _) = execute(&fixture.request(), None);
    assert_eq!(terminal["op"], "failure");
    assert_eq!(calls, vec!["source_acquire"]);
}
#[test]
fn reclamation_intent_cannot_be_wrapped_by_finite_output_readmission() {
    let fixture = Fixture::new(zip::CompressionMethod::Stored);
    let private = fixture.request();
    let id = private.operation_id;
    let control = OperationControl::with_operation_id(id);
    let interaction = InteractionPort::new(id, 1);
    let mut runs = 0;
    let command = ArchiveJobCommand {
        version: 1,
        operation_id: id,
        session_id: Uuid::new_v4(),
        generation: 1,
        action: ArchiveJobAction::Start,
        reply: None,
        request: Some(ArchiveJobRequest {
            kind: ArchiveJobKind::Extract,
            paths: vec!["C:\\owned\\a.zip".into()],
            destination: Some("C:\\owned\\output".into()),
            codepage: 0,
            limits: Default::default(),
            extract: private.job_extract.clone(),
            create: None,
            modify: None,
            readmission: None,
        }),
    };
    let result = nxtools_archive::product_job::execute_with_output_readmission(
        command,
        &control,
        &interaction,
        |_, _| {
            runs += 1;
            panic!("destructive operation must never reach ordinary attempt runner")
        },
    );
    assert!(result.is_err());
    assert_eq!(runs, 0);
}
