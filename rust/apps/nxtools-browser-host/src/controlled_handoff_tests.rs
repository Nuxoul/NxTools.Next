use super::*;
use nxtools_download::{
    BrowserDownloadDialogSource, DownloadEngine, DownloadResolutionMode, DownloadResolutionOptions,
};
use nxtools_download_ipc::{DownloadHostActivity, run_download_host};

fn fixture() -> (AppPaths, DownloadEngine, String) {
    let root = std::env::temp_dir().join(format!("browser-controlled-{}", Uuid::new_v4()));
    let paths = AppPaths {
        config_dir: root.join("config"),
        data_dir: root.join("data"),
        cache_dir: root.join("cache"),
        log_dir: root.join("logs"),
    };
    std::fs::create_dir_all(&paths.config_dir).expect("temporary config directory");
    let mut config = AppConfig::default();
    config.extensions.bundled_defaults_initialized = true;
    config.download.default_directory = root.join("downloads").to_string_lossy().into_owned();
    config
        .save_to(&paths.config_file())
        .expect("temporary settings");
    let engine =
        DownloadEngine::open(root.join("downloads.sqlite3")).expect("temporary SQLite engine");
    let pipe = format!(r"\\.\pipe\nxtools-controlled-{}", Uuid::new_v4());
    let host_pipe = pipe.clone();
    let host_engine = engine.clone();
    std::thread::spawn(move || {
        let _ = run_download_host(&host_pipe, host_engine, DownloadHostActivity::default());
    });
    (paths, engine, pipe)
}

fn operation(pipe: &str, kind: &str, handoff: Uuid, task: Option<Uuid>) -> BrowserResponse {
    operate_with_client(
        handoff.to_string(),
        kind,
        json!({"handoffId": handoff, "taskId": task}),
        || DownloadIpcClient::connect(pipe, Duration::from_secs(2)),
    )
    .expect("controlled operation crosses owned pipe")
}

fn prepare_pending(
    paths: &AppPaths,
    engine: &DownloadEngine,
    pipe: &str,
    handoff: Uuid,
) -> DownloadRequest {
    let dialogs = engine
        .take_browser_dialog_receiver()
        .expect("desktop dialog receiver");
    let consumer = std::thread::spawn(move || {
        let dialog = dialogs.recv_blocking().expect("original desktop request");
        assert_eq!(dialog.handoff_id, handoff);
        assert_eq!(dialog.correlation_id, handoff);
        let BrowserDownloadDialogSource::ControlledDownload(request) = dialog.source else {
            panic!("controlled source required")
        };
        dialog
            .completion
            .send(Ok(()))
            .expect("created confirmation window");
        *request
    });
    let response = prepare_with_client(handoff.to_string(), json!({
        "handoffId": handoff, "url": "https://fixture.test/controlled.zip", "fileName": "controlled.zip",
        "sourcePageUrl": "https://page.example.test/controlled", "sourcePageTitle": "Controlled page",
        "cookie": "synthetic=container", "openDialog": true, "startImmediately": false
    }), paths, |mode, correlation| {
        assert_eq!(mode, DesktopStartMode::Interactive);
        assert_eq!(correlation, handoff);
        DownloadIpcClient::connect(pipe, Duration::from_secs(2))
    }).expect("prepare crosses owned pipe");
    assert!(!response.accepted);
    assert_eq!(
        response.data.expect("pending metadata")["browserHandoffState"],
        "pending"
    );
    assert!(
        engine
            .tasks()
            .expect("no task before confirmation")
            .is_empty()
    );
    consumer.join().expect("confirmation request")
}

fn accept(pipe: &str, handoff: Uuid, request: DownloadRequest) -> DownloadTaskId {
    let mut client = DownloadIpcClient::connect(pipe, Duration::from_secs(2))
        .expect("desktop confirmation client");
    let result = client
        .command(command_envelope(
            handoff,
            DownloadCommand::BrowserAcceptHandoff {
                handoff_id: handoff,
                request,
                mode: DownloadSubmitMode::QueueOnly,
                resolution: DownloadResolutionOptions {
                    operation_id: handoff,
                    mode: DownloadResolutionMode::Bypass,
                },
            },
            RequestOrigin::Browser,
        ))
        .expect("durable held acceptance");
    let DownloadCommandOutcome::BrowserHandoff {
        handoff_id,
        state: BrowserHandoffState::Held,
        task_id: Some(task),
    } = result.outcome
    else {
        panic!("held receipt required: {:?}", result.outcome)
    };
    assert_eq!(handoff_id, handoff);
    task
}

#[test]
fn owned_pipe_pending_held_commit_replay_and_stale_abort_keep_one_task() {
    let (paths, engine, pipe) = fixture();
    let handoff = Uuid::new_v4();
    let request = prepare_pending(&paths, &engine, &pipe, handoff);
    assert_eq!(request.cookie.as_deref(), Some("synthetic=container"));
    assert_eq!(
        request.source_page.as_ref().expect("controlled page").url,
        "https://page.example.test/controlled"
    );
    let task = accept(&pipe, handoff, request);
    let held = operation(&pipe, "get_browser_handoff", handoff, None);
    assert!(held.accepted);
    assert_eq!(held.task_id, Some(task.as_uuid().to_string()));
    assert_eq!(
        held.data.expect("held metadata")["browserHandoffState"],
        "held"
    );
    assert_eq!(engine.tasks().expect("durable task").len(), 1);
    for _ in 0..2 {
        let committed = operation(
            &pipe,
            "commit_browser_handoff",
            handoff,
            Some(*task.as_uuid()),
        );
        assert!(committed.accepted);
        assert_eq!(
            committed.data.expect("commit metadata")["browserHandoffState"],
            "committed"
        );
    }
    let stale = operation(&pipe, "abort_browser_handoff", handoff, None);
    assert_eq!(
        stale.data.expect("stale abort metadata")["browserHandoffState"],
        "committed"
    );
    assert_eq!(engine.tasks().expect("no duplicate task").len(), 1);
}

#[test]
fn owned_pipe_abort_blocks_late_confirmation_and_unknown_abort_is_durable() {
    let (paths, engine, pipe) = fixture();
    let handoff = Uuid::new_v4();
    let request = prepare_pending(&paths, &engine, &pipe, handoff);
    let aborted = operation(&pipe, "abort_browser_handoff", handoff, None);
    assert!(!aborted.accepted);
    assert_eq!(
        aborted.data.expect("abort metadata")["browserHandoffState"],
        "aborted"
    );
    let mut client = DownloadIpcClient::connect(&pipe, Duration::from_secs(2))
        .expect("late desktop confirmation");
    let late = client
        .command(command_envelope(
            handoff,
            DownloadCommand::BrowserAcceptHandoff {
                handoff_id: handoff,
                request,
                mode: DownloadSubmitMode::StartNow,
                resolution: DownloadResolutionOptions::default(),
            },
            RequestOrigin::Browser,
        ))
        .expect("late confirmation result");
    assert!(matches!(
        late.outcome,
        DownloadCommandOutcome::BrowserHandoff {
            state: BrowserHandoffState::Aborted,
            task_id: None,
            ..
        }
    ));
    assert!(engine.tasks().expect("no late task").is_empty());
    let unknown = Uuid::new_v4();
    let tombstone = operation(&pipe, "abort_browser_handoff", unknown, None);
    assert_eq!(
        tombstone.data.expect("unknown abort")["browserHandoffState"],
        "aborted"
    );
    let status = operation(&pipe, "get_browser_handoff", unknown, None);
    assert_eq!(
        status.data.expect("persisted abort")["browserHandoffState"],
        "aborted"
    );
}
