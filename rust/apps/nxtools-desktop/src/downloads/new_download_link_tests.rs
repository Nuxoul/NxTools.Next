use super::*;
use crate::clipboard::ClipboardApplication;
use crate::clipboard_worker::ClipboardCaptureWorkerHandle;
use crate::tools::link_detector::{DetectedLink, DetectedLinkKind};
use gpui_kit::{TestAppContext, VisualTestContext};
use std::io::{Read as _, Write as _};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

struct LinkIntakeFixture {
    clipboard: Entity<ClipboardApplication>,
    downloads: Entity<DownloadApplication>,
    endpoint: nxtools_ipc::PlatformServerEndpoint,
    address: std::net::SocketAddr,
    server_stop: Arc<AtomicBool>,
    request_count: Arc<AtomicUsize>,
    server: Option<std::thread::JoinHandle<()>>,
    worker: ClipboardCaptureWorkerHandle,
    _temporary: tempfile::TempDir,
}

impl LinkIntakeFixture {
    fn new(cx: &mut TestAppContext) -> Self {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        cx.executor().allow_parking();
        let temporary = tempfile::tempdir().expect("link intake fixture");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("HTTP observation socket");
        let address = listener.local_addr().expect("HTTP fixture address");
        listener
            .set_nonblocking(true)
            .expect("bounded HTTP observation");
        let server_stop = Arc::new(AtomicBool::new(false));
        let request_count = Arc::new(AtomicUsize::new(0));
        let worker_stop = Arc::clone(&server_stop);
        let worker_count = Arc::clone(&request_count);
        let server = std::thread::spawn(move || {
            while !worker_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_nonblocking(false)
                            .expect("accepted blocking fixture");
                        stream
                            .set_read_timeout(Some(Duration::from_secs(1)))
                            .expect("bounded fixture read");
                        let mut request = [0_u8; 4096];
                        let _ = stream.read(&mut request);
                        worker_count.fetch_add(1, Ordering::AcqRel);
                        if !worker_stop.load(Ordering::Acquire) {
                            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\nfixture!");
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });
        let worker = ClipboardCaptureWorkerHandle::spawn(
            temporary.path().join("history.db"),
            temporary.path().join("assets"),
            nxtools_clipboard::ClipboardRetentionPolicy {
                max_items: 1000,
                retention_days: 30,
            },
            1000,
        )
        .expect("original clipboard worker");
        let (platform, endpoint) = nxtools_ipc::platform_bridge(64);
        endpoint.set_connected_for_contract_test(true);
        let platform = Rc::new(platform);
        let (clipboard, downloads) = cx.update(|cx| {
            let notifications = cx.new(|_| NotificationCenter::default());
            let downloads = cx.new(|cx| {
                super::ordinary_presentation_fixture::confirmed_application(
                    DownloadEngine::open_in_memory().expect("original download engine"),
                    platform.clone(),
                    notifications.clone(),
                    DownloadRuntimeContext {
                        fallback_default_directory: temporary.path().into(),
                        config: AppConfig::default(),
                        config_path: temporary.path().join("config.json"),
                        log_directory: temporary.path().into(),
                        database_path: temporary.path().join("downloads.db"),
                        ffmpeg_install_directory: temporary.path().join("ffmpeg"),
                    },
                    cx,
                )
            });
            let clipboard = cx.new(|cx| {
                ClipboardApplication::new(
                    worker.clone(),
                    platform,
                    AppConfig::default(),
                    temporary.path().join("config.json"),
                    notifications,
                    cx,
                )
            });
            clipboard.update(cx, |app, _| app.bind_downloads(downloads.downgrade()));
            downloads.update(cx, |app, _| {
                app.new_download_native_handles_override = Some((701, 702));
            });
            (clipboard, downloads)
        });
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("sentinel".into()));
        Self {
            clipboard,
            downloads,
            endpoint,
            address,
            server_stop,
            request_count,
            server: Some(server),
            worker,
            _temporary: temporary,
        }
    }

    fn target(&self, path: &str) -> String {
        format!("http://{}/{path}", self.address)
    }

