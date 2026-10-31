//! Auxiliary download renders must read the active palette without replacing drafts.
use super::*;
use gpui_kit::{AppContext, TestAppContext};

fn application(cx: &mut TestAppContext, directory: &Path) -> Entity<DownloadApplication> {
    let (bridge, _endpoint) = nxtools_ipc::platform_bridge(16);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    cx.update(|cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                DownloadEngine::open_in_memory().expect("isolated engine"),
                Rc::new(bridge),
                notifications,
                DownloadRuntimeContext {
                    fallback_default_directory: directory.into(),
                    config: AppConfig::default(),
                    config_path: directory.join("config.json"),
                    log_directory: directory.into(),
                    database_path: directory.join("downloads.sqlite3"),
                    ffmpeg_install_directory: directory.join("ffmpeg"),
                },
                cx,
            )
        })
    })
}

fn light(cx: &mut App) {
    crate::theme::set_theme_preference(nxtools_core::ThemePreference::Light, cx);
}

fn paints_canvas(window: &mut gpui_kit::VisualTestContext) {
    assert!(
        window
            .update(|window, _| window.painted_quads().iter().any(|quad| {
                quad.background == gpui_kit::solid_background(Theme::light().canvas)
            })),
        "custom production shell must paint the active light palette"
    );
}

#[gpui_kit::test]
fn auxiliary_download_schedule_rethemes_original_time_entities(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let directory = tempfile::tempdir().expect("isolated schedule");
    let app = application(cx, directory.path());
    let (view, window) =
        cx.add_window_view(|window, cx| DownloadScheduleView::new(app, None, window, cx));
    window.simulate_resize(size(px(520.0), px(320.0)));
    window.refresh().expect("dark schedule");
    let inputs = view.read_with(window, |view, _| {
        (view.start_time.entity_id(), view.stop_time.entity_id())
    });
    view.update_in(window, |_, _, cx| light(cx));
    window.run_until_parked();
    window.refresh().expect("light schedule");
    paints_canvas(window);
    assert_eq!(
        view.read_with(window, |view, _| (
            view.start_time.entity_id(),
            view.stop_time.entity_id()
        )),
        inputs
    );
    assert!(!view.read_with(window, |view, _| view.saving));
}

#[gpui_kit::test]
fn auxiliary_new_download_rethemes_without_replacing_url_or_directory(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let directory = tempfile::tempdir().expect("isolated new download");
    let app = application(cx, directory.path());
    let (view, window) = cx.add_window_view(|window, cx| {
        NewDownloadView::new(
            NewDownloadViewInit {
                application: app,
                default_directory: directory.path().into(),
                default_connection_limit: 4,
                default_adaptive_segment_policy: Default::default(),
                categories: Vec::new(),
                initial_source: None,
            },
            window,
            cx,
        )
    });
    window.simulate_resize(new_download_window_size(false));
    window.refresh().expect("dark new download");
    let inputs = view.read_with(window, |view, _| {
        (view.url.entity_id(), view.directory.entity_id())
    });
    view.update_in(window, |_, _, cx| light(cx));
    window.run_until_parked();
    window.refresh().expect("light new download");
    paints_canvas(window);
    assert_eq!(
        view.read_with(window, |view, _| (
            view.url.entity_id(),
            view.directory.entity_id()
        )),
        inputs
    );
    assert!(!view.read_with(window, |view, _| view.submitting));
}

#[gpui_kit::test]
fn auxiliary_delete_confirmation_rethemes_without_changing_target(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let directory = tempfile::tempdir().expect("isolated confirmation");
    let app = application(cx, directory.path());
    let task_id = DownloadTaskId::new();
    let (view, window) = cx.add_window_view(|_, _| DeleteDownloadView {
        theme: Theme::zed_dark(),
        application: app,
        task_id,
        path: "synthetic.bin".into(),
    });
    window.simulate_resize(size(px(440.0), px(200.0)));
    window.refresh().expect("dark delete confirmation");
    view.update_in(window, |_, _, cx| light(cx));
    window.run_until_parked();
    window.refresh().expect("light delete confirmation");
    paints_canvas(window);
    assert_eq!(view.read_with(window, |view, _| view.task_id), task_id);
    assert_eq!(
        view.read_with(window, |view, _| view.path.clone()),
        "synthetic.bin"
    );
}

#[gpui_kit::test]
fn auxiliary_shutdown_rethemes_without_changing_countdown_identity(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let directory = tempfile::tempdir().expect("isolated countdown");
    let app = application(cx, directory.path());
    let correlation = uuid::Uuid::new_v4();
    let (view, window) =
        cx.add_window_view(|_, _| DownloadShutdownCountdownView::new(app, correlation, 30));
    window.simulate_resize(size(px(440.0), px(240.0)));
    window.refresh().expect("dark countdown");
    view.update_in(window, |_, _, cx| light(cx));
    window.run_until_parked();
    window.refresh().expect("light countdown");
    paints_canvas(window);
    assert_eq!(
        view.read_with(window, |view, _| view.correlation_id),
        correlation
    );
    assert_eq!(view.read_with(window, |view, _| view.remaining_seconds), 30);
    assert!(!view.read_with(window, |view, _| view.executing));
}

#[gpui_kit::test]
fn auxiliary_task_window_rethemes_without_changing_task_generation(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let directory = tempfile::tempdir().expect("isolated task window");
    let app = application(cx, directory.path());
    let (bridge, _endpoint) = nxtools_ipc::platform_bridge(16);
    let task_id = DownloadTaskId::new();
    let generation = uuid::Uuid::new_v4();
    let (view, window) = cx.add_window_view(|_, _| {
        DownloadTaskWindowView::new(
            app,
            Rc::new(bridge),
            task_id,
            TaskWindowIdentity {
                native_window_handle: 0,
                generation,
            },
            None,
            None,
            false,
        )
    });
    window.simulate_resize(size(px(440.0), px(184.0)));
    window.refresh().expect("dark removed task state");
    view.update_in(window, |_, _, cx| light(cx));
    window.run_until_parked();
    window.refresh().expect("light removed task state");
    paints_canvas(window);
    assert_eq!(view.read_with(window, |view, _| view.task_id), task_id);
    assert_eq!(
        view.read_with(window, |view, _| view.window_generation),
        generation
    );
    assert!(!view.read_with(window, |view, _| view.details_expanded));
}

#[gpui_kit::test]
fn auxiliary_download_settings_rethemes_without_replacing_draft_inputs(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let directory = tempfile::tempdir().expect("isolated settings");
    let app = application(cx, directory.path());
    let initial = app.read_with(cx, |app, _| app.settings_view_initial());
    let (view, window) =
        cx.add_window_view(|window, cx| DownloadSettingsView::new(app, initial, window, cx));
    window.simulate_resize(size(px(720.0), px(520.0)));
    window.refresh().expect("dark download settings");
    let inputs = view.read_with(window, |view, _| {
        (view.directory.entity_id(), view.default_cookie.entity_id())
    });
    view.update_in(window, |_, _, cx| light(cx));
    window.run_until_parked();
    window.refresh().expect("light download settings");
    assert_eq!(
        view.read_with(window, |view, _| view.theme.canvas),
        Theme::light().canvas
    );
    assert_eq!(
        view.read_with(window, |view, _| (
            view.directory.entity_id(),
            view.default_cookie.entity_id()
        )),
        inputs
    );
    assert!(!view.read_with(window, |view, _| view.saving));
}
