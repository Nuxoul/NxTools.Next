#![cfg(windows)]
use nxtools_archive::{
    ArchiveError, OperationControl,
    interaction::{InteractionPort, Prompt, PromptResponse},
    output_readmission::{self, AttemptResult, OutputBudget},
    worker::{WorkerRequest, read_frame, write_frame},
};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

struct Host(Child);
impl Drop for Host {
    fn drop(&mut self) {
        drop(self.0.stdin.take());
        let cleanup = std::time::Instant::now();
        while self.0.try_wait().ok().flatten().is_none()
            && cleanup.elapsed() < Duration::from_secs(3)
        {
            std::thread::sleep(Duration::from_millis(10));
        }
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
fn fixture(root: &std::path::Path, mode: &str) -> PathBuf {
    let source = root.join("tiny.zip");
    let script = "import sys,zipfile,struct\np=sys.argv[1];m=sys.argv[2]\nwith zipfile.ZipFile(p,'w',compression=zipfile.ZIP_DEFLATED) as z:\n z.writestr('payload.bin',bytes(range(256))*16)\n if m=='total': z.writestr('second.bin',b'B'*2048)\nif m=='actual':\n b=bytearray(open(p,'rb').read()); c=b.index(b'PK\\x01\\x02');struct.pack_into('<I',b,22,2048);struct.pack_into('<I',b,c+24,2048);open(p,'wb').write(b)";
    assert!(
        Command::new("python")
            .args(["-c", script])
            .arg(&source)
            .arg(mode)
            .status()
            .expect("independent tiny fixture")
            .success()
    );
    source
}
#[test]
fn real_rust_coordinator_reenters_original_native_admission_only_after_cleanup() {
    let scratch = PathBuf::from(std::env::var_os("PI_SCRATCH_DIR").expect("scratch"));
    let apphost = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe");
    assert!(apphost.is_file(), "build isolated apphost first");
    for mode in ["member", "total", "cancel", "source_changed", "actual"] {
        eprintln!("readmission scenario: {mode}");
        let root = tempfile::Builder::new()
            .prefix("readmit-chain-")
            .tempdir_in(&scratch)
            .expect("owned fixture");
        let source = fixture(root.path(), mode);
        let output = root.path().join("missing").join("output");
        let control = OperationControl::default();
        let port = InteractionPort::new(control.operation_id(), 1);
        let mut request = WorkerRequest::open(
            control.operation_id(),
            source.to_str().expect("path").into(),
        );
        request.version = 2;
        request.operation = "extract".into();
        request.codepage = 936;
        request.conflict_policy =
            Some(serde_json::from_value(json!("auto_rename")).expect("policy"));
        request.extract_options =
            Some(nxtools_archive::worker::ExtractOptions { copy_motw: false, reclamation: None });
        request.limits.max_entries = 100;
        request.limits.max_entry_bytes = if mode == "total" { 8192 } else { 1024 };
        request.limits.max_output_bytes = if mode == "total" { 4096 } else { 8192 };
        let original_id = request.operation_id;
        let original_limits = request.limits;
        let history = Arc::new(Mutex::new(Vec::<Value>::new()));
        let worker_history = history.clone();
        let worker_control = control.clone();
        let worker_port = port.clone();
        let target = output.clone();
        let host_path = apphost.clone();
        let (finished_tx, finished_rx) = mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            let mut host = Host(
                Command::new(host_path)
                    .arg("--archive-output-readmission-host")
                    .arg(&target)
                    .env(
                        "NXTOOLS_ARCHIVE_WORKER_FIXTURE",
                        env!("CARGO_BIN_EXE_nxtools-archive-worker"),
                    )
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("original runner test adapter"),
            );
            let mut attempts = 0;
            let result = output_readmission::execute(
                request,
                &worker_control,
                &worker_port,
                |attempt, attempt_control| {
                    assert_eq!(attempt.operation_id, attempt_control.operation_id());
                    attempts += 1;
                    assert_eq!(attempt.path, source.to_str().expect("same source"));
                    assert_eq!(attempt.codepage, 936);
                    assert_eq!(
                        attempt.limits.max_working_bytes,
                        original_limits.max_working_bytes
                    );
                    assert_eq!(
                        attempt.limits.max_metadata_bytes,
                        original_limits.max_metadata_bytes
                    );
                    assert_eq!(attempt.limits.max_entries, 100);
                    assert!(!attempt.extract_options.expect("kept options").copy_motw);
                    assert_eq!(
                        serde_json::to_value(attempt.conflict_policy).expect("kept policy"),
                        json!("auto_rename")
                    );
                    write_frame(host.0.stdin.as_mut().expect("host input"), attempt)
                        .expect("private test request");
                    let packet: Value = read_frame(host.0.stdout.as_mut().expect("host output"))
                        .expect("host response")
                        .expect("actual cleaned result");
                    assert_eq!(packet["audit"]["attempt"], attempts);
                    assert_eq!(packet["audit"]["started"], 1, "{packet}");
                    assert_eq!(packet["audit"]["active"], 0);
                    assert_eq!(packet["audit"]["source_released"], true);
                    assert_eq!(packet["audit"]["helper_released"], true);
                    assert_eq!(packet["audit"]["source_acquired"], true);
                    assert_eq!(packet["audit"]["codepage"], 936);
                    assert_eq!(packet["audit"]["conflict_policy"], "auto_rename");
                    assert_eq!(packet["audit"]["copy_motw"], false);
                    assert_eq!(
                        packet["audit"]["memory_budget"],
                        original_limits.max_working_bytes
                            + original_limits.max_metadata_bytes
                            + (64 << 20)
                    );
                    let result: AttemptResult =
                        serde_json::from_value(packet.clone()).expect("typed cleaned facts");
                    worker_history.lock().expect("history").push(packet);
                    Ok(result)
                },
            );
            drop(host.0.stdin.take());
            let exit = host.0.wait().expect("adapter exit");
            assert!(exit.success(), "clean adapter shutdown");
            finished_tx.send((result, attempts)).expect("report");
        });
        let deadline = std::time::Instant::now();
        let pending = loop {
            if let Some(pending) = port.pending() {
                break pending;
            }
            if let Ok((result, _)) = finished_rx.try_recv() {
                panic!("no budget prompt: {:?}", result.err());
            }
            assert!(deadline.elapsed() < Duration::from_secs(15));
            std::thread::sleep(Duration::from_millis(5));
        };
        let Prompt::OutputBudget { current, required } = pending.prompt else {
            panic!("separate output-budget prompt, never Risk");
        };
        assert_eq!(
            history.lock().expect("history").len(),
            1,
            "initial actual exit before prompt"
        );
        assert_eq!(
            history.lock().expect("history")[0]["audit"]["output_exists"],
            false
        );
        assert_eq!(current.entry_bytes, original_limits.max_entry_bytes);
        assert_eq!(current.output_bytes, original_limits.max_output_bytes);
        assert_eq!(
            required.entry_bytes,
            if mode == "actual" { 2048 } else { 4096 }
        );
        assert_eq!(
            required.output_bytes,
            if mode == "total" {
                6144
            } else if mode == "actual" {
                2048
            } else {
                4096
            }
        );
        assert!(
            !root.path().join("missing").exists(),
            "no target directory side effects before approval"
        );
        if mode == "cancel" {
            assert!(
                port.respond(pending.ticket, PromptResponse::Cancel)
                    .expect("cancel approval")
            );
        } else {
            let approved = OutputBudget {
                entry_bytes: current.entry_bytes.max(required.entry_bytes),
                output_bytes: current.output_bytes.max(required.output_bytes),
            };
            let mut stale = pending.ticket;
            stale.prompt_id = uuid::Uuid::new_v4();
            assert!(
                !port
                    .respond(stale, PromptResponse::OutputBudget { approved })
                    .expect("old approval ignored")
            );
            if mode == "source_changed" {
                std::fs::write(root.path().join("tiny.zip"), b"replaced source")
                    .expect("replace during approval");
            }
            assert!(
                port.respond(pending.ticket, PromptResponse::OutputBudget { approved })
                    .expect("finite larger output approval")
            );
            assert!(
                !port
                    .respond(pending.ticket, PromptResponse::OutputBudget { approved })
                    .expect("duplicate ignored")
            );
        }
        let (result, attempts) = finished_rx
            .recv_timeout(Duration::from_secs(15))
            .expect("coordinator terminates");
        worker.join().expect("coordinator thread");
        assert_eq!(attempts, if mode == "cancel" { 1 } else { 2 });
        if mode == "cancel" {
            assert!(matches!(result, Err(ArchiveError::Cancelled)));
            assert!(!output.exists());
        } else {
            let result = result.expect("actual result");
            assert_ne!(result.operation_id, original_id);
            if mode == "source_changed" {
                assert_eq!(result.error_code.as_deref(), Some("SourceChanged"));
                assert!(!output.exists());
            } else if mode == "actual" {
                assert_eq!(result.error_code.as_deref(), Some("ResourceLimit"));
                assert_eq!(result.files_committed, 0);
                assert!(!output.join("payload.bin").exists());
                assert_eq!(
                    std::fs::read_dir(&output).expect("prepared output").count(),
                    0,
                    "owned partial removed after actual overflow"
                );
            } else {
                assert!(result.error_code.is_none());
                assert_eq!(
                    std::fs::read(output.join("payload.bin")).expect("actual native bytes"),
                    (0u8..=255).cycle().take(4096).collect::<Vec<_>>()
                );
                if mode == "total" {
                    assert_eq!(
                        std::fs::read(output.join("second.bin")).expect("second native file"),
                        vec![b'B'; 2048]
                    );
                }
            }
        }
        assert!(port.pending().is_none());
    }
}
