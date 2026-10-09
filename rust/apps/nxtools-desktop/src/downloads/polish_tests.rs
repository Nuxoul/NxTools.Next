//! Production-window geometry gates for the Downloads presentation pass.
use super::*;
use gpui::{AppContext, Modifiers, TestAppContext, VisualTestContext};

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

#[gpui::test]
fn polished_new_download_preserves_compact_footer_and_confirmed_budget(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
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
    assert!(title.bottom() < guidance.top());
    assert!(guidance.bottom() <= footer.top());
    assert!(cancel.top() >= footer.top() && cancel.bottom() <= footer.bottom());
    assert_eq!(footer.bottom(), root.bottom());
    assert_eq!(new_download_window_size(true).height, px(278.0));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui::test]
fn polished_settings_navigation_uses_actual_selected_hit_bounds(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
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
    surface
        .refresh()
        .expect("render production settings navigation");
    let item = surface
        .debug_bounds("download-settings-nav-general")
        .expect("navigation hit target");
    let title = surface
        .debug_bounds("download-settings-nav-general-title")
        .expect("navigation title");
    let description = surface
        .debug_bounds("download-settings-nav-general-description")
        .expect("navigation description");
    assert!(item.size.height.as_f32() >= 48.0);
    assert!(title.top() >= item.top() && description.bottom() <= item.bottom());
    assert!(title.bottom() <= description.top());
    surface.simulate_click(item.center(), Modifiers::default());
    surface.refresh().expect("paint navigation trigger focus");
    surface.simulate_keystrokes("tab");
    surface
        .refresh()
        .expect("paint next settings navigation focus");
    surface.simulate_keystrokes("enter");
    surface.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").expect("settings navigation release"),
    });
    let settings = cx.read(|cx| {
        application
            .read(cx)
            .settings_view
            .clone()
            .expect("actual settings view")
    });
    assert!(
        settings.read_with(cx, |view, _| view.page == DownloadSettingsPage::Request),
        "keyboard must select request settings through the same navigation action"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui::test]
fn polished_schedule_weekdays_fit_actual_dialog_and_footer(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
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
    assert_eq!(first_day.size.height, px(CONTROL_HEIGHT_ACTION));
    assert_eq!(first_day.size.width, px(40.0));
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

#[gpui::test]
fn polished_schedule_keyboard_changes_the_actual_weekday_state(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
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
    surface.update(|window, _| {
        window.focus_next();
        window.focus_next();
    });
    surface
        .refresh()
        .expect("focus first weekday after caption close");
    let view = slot.borrow().clone().expect("actual schedule entity");
    let day = DownloadWeekday::ALL[0];
    assert!(view.read_with(surface, |view, _| view.days.contains(&day)));
    surface.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse("space").expect("weekday Space"),
        is_held: false,
    });
    surface.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("space").expect("weekday release"),
    });
    assert!(
        !view.read_with(surface, |view, _| view.days.contains(&day)),
        "Space must change real weekday selection"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui::test]
fn polished_calendar_eligible_keyup_and_disabled_past_preserve_draft_and_bounds(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_component::init);
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
    surface.update(|window, _| {
        window.blur();
        for _ in 0..4 {
            window.focus_next();
        }
    });
    surface
        .refresh()
        .expect("Tab skips disabled past dates and reaches next date");
    surface.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse("space").expect("calendar Space"),
        is_held: false,
    });
    assert_eq!(
        surface.read(|cx| view.read(cx).scheduled_date),
        initial,
        "keydown cannot commit draft"
    );
    surface.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("space").expect("calendar release"),
    });
    surface.run_until_parked();
    assert_eq!(
        surface.read(|cx| view.read(cx).scheduled_date),
        initial + ChronoDuration::days(1)
    );
    assert!(surface.read(|cx| view.read(cx).committed_schedule.is_none()));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui::test]
fn polished_progress_disclosure_keyup_reuses_the_task_window(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
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
                    file_name: Some("disclosure.bin".into()),
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
    surface.refresh().expect("render actual progress window");
    let toggle = surface
        .debug_bounds("download-window-details-toggle")
        .expect("disclosure button");
    let body = surface
        .debug_bounds("download-task-window-body")
        .expect("progress body");
    assert_eq!(toggle.size, size(px(20.0), px(20.0)));
    assert!(toggle.top() >= body.top() && toggle.bottom() <= body.bottom());
    surface.update(|window, _| {
        window.focus_next();
        window.focus_next();
    });
    surface
        .refresh()
        .expect("Tab after caption close reaches actual disclosure");
    surface.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse("enter").expect("disclosure Enter"),
        is_held: false,
    });
    assert!(
        !cx.read(|cx| task_view.read(cx).details_expanded),
        "activation waits for KeyUp"
    );
    surface.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").expect("disclosure release"),
    });
    surface.run_until_parked();
    assert!(cx.read(|cx| task_view.read(cx).details_expanded));
    assert_eq!(cx.read(|cx| application.read(cx).task_windows.len()), 1);
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui::test]
fn successful_probe_replaces_compact_window_with_fixed_confirmation(cx: &mut TestAppContext) {
    use std::io::{Read as _, Write as _};
    cx.update(gpui_component::init);
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
                    input.set_value(format!("http://{address}/fixture.bin"), window, cx)
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
    assert_eq!(root.size.height, px(278.0));
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

#[gpui::test]
fn fixed_confirmation_failure_disconnect_timeout_retain_probe_and_retry_without_network(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_component::init);
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
        assert_eq!(root.size.height, px(278.0));
        assert_eq!(footer.bottom(), root.bottom());
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[gpui::test]
fn fixed_confirmation_close_and_stale_generation_never_resurrect_source(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
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

#[gpui::test]
fn pending_fixed_handoff_locks_url_and_paste_until_terminal(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let (application, directory, _endpoint) = window_fixture(cx);
    let (source, view, id) = stage_fixed_probe(cx, &application);
    let mut surface = VisualTestContext::from_window(source, cx);
    surface.refresh().expect("pending compact source");
    let before = view.read_with(&surface, |view, cx| view.url.read(cx).value().to_string());
    view.update_in(&mut surface, |view, window, cx| {
        view.url.focus_handle(cx).focus(window)
    });
    surface.simulate_input("changed");
    assert_eq!(
        view.read_with(&surface, |view, cx| view.url.read(cx).value().to_string()),
        before,
        "pending native handoff must not accept URL edits"
    );
    surface.update(|_, cx| {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
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
        view.url.focus_handle(cx).focus(window)
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

#[gpui::test]
fn collapsed_task_completion_changes_content_without_native_resize(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
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
