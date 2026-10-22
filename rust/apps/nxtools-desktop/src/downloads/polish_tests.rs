//! Production-window geometry gates for the Downloads presentation pass.
use super::*;
use gpui_kit::{AppContext, Modifiers, TestAppContext, VisualTestContext};

fn window_fixture(
    cx: &mut TestAppContext,
) -> (
    Entity<DownloadApplication>,
    PathBuf,
    nxtools_ipc::PlatformServerEndpoint,
) {
    let directory = std::env::temp_dir().join(format!("download-polish-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).expect("create test directory");
    let engine = DownloadEngine::open_in_memory().expect("open in-memory download engine");
    let (platform, endpoint) = nxtools_ipc::platform_bridge(16);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let application = cx.update(|cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                engine,
                Rc::new(platform),
                notifications,
                DownloadRuntimeContext {
                    fallback_default_directory: directory.clone(),
                    config: AppConfig::default(),
                    config_path: directory.join("config.json"),
                    log_directory: directory.clone(),
                    database_path: directory.join("downloads.sqlite3"),
                    ffmpeg_install_directory: directory.join("tools").join("ffmpeg"),
                },
                cx,
            )
        })
    });
    (application, directory, endpoint)
}

#[gpui_kit::test]
fn polished_new_download_preserves_compact_footer_and_confirmed_budget(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, _endpoint) = window_fixture(cx);
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.open_new_download_window(cx);
        });
    });
    cx.run_until_parked();
    let handle = cx
        .read(|cx| application.read(cx).new_download_window)
        .expect("production new-download window");
    assert_eq!(
        handle
            .update(cx, |_, window, _| window.bounds().size)
            .expect("read native target"),
        new_download_window_size(false),
    );
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.refresh().expect("render compact form");
    let root = surface
        .debug_bounds("download-new-root")
        .expect("actual root");
    let title = surface
        .debug_bounds("download-new-titlebar")
        .expect("titlebar");
    let guidance = surface
        .debug_bounds("download-new-probe-guidance")
        .expect("guidance");
    let footer = surface.debug_bounds("download-new-footer").expect("footer");
    let cancel = surface
        .debug_bounds("download-new-cancel-action")
        .expect("cancel hit region");
    assert_eq!(root.size.height, px(176.0));
    let input = surface
        .debug_bounds("download-new-url-field-input")
        .expect("compact input");
    let paste = surface
        .debug_bounds("download-new-url-paste")
        .expect("compact paste");
    assert_eq!(input.size.height, px(CONTROL_HEIGHT_INPUT));
    assert_eq!(paste.size.height, input.size.height);
    assert_eq!(paste.top(), input.top());
    assert!(paste.bottom() <= guidance.top());
    assert!(title.bottom() < guidance.top());
    assert!(guidance.bottom() <= footer.top());
    assert!(cancel.top() >= footer.top() && cancel.bottom() <= footer.bottom());
    assert_eq!(footer.bottom(), root.bottom());
    assert_eq!(
        new_download_window_size(true).height,
        px(NEW_DOWNLOAD_CONFIRMED_HEIGHT)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn polished_settings_navigation_uses_actual_selected_hit_bounds(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, _endpoint) = window_fixture(cx);
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.open_settings_window(cx);
        });
    });
    cx.run_until_parked();
    let handle = cx
        .read(|cx| application.read(cx).settings_window)
        .expect("production settings window");
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(984.0), px(650.0)));
    surface
        .refresh()
        .expect("render production settings navigation");
    let item = official_download_navigation_bounds(&mut surface, DownloadSettingsPage::General);
    for page in [
        DownloadSettingsPage::General,
        DownloadSettingsPage::Request,
        DownloadSettingsPage::Transfer,
        DownloadSettingsPage::Window,
        DownloadSettingsPage::Diagnostics,
    ] {
        let row = official_download_navigation_bounds(&mut surface, page);
        assert_eq!(row.left(), item.left());
        assert_eq!(row.size.width, item.size.width);
        assert!(row.size.height >= px(28.0));
    }
    select_official_download_page(&mut surface, DownloadSettingsPage::Request);
    assert!(
        surface
            .update(|window, _| gpui_kit::base::test_support::snapshots(window))
            .iter()
            .any(|snapshot| snapshot.role() == Some(gpui_kit::Role::Group)
                && snapshot.label() == Some("请求")),
        "official page mounts the same request draft"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn polished_schedule_weekdays_fit_actual_dialog_and_footer(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, _endpoint) = window_fixture(cx);
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.open_schedule_window(cx);
        });
    });
    cx.run_until_parked();
    let handle = cx
        .read(|cx| application.read(cx).schedule_window)
        .expect("production schedule dialog");
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.refresh().expect("render daily schedule");
    let body = surface
        .debug_bounds("download-schedule-dialog-body")
        .expect("bounded body");
    let footer = surface
        .debug_bounds("download-schedule-dialog-footer")
        .expect("fixed footer");
    let cancel = surface
        .debug_bounds("download-schedule-secondary-cancel-action")
        .expect("cancel");
    let apply = surface
        .debug_bounds("download-schedule-primary-apply-action")
        .expect("apply");
    let first_day = surface
        .debug_bounds("download-schedule-day-0")
        .expect("first weekday hit target");
    let last_day = surface
        .debug_bounds("download-schedule-day-6")
        .expect("last weekday hit target");
    assert_eq!(body.bottom(), footer.top());
    assert_eq!(
        first_day.size.height,
        px(24.0),
        "released Small weekday Toggle"
    );
    assert!(first_day.size.width > px(0.0));
    assert_eq!(last_day.size, first_day.size);
    assert!(first_day.left() >= body.left() && last_day.right() <= body.right());
    assert!(last_day.bottom() <= body.bottom());
    assert!(cancel.top() >= footer.top() && apply.bottom() <= footer.bottom());
    assert!(cancel.right() <= apply.left());
    surface.simulate_click(first_day.center(), Modifiers::default());
    surface.run_until_parked();
    assert!(cx.read(|cx| application.read(cx).schedule_window.is_some()));
    surface
        .refresh()
        .expect("render after local weekday selection");
    let selected_day = surface
        .debug_bounds("download-schedule-day-0")
        .expect("selected weekday hit target");
    assert_eq!(selected_day, first_day);
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn polished_schedule_keyboard_changes_the_actual_weekday_state(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, _endpoint) = window_fixture(cx);
    let slot = Rc::new(std::cell::RefCell::new(None));
    let rendered_slot = slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(520.0), px(360.0)));
        let view = cx.new(|cx| DownloadScheduleView::new(application, None, window, cx));
        *rendered_slot.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    surface
        .refresh()
        .expect("render actual schedule keyboard fixture");
    surface.update(|window, cx| {
        window.focus_next(cx);
        window.focus_next(cx);
    });
    surface
        .refresh()
        .expect("focus first weekday after caption close");
    let view = slot.borrow().clone().expect("actual schedule entity");
    let day = DownloadWeekday::ALL[0];
    assert!(view.read_with(surface, |view, _| view.days.contains(&day)));
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("weekday Space"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("weekday release"),
    });
    assert!(
        !view.read_with(surface, |view, _| view.days.contains(&day)),
        "Space must change real weekday selection"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn polished_calendar_eligible_keyup_and_disabled_past_preserve_draft_and_bounds(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory, _endpoint) = window_fixture(cx);
    let init_application = application.clone();
    let init_directory = directory.clone();
    let blob_directory = directory.clone();
    let view_slot = Rc::new(std::cell::RefCell::new(None));
    let opened_slot = view_slot.clone();
    let default_connection_limit = cx.read(|cx| application.read(cx).default_connection_limit());
    let default_adaptive_segment_policy =
        cx.read(|cx| application.read(cx).default_adaptive_segment_policy());
    let (_, surface) = cx.add_window_view(move |window, cx| {
        window.resize(new_download_window_size(true));
        let view = cx.new(|cx| {
            NewDownloadView::new(
                NewDownloadViewInit {
                    application: init_application,
                    default_directory: init_directory,
                    default_connection_limit,
                    default_adaptive_segment_policy,
                    categories: DownloadSettings::default().categories,
                    initial_source: Some(NewDownloadInitialSource::Blob(
                        BrowserBlobDialogContext {
                            correlation_id: uuid::Uuid::new_v4(),
                            handoff_id: uuid::Uuid::new_v4(),
                            blob: BrowserBlob {
                                id: uuid::Uuid::new_v4(),
                                file_name: "calendar-test.bin".into(),
                                media_type: None,
                                size: 4096,
                            },
                            save_directory: blob_directory.clone(),
                        },
                    )),
                },
                window,
                cx,
            )
        });
        *opened_slot.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    let view = view_slot
        .borrow()
        .clone()
        .expect("production add-download view");
    surface.refresh().expect("render confirmed download stage");
    let schedule = surface
        .debug_bounds("download-new-schedule-control")
        .expect("schedule trigger");
    surface.simulate_click(schedule.center(), Modifiers::default());
    surface.run_until_parked();
    surface
        .refresh()
        .expect("render production schedule popover");
    let shell = surface
        .debug_bounds("download-new-schedule-popover-shell")
        .expect("bounded popover");
    let today = surface
        .debug_bounds("download-new-schedule-today")
        .expect("today hit bounds");
    let past = surface
        .debug_bounds("download-new-schedule-past-day")
        .expect("past hit bounds");
    let next = surface
        .debug_bounds("download-new-schedule-next-day")
        .expect("next date hit bounds");
    assert_eq!(next.size, size(px(40.0), px(19.0)));
    for day in [today, past, next] {
        assert!(day.left() >= shell.left() && day.right() <= shell.right());
        assert!(day.top() >= shell.top() && day.bottom() <= shell.bottom());
    }
    let initial = surface.read(|cx| view.read(cx).scheduled_date);
    surface.simulate_click(past.center(), Modifiers::default());
    surface.run_until_parked();
    assert_eq!(
        surface.read(|cx| view.read(cx).scheduled_date),
        initial,
        "past date cannot dispatch"
    );
    struct CalendarKeyboardFixture {
        view: Entity<NewDownloadView>,
        today: NaiveDate,
    }
    impl Render for CalendarKeyboardFixture {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let date = self.view.read(cx).scheduled_date;
            div().w(px(280.0)).child(delayed_start_calendar(
                Theme::zed_dark(),
                calendar_month_start(self.today),
                date,
                self.today,
                self.view.downgrade(),
            ))
        }
    }
    let calendar_view = view.clone();
    let (_, surface) = cx.add_window_view(move |_, _| CalendarKeyboardFixture {
        view: calendar_view,
        today: initial,
    });
    surface
        .refresh()
        .expect("render actual calendar in isolated focus fixture");
    surface.update(|window, cx| {
        window.blur(cx);
        for _ in 0..4 {
            window.focus_next(cx);
        }
    });
    surface
        .refresh()
        .expect("Tab skips disabled past dates and reaches next date");
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("calendar Space"),
        is_held: false,
        prefer_character_input: false,
    });
    assert_eq!(
        surface.read(|cx| view.read(cx).scheduled_date),
        initial,
        "keydown cannot commit draft"
    );
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("calendar release"),
    });
    surface.run_until_parked();
    assert_eq!(
        surface.read(|cx| view.read(cx).scheduled_date),
        initial + ChronoDuration::days(1)
    );
    assert!(surface.read(|cx| view.read(cx).committed_schedule.is_none()));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn polished_progress_disclosure_keyup_reuses_the_task_window(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, endpoint) = window_fixture(cx);
    endpoint.set_connected_for_contract_test(true);
    let task_id = cx.read(|cx| {
        application
            .read(cx)
            .engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/disclosure.bin".into(),
                    save_directory: directory.clone(),
                    save_directory_explicit: false,
                    file_name: Some(
                        "可读的完整文件名-很长的中文名称-2026-归档与资料-最终版本-disclosure.bin"
                            .into(),
                    ),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: Some(4096),
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
            )
            .expect("queue synthetic task")
    });
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.refresh(cx);
            application.request_task_window(task_id, cx);
        });
    });
    cx.run_until_parked();
    let (handle, task_view) = cx.read(|cx| {
        let app = application.read(cx);
        let entry = app
            .task_windows
            .get(&task_id)
            .expect("keyed progress window");
        (entry.window, entry.view.clone())
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(440.0), px(184.0)));
    surface.refresh().expect("render actual progress window");
    let toggle = surface
        .debug_bounds("download-window-details-toggle")
        .expect("disclosure button");
    let body = surface
        .debug_bounds("download-task-window-body")
        .expect("progress body");
    assert_eq!(toggle.size, size(px(20.0), px(20.0)));
    assert!(toggle.top() >= body.top() && toggle.bottom() <= body.bottom());
    // Approved monitor replaces four stacked metrics/path with two compact
    // label/value columns; this asserts the actual production layout budget.
    let filename = surface
        .debug_bounds("download-window-progress-name")
        .expect("readable filename");
    let progress = surface
        .debug_bounds("download-window-progress-track")
        .expect("noncompeting progress");
    let metrics = surface
        .debug_bounds("download-window-progress-metrics")
        .expect("two-column metrics");
    let capabilities = surface
        .debug_bounds("download-window-progress-capabilities")
        .expect("resume and limit state");
    let footer = surface
        .debug_bounds("download-task-window-footer")
        .expect("footer");
    assert!(filename.size.height <= px(32.0));
    assert!(filename.bottom() <= progress.top() && progress.bottom() <= metrics.top());
    assert!(metrics.bottom() <= capabilities.top() && capabilities.bottom() <= footer.top());
    for (label_selector, value_selector) in [
        (
            "download-window-metric-已下载-label",
            "download-window-metric-已下载-value",
        ),
        (
            "download-window-metric-速度-label",
            "download-window-metric-速度-value",
        ),
        (
            "download-window-metric-剩余-label",
            "download-window-metric-剩余-value",
        ),
        (
            "download-window-metric-状态-label",
            "download-window-metric-状态-value",
        ),
    ] {
        let label_bounds = surface.debug_bounds(label_selector).expect("metric label");
        let value_bounds = surface.debug_bounds(value_selector).expect("metric value");
        assert!(label_bounds.right() <= value_bounds.left());
        assert_eq!(label_bounds.top(), value_bounds.top());
    }
    assert!(
        surface
            .debug_bounds("download-window-progress-path")
            .is_none()
    );
    surface.update(|window, cx| {
        window.focus_next(cx);
        window.focus_next(cx);
    });
    surface
        .refresh()
        .expect("Tab after caption close reaches actual disclosure");
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("disclosure Enter"),
        is_held: false,
        prefer_character_input: false,
    });
    assert!(
        !cx.read(|cx| task_view.read(cx).details_expanded),
        "activation waits for KeyUp"
    );
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("disclosure release"),
    });
    surface.run_until_parked();
    assert!(cx.read(|cx| task_view.read(cx).details_expanded));
    assert_eq!(cx.read(|cx| application.read(cx).task_windows.len()), 1);
    surface.simulate_resize(size(px(440.0), px(280.0)));
    surface.refresh().expect("expanded unchanged budget");
    let table = surface
        .debug_bounds("download-window-segment-table")
        .expect("part table instead of dots");
    let footer = surface
        .debug_bounds("download-task-window-footer")
        .expect("expanded footer");
    assert!(table.bottom() <= footer.top());
    assert!(
        surface
            .debug_bounds("download-window-details-source")
            .is_none()
    );
    assert!(
        surface
            .debug_bounds("download-window-details-destination")
            .is_none()
    );
    assert!(
        surface
            .debug_bounds("download-window-segment-dots")
            .is_none()
    );
    assert_eq!(
        surface
            .debug_bounds("download-task-window-shell")
            .expect("shell")
            .size
            .height,
        px(280.0)
    );
    let cancel = surface
        .debug_bounds("download-window-cancel")
        .expect("actual cancel");
    surface.simulate_click(cancel.center(), Modifiers::default());
    surface.run_until_parked();
    assert_eq!(
        cx.read(|cx| application
            .read(cx)
            .task(task_id)
            .expect("task retained")
            .task
            .status()),
        DownloadStatus::Cancelled
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn successful_probe_replaces_compact_window_with_fixed_confirmation(cx: &mut TestAppContext) {
    use std::io::{Read as _, Write as _};
    cx.update(gpui_kit::init);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("synthetic probe server");
    let address = listener.local_addr().expect("fixture address");
    listener.set_nonblocking(true).expect("bounded listener");
    let worker = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
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
                    let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\nfixture!");
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(_) => break,
            }
        }
    });
    let (application, directory, _endpoint) = window_fixture(cx);
    application.update(cx, |application, cx| {
        application.open_new_download_window(cx)
    });
    cx.run_until_parked();
    let (source, view) = cx.read(|cx| {
        let application = application.read(cx);
        (
            application.new_download_window.expect("compact source"),
            application.new_download_view.clone().expect("source view"),
        )
    });
    source
        .update(cx, |_, window, cx| {
            view.update(cx, |view, cx| {
                view.url.update(cx, |input, cx| {
                    input.replace_all(format!("http://{address}/fixture.bin"), window, cx)
                });
                view.begin_probe(window, cx);
            })
        })
        .expect("start actual probe");
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        cx.run_until_parked();
        if view.read_with(cx, |view, _| !view.probing) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "actual fixture probe must finish"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    cx.run_until_parked();
    let target = cx.read(|cx| {
        application
            .read(cx)
            .new_download_window
            .expect("fixed confirmation")
    });
    assert!(
        target != source,
        "probe success must create the fixed confirmation HWND, not resize the compact HWND"
    );
    assert!(
        source.update(cx, |_, _, _| {}).is_err(),
        "retired compact window must close once"
    );
    let mut surface = VisualTestContext::from_window(target, cx);
    surface
        .refresh()
        .expect("render actual confirmation after probe");
    let root = surface
        .debug_bounds("download-new-root")
        .expect("confirmation root");
    let footer = surface.debug_bounds("download-new-footer").expect("footer");
    let form = surface
        .debug_bounds("download-new-confirmed-form")
        .expect("confirmed fields");
    assert_eq!(root.size.height, px(NEW_DOWNLOAD_CONFIRMED_HEIGHT));
    assert!(form.bottom() <= footer.top());
    assert_eq!(footer.bottom(), root.bottom());
    worker.join().expect("bounded fixture ends");
    let _ = std::fs::remove_dir_all(directory);
}