    fn panel(
        &self,
        cx: &mut TestAppContext,
        target: &str,
        kind: DetectedLinkKind,
    ) -> AnyWindowHandle {
        self.clipboard.update(cx, |app, cx| {
            app.show_link_panel(
                vec![DetectedLink {
                    display: "完整下载目标名称".repeat(24),
                    copy_value: target.into(),
                    kind,
                }],
                cx,
            )
        });
        let panel = *cx.windows().last().expect("production panel");
        while self
            .endpoint
            .try_receive_command_for_contract_test()
            .is_ok()
        {}
        panel
    }

    fn activate(&self, cx: &mut TestAppContext, panel: AnyWindowHandle, key: Option<&'static str>) {
        let mut surface = VisualTestContext::from_window(panel, cx);
        surface.refresh().expect("production panel rendering");
        let row = surface.debug_bounds("link-panel-row-0").expect("row");
        let name = surface
            .debug_bounds("link-panel-name-0")
            .expect("name lane");
        let button = surface
            .debug_bounds("link-panel-download-0")
            .expect("named production 使用 NxTools 下载 button");
        let copy = surface
            .debug_bounds("link-panel-copy-0")
            .expect("independent copy");
        assert!(surface.debug_bounds("link-panel-open-0").is_none());
        assert!(name.size.width >= px(40.0), "name lane remains readable");
        assert!(button.left() >= copy.right() && button.right() <= row.right());
        assert!(button.top() >= row.top() && button.bottom() <= row.bottom());
        assert_eq!(row.size.height, px(28.0));
        let semantic = surface
            .update(|window, _| gpui_kit::base::test_support::snapshots(window))
            .into_iter()
            .find(|node| node.label() == Some("使用 NxTools 下载"))
            .expect("official named Button snapshot");
        assert_eq!(semantic.bounds(), button);
        if let Some(key) = key {
            surface.update(|window, cx| {
                window.activate_window();
                window.focus_next(cx);
            });
            let mut focused = false;
            for _ in 0..16 {
                surface.simulate_keystrokes("tab");
                surface.refresh().expect("actual Tab focus");
                focused = surface
                    .update(|window, _| gpui_kit::base::test_support::snapshots(window))
                    .iter()
                    .any(|node| {
                        node.label() == Some("使用 NxTools 下载") && node.focused() == Some(true)
                    });
                if focused {
                    break;
                }
            }
            assert!(
                focused,
                "named download Button is reachable in actual Tab order"
            );
            surface.simulate_keystrokes(key);
            surface.simulate_event(gpui_kit::KeyUpEvent {
                keystroke: gpui_kit::Keystroke::parse(key).expect("activation key"),
            });
        } else {
            surface.simulate_click(button.center(), Default::default());
        }
        cx.run_until_parked();
        assert!(
            panel.update(cx, |_, _, _| {}).is_err(),
            "source panel closes locally"
        );
    }

    fn intake(&self, cx: &TestAppContext) -> (AnyWindowHandle, Entity<NewDownloadView>) {
        self.downloads.read_with(cx, |app, _| {
            (
                app.new_download_window.expect("original intake"),
                app.new_download_view.clone().expect("original input owner"),
            )
        })
    }

    fn request_count(&self) -> usize {
        self.request_count.load(Ordering::Acquire)
    }

