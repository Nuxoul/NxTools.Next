//! Actual production Settings/MainWindow boundaries, scratch-only configuration.
use super::archive_page::ArchiveSettingsView;
use super::*;
use gpui_kit::{KeyUpEvent, Keystroke, Modifiers, TestAppContext, VisualTestContext, size};
use nxtools_core::AppConfig;

fn mount(
    cx: &mut TestAppContext,
    path: PathBuf,
) -> (
    Entity<MainWindow>,
    Entity<ArchiveSettingsView>,
    VisualTestContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.executor().allow_parking();
    let notifications = cx.new(|_| NotificationCenter::default());
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let (_, window) = cx.add_window_view(move |window, cx| {
        let view =
            cx.new(|cx| ArchiveSettingsView::new(AppConfig::default().archive, path, window, cx));
        let main = cx.new(|cx| {
            let mut main = MainWindow::new(
                gpui_kit::WeakEntity::new_invalid(),
                gpui_kit::WeakEntity::new_invalid(),
                gpui_kit::WeakEntity::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            );
            main.archive_settings = Some(view.clone());
            main.shell.activate(MainModule::Archive);
            main
        });
        *slot.borrow_mut() = Some((main.clone(), view));
        gpui_kit::base::Root::new(main, window, cx)
    });
    let handle = window.update(|window, _| window.window_handle());
    let (main, view) = retained.borrow().clone().expect("production views");
    let window = VisualTestContext::from_window(handle, cx);
    window.simulate_resize(size(px(920.0), px(480.0)));
    (main, view, window)
}
fn click(window: &mut VisualTestContext, selector: &'static str) {
    window.update(|window, cx| window.draw(cx).clear(cx));
    let bounds = window
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("mounted {selector}"));
    window.simulate_click(bounds.center(), Modifiers::default());
    window.run_until_parked();
    window.update(|window, cx| window.draw(cx).clear(cx));
}
fn select_page(window: &mut VisualTestContext, index: usize) {
    window.update(|window, cx| window.draw(cx).clear(cx));
    let bounds = window.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.path().last() == Some(&format!("0-{index}").into()))
            .expect("official Settings page")
            .bounds()
    });
    window.simulate_click(bounds.center(), Modifiers::default());
    window.run_until_parked();
    window.update(|window, cx| window.draw(cx).clear(cx));
}
#[gpui_kit::test]
fn archive_settings_navigation_save_failure_and_return_use_production_owners(
    cx: &mut TestAppContext,
) {
    let temp = tempfile::tempdir().expect("owned config");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let original = std::fs::read(&path).expect("initial bytes");
    let (main, view, mut window) = mount(cx, path.clone());
    window.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(
        std::fs::read(&path).expect("render bytes"),
        original,
        "render must not save"
    );
    select_page(&mut window, 2);
    click(&mut window, "archive-test-after-create");
    assert!(
        !AppConfig::load_from(&path)
            .expect("saved Switch")
            .archive
            .test_after_create
    );
    let input_id = view.read_with(&window, |view, _| view.exclusions.entity_id());
    click(&mut window, "main-module-其他工具");
    click(&mut window, "main-module-压缩");
    assert_eq!(
        main.read_with(&window, |main, _| main.shell.active_module()),
        MainModule::Archive
    );
    assert_eq!(
        view.read_with(&window, |view, _| view.exclusions.entity_id()),
        input_id
    );
    assert!(!view.read_with(&window, |view, _| view.settings.test_after_create));
    select_page(&mut window, 2);
    std::fs::write(&path, b"{broken").expect("corrupt fixture");
    click(&mut window, "archive-test-after-create");
    assert!(
        !view.read_with(&window, |view, _| view.settings.test_after_create),
        "failed save retains prior live value"
    );
    assert!(view.read_with(&window, |view, _| view.error.is_some()));
    assert!(
        window.debug_bounds("archive-settings-error").is_some(),
        "failure visible in original page"
    );
    assert_eq!(
        std::fs::read(path).expect("unchanged corruption"),
        b"{broken"
    );
}
#[gpui_kit::test]
fn archive_settings_real_keyboard_scroll_and_themes_keep_controls_reachable(
    cx: &mut TestAppContext,
) {
    let temp = tempfile::tempdir().expect("owned config");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let (_, _, mut window) = mount(cx, path.clone());
    for preference in [
        nxtools_core::ThemePreference::Light,
        nxtools_core::ThemePreference::Dark,
    ] {
        window.update(|_, cx| crate::theme::set_theme_preference(preference, cx));
        for dimensions in [size(px(1060.0), px(650.0)), size(px(920.0), px(480.0))] {
            window.simulate_resize(dimensions);
            select_page(&mut window, 2);
            let mut reached = false;
            for _ in 0..30 {
                window.update(|window, cx| window.draw(cx).clear(cx));
                if let Some(bounds) = window.debug_bounds("archive-exclusions-save")
                    && bounds.top() >= px(72.0)
                    && bounds.bottom() <= dimensions.height
                {
                    reached = true;
                    break;
                }
                window.simulate_event(gpui_kit::ScrollWheelEvent {
                    position: point(dimensions.width - px(150.0), dimensions.height - px(90.0)),
                    delta: gpui_kit::ScrollDelta::Lines(point(0.0, -3.0)),
                    ..Default::default()
                });
            }
            assert!(reached, "official Settings scroll reaches exclusions Save");
        }
    }
    select_page(&mut window, 5);
    click(&mut window, "archive-copy-motw");
    window.simulate_keystrokes("tab shift-tab");
    window.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: Keystroke::parse("space").expect("Space"),
        is_held: false,
        prefer_character_input: false,
    });
    window.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("space").expect("release"),
    });
    window.run_until_parked();
    assert!(
        AppConfig::load_from(&path)
            .expect("keyboard save")
            .archive
            .copy_motw,
        "one pointer then one keyboard activation"
    );
}

