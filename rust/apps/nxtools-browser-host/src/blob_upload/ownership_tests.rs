use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use nxtools_download::{
    BrowserBlob, DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadCommand, DownloadCommandEnvelope,
    DownloadCommandOutcome, DownloadEngine, RequestOrigin,
};
use uuid::Uuid;

struct Directory(PathBuf);

impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("nxtools-blob-owner-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&path).expect("temporary owner fixture");
        Self(path)
    }

    fn staging(&self) -> PathBuf {
        self.0.join("browser-blob-staging")
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn execute(engine: &DownloadEngine, command: DownloadCommand) -> DownloadCommandOutcome {
    engine
        .handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: Uuid::new_v4(),
            origin: RequestOrigin::Browser,
            command,
        })
        .outcome
}

fn staged_blob(root: &Path) -> BrowserBlob {
    std::fs::create_dir_all(root).expect("staging root");
    let blob = BrowserBlob {
        id: Uuid::new_v4(),
        file_name: "owned.bin".into(),
        media_type: Some("application/octet-stream".into()),
        size: 10,
    };
    let path = root.join(format!("{}.blob", blob.id));
    std::fs::write(&path, b"owned-blob").expect("staged bytes");
    std::fs::File::options()
        .write(true)
        .open(path)
        .expect("open fixture timestamp")
        .set_times(
            std::fs::FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(7200)),
        )
        .expect("age upload beyond host expiry");
    blob
}

fn open_dialog(engine: &DownloadEngine, blob: BrowserBlob, handoff_id: Uuid, destination: PathBuf) {
    let dialogs = engine
        .take_browser_dialog_receiver()
        .expect("private confirmation receiver");
    let command_engine = engine.clone();
    let command = std::thread::spawn(move || {
        execute(
            &command_engine,
            DownloadCommand::BrowserOpenBlobDialog {
                handoff_id,
                blob,
                save_directory: destination,
            },
        )
    });
    let started = std::time::Instant::now();
    let dialog = loop {
        if let Ok(dialog) = dialogs.try_recv() {
            break dialog;
        }
        assert!(
            !command.is_finished(),
            "command ended before confirmation delivery"
        );
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "bounded confirmation delivery"
        );
        std::thread::sleep(Duration::from_millis(2));
    };
    dialog.completion.send(Ok(())).expect("confirmation opened");
    assert!(matches!(
        command.join().expect("confirmation worker"),
        DownloadCommandOutcome::BrowserDialogAccepted {
            duplicate: false,
            ..
        }
    ));
}

#[test]
fn l7_confirmation_survives_host_expiry_and_imports_exact_bytes() {
    let directory = Directory::new();
    let engine = DownloadEngine::open(directory.0.join("downloads.sqlite3")).expect("engine owner");
    let root = directory.staging();
    let blob = staged_blob(&root);
    let handoff_id = Uuid::new_v4();
    let destination = directory.0.join("downloads");
    open_dialog(&engine, blob.clone(), handoff_id, destination.clone());
    let host = super::BlobUploadService::start(&root).expect("subsequent host startup");
    drop(host);
    assert!(
        root.join(format!("{}.blob", blob.id)).exists()
            || root
                .join("pending")
                .join(format!("{}.blob", blob.id))
                .exists(),
        "accepted confirmation retains its artifact past the host expiry threshold"
    );
    let result = execute(
        &engine,
        DownloadCommand::BrowserImportBlob {
            handoff_id,
            blob: blob.clone(),
            save_directory: destination.clone(),
            file_name: "owned.bin".into(),
            category_id: None,
            save_directory_explicit: true,
        },
    );
    assert!(
        matches!(result, DownloadCommandOutcome::BrowserAccepted { .. }),
        "confirmation import: {result:?}"
    );
    assert_eq!(
        std::fs::read(destination.join("owned.bin")).expect("imported artifact"),
        b"owned-blob"
    );
    assert!(
        !root
            .join("pending")
            .join(format!("{}.blob", blob.id))
            .exists()
    );
}

#[test]
fn l7_confirmation_cancel_cleans_the_engine_owned_artifact() {
    let directory = Directory::new();
    let engine = DownloadEngine::open(directory.0.join("downloads.sqlite3")).expect("engine owner");
    let root = directory.staging();
    let blob = staged_blob(&root);
    let handoff_id = Uuid::new_v4();
    open_dialog(
        &engine,
        blob.clone(),
        handoff_id,
        directory.0.join("downloads"),
    );
    let claimed = root.join("pending").join(format!("{}.blob", blob.id));
    assert!(
        claimed.exists(),
        "engine owns a separate pending-confirmation artifact"
    );
    let result = execute(
        &engine,
        DownloadCommand::BrowserDiscardBlob {
            handoff_id,
            blob_id: blob.id,
        },
    );
    assert!(matches!(
        result,
        DownloadCommandOutcome::Accepted { task_id: None }
    ));
    assert!(!claimed.exists());
    assert!(!root.join(format!("{}.blob", blob.id)).exists());
}

