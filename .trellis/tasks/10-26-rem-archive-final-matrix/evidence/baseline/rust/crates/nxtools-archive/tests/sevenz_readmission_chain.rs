#![cfg(all(windows, feature = "isolated-codecs"))]
//! Original Rust coordinator -> fixture transport -> original C# runner/Job/native output.
use nxtools_archive::{
    ArchiveError, OperationControl,
    interaction::{InteractionPort, Prompt, PromptResponse},
    output_readmission::{self, AttemptResult, OutputBudget},
    worker::{WorkerRequest, read_frame, write_frame},
};
use nxtools_protocol::archive_job::*;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

struct Host(Child);
impl Drop for Host {
    fn drop(&mut self) {
        drop(self.0.stdin.take());
        let start = Instant::now();
        while self.0.try_wait().ok().flatten().is_none() && start.elapsed() < Duration::from_secs(3)
        {
            std::thread::sleep(Duration::from_millis(10));
        }
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

#[test]
fn original_sevenz_coordinator_waits_for_cleanup_and_readmits_once_with_same_intent() {
    let parent = PathBuf::from(
        std::env::var_os("NXTOOLS_SEVENZ_EVIDENCE_ROOT").expect("explicit evidence root"),
    );
    let fixtures = PathBuf::from(
        std::env::var_os("NXTOOLS_SEVENZ_FIXTURES").expect("independent tiny7z fixtures"),
    );
    let apphost = PathBuf::from(std::env::var_os("NXTOOLS_SEVENZ_HOST").expect("isolated host"));
    for mode in [
        "entry",
        "total",
        "solid_total",
        "cancel",
        "shared_cancel",
        "source_changed",
        "selected_success",
        "actual",
        "second_actual",
        "committed",
    ] {
        if std::env::var("NXTOOLS_SEVENZ_CHAIN_CASES")
            .ok()
            .is_some_and(|cases| !cases.split(',').any(|value| value == mode))
        {
            continue;
        }
        eprintln!("7z native coordinator scenario: {mode}");
        let root = parent.join(format!("chain-{mode}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).expect("owned new root");
        let source = root.join("tiny.7z");
        let selection = matches!(mode, "selected_success" | "actual" | "second_actual");
        let solid = matches!(mode, "solid_total" | "actual" | "second_actual");
        std::fs::copy(
            fixtures.join(if solid { "solid.7z" } else { "nonsolid.7z" }),
            &source,
        )
        .expect("own independent input");
        let destination = root.join("output");
        if selection {
            std::fs::create_dir(&destination).expect("existing explicit empty selection target");
        }
        let control = OperationControl::default();
        let interaction = InteractionPort::new(control.operation_id(), 1);
        let mut request = WorkerRequest::open(
            control.operation_id(),
            source.to_string_lossy().into_owned(),
        );
        request.version = 2;
        request.operation = "extract".into();
        request.codepage = 936;
        request.password = Some("fixture-unused-password".into());
        request.conflict_policy =
            Some(serde_json::from_value(json!("auto_rename")).expect("policy"));
        request.extract_options =
            Some(nxtools_archive::worker::ExtractOptions { copy_motw: false });
        request.limits.max_entries = 100;
        request.limits.max_metadata_bytes = 4 << 20;
        request.limits.max_working_bytes = 64 << 20;
        request.limits.max_entry_bytes =
            if matches!(mode, "total" | "solid_total" | "committed" | "actual") {
                8192
            } else {
                1024
            };
        request.limits.max_output_bytes = if matches!(mode, "total" | "solid_total") {
            1
        } else if matches!(mode, "actual" | "second_actual") {
            4096
        } else {
            16384
        };
        let original_id = request.operation_id;
        let history = Arc::new(Mutex::new(Vec::<Value>::new()));
        let thread_history = history.clone();
        let work_control = control.clone();
        let work_interaction = interaction.clone();
        let output = destination.clone();
        let source_for_worker = source.clone();
        let root_for_worker = root.clone();
        let apphost = apphost.clone();
        let (done_tx, done_rx) = mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            let mut host = Host(
                Command::new(apphost)
                    .arg("--sevenz-output-readmission-host")
                    .arg(&output)
                    .env(
                        "NXTOOLS_SEVENZ_HOST_MODE",
                        if mode == "committed" {
                            "cancel_after_commit"
                        } else {
                            "cleanup_barrier"
                        },
                    )
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("original native transport"),
            );
            if selection {
                write_frame(
                    host.0.stdin.as_mut().expect("host input"),
                    &json!({"fixture_control":"capture_source","path":source_for_worker}),
                )
                .expect("capture held source");
                let context: Value = read_frame(host.0.stdout.as_mut().expect("host output"))
                    .expect("source frame")
                    .expect("source context");
                request.job_execution = Some(true);
                request.job_extract = Some(ArchiveJobExtract {
                    strategy: ArchiveJobStrategy::Here,
                    single_file_folder: false,
                    conflict: ArchiveJobConflict::AutoRename,
                    copy_motw: false,
                    selection: Some(ArchiveJobSelection {
                        browser: serde_json::from_value(context["browser"].clone())
                            .expect("original source context"),
                        ordinals: vec![0],
                    }),
                });
            }
            let original = request.clone();
            let mut attempts = 0;
            let result = output_readmission::execute(
                request,
                &work_control,
                &work_interaction,
                |attempt, attempt_control| {
                    attempts += 1;
                    assert!(attempts <= 2);
                    assert_eq!(attempt.operation_id, attempt_control.operation_id());
                    let mut expected = original.clone();
                    expected.operation_id = attempt.operation_id;
                    expected.limits.max_entry_bytes = attempt.limits.max_entry_bytes;
                    expected.limits.max_output_bytes = attempt.limits.max_output_bytes;
                    expected.output_readmission = attempt.output_readmission.clone();
                    assert!(
                        serde_json::to_value(&expected).expect("intent")
                            == serde_json::to_value(attempt).expect("attempt"),
                        "only ID/readmission binding/two output budgets may change; do not print secret-bearing requests"
                    );
                    if attempts == 2 {
                        assert_ne!(attempt.operation_id, original_id);
                        let binding = attempt
                            .output_readmission
                            .as_ref()
                            .expect("same logical owner");
                        assert_eq!(binding.logical_id, original_id);
                        assert!(binding.expected_source.is_some());
                    }
                    write_frame(host.0.stdin.as_mut().expect("host input"), attempt)
                        .expect("same request");
                    let mut packet: Value =
                        read_frame(host.0.stdout.as_mut().expect("host output"))
                            .expect("framed host")
                            .expect("owner result");
                    if packet["observation"] == "cleanup_held" {
                        assert_eq!(packet["operation_id"], attempt.operation_id.to_string());
                        assert!(
                            work_interaction.pending().is_none(),
                            "no budget approval before actual cleanup"
                        );
                        assert_eq!(packet["active"], 1);
                        assert_eq!(packet["source_protected"], true);
                        assert_eq!(packet["helper_protected"], true);
                        write_frame(
                            host.0.stdin.as_mut().expect("host input"),
                            &json!({"fixture_control":"release_cleanup"}),
                        )
                        .expect("release original cleanup checkpoint");
                        packet = read_frame(host.0.stdout.as_mut().expect("host output"))
                            .expect("cleaned frame")
                            .expect("terminal");
                    }
                    assert_eq!(packet["cleanup_complete"], true);
                    assert_eq!(packet["audit"]["active"], 0);
                    assert_eq!(packet["audit"]["source_released"], true);
                    assert_eq!(packet["audit"]["helper_released"], true);
                    assert_eq!(packet["audit"]["codepage"], 936);
                    assert_eq!(packet["audit"]["has_password"], true);
                    assert_eq!(packet["audit"]["conflict_policy"], "auto_rename");
                    assert_eq!(packet["audit"]["copy_motw"], false);
                    assert_eq!(
                        packet["audit"]["memory_budget"],
                        original.limits.max_working_bytes
                            + original.limits.max_metadata_bytes
                            + (64 << 20)
                    );
                    if selection {
                        assert_eq!(
                            packet["audit"]["job_extract"]["selection_ordinals"],
                            json!([0])
                        );
                    }
                    let typed: AttemptResult =
                        serde_json::from_value(packet.clone()).expect("actual cleanup terminal");
                    if typed.shortfall.is_some() {
                        assert_eq!(typed.files_committed, 0);
                        assert_eq!(packet["audit"]["output_effects"]["native_write_bytes"], 0);
                        assert_eq!(packet["audit"]["output_effects"]["native_publications"], 0);
                        assert_eq!(packet["audit"]["output_effects"]["partials_remaining"], 0);
                    }
                    thread_history.lock().expect("history").push(packet.clone());
                    std::fs::write(
                        root_for_worker.join(format!("attempt-{attempts}.json")),
                        serde_json::to_vec_pretty(&packet).expect("evidence"),
                    )
                    .expect("record owner facts");
                    Ok(typed)
                },
            );
            drop(host.0.stdin.take());
            assert!(host.0.wait().expect("clean host exit").success());
            done_tx.send((result, attempts)).expect("final coordinator");
        });
        let mut early = None;
        let wait_started = Instant::now();
        let pending = loop {
            if let Some(pending) = interaction.pending() {
                break Some(pending);
            }
            if let Ok(done) = done_rx.try_recv() {
                early = Some(done);
                break None;
            }
            assert!(
                wait_started.elapsed() < Duration::from_secs(15),
                "bounded output approval"
            );
            std::thread::sleep(Duration::from_millis(2));
        };
        if matches!(mode, "committed" | "actual") {
            assert!(
                pending.is_none(),
                "published/actual byte failure cannot request readmission"
            );
        } else {
            let pending = pending.expect("declared shortage creates one finite approval");
            let Prompt::OutputBudget { current, required } = pending.prompt else {
                panic!("OutputBudget not Risk")
            };
            assert_eq!(
                history.lock().expect("history").len(),
                1,
                "actual cleanup result precedes prompt"
            );
            assert_eq!(required.entry_bytes, 4096);
            assert_eq!(required.output_bytes, if selection { 4096 } else { 6144 });
            if selection {
                assert!(
                    std::fs::read_dir(&destination)
                        .expect("empty target")
                        .next()
                        .is_none()
                );
            } else {
                assert!(!destination.exists());
            }
            let approved = OutputBudget {
                entry_bytes: current.entry_bytes.max(required.entry_bytes),
                output_bytes: current.output_bytes.max(required.output_bytes),
            };
            let mut stale = pending.ticket;
            stale.operation_id = uuid::Uuid::new_v4();
            assert!(
                !interaction
                    .respond(stale, PromptResponse::OutputBudget { approved })
                    .expect("crossoperation denied")
            );
            if mode == "cancel" {
                assert!(
                    interaction
                        .respond(pending.ticket, PromptResponse::Cancel)
                        .expect("decline")
                );
            } else if mode == "shared_cancel" {
                control.cancel();
            } else {
                if mode == "source_changed" {
                    std::fs::write(&source, b"changed owned source").expect("change after release");
                }
                assert!(
                    interaction
                        .respond(pending.ticket, PromptResponse::OutputBudget { approved })
                        .expect("explicit finite approval")
                );
                assert!(
                    !interaction
                        .respond(pending.ticket, PromptResponse::OutputBudget { approved })
                        .expect("duplicate denied")
                );
            }
        }
        let (result, attempts) = early.unwrap_or_else(|| {
            done_rx
                .recv_timeout(Duration::from_secs(20))
                .expect("bounded coordinator finishes")
        });
        worker.join().expect("original coordinator thread");
        if matches!(mode, "cancel" | "shared_cancel") {
            assert_eq!(attempts, 1);
            assert!(matches!(result, Err(ArchiveError::Cancelled)));
        } else {
            let result = result.expect("native terminal");
            assert_eq!(
                attempts,
                if matches!(mode, "committed" | "actual") {
                    1
                } else {
                    2
                }
            );
            if mode == "committed" {
                assert_eq!(result.files_committed, 1);
                let facts = history.lock().expect("history");
                assert_eq!(
                    facts[0]["audit"]["category"],
                    "cancelled_with_committed_output"
                );
                assert_eq!(facts[0]["audit"]["bytes_committed"], 2048);
                assert_eq!(facts[0]["audit"]["commit_state"], "Committed");
                assert!(
                    !facts[0]["audit"]["journal"]
                        .as_array()
                        .expect("native journal")
                        .is_empty()
                );
            } else if mode == "source_changed" {
                assert_eq!(result.error_code.as_deref(), Some("source_changed"));
                assert!(!destination.exists());
            } else if matches!(mode, "actual" | "second_actual") {
                assert_eq!(result.error_code.as_deref(), Some("ResourceLimit"));
                assert_eq!(result.files_committed, 0);
                assert!(
                    result.shortfall.is_none(),
                    "actual overflow has no declared shortage/third attempt"
                );
                assert!(
                    std::fs::read_dir(&destination)
                        .expect("empty after partial cleanup")
                        .next()
                        .is_none()
                );
            } else {
                assert!(result.error_code.is_none());
                assert_eq!(result.files_committed, if selection { 1 } else { 2 });
                assert_eq!(
                    std::fs::read(destination.join("first.bin")).expect("literal first"),
                    (0..=255u8).cycle().take(4096).collect::<Vec<_>>()
                );
                if !selection {
                    assert_eq!(
                        std::fs::read(destination.join("second.bin")).expect("literal second"),
                        vec![b'B'; 2048]
                    );
                }
            }
        }
        assert!(interaction.pending().is_none());
        std::fs::write(
            root.join("summary.json"),
            serde_json::to_vec_pretty(
                &json!({"mode":mode,"attempts":attempts,"pending":false,"passed":true}),
            )
            .expect("summary"),
        )
        .expect("retain scenario summary");
    }
}