fn stage_fixed_probe(
    cx: &mut TestAppContext,
    application: &Entity<DownloadApplication>,
) -> (AnyWindowHandle, Entity<NewDownloadView>, uuid::Uuid) {
    application.update(cx, |application, cx| {
        application.new_download_native_handles_override = Some((701, 702));
        application.open_new_download_window(cx);
    });
    cx.run_until_parked();
    let (source, view) = cx.read(|cx| {
        let application = application.read(cx);
        (
            application.new_download_window.expect("source"),
            application.new_download_view.clone().expect("view"),
        )
    });
    let operation_id = uuid::Uuid::new_v4();
    source
        .update(cx, |_, window, cx| {
            view.update(cx, |view, cx| {
                view.url.update(cx, |input, cx| {
                    input.set_value("https://fixture.invalid/file.bin", window, cx)
                });
                view.file_name
                    .update(cx, |input, cx| input.set_value("draft.bin", window, cx));
                view.confirmed_request = Some(view.request(cx).expect("valid staged request"));
                let mut external = view.confirmed_request.clone().expect("request");
                external.cookie = Some("synthetic-cookie".into());
                external.headers = vec![nxtools_download::DownloadRequestHeader {
                    name: "X-Synthetic-Header".into(),
                    value: "fixture-value".into(),
                }];
                external.referrer = Some("https://fixture.invalid/parent".into());
                external.speed_limit_bytes_per_second = Some(2048);
                view.external_request = Some(external.clone());
                view.confirmed_request = Some(external);
                view.selected_category_id = Some("synthetic-category".into());
                view.committed_schedule = Some(DelayedStartSelection {
                    date: view.scheduled_date,
                    hour: 12,
                    minute: 34,
                });
                view.selected_hls_variant = Some(2);
                view.selected_dash_video_track = Some(1);
                view.selected_dash_audio_track = Some(3);
                view.probe = Some(DownloadProbe {
                    resolution: None,
                    final_url: "https://fixture.invalid/file.bin".into(),
                    file_name: Some("server.bin".into()),
                    total_bytes: Some(8),
                    supports_ranges: false,
                    content_type: None,
                    validator: nxtools_download::ResourceValidator::default(),
                    media: None,
                });
                view.probe_generation = 7;
                view.handoff_staged = true;
                view.handoff_pending = true;
                view.save_directory_explicit = false;
                cx.notify();
            });
        })
        .expect("stage retained probe");
    cx.run_until_parked();
    application.update(cx, |application, cx| {
        application.create_fixed_confirmation(source, view.downgrade(), 7, operation_id, cx)
    });
    cx.run_until_parked();
    (source, view, operation_id)
}

fn fixed_result(
    cx: &mut TestAppContext,
    application: &Entity<DownloadApplication>,
    id: uuid::Uuid,
    success: bool,
) {
    application.update(cx, |application, cx| {
        application.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: id,
                result: PlatformCommandResult {
                    success,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        )
    });
    cx.run_until_parked();
}

