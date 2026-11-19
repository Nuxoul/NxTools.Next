use super::*;
use crate::notifications::{self, NotificationCenter, NotificationLevel};
use gpui_kit::{TestAppContext, VisualTestContext};

fn open_settings(
    cx: &mut TestAppContext,
    platform: Option<Rc<PlatformBridge>>,
) -> (Entity<NotificationCenter>, &mut VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let center = cx.new(|_| NotificationCenter::default());
    cx.update(|cx| notifications::register(&center, cx));
    let (_, surface) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| SettingsView::new(platform, window, cx));
        gpui_kit::base::Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(460.), px(640.)));
    surface.run_until_parked();
    surface.refresh().unwrap();
    (center, surface)
}

#[gpui_kit::test]
fn markdown_reader_settings_button_notifies_durable_save_and_failure(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let config = AppConfig::default();
    config.save_to(&path).unwrap();
    cx.update(|cx| install_settings(&config, path.clone(), cx));
    let (center, surface) = open_settings(cx, None);
    assert!(center.read_with(surface, |center, _| center.items().is_empty()));
    let source = surface.update(|window, _| window.window_handle());
    let original_bounds = surface
        .debug_bounds("markdown-font-default-larger")
        .unwrap();
    let click_larger = |surface: &mut VisualTestContext| {
        surface.refresh().unwrap();
        let bounds = surface
            .debug_bounds("markdown-font-default-larger")
            .unwrap();
        surface.simulate_click(bounds.center(), Default::default());
        surface.run_until_parked();
    };
    surface.refresh().unwrap();
    click_larger(surface);
    assert_eq!(
        AppConfig::load_from(&path)
            .unwrap()
            .markdown_reader
            .font_size,
        21
    );
    let first = center.read_with(surface, |center, _| {
        assert_eq!(
            center.items().len(),
            1,
            "actual settings button must use NotificationCenter"
        );
        let notice = &center.items()[0];
        assert_eq!(notice.level, NotificationLevel::Success);
        assert_eq!(notice.target, Some(source));
        notice.id.clone()
    });
    std::fs::write(&path, b"broken").unwrap();
    click_larger(surface);
    center.read_with(surface, |center, _| {
        assert_eq!(center.items().len(), 2);
        let notice = center
            .items()
            .iter()
            .find(|notice| notice.id != first)
            .unwrap();
        assert_eq!(notice.level, NotificationLevel::Error);
        assert_eq!(notice.target, Some(source));
        assert!(notice.message.contains("原设置仍然有效"));
    });
    surface.read(|cx| assert_eq!(preferences(cx).font_size, 21));
    assert_eq!(std::fs::read(&path).unwrap(), b"broken");
    surface.refresh().unwrap();
    assert_eq!(
        surface
            .debug_bounds("markdown-font-default-larger")
            .unwrap(),
        original_bounds,
        "operation feedback must not insert a banner or move settings rows"
    );
}

fn reply(id: Uuid, success: bool, data: &str) -> PlatformNotification {
    PlatformNotification::CommandResult {
        correlation_id: id,
        result: nxtools_protocol::PlatformCommandResult {
            success,
            error: None,
            native_error_code: None,
            data: Some(data.to_owned()),
        },
    }
}

const REGISTERED: &str = r#"{"md":{"registered":true,"is_default":false},"markdown":{"registered":true,"is_default":false}}"#;
const UNREGISTERED: &str = r#"{"md":{"registered":false,"is_default":false},"markdown":{"registered":false,"is_default":false}}"#;

#[gpui_kit::test]
fn markdown_reader_settings_association_buttons_notify_once_and_keep_source(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| install_settings(&AppConfig::default(), PathBuf::new(), cx));
    let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let (center, surface) = open_settings(cx, Some(Rc::new(platform)));
    let source = surface.update(|window, _| window.window_handle());
    surface.update(|_, cx| {
        let pending = cx.global::<Settings>().request.unwrap();
        platform_notification(&reply(pending.id, true, REGISTERED), cx);
    });
    surface.run_until_parked();
    assert!(center.read_with(surface, |center, _| center.items().is_empty()));
    for (selector, data, level, message) in [
        (
            "markdown-register",
            REGISTERED,
            NotificationLevel::Success,
            "已添加打开方式",
        ),
        (
            "markdown-default-apps",
            "opened",
            NotificationLevel::Info,
            "分别选择",
        ),
        (
            "markdown-assoc-refresh",
            REGISTERED,
            NotificationLevel::Info,
            "已刷新",
        ),
        (
            "markdown-unregister",
            UNREGISTERED,
            NotificationLevel::Success,
            "已移除",
        ),
    ] {
        surface.refresh().unwrap();
        let bounds = surface.debug_bounds(selector).unwrap();
        surface.simulate_click(bounds.center(), Default::default());
        surface.run_until_parked();
        let id = surface.read(|cx| cx.global::<Settings>().request.unwrap().id);
        surface.update(|_, cx| {
            platform_notification(&reply(Uuid::new_v4(), true, data), cx);
            platform_notification(&reply(id, true, data), cx);
            platform_notification(&reply(id, true, data), cx);
        });
        surface.run_until_parked();
        let generation = center.read_with(surface, |center, _| {
            assert_eq!(center.items().len(), 1, "one result per explicit action");
            let notice = &center.items()[0];
            assert!(notice.id.contains(&id.to_string()));
            assert_eq!(notice.level, level);
            assert_eq!(notice.target, Some(source));
            assert!(notice.message.contains(message), "{}", notice.message);
            notice.generation()
        });
        surface.update(|_, cx| platform_notification(&reply(id, true, data), cx));
        surface.run_until_parked();
        center.update(surface, |center, cx| {
            assert_eq!(center.items()[0].generation(), generation);
            center.dismiss_window(source, cx);
        });
        surface.read(|cx| {
            let observed = cx.global::<Settings>().association.as_ref().unwrap();
            assert!(!observed.md.is_default && !observed.markdown.is_default);
        });
    }
}

