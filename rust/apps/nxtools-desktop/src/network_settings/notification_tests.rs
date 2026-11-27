use super::*;
use gpui_kit::{TestAppContext, VisualTestContext};

#[gpui_kit::test]
fn notification_duration_global_input_cancel_range_save_reopen_and_failure(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (application, directory, _) = tests::application_fixture(cx, "notification-duration");
    let path = directory.join("config.json");
    let mut config = AppConfig::load_from(&path).expect("owned config");
    config.system.notification_duration_seconds = 17;
    config.save_to(&path).expect("existing setting");
    let before = std::fs::read(&path).expect("bytes");
    application.update(cx, |app, cx| app.open_window(cx));
    cx.run_until_parked();
    let (handle, view) = application.read_with(cx, |app, _| {
        (
            app.window.expect("window"),
            app.settings_view.clone().expect("draft"),
        )
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(720.0), px(480.0)));
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    let bounds = surface
        .debug_bounds("global-notification-duration")
        .expect("official NumberInput");
    assert_eq!(bounds.size.height, px(32.0));
    let row = surface
        .debug_bounds("global-setting-8")
        .expect("official setting row");
    assert!(
        bounds.left() >= row.left()
            && bounds.right() <= row.right()
            && bounds.bottom() <= px(430.0),
        "input must fit actual official row: {bounds:?} row={row:?}"
    );
    let input = view.read_with(&surface, |view, _| view.notification_duration.clone());
    assert_eq!(input.read_with(&surface, |input, _| input.value()), "17");
    surface.simulate_click(bounds.center(), Default::default());
    assert!(
        surface.update(|window, cx| {
            use gpui_kit::Focusable as _;
            input.focus_handle(cx).is_focused(window)
        }),
        "actual pointer must focus NumberInput text slot"
    );
    surface.simulate_keystrokes("ctrl-a 8");
    surface.run_until_parked();
    assert_eq!(input.read_with(&surface, |input, _| input.value()), "8");
    assert_eq!(std::fs::read(&path).expect("draft no write"), before);
    let nodes = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    let help = nodes
        .iter()
        .find(|node| node.path().last() == Some(&"global-setting-8-help-button".into()))
        .expect("duration help");
    assert!(
        help.label()
            .expect("description")
            .contains("暂停整个通知列表的剩余时间")
    );
    let increment = nodes
        .iter()
        .find(|node| node.path().last() == Some(&"increment".into()))
        .expect("official increment button")
        .bounds();
    let decrement = nodes
        .iter()
        .find(|node| node.path().last() == Some(&"decrement".into()))
        .expect("official decrement button")
        .bounds();
    surface.simulate_click(increment.center(), Default::default());
    surface.run_until_parked();
    assert_eq!(input.read_with(&surface, |input, _| input.value()), "9");
    surface.simulate_click(bounds.center(), Default::default());
    surface.simulate_keystrokes("up");
    surface.run_until_parked();
    assert_eq!(input.read_with(&surface, |input, _| input.value()), "10");
    surface.simulate_keystrokes("down");
    surface.run_until_parked();
    assert_eq!(input.read_with(&surface, |input, _| input.value()), "9");
    input.update_in(&mut surface, |input, window, cx| {
        input.set_value("60", window, cx)
    });
    surface.simulate_click(increment.center(), Default::default());
    surface.run_until_parked();
    assert_eq!(input.read_with(&surface, |input, _| input.value()), "60");
    input.update_in(&mut surface, |input, window, cx| {
        input.set_value("1", window, cx)
    });
    surface.simulate_click(decrement.center(), Default::default());
    surface.run_until_parked();
    assert_eq!(input.read_with(&surface, |input, _| input.value()), "1");
    view.update(&mut surface, |view, cx| {
        view.saving = true;
        cx.notify();
    });
    surface.refresh().expect("disabled number input");
    surface.simulate_click(increment.center(), Default::default());
    surface.run_until_parked();
    assert_eq!(input.read_with(&surface, |input, _| input.value()), "1");
    view.update(&mut surface, |view, cx| {
        view.saving = false;
        cx.notify();
    });
    application.update(&mut surface, |app, cx| app.close_window(cx));
    assert_eq!(std::fs::read(&path).expect("cancel"), before);
    application.update(&mut surface, |app, cx| app.open_window(cx));
    surface.run_until_parked();
    let (handle, view) = application.read_with(&surface, |app, _| {
        (
            app.window.expect("reopen"),
            app.settings_view.clone().expect("draft"),
        )
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    let input = view.read_with(&surface, |view, _| view.notification_duration.clone());
    assert_eq!(input.read_with(&surface, |input, _| input.value()), "17");
    for invalid in ["", "0", "61", "abc", "1.5"] {
        input.update_in(&mut surface, |input, window, cx| {
            input.set_value(invalid, window, cx)
        });
        view.update_in(&mut surface, |view, window, cx| view.save(window, cx));
        surface.run_until_parked();
        assert!(view.read_with(&surface, |view, _| {
            view.error
                .as_ref()
                .is_some_and(|error| error.contains("1–60"))
        }));
        assert_eq!(std::fs::read(&path).expect("invalid no write"), before);
    }
    input.update_in(&mut surface, |input, window, cx| {
        input.set_value("9", window, cx)
    });
    view.update_in(&mut surface, |view, window, cx| view.save(window, cx));
    surface.run_until_parked();
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("saved")
            .system
            .notification_duration_seconds,
        9
    );
    application.read_with(&surface, |app, cx| {
        assert_eq!(app.config.system.notification_duration_seconds, 9);
        assert_eq!(app.notifications.read(cx).duration_seconds(), 9);
    });
    let good = std::fs::read(&path).expect("good");
    std::fs::write(&path, b"corrupt owned fixture").expect("failure");
    application.update(&mut surface, |app, cx| {
        let mut system = app.config.system.clone();
        system.notification_duration_seconds = 60;
        assert!(
            app.apply_global(
                app.config.network.clone(),
                app.config.system.clone(),
                system,
                app.config.process_blacklist.clone(),
                uuid::Uuid::new_v4(),
                None,
                cx
            )
            .is_err()
        );
    });
    surface.run_until_parked();
    application.read_with(&surface, |app, cx| {
        assert_eq!(app.notifications.read(cx).duration_seconds(), 9)
    });
    assert_eq!(
        std::fs::read(&path).expect("preserved"),
        b"corrupt owned fixture"
    );
    std::fs::write(&path, good).expect("restore");
    let _ = std::fs::remove_dir_all(directory);
}
