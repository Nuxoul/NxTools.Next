use super::*;
use gpui_kit::base::test_support;
use gpui_kit::{Modifiers, TestAppContext, VisualTestContext};

struct KeyboardFixture {
    application: Entity<DownloadApplication>,
    view: Entity<NewDownloadView>,
    directory: PathBuf,
    _endpoint: nxtools_ipc::PlatformServerEndpoint,
}

fn fixture(cx: &mut TestAppContext, confirmed: bool) -> (KeyboardFixture, VisualTestContext) {
    cx.update(gpui_kit::init);
    let directory = std::env::temp_dir().join(format!("download-enter-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).expect("create isolated keyboard fixture");
    let (platform, endpoint) = nxtools_ipc::platform_bridge(16);
    let application = cx.update(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        cx.new(|cx| {
            DownloadApplication::new(
                DownloadEngine::open_in_memory().expect("in-memory keyboard fixture engine"),
                Rc::new(platform),
                notifications,
                DownloadRuntimeContext {
                    fallback_default_directory: directory.clone(),
                    config: AppConfig::default(),
                    config_path: directory.join("config.json"),
                    log_directory: directory.clone(),
                    database_path: directory.join("downloads.sqlite3"),
                    ffmpeg_install_directory: directory.join("tools"),
                },
                cx,
            )
        })
    });
    let app = application.clone();
    let save_directory = directory.clone();
    let slot = Rc::new(std::cell::RefCell::new(None));
    let opened = slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        window.resize(new_download_window_size(confirmed));
        let view = cx.new(|cx| {
            NewDownloadView::new(
                NewDownloadViewInit {
                    application: app,
                    default_directory: save_directory,
                    default_connection_limit: 4,
                    default_adaptive_segment_policy: Default::default(),
                    categories: DownloadSettings::default().categories,
                    initial_source: None,
                },
                window,
                cx,
            )
        });
        *opened.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let mut surface = VisualTestContext::from_window(handle, surface);
    let view = slot.borrow().clone().expect("actual production view");
    if confirmed {
        view.update_in(&mut surface, |view, window, cx| {
            view.url.update(cx, |input, cx| {
                input.set_value("https://example.test/keyboard.bin", window, cx);
            });
            view.file_name.update(cx, |input, cx| {
                input.set_value("keyboard.bin", window, cx);
            });
            view.confirmed_request = Some(view.request(cx).expect("synthetic confirmed request"));
            view.probe = Some(DownloadProbe {
                resolution: None,
                final_url: "https://example.test/keyboard.bin".into(),
                file_name: Some("keyboard.bin".into()),
                total_bytes: Some(1024),
                supports_ranges: true,
                content_type: None,
                validator: Default::default(),
                media: None,
            });
            // Validation is a synchronous observable submit boundary, without network or tasks.
            view.directory
                .update(cx, |input, cx| input.set_value("", window, cx));
            cx.notify();
        });
    }
    surface
        .refresh()
        .expect("draw actual production keyboard surface");
    (
        KeyboardFixture {
            application,
            view,
            directory,
            _endpoint: endpoint,
        },
        surface,
    )
}

impl Drop for KeyboardFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn down(surface: &mut VisualTestContext, key: &str, held: bool, character: bool) {
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse(key).expect("keyboard fixture key"),
        is_held: held,
        prefer_character_input: character,
    });
}

fn up(surface: &mut VisualTestContext, key: &str) {
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse(key).expect("keyboard fixture release"),
    });
}

fn focus_input(f: &KeyboardFixture, surface: &mut VisualTestContext, input: usize) {
    f.view.update_in(surface, |view, window, cx| {
        [&view.url, &view.directory, &view.file_name][input]
            .focus_handle(cx)
            .focus(window, cx);
    });
    surface.refresh().expect("draw actual input focus path");
}

fn focus_button(surface: &mut VisualTestContext, id: &'static str) {
    surface.update(|window, cx| window.blur(cx));
    for _ in 0..100 {
        surface.update(|window, cx| window.focus_next(cx));
        surface.refresh().expect("paint actual Tab focus");
        if surface.update(|window, _| {
            test_support::snapshots(window).iter().any(|node| {
                node.path().last() == Some(&gpui_kit::ElementId::from(id))
                    && node.focused() == Some(true)
            })
        }) {
            return;
        }
    }
    panic!("production button {id} was not reachable by Tab");
}

fn assert_no_outer_action(f: &KeyboardFixture, surface: &VisualTestContext) {
    f.view.read_with(surface, |view, _| {
        assert!(
            view.error.is_none(),
            "child/guarded Enter must not enter outer validation: {:?}",
            view.error
        );
        assert!(!view.probing && !view.submitting, "no outer probe/submit");
    });
    f.application
        .read_with(surface, |app, _| assert!(app.tasks().is_empty()));
}