#[gpui_kit::test]
fn fixed_confirmation_failure_disconnect_timeout_retain_probe_and_retry_without_network(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    for terminal in ["native_result", "disconnect", "timeout"] {
        let (application, directory, _endpoint) = window_fixture(cx);
        let (source, view, id) = stage_fixed_probe(cx, &application);
        let key = view.read_with(cx, |view, _| view.submission_idempotency_key);
        assert_eq!(
            cx.read(|cx| application.read(cx).new_download_handoff_commands.len()),
            1
        );
        let windows_before_terminal = cx.windows().len();
        let mut surface = VisualTestContext::from_window(source, cx);
        surface.refresh().expect("compact source pending");
        assert!(
            surface
                .debug_bounds("download-new-confirmed-form")
                .is_none()
        );
        assert_eq!(
            surface
                .debug_bounds("download-new-root")
                .expect("root")
                .size
                .height,
            px(176.0)
        );
        match terminal {
            "native_result" => fixed_result(cx, &application, id, false),
            "disconnect" => {
                application.update(cx, |application, cx| {
                    application.handle_platform_notification(
                        PlatformNotification::Disconnected {
                            generation: 1,
                            pending_command_ids: vec![id],
                        },
                        cx,
                    )
                });
                cx.run_until_parked();
            }
            _ => {
                cx.executor().advance_clock(Duration::from_secs(10));
                cx.run_until_parked();
            }
        }
        assert!(cx.read(|cx| application.read(cx).pending_new_download_handoff.is_none()));
        assert_eq!(
            cx.windows().len() + 1,
            windows_before_terminal,
            "failed hidden target must close"
        );
        assert!(cx.read(|cx| application.read(cx).new_download_window) == Some(source));
        assert!(view.read_with(cx, |view, _| view.probe.is_some()
            && view.confirmed_request.is_some()
            && !view.handoff_pending));
        fixed_result(cx, &application, id, true);
        assert!(
            cx.read(|cx| application.read(cx).new_download_window) == Some(source),
            "late success has no authority"
        );
        source
            .update(cx, |_, window, cx| {
                view.update(cx, |view, cx| view.begin_probe(window, cx))
            })
            .expect("retry presentation only");
        cx.run_until_parked();
        let retry_id = cx.read(|cx| {
            application
                .read(cx)
                .pending_new_download_handoff
                .as_ref()
                .expect("retry slot")
                .command_id
        });
        assert!(view.read_with(cx, |view, _| !view.probing && view.probe_generation == 7));
        assert_eq!(
            cx.read(|cx| application.read(cx).new_download_handoff_commands.len()),
            2
        );
        fixed_result(cx, &application, retry_id, true);
        let target_view = cx.read(|cx| {
            application
                .read(cx)
                .new_download_view
                .clone()
                .expect("new view")
        });
        assert!(source.update(cx, |_, _, _| {}).is_err());
        assert!(
            target_view.read_with(cx, |target, cx| target.submission_idempotency_key == key
                && !target.save_directory_explicit
                && target.file_name.read(cx).value() == "draft.bin"
                && target.url.entity_id() != view.read(cx).url.entity_id()
                && target.confirmed_request == view.read(cx).confirmed_request
                && target.external_request == view.read(cx).external_request
                && target.selected_category_id == view.read(cx).selected_category_id
                && target.committed_schedule == view.read(cx).committed_schedule
                && target.selected_hls_variant == Some(2)
                && target.selected_dash_video_track == Some(1)
                && target.selected_dash_audio_track == Some(3))
        );
        let target = cx.read(|cx| {
            application
                .read(cx)
                .new_download_window
                .expect("fixed target")
        });
        let mut confirmation = VisualTestContext::from_window(target, cx);
        confirmation
            .refresh()
            .expect("confirmation footer after native seam");
        let root = confirmation
            .debug_bounds("download-new-root")
            .expect("root");
        let footer = confirmation
            .debug_bounds("download-new-footer")
            .expect("footer");
        assert_eq!(root.size.height, px(NEW_DOWNLOAD_CONFIRMED_HEIGHT));
        assert_eq!(footer.bottom(), root.bottom());
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[gpui_kit::test]
fn fixed_confirmation_close_and_stale_generation_never_resurrect_source(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    for close in [true, false] {
        let (application, directory, _endpoint) = window_fixture(cx);
        let (source, view, id) = stage_fixed_probe(cx, &application);
        if close {
            source
                .update(cx, |_, window, _| window.remove_window())
                .expect("cancel source");
        } else {
            view.update(cx, |view, _| view.probe_generation += 1);
        }
        cx.run_until_parked();
        fixed_result(cx, &application, id, true);
        assert!(cx.read(|cx| application.read(cx).pending_new_download_handoff.is_none()));
        let before = cx.windows().len();
        application.update(cx, |application, cx| {
            application.create_fixed_confirmation(
                source,
                view.downgrade(),
                7,
                uuid::Uuid::new_v4(),
                cx,
            )
        });
        cx.run_until_parked();
        assert_eq!(
            cx.windows().len(),
            before,
            "retired owner must not create a replacement"
        );
        if close {
            assert!(cx.read(|cx| application.read(cx).new_download_window.is_none()));
        } else {
            assert!(cx.read(|cx| application.read(cx).new_download_window) == Some(source));
        }
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[gpui_kit::test]
fn pending_fixed_handoff_locks_url_and_paste_until_terminal(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, _endpoint) = window_fixture(cx);
    let (source, view, id) = stage_fixed_probe(cx, &application);
    let mut surface = VisualTestContext::from_window(source, cx);
    surface.refresh().expect("pending compact source");
    let before = view.read_with(&surface, |view, cx| view.url.read(cx).value().to_string());
    view.update_in(&mut surface, |view, window, cx| {
        view.url.focus_handle(cx).focus(window, cx)
    });
    surface.simulate_input("changed");
    assert_eq!(
        view.read_with(&surface, |view, cx| view.url.read(cx).value().to_string()),
        before,
        "pending native handoff must not accept URL edits"
    );
    surface.update(|_, cx| {
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
            "https://example.test/changed".into(),
        ))
    });
    let paste = surface
        .debug_bounds("download-new-url-paste")
        .expect("paste action");
    surface.simulate_click(paste.center(), Modifiers::default());
    assert_eq!(
        view.read_with(&surface, |view, cx| view.url.read(cx).value().to_string()),
        before,
        "pending native handoff must not accept paste"
    );
    fixed_result(cx, &application, id, false);
    surface.refresh().expect("failure restores editing");
    view.update_in(&mut surface, |view, window, cx| {
        view.url.focus_handle(cx).focus(window, cx)
    });
    surface.simulate_input("changed");
    assert_ne!(
        view.read_with(&surface, |view, cx| view.url.read(cx).value().to_string()),
        before
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn collapsed_completion_keeps_progress_geometry_without_a_two_pixel_drop() {
    assert_eq!(
        download_task_window_size(DownloadWindowPhase::Progress),
        download_task_window_size(DownloadWindowPhase::Completed),
        "ordinary completion should swap content without moving/resizing the compact HWND"
    );
}

#[gpui_kit::test]
fn collapsed_task_completion_changes_content_without_native_resize(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, endpoint) = window_fixture(cx);
    endpoint.set_connected_for_contract_test(true);
    let engine = application.read_with(cx, |application, _| application.engine.clone());
    let task_id = engine
        .enqueue(
            DownloadRequest {
                source_url: "https://example.test/completion-fixture.bin".into(),
                save_directory: directory.clone(),
                save_directory_explicit: false,
                file_name: Some("completion-fixture.bin".into()),
                headers: vec![],
                cookie: None,
                referrer: None,
                certificate_policy: Default::default(),
                connection_limit: Some(1),
                speed_limit_bytes_per_second: None,
                expected_total_bytes: Some(4096),
                scheduled_at_unix_ms: None,
                adaptive_segment_policy: None,
                media: None,
                category_id: None,
                category_name: None,
                duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
            },
            RequestOrigin::Internal,
        )
        .expect("synthetic task");
    application.update(cx, |application, cx| {
        application.task_window_native_handle_override = Some(42);
        application.refresh(cx);
        application.request_task_window(task_id, cx);
    });
    cx.run_until_parked();
    let (source, generation) = application.read_with(cx, |application, _| {
        let entry = application
            .task_windows
            .get(&task_id)
            .expect("registered progress");
        (entry.window, entry.generation)
    });
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    let mut completed = load_download_task_detail(&engine, task_id)
        .expect("synthetic detail")
        .task;
    for (status, timestamp) in [
        (DownloadStatus::Probing, 2),
        (DownloadStatus::Waiting, 3),
        (DownloadStatus::Downloading, 4),
        (DownloadStatus::Finalizing, 5),
        (DownloadStatus::Completed, 6),
    ] {
        completed
            .task
            .transition_to(status, timestamp)
            .expect("synthetic terminal transition");
    }
    application.update(cx, |application, cx| {
        application.replace_tasks_for_test(vec![completed]);
        let directive = application
            .task_window_lifecycle
            .task_changed(task_id, DownloadStatus::Completed);
        application.apply_task_window_directive(task_id, directive, cx);
    });
    cx.run_until_parked();
    let commands: Vec<_> =
        std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok()).collect();
    assert!(
        !commands.iter().any(|envelope| matches!(
            envelope.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowBoundsCentered(_))
        )),
        "same-size ordinary completion must not enqueue a resize that shifts its top edge"
    );
    application.read_with(cx, |application, _| {
        let entry = application
            .task_windows
            .get(&task_id)
            .expect("same registered task");
        assert!(entry.window == source);
        assert_eq!(entry.generation, generation);
    });
    let mut surface = VisualTestContext::from_window(source, cx);
    surface
        .refresh()
        .expect("completed content without native size transition");
    let shell = surface
        .debug_bounds("download-task-window-shell")
        .expect("task shell");
    let footer = surface
        .debug_bounds("download-task-window-footer")
        .expect("fixed footer");
    assert_eq!(shell.size.height, px(184.0));
    assert_eq!(shell.bottom(), footer.bottom());
    assert!(surface.debug_bounds("download-complete-summary").is_some());
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn auxiliary_confirmation_input_picker_tier_preserves_fixed_handoff(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, endpoint) = window_fixture(cx);
    endpoint.set_connected_for_contract_test(true);
    let (source, source_view, id) = stage_fixed_probe(cx, &application);
    let key = source_view.read_with(cx, |view, _| view.submission_idempotency_key);
    fixed_result(cx, &application, id, true);
    let (target, view) = cx.read(|cx| {
        let app = application.read(cx);
        (
            app.new_download_window.expect("fixed target"),
            app.new_download_view.clone().expect("view"),
        )
    });
    assert!(source.update(cx, |_, _, _| ()).is_err());
    assert!(view.read_with(cx, |view, _| view.probe_generation == 7
        && view.submission_idempotency_key == key
        && view.committed_schedule.is_some()));
    let mut surface = VisualTestContext::from_window(target, cx);
    surface.refresh().expect("fixed confirmation");
    let root = surface.debug_bounds("download-new-root").expect("root");
    let footer = surface.debug_bounds("download-new-footer").expect("footer");
    assert_eq!(root.size, new_download_window_size(true));
    for (input, picker) in [
        ("download-new-url-field-input", "download-new-url-paste"),
        (
            "download-new-directory-input",
            "download-new-directory-picker",
        ),
    ] {
        let input = surface.debug_bounds(input).expect("field");
        let picker = surface.debug_bounds(picker).expect("chooser");
        assert_eq!(input.size.height, px(CONTROL_HEIGHT_INPUT));
        assert_eq!(picker.size.height, input.size.height);
        assert_eq!(picker.top(), input.top());
        assert!(picker.right() <= root.right() && picker.bottom() <= footer.top());
    }
    let capability = surface
        .debug_bounds("download-new-probe-capability")
        .expect("capability");
    let inspector = surface
        .debug_bounds("download-new-probe-inspector")
        .expect("inspector");
    assert_eq!(capability.size.height, px(LINE_HEIGHT_BODY));
    assert!(capability.left() >= inspector.left() && capability.right() <= inspector.right());
    assert!(capability.bottom() <= inspector.bottom() && inspector.bottom() <= footer.top());
    assert!(
        footer.top() - inspector.bottom() <= px(8.0),
        "ordinary confirmed form must not reserve a blank media-sized tail"
    );
    let filename = surface
        .debug_bounds("download-new-file-name-field-label")
        .expect("filename Form label");
    let category = surface
        .debug_bounds("download-new-category-field-label")
        .expect("category Form label");
    assert_eq!(filename.top(), category.top());
    // The real popover/list rows must constrain their internal Kit children,
    // not only the caller's outer div. Existing keyboard owner is unchanged.
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.categories = vec![DownloadCategorySettings {
                id: "long-category".into(),
                name: "中文长分类标题用于检验宽度不会挤压滚动区域".repeat(4),
                save_directory: "C:\\synthetic\\long-directory\\".repeat(12),
                enabled: true,
                extensions: Vec::new(),
                url_patterns: Vec::new(),
            }];
            view.category_popover_open = true;
            cx.notify();
        });
    });
    surface
        .refresh()
        .expect("real category popover long labels");
    let popover = surface
        .debug_bounds("download-new-category-popover-content")
        .expect("popover");
    let search = surface
        .debug_bounds("download-new-category-search")
        .expect("search header");
    let list = surface
        .debug_bounds("download-new-category-list")
        .expect("list viewport");
    let row = surface
        .debug_bounds("download-new-category-option-0")
        .expect("real ListItem");
    let title = surface
        .debug_bounds("download-new-category-option-title-0")
        .expect("title");
    let detail = surface
        .debug_bounds("download-new-category-option-directory-0")
        .expect("directory");
    assert!(search.bottom() <= list.top() && list.bottom() <= popover.bottom());
    assert!(title.left() >= row.left() && title.right() <= row.right());
    assert!(title.bottom() <= detail.top() && detail.bottom() <= row.bottom());
    assert!(detail.right() <= row.right());
    assert_eq!(
        cx.read(|cx| view.read(cx).category_scroll.max_offset().x),
        px(0.0)
    );
    surface.simulate_keystrokes("escape");
    surface.run_until_parked();
    let cancel = surface
        .debug_bounds("download-new-cancel-action")
        .expect("cancel");
    surface.simulate_click(cancel.center(), Modifiers::default());
    surface.run_until_parked();
    assert!(target.update(cx, |_, _, _| ()).is_err());
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn auxiliary_schedule_rails_long_validation_and_local_actions(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, _endpoint) = window_fixture(cx);
    let slot = Rc::new(std::cell::RefCell::new(None));
    let rendered_slot = slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(430.0), px(326.0)));
        let view = cx.new(|cx| DownloadScheduleView::new(application, None, window, cx));
        *rendered_slot.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(430.0), px(326.0)));
    let view = slot.borrow().clone().expect("schedule entity");
    surface.refresh().expect("default schedule");
    for id in [
        "download-schedule-start-toggle",
        "download-schedule-stop-toggle",
    ] {
        let toggle = surface.debug_bounds(id).expect("independent switch");
        surface.simulate_click(toggle.center(), Modifiers::default());
        surface.refresh().expect("local time row");
    }
    assert!(view.read_with(surface, |view, _| view.enabled_start && view.enabled_stop));
    let first = surface
        .debug_bounds("download-schedule-day-0")
        .expect("weekday");
    surface.simulate_click(first.center(), Modifiers::default());
    assert!(!view.read_with(surface, |view, _| {
        view.days.contains(&DownloadWeekday::ALL[0])
    }));
    for id in [
        "download-schedule-start-toggle",
        "download-schedule-stop-toggle",
    ] {
        let toggle = surface.debug_bounds(id).expect("toggle");
        let input = surface
            .debug_bounds(Box::leak(format!("{id}-input").into_boxed_str()))
            .expect("clock shell");
        let title = surface
            .debug_bounds(Box::leak(format!("{id}-title").into_boxed_str()))
            .expect("title");
        let description = surface
            .debug_bounds(Box::leak(format!("{id}-description").into_boxed_str()))
            .expect("description");
        assert_eq!(
            toggle.size.height,
            px(32.0),
            "default Switch composition slot"
        );
        assert_eq!(toggle.top(), input.top());
        assert_eq!(toggle.size.height, input.size.height);
        assert_eq!(title.size.height, px(22.5), "released Form text_sm label");
        assert!(description.size.width <= px(24.0));
        assert!(
            (title.center().y - description.center().y).abs() <= px(1.0)
                && description.right() <= toggle.left()
        );
    }
    let start = surface
        .debug_bounds("download-schedule-start-toggle-input")
        .expect("start rail");
    let stop = surface
        .debug_bounds("download-schedule-stop-toggle-input")
        .expect("stop rail");
    assert_eq!(start.left(), stop.left());
    assert!(
        stop.top() - start.bottom() <= px(12.0),
        "time rows need compact rhythm: start={start:?} stop={stop:?}"
    );
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.days.clear();
            cx.notify();
        })
    });
    let apply = surface
        .debug_bounds("download-schedule-primary-apply-action")
        .expect("apply");
    surface.simulate_click(apply.center(), Modifiers::default());
    surface.run_until_parked();
    assert!(view.read_with(surface, |view, _| !view.saving
        && view.error.is_some()
        && view.enabled_start
        && view.enabled_stop));
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.error = Some("计划时间验证失败，请检查开始、停止时间与生效星期。".repeat(20));
            cx.notify();
        })
    });
    surface.refresh().expect("long validation");
    let error = surface
        .debug_bounds("download-schedule-validation")
        .expect("bounded error");
    let body = surface
        .debug_bounds("download-schedule-dialog-body")
        .expect("body");
    let footer = surface
        .debug_bounds("download-schedule-dialog-footer")
        .expect("footer");
    assert!(error.size.height <= px(36.0));
    assert!(
        error.left() >= body.left()
            && error.right() <= body.right()
            && error.bottom() <= footer.top()
    );
    assert_eq!(
        surface
            .debug_bounds("download-schedule-dialog-shell")
            .expect("shell")
            .size,
        size(px(430.0), px(326.0))
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn auxiliary_shutdown_readable_consequence_and_pending_actions(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, endpoint) = window_fixture(cx);
    endpoint.set_connected_for_contract_test(true);
    let task_id = DownloadTaskId::new();
    application.update(cx, |application, cx| {
        application
            .completion_actions
            .set_shutdown_armed(task_id, true);
        let transition = application
            .completion_actions
            .evaluate_shutdown([(task_id, DownloadStatus::Completed)], Instant::now());
        application.apply_shutdown_transition(transition, cx);
    });
    cx.run_until_parked();
    let handle = cx.read(|cx| {
        application
            .read(cx)
            .shutdown_window
            .as_ref()
            .expect("countdown")
            .window
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(450.0), px(200.0)));
    surface.refresh().expect("countdown");
    let copy = surface
        .debug_bounds("download-shutdown-consequence")
        .expect("consequence");
    let title = surface
        .debug_bounds("download-shutdown-heading")
        .expect("heading");
    let footer = surface
        .debug_bounds("download-shutdown-dialog-footer")
        .expect("footer");
    assert_eq!(title.left(), copy.left());
    assert!(
        copy.size.height >= px(36.5),
        "actual risk Alert natural lines remain visible"
    );
    assert!(title.bottom() <= copy.top() && copy.bottom() <= footer.top());
    assert_eq!(
        surface
            .debug_bounds("download-shutdown-dialog-shell")
            .expect("shell")
            .size,
        size(px(450.0), px(200.0))
    );
    let now = surface
        .debug_bounds("download-shutdown-danger-now-action")
        .expect("now");
    surface.simulate_click(now.center(), Modifiers::default());
    surface.run_until_parked();
    let envelope = endpoint
        .try_receive_command_for_contract_test()
        .expect("typed shutdown command");
    assert!(matches!(
        envelope.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::ShutdownSystem(
            ShutdownSystemCommand { delay_seconds: 1 }
        ))
    ));
    surface.refresh().expect("pending");
    let cancel = surface
        .debug_bounds("download-shutdown-secondary-cancel-action")
        .expect("cancel");
    surface.simulate_click(cancel.center(), Modifiers::default());
    surface.simulate_click(now.center(), Modifiers::default());
    surface.run_until_parked();
    assert!(
        cx.read(|cx| application.read(cx).pending_shutdown_command.is_some()
            && application.read(cx).shutdown_window.is_some())
    );
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "pending actions must not dispatch twice"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn auxiliary_delete_long_path_and_consequence_keep_cancel_presentation_only(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory, _endpoint) = window_fixture(cx);
    let task_id = cx.read(|cx| {
        application
            .read(cx)
            .engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://fixture.invalid/delete.bin".into(),
                    save_directory: directory.clone(),
                    save_directory_explicit: false,
                    file_name: Some("delete.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
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
                },
                RequestOrigin::Internal,
            )
            .expect("synthetic task")
    });
    cx.read(|cx| {
        application
            .read(cx)
            .engine
            .cancel(task_id)
            .expect("cancel synthetic task")
    });
    application.update(cx, |application, cx| {
        application.refresh(cx);
        let mut task = application
            .task(task_id)
            .expect("synthetic snapshot")
            .clone();
        task.final_path = Some(PathBuf::from(format!(
            "C:\\{}\\不可撤销的文件.bin",
            "很长的合成目录\\".repeat(40)
        )));
        application.replace_tasks_for_test(vec![task]);
        application.config.download.task_removal_deletes_file = true;
        application.request_delete_confirmation(task_id, cx);
    });
    cx.run_until_parked();
    let handle = cx.read(|cx| {
        application
            .read(cx)
            .delete_confirmation_windows
            .get(&task_id)
            .copied()
            .expect("delete window")
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.refresh().expect("delete consequence");
    let path = surface
        .debug_bounds("download-delete-path")
        .expect("bounded path");
    let consequence = surface
        .debug_bounds("download-delete-consequence")
        // Official XSmall Alert has natural text plus released padding/ring.
        .expect("consequence");
    let body = surface
        .debug_bounds("download-delete-dialog-body")
        .expect("body");
    let footer = surface
        .debug_bounds("download-delete-dialog-footer")
        .expect("footer");
    assert_eq!(path.size.height, px(LINE_HEIGHT_BODY));
    assert_eq!(path.left(), consequence.left());
    assert!(consequence.bottom() <= path.top() && path.bottom() <= footer.top());
    assert!(
        consequence.size.height >= px(36.5),
        "real XSmall risk Alert must retain its natural visible content"
    );
    assert!(path.left() >= body.left() && path.right() <= body.right());
    let cancel = surface
        .debug_bounds("download-delete-secondary-cancel-action")
        .expect("cancel");
    let confirm = surface
        .debug_bounds("download-delete-danger-confirm-action")
        .expect("confirm");
    assert!(cancel.right() <= confirm.left() && confirm.bottom() <= footer.bottom());
    surface.simulate_click(cancel.center(), Modifiers::default());
    surface.run_until_parked();
    assert!(handle.update(cx, |_, _, _| ()).is_err());
    assert!(cx.read(|cx| {
        application.read(cx).task(task_id).is_some()
            && !application
                .read(cx)
                .delete_confirmation_windows
                .contains_key(&task_id)
    }));
    let _ = std::fs::remove_dir_all(directory);
}

fn queue02_settings_window(
    cx: &mut TestAppContext,
) -> (Entity<DownloadApplication>, PathBuf, VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (application, directory, _) = window_fixture(cx);
    cx.update(|cx| application.update(cx, |application, cx| application.open_settings_window(cx)));
    cx.run_until_parked();
    let handle = cx
        .read(|cx| application.read(cx).settings_window)
        .expect("settings window");
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(984.0), px(650.0)));
    surface.refresh().expect("production settings");
    (application, directory, surface)
}

