//! Production scene regressions for live auxiliary themes and taskbar Root alpha.
use super::*;
use gpui_kit::{AppContext, TestAppContext, size};

fn assert_paints_color(window: &mut gpui_kit::VisualTestContext, color: gpui_kit::Rgba) {
    let expected = gpui_kit::solid_background(color);
    assert!(
        window.update(|window, _| window
            .painted_quads()
            .iter()
            .any(|quad| { quad.background == expected })),
        "production scene must paint active business palette {expected:?}"
    );
}

#[gpui_kit::test]
fn auxiliary_link_panel_rethemes_while_preserving_expansion_and_window(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (view, window) = cx.add_window_view(|_, _| crate::tools::link_panel::LinkPanelView {
        links: Vec::new(),
        application: gpui_kit::WeakEntity::new_invalid(),
        expanded: true,
        auto_hide_ms: 6000,
        window_handle: None,
        auto_hide_task: None,
    });
    window.simulate_resize(size(px(420.0), px(60.0)));
    window.refresh().expect("dark production link panel");
    let identity = window.update(|window, _| window.window_handle());
    assert_paints_color(window, Theme::zed_dark().surface);
    view.update_in(window, |_, _, cx| {
        crate::theme::set_theme_preference(nxtools_core::ThemePreference::System, cx);
        crate::theme::apply_system_theme(false, cx);
    });
    window.run_until_parked();
    window.refresh().expect("System light link panel");
    assert_paints_color(window, Theme::light().surface);
    assert!(view.read_with(window, |view, _| view.expanded));
    assert_eq!(window.update(|window, _| window.window_handle()), identity);
}

#[gpui_kit::test]
fn taskbar_production_root_does_not_paint_an_opaque_external_client(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let directory = tempfile::tempdir().expect("isolated overlay fixture");
    let (bridge, _endpoint) = nxtools_ipc::platform_bridge(16);
    let bridge = Rc::new(bridge);
    let notifications =
        cx.update(|cx| cx.new(|_| crate::notifications::NotificationCenter::default()));
    let application = cx.update(|cx| {
        cx.new(|cx| {
            crate::downloads::DownloadApplication::new(
                nxtools_download::DownloadEngine::open_in_memory().expect("synthetic engine"),
                bridge.clone(),
                notifications,
                crate::downloads::DownloadRuntimeContext {
                    fallback_default_directory: directory.path().into(),
                    config: nxtools_core::AppConfig::default(),
                    config_path: directory.path().join("config.json"),
                    log_directory: directory.path().into(),
                    database_path: directory.path().join("downloads.sqlite3"),
                    ffmpeg_install_directory: directory.path().join("ffmpeg"),
                },
                cx,
            )
        })
    });
    let settings = nxtools_core::DownloadTaskbarProgressOverlaySettings {
        width: 240,
        height: 32,
        show_speed: false,
        show_remaining_time: false,
        show_active_tasks: false,
        show_active_connections: false,
        show_transferred_bytes: false,
        show_progress: true,
        ..Default::default()
    };
    let model = crate::download_taskbar_overlay::DownloadTaskbarOverlayModel::Visible {
        lanes: vec![
            crate::download_taskbar_overlay::DownloadTaskbarOverlayLane::Determinate {
                completed: 50,
                total: 100,
            },
        ],
        metrics: crate::download_taskbar_overlay::DownloadTaskbarOverlayMetrics::default(),
        settings,
        editing: false,
    };
    let (_, window) = cx.add_window_view(move |window, cx| {
        window.set_background_appearance(gpui_kit::WindowBackgroundAppearance::Transparent);
        let view = cx.new(|cx| {
            crate::download_taskbar_overlay::DownloadTaskbarOverlayView::new(
                application.clone(),
                bridge.clone(),
                0,
                model,
                window,
                cx,
            )
        });
        gpui_kit::base::Root::new(view, window, cx)
    });
    window.simulate_resize(size(px(240.0), px(32.0)));
    window.refresh().expect("stage production Root refinement");
    window.run_until_parked();
    window.refresh().expect("settled transparent Root");
    let bounds = window
        .debug_bounds("download-taskbar-overlay-root")
        .expect("actual overlay root");
    assert_eq!(bounds.size, size(px(240.0), px(32.0)));
    let (quads, scale) = window.update(|window, _| (window.painted_quads(), window.scale_factor()));
    assert!(
        !quads.iter().any(|quad| {
            !quad.background.is_transparent()
                && quad.bounds.size.width.as_f32() / scale >= 238.0
                && quad.bounds.size.height.as_f32() / scale >= 30.0
        }),
        "transparent child cannot erase Kit Root's full-client fill: {quads:?}"
    );
    assert!(
        quads.iter().any(|quad| {
            !quad.background.is_transparent()
                && quad.bounds.size.width.as_f32() / scale == 120.0
                && quad.bounds.size.height.as_f32() / scale
                    == crate::download_taskbar_overlay::TASKBAR_OVERLAY_LANE_HEIGHT
        }),
        "real determinate indicator remains painted; no blanket appearance suppression"
    );
}
