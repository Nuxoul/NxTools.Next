use super::tools_page::floating_assistant_settings::{
    self as settings, FloatingAssistantSettingsBody,
};
use super::*;
use gpui_kit::{TestAppContext, VisualTestContext, size};

#[test]
fn floating_assistant_action_options_are_exact_and_defaults_are_covered() {
    use nxtools_core::{ClickAction, FloatingAssistantSettings, ImageDropAction, TextDropAction};
    let defaults = FloatingAssistantSettings::default();
    assert!(!defaults.enabled);
    assert_eq!(defaults.left_click_action, ClickAction::OpenMain);
    assert_eq!(defaults.right_click_action, ClickAction::AddDownload);
    assert_eq!(defaults.text_drop_action, TextDropAction::Automatic);
    assert_eq!(defaults.image_drop_action, ImageDropAction::Automatic);
    assert_eq!(
        settings::choice_labels(),
        [
            vec!["无", "打开主窗口", "添加下载"],
            vec!["无", "打开主窗口", "添加下载"],
            vec!["无", "自动判断", "添加到下载", "速启搜索"],
            vec!["无", "自动判断", "添加到下载", "写入剪切板"],
        ]
    );
}

#[gpui_kit::test]
fn floating_assistant_config_shell_uses_actual_controls_without_an_owner(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_root, window) = cx.add_window_view(|window, cx| {
        let body = cx.new(|cx| FloatingAssistantSettingsBody::new(window, cx));
        gpui_kit::base::Root::new(body, window, cx)
    });
    let handle = window.update(|window, _| window.window_handle());
    let mut window = VisualTestContext::from_window(handle, cx);
    window.simulate_resize(size(px(460.0), px(320.0)));
    window.refresh().expect("feature shell");
    let size_control = window.debug_bounds("floating-size").expect("number field");
    assert_eq!(size_control.size.height, px(32.0));
    assert!(size_control.size.width > px(100.0));
    assert!(window.debug_bounds("floating-settings-toggle").is_some());
    assert!(window.debug_bounds("floating-image-pick").is_some());
    assert!(window.debug_bounds("floating-image-reset").is_some());
    let nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert!(nodes.iter().any(|node| node.label() == Some("选择图片")));
}

#[gpui_kit::test]
fn floating_assistant_tile_configure_persistence_and_last_control_scroll(cx: &mut TestAppContext) {
    use crate::tools::floating_assistant::{self, FloatingAssistantApplication};
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let directory = tempfile::tempdir().expect("temporary UI config");
    let path = directory.path().join("config.json");
    let config = nxtools_core::AppConfig::default();
    config.save_to(&path).expect("seed config");
    let center = cx.new(|_| NotificationCenter::default());
    let main_slot = Rc::new(RefCell::new(None));
    let slot = main_slot.clone();
    let notices = center.clone();
    let (_, window) = cx.add_window_view(move |window, cx| {
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
                notices,
                window,
                cx,
            );
            main.shell.activate(MainModule::Tools);
            main
        });
        *slot.borrow_mut() = Some(main.clone());
        gpui_kit::base::Root::new(main, window, cx)
    });
    let source = window.update(|window, _| window.window_handle());
    let (bridge, _endpoint) = nxtools_ipc::platform_bridge(64);
    let owner = cx.new(|cx| {
        FloatingAssistantApplication::new(
            Rc::new(bridge),
            config,
            path.clone(),
            center,
            source,
            gpui_kit::WeakEntity::new_invalid(),
            gpui_kit::WeakEntity::new_invalid(),
            cx,
        )
    });
    cx.update(|cx| floating_assistant::register(owner.clone(), cx));
    let main = main_slot.borrow().clone().expect("main");
    let mut window = VisualTestContext::from_window(source, cx);
    window.simulate_resize(size(px(920.0), px(480.0)));
    window.refresh().expect("tools");
    let card = window
        .debug_bounds("tool-floating-assistant-card")
        .expect("tile");
    assert_eq!(card.size, size(px(180.0), px(160.0)));
    assert!(
        window
            .debug_bounds("tool-floating-assistant-status-dot")
            .is_some(),
        "floating tool uses the shared enabled indicator"
    );
    let configure = window
        .debug_bounds("tool-floating-assistant-configure")
        .expect("configure");
    window.simulate_click(configure.center(), Default::default());
    window.run_until_parked();
    assert!(
        !window.read(|cx| owner.read(cx).settings().enabled),
        "Configure cannot toggle"
    );
    let settings_handle = window.read(|cx| {
        *main
            .read(cx)
            .tool_settings_windows
            .get("tool-floating-assistant")
            .expect("settings")
    });
    window.simulate_click(configure.center(), Default::default());
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| *main
            .read(cx)
            .tool_settings_windows
            .get("tool-floating-assistant")
            .expect("reuse")),
        settings_handle
    );
    let mut settings = VisualTestContext::from_window(settings_handle, cx);
    settings.simulate_resize(size(px(460.0), px(320.0)));
    settings.refresh().expect("settings");
    let toggle = settings
        .debug_bounds("floating-settings-toggle")
        .expect("enable switch");
    settings.simulate_click(toggle.center(), Default::default());
    settings.run_until_parked();
    assert!(
        nxtools_core::AppConfig::load_from(&path)
            .expect("saved enable")
            .floating_assistant
            .enabled
    );
    let number = settings.debug_bounds("floating-size").expect("size field");
    settings.simulate_click(number.center(), Default::default());
    settings.simulate_keystrokes("ctrl-a");
    settings.simulate_input("96");
    settings.simulate_keystrokes("enter");
    settings.run_until_parked();
    assert_eq!(
        nxtools_core::AppConfig::load_from(&path)
            .expect("saved size")
            .floating_assistant
            .size,
        96
    );
    for _ in 0..12 {
        settings.simulate_event(gpui_kit::ScrollWheelEvent {
            position: point(px(220.0), px(240.0)),
            delta: gpui_kit::ScrollDelta::Pixels(point(px(0.0), px(-160.0))),
            modifiers: Default::default(),
            touch_phase: gpui_kit::TouchPhase::Moved,
        });
        settings.run_until_parked();
        settings.refresh().expect("scroll");
    }
    let pass = settings
        .debug_bounds("floating-click-through-control")
        .expect("last control");
    assert!(
        pass.top() >= px(48.0) && pass.bottom() <= px(320.0),
        "last setting is actually inside scroll viewport"
    );
}