    fn wait_for_pending_confirmation(
        &self,
        cx: &mut TestAppContext,
        expected: &str,
        requests_before: usize,
    ) -> (AnyWindowHandle, Entity<NewDownloadView>) {
        let deadline = Instant::now() + Duration::from_secs(4);
        loop {
            cx.run_until_parked();
            let intake = self.downloads.read_with(cx, |app, _| {
                app.new_download_window.zip(app.new_download_view.clone())
            });
            if let Some((window, view)) = intake
                && view.read_with(cx, |view, cx| {
                    !view.probing
                        && view.probe.is_some()
                        && view.handoff_pending
                        && view.url.read(cx).value().as_ref() == expected
                })
                && self.request_count() > requests_before
            {
                self.downloads.read_with(cx, |app, _| {
                    assert!(app.engine.tasks().expect("original task store").is_empty());
                });
                return (window, view);
            }
            assert!(
                Instant::now() < deadline,
                "automatic probe missing: requests={}, state={:?}",
                self.request_count(),
                self.downloads
                    .read_with(cx, |app, cx| app.new_download_view.as_ref().map(|view| {
                        let view = view.read(cx);
                        (
                            view.probing,
                            view.probe.is_some(),
                            view.handoff_staged,
                            view.probe_generation,
                        )
                    }))
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn present_confirmation(&self, cx: &mut TestAppContext) {
        let command_id = self.downloads.read_with(cx, |app, _| {
            app.pending_new_download_handoff
                .as_ref()
                .expect("pending native presentation")
                .command_id
        });
        self.downloads.update(cx, |app, cx| {
            app.finish_new_download_handoff(command_id, true, "test_native_presented", cx);
        });
        cx.run_until_parked();
    }

    fn wait_for_confirmation(
        &self,
        cx: &mut TestAppContext,
        source: AnyWindowHandle,
        expected: &str,
        requests_before: usize,
    ) -> (AnyWindowHandle, Entity<NewDownloadView>) {
        let deadline = Instant::now() + Duration::from_secs(4);
        loop {
            cx.run_until_parked();
            let intake = self.downloads.read_with(cx, |app, _| {
                app.new_download_window.zip(app.new_download_view.clone())
            });
            if let Some((window, view)) = intake
                && window != source
                && view.read_with(cx, |view, cx| {
                    !view.probing
                        && view.probe.is_some()
                        && view.url.read(cx).value().as_ref() == expected
                })
            {
                assert!(self.request_count() > requests_before);
                assert!(
                    source.update(cx, |_, _, _| {}).is_err(),
                    "compact source closes after fixed confirmation is ready"
                );
                self.downloads.read_with(cx, |app, _| {
                    assert!(app.engine.tasks().expect("original task store").is_empty());
                });
                let mut surface = VisualTestContext::from_window(window, cx);
                surface.refresh().expect("fixed confirmation rendering");
                assert!(
                    surface
                        .debug_bounds("download-new-confirmed-form")
                        .is_some()
                );
                assert!(surface.debug_bounds("download-new-footer").is_some());
                return (window, view);
            }
            assert!(
                Instant::now() < deadline,
                "automatic probe must reach fixed confirmation"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn assert_preserved(
        &self,
        cx: &TestAppContext,
        expected: &str,
        requests_before: usize,
        probing: bool,
    ) {
        let (_, view) = self.intake(cx);
        self.downloads.read_with(cx, |app, _| {
            assert!(app.engine.tasks().expect("original task store").is_empty());
        });
        view.read_with(cx, |view, cx| {
            assert_eq!(view.url.read(cx).value().as_ref(), expected);
            assert_eq!(view.probing, probing);
            assert!(!view.submitting);
        });
        assert_eq!(self.request_count(), requests_before);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("sentinel")
        );
    }

    fn finish(mut self, cx: &mut TestAppContext) {
        for handle in cx.windows() {
            if cx.windows().contains(&handle) {
                handle
                    .update(cx, |_, window, _| window.remove_window())
                    .expect("close live fixture window");
            }
        }
        cx.run_until_parked();
        assert!(cx.windows().is_empty(), "all fixture window owners retired");
        self.server_stop.store(true, Ordering::Release);
        self.server
            .take()
            .expect("fixture server")
            .join()
            .expect("bounded fixture server");
        self.worker.shutdown().expect("flush fixture history");
    }
}

#[gpui_kit::test]
fn external_link_action_auto_probes_and_opens_fixed_confirmation(cx: &mut TestAppContext) {
    let fixture = LinkIntakeFixture::new(cx);
    let target = fixture.target("selected.zip");
    let requests_before = fixture.request_count();
    let panel = fixture.panel(cx, &target, DetectedLinkKind::Web);
    fixture.activate(cx, panel, None);
    assert_eq!(
        cx.windows().len(),
        2,
        "compact intake and hidden confirmation"
    );
    let (source, view) = fixture.wait_for_pending_confirmation(cx, &target, requests_before);
    let mut surface = VisualTestContext::from_window(source, cx);
    surface.refresh().expect("original URL input");
    assert!(surface.update(|window, cx| {
        view.read(cx)
            .url
            .read(cx)
            .focus_handle(cx)
            .is_focused(window)
    }));
    fixture.present_confirmation(cx);
    let _ = fixture.wait_for_confirmation(cx, source, &target, requests_before);
    fixture.finish(cx);
}

#[gpui_kit::test]
fn external_link_keyboard_reuse_preserves_pending_confirmation(cx: &mut TestAppContext) {
    let fixture = LinkIntakeFixture::new(cx);
    let first = fixture.target("first.zip");
    let requests_before = fixture.request_count();
    let panel = fixture.panel(cx, &first, DetectedLinkKind::Web);
    fixture.activate(cx, panel, Some("enter"));
    let (original, view) = fixture.wait_for_pending_confirmation(cx, &first, requests_before);
    let input = view.read_with(cx, |view, _| view.url.entity_id());
    let started_requests = fixture.request_count();
    let panel = fixture.panel(cx, &fixture.target("second.zip"), DetectedLinkKind::Web);
    fixture.activate(cx, panel, Some("space"));
    let (reused, reused_view) = fixture.intake(cx);
    assert_eq!(reused, original);
    assert_eq!(reused_view.entity_id(), view.entity_id());
    assert_eq!(
        reused_view.read_with(cx, |view, _| view.url.entity_id()),
        input
    );
    assert!(cx.windows().contains(&original));
    assert_eq!(
        cx.windows().len(),
        2,
        "original source and pending target stay keyed"
    );
    fixture.assert_preserved(cx, &first, started_requests, false);
    assert!(view.read_with(cx, |view, _| view.handoff_pending));
    fixture.finish(cx);
}

#[gpui_kit::test]
fn external_link_reuse_preserves_populated_draft_without_new_probe(cx: &mut TestAppContext) {
    let fixture = LinkIntakeFixture::new(cx);
    fixture
        .downloads
        .update(cx, |app, cx| app.open_new_download_window(cx));
    cx.run_until_parked();
    let (original, view) = fixture.intake(cx);
    let existing = fixture.target("existing.zip");
    original
        .update(cx, |_, window, cx| {
            view.update(cx, |view, cx| {
                view.url.update(cx, |input, cx| {
                    input.replace_all(existing.clone(), window, cx)
                });
            });
        })
        .expect("populate original draft");
    let requests_before = fixture.request_count();
    let panel = fixture.panel(
        cx,
        &fixture.target("replacement.zip"),
        DetectedLinkKind::Web,
    );
    fixture.activate(cx, panel, None);
    assert_eq!(fixture.intake(cx).0, original);
    fixture.assert_preserved(cx, &existing, requests_before, false);
    fixture.finish(cx);
}

#[gpui_kit::test]
fn external_link_pristine_reuse_auto_probes_and_retires_late_clipboard(cx: &mut TestAppContext) {
    let fixture = LinkIntakeFixture::new(cx);
    fixture
        .downloads
        .update(cx, |app, cx| app.open_new_download_window(cx));
    cx.run_until_parked();
    let (original, view) = fixture.intake(cx);
    let input = view.read_with(cx, |view, _| view.url.entity_id());
    let read = std::iter::from_fn(|| {
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .ok()
    })
    .find(|envelope| {
        matches!(
            &envelope.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::ReadClipboardText)
        )
    })
    .expect("original one-shot clipboard request")
    .id;
    let target = fixture.target("explicit.zip");
    let requests_before = fixture.request_count();
    let panel = fixture.panel(cx, &target, DetectedLinkKind::Web);
    fixture.activate(cx, panel, None);
    let (source, _) = fixture.wait_for_pending_confirmation(cx, &target, requests_before);
    assert_eq!(source, original);
    fixture.downloads.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: read,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: Some("https://late.invalid/wrong.zip".into()),
                },
            },
            cx,
        )
    });
    cx.run_until_parked();
    assert_eq!(fixture.intake(cx).0, original);
    assert_eq!(view.read_with(cx, |view, _| view.url.entity_id()), input);
    assert_eq!(
        view.read_with(cx, |view, cx| view.url.read(cx).value().to_string()),
        target
    );
    fixture.present_confirmation(cx);
    let _ = fixture.wait_for_confirmation(cx, original, &target, requests_before);
    fixture.finish(cx);
}