#[gpui_kit::test]
fn download_settings_save_failure_uses_center_and_keeps_original_draft(cx: &mut TestAppContext) {
    let (application, directory, mut surface) = queue02_settings_window(cx);
    let view = surface.read(|cx| {
        application
            .read(cx)
            .settings_view
            .clone()
            .expect("production settings view")
    });
    let input = surface.read(|cx| view.read(cx).directory.entity_id());
    view.update_in(&mut surface, |view, window, cx| view.save(window, cx));
    surface.run_until_parked();
    view.read_with(&surface, |view, _| {
        assert!(!view.saving);
        assert!(
            view.error.is_none(),
            "persistence failure must not leave a second page banner"
        );
        assert_eq!(view.directory.entity_id(), input);
    });
    application.read_with(&surface, |app, cx| {
        assert!(
            app.settings_window.is_some(),
            "failed save remains retryable"
        );
        assert!(
            app.notifications
                .read(cx)
                .items()
                .iter()
                .any(|notice| notice.level == crate::notifications::NotificationLevel::Error)
        );
    });
    assert!(
        !directory.join("config.json").exists(),
        "missing config never resurrected"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn browser_export_completion_uses_center_not_settings_description(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (application, directory, endpoint) = window_fixture(cx);
    endpoint.set_connected_for_contract_test(true);
    application.update(cx, |app, cx| app.open_settings_window(cx));
    cx.run_until_parked();
    let handle = application.read_with(cx, |app, _| app.settings_window.expect("settings"));
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(984.0), px(650.0)));
    surface.refresh().expect("real settings");
    let viewport = surface
        .debug_bounds("download-settings-shell-footer")
        .expect("mounted footer");
    for (index, button) in [
        "download-settings-extension-export-chrome-button",
        "download-settings-extension-export-firefox-button",
    ]
    .into_iter()
    .enumerate()
    {
        let mut target = None;
        for _ in 0..60 {
            surface.refresh().expect("export action frame");
            target = surface
                .update(|window, _| gpui_kit::base::test_support::snapshots(window))
                .into_iter()
                .find(|node| {
                    node.path().last() == Some(&button.into())
                        && node.bounds().top() >= px(110.0)
                        && node.bounds().bottom() <= px(590.0)
                })
                .map(|node| node.bounds());
            if target.is_some() {
                break;
            }
            surface.simulate_event(gpui_kit::ScrollWheelEvent {
                position: gpui_kit::point(px(900.0), px(320.0)),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -2.0)),
                modifiers: Modifiers::default(),
                touch_phase: gpui_kit::TouchPhase::Moved,
            });
            surface.run_until_parked();
        }
        surface.simulate_click(
            target.expect("reachable official export Button").center(),
            Modifiers::default(),
        );
        surface.run_until_parked();
        let command = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
            .find(|command| {
                matches!(
                    command.message,
                    nxtools_protocol::Message::PlatformCommand(
                        PlatformCommand::ExportBrowserExtension(_)
                    )
                )
            })
            .expect("export dispatched");
        let key = format!("download-browser-export-{}", command.id);
        application.read_with(&surface, |app, cx| {
            let notices = app.notifications.read(cx).items();
            assert_eq!(notices.len(), index + 1);
            let notice = notices
                .iter()
                .find(|notice| notice.id.as_ref() == key.as_str())
                .expect("matching invocation progress");
            assert_eq!(
                notice.level,
                crate::notifications::NotificationLevel::Progress
            );
        });
        application.update(&mut surface, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: command.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some("synthetic-extension.zip".into()),
                    },
                },
                cx,
            )
        });
        surface.run_until_parked();
        application.read_with(&surface, |app, cx| {
            let notices = app.notifications.read(cx).items();
            assert_eq!(
                notices.len(),
                index + 1,
                "terminal replaces only this invocation's progress"
            );
            assert!(
                notices
                    .iter()
                    .all(|notice| notice.level == crate::notifications::NotificationLevel::Success)
            );
        });
    }
    surface
        .refresh()
        .expect("export feedback does not relayout settings");
    assert_eq!(
        surface
            .debug_bounds("download-settings-shell-footer")
            .expect("unchanged footer"),
        viewport
    );
    application.read_with(&surface, |app, cx| {
        let notices = app.notifications.read(cx).items();
        assert_eq!(notices.len(), 2);
        assert_ne!(notices[0].id, notices[1].id);
    });
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn schedule_terminal_failure_is_not_repainted_as_field_validation(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, _) = window_fixture(cx);
    let retained = application.clone();
    let slot = Rc::new(RefCell::new(None));
    let opened = slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| DownloadScheduleView::new(retained, None, window, cx));
        *opened.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    let view = slot.borrow().clone().expect("production schedule");
    view.update_in(surface, |view, _, cx| {
        view.saving = true;
        view.finish_save_failure("合成队列写入失败".into(), cx);
        assert!(!view.saving);
        assert!(
            view.error.is_none(),
            "terminal error is already owned by NotificationCenter"
        );
    });
    surface.run_until_parked();
    application.read_with(surface, |app, cx| {
        assert!(
            app.notifications
                .read(cx)
                .items()
                .iter()
                .any(|notice| notice.message == "合成队列写入失败")
        )
    });
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn queue02_settings_mounts_real_official_search_and_hierarchy(cx: &mut TestAppContext) {
    let (_application, directory, mut surface) = queue02_settings_window(cx);
    let snapshots = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert!(
        snapshots.iter().any(|snapshot| snapshot
            .path()
            .iter()
            .any(|id| id == &gpui_kit::ElementId::from("settings-sidebar"))
            && snapshot.role() == Some(gpui_kit::Role::TextInput)),
        "official Settings search"
    );
    let footer = surface
        .debug_bounds("download-settings-shell-footer")
        .expect("stable footer");
    assert_eq!(footer.bottom(), px(650.0));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn queue02_headers_have_one_styled_textarea_frame(cx: &mut TestAppContext) {
    let (application, directory, mut surface) = queue02_settings_window(cx);
    let view = surface.read(|cx| application.read(cx).settings_view.clone().expect("view"));
    let input = view.read_with(&surface, |view, _| view.default_request_headers.clone());
    select_official_download_page(&mut surface, DownloadSettingsPage::Request);
    surface.refresh().expect("request page");
    let field = surface
        .debug_bounds("download-settings-default-headers-field")
        .expect("header slot");
    let actual = surface
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|snapshot| {
            snapshot
                .path()
                .iter()
                .any(|id| id == &gpui_kit::ElementId::from(("input", input.entity_id())))
        })
        .expect("original textarea entity")
        .bounds();
    assert_eq!(
        actual.size, field.size,
        "no duplicate painted frame or inset"
    );
    assert_eq!(actual.size.height, px(120.0));
    let identity = input.entity_id();
    surface.simulate_click(actual.center(), Modifiers::default());
    surface.simulate_input("X-Synthetic: 合成输入\nX-Second: bounded");
    surface.run_until_parked();
    assert!(
        surface.read(|cx| input.read(cx).value().contains("合成输入")),
        "original multiline entity accepts user text without logging payload"
    );
    select_official_download_page(&mut surface, DownloadSettingsPage::General);
    select_official_download_page(&mut surface, DownloadSettingsPage::Request);
    assert_eq!(
        view.read_with(&surface, |view, _| view.default_request_headers.entity_id()),
        identity
    );
    assert_eq!(
        surface
            .debug_bounds("download-settings-default-headers-field")
            .expect("stable frame")
            .size,
        actual.size
    );
    let _ = std::fs::remove_dir_all(directory);
}