fn begin(intent: Intent, source: AnyWindowHandle, cx: &mut App) -> Uuid {
    let id = Uuid::new_v4();
    cx.global_mut::<Settings>().request = Some(Pending {
        id,
        intent,
        source,
        started: Instant::now(),
    });
    id
}

#[gpui_kit::test]
fn markdown_reader_settings_publish_only_durable_preferences(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let config = AppConfig::default();
    config.save_to(&path).unwrap();
    cx.update(|cx| install_settings(&config, path.clone(), cx));
    let (center, surface) = open_settings(cx, None);
    let source = surface.update(|window, cx| {
        let source = window.window_handle();
        save(Change::Font(22), source, cx);
        save(Change::Font(24), source, cx);
        assert_eq!(preferences(cx).font_size, config.markdown_reader.font_size);
        source
    });
    surface.run_until_parked();
    surface.read(|cx| assert_eq!(preferences(cx).font_size, 22));
    assert_eq!(
        AppConfig::load_from(&path)
            .unwrap()
            .markdown_reader
            .font_size,
        22
    );
    assert_eq!(
        center.read_with(surface, |center, _| center.items().len()),
        1
    );
    surface.update(|_, cx| save(Change::Font(22), source, cx));
    surface.run_until_parked();
    assert_eq!(
        center.read_with(surface, |center, _| center.items().len()),
        1,
        "no-op does not notify"
    );
    std::fs::write(&path, b"broken").unwrap();
    surface.update(|_, cx| save(Change::Font(24), source, cx));
    surface.run_until_parked();
    surface.read(|cx| assert_eq!(preferences(cx).font_size, 22));
    center.read_with(surface, |center, _| {
        assert!(
            center
                .items()
                .iter()
                .any(|item| item.level == NotificationLevel::Error)
        )
    });
    assert_eq!(std::fs::read(&path).unwrap(), b"broken");
}

