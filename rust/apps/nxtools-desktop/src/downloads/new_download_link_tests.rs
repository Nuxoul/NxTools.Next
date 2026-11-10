use super::*;
use crate::clipboard::ClipboardApplication;
use crate::clipboard_worker::ClipboardCaptureWorkerHandle;
use crate::tools::link_detector::{DetectedLink, DetectedLinkKind};
use gpui_kit::{TestAppContext, VisualTestContext};

struct LinkIntakeFixture {
    clipboard: Entity<ClipboardApplication>,
    downloads: Entity<DownloadApplication>,
    endpoint: nxtools_ipc::PlatformServerEndpoint,
    worker: ClipboardCaptureWorkerHandle,
    listener: std::net::TcpListener,
    _temporary: tempfile::TempDir,
}

impl LinkIntakeFixture {
    fn new(cx: &mut TestAppContext) -> Self {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        cx.executor().allow_parking();
        let temporary = tempfile::tempdir().expect("link intake fixture");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("HTTP observation socket");
        listener
            .set_nonblocking(true)
            .expect("bounded HTTP observation");
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
                DownloadApplication::new(
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
            (clipboard, downloads)
        });
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("sentinel".into()));
        Self {
            clipboard,
            downloads,
            endpoint,
            worker,
            listener,
            _temporary: temporary,
        }
    }

    fn target(&self, path: &str) -> String {
        format!(
            "http://{}/{path}",
            self.listener.local_addr().expect("address")
        )
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

    fn activate(&self, cx: &mut TestAppContext, panel: AnyWindowHandle, key: Option<&str>) {
        let mut surface = VisualTestContext::from_window(panel, cx);
        surface.refresh().expect("production panel rendering");
        let row = surface.debug_bounds("link-panel-row-0").expect("row");
        let name = surface
            .debug_bounds("link-panel-name-0")
            .expect("name lane");
        let button = surface
            .debug_bounds("link-panel-download-0")
            .expect("named production 使用 NxTools 下载 button");
        let open = surface
            .debug_bounds("link-panel-open-0")
            .expect("independent open");
        assert!(name.size.width >= px(40.0), "name lane remains readable");
        assert!(button.left() >= open.right() && button.right() <= row.right());
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

    fn assert_idle(&self, cx: &TestAppContext, expected: &str) {
        let (_, view) = self.intake(cx);
        self.downloads.read_with(cx, |app, _| {
            assert!(app.engine.tasks().expect("original task store").is_empty());
        });
        view.read_with(cx, |view, cx| {
            assert_eq!(view.url.read(cx).value().as_ref(), expected);
            assert!(!view.probing && !view.submitting);
            assert!(view.probe.is_none());
            assert!(view.external_request.is_none() && view.external_blob.is_none());
        });
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("sentinel")
        );
        let commands =
            std::iter::from_fn(|| self.endpoint.try_receive_command_for_contract_test().ok())
                .filter_map(|envelope| match envelope.message {
                    nxtools_protocol::Message::PlatformCommand(command) => Some(command),
                    _ => None,
                })
                .collect::<Vec<_>>();
        assert!(
            commands.iter().all(|command| matches!(
                command,
                PlatformCommand::SetWindowTopmost(_) | PlatformCommand::ActivateWindow(_)
            )),
            "intake queues only original window commands, never clipboard or Shell work"
        );
        assert!(
            self.listener.accept().is_err(),
            "opening intake performs no HTTP request"
        );
    }

    fn finish(self, cx: &mut TestAppContext) {
        for handle in cx.windows() {
            handle
                .update(cx, |_, window, _| window.remove_window())
                .expect("close fixture window");
        }
        cx.run_until_parked();
        self.worker.shutdown().expect("flush fixture history");
    }
}

#[gpui_kit::test]
fn ref04_production_download_action_only_opens_original_url_intake(cx: &mut TestAppContext) {
    let fixture = LinkIntakeFixture::new(cx);
    let target = fixture.target("selected.zip");
    let panel = fixture.panel(cx, &target, DetectedLinkKind::Web);
    fixture.activate(cx, panel, None);
    assert_eq!(cx.windows().len(), 1, "one original intake window");
    fixture.assert_idle(cx, &target);
    let (handle, view) = fixture.intake(cx);
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.refresh().expect("original URL input");
    assert!(surface.update(|window, cx| {
        view.read(cx)
            .url
            .read(cx)
            .focus_handle(cx)
            .is_focused(window)
    }));
    fixture.finish(cx);
}

#[gpui_kit::test]
fn ref04_keyboard_enter_space_reuse_window_and_preserve_populated_draft(cx: &mut TestAppContext) {
    let fixture = LinkIntakeFixture::new(cx);
    let first = fixture.target("first.zip");
    let panel = fixture.panel(cx, &first, DetectedLinkKind::Web);
    fixture.activate(cx, panel, Some("enter"));
    let (original, view) = fixture.intake(cx);
    let input = view.read_with(cx, |view, _| view.url.entity_id());
    let panel = fixture.panel(cx, &fixture.target("second.zip"), DetectedLinkKind::Web);
    fixture.activate(cx, panel, Some("space"));
    let (reused, reused_view) = fixture.intake(cx);
    assert_eq!(reused, original);
    assert_eq!(reused_view.entity_id(), view.entity_id());
    assert_eq!(
        reused_view.read_with(cx, |view, _| view.url.entity_id()),
        input
    );
    assert_eq!(cx.windows(), vec![original]);
    fixture.assert_idle(cx, &first);
    fixture.finish(cx);
}

#[gpui_kit::test]
fn ref04_explicit_url_fills_pristine_window_and_retires_late_clipboard(cx: &mut TestAppContext) {
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
    let panel = fixture.panel(cx, &target, DetectedLinkKind::Web);
    fixture.activate(cx, panel, None);
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
    fixture.assert_idle(cx, &target);
    fixture.finish(cx);
}

#[gpui_kit::test]
fn ref04_user_edit_then_clear_is_not_an_empty_replaceable_draft(cx: &mut TestAppContext) {
    let fixture = LinkIntakeFixture::new(cx);
    let target = fixture.target("first.zip");
    let panel = fixture.panel(cx, &target, DetectedLinkKind::Web);
    fixture.activate(cx, panel, None);
    let (original, _) = fixture.intake(cx);
    let mut surface = VisualTestContext::from_window(original, cx);
    surface.refresh().expect("original input");
    let input = surface
        .debug_bounds("download-new-url-field-input")
        .expect("URL input");
    surface.simulate_click(input.center(), Default::default());
    surface.simulate_keystrokes("ctrl-a u s e r ctrl-a backspace");
    let panel = fixture.panel(cx, &fixture.target("second.zip"), DetectedLinkKind::Web);
    fixture.activate(cx, panel, None);
    assert_eq!(fixture.intake(cx).0, original);
    fixture.assert_idle(cx, "");
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
    assert!(fixture.listener.accept().is_err());
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