pub(super) fn official_download_navigation_bounds(
    window: &mut VisualTestContext,
    page: DownloadSettingsPage,
) -> gpui_kit::Bounds<gpui_kit::Pixels> {
    window.refresh().expect("official download navigation");
    window
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|snapshot| {
            snapshot.path().last()
                == Some(&gpui_kit::ElementId::from(gpui_kit::SharedString::from(
                    format!("0-{}", page.index()),
                )))
        })
        .expect("published Settings menu node")
        .bounds()
}

pub(super) fn select_official_download_page(
    window: &mut VisualTestContext,
    page: DownloadSettingsPage,
) {
    let bounds = official_download_navigation_bounds(window, page);
    window.simulate_click(bounds.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("official download page");
}

pub(super) fn reveal_download_setting(
    window: &mut VisualTestContext,
    selector: &'static str,
) -> gpui_kit::Bounds<gpui_kit::Pixels> {
    for _ in 0..60 {
        window.refresh().expect("real setting frame");
        if let Some(bounds) = window.debug_bounds(selector)
            && bounds.top() >= px(110.0)
            && bounds.bottom() <= px(590.0)
        {
            return bounds;
        }
        window.simulate_event(gpui_kit::ScrollWheelEvent {
            position: gpui_kit::point(px(900.0), px(320.0)),
            delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -2.0)),
            modifiers: Modifiers::default(),
            touch_phase: gpui_kit::TouchPhase::Moved,
        });
        window.run_until_parked();
    }
    panic!("setting must be reachable inside fixed content viewport: {selector}");
}

pub(super) fn official_download_group_bounds(
    window: &mut VisualTestContext,
    page: DownloadSettingsPage,
    group: usize,
) -> gpui_kit::Bounds<gpui_kit::Pixels> {
    window.refresh().expect("official group");
    let snapshots = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    let group_id =
        gpui_kit::ElementId::from(gpui_kit::SharedString::from(format!("group-{group}")));
    snapshots
        .into_iter()
        .filter(|snapshot| {
            snapshot.path().iter().any(|id| id == &group_id)
                && snapshot.role() == Some(gpui_kit::Role::Group)
                && snapshot.label() == Some(page.label())
        })
        .max_by(|a, b| {
            a.bounds()
                .size
                .width
                .as_f32()
                .total_cmp(&b.bounds().size.width.as_f32())
        })
        .unwrap_or_else(|| {
            panic!(
                "current official group content: {} group{}",
                page.label(),
                group
            )
        })
        .bounds()
}