#[gpui_kit::test]
fn new_download_keyboard_schedule_layer_and_select_keep_enter(cx: &mut TestAppContext) {
    let (f, mut surface) = fixture(cx, true);
    focus_input(&f, &mut surface, 2);
    let trigger = surface
        .debug_bounds("download-new-schedule-control")
        .expect("schedule trigger");
    surface.simulate_click(trigger.center(), Modifiers::default());
    surface.run_until_parked();
    surface.refresh().expect("actual schedule layer");
    assert!(
        f.view
            .read_with(&surface, |view, _| view.schedule_popover_open)
    );
    // Even a retained outer input focus cannot submit behind an active child layer.
    focus_input(&f, &mut surface, 2);
    down(&mut surface, "enter", false, false);
    up(&mut surface, "enter");
    assert_no_outer_action(&f, &surface);
    // Open the real official Select through its own focus and key binding.
    let hour = f
        .view
        .read_with(&surface, |view, _| view.scheduled_hour.clone());
    let previous_hour = surface.read(|cx| hour.read(cx).selected_index(cx));
    surface.update(|window, cx| hour.update(cx, |state, cx| state.focus(window, cx)));
    surface.refresh().expect("hour focus");
    surface.simulate_keystrokes("down down enter");
    surface.run_until_parked();
    assert_no_outer_action(&f, &surface);
    assert_ne!(
        surface.read(|cx| hour.read(cx).selected_index(cx)),
        previous_hour,
        "official Select keyboard confirmation changes the hour draft"
    );
    f.view.read_with(&surface, |view, _| {
        assert!(
            view.schedule_popover_open,
            "Select confirmation must not close the schedule"
        );
        assert!(
            view.committed_schedule.is_none(),
            "Select only changes the schedule draft"
        );
    });
}

#[gpui_kit::test]
fn new_download_keyboard_schedule_confirm_clear_and_dismiss_keep_owners(cx: &mut TestAppContext) {
    let (f, mut surface) = fixture(cx, true);
    let trigger = surface
        .debug_bounds("download-new-schedule-control")
        .expect("schedule trigger");
    surface.simulate_click(trigger.center(), Modifiers::default());
    surface.run_until_parked();
    surface.refresh().expect("schedule layer");
    focus_button(&mut surface, "download-new-schedule-clear");
    down(&mut surface, "space", false, false);
    up(&mut surface, "space");
    surface.run_until_parked();
    surface.refresh().expect("keyboard-cleared layer");
    assert_no_outer_action(&f, &surface);
    surface.simulate_click(trigger.center(), Modifiers::default());
    surface.run_until_parked();
    surface.refresh().expect("reopened schedule");
    focus_button(&mut surface, "download-new-schedule-confirm-button");
    down(&mut surface, "enter", false, false);
    up(&mut surface, "enter");
    surface.run_until_parked();
    assert_no_outer_action(&f, &surface);
    assert!(
        f.view
            .read_with(&surface, |view, _| view.committed_schedule.is_some())
    );
    assert!(
        !f.view
            .read_with(&surface, |view, _| view.schedule_popover_open)
    );
    // Dismissal discards a new date draft and preserves the committed schedule.
    let committed = f
        .view
        .read_with(&surface, |view, _| view.committed_schedule);
    surface.refresh().expect("confirmed schedule label");
    surface.simulate_click(trigger.center(), Modifiers::default());
    surface.run_until_parked();
    surface.refresh().expect("open editable schedule draft");
    f.view.update_in(&mut surface, |view, _, cx| {
        view.scheduled_date += ChronoDuration::days(1);
        cx.notify();
    });
    surface.simulate_keystrokes("escape");
    surface.run_until_parked();
    assert_eq!(
        f.view
            .read_with(&surface, |view, _| view.committed_schedule),
        committed
    );
    assert!(
        !f.view
            .read_with(&surface, |view, _| view.schedule_popover_open)
    );
    assert_no_outer_action(&f, &surface);
}

#[gpui_kit::test]
fn new_download_keyboard_outer_inputs_keep_default_and_guard_repeats(cx: &mut TestAppContext) {
    let (f, mut surface) = fixture(cx, true);
    for input in 0..3 {
        focus_input(&f, &mut surface, input);
        for (key, held, character) in [
            ("ctrl-enter", false, false),
            ("shift-enter", false, false),
            ("enter", true, false),
            ("enter", false, true),
        ] {
            down(&mut surface, key, held, character);
            up(&mut surface, key);
            assert_no_outer_action(&f, &surface);
        }
        let before = f
            .application
            .read_with(&surface, |app, cx| app.notifications.read(cx).items().len());
        down(&mut surface, "enter", false, false);
        surface.run_until_parked();
        assert!(f.view.read_with(&surface, |view, _| view.error.is_none()));
        assert_eq!(
            f.application
                .read_with(&surface, |app, cx| app.notifications.read(cx).items().len()),
            before + 1,
            "Enter reaches shared validation notice"
        );
        // Same held press cannot submit or notify twice.
        down(&mut surface, "enter", false, false);
        surface.run_until_parked();
        assert_eq!(
            f.application
                .read_with(&surface, |app, cx| app.notifications.read(cx).items().len()),
            before + 1
        );
        assert_no_outer_action(&f, &surface);
        up(&mut surface, "enter");
    }
}