#[test]
fn l7_engine_startup_reclaims_confirmation_orphans_without_touching_uploads() {
    let directory = Directory::new();
    let root = directory.staging();
    let pending = root.join("pending");
    std::fs::create_dir_all(&pending).expect("former owner directory");
    let orphan = pending.join(format!("{}.blob", Uuid::new_v4()));
    let unclaimed = root.join(format!("{}.blob", Uuid::new_v4()));
    let uploading = root.join(format!("{}.uploading", Uuid::new_v4()));
    std::fs::write(&orphan, b"orphan").expect("pending orphan");
    std::fs::write(&unclaimed, b"unclaimed").expect("completed upload");
    std::fs::write(&uploading, b"receiving").expect("active upload");
    let _engine =
        DownloadEngine::open(directory.0.join("downloads.sqlite3")).expect("new engine owner");
    assert!(
        !orphan.exists(),
        "new engine startup reclaims previous confirmation ownership"
    );
    assert!(unclaimed.exists());
    assert!(uploading.exists());
}

#[test]
fn l7_consumer_not_ready_retains_the_original_upload_for_startup_retry() {
    let directory = Directory::new();
    let engine = DownloadEngine::open(directory.0.join("downloads.sqlite3")).expect("engine owner");
    let root = directory.staging();
    let blob = staged_blob(&root);
    let result = execute(
        &engine,
        DownloadCommand::BrowserOpenBlobDialog {
            handoff_id: Uuid::new_v4(),
            blob: blob.clone(),
            save_directory: directory.0.join("downloads"),
        },
    );
    assert!(matches!(result, DownloadCommandOutcome::Rejected { .. }));
    assert!(root.join(format!("{}.blob", blob.id)).exists());
    assert!(
        !root
            .join("pending")
            .join(format!("{}.blob", blob.id))
            .exists()
    );
}

#[test]
fn l7_rejected_confirmation_cleans_new_claim_but_timeout_preserves_late_owner() {
    for late in [false, true] {
        let directory = Directory::new();
        let engine =
            DownloadEngine::open(directory.0.join("downloads.sqlite3")).expect("engine owner");
        let root = directory.staging();
        let blob = staged_blob(&root);
        let handoff_id = Uuid::new_v4();
        let dialogs = engine
            .take_browser_dialog_receiver()
            .expect("private receiver");
        let command_engine = engine.clone();
        let request_blob = blob.clone();
        let destination = directory.0.join("downloads");
        let command = std::thread::spawn(move || {
            execute(
                &command_engine,
                DownloadCommand::BrowserOpenBlobDialog {
                    handoff_id,
                    blob: request_blob,
                    save_directory: destination,
                },
            )
        });
        let started = std::time::Instant::now();
        let dialog = loop {
            if let Ok(dialog) = dialogs.try_recv() {
                break dialog;
            }
            assert!(
                !command.is_finished(),
                "command must reach its private confirmation boundary"
            );
            assert!(
                started.elapsed() < Duration::from_secs(3),
                "bounded confirmation delivery"
            );
            std::thread::sleep(Duration::from_millis(2));
        };
        let claimed = root.join("pending").join(format!("{}.blob", blob.id));
        assert!(claimed.exists());
        if !late {
            dialog
                .completion
                .send(Err("synthetic window rejection".into()))
                .expect("reject confirmation");
        }
        assert!(matches!(
            command.join().expect("window admission worker"),
            DownloadCommandOutcome::Rejected { .. }
        ));
        assert_eq!(
            claimed.exists(),
            late,
            "only a potentially live late owner retains staging"
        );
        if late {
            let host = super::BlobUploadService::start(&root).expect("subsequent host startup");
            drop(host);
            assert!(
                claimed.exists(),
                "host expiry cannot claim a timed-out queued window's artifact"
            );
            assert!(
                dialog.completion.send(Ok(())).is_err(),
                "late acknowledgement cannot change terminal admission"
            );
            assert!(matches!(
                execute(
                    &engine,
                    DownloadCommand::BrowserDiscardBlob {
                        handoff_id,
                        blob_id: blob.id
                    }
                ),
                DownloadCommandOutcome::Accepted { task_id: None }
            ));
            assert!(!claimed.exists());
        }
        assert!(!root.join(format!("{}.blob", blob.id)).exists());
    }
}
