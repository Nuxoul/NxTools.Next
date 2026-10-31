//! Queue08 actual Global Settings presentation and original durable action owner.
use super::*;
use gpui_kit::component::Theme as ComponentTheme;
use gpui_kit::{TestAppContext, VisualTestContext};

fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| { window.refresh(); window.draw(cx).clear(cx); });
}
fn base(surface: &mut VisualTestContext, value: f32) {
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| theme.font_size = px(value)));
    draw(surface);
    assert_eq!(surface.update(|window, _| window.rem_size()), px(value));
}
fn fixture(cx: &mut TestAppContext) -> (Entity<NetworkSettingsApplication>, Entity<NetworkSettingsView>, VisualTestContext, PathBuf) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (app, directory, _) = tests::application_fixture(cx, "queue08-rem");
    app.update(cx, |app, cx| app.open_window(cx));
    cx.run_until_parked();
    let (handle, view) = cx.read(|cx| { let app = app.read(cx); (app.window.expect("original settings window"), app.settings_view.clone().expect("original draft")) });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(720.0), px(480.0)));
    surface.update(|window, _| window.activate_window());
    draw(&mut surface);
    (app, view, surface, directory)
}
#[gpui_kit::test]
fn audit06_pages_global_ordinary_input_rules_and_fixed_footer_follow_root(cx: &mut TestAppContext) {
    let (app, view, mut surface, directory) = fixture(cx);
    let original = surface.read(|cx| ComponentTheme::global(cx).clone());
    view.update(&mut surface, |view, cx| {
        view.rules = (0..24).map(|index| ProcessBlacklistRule::new(&format!("synthetic{index}.exe"), true, false).expect("synthetic rule")).collect();
        cx.notify();
    });
    for value in [16.0, 14.0, 18.0] {
        view.update(&mut surface, |_, cx| cx.notify());
        base(&mut surface, value);
        kit_migration_tests::select_page(&mut surface, 0);
        draw(&mut surface);
        let input = surface.debug_bounds("global-notification-duration").expect("actual NumberInput rail");
        assert!((input.size.width - px(168.0 * value / 16.0)).abs() <= px(0.5), "ordinary input width at{value}: {input:?}");
        assert_eq!(input.size.height, px(32.0 * value / 16.0));
        kit_migration_tests::select_page(&mut surface, 2);
        draw(&mut surface);
        let row = surface.debug_bounds("global-blacklist-row-0").expect("actual rule row");
        assert!((row.size.height - px(40.0 * value / 16.0)).abs() <= px(0.5), "ordinary rule row at{value}: {row:?}");
        let column = surface.debug_bounds("global-blacklist-header-clipboard").expect("actual scope header");
        assert!((column.size.width - px(96.0 * value / 16.0)).abs() <= px(0.5));
        let footer = surface.debug_bounds("network-settings-footer").expect("fixed footer");
        assert_eq!(footer.size.height, px(48.0));
        assert_eq!(footer.bottom(), px(480.0));
        let action = kit_migration_tests::control_bounds(&mut surface, "network-settings-save-button");
        assert!(action.top() >= footer.top() && action.bottom() <= footer.bottom());
    }
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
    app.update(&mut surface, |app, cx| app.close_window(cx));
    std::fs::remove_dir_all(directory).expect("remove owned fixture");
}

