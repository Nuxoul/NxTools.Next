#![cfg(windows)]
//! Original finite coordinator -> owned fixture transport -> original native runner/Job/output.
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
        let started = Instant::now();
        while self.0.try_wait().ok().flatten().is_none()
            && started.elapsed() < Duration::from_secs(3)
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
fn original_tar_stream_coordinator_readmits_only_after_actual_cleanup() {
    let parent =
        PathBuf::from(std::env::var_os("NXTOOLS_TAR_STREAM_EVIDENCE_ROOT").expect("owned root"));
    let fixtures = PathBuf::from(
        std::env::var_os("NXTOOLS_TAR_STREAM_FIXTURES").expect("independent fixtures"),
    );
    let apphost =
        PathBuf::from(std::env::var_os("NXTOOLS_TAR_STREAM_HOST").expect("isolated transport"));
    for (mode, fixture) in [
        ("tar_entry", "plain.tar"),
        ("tar_total", "plain.tar"),
        ("tgz_total", "plain.tgz"),
        ("txz_total", "plain.txz"),
        ("xz_total", "known.xz"),
        ("zstd_total", "known.zst"),
        ("second_corrupt", "bad-known.xz"),
        ("second_false_declaration", "false-size.zst"),
        ("cancel", "plain.tar"),
        ("shared_cancel", "plain.tgz"),
        ("source_changed", "plain.tar"),
        ("selected", "plain.tar"),
        ("actual_gzip", "single.gz"),
        ("actual_bzip", "single.bz2"),
        ("actual_zstd", "unknown.zst"),
        ("spool_limit", "plain.tgz"),
        ("committed", "plain.tar"),
    ] {
        if std::env::var("NXTOOLS_TAR_STREAM_CHAIN_CASES")
            .ok()
            .is_some_and(|cases| !cases.split(',').any(|value| value == mode))
        {
            continue;
        }
        eprintln!("TAR stream coordinator scenario: {mode}");
        let root = parent.join(format!("chain-{mode}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).expect("new owned root");
        let source = root.join(fixture);
        std::fs::copy(fixtures.join(fixture), &source).expect("own source copy");
        let output = root.join("output");
        if mode == "selected" {
            std::fs::create_dir(&output).expect("existing explicit target");
        }
        let control = OperationControl::default();
        let port = InteractionPort::new(control.operation_id(), 1);
        let mut request = WorkerRequest::open(
            control.operation_id(),
            source.to_string_lossy().into_owned(),
        );
        request.version = 2;
        request.operation = "extract".into();
        request.codepage = 936;
        request.conflict_policy =
            Some(serde_json::from_value(json!("auto_rename")).expect("policy"));
        request.extract_options =
            Some(nxtools_archive::worker::ExtractOptions { copy_motw: false });
        request.limits.max_entries = 100;
        request.limits.max_metadata_bytes = if mode == "spool_limit" { 2048 } else { 4 << 20 };
        request.limits.max_working_bytes = 64 << 20;
        request.limits.max_entry_bytes = if mode.starts_with("actual_") {
            3072
        } else if mode.ends_with("total") || mode == "committed" || mode == "spool_limit" {
            8192
        } else {
            1
        };
        request.limits.max_output_bytes = if mode.ends_with("total") || mode == "spool_limit" {
            1
        } else {
            1 << 20
        };
        let original_id = request.operation_id;
        let history = Arc::new(Mutex::new(Vec::<Value>::new()));
        let thread_history = history.clone();
        let work_control = control.clone();
        let work_port = port.clone();
        let work_root = root.clone();
        let work_output = output.clone();
        let work_source = source.clone();
        let host_path = apphost.clone();
        let (done_tx, done_rx) = mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            let mut host = Host(
                Command::new(host_path)
                    .arg("--tar-stream-output-readmission-host")
                    .arg(&work_output)
                    .env(
                        "NXTOOLS_TAR_STREAM_HOST_MODE",
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
            if mode == "selected" {
                write_frame(
                    host.0.stdin.as_mut().expect("input"),
                    &json!({"fixture_control":"capture_source","path":work_source}),
                )
                .expect("capture held source");
                let context: Value = read_frame(host.0.stdout.as_mut().expect("output"))
                    .expect("context frame")
                    .expect("context");
                request.job_execution = Some(true);
                request.job_extract = Some(ArchiveJobExtract {
                    strategy: ArchiveJobStrategy::Here,
                    single_file_folder: false,
                    conflict: ArchiveJobConflict::AutoRename,
                    copy_motw: false,
                    selection: Some(ArchiveJobSelection {
                        browser: serde_json::from_value(context["browser"].clone())
                            .expect("original binding"),
                        ordinals: vec![0],
                    }),
                });
            }
            let original = request.clone();
            let mut attempts = 0;
            let result = output_readmission::execute(
                request,
                &work_control,
                &work_port,
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
                        serde_json::to_value(expected).expect("intent")
                            == serde_json::to_value(attempt).expect("intent"),
                        "only ID/readmission/two output limits change"
                    );
                    if attempts == 2 {
                        assert_ne!(attempt.operation_id, original_id);
                        let binding = attempt.output_readmission.as_ref().expect("binding");
                        assert_eq!(binding.logical_id, original_id);
                        assert!(binding.expected_source.is_some());
                    }
                    write_frame(host.0.stdin.as_mut().expect("input"), attempt)
                        .expect("original request");
                    let mut packet: Value = read_frame(host.0.stdout.as_mut().expect("output"))
                        .expect("native frame")
                        .expect("native facts");
                    if packet["observation"] == "cleanup_held" {
                        assert!(
                            work_port.pending().is_none(),
                            "no budget prompt while cleanup held"
                        );
                        assert_eq!(packet["active"], 1);
                        assert_eq!(packet["source_protected"], true);
                        assert_eq!(packet["helper_protected"], true);
                        write_frame(
                            host.0.stdin.as_mut().expect("input"),
                            &json!({"fixture_control":"release_cleanup"}),
                        )
                        .expect("release actual cleanup");
                        packet = read_frame(host.0.stdout.as_mut().expect("output"))
                            .expect("cleanup frame")
                            .expect("cleanup terminal");
                    }
                    assert_eq!(packet["cleanup_complete"], true);
                    assert_eq!(packet["audit"]["active"], 0);
                    assert_eq!(packet["audit"]["source_released"], true);
                    assert_eq!(packet["audit"]["helper_released"], true);
                    assert_eq!(packet["audit"]["codepage"], 936);
                    assert_eq!(packet["audit"]["conflict_policy"], "auto_rename");
                    assert_eq!(packet["audit"]["copy_motw"], false);
                    assert_eq!(
                        packet["audit"]["memory_budget"],
                        original.limits.max_working_bytes
                            + original.limits.max_metadata_bytes
                            + (64 << 20)
                    );
                    if mode == "selected" {
                        assert_eq!(
                            packet["audit"]["job_extract"]["selection_ordinals"],
                            json!([0])
                        );
                    }
                    let typed: AttemptResult =
                        serde_json::from_value(packet.clone()).expect("actual result");
                    if typed.shortfall.is_some() {
                        assert_eq!(typed.files_committed, 0);
                        assert_eq!(packet["audit"]["output_effects"]["native_write_bytes"], 0);
                        assert_eq!(packet["audit"]["output_effects"]["native_publications"], 0);
                    }
                    thread_history.lock().expect("history").push(packet.clone());
                    std::fs::write(
                        work_root.join(format!("attempt-{attempts}.json")),
                        serde_json::to_vec_pretty(&packet).expect("safe facts"),
                    )
                    .expect("record facts");
                    Ok(typed)
                },
            );
            drop(host.0.stdin.take());
            assert!(host.0.wait().expect("host exit").success());
            done_tx
                .send((result, attempts))
                .expect("coordinator result");
        });
        let mut early = None;
        let started = Instant::now();
        let pending = loop {
            if let Some(pending) = port.pending() {
                break Some(pending);
            }
            if let Ok(done) = done_rx.try_recv() {
                early = Some(done);
                break None;
            }
            assert!(started.elapsed() < Duration::from_secs(20));
            std::thread::sleep(Duration::from_millis(2));
        };
        let terminal_only =
            mode.starts_with("actual_") || matches!(mode, "spool_limit" | "committed");
        if terminal_only {
            assert!(
                pending.is_none(),
                "actual/spool/committed failure cannot finance budget"
            );
        } else {
            let pending = pending.expect("declared shortfall approval");
            let Prompt::OutputBudget { current, required } = pending.prompt else {
                panic!("OutputBudget not Risk")
            };
            assert_eq!(history.lock().expect("history").len(), 1);
            assert_eq!(required.entry_bytes, 4096);
            assert_eq!(
                required.output_bytes,
                if mode == "selected"
                    || matches!(
                        mode,
                        "xz_total" | "zstd_total" | "second_corrupt" | "second_false_declaration"
                    )
                {
                    4096
                } else {
                    6144
                }
            );
            if mode == "selected" {
                assert!(std::fs::read_dir(&output).expect("target").next().is_none());
            } else {
                assert!(!output.exists());
            }
            let approved = OutputBudget {
                entry_bytes: current.entry_bytes.max(required.entry_bytes),
                output_bytes: current.output_bytes.max(required.output_bytes),
            };
            let mut stale = pending.ticket;
            stale.operation_id = uuid::Uuid::new_v4();
            assert!(
                !port
                    .respond(stale, PromptResponse::OutputBudget { approved })
                    .expect("stale rejection")
            );
            if mode == "cancel" {
                assert!(
                    port.respond(pending.ticket, PromptResponse::Cancel)
                        .expect("decline")
                );
            } else if mode == "shared_cancel" {
                control.cancel();
            } else {
                if mode == "source_changed" {
                    std::fs::write(&source, b"changed owned input")
                        .expect("change only after cleanup");
                }
                assert!(
                    port.respond(pending.ticket, PromptResponse::OutputBudget { approved })
                        .expect("finite approval")
                );
                assert!(
                    !port
                        .respond(pending.ticket, PromptResponse::OutputBudget { approved })
                        .expect("duplicate rejection")
                );
            }
        }
        let (result, attempts) = early.unwrap_or_else(|| {
            done_rx
                .recv_timeout(Duration::from_secs(25))
                .expect("bounded coordinator finish")
        });
        worker.join().expect("coordinator thread");
        if matches!(mode, "cancel" | "shared_cancel") {
            assert_eq!(attempts, 1);
            assert!(matches!(result, Err(ArchiveError::Cancelled)));
        } else {
            let result = result.expect("native result");
            assert_eq!(attempts, if terminal_only { 1 } else { 2 });
            if mode == "source_changed" {
                assert_eq!(result.error_code.as_deref(), Some("source_changed"));
                assert!(!output.exists());
            } else if mode.starts_with("actual_") || mode == "spool_limit" {
                assert_eq!(result.error_code.as_deref(), Some("ResourceLimit"));
                assert!(result.shortfall.is_none());
                assert_eq!(result.files_committed, 0);
            } else if matches!(mode, "second_corrupt" | "second_false_declaration") {
                assert_eq!(result.error_code.as_deref(), Some("CorruptArchive"));
                assert!(result.shortfall.is_none());
                assert_eq!(result.files_committed, 0);
            } else if mode == "committed" {
                assert_eq!(result.files_committed, 1);
                let facts = history.lock().expect("history");
                assert_eq!(
                    facts[0]["audit"]["category"],
                    "cancelled_with_committed_output"
                );
                assert_eq!(facts[0]["audit"]["commit_state"], "Committed");
                assert!(
                    !facts[0]["audit"]["journal"]
                        .as_array()
                        .expect("journal")
                        .is_empty()
                );
            } else {
                assert!(result.error_code.is_none());
                assert_eq!(
                    result.files_committed,
                    if mode == "selected" || matches!(mode, "xz_total" | "zstd_total") {
                        1
                    } else {
                        2
                    }
                );
                if mode == "xz_total" {
                    assert_eq!(
                        std::fs::read(output.join("known")).expect("payload"),
                        vec![0; 4096]
                    );
                } else if mode == "zstd_total" {
                    assert_eq!(
                        std::fs::read(output.join("known")).expect("payload"),
                        vec![b'B'; 4096]
                    );
                } else {
                    assert_eq!(
                        std::fs::read(output.join("alpha.bin")).expect("payload"),
                        (0..=255u8).cycle().take(4096).collect::<Vec<_>>()
                    );
                    if mode != "selected" {
                        assert_eq!(
                            std::fs::read(output.join("beta.bin")).expect("payload"),
                            vec![b'B'; 2048]
                        );
                    }
                }
            }
        }
        assert!(port.pending().is_none());
        std::fs::write(
            root.join("summary.json"),
            serde_json::to_vec_pretty(
                &json!({"mode":mode,"attempts":attempts,"passed":true,"pending":false}),
            )
            .expect("summary"),
        )
        .expect("record summary");
    }
}