#[gpui_kit::test]
fn external_link_reuse_preserves_edited_then_cleared_manual_draft(cx: &mut TestAppContext) {
    let fixture = LinkIntakeFixture::new(cx);
    fixture
        .downloads
        .update(cx, |app, cx| app.open_new_download_window(cx));
    cx.run_until_parked();
    let (original, view) = fixture.intake(cx);
    let started_requests = fixture.request_count();
    let mut surface = VisualTestContext::from_window(original, cx);
    surface.refresh().expect("original input");
    let input = surface
        .debug_bounds("download-new-url-field-input")
        .expect("URL input");
    surface.simulate_click(input.center(), Default::default());
    surface.simulate_keystrokes("ctrl-a u s e r ctrl-a backspace");
    assert!(!view.read_with(cx, |view, cx| view.clipboard_prefill_eligible(cx)));
    let panel = fixture.panel(cx, &fixture.target("second.zip"), DetectedLinkKind::Web);
    fixture.activate(cx, panel, None);
    assert_eq!(fixture.intake(cx).0, original);
    fixture.assert_preserved(cx, "", started_requests, false);
    fixture.finish(cx);
}

#[gpui_kit::test]
fn ref04_replaced_panel_cannot_change_or_open_download_intake(cx: &mut TestAppContext) {
    let fixture = LinkIntakeFixture::new(cx);
    let panel = fixture.panel(cx, &fixture.target("retired.zip"), DetectedLinkKind::Web);
    let clipboard = fixture.clipboard.clone();
    let retired_id = panel.window_id();
    let subscription = cx.update(|cx| {
        cx.on_window_closed(move |cx, closed| {
            if closed == retired_id {
                clipboard.update(cx, |app, cx| {
                    app.show_link_panel(
                        vec![DetectedLink {
                            display: "current".into(),
                            copy_value: "https://current.invalid/current.zip".into(),
                            kind: DetectedLinkKind::Web,
                        }],
                        cx,
                    )
                });
            }
        })
    });
    fixture.activate(cx, panel, None);
    assert_eq!(
        cx.windows().len(),
        1,
        "only the replacement link panel remains"
    );
    fixture.downloads.read_with(cx, |app, _| {
        assert!(app.new_download_window.is_none());
        assert!(app.engine.tasks().expect("original task store").is_empty());
    });
    assert_eq!(fixture.request_count(), 0);
    drop(subscription);
    fixture.finish(cx);
}

#[gpui_kit::test]
fn ref04_local_and_unsupported_targets_have_no_download_action(cx: &mut TestAppContext) {
    let fixture = LinkIntakeFixture::new(cx);
    for (target, kind) in [
        (r"C:\synthetic\file.zip", DetectedLinkKind::LocalFile),
        (r"C:\synthetic", DetectedLinkKind::LocalDirectory),
        ("file:///C:/synthetic/file.zip", DetectedLinkKind::Web),
        ("ftp://synthetic.invalid/file.zip", DetectedLinkKind::Web),
    ] {
        let panel = fixture.panel(cx, target, kind);
        let mut surface = VisualTestContext::from_window(panel, cx);
        surface.refresh().expect("production local/unsupported row");
        assert!(surface.debug_bounds("link-panel-download-0").is_none());
        fixture.downloads.update(cx, |app, cx| {
            app.open_link_download(target.into(), uuid::Uuid::new_v4(), Instant::now(), cx)
        });
        cx.run_until_parked();
        fixture
            .downloads
            .read_with(cx, |app, _| assert!(app.new_download_window.is_none()));
        panel
            .update(cx, |_, window, _| window.remove_window())
            .expect("close source");
        cx.run_until_parked();
    }
    fixture.finish(cx);
}
