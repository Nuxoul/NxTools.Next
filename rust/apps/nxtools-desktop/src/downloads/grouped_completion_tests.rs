use super::*;
use gpui_kit::TestAppContext;
use nxtools_download::BrowserDownloadSubmission;

fn request(name: &str, directory: &Path) -> DownloadRequest {
    DownloadRequest {
        source_url: format!("https://example.test/{name}"),
        save_directory: directory.to_path_buf(),
        save_directory_explicit: false,
        file_name: Some(name.into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        source_page: None,
        certificate_policy: Default::default(),
        connection_limit: Some(1),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    }
}

fn accept(engine: &DownloadEngine, directory: &Path, names: &[&str]) -> Vec<DownloadTaskId> {
    let receipt = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserBatchSubmit {
            submissions: names
                .iter()
                .map(|name| BrowserDownloadSubmission {
                    handoff_id: uuid::Uuid::new_v4(),
                    request: request(name, directory),
                })
                .collect(),
        },
    });
    let DownloadCommandOutcome::BrowserBatchAccepted { results } = receipt.outcome else {
        panic!("browser batch intake rejected");
    };
    results
        .into_iter()
        .map(|result| {
            assert!(!result.duplicate);
            result.task_id.expect("accepted task")
        })
        .collect()
}

fn terminal(id: DownloadTaskId, status: DownloadStatus) -> CompletionTerminal {
    CompletionTerminal {
        task_id: id,
        status,
        directory: None,
        display_name: "fixture.bin".into(),
        error_message: None,
    }
}