fn wheel(surface: &mut VisualTestContext, position: gpui_kit::Point<gpui_kit::Pixels>, lines: f32) {
    surface.simulate_event(gpui_kit::ScrollWheelEvent { position, delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, lines)), ..Default::default() });
    draw(surface);
}
#[gpui_kit::test]
fn audit06_pages_global_long_rules_reach_last_real_action_and_tab_scope(cx: &mut TestAppContext) {
    let (app, view, mut surface, directory) = fixture(cx);
    let original = surface.read(|cx| ComponentTheme::global(cx).clone());
    for value in [16.0, 14.0, 18.0] {
        view.update(&mut surface, |view, cx| {
            view.rules = (0..24).map(|index| ProcessBlacklistRule::new(&format!("合成中文应用进程名称用于完整提示验证{index}.exe"), true, false).expect("synthetic rule")).collect();
            cx.notify();
        });
        base(&mut surface, value);
        kit_migration_tests::select_page(&mut surface, 2);
        draw(&mut surface);
        for _ in 0..10 {
            let viewport = surface.debug_bounds("global-blacklist-viewport").expect("real bounded list viewport");
            let content = surface.debug_bounds("network-settings-content").expect("content clip");
            let footer = surface.debug_bounds("network-settings-footer").expect("footer");
            let top = viewport.top().max(content.top());
            let bottom = viewport.bottom().min(footer.top());
            let last = surface.debug_bounds("global-blacklist-row-23").expect("actual final rule");
            if last.top() >= top && last.bottom() <= bottom { break; }
            wheel(&mut surface, gpui_kit::point(viewport.center().x, (top + px(40.0)).min(bottom - px(4.0))), -30.0);
        }
        let last = surface.debug_bounds("global-blacklist-row-23").expect("current last rule");
        let viewport = surface.debug_bounds("global-blacklist-viewport").expect("current list viewport");
        let footer = surface.debug_bounds("network-settings-footer").expect("fixed footer");
        assert!(last.top() >= viewport.top() && last.bottom() <= viewport.bottom().min(footer.top()), "actual last rule inside original120viewport at{value}: {last:?} / {viewport:?}");
        let edit = kit_migration_tests::control_bounds(&mut surface, "global-blacklist-edit-23");
        surface.simulate_click(edit.center(), Default::default());
        draw(&mut surface);
        assert_eq!(view.read_with(&surface, |view, _| view.editing_rule), Some(23));
        let input = view.read_with(&surface, |view, _| view.process_input.clone());
        assert!(input.read_with(&surface, |input, _| input.value().contains("23")));
        let scope = kit_migration_tests::control_bounds(&mut surface, "global-blacklist-quick-23");
        surface.simulate_click(scope.center(), Default::default());
        draw(&mut surface);
        assert!(view.read_with(&surface, |view, _| view.rules[23].disable_quick_launch));
        surface.update(|window, cx| input.update(cx, |input, cx| input.focus(window, cx)));
        draw(&mut surface);
        let target = gpui_kit::ElementId::from("global-blacklist-quick-23");
        let mut focused = false;
        for _ in 0..160 {
            surface.simulate_keystrokes("tab");
            draw(&mut surface);
            focused = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window).iter().any(|node| node.path().last() == Some(&target) && node.focused() == Some(true)));
            if focused { break; }
        }
        assert!(focused, "actual Tab reaches original final scope Checkbox");
        surface.simulate_keystrokes("space");
        surface.simulate_event(gpui_kit::KeyUpEvent { keystroke: gpui_kit::Keystroke::parse("space").expect("Space release") });
        draw(&mut surface);
        assert!(!view.read_with(&surface, |view, _| view.rules[23].disable_quick_launch));
    }
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
    let before = std::fs::read(directory.join("config.json")).expect("owned saved config");
    let cancel = kit_migration_tests::control_bounds(&mut surface, "network-settings-cancel-button");
    surface.simulate_click(cancel.center(), Default::default());
    surface.run_until_parked();
    assert!(app.read_with(&surface, |app, _| app.window.is_none()));
    assert_eq!(std::fs::read(directory.join("config.json")).expect("Cancel config"), before);
    std::fs::remove_dir_all(directory).expect("remove owned fixture");
}

#[gpui_kit::test]
fn audit06_pages_global_real_apply_failure_keeps_long_draft_and_section_merge(cx: &mut TestAppContext) {
    let (app, view, mut surface, directory) = fixture(cx);
    let original = surface.read(|cx| ComponentTheme::global(cx).clone());
    let path = directory.join("config.json");
    let before = std::fs::read(&path).expect("owned config bytes");
    let host = view.read_with(&surface, |view, _| view.host.clone());
    let identity = host.entity_id();
    for value in [16.0, 14.0, 18.0] {
        view.update(&mut surface, |_, cx| cx.notify());
        base(&mut surface, value);
        kit_migration_tests::select_page(&mut surface, 1);
        let manual = kit_migration_tests::control_bounds(&mut surface, "network-settings-proxy-mode-manual");
        surface.simulate_click(manual.center(), Default::default());
        draw(&mut surface);
        surface.update(|window, cx| host.update(cx, |input, cx| input.focus(window, cx)));
        surface.simulate_keystrokes("ctrl-a");
        surface.simulate_input("synthetic-proxy.invalid");
        draw(&mut surface);
        let actual = surface.debug_bounds("network-settings-proxy-host-field").expect("real Input");
        assert!(actual.left() >= px(128.0) && actual.right() <= px(720.0));
        std::fs::remove_file(&path).expect("inject owned durable failure");
        let apply = kit_migration_tests::control_bounds(&mut surface, "network-settings-save-button");
        surface.simulate_click(apply.center(), Default::default());
        draw(&mut surface);
        assert!(app.read_with(&surface, |app, _| app.window.is_some() && app.pending_apply.is_none()));
        assert!(!view.read_with(&surface, |view, _| view.saving || view.draft_busy.get()));
        assert_eq!(host.entity_id(), identity);
        assert_eq!(host.read_with(&surface, |input, _| input.value().to_string()), "synthetic-proxy.invalid");
        assert!(app.read_with(&surface, |app, cx| app.notifications.read(cx).items().iter().any(|notice| notice.level == crate::notifications::NotificationLevel::Error)));
        std::fs::write(&path, &before).expect("restore owned fixture");
    }
    let mut newer: AppConfig = serde_json::from_slice(&before).expect("owned JSON");
    newer.clipboard.max_items = 3210;
    newer.save_to(&path).expect("newer unrelated section");
    let apply = kit_migration_tests::control_bounds(&mut surface, "network-settings-save-button");
    surface.simulate_click(apply.center(), Default::default());
    surface.run_until_parked();
    let saved = AppConfig::load_from(&path).expect("original transaction");
    assert_eq!(saved.network.proxy.host, "synthetic-proxy.invalid");
    assert_eq!(saved.clipboard.max_items, 3210);
    assert!(app.read_with(&surface, |app, _| app.window.is_none()));
    cx.update(|cx| ComponentTheme::update(cx, |theme| *theme = original));
    std::fs::remove_dir_all(directory).expect("remove owned fixture");
}