#[gpui_kit::test]
fn queue02_settings_search_preserves_draft_and_busy_footer(cx: &mut TestAppContext) {
    let (application, directory, mut surface) = queue02_settings_window(cx);
    let view = surface.read(|cx| {
        application
            .read(cx)
            .settings_view
            .clone()
            .expect("settings view")
    });
    let snapshots = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    let search = snapshots
        .iter()
        .find(|snapshot| {
            snapshot.role() == Some(gpui_kit::Role::TextInput)
                && snapshot
                    .path()
                    .iter()
                    .any(|id| id == &gpui_kit::ElementId::from("settings-sidebar"))
        })
        .expect("official search")
        .bounds();
    surface.simulate_click(search.center(), Modifiers::default());
    for (query, expected) in [
        ("Cookie", Some("请求")),
        ("日志", Some("诊断")),
        ("no-synthetic-setting-match", None),
    ] {
        surface.simulate_keystrokes("ctrl-a");
        surface.simulate_input(query);
        surface.run_until_parked();
        surface.refresh().expect("official search filter");
        let snapshots = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
        let active = snapshots
            .iter()
            .filter(|snapshot| {
                snapshot.role() == Some(gpui_kit::Role::Group)
                    && ["常规", "请求", "传输", "窗口", "诊断"]
                        .contains(&snapshot.label().unwrap_or(""))
            })
            .collect::<Vec<_>>();
        match expected {
            Some(expected) => {
                assert!(!active.is_empty());
                assert!(
                    active
                        .iter()
                        .all(|snapshot| snapshot.label() == Some(expected))
                );
            }
            None => assert!(active.is_empty(), "no-match unmounts business actions"),
        }
        assert!(
            !surface.read(|cx| view.read(cx).saving),
            "search never submits draft"
        );
    }
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_keystrokes("backspace");
    surface.run_until_parked();
    surface
        .refresh()
        .expect("clear search retains official selection");
    assert!(
        surface
            .update(|window, _| gpui_kit::base::test_support::snapshots(window))
            .iter()
            .any(|snapshot| snapshot.role() == Some(gpui_kit::Role::Group)),
        "clear remounts official current content"
    );
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input("日志");
    surface.run_until_parked();
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_keystrokes("backspace");
    surface.run_until_parked();
    surface.refresh().expect("clear matched search");
    assert!(
        surface
            .update(|window, _| gpui_kit::base::test_support::snapshots(window))
            .iter()
            .any(|snapshot| snapshot.role() == Some(gpui_kit::Role::Group)
                && snapshot.label() == Some("诊断")),
        "clear directly after log match retains official diagnostics selection"
    );
    assert!(
        !directory.join("config.json").exists(),
        "navigation/search does not create persisted settings"
    );
    view.update_in(&mut surface, |view, _, cx| {
        view.saving = true;
        cx.notify();
    });
    surface.refresh().expect("busy footer");
    let snapshots = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    for id in [
        "download-settings-cancel-button",
        "download-settings-apply-button",
    ] {
        let action = snapshots
            .iter()
            .find(|snapshot| {
                snapshot
                    .path()
                    .iter()
                    .any(|element| element == &gpui_kit::ElementId::from(id))
                    && snapshot.role() == Some(gpui_kit::Role::Button)
            })
            .expect("actual busy Button node");
        // This release snapshot omits disabled; verify actual no-dispatch below.
        assert!(action.bounds().bottom() <= px(650.0));
        surface.simulate_click(action.bounds().center(), Modifiers::default());
        surface.run_until_parked();
        assert!(surface.read(|cx| view.read(cx).saving));
        surface
            .refresh()
            .expect("busy action cannot close the settings window");
    }
    assert!(
        !directory.join("config.json").exists(),
        "busy actions do not submit a second save"
    );
    view.update_in(&mut surface, |view, _, cx| {
        view.saving = false;
        view.error = Some("合成保存失败；保留草稿并重试。".repeat(100));
        cx.notify();
    });
    surface.refresh().expect("bounded long save failure");
    let failure = surface
        .debug_bounds("download-settings-error-scroll")
        .expect("visible bounded error");
    let footer = surface
        .debug_bounds("download-settings-shell-footer")
        .expect("reachable retry footer");
    assert!(failure.size.height <= px(96.0) && failure.bottom() <= footer.top());
    assert_eq!(footer.bottom(), px(650.0));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn queue04_fixed_confirmation_uses_natural_form_labels(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (application, directory, endpoint) = window_fixture(cx);
    endpoint.set_connected_for_contract_test(true);
    let (_, _, id) = stage_fixed_probe(cx, &application);
    fixed_result(cx, &application, id, true);
    let target = cx.read(|cx| {
        application
            .read(cx)
            .new_download_window
            .expect("fixed target")
    });
    let mut surface = VisualTestContext::from_window(target, cx);
    surface.simulate_resize(new_download_window_size(true));
    surface.refresh().expect("production confirmation Form");
    let label = surface
        .debug_bounds("download-new-url-field-label")
        .expect("URL Form label");
    assert_eq!(
        label.size.height,
        px(22.5),
        "official Form text_sm natural label"
    );
    let footer = surface
        .debug_bounds("download-new-footer")
        .expect("unchanged footer");
    assert_eq!(footer.bottom(), px(NEW_DOWNLOAD_CONFIRMED_HEIGHT));
    let view = application.read_with(&surface, |app, _| {
        app.new_download_view.clone().expect("fixed Form owner")
    });
    for selector in [
        "download-new-url-field-input",
        "download-new-directory-input",
        "download-new-file-name-control",
        "download-new-category-trigger",
    ] {
        let bounds = surface
            .debug_bounds(selector)
            .expect("every production field remains mounted");
        assert!(
            bounds.left() >= px(0.0)
                && bounds.right() <= px(560.0)
                && bounds.bottom() <= footer.top()
        );
    }
    let (file_name, url_id, directory_id, search_id) = view.read_with(&surface, |view, _| {
        (
            view.file_name.clone(),
            view.url.entity_id(),
            view.directory.entity_id(),
            view.category_search.entity_id(),
        )
    });
    let file_id = file_name.entity_id();
    surface.update(|window, cx| {
        window.activate_window();
        file_name.read(cx).focus_handle(cx).focus(window, cx);
    });
    surface.simulate_input("中文草稿.bin");
    surface.run_until_parked();
    surface
        .refresh()
        .expect("original Form input edited/repainted");
    view.read_with(&surface, |view, cx| {
        assert_eq!(view.url.entity_id(), url_id);
        assert_eq!(view.directory.entity_id(), directory_id);
        assert_eq!(view.file_name.entity_id(), file_id);
        assert_eq!(view.category_search.entity_id(), search_id);
        assert!(view.file_name.read(cx).value().contains("中文草稿"));
        assert_eq!(
            view.external_request
                .as_ref()
                .expect("original carried request")
                .cookie
                .as_deref(),
            Some("synthetic-cookie")
        );
        assert_eq!(
            view.external_request
                .as_ref()
                .expect("carried headers")
                .headers,
            vec![nxtools_download::DownloadRequestHeader {
                name: "X-Synthetic-Header".into(),
                value: "fixture-value".into()
            }]
        );
    });
    let _ = std::fs::remove_dir_all(directory);
}

struct Queue04HlsFixture;
impl Render for Queue04HlsFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        hls_quality_options(
            Theme::zed_dark(),
            &[nxtools_download::HlsVariantProbe {
                playlist_url: "https://synthetic.invalid/quality".into(),
                bandwidth: 800_000,
                average_bandwidth: None,
                resolution: Some((640, 360)),
                frame_rate_milli: None,
                codecs: "avc1".into(),
                audio_group: None,
                subtitles_group: None,
            }],
            0,
            gpui_kit::WeakEntity::new_invalid(),
        )
    }
}
#[gpui_kit::test]
fn queue04_production_hls_options_are_real_radio_controls(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, surface) = cx.add_window_view(|_, _| Queue04HlsFixture);
    surface.refresh().expect("production HLS choice");
    let nodes = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert_eq!(
        nodes
            .iter()
            .filter(|node| node.role() == Some(gpui_kit::Role::RadioButton)
                && node.checked() == Some(true))
            .count(),
        1,
        "actual styled selected Radio"
    );
}

struct Queue04RadioFixture {
    changes: Rc<RefCell<usize>>,
}
impl Render for Queue04RadioFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let changes = self.changes.clone();
        gpui_kit::component::radio::Radio::new("queue04-selected-radio")
            .checked(true)
            .label("Selected optional track")
            .on_click(move |_, _, _| {})
            .map(|radio| {
                gpui_kit::StatefulInteractiveElement::on_click(radio, move |_, _, _| {
                    *changes.borrow_mut() += 1
                })
            })
    }
}
#[gpui_kit::test]
fn queue04_radio_public_click_seam_handles_selected_optional_track_once(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let changes = Rc::new(RefCell::new(0));
    let retained = changes.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|_| Queue04RadioFixture { changes: retained });
        Root::new(view, window, cx)
    });
    surface
        .refresh()
        .expect("released selected Radio public click seam");
    let bounds = surface
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| node.role() == Some(gpui_kit::Role::RadioButton))
        .expect("real Radio")
        .bounds();
    surface.simulate_click(bounds.center(), Modifiers::default());
    assert_eq!(
        *changes.borrow(),
        1,
        "single selected-control click owner can preserve repeat-to-clear"
    );
    surface.update(|window, _| window.activate_window());
    surface.run_until_parked();
    surface.update(|window, cx| window.focus_next(cx));
    surface.refresh().expect("focused actual Radio frame");
    assert!(
        surface
            .update(|window, _| gpui_kit::base::test_support::snapshots(window))
            .into_iter()
            .any(|node| node.role() == Some(gpui_kit::Role::RadioButton)
                && node.focused() == Some(true))
    );
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("Space"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("Space release"),
    });
    assert_eq!(
        *changes.borrow(),
        2,
        "same official Radio focus/release owner handles keyboard once"
    );
}

struct Queue04CalendarFixture {
    state: Entity<gpui_kit::component::calendar::CalendarState>,
}
impl Render for Queue04CalendarFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .debug_selector(|| "queue04-official-calendar-budget".into())
            .w(px(220.0))
            .child(gpui_kit::component::calendar::Calendar::new(&self.state).small())
    }
}
#[gpui_kit::test]
fn queue04_official_small_calendar_exceeds_existing_inline_date_budget(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, surface) = cx.add_window_view(|window, cx| {
        let state = cx.new(|cx| gpui_kit::component::calendar::CalendarState::new(window, cx));
        Queue04CalendarFixture { state }
    });
    surface.refresh().expect("actual released Small Calendar");
    let bounds = surface
        .debug_bounds("queue04-official-calendar-budget")
        .expect("published Calendar geometry");
    assert!(
        bounds.size.height > px(156.0),
        "existing inline calendar156px leaves original time/actions in240px; released Small={bounds:?}"
    );
}