#[gpui_kit::test]
fn grouped_and_standalone_completion_notices_have_no_actions(cx: &mut TestAppContext) {
    let (notifications, surface) = crate::notifications::presentation_tests::mount(cx);
    let directory = tempfile::tempdir().expect("temporary download directory");
    let engine = DownloadEngine::open_in_memory().expect("download engine");
    let members = accept(&engine, directory.path(), &["one.bin", "two.bin"]);
    let mut config = AppConfig::default();
    config.download.completion_sound_enabled = true;
    config.download.open_directory_on_completion = false;
    let original_settings = config.download.clone();
    let config_path = directory.path().join("config.json");
    let (platform, platform_endpoint) = nxtools_ipc::platform_bridge(16);
    platform_endpoint.set_connected_for_contract_test(true);
    let application = surface.update(|_, cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                engine.clone(),
                Rc::new(platform),
                notifications.clone(),
                DownloadRuntimeContext {
                    fallback_default_directory: directory.path().to_path_buf(),
                    config,
                    config_path: config_path.clone(),
                    log_directory: directory.path().to_path_buf(),
                    database_path: directory.path().join("downloads.sqlite3"),
                    ffmpeg_install_directory: directory.path().join("ffmpeg"),
                },
                cx,
            )
        })
    });
    surface.update(|_, cx| application.update(cx, |app, cx| app.refresh_snapshot(false, cx)));
    let mut snapshot = engine
        .completion_snapshot()
        .expect("batch membership snapshot");
    assert_eq!(
        snapshot
            .tasks
            .iter()
            .filter(|task| task.completion_batch_id.is_some())
            .count(),
        2
    );
    let correlation = snapshot.tasks[0]
        .completion_batch_id
        .expect("original batch correlation");
    surface.update(|_, cx| {
        application.update(cx, |app, cx| {
            app.completion_actions
                .record_terminal_attempt(terminal(members[0], DownloadStatus::Completed), 0);
            snapshot
                .tasks
                .iter_mut()
                .find(|task| task.task_id == members[0])
                .expect("first member")
                .status = DownloadStatus::Completed;
            app.completion_actions.settle_completion_members(&snapshot);
            app.flush_completion_feedback(cx);
        })
    });
    surface.run_until_parked();
    assert!(
        notifications.read_with(surface, |center, _| center.items().is_empty()),
        "partial group emitted a notice"
    );
    surface.update(|_, cx| {
        application.update(cx, |app, cx| {
            app.completion_actions
                .record_terminal_attempt(terminal(members[1], DownloadStatus::Failed), 0);
            snapshot
                .tasks
                .iter_mut()
                .find(|task| task.task_id == members[1])
                .expect("second member")
                .status = DownloadStatus::Failed;
            app.completion_actions.settle_completion_members(&snapshot);
            app.flush_completion_feedback(cx);
        })
    });
    surface.run_until_parked();
    let (notice_id, generation) = notifications.read_with(surface, |center, _| {
        assert_eq!(center.items().len(), 1);
        assert_eq!(center.items()[0].message.as_ref(), "完成 1 个，失败 1 个");
        assert!(
            center.items()[0].action.is_none(),
            "download notice has a primary action"
        );
        assert!(
            center.items()[0].additional_actions.is_empty(),
            "download notice has extra actions"
        );
        (
            center.items()[0].id.to_string(),
            center.items()[0].generation(),
        )
    });
    assert_eq!(notice_id, format!("download-completion-{correlation}"));
    let mut sound_commands = 0;
    for _ in 0..16 {
        let Ok(envelope) = platform_endpoint.try_receive_command_for_contract_test() else {
            break;
        };
        if matches!(
            envelope.message,
            nxtools_protocol::Message::PlatformCommand(
                PlatformCommand::PlaySystemNotificationSound
            )
        ) {
            assert_eq!(
                envelope.correlation_id,
                Some(correlation),
                "native sound lost original batch identity"
            );
            sound_commands += 1;
        }
    }
    assert_eq!(
        sound_commands, 1,
        "settled group must play at most one sound"
    );
    surface.update(|_, cx| application.update(cx, |app, cx| app.flush_completion_feedback(cx)));
    surface.run_until_parked();
    assert_eq!(
        notifications.read_with(surface, |center, _| center.items()[0].generation()),
        generation
    );
    surface.update(|_, cx| {
        application.update(cx, |app, cx| {
            app.completion_actions
                .record_terminal(terminal(DownloadTaskId::new(), DownloadStatus::Completed));
            app.flush_completion_feedback(cx);
        })
    });
    surface.run_until_parked();
    surface.update(|_, cx| {
        application.update(cx, |app, cx| {
            app.completion_actions
                .record_terminal(terminal(DownloadTaskId::new(), DownloadStatus::Failed));
            app.flush_completion_feedback(cx);
        })
    });
    surface.run_until_parked();
    notifications.read_with(surface, |center, _| {
        assert_eq!(center.items().len(), 3);
        for notice in center.items() {
            assert!(
                notice.action.is_none(),
                "download notice has a primary action"
            );
            assert!(
                notice.additional_actions.is_empty(),
                "download notice has extra actions"
            );
        }
    });
    let mut standalone_sounds = 0;
    for _ in 0..16 {
        let Ok(envelope) = platform_endpoint.try_receive_command_for_contract_test() else {
            break;
        };
        standalone_sounds += usize::from(matches!(
            envelope.message,
            nxtools_protocol::Message::PlatformCommand(
                PlatformCommand::PlaySystemNotificationSound
            )
        ));
    }
    assert_eq!(
        standalone_sounds, 1,
        "only a successful standalone task plays a sound"
    );
    application.update(surface, |app, _| {
        assert_eq!(app.config.download, original_settings);
    });
    assert!(
        !config_path.exists(),
        "completion feedback wrote permanent config"
    );
}

