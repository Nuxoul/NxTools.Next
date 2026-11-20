use super::*;
use gpui_kit::{Modifiers, TestAppContext, VisualTestContext};
use std::io::{Read as _, Write as _};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

struct ProbeServer {
    address: std::net::SocketAddr,
    requests: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    hold_response: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl ProbeServer {
    fn new() -> Self {
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("controlled handoff probe listener");
        let address = listener.local_addr().expect("controlled probe address");
        listener
            .set_nonblocking(true)
            .expect("bounded controlled probe listener");
        let requests = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_requests = Arc::clone(&requests);
        let hold_response = Arc::new(AtomicBool::new(false));
        let worker_hold_response = Arc::clone(&hold_response);
        let worker_stop = Arc::clone(&stop);
        let worker = std::thread::spawn(move || {
            while !worker_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_nonblocking(false)
                            .expect("accepted controlled probe stream");
                        stream
                            .set_read_timeout(Some(Duration::from_secs(1)))
                            .expect("bounded controlled probe read");
                        let mut request = [0_u8; 4096];
                        let _ = stream.read(&mut request);
                        worker_requests.fetch_add(1, Ordering::AcqRel);
                        while worker_hold_response.load(Ordering::Acquire) {
                            if worker_stop.load(Ordering::Acquire) {
                                return;
                            }
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\nfixture!");
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            address,
            requests,
            stop,
            hold_response,
            worker: Some(worker),
        }
    }

    fn url(&self, name: &str) -> String {
        format!("http://{}/{name}", self.address)
    }

    fn request_count(&self) -> usize {
        self.requests.load(Ordering::Acquire)
    }

    fn finish(mut self) {
        self.stop.store(true, Ordering::Release);
        self.worker
            .take()
            .expect("controlled probe worker")
            .join()
            .expect("bounded controlled probe worker");
    }
}

impl Drop for ProbeServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

fn controlled_request(directory: &Path, source_url: String) -> DownloadRequest {
    DownloadRequest {
        source_url,
        save_directory: directory.to_path_buf(),
        save_directory_explicit: false,
        file_name: None,
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
        duplicate_strategy: Default::default(),
    }
}

fn engine_outcome(
    application: &Entity<DownloadApplication>,
    cx: &TestAppContext,
    command: DownloadCommand,
    request_id: uuid::Uuid,
) -> DownloadCommandOutcome {
    application.read_with(cx, |application, _| {
        application
            .engine
            .handle_command(DownloadCommandEnvelope {
                protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                request_id,
                origin: RequestOrigin::Ui,
                command,
            })
            .outcome
    })
}

fn handoff_status(
    application: &Entity<DownloadApplication>,
    cx: &TestAppContext,
    handoff_id: uuid::Uuid,
) -> (BrowserHandoffState, Option<DownloadTaskId>) {
    match engine_outcome(
        application,
        cx,
        DownloadCommand::BrowserHandoffStatus { handoff_id },
        handoff_id,
    ) {
        DownloadCommandOutcome::BrowserHandoff {
            handoff_id: actual,
            state,
            task_id,
        } => {
            assert_eq!(actual, handoff_id);
            (state, task_id)
        }
        outcome => panic!("unexpected controlled handoff status: {outcome:?}"),
    }
}

fn prepare_controlled(
    application: &Entity<DownloadApplication>,
    cx: &mut TestAppContext,
    request: DownloadRequest,
    handoff_id: uuid::Uuid,
) -> (AnyWindowHandle, Entity<NewDownloadView>) {
    application.update(cx, |application, _| {
        application
            .new_download_native_handles_override
            .get_or_insert((701, 702));
    });
    let engine = application.read_with(cx, |application, _| application.engine.clone());
    let worker = std::thread::spawn(move || {
        engine
            .handle_command(DownloadCommandEnvelope {
                protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                request_id: handoff_id,
                origin: RequestOrigin::Browser,
                command: DownloadCommand::BrowserPrepareHandoff {
                    handoff_id,
                    request,
                    open_dialog: true,
                    mode: DownloadSubmitMode::StartNow,
                },
            })
            .outcome
    });
    let deadline = Instant::now() + Duration::from_secs(4);
    let intake = loop {
        cx.run_until_parked();
        let intake = application.read_with(cx, |application, _| {
            application
                .new_download_window
                .zip(application.new_download_view.clone())
        });
        if let Some(intake) = intake {
            break intake;
        }
        assert!(
            Instant::now() < deadline,
            "controlled prepare opens the production GPUI source"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    match worker.join().expect("controlled prepare worker") {
        DownloadCommandOutcome::BrowserHandoff {
            handoff_id: actual,
            state: BrowserHandoffState::Pending,
            task_id: None,
        } => assert_eq!(actual, handoff_id),
        outcome => panic!("unexpected controlled prepare result: {outcome:?}"),
    }
    intake
}

fn wait_for_fixed_confirmation(
    application: &Entity<DownloadApplication>,
    cx: &mut TestAppContext,
    source: AnyWindowHandle,
) -> (AnyWindowHandle, Entity<NewDownloadView>) {
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        cx.run_until_parked();
        if let Some(command_id) = application.read_with(cx, |application, _| {
            application
                .pending_new_download_handoff
                .as_ref()
                .map(|pending| pending.command_id)
        }) {
            application.update(cx, |application, cx| {
                application.finish_new_download_handoff(
                    command_id,
                    true,
                    "test_native_presented",
                    cx,
                );
            });
            cx.run_until_parked();
        }
        let intake = application.read_with(cx, |application, _| {
            application
                .new_download_window
                .zip(application.new_download_view.clone())
        });
        if let Some((target, view)) = intake
            && target != source
            && view.read_with(cx, |view, _| !view.probing && view.probe.is_some())
        {
            assert!(source.update(cx, |_, _, _| {}).is_err());
            return (target, view);
        }
        assert!(
            Instant::now() < deadline,
            "fixed confirmation missing: (same source, probing, probe, staged, pending, active owner)={:?}",
            application.read_with(cx, |application, cx| {
                application
                    .new_download_window
                    .zip(application.new_download_view.as_ref())
                    .map(|(window, view)| {
                        let view = view.read(cx);
                        (
                            window == source,
                            view.probing,
                            view.probe.is_some(),
                            view.handoff_staged,
                            view.handoff_pending,
                            view.controlled_handoff
                                .as_ref()
                                .is_some_and(|context| context.is_active_owner(window)),
                        )
                    })
            })
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn wait_for_pending_fixed_confirmation(
    application: &Entity<DownloadApplication>,
    cx: &mut TestAppContext,
) -> (AnyWindowHandle, uuid::Uuid) {
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        cx.run_until_parked();
        if let Some((target, command_id)) = application.read_with(cx, |application, _| {
            application
                .pending_new_download_handoff
                .as_ref()
                .map(|pending| (pending.target, pending.command_id))
        }) {
            return (target, command_id);
        }
        assert!(
            Instant::now() < deadline,
            "controlled probe reaches pending native confirmation presentation"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[gpui_kit::test]
fn controlled_source_transfers_identity_then_cancel_aborts_without_task(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (application, directory) =
        super::tests::download_application_for_window_test(cx, "controlled-cancel");
    let server = ProbeServer::new();
    let handoff_id = uuid::Uuid::new_v4();
    let (source, _) = prepare_controlled(
        &application,
        cx,
        controlled_request(&directory, server.url("cancel.bin")),
        handoff_id,
    );
    let (target, target_view) = wait_for_fixed_confirmation(&application, cx, source);
    target_view.read_with(cx, |view, _| {
        let context = view
            .controlled_handoff
            .as_ref()
            .expect("controlled identity");
        assert_eq!(context.handoff_id, handoff_id);
        assert!(context.is_active_owner(target));
    });
    assert_eq!(
        handoff_status(&application, cx, handoff_id),
        (BrowserHandoffState::Pending, None)
    );

    let mut surface = VisualTestContext::from_window(target, cx);
    surface.refresh().expect("controlled confirmation");
    let cancel = surface
        .debug_bounds("download-new-cancel-action")
        .expect("production cancel action");
    surface.simulate_click(cancel.center(), Modifiers::default());
    surface.run_until_parked();
    assert!(target.update(cx, |_, _, _| {}).is_err());
    assert_eq!(
        handoff_status(&application, cx, handoff_id),
        (BrowserHandoffState::Aborted, None)
    );
    application.read_with(cx, |application, _| {
        assert!(application.engine.tasks().expect("task store").is_empty());
    });
    server.finish();
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn controlled_confirm_persists_held_without_commit_or_transfer_bytes(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (application, directory) =
        super::tests::download_application_for_window_test(cx, "controlled-held");
    let server = ProbeServer::new();
    let handoff_id = uuid::Uuid::new_v4();
    let (source, _) = prepare_controlled(
        &application,
        cx,
        controlled_request(&directory, server.url("held.bin")),
        handoff_id,
    );
    let (target, _) = wait_for_fixed_confirmation(&application, cx, source);
    let probe_requests = server.request_count();
    assert!(probe_requests > 0);

    let mut surface = VisualTestContext::from_window(target, cx);
    surface.refresh().expect("controlled confirmation");
    let download = surface
        .debug_bounds("download-start-now")
        .expect("production Download action");
    surface.simulate_click(download.center(), Modifiers::default());
    let deadline = Instant::now() + Duration::from_secs(4);
    while target.update(cx, |_, _, _| {}).is_ok() {
        surface.run_until_parked();
        assert!(
            Instant::now() < deadline,
            "held acceptance closes confirmation"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let (state, task_id) = handoff_status(&application, cx, handoff_id);
    assert_eq!(state, BrowserHandoffState::Held);
    let task_id = task_id.expect("held task identity");
    application.read_with(cx, |application, _| {
        let task = application
            .engine
            .task(task_id)
            .expect("held task read")
            .expect("held task");
        assert_ne!(task.task.status(), DownloadStatus::Downloading);
    });
    cx.run_until_parked();
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(
        server.request_count(),
        probe_requests,
        "Desktop acceptance must not commit or transmit held work"
    );
    assert_eq!(
        handoff_status(&application, cx, handoff_id).0,
        BrowserHandoffState::Held
    );
    server.finish();
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn controlled_late_confirm_after_abort_is_rejected_and_window_remains_retry_safe(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (application, directory) =
        super::tests::download_application_for_window_test(cx, "controlled-late");
    let server = ProbeServer::new();
    let handoff_id = uuid::Uuid::new_v4();
    let (source, _) = prepare_controlled(
        &application,
        cx,
        controlled_request(&directory, server.url("late.bin")),
        handoff_id,
    );
    let (target, view) = wait_for_fixed_confirmation(&application, cx, source);
    assert!(matches!(
        engine_outcome(
            &application,
            cx,
            DownloadCommand::BrowserAbortHandoff { handoff_id },
            handoff_id,
        ),
        DownloadCommandOutcome::BrowserHandoff {
            state: BrowserHandoffState::Aborted,
            ..
        }
    ));

    let mut surface = VisualTestContext::from_window(target, cx);
    surface.refresh().expect("late controlled confirmation");
    let download = surface
        .debug_bounds("download-start-now")
        .expect("production Download action");
    surface.simulate_click(download.center(), Modifiers::default());
    surface.run_until_parked();
    assert!(target.update(cx, |_, _, _| {}).is_ok());
    assert!(!view.read_with(cx, |view, _| view.submitting));
    assert_eq!(
        handoff_status(&application, cx, handoff_id),
        (BrowserHandoffState::Aborted, None)
    );
    application.read_with(cx, |application, _| {
        assert!(application.engine.tasks().expect("task store").is_empty());
    });
    let _ = target.update(cx, |_, window, _| window.remove_window());
    cx.run_until_parked();
    server.finish();
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn controlled_probe_failure_aborts_pending_identity_without_creating_task(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (application, directory) =
        super::tests::download_application_for_window_test(cx, "controlled-probe-failure");
    let unavailable = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("reserve unavailable controlled endpoint");
    let unavailable_address = unavailable.local_addr().expect("unavailable address");
    drop(unavailable);
    let handoff_id = uuid::Uuid::new_v4();
    let (source, view) = prepare_controlled(
        &application,
        cx,
        controlled_request(
            &directory,
            format!("http://{unavailable_address}/unavailable.bin"),
        ),
        handoff_id,
    );
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        cx.run_until_parked();
        if view.read_with(cx, |view, _| !view.probing) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "controlled probe failure reaches a terminal view state"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        handoff_status(&application, cx, handoff_id),
        (BrowserHandoffState::Aborted, None)
    );
    application.read_with(cx, |application, _| {
        assert!(application.engine.tasks().expect("task store").is_empty());
    });
    assert!(source.update(cx, |_, _, _| {}).is_ok());
    let _ = source.update(cx, |_, window, _| window.remove_window());
    cx.run_until_parked();
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn controlled_fixed_presentation_failure_aborts_source_but_not_hidden_target(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (application, directory) =
        super::tests::download_application_for_window_test(cx, "controlled-fixed-failure");
    application.update(cx, |application, _| {
        application.new_download_native_handles_override = Some((701, 702));
    });
    let server = ProbeServer::new();
    let handoff_id = uuid::Uuid::new_v4();
    let (source, _) = prepare_controlled(
        &application,
        cx,
        controlled_request(&directory, server.url("fixed-failure.bin")),
        handoff_id,
    );
    let (target, command_id) = wait_for_pending_fixed_confirmation(&application, cx);
    assert!(target != source);
    assert_eq!(
        handoff_status(&application, cx, handoff_id),
        (BrowserHandoffState::Pending, None)
    );
    application.update(cx, |application, cx| {
        application.finish_new_download_handoff(command_id, false, "controlled_test_failure", cx);
    });
    cx.run_until_parked();
    assert!(target.update(cx, |_, _, _| {}).is_err());
    assert!(source.update(cx, |_, _, _| {}).is_ok());
    assert_eq!(
        handoff_status(&application, cx, handoff_id),
        (BrowserHandoffState::Aborted, None)
    );
    application.read_with(cx, |application, _| {
        assert!(application.engine.tasks().expect("task store").is_empty());
    });
    let _ = source.update(cx, |_, window, _| window.remove_window());
    cx.run_until_parked();
    server.finish();
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn controlled_titlebar_close_aborts_pending_without_waiting_for_lease(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (application, directory) =
        super::tests::download_application_for_window_test(cx, "controlled-titlebar");
    let server = ProbeServer::new();
    let handoff_id = uuid::Uuid::new_v4();
    let (source, _) = prepare_controlled(
        &application,
        cx,
        controlled_request(&directory, server.url("titlebar.bin")),
        handoff_id,
    );
    let (target, _) = wait_for_fixed_confirmation(&application, cx, source);
    let mut surface = VisualTestContext::from_window(target, cx);
    surface.refresh().expect("controlled titlebar");
    let close = surface
        .debug_bounds("download-window-close")
        .expect("production titlebar close");
    surface.simulate_click(close.center(), Modifiers::default());
    surface.run_until_parked();
    assert!(target.update(cx, |_, _, _| {}).is_err());
    assert_eq!(
        handoff_status(&application, cx, handoff_id),
        (BrowserHandoffState::Aborted, None)
    );
    application.read_with(cx, |application, _| {
        assert!(
            application
                .engine
                .tasks()
                .expect("no closed task")
                .is_empty()
        );
    });
    server.finish();
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn controlled_edited_and_reprobed_resource_cannot_replace_browser_original(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (application, directory) =
        super::tests::download_application_for_window_test(cx, "controlled-edited");
    let server = ProbeServer::new();
    let handoff_id = uuid::Uuid::new_v4();
    let (source, _) = prepare_controlled(
        &application,
        cx,
        controlled_request(&directory, server.url("original.bin")),
        handoff_id,
    );
    let (target, view) = wait_for_fixed_confirmation(&application, cx, source);
    target
        .update(cx, |_, window, cx| {
            view.update(cx, |view, cx| {
                view.url.update(cx, |input, cx| {
                    input.set_value(server.url("replacement.bin"), window, cx)
                });
                view.begin_probe(window, cx);
            });
        })
        .expect("edit and reprobe controlled source");
    let (target, view) = wait_for_fixed_confirmation(&application, cx, target);
    let mut surface = VisualTestContext::from_window(target, cx);
    surface.refresh().expect("reprobed controlled source");
    let download = surface
        .debug_bounds("download-start-now")
        .expect("production Download action");
    surface.simulate_click(download.center(), Modifiers::default());
    surface.run_until_parked();
    assert_eq!(
        handoff_status(&application, cx, handoff_id),
        (BrowserHandoffState::Aborted, None)
    );
    assert!(!view.read_with(cx, |view, _| view.submitting));
    application.read_with(cx, |application, _| {
        assert!(
            application
                .engine
                .tasks()
                .expect("no replacement task")
                .is_empty()
        );
    });
    let _ = target.update(cx, |_, window, _| window.remove_window());
    cx.run_until_parked();
    server.finish();
    let _ = std::fs::remove_dir_all(directory);
}

fn click_controlled_confirmation(
    cx: &mut TestAppContext,
    target: AnyWindowHandle,
    selector: &'static str,
) {
    let mut surface = VisualTestContext::from_window(target, cx);
    surface.refresh().expect("controlled confirmation");
    let action = surface.debug_bounds(selector).expect("confirmation action");
    surface.simulate_click(action.center(), Modifiers::default());
    surface.run_until_parked();
}

fn commit_controlled(
    application: &Entity<DownloadApplication>,
    cx: &TestAppContext,
    handoff_id: uuid::Uuid,
    task_id: DownloadTaskId,
) {
    assert!(matches!(
        engine_outcome(
            application,
            cx,
            DownloadCommand::BrowserCommitHandoff { handoff_id, task_id },
            handoff_id,
        ),
        DownloadCommandOutcome::BrowserHandoff {
            state: BrowserHandoffState::Committed,
            task_id: Some(committed),
            ..
        } if committed == task_id
    ));
}

#[gpui_kit::test]
fn controlled_progress_stashes_live_source_and_opens_once_only_after_commit(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (application, directory, endpoint) =
        super::tests::download_application_for_window_test_with_platform(cx, "controlled-progress");
    let server = ProbeServer::new();
    let existing = [8, 24].map(|limit| {
        let mut request = controlled_request(&directory, server.url(&format!("older-{limit}.bin")));
        request.connection_limit = Some(limit);
        let task_id = application.read_with(cx, |app, _| {
            app.engine
                .enqueue(request, RequestOrigin::Internal)
                .expect("seed existing request")
        });
        (task_id, limit)
    });
    let handoff_id = uuid::Uuid::new_v4();
    let (source, _) = prepare_controlled(
        &application,
        cx,
        controlled_request(&directory, server.url("progress.bin")),
        handoff_id,
    );
    let (target, _) = wait_for_fixed_confirmation(&application, cx, source);
    endpoint.set_connected_for_contract_test(true);
    application.update(cx, |app, _| {
        app.task_window_native_handle_override = Some(703)
    });
    server.hold_response.store(true, Ordering::Release);
    let probe_requests = server.request_count();
    click_controlled_confirmation(cx, target, "download-start-now");
    let (state, task_id) = handoff_status(&application, cx, handoff_id);
    assert_eq!(state, BrowserHandoffState::Held);
    let task_id = task_id.expect("confirmed task");
    assert_eq!(server.request_count(), probe_requests);
    assert!(application.read_with(cx, |app, _| app.task_windows.is_empty()));
    assert!(
        target.update(cx, |_, _, _| {}).is_ok(),
        "the source must live until native capture replies"
    );
    let (capture_id, key) =
        std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
            .find_map(|envelope| match envelope.message {
                nxtools_protocol::Message::PlatformCommand(PlatformCommand::StashWindowBounds(
                    command,
                )) => {
                    assert_eq!(command.window_handle, 702);
                    assert_eq!(envelope.correlation_id, Some(handoff_id));
                    Some((envelope.id, command.key))
                }
                _ => None,
            })
            .expect("native capture of the fixed confirmation");
    application.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: capture_id,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        );
    });
    cx.run_until_parked();
    assert!(target.update(cx, |_, _, _| {}).is_err());
    application.read_with(cx, |app, _| {
        assert!(app.task_windows.is_empty());
        assert!(app.task_window_activation_commands.borrow().is_empty());
        assert_eq!(
            app.engine
                .task(task_id)
                .expect("read task")
                .expect("held task")
                .task
                .request()
                .connection_limit,
            Some(16)
        );
        for (existing_task, limit) in existing {
            assert_eq!(
                app.engine
                    .task(existing_task)
                    .expect("read existing task")
                    .expect("existing task")
                    .task
                    .request()
                    .connection_limit,
                Some(limit)
            );
        }
        assert_eq!(
            app.default_queue()
                .expect("default queue")
                .max_concurrent_tasks,
            Some(3)
        );
    });

    commit_controlled(&application, cx, handoff_id, task_id);
    cx.run_until_parked();
    let window = application.read_with(cx, |app, _| {
        assert_eq!(app.task_window_activation_commands.borrow().len(), 1);
        app.task_windows
            .get(&task_id)
            .expect("progress on the same committed task")
            .window
    });
    let mut presentation =
        std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
            .filter(|envelope| envelope.correlation_id == Some(handoff_id))
            .map(|envelope| envelope.message);
    assert!(
        matches!(presentation.next(), Some(nxtools_protocol::Message::PlatformCommand(
        PlatformCommand::MoveWindowCenteredOverKey(command)))
        if command.window_handle == 703 && command.key == key)
    );
    assert!(
        matches!(presentation.next(), Some(nxtools_protocol::Message::PlatformCommand(
        PlatformCommand::SetWindowVisibility(command)))
        if command.window_handle == 703 && command.visible)
    );
    commit_controlled(&application, cx, handoff_id, task_id);
    application.update(cx, |app, cx| app.refresh(cx));
    assert_eq!(
        application.read_with(cx, |app, _| app.task_windows[&task_id].window),
        window
    );

    application.update(cx, |app, cx| {
        app.close_task_window(task_id, cx);
        app.refresh(cx);
    });
    cx.run_until_parked();
    assert!(window.update(cx, |_, _, _| {}).is_err());
    application.read_with(cx, |app, _| {
        assert!(!app.task_windows.contains_key(&task_id));
        assert_eq!(app.task_window_activation_commands.borrow().len(), 1);
        assert!(app.engine.task(task_id).expect("task read").is_some());
    });
    server.finish();
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn controlled_progress_skips_queue_only_disabled_delayed_and_aborted_work(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    for (name, selector) in [
        ("queue-only", "download-add-to-queue"),
        ("disabled", "download-start-now"),
        ("delayed", "download-start-now"),
        ("aborted", "download-start-now"),
    ] {
        let (application, directory, endpoint) =
            super::tests::download_application_for_window_test_with_platform(cx, name);
        let server = ProbeServer::new();
        let handoff_id = uuid::Uuid::new_v4();
        application.update(cx, |app, _| {
            app.task_window_native_handle_override = Some(703);
            app.show_progress_window = name != "disabled";
        });
        let (source, _) = prepare_controlled(
            &application,
            cx,
            controlled_request(&directory, server.url("suppressed.bin")),
            handoff_id,
        );
        let (target, view) = wait_for_fixed_confirmation(&application, cx, source);
        endpoint.set_connected_for_contract_test(name == "aborted");
        if name == "delayed" {
            let due = Local::now() + ChronoDuration::minutes(10);
            view.update(cx, |view, _| {
                view.committed_schedule = Some(DelayedStartSelection {
                    date: due.date_naive(),
                    hour: due.hour(),
                    minute: due.minute(),
                })
            });
        }
        server.hold_response.store(true, Ordering::Release);
        click_controlled_confirmation(cx, target, selector);
        let capture_id = if name == "aborted" {
            assert!(target.update(cx, |_, _, _| {}).is_ok());
            Some(
                std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
                    .find(|envelope| {
                        matches!(
                            envelope.message,
                            nxtools_protocol::Message::PlatformCommand(
                                PlatformCommand::StashWindowBounds(_)
                            )
                        )
                    })
                    .expect("capture requested before abort")
                    .id,
            )
        } else {
            assert!(target.update(cx, |_, _, _| {}).is_err());
            None
        };
        let (state, task_id) = handoff_status(&application, cx, handoff_id);
        assert_eq!(state, BrowserHandoffState::Held);
        let task_id = task_id.expect("held task");
        let delayed_due = application.read_with(cx, |app, _| {
            let task = app
                .engine
                .task(task_id)
                .expect("read task")
                .expect("held task");
            let due = task.task.request().scheduled_at_unix_ms;
            if let Some(due) = due {
                assert_eq!(
                    app.engine
                        .process_due_schedules_at(due)
                        .expect("Held schedule check"),
                    0
                );
            }
            due
        });
        assert_eq!(delayed_due.is_some(), name == "delayed");
        if name == "aborted" {
            assert!(matches!(
                engine_outcome(
                    &application,
                    cx,
                    DownloadCommand::BrowserAbortHandoff { handoff_id },
                    handoff_id
                ),
                DownloadCommandOutcome::BrowserHandoff {
                    state: BrowserHandoffState::Aborted,
                    ..
                }
            ));
        } else {
            commit_controlled(&application, cx, handoff_id, task_id);
        }
        cx.run_until_parked();
        if let Some(capture_id) = capture_id {
            assert!(target.update(cx, |_, _, _| {}).is_err());
            application.update(cx, |app, cx| {
                app.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: capture_id,
                        result: PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: None,
                        },
                    },
                    cx,
                );
                assert!(app.pending_controlled_progress.is_empty());
            });
        }
        application.read_with(cx, |app, _| {
            assert!(app.task_windows.is_empty(), "{name}");
            assert!(
                app.task_window_activation_commands.borrow().is_empty(),
                "{name}"
            );
        });
        if let Some(due) = delayed_due {
            application.read_with(cx, |app, _| {
                assert_eq!(
                    app.engine
                        .process_due_schedules_at(due)
                        .expect("committed delayed start"),
                    1
                );
            });
            cx.run_until_parked();
            application.update(cx, |app, cx| {
                assert!(app.task_windows.contains_key(&task_id));
                assert!(
                    app.task_window_activation_commands.borrow().is_empty(),
                    "delayed progress must stay passive"
                );
                app.close_task_window(task_id, cx);
            });
        }
        if name == "queue-only" {
            application.update(cx, |app, cx| {
                app.batch_action(vec![task_id], DownloadBatchAction::Resume, cx)
            });
            let deadline = Instant::now() + Duration::from_secs(4);
            while application.read_with(cx, |app, _| app.batch_busy()) {
                cx.run_until_parked();
                assert!(Instant::now() < deadline, "explicit list start completes");
                std::thread::sleep(Duration::from_millis(10));
            }
            let first_window =
                application.read_with(cx, |app, _| app.task_windows[&task_id].window);
            application.update(cx, |app, cx| app.close_task_window(task_id, cx));
            assert!(first_window.update(cx, |_, _, _| {}).is_err());
            application
                .update(cx, |app, cx| app.retry(task_id, cx))
                .expect("explicit retry on admitted work");
            let retry_window =
                application.read_with(cx, |app, _| app.task_windows[&task_id].window);
            assert!(retry_window.update(cx, |_, _, _| {}).is_ok());
            application.update(cx, |app, cx| app.close_task_window(task_id, cx));
        }
        server.finish();
        let _ = std::fs::remove_dir_all(directory);
    }
}