#[gpui_kit::test]
fn queue05_schedule_mounts_controlled_switches_and_independent_toggles(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (application, directory, _) = window_fixture(cx);
    let slot = Rc::new(RefCell::new(None));
    let retained = slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| DownloadScheduleView::new(application, None, window, cx));
        *retained.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(430.0), px(326.0)));
    surface.refresh().expect("production schedule controls");
    let nodes = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert_eq!(
        nodes
            .iter()
            .filter(|node| node.role() == Some(gpui_kit::Role::Switch))
            .count(),
        2,
        "actual independent start/stop Switches"
    );
    assert_eq!(
        nodes
            .iter()
            .filter(
                |node| node.role() == Some(gpui_kit::Role::Button) && node.checked() == Some(true)
            )
            .count(),
        7,
        "all independent weekday Toggles"
    );
    let view = slot.borrow().clone().expect("production schedule owner");
    surface.update(|window, _| window.activate_window());
    surface.run_until_parked();
    for selector in [
        "download-schedule-start-toggle",
        "download-schedule-stop-toggle",
    ] {
        let bounds = surface.debug_bounds(selector).expect("Switch slot");
        surface.simulate_click(bounds.center(), Modifiers::default());
        surface.refresh().expect("checked controlled Switch");
        assert!(
            view.read_with(surface, |view, _| if selector.contains("start") {
                view.enabled_start
            } else {
                view.enabled_stop
            })
        );
        surface.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: gpui_kit::Keystroke::parse("space").expect("Switch Space"),
            is_held: false,
            prefer_character_input: false,
        });
        surface.simulate_event(gpui_kit::KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse("space").expect("Switch release"),
        });
        surface.refresh().expect("unchecked controlled Switch");
        assert!(
            !view.read_with(surface, |view, _| if selector.contains("start") {
                view.enabled_start
            } else {
                view.enabled_stop
            })
        );
    }
    assert_eq!(
        view.read_with(surface, |view, _| view.days.len()),
        7,
        "start/stop never mutate independent weekdays"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn queue05_task_uses_official_xsmall_aggregate_progress(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (application, directory, _) = window_fixture(cx);
    let (_, view, _) = stage_fixed_probe(cx, &application);
    let request = view.read_with(cx, |view, _| {
        view.confirmed_request.clone().expect("synthetic request")
    });
    let task_id = application.read_with(cx, |app, _| {
        app.engine
            .enqueue(request, RequestOrigin::Internal)
            .expect("queue only")
    });
    application.update(cx, |app, cx| {
        app.refresh(cx);
        app.request_task_window(task_id, cx);
    });
    cx.run_until_parked();
    let handle = application.read_with(cx, |app, _| {
        app.task_windows
            .get(&task_id)
            .expect("keyed production task")
            .window
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(440.0), px(184.0)));
    surface.refresh().expect("production aggregate progress");
    let track = surface
        .debug_bounds("download-window-progress-component")
        .expect("actual Progress container");
    assert_eq!(
        track.size.height,
        px(4.0),
        "released XSmall Progress, not manual5px track"
    );
    let nodes = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert!(
        nodes
            .iter()
            .any(|node| node.role() == Some(gpui_kit::Role::CheckBox)
                && node
                    .path()
                    .contains(&gpui_kit::ElementId::from("download-task-window-toolbar"))),
        "real Checkbox under actual Toolbar"
    );
    assert!(
        nodes
            .iter()
            .any(|node| node.path().contains(&gpui_kit::ElementId::from(
                "download-task-window-action-group"
            ))),
        "original actions under official ToolbarGroup"
    );
    surface.update(|window, _| window.activate_window());
    surface.run_until_parked();
    surface.update(|window, cx| {
        for _ in 0..3 {
            window.focus_next(cx);
        }
    });
    let first = surface
        .update(|window, cx| window.focused(cx))
        .expect("task Toolbar Checkbox focus");
    surface.simulate_keystrokes("right");
    let second = surface
        .update(|window, cx| window.focused(cx))
        .expect("next original task action");
    assert_ne!(first, second);
    surface.simulate_keystrokes("right right");
    assert_eq!(
        surface.update(|window, cx| window.focused(cx)),
        Some(first),
        "three original Toolbar controls wrap"
    );
    let _ = std::fs::remove_dir_all(directory);
}

struct Queue05TimeFixture {
    state: Entity<gpui_kit::component::time_field::TimeFieldState>,
}
impl Render for Queue05TimeFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui_kit::component::time_field::TimeField::new(&self.state)
    }
}
#[gpui_kit::test]
fn queue05_timefield_keeps_a_valid_time_instead_of_raw_invalid_draft(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let slot = Rc::new(RefCell::new(None));
    let retained = slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let state = cx.new(|cx| {
            gpui_kit::component::time_field::TimeFieldState::new(window, cx)
                .hour_cycle(gpui_kit::component::time_field::HourCycle::H23)
        });
        *retained.borrow_mut() = Some(state.clone());
        let view = cx.new(|_| Queue05TimeFixture { state });
        Root::new(view, window, cx)
    });
    surface.refresh().expect("released TimeField");
    let state = slot.borrow().clone().expect("actual time state");
    surface.update(|window, cx| {
        window.activate_window();
        let focus = state.read(cx).focus_handle(cx);
        focus.focus(window, cx);
    });
    surface.run_until_parked();
    surface.refresh().expect("focused TimeField");
    let before = surface.read(|cx| state.read(cx).time());
    surface.simulate_keystrokes("9 9");
    assert_ne!(
        surface.read(|cx| state.read(cx).time()),
        before,
        "actual digit edit, not an inert fixture"
    );
    let formatted = surface.read(|cx| state.read(cx).time().format("%H:%M").to_string());
    assert_ne!(
        formatted, "99:00",
        "released segmented editor cannot retain raw invalid hour for original Apply error"
    );
    assert!(parse_clock_minute(&formatted).is_ok());
    assert!(parse_clock_minute("99:00").is_err());
}

#[gpui_kit::test]
fn feedback_schedule_uses_two_real_official_time_fields(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, _) = window_fixture(cx);
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            let mut view = DownloadScheduleView::new(application, None, window, cx);
            view.enabled_start = true;
            view.enabled_stop = true;
            view
        });
        Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(430.0), px(326.0)));
    surface.refresh().expect("real schedule");
    let nodes = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert_eq!(
        nodes
            .iter()
            .filter(|n| n.role() == Some(gpui_kit::Role::TimeInput))
            .count(),
        2,
        "official TimeFields"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn feedback_download_category_master_uses_real_list_item(cx: &mut TestAppContext) {
    let (_, directory, mut surface) = queue02_settings_window(cx);
    surface.refresh().expect("real categorymaster");
    let nodes = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert!(
        nodes
            .iter()
            .any(|n| n.path().last()
                == Some(&gpui_kit::ElementId::from("download-settings-category-0"))),
        "actualListItem row"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn feedback_timefield_apply_preserves_cross_midnight_and_validation(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, _) = window_fixture(cx);
    let owner = application.clone();
    let slot = Rc::new(RefCell::new(None));
    let retained = slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            let mut view = DownloadScheduleView::new(owner, None, window, cx);
            view.enabled_start = true;
            view.enabled_stop = true;
            view
        });
        *retained.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(430.0), px(326.0)));
    surface.refresh().expect("officialTimeField");
    let view = slot.borrow().clone().expect("view");
    surface.update(|window, cx| {
        window.activate_window();
        let input = view.read(cx).start_time.clone();
        input.update(cx, |state, cx| state.focus(window, cx));
    });
    surface.refresh().expect("actual TimeField focus");
    for key in ["2", "3", "4", "0"] {
        surface.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: gpui_kit::Keystroke::parse(key).expect("time digit"),
            is_held: false,
            prefer_character_input: false,
        });
        surface.simulate_event(gpui_kit::KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse(key).expect("digit release"),
        });
        surface.refresh().expect("real segmented edit frame");
    }
    assert_eq!(
        view.read_with(surface, |v, cx| v.start_time.read(cx).time()),
        NaiveTime::from_hms_opt(23, 40, 0).expect("edited time")
    );
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.start_time.update(cx, |state, cx| {
                state.set_time(
                    NaiveTime::from_hms_opt(23, 40, 0).expect("validstart"),
                    window,
                    cx,
                )
            });
            view.stop_time.update(cx, |state, cx| {
                state.set_time(
                    NaiveTime::from_hms_opt(1, 15, 0).expect("validstop"),
                    window,
                    cx,
                )
            });
            view.days.clear();
            view.save(window, cx);
            assert!(view.error.is_some() && !view.saving);
            view.days.insert(DownloadWeekday::Monday);
            view.save(window, cx);
        })
    });
    surface.run_until_parked();
    let daily = application.read_with(surface, |app, _| {
        app.default_queue()
            .expect("defaultqueue")
            .schedule
            .as_ref()
            .expect("appliedschedule")
            .daily
            .clone()
    });
    assert_eq!((daily.start_minute, daily.stop_minute), (1420, 75));
    assert!(daily.enabled_start && daily.enabled_stop);
    assert_eq!(daily.days, vec![DownloadWeekday::Monday]);
    let _ = std::fs::remove_dir_all(directory);
}

struct FeedbackCalendarSizing {
    state: Entity<gpui_kit::component::calendar::CalendarState>,
    tier: gpui_kit::component::Size,
    padding: f32,
}
impl Render for FeedbackCalendarSizing {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().flex().items_start().child(
            div()
                .debug_selector(|| "feedback-calendar-natural".into())
                .child(
                    gpui_kit::component::calendar::Calendar::new(&self.state)
                        .with_size(self.tier)
                        .p(px(self.padding)),
                ),
        )
    }
}
#[gpui_kit::test]
fn feedback_calendar_public_size_and_padding_measure_natural_geometry(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let mut measurements = Vec::new();
    for (tier, padding) in [
        (gpui_kit::component::Size::Small, 12.0),
        (gpui_kit::component::Size::Small, 0.0),
        (gpui_kit::component::Size::Medium, 0.0),
        (gpui_kit::component::Size::Size(px(20.0)), 0.0),
    ] {
        let (_, surface) = cx.add_window_view(move |window, cx| {
            let state = cx.new(|cx| gpui_kit::component::calendar::CalendarState::new(window, cx));
            FeedbackCalendarSizing {
                state,
                tier,
                padding,
            }
        });
        surface.refresh().expect("real Calendar public composition");
        let bounds = surface
            .debug_bounds("feedback-calendar-natural")
            .expect("natural geometry");
        println!("Calendar {tier:?} padding={padding}: {:?}", bounds.size);
        measurements.push(bounds.size);
    }
    assert!(
        measurements[1].height < measurements[0].height,
        "outer padding is adjustable"
    );
    assert!(
        measurements[1].height > px(156.0),
        "even zero padding cannot fit all date rows in original156px"
    );
    assert_eq!(
        measurements[2], measurements[3],
        "customSize does not resize date cells in release0.7.1"
    );
}