#[gpui_kit::test]
fn archive_settings_official_search_never_saves(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().expect("owned search config");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let before = std::fs::read(&path).expect("before");
    let (_, _, mut window) = mount(cx, path.clone());
    window.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    let search = window.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.role() == Some(gpui_kit::Role::TextInput))
            .expect("official search")
            .bounds()
    });
    window.simulate_click(search.center(), Modifiers::default());
    window.simulate_input("压缩后测试");
    window.run_until_parked();
    window.update(|window, cx| window.draw(cx).clear(cx));
    assert!(window.debug_bounds("archive-test-after-create").is_some());
    window.simulate_keystrokes("ctrl-a");
    window.simulate_input("没有任何匹配的合成设置");
    window.run_until_parked();
    window.update(|window, cx| window.draw(cx).clear(cx));
    assert!(window.update(|window, _| {
        !gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|node| node.label() == Some("压缩后测试"))
    }));
    assert_eq!(std::fs::read(path).expect("after search"), before);
}

#[gpui_kit::test]
fn archive_settings_exclusion_editor_is_lossless_and_stream_levels_are_valid(
    cx: &mut TestAppContext,
) {
    let temp = tempfile::tempdir().expect("owned presets");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let (_, view, mut window) = mount(cx, path.clone());
    let patterns = vec![
        " keep-spaces.txt ".to_owned(),
        "semi;colon".into(),
        "line\nbreak".into(),
    ];
    view.update_in(&mut window, |view, window, cx| {
        view.exclusions.update(cx, |input, cx| {
            input.set_value(
                serde_json::to_string(&patterns).expect("lossless JSON"),
                window,
                cx,
            )
        });
    });
    select_page(&mut window, 2);
    for _ in 0..20 {
        window.update(|window, cx| window.draw(cx).clear(cx));
        if window
            .debug_bounds("archive-exclusions-save")
            .is_some_and(|bounds| bounds.bottom() <= px(480.0))
        {
            break;
        }
        window.simulate_event(gpui_kit::ScrollWheelEvent {
            position: point(px(750.0), px(390.0)),
            delta: gpui_kit::ScrollDelta::Lines(point(0.0, -4.0)),
            ..Default::default()
        });
    }
    click(&mut window, "archive-exclusions-save");
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("saved")
            .archive
            .exclude_patterns,
        patterns
    );
    for format in [
        nxtools_core::ArchiveCreateFormat::Gz,
        nxtools_core::ArchiveCreateFormat::Xz,
    ] {
        view.update(&mut window, |view, cx| {
            view.settings.create_format = format;
            cx.notify();
        });
        select_page(&mut window, 0);
        select_page(&mut window, 2);
        window.update(|window, cx| window.draw(cx).clear(cx));
        let level = window.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| {
                    node.label() == Some("默认压缩级别: 标准")
                        && node.role() == Some(gpui_kit::Role::Button)
                })
                .expect("level Button")
                .bounds()
        });
        window.simulate_click(level.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            window.update(|window, _| !gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| node.label() == Some("仅存储"))),
            "stream must not offer Store"
        );
        window.simulate_keystrokes("escape");
    }
}

#[gpui_kit::test]
fn phase_d_production_quick_actions_are_real_and_reachable(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().expect("owned phase D config");
    let path = temp.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let before = std::fs::read(&path).expect("before");
    let (_, _, mut window) = mount(cx, path.clone());
    window.update(|window, cx| window.draw(cx).clear(cx));
    for selector in ["archive-open", "archive-create", "archive-extract"] {
        let bounds = window
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("production Phase D action missing: {selector}"));
        assert!(bounds.top() >= px(0.0) && bounds.bottom() <= px(480.0));
        assert!(bounds.size.height >= px(32.0), "official Medium action");
    }
    assert_eq!(std::fs::read(path).expect("after render"), before);
}