#[gpui_kit::test]
fn browser_batch_http_terminals_publish_one_production_notification(cx: &mut TestAppContext) {
    use std::io::{Read as _, Write as _};
    use std::sync::atomic::{AtomicBool, Ordering};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("local HTTP fixture");
    let address = listener.local_addr().expect("fixture address");
    listener
        .set_nonblocking(true)
        .expect("bounded fixture accept");
    let stop = std::sync::Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let worker = std::thread::spawn(move || {
        while !worker_stop.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_nonblocking(false)
                        .expect("blocking accepted socket");
                    stream
                        .set_read_timeout(Some(Duration::from_secs(1)))
                        .expect("bounded fixture request");
                    let mut request = [0u8; 4096];
                    let _ = stream.read(&mut request);
                    let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\nfixture!");
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(_) => break,
            }
        }
    });
    let (notifications, surface) = crate::notifications::presentation_tests::mount(cx);
    let directory = tempfile::tempdir().expect("HTTP fixture directory");
    let engine = DownloadEngine::open_in_memory().expect("HTTP fixture engine");
    let receipt = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserBatchSubmit {
            submissions: ["alpha.bin", "beta.bin"]
                .into_iter()
                .map(|name| {
                    let mut request = request(name, directory.path());
                    request.source_url = format!("http://{address}/{name}");
                    BrowserDownloadSubmission {
                        handoff_id: uuid::Uuid::new_v4(),
                        request,
                    }
                })
                .collect(),
        },
    });
    let DownloadCommandOutcome::BrowserBatchAccepted { results } = receipt.outcome else {
        panic!("local batch intake rejected");
    };
    let ids: Vec<_> = results
        .into_iter()
        .map(|result| result.task_id.expect("accepted task id"))
        .collect();
    let (platform, _) = nxtools_ipc::platform_bridge(16);
    let mut config = AppConfig::default();
    config.download.completion_sound_enabled = false;
    config.download.open_directory_on_completion = false;
    config.download.show_progress_window = false;
    config.download.show_completion_window = true;
    let application = surface.update(|_, cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                engine.clone(),
                Rc::new(platform),
                notifications.clone(),
                DownloadRuntimeContext {
                    fallback_default_directory: directory.path().into(),
                    config,
                    config_path: directory.path().join("config.json"),
                    log_directory: directory.path().into(),
                    database_path: directory.path().join("downloads.db"),
                    ffmpeg_install_directory: directory.path().join("ffmpeg"),
                },
                cx,
            )
        })
    });
    surface.update(|_, cx| application.update(cx, |app, cx| app.refresh_snapshot(false, cx)));
    surface.update(|_, cx| {
        application.update(cx, |app, cx| {
            let _ = app.configure_task_window_activation_test(42);
            app.request_task_window(ids[0], cx);
        })
    });
    surface.run_until_parked();
    let generation = application.read_with(surface, |app, _| {
        app.task_windows
            .get(&ids[0])
            .expect("explicit active task window")
            .generation
    });
    for id in &ids {
        engine.resume(*id).expect("resume browser batch item");
    }
    let deadline = Instant::now() + Duration::from_secs(12);
    while Instant::now() < deadline {
        surface.run_until_parked();
        if ids.iter().all(|id| {
            engine
                .task(*id)
                .expect("read engine terminal")
                .is_some_and(|task| task.task.status() == DownloadStatus::Completed)
        }) {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(
        ids.iter().all(|id| engine
            .task(*id)
            .expect("read task")
            .is_some_and(|task| task.task.status() == DownloadStatus::Completed)),
        "local HTTP batch failed to complete"
    );
    surface.update(|_, cx| application.update(cx, |app, cx| app.refresh_snapshot(false, cx)));
    surface
        .executor()
        .advance_clock(COMPLETION_FEEDBACK_DELAY + Duration::from_millis(50));
    surface.run_until_parked();
    surface.update(|_, cx| application.update(cx, |app, cx| app.flush_completion_feedback(cx)));
    surface.run_until_parked();
    notifications.read_with(surface, |center, _| {
        assert_eq!(center.items().len(), 1, "real terminals were not grouped");
        assert_eq!(center.items()[0].message.as_ref(), "2 个下载任务已完成");
    });
    application.read_with(surface, |app, _| {
        assert_eq!(
            app.task_windows
                .get(&ids[0])
                .expect("existing keyed window transformed")
                .generation,
            generation
        );
        assert!(
            !app.task_windows.contains_key(&ids[1]),
            "batch member opened an individual passive completion window"
        );
    });
    stop.store(true, Ordering::Release);
    worker.join().expect("join bounded local HTTP fixture");
}