#[gpui_kit::test]
fn balance_download_settings_short_controls_stay_near_their_labels(cx: &mut TestAppContext) {
    let (_, directory, mut surface) = queue02_settings_window(cx);
    select_official_download_page(&mut surface, DownloadSettingsPage::Window);
    let nodes = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    let switch = nodes
        .iter()
        .find(|n| {
            n.role() == Some(gpui_kit::Role::Switch)
                && n.path().last()
                    == Some(&gpui_kit::ElementId::from(
                        "download-settings-progress-window",
                    ))
        })
        .expect("actual progress Switch");
    assert!(
        switch.bounds().right() >= px(900.0),
        "standalone Switch must reach settings right edge: {:?}",
        switch.bounds()
    );
    assert!(
        switch.bounds().size.width <= px(40.0),
        "official Switch cannot stretch"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn download_monitor_footer_controls_are_vertically_centered(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory, endpoint) = window_fixture(cx);
    endpoint.set_connected_for_contract_test(true);
    let task_id = cx.read(|cx| {
        application
            .read(cx)
            .engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/footer-center.bin".into(),
                    save_directory: directory.clone(),
                    save_directory_explicit: false,
                    file_name: Some("footer-center.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(8),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: Some(4096),
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
            )
            .expect("queue synthetic footer task")
    });
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.refresh(cx);
            application.request_task_window(task_id, cx);
        })
    });
    cx.run_until_parked();
    let (handle, view) = cx.read(|cx| {
        let entry = application
            .read(cx)
            .task_windows
            .get(&task_id)
            .expect("keyed window");
        (entry.window, entry.view.clone())
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    for (expanded, downloading) in [(false, false), (true, false), (false, true), (true, true)] {
        surface.update(|_, cx| {
            view.update(cx, |view, cx| {
                if downloading {
                    let task = &mut view.snapshot.as_mut().expect("task snapshot").task;
                    if task.status() == DownloadStatus::Created {
                        for (status, timestamp) in [
                            (DownloadStatus::Probing, 2),
                            (DownloadStatus::Waiting, 3),
                            (DownloadStatus::Downloading, 4),
                        ] {
                            task.transition_to(status, timestamp)
                                .expect("fixture transfer phase");
                        }
                    }
                }
                view.details_expanded = expanded;
                cx.notify();
            })
        });
        surface.simulate_resize(download_task_window_size_for_details(
            DownloadWindowPhase::Progress,
            expanded,
        ));
        surface.refresh().expect("render real monitor footer");
        let footer = surface
            .debug_bounds("download-task-window-footer")
            .expect("fixed footer");
        assert_eq!(footer.size.height, px(44.0));
        for selector in [
            "download-window-shutdown-arm-button",
            if downloading {
                "download-window-pause"
            } else {
                "download-window-resume"
            },
            if downloading {
                "download-window-cancel-active"
            } else {
                "download-window-cancel"
            },
        ] {
            let control = surface
                .debug_bounds(selector)
                .expect("actual footer control");
            assert!(control.top() >= footer.top() && control.bottom() <= footer.bottom());
            // The top divider consumes one logical pixel; allow half-pixel centering.
            assert!(
                (control.center().y - footer.center().y).as_f32().abs() <= 1.0,
                "{selector} must be centered, not merely contained: {control:?} / {footer:?}"
            );
            surface.simulate_mouse_move(control.center(), None, Modifiers::default());
            surface
                .refresh()
                .expect("hover does not move footer control");
            assert_eq!(surface.debug_bounds(selector), Some(control));
        }
    }
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.close_task_window(task_id, cx)
        })
    });
    std::fs::remove_dir_all(directory).expect("remove isolated footer fixture");
}

#[test]
fn download_monitor_capabilities_distinguish_active_connections_from_total_segments() {
    let task_id = DownloadTaskId::new();
    let progress = nxtools_download::DownloadProgress {
        task_id,
        downloaded_bytes: 3072,
        total_bytes: Some(11264),
        bytes_per_second: 1024,
        active_connections: 8,
        segments: (0..11)
            .map(|segment_index| nxtools_download::DownloadSegmentProgress {
                segment_index,
                downloaded_bytes: if segment_index < 3 { 1024 } else { 0 },
                total_bytes: Some(1024),
            })
            .collect(),
    };
    let label = super::task_window::task_window_capabilities(
        "支持续传",
        "不限速",
        DownloadStatus::Downloading,
        Some(&progress),
    );
    assert_eq!(label, "支持续传 · 不限速 · 活动连接 8 · 总分段 11");
    for status in [
        DownloadStatus::Paused,
        DownloadStatus::Cancelled,
        DownloadStatus::Finalizing,
    ] {
        let label = super::task_window::task_window_capabilities(
            "支持续传",
            "不限速",
            status,
            Some(&progress),
        );
        assert_eq!(
            label, "支持续传 · 不限速 · 活动连接 0 · 总分段 11",
            "cached completed segments must never be relabelled active"
        );
    }
    assert_eq!(
        super::task_window::task_window_capabilities(
            "续传待确认",
            "不限速",
            DownloadStatus::Probing,
            None,
        ),
        "续传待确认 · 不限速 · 活动连接 — · 总分段 —"
    );
}

#[gpui_kit::test]
fn followup_probe_spinner_runs_only_for_motion_allowed_probe_and_keeps_compact_footer(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory, _endpoint) = window_fixture(cx);
    application.update(cx, |app, cx| app.open_new_download_window(cx));
    cx.run_until_parked();
    let (handle, view) = application.read_with(cx, |app, _| {
        (
            app.new_download_window.expect("compact owner"),
            app.new_download_view.clone().expect("probe view"),
        )
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.update(|window, cx| window.blur(cx));
    for (probing, handoff_pending, motion, animate) in [
        (false, false, true, false),
        (true, false, false, false),
        (true, false, true, true),
        (true, false, false, false),
        (false, true, true, false),
        (false, false, true, false),
    ] {
        surface.update(|_, cx| {
            cx.set_reduce_motion(!motion);
            cx.set_global(crate::ui::motion::UiMotionPreference::Known {
                reduced_motion: !motion,
            });
        });
        view.update_in(&mut surface, |view, _, cx| {
            view.probing = probing;
            view.handoff_pending = handoff_pending;
            view.error = None;
            cx.notify();
        });
        surface.refresh().expect("production probe guidance");
        surface.run_until_parked();
        surface.update(|window, cx| {
            window.blur(cx);
            window.simulate_next_frame(cx);
            window.refresh();
            window.draw(cx).clear(cx);
        });
        let root = surface
            .debug_bounds("download-new-root")
            .expect("compact root");
        let guidance = surface
            .debug_bounds("download-new-probe-guidance")
            .expect("guidance");
        let footer = surface
            .debug_bounds("download-new-footer")
            .expect("fixed footer");
        let cancel = surface
            .debug_bounds("download-new-cancel-action")
            .expect("cancel");
        assert_eq!(root.size.height, px(176.0));
        assert!(guidance.bottom() <= footer.top() && cancel.bottom() <= footer.bottom());
        let frames = surface.update(|window, cx| window.simulate_next_frame(cx));
        assert_eq!(
            frames > 0,
            animate,
            "only current pending Spinner requests frames: probing={probing},handoff={handoff_pending},motion={motion}"
        );
        if animate {
            let spinner = surface
                .debug_bounds("download-new-probe-spinner")
                .expect("real XSmall Spinner slot");
            assert_eq!(spinner.size.width, px(12.0));
            assert!(spinner.top() >= guidance.top() && spinner.bottom() <= guidance.bottom());
        } else {
            assert!(surface.debug_bounds("download-new-probe-spinner").is_none());
        }
    }
    view.update_in(&mut surface, |view, _, cx| {
        view.error = Some("合成探测错误，保留原始重试边界".into());
        cx.notify();
    });
    surface.refresh().expect("compact probe error fallback");
    assert!(surface.debug_bounds("download-new-probe-spinner").is_none());
    assert_eq!(
        surface
            .debug_bounds("download-new-root")
            .expect("error root")
            .size
            .height,
        px(176.0)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn followup_ffmpeg_progress_paints_existing_percent_only_for_known_download_total(
    cx: &mut TestAppContext,
) {
    let (application, directory, mut surface) = queue02_settings_window(cx);
    surface.update(|_, cx| cx.set_reduce_motion(true));
    let view = application.read_with(&surface, |app, _| {
        app.settings_view.clone().expect("settings view")
    });
    select_official_download_page(&mut surface, DownloadSettingsPage::Transfer);
    let search = surface
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| {
            node.role() == Some(gpui_kit::Role::TextInput)
                && node
                    .path()
                    .contains(&gpui_kit::ElementId::from("settings-sidebar"))
        })
        .expect("official settings search")
        .bounds();
    surface.simulate_click(search.center(), Modifiers::default());
    surface.simulate_input("自动安装");
    surface.run_until_parked();
    for (phase, total, percent, cancelling, visible) in [
        (FfmpegInstallPhase::Downloading, Some(1000), 37, false, true),
        (FfmpegInstallPhase::Downloading, Some(1000), 37, true, true),
        (FfmpegInstallPhase::Downloading, None, 0, false, false),
        (FfmpegInstallPhase::Downloading, Some(0), 0, false, false),
        (
            FfmpegInstallPhase::Downloading,
            Some(1000),
            101,
            false,
            false,
        ),
        (FfmpegInstallPhase::Extracting, Some(1000), 80, false, false),
        (FfmpegInstallPhase::Connecting, None, 0, false, false),
    ] {
        view.update_in(&mut surface, |view, _, cx| {
            view.ffmpeg_install_state = FfmpegInstallViewState::Installing {
                phase,
                percent,
                downloaded_bytes: 529,
                total_bytes: total,
                cancelling,
            };
            cx.notify();
        });
        surface.refresh().expect("real install projection");
        let progress = surface.debug_bounds("download-settings-ffmpeg-progress");
        assert_eq!(
            progress.is_some(),
            visible,
            "unknown totals and phase milestones must not claim measured progress"
        );
        let status = surface
            .debug_bounds("download-settings-ffmpeg-install-status")
            .expect("original phase/bytes copy");
        let cancel = surface
            .debug_bounds("download-settings-ffmpeg-cancel")
            .expect("original cancel action");
        let footer = surface
            .debug_bounds("download-settings-shell-footer")
            .expect("fixed settings footer");
        assert!(status.bottom() <= cancel.top() && cancel.bottom() <= footer.top());
        assert_eq!(footer.bottom(), px(650.0));
        if let Some(progress) = progress {
            assert_eq!(progress.size.height, px(4.0), "released XSmall Progress");
            assert!(progress.top() >= status.bottom() && progress.bottom() <= cancel.top());
            let (quads, scale) =
                surface.update(|window, _| (window.painted_quads(), window.scale_factor()));
            let expected = progress.size.width.as_f32() * 0.37;
            assert!(
                quads.iter().any(|quad| {
                    (quad.bounds.size.height.as_f32() / scale - 4.0).abs() < 0.1
                        && (quad.bounds.size.width.as_f32() / scale - expected).abs() < 1.0
                        && (quad.bounds.origin.y.as_f32() / scale - progress.top().as_f32()).abs()
                            < 1.0
                }),
                "paint uses supplied37%, not recomputed52.9% or a synthetic phase value"
            );
        }
    }
    for terminal in [
        FfmpegInstallViewState::Idle,
        FfmpegInstallViewState::Completed,
        FfmpegInstallViewState::Cancelled,
        FfmpegInstallViewState::Failed("合成安装失败".into()),
    ] {
        view.update_in(&mut surface, |view, _, cx| {
            view.ffmpeg_install_state = terminal;
            cx.notify();
        });
        surface.refresh().expect("terminal install state");
        assert!(
            surface
                .debug_bounds("download-settings-ffmpeg-progress")
                .is_none()
        );
        assert!(
            surface
                .debug_bounds("download-settings-ffmpeg-cancel")
                .is_none()
        );
        assert!(
            surface
                .debug_bounds("download-settings-ffmpeg-install-status")
                .is_some()
        );
    }
    assert!(
        !directory.join("config.json").exists(),
        "rendering progress cannot save a path draft"
    );
    let _ = std::fs::remove_dir_all(directory);
}
