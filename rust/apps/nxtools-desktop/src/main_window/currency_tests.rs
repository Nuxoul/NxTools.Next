use super::*;
use gpui_kit::size;
use gpui_kit::{KeyDownEvent, KeyUpEvent, Keystroke, TestAppContext, VisualTestContext};
use nxtools_core::AppConfig;

#[gpui_kit::test]
fn currency_tool_card_settings_keyboard_persistence_bounds_and_reuse(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let dir = tempfile::tempdir().expect("currency UI fixture");
    let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        dir.path().join("history.db"),
        dir.path().join("images"),
        nxtools_clipboard::ClipboardRetentionPolicy {
            max_items: 100,
            retention_days: 30,
        },
        1_000,
    )
    .expect("worker");
    let (platform, _endpoint) = nxtools_ipc::platform_bridge(64);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let config = AppConfig::default();
    let path = dir.path().join("config.json");
    config.save_to(&path).expect("seed");
    let clipboard = cx.update(|cx| {
        cx.new(|cx| {
            ClipboardApplication::new(
                worker.clone(),
                Rc::new(platform),
                config,
                path.clone(),
                notifications.clone(),
                cx,
            )
        })
    });
    let rates = crate::tools::currency::RateSnapshot {
        version: 2,
        provider: "fawazahmed0 Currency API".into(),
        date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        fetched_at: nxtools_ipc::unix_time_ms(),
        rates: [
            ("CNY".into(), 1.0),
            ("USD".into(), 0.14),
            ("KRW".into(), 200.0),
            ("USDT".into(), 0.14),
            ("XAU".into(), 0.00005),
        ]
        .into(),
        names: [
            ("CNY".into(), "Chinese Yuan".into()),
            ("USD".into(), "US Dollar".into()),
            ("KRW".into(), "South Korean Won".into()),
            ("USDT".into(), "Tether".into()),
            ("XAU".into(), "Gold".into()),
        ]
        .into(),
    };
    // Production cache loading, with no public endpoint or live user clipboard.
    crate::tools::currency::save_cache(&dir.path().join("currency-rates-v2.json"), &rates)
        .expect("cache");
    cx.update(|cx| {
        clipboard.update(cx, |owner, cx| {
            owner.bind_currency_network(
                nxtools_network::NetworkPolicy::new(nxtools_core::NetworkSettings::default())
                    .expect("policy"),
                cx,
            )
        })
    });
    cx.run_until_parked();
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let clip = clipboard.downgrade();
    let (_root, window) = cx.add_window_view(move |window, cx| {
        let main = cx.new(|cx| {
            let mut main = MainWindow::new(
                clip,
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
            main.shell.activate(MainModule::Tools);
            main
        });
        *slot.borrow_mut() = Some(main.clone());
        gpui_kit::base::Root::new(main, window, cx)
    });
    let main_handle = window.update(|window, _| window.window_handle());
    let mut window = VisualTestContext::from_window(main_handle, cx);
    let main = retained.borrow().clone().expect("main");
    window.simulate_resize(size(px(920.0), px(480.0)));
    window.refresh().expect("tiles");
    let card = window
        .debug_bounds("tool-currency-card")
        .expect("currency tile");
    assert_eq!(card.size, size(px(180.0), px(160.0)));
    window.simulate_click(card.center(), gpui_kit::Modifiers::default());
    window.run_until_parked();
    assert!(window.read(|cx| clipboard.read(cx).currency_status().enabled));
    assert!(
        AppConfig::load_from(&path)
            .expect("persisted")
            .clipboard
            .currency_conversion
            .enabled
    );
    assert!(window.read(|cx| main.read(cx).tool_settings_windows.is_empty()));
    window.update(|window, cx| {
        let focus = main.read(cx).currency_tool_focus.clone();
        focus.focus(window, cx);
    });
    window.simulate_event(KeyDownEvent {
        keystroke: Keystroke::parse("space").expect("space"),
        is_held: false,
        prefer_character_input: false,
    });
    window.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("space").expect("release"),
    });
    window.run_until_parked();
    assert!(!window.read(|cx| clipboard.read(cx).currency_status().enabled));
    let configure = window
        .debug_bounds("tool-currency-configure")
        .expect("configure");
    window.simulate_click(configure.center(), gpui_kit::Modifiers::default());
    window.run_until_parked();
    let handle = window.read(|cx| {
        *main
            .read(cx)
            .tool_settings_windows
            .get("tool-currency")
            .expect("settings")
    });
    assert!(
        !window.read(|cx| clipboard.read(cx).currency_status().enabled),
        "configure cannot bubble"
    );
    window.simulate_click(configure.center(), gpui_kit::Modifiers::default());
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| *main
            .read(cx)
            .tool_settings_windows
            .get("tool-currency")
            .expect("reused")),
        handle
    );
    let mut settings = VisualTestContext::from_window(handle, cx);
    settings.simulate_resize(size(px(460.0), px(320.0)));
    settings.refresh().expect("feature shell");
    let target = settings
        .debug_bounds("currency-target")
        .expect("target Select");
    assert!(target.size.width > px(100.0));
    assert_eq!(target.size.height, px(32.0));
    let update = settings
        .debug_bounds("currency-update")
        .expect("outlined update");
    assert_eq!(update.size.height, px(24.0));
    assert!(update.bottom() < px(320.0));
    settings.update(|_, cx| {
        clipboard.update(cx, |owner, cx| {
            owner.currency_fixture_refresh_state(true, None, cx)
        })
    });
    settings.run_until_parked();
    settings.refresh().expect("busy state");
    let busy_nodes = settings.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert!(
        busy_nodes
            .iter()
            .any(|node| node.label() == Some("更新中…")),
        "busy label uses real official Button"
    );
    // Locked Kit snapshots omit disabled; exercise both pointer and keyboard
    // against the real button instead of pretending None means enabled.
    settings.simulate_click(update.center(), gpui_kit::Modifiers::default());
    settings.run_until_parked();
    settings.simulate_keystrokes("tab");
    settings.simulate_event(KeyDownEvent {
        keystroke: Keystroke::parse("enter").expect("busy Enter"),
        is_held: false,
        prefer_character_input: false,
    });
    settings.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("enter").expect("busy release"),
    });
    settings.run_until_parked();
    assert!(settings.read(|cx| clipboard.read(cx).currency_status().busy));
    settings.update(|_, cx| {
        clipboard.update(cx, |owner, cx| {
            owner.currency_fixture_refresh_state(false, Some("汇率更新失败，请重试"), cx)
        })
    });
    settings.run_until_parked();
    settings.refresh().expect("error state");
    assert!(settings.read(|cx| clipboard.read(cx).currency_status().error.is_some()));
    assert!(
        settings
            .debug_bounds("currency-update")
            .expect("retry remains reachable")
            .bottom()
            < px(320.0)
    );
    settings.update(|_, cx| {
        clipboard.update(cx, |owner, cx| {
            owner.currency_fixture_refresh_state(false, None, cx)
        })
    });
    settings.run_until_parked();
    settings.update(|window, _| window.activate_window());
    settings.simulate_click(target.center(), gpui_kit::Modifiers::default());
    settings.run_until_parked();
    settings.simulate_keystrokes("krw");
    settings.run_until_parked();
    settings.refresh().expect("actual search dropdown");
    settings.update(|_, cx| {
        clipboard.update(cx, |owner, cx| {
            owner.currency_fixture_refresh_state(false, Some("合成状态刷新"), cx)
        })
    });
    settings.run_until_parked();
    settings.simulate_keystrokes("enter");
    settings.run_until_parked();
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("search confirmed")
            .clipboard
            .currency_conversion
            .target,
        "KRW",
        "real search filters,doesnotpick oldUSDrow"
    );
    let target = settings.debug_bounds("currency-target").expect("trigger");
    settings.simulate_click(target.center(), Default::default());
    settings.run_until_parked();
    settings.simulate_keystrokes("tether");
    settings.run_until_parked();
    settings.simulate_keystrokes("escape");
    settings.run_until_parked();
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("Escape no save")
            .clipboard
            .currency_conversion
            .target,
        "KRW"
    );
    settings.simulate_click(target.center(), Default::default());
    settings.run_until_parked();
    settings.simulate_keystrokes("notacurrency");
    settings.run_until_parked();
    settings.simulate_keystrokes("enter");
    settings.run_until_parked();
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("no match no save")
            .clipboard
            .currency_conversion
            .target,
        "KRW"
    );
    settings.simulate_keystrokes("escape");
    settings.run_until_parked();
    settings.simulate_click(target.center(), Default::default());
    settings.run_until_parked();
    settings.simulate_keystrokes("tether");
    settings.run_until_parked();
    settings.simulate_keystrokes("enter");
    settings.run_until_parked();
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("API name search")
            .clipboard
            .currency_conversion
            .target,
        "USDT"
    );
    let mut refreshed = rates.clone();
    refreshed.fetched_at += 1;
    refreshed.names.insert("NEWCOIN".into(), "New Coin".into());
    refreshed.rates.insert("NEWCOIN".into(), 0.5);
    settings.update(|_, cx| {
        clipboard.update(cx, |owner, cx| {
            owner.currency_fixture_apply_rates(refreshed, cx)
        })
    });
    settings.run_until_parked();
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("refresh keepsselection")
            .clipboard
            .currency_conversion
            .target,
        "USDT"
    );
    settings.simulate_click(target.center(), Default::default());
    settings.run_until_parked();
    settings.simulate_keystrokes("newcoin");
    settings.run_until_parked();
    settings.simulate_keystrokes("enter");
    settings.run_until_parked();
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("new dynamic catalogchoice")
            .clipboard
            .currency_conversion
            .target,
        "NEWCOIN"
    );
    settings.simulate_click(target.center(), Default::default());
    settings.run_until_parked();
    settings.simulate_input("美刀");
    settings.run_until_parked();
    settings.simulate_keystrokes("enter");
    settings.run_until_parked();
    let saved = AppConfig::load_from(&path).expect("target persisted");
    assert_eq!(saved.clipboard.currency_conversion.target, "USD");
    assert!(!saved.clipboard.currency_conversion.enabled);
    let toggle = settings
        .debug_bounds("currency-settings-toggle")
        .expect("switch");
    settings.simulate_click(toggle.center(), gpui_kit::Modifiers::default());
    settings.run_until_parked();
    assert!(settings.read(|cx| clipboard.read(cx).currency_status().enabled));
    let close = settings
        .debug_bounds("currency-settings-close")
        .expect("close");
    settings.simulate_click(close.center(), gpui_kit::Modifiers::default());
    settings.run_until_parked();
    drop(settings);
    window.simulate_click(configure.center(), gpui_kit::Modifiers::default());
    window.run_until_parked();
    let reopened = window.read(|cx| {
        *main
            .read(cx)
            .tool_settings_windows
            .get("tool-currency")
            .expect("reopened")
    });
    assert_ne!(handle, reopened);
    let mut settings = VisualTestContext::from_window(reopened, cx);
    settings.refresh().expect("reopen");
    assert!(settings.read(|cx| clipboard.read(cx).currency_status().enabled));
    let close = settings
        .debug_bounds("currency-settings-close")
        .expect("close");
    settings.simulate_click(close.center(), gpui_kit::Modifiers::default());
    settings.run_until_parked();
    worker.shutdown().expect("shutdown");
}
