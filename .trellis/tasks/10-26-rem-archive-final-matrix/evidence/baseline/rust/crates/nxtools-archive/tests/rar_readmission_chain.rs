#![cfg(all(windows, feature = "isolated-codecs"))]
//! Original policy coordinator -> fixture transport -> original native runner/Job/output.
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
fn original_rar_coordinator_requires_cleanup_and_preserves_one_new_admission() {
    let parent =
        PathBuf::from(std::env::var_os("NXTOOLS_RAR_EVIDENCE_ROOT").expect("owned evidence"));
    let fixtures =
        PathBuf::from(std::env::var_os("NXTOOLS_RAR_FIXTURES").expect("attributed fixtures"));
    let apphost = PathBuf::from(
        std::env::var_os("NXTOOLS_RAR_HOST").expect("isolated original runner transport"),
    );
    for (mode, fixture, member, payload) in [
        (
            "rar4_entry",
            "rar4.rar",
            "VERSION",
            b"unrar-0.4.0".as_slice(),
        ),
        (
            "rar4_total",
            "rar4.rar",
            "VERSION",
            b"unrar-0.4.0".as_slice(),
        ),
        (
            "rar5_entry",
            "rar5.rar",
            "unicodefilename❤️.txt",
            b"foobar\n".as_slice(),
        ),
        (
            "rar5_total",
            "rar5.rar",
            "unicodefilename❤️.txt",
            b"foobar\n".as_slice(),
        ),
        ("cancel", "rar4.rar", "VERSION", b"unrar-0.4.0".as_slice()),
        (
            "shared_cancel",
            "rar5.rar",
            "unicodefilename❤️.txt",
            b"foobar\n".as_slice(),
        ),
        (
            "source_changed",
            "rar4.rar",
            "VERSION",
            b"unrar-0.4.0".as_slice(),
        ),
        (
            "selected",
            "rar5.rar",
            "unicodefilename❤️.txt",
            b"foobar\n".as_slice(),
        ),
        (
            "encrypted_total",
            "rar5-header-encrypted.rar",
            ".gitignore",
            b"target\nCargo.lock\n".as_slice(),
        ),
        (
            "actual",
            "rar4-unknown-size.rar",
            "VERSION",
            b"unrar-0.4.0".as_slice(),
        ),
        (
            "solid_total",
            "test_read_format_rar5_multiple_files_solid.rar",
            "test1.bin",
            b"".as_slice(),
        ),
        (
            "second_actual",
            "rar5-solid-first-unknown.rar",
            "test2.bin",
            b"".as_slice(),
        ),
        (
            "committed",
            "rar4.rar",
            "VERSION",
            b"unrar-0.4.0".as_slice(),
        ),
    ] {
        if std::env::var("NXTOOLS_RAR_CHAIN_CASES")
            .ok()
            .is_some_and(|cases| !cases.split(',').any(|value| value == mode))
        {
            continue;
        }
        eprintln!("RAR native coordinator scenario: {mode}");
        let root = parent.join(format!("chain-{mode}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).expect("new owned root");
        let source = root.join("tiny.rar");
        std::fs::copy(fixtures.join(fixture), &source).expect("own source copy");
        let output = root.join("output");
        if matches!(mode, "selected" | "second_actual") {
            std::fs::create_dir(&output).expect("existing explicit empty selection target");
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
        request.password = Some(
            if mode == "encrypted_total" {
                "password"
            } else {
                "fixture-unused-password"
            }
            .into(),
        );
        request.conflict_policy =
            Some(serde_json::from_value(json!("auto_rename")).expect("policy"));
        request.extract_options =
            Some(nxtools_archive::worker::ExtractOptions { copy_motw: false });
        request.limits.max_entries = 100;
        request.limits.max_metadata_bytes = 4 << 20;
        request.limits.max_working_bytes = 64 << 20;
        request.limits.max_entry_bytes = if mode == "actual" {
            10
        } else if mode.ends_with("total") || mode == "committed" {
            1024
        } else {
            1
        };
        request.limits.max_output_bytes = if mode.ends_with("total") {
            1
        } else if mode == "second_actual" {
            4096
        } else {
            1 << 20
        };
        let original_id = request.operation_id;
        let history = Arc::new(Mutex::new(Vec::<Value>::new()));
        let thread_history = history.clone();
        let worker_control = control.clone();
        let worker_port = interaction.clone();
        let worker_output = output.clone();
        let worker_source = source.clone();
        let worker_root = root.clone();
        let host_path = apphost.clone();
        let (done_tx, done_rx) = mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            let mut host = Host(
                Command::new(host_path)
                    .arg("--rar-output-readmission-host")
                    .arg(&worker_output)
                    .env(
                        "NXTOOLS_RAR_HOST_MODE",
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
                    .expect("owned native host"),
            );
            if matches!(mode, "selected" | "second_actual") {
                write_frame(
                    host.0.stdin.as_mut().expect("input"),
                    &json!({"fixture_control":"capture_source","path":worker_source}),
                )
                .expect("held binding");
                let context: Value = read_frame(host.0.stdout.as_mut().expect("output"))
                    .expect("binding frame")
                    .expect("binding");
                request.job_execution = Some(true);
                request.job_extract = Some(ArchiveJobExtract {
                    strategy: ArchiveJobStrategy::Here,
                    single_file_folder: false,
                    conflict: ArchiveJobConflict::AutoRename,
                    copy_motw: false,
                    selection: Some(ArchiveJobSelection {
                        browser: serde_json::from_value(context["browser"].clone())
                            .expect("original source context"),
                        ordinals: vec![if mode == "second_actual" { 1 } else { 0 }],
                    }),
                });
            }
            let original = request.clone();
            let mut attempts = 0;
            let result = output_readmission::execute(
                request,
                &worker_control,
                &worker_port,
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
                        "only IDs/readmission/two output limits change; never print secret requests"
                    );
                    if attempts == 2 {
                        assert_ne!(attempt.operation_id, original_id);
                        let options = attempt
                            .output_readmission
                            .as_ref()
                            .expect("original binding");
                        assert_eq!(options.logical_id, original_id);
                        assert!(options.expected_source.is_some());
                    }
                    write_frame(host.0.stdin.as_mut().expect("input"), attempt)
                        .expect("same original request");
                    let mut packet: Value = read_frame(host.0.stdout.as_mut().expect("output"))
                        .expect("native result")
                        .expect("native terminal");
                    if packet["observation"] == "cleanup_held" {
                        assert!(
                            worker_port.pending().is_none(),
                            "actual cleanup before approval"
                        );
                        assert_eq!(packet["active"], 1);
                        assert_eq!(packet["source_protected"], true);
                        assert_eq!(packet["helper_protected"], true);
                        write_frame(
                            host.0.stdin.as_mut().expect("input"),
                            &json!({"fixture_control":"release_cleanup"}),
                        )
                        .expect("release cleanup");
                        packet = read_frame(host.0.stdout.as_mut().expect("output"))
                            .expect("cleanup frame")
                            .expect("actual cleanup terminal");
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
                    if matches!(mode, "selected" | "second_actual") {
                        assert_eq!(
                            packet["audit"]["job_extract"]["selection_ordinals"],
                            json!([if mode == "second_actual" { 1 } else { 0 }])
                        );
                    }
                    let typed: AttemptResult =
                        serde_json::from_value(packet.clone()).expect("original result");
                    if typed.shortfall.is_some() {
                        assert_eq!(typed.files_committed, 0);
                        assert_eq!(packet["audit"]["output_effects"]["native_write_bytes"], 0);
                        assert_eq!(packet["audit"]["output_effects"]["native_publications"], 0);
                    }
                    thread_history.lock().expect("history").push(packet.clone());
                    std::fs::write(
                        worker_root.join(format!("attempt-{attempts}.json")),
                        serde_json::to_vec_pretty(&packet).expect("safe facts"),
                    )
                    .expect("retain evidence");
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
            if let Some(pending) = interaction.pending() {
                break Some(pending);
            }
            if let Ok(done) = done_rx.try_recv() {
                early = Some(done);
                break None;
            }
            assert!(
                started.elapsed() < Duration::from_secs(20),
                "bounded original approval"
            );
            std::thread::sleep(Duration::from_millis(2));
        };
        if matches!(mode, "actual" | "committed") {
            assert!(
                pending.is_none(),
                "actual failure/commit cannot request budget"
            );
        } else {
            let pending = pending.expect("original declaration approval");
            let Prompt::OutputBudget { current, required } = pending.prompt else {
                panic!("OutputBudget not Risk")
            };
            assert_eq!(history.lock().expect("history").len(), 1);
            let required_size = if matches!(mode, "second_actual" | "solid_total") {
                4096
            } else {
                payload.len() as u64
            };
            assert_eq!(required.entry_bytes, required_size);
            assert_eq!(
                required.output_bytes,
                if mode == "solid_total" {
                    16384
                } else {
                    required_size
                }
            );
            if matches!(mode, "selected" | "second_actual") {
                assert!(
                    std::fs::read_dir(&output)
                        .expect("existing empty target")
                        .next()
                        .is_none()
                );
            } else {
                assert!(!output.exists(), "shortage before destination effects");
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
                    .expect("crossoperation rejection")
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
                    std::fs::write(&source, b"changed owned source")
                        .expect("change after actual release");
                }
                assert!(
                    interaction
                        .respond(pending.ticket, PromptResponse::OutputBudget { approved })
                        .expect("explicit finite approval")
                );
                assert!(
                    !interaction
                        .respond(pending.ticket, PromptResponse::OutputBudget { approved })
                        .expect("duplicate rejection")
                );
            }
        }
        let (result, attempts) = early.unwrap_or_else(|| {
            done_rx
                .recv_timeout(Duration::from_secs(25))
                .expect("bounded native finish")
        });
        worker.join().expect("original coordinator thread");
        if matches!(mode, "cancel" | "shared_cancel") {
            assert_eq!(attempts, 1);
            assert!(matches!(result, Err(ArchiveError::Cancelled)));
        } else {
            let result = result.expect("native terminal");
            assert_eq!(
                attempts,
                if matches!(mode, "actual" | "committed") {
                    1
                } else {
                    2
                }
            );
            if mode == "source_changed" {
                assert_eq!(result.error_code.as_deref(), Some("source_changed"));
                assert!(!output.exists());
            } else if matches!(mode, "actual" | "second_actual") {
                assert_eq!(result.error_code.as_deref(), Some("ResourceLimit"));
                assert!(result.shortfall.is_none());
                assert_eq!(result.files_committed, 0);
            } else if mode == "solid_total" {
                assert!(result.error_code.is_none());
                assert_eq!(result.files_committed, 4);
                let expected = [
                    "7d89f86f9f69d744ffff3fc043e15bf89fc3ffc134ffcbb31d164a99bb8b67b0",
                    "f81e6fceeeab366306b23466bf6bb3aac2875e0906dc20a8652be0696ceb15a2",
                    "5e621f2b6ce8fed758c3df8221f994eda55d1e432c7cc4349c34a30ec2e1c43d",
                    "2627f40180217252956edb9a426e8d3e344adaf89019d3bccbe04f6c3416dcdd",
                ];
                for (index, hash) in expected.into_iter().enumerate() {
                    use sha2::{Digest, Sha256};
                    let bytes = std::fs::read(output.join(format!("test{}.bin", index + 1)))
                        .expect("independent solid payload");
                    assert_eq!(bytes.len(), 4096);
                    assert_eq!(format!("{:x}", Sha256::digest(bytes)), hash);
                }
            } else {
                assert_eq!(result.files_committed, 1);
                assert_eq!(
                    std::fs::read(output.join(member)).expect("native output bytes"),
                    payload
                );
                if mode == "committed" {
                    let facts = history.lock().expect("history");
                    assert_eq!(
                        facts[0]["audit"]["category"],
                        "cancelled_with_committed_output"
                    );
                    assert_eq!(facts[0]["audit"]["commit_state"], "Committed");
                    assert_eq!(facts[0]["audit"]["bytes_committed"], payload.len() as u64);
                    assert!(
                        !facts[0]["audit"]["journal"]
                            .as_array()
                            .expect("journal")
                            .is_empty()
                    );
                } else {
                    assert!(result.error_code.is_none());
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
        .expect("retain summary");
    }
}