#[gpui_kit::test]
fn markdown_reader_settings_refuse_disable_while_windows_uses_its_handler(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let config = AppConfig::default();
    config.save_to(&path).unwrap();
    cx.update(|cx| install_settings(&config, path.clone(), cx));
    let (center, surface) = open_settings(cx, None);
    let source = surface.update(|window, cx| {
        let source = window.window_handle();
        let id = begin(Intent::CheckDisable, source, cx);
        platform_notification(&reply(id, true, r#"{"md":{"registered":true,"is_default":true},"markdown":{"registered":true,"is_default":false}}"#), cx);
        source
    });
    surface.run_until_parked();
    surface.read(|cx| assert!(enabled(cx)));
    assert!(AppConfig::load_from(&path).unwrap().markdown_reader.enabled);
    center.update(surface, |center, cx| {
        assert_eq!(center.items().len(), 1);
        assert_eq!(center.items()[0].level, NotificationLevel::Error);
        assert!(center.items()[0].message.contains("选回其他应用"));
        center.dismiss_window(source, cx);
    });
    let id = surface.update(|_, cx| {
        let id = begin(Intent::CheckDisable, source, cx);
        platform_notification(&reply(id, true, UNREGISTERED), cx);
        id
    });
    surface.run_until_parked();
    assert!(!AppConfig::load_from(&path).unwrap().markdown_reader.enabled);
    center.read_with(surface, |center, _| {
        assert_eq!(
            center.items().len(),
            1,
            "disable query and save have one terminal notice"
        );
        assert_eq!(center.items()[0].level, NotificationLevel::Success);
        assert!(center.items()[0].id.contains(&id.to_string()));
        assert_eq!(center.items()[0].target, Some(source));
    });
}

#[gpui_kit::test]
fn markdown_reader_default_settings_waits_for_per_extension_readback(cx: &mut TestAppContext) {
    cx.update(|cx| install_settings(&AppConfig::default(), PathBuf::new(), cx));
    let (center, surface) = open_settings(cx, None);
    let source = surface.update(|window, cx| {
        let source = window.window_handle();
        let id = begin(Intent::Query, source, cx);
        platform_notification(&reply(id, true, REGISTERED), cx);
        source
    });
    surface.run_until_parked();
    surface.update(|_, cx| {
        let id = begin(Intent::DefaultApps, source, cx);
        platform_notification(&reply(id, true, "opened"), cx);
    });
    surface.run_until_parked();
    surface.update(|_, cx| {
        let state = cx.global::<Settings>();
        assert!(!state.association.as_ref().unwrap().md.is_default);
        assert!(!state.association.as_ref().unwrap().markdown.is_default);
        let id = begin(Intent::Query, source, cx);
        platform_notification(&reply(id, true, r#"{"md":{"registered":true,"is_default":true},"markdown":{"registered":true,"is_default":false}}"#), cx);
    });
    surface.run_until_parked();
    surface.read(|cx| {
        let observed = cx.global::<Settings>().association.as_ref().unwrap();
        assert!(observed.md.is_default);
        assert!(!observed.markdown.is_default);
    });
    center.read_with(surface, |center, _| {
        assert_eq!(
            center.items().len(),
            1,
            "automatic readback must not add a notification"
        );
        assert_eq!(center.items()[0].level, NotificationLevel::Info);
    });
}

#[gpui_kit::test]
fn markdown_reader_settings_explicit_errors_notify_but_automatic_queries_are_silent(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| install_settings(&AppConfig::default(), PathBuf::new(), cx));
    let (center, surface) = open_settings(cx, None);
    let source = surface.update(|window, _| window.window_handle());
    for status in [
        "identity_unavailable",
        "registration_conflict",
        "open_defaults_failed",
        "still_default",
        "unexpected",
    ] {
        surface.update(|_, cx| {
            let id = begin(Intent::Query, source, cx);
            platform_notification(&reply(id, false, status), cx);
        });
        surface.run_until_parked();
        assert!(center.read_with(surface, |center, _| center.items().is_empty()));
        surface.update(|_, cx| {
            let id = begin(Intent::Register, source, cx);
            platform_notification(&reply(id, false, status), cx);
        });
        surface.run_until_parked();
        center.update(surface, |center, cx| {
            assert_eq!(center.items().len(), 1);
            assert_eq!(center.items()[0].level, NotificationLevel::Error);
            assert_eq!(center.items()[0].target, Some(source));
            center.dismiss_window(source, cx);
        });
    }
    surface.update(|_, cx| {
        association_request(None, Intent::Query, source, cx);
        association_request(None, Intent::Refresh, source, cx);
    });
    surface.run_until_parked();
    center.update(surface, |center, cx| {
        assert_eq!(
            center.items().len(),
            1,
            "only the explicit failed submit notifies"
        );
        assert_eq!(center.items()[0].level, NotificationLevel::Error);
        center.dismiss_window(source, cx);
    });
    for intent in [Intent::Query, Intent::Refresh] {
        surface.update(|_, cx| {
            let id = begin(intent, source, cx);
            platform_notification(
                &PlatformNotification::Disconnected {
                    generation: 1,
                    pending_command_ids: vec![id],
                },
                cx,
            );
        });
        surface.run_until_parked();
        assert_eq!(
            center.read_with(surface, |center, _| center.items().len()),
            usize::from(intent == Intent::Refresh)
        );
        surface.read(|cx| assert!(cx.global::<Settings>().request.is_none()));
    }
}

#[gpui_kit::test]
fn markdown_reader_settings_superseded_completion_is_silent(cx: &mut TestAppContext) {
    cx.update(|cx| install_settings(&AppConfig::default(), PathBuf::new(), cx));
    let (center, surface) = open_settings(cx, None);
    let current = surface.update(|window, cx| {
        let source = window.window_handle();
        let old = begin(Intent::Register, source, cx);
        platform_notification(&reply(old, true, REGISTERED), cx);
        begin(Intent::Refresh, source, cx)
    });
    surface.run_until_parked();
    assert!(center.read_with(surface, |center, _| center.items().is_empty()));
    surface.update(|_, cx| {
        assert_eq!(cx.global::<Settings>().request.unwrap().id, current);
        assert!(cx.global::<Settings>().association.is_none());
        platform_notification(&reply(current, true, UNREGISTERED), cx);
    });
    surface.run_until_parked();
    center.read_with(surface, |center, _| {
        assert_eq!(center.items().len(), 1);
        assert!(center.items()[0].id.contains(&current.to_string()));
        assert_eq!(center.items()[0].level, NotificationLevel::Info);
    });
}