#[gpui_kit::test]
fn new_download_keyboard_ime_busy_and_native_gate_do_not_submit(cx: &mut TestAppContext) {
    let (f, mut surface) = fixture(cx, true);
    focus_input(&f, &mut surface, 2);
    f.view.update_in(&mut surface, |view, window, cx| {
        view.file_name.update(cx, |input, cx| {
            input.replace_and_mark_text_in_range(None, "中", Some(1..1), window, cx);
        });
    });
    down(&mut surface, "enter", false, false);
    up(&mut surface, "enter");
    assert_no_outer_action(&f, &surface);
    f.view.update_in(&mut surface, |view, window, cx| {
        view.file_name
            .update(cx, |input, cx| input.unmark_text(window, cx));
    });
    for gate in 0..4 {
        f.view.update_in(&mut surface, |view, window, cx| {
            view.probing = gate == 0;
            view.submitting = gate == 1;
            view.handoff_pending = gate == 2;
            view.handoff_staged = gate == 2;
            view.confirmed_target_height =
                (gate == 3).then_some(window.viewport_size().height + px(100.0));
            if gate == 3 {
                assert!(
                    view.confirmed_window_pending(window),
                    "exercise actual native gate"
                );
            }
            cx.notify();
        });
        surface.refresh().expect("draw guarded surface");
        down(&mut surface, "enter", false, false);
        up(&mut surface, "enter");
        f.view.read_with(&surface, |view, _| {
            assert!(view.error.is_none(), "gate {gate} must not submit")
        });
    }
}

#[gpui_kit::test]
fn new_download_keyboard_focused_cancel_does_not_probe(cx: &mut TestAppContext) {
    let (f, mut surface) = fixture(cx, false);
    focus_button(&mut surface, "download-new-cancel");
    down(&mut surface, "enter", false, false);
    assert_no_outer_action(&f, &surface);
    up(&mut surface, "enter");
}

#[gpui_kit::test]
fn new_download_keyboard_compact_input_enter_keeps_probe_validation(cx: &mut TestAppContext) {
    let (f, mut surface) = fixture(cx, false);
    focus_input(&f, &mut surface, 0);
    down(&mut surface, "enter", false, false);
    up(&mut surface, "enter");
    surface.run_until_parked();
    assert!(f.view.read_with(&surface, |view, _| view.error.is_none()));
    assert_eq!(
        f.application
            .read_with(&surface, |app, cx| app.notifications.read(cx).items().len()),
        1,
        "URL Enter reaches original validation through shared notice"
    );
    assert_no_outer_action(&f, &surface);
}

#[gpui_kit::test]
fn new_download_keyboard_category_confirm_does_not_fall_through(cx: &mut TestAppContext) {
    let (f, mut surface) = fixture(cx, true);
    let trigger = surface
        .debug_bounds("download-new-category-trigger")
        .expect("category trigger");
    surface.simulate_click(trigger.center(), Modifiers::default());
    surface.run_until_parked();
    surface.refresh().expect("actual category layer");
    assert!(
        f.view
            .read_with(&surface, |view, _| view.category_popover_open)
    );
    surface.simulate_keystrokes("down enter");
    surface.run_until_parked();
    assert_no_outer_action(&f, &surface);
    assert!(
        !f.view
            .read_with(&surface, |view, _| view.category_popover_open)
    );
}

#[gpui_kit::test]
fn new_download_keyboard_outer_enter_submits_one_committed_future_task(cx: &mut TestAppContext) {
    let (f, mut surface) = fixture(cx, true);
    let future = Local::now() + ChronoDuration::days(2);
    let committed = DelayedStartSelection {
        date: future.date_naive(),
        hour: future.hour(),
        minute: future.minute(),
    };
    f.view.update_in(&mut surface, |view, window, cx| {
        view.directory.update(cx, |input, cx| {
            input.set_value(f.directory.to_string_lossy().into_owned(), window, cx);
        });
        view.committed_schedule = Some(committed);
        // A discarded draft must never replace the confirmed schedule.
        view.scheduled_date = future.date_naive() + ChronoDuration::days(5);
    });
    focus_input(&f, &mut surface, 2);
    down(&mut surface, "enter", false, false);
    surface.run_until_parked();
    f.application.read_with(&surface, |app, _| {
        assert_eq!(
            app.tasks().len(),
            1,
            "one keyboard gesture submits exactly once"
        );
        let task = app
            .engine
            .task(app.tasks()[0].task.id())
            .expect("read full accepted task")
            .expect("accepted task exists");
        assert_eq!(
            task.task.request().scheduled_at_unix_ms,
            resolve_scheduled_start(Some(committed), Local::now().timestamp_millis())
                .expect("future schedule")
        );
        assert!(
            task.scheduled && !task.queue_eligible,
            "future task stays outside transfer admission"
        );
    });
}
