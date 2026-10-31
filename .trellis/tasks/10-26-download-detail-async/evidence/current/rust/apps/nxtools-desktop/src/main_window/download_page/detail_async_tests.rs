//! Real production page gestures while selected-detail storage is blocked.
use super::*;
use crate::downloads::detail_request_tests::{HeldStore, fixture};
use gpui_kit::{AppContext, Modifiers, TestAppContext, VisualTestContext, size};

fn mount(
    cx: &mut TestAppContext,
    app: &Entity<DownloadApplication>,
) -> (Entity<MainWindow>, gpui_kit::AnyWindowHandle) {
    cx.update(crate::theme::apply_component_theme);
    let app = app.downgrade();
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let retained = Rc::new(RefCell::new(None));
    let result = retained.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let main = cx.new(|cx| {
            let mut main = MainWindow::new(
                gpui_kit::WeakEntity::new_invalid(),
                gpui_kit::WeakEntity::new_invalid(),
                app,
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            );
            main.shell.activate(MainModule::Downloads);
            main
        });
        *result.borrow_mut() = Some(main.clone());
        gpui_kit::base::Root::new(main, window, cx)
    });
    let main = retained.borrow_mut().take().expect("production MainWindow");
    (main, surface.update(|window, _| window.window_handle()))
}
fn click_row(surface: &mut VisualTestContext, id: DownloadTaskId) {
    let selector: &'static str =
        Box::leak(format!("download-task-row-{}", id.as_uuid()).into_boxed_str());
    let bounds = surface
        .debug_bounds(selector)
        .expect("current real task row");
    surface.simulate_click(bounds.center(), Modifiers::default());
    surface.run_until_parked();
}

#[gpui_kit::test]
async fn audit12_real_page_aba_and_tab_remain_responsive_while_store_is_held(
    cx: &mut TestAppContext,
) {
    let (app, engine, ids, _temp) = fixture(cx);
    let (main, handle) = mount(cx, &app);
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(920.0), px(480.0)));
    let held = HeldStore::new(engine);
    let (a, b) = (ids[0], ids[1]);
    click_row(&mut surface, a);
    assert_eq!(
        main.read_with(&surface, |main, _| main.selected_download_task),
        Some(a)
    );
    let tab = surface
        .debug_bounds("download-detail-tab-segments")
        .expect("original official tab");
    surface.simulate_click(tab.center(), Modifiers::default());
    surface.run_until_parked();
    assert_eq!(
        main.read_with(&surface, |main, _| main.download_detail_tab),
        DownloadDetailTab::Segments
    );
    assert!(
        surface
            .debug_bounds("download-detail-segments-progress")
            .is_some(),
        "actual loading state paints while storage is blocked"
    );
    click_row(&mut surface, b);
    click_row(&mut surface, a);
    assert_eq!(
        main.read_with(&surface, |main, _| main.selected_download_task),
        Some(a)
    );
    assert_eq!(
        app.read_with(&surface, |app, _| app.detail_request_events_for_test()),
        (1, 0, 0)
    );
    held.release();
    cx.condition(&app, |app, _| {
        !app.detail_request_state_for_test().1 && app.task_detail(a).is_some()
    })
    .await;
    assert_eq!(
        app.read_with(cx, |app, _| app.detail_request_events_for_test()),
        (2, 1, 1)
    );
    surface
        .refresh()
        .expect("current detail response paints on next application turn");
    assert!(
        surface
            .debug_bounds("download-detail-segments-empty")
            .is_some()
    );
    surface.update(|window, _| window.remove_window());
    surface.run_until_parked();
}

#[gpui_kit::test]
async fn audit12_closed_window_retained_entity_rejects_late_detail(cx: &mut TestAppContext) {
    let (app, engine, ids, _temp) = fixture(cx);
    let (main, handle) = mount(cx, &app);
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(920.0), px(480.0)));
    let held = HeldStore::new(engine);
    let id = ids[0];
    click_row(&mut surface, id);
    assert_eq!(
        main.read_with(&surface, |main, _| main.selected_download_task),
        Some(id)
    );
    surface.update(|window, _| window.remove_window());
    surface.run_until_parked();
    assert!(
        main.read_with(&surface, |main, _| main.download_detail_scope.is_none()),
        "closing the window retires scope even if entity remains owned"
    );
    held.release();
    cx.condition(&app, |app, _| !app.detail_request_state_for_test().1)
        .await;
    assert!(app.read_with(cx, |app, _| app.task_detail(id).is_none()));
    assert!(app.read_with(cx, |app, _| app.task_detail_error(id).is_none()));
    assert_eq!(
        app.read_with(cx, |app, _| app.detail_request_events_for_test()),
        (1, 0, 1)
    );
}

#[gpui_kit::test]
async fn audit12_real_module_switch_retires_pending_detail_without_cancelling_task(
    cx: &mut TestAppContext,
) {
    let (app, engine, ids, _temp) = fixture(cx);
    let (main, handle) = mount(cx, &app);
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(920.0), px(480.0)));
    let held = HeldStore::new(engine);
    let id = ids[0];
    click_row(&mut surface, id);
    let tools = surface
        .debug_bounds("main-module-其他工具")
        .expect("original official module button");
    surface.simulate_click(tools.center(), Modifiers::default());
    surface.run_until_parked();
    assert_eq!(
        main.read_with(&surface, |main, _| main.shell.active_module()),
        MainModule::Tools
    );
    assert!(main.read_with(&surface, |main, _| main.download_detail_scope.is_none()));
    held.release();
    cx.condition(&app, |app, _| !app.detail_request_state_for_test().1)
        .await;
    assert!(app.read_with(cx, |app, _| app.task_detail(id).is_none()));
    assert_eq!(
        app.read_with(cx, |app, _| app.detail_request_events_for_test()),
        (1, 0, 1)
    );
    assert!(
        app.read_with(cx, |app, _| app.tasks().iter().any(|task| task.task.id()
            == id
            && task.task.status() == DownloadStatus::Created)),
        "presentation retirement never cancels queued download"
    );
    surface.update(|window, _| window.remove_window());
}
