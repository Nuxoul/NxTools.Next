use super::*;
use gpui_kit::base::test_support;
use gpui_kit::{AppContext, TestAppContext, VisualTestContext};

fn fixture(
    cx: &mut TestAppContext,
) -> (
    Entity<NetworkSettingsApplication>,
    Entity<NetworkSettingsView>,
    VisualTestContext,
    PathBuf,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.executor().allow_parking();
    let directory = std::env::temp_dir().join(format!("nxtools-queue09-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).expect("temp config directory");
    let config = AppConfig::default();
    let path = directory.join("config.json");
    config.save_to(&path).expect("synthetic config");
    let policy = NetworkPolicy::new(config.network.clone()).expect("isolated policy");
    let application = cx.update(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        cx.new(|_| {
            NetworkSettingsApplication::new(
                policy,
                config,
                path,
                notifications,
                None,
                Rc::new(RefCell::new(SystemSettings::default())),
                false,
            )
        })
    });
    cx.update(|cx| application.update(cx, |application, cx| application.open_window(cx)));
    cx.run_until_parked();
    let (handle, view) = cx.read(|cx| {
        let app = application.read(cx);
        (
            app.window.expect("window"),
            app.settings_view.clone().expect("view"),
        )
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(720.0), px(480.0)));
    surface.update(|window, _| window.activate_window());
    surface.refresh().expect("actual global settings");
    (application, view, surface, directory)
}
pub(super) fn select_page(surface: &mut VisualTestContext, index: usize) {
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    let node = nodes
        .iter()
        .find(|node| node.path().last() == Some(&format!("0-{index}").into()))
        .expect("official Settings Sidebar page");
    surface.simulate_click(node.bounds().center(), Default::default());
    surface.refresh().expect("selected official page");
}
#[gpui_kit::test]
fn queue09_production_settings_search_and_independent_checkbox_are_real(cx: &mut TestAppContext) {
    let (application, _, mut surface, directory) = fixture(cx);
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    assert!(
        nodes
            .iter()
            .any(|node| node.role() == Some(gpui_kit::Role::TextInput)),
        "actual Settings search"
    );
    select_page(&mut surface, 2);
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    assert!(
        nodes
            .iter()
            .filter(|node| node.role() == Some(gpui_kit::Role::CheckBox))
            .count()
            >= 2,
        "real independent scope Checkbox"
    );
    cx.update(|cx| application.update(cx, |application, cx| application.close_window(cx)));
    std::fs::remove_dir_all(directory).expect("cleanup synthetic config");
}

pub(super) fn page_bounds(
    surface: &mut VisualTestContext,
    index: usize,
) -> gpui_kit::Bounds<gpui_kit::Pixels> {
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    nodes
        .iter()
        .find(|node| node.path().last() == Some(&format!("0-{index}").into()))
        .expect("official page")
        .bounds()
}
pub(super) fn control_bounds(
    surface: &mut VisualTestContext,
    id: &str,
) -> gpui_kit::Bounds<gpui_kit::Pixels> {
    surface
        .update(|window, _| test_support::snapshots(window))
        .iter()
        .find(|node| node.path().last() == Some(&id.to_owned().into()))
        .expect("actual control")
        .bounds()
}
pub(super) fn focus_control(surface: &mut VisualTestContext, id: &str) {
    for _ in 0..60 {
        surface.update(|window, cx| {
            window.activate_window();
            window.focus_next(cx);
        });
        surface.refresh().expect("actual Tab frame");
        let focused = surface
            .update(|window, _| test_support::snapshots(window))
            .iter()
            .any(|node| {
                node.path().last() == Some(&id.to_owned().into()) && node.focused() == Some(true)
            });
        if focused {
            return;
        }
    }
    panic!("actual Tab cannot reach {id}");
}

fn release_space(surface: &mut VisualTestContext) {
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("Space"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("release"),
    });
    surface.run_until_parked();
    surface.refresh().expect("released frame");
}

#[gpui_kit::test]
fn queue09_scope_pointer_tab_release_is_independent_local_and_cancel_discards(
    cx: &mut TestAppContext,
) {
    let (application, view, mut surface, directory) = fixture(cx);
    let path = directory.join("config.json");
    let before = std::fs::read(&path).expect("original disk");
    select_page(&mut surface, 2);
    let original = view.read_with(&surface, |view, _| view.rules[0].clone());
    let checkbox = control_bounds(&mut surface, "global-blacklist-clipboard-0");
    surface.simulate_click(checkbox.center(), Default::default());
    surface.refresh().expect("requested Checkbox value");
    view.read_with(&surface, |view, _| {
        assert_eq!(view.rules[0].disable_clipboard, !original.disable_clipboard);
        assert_eq!(
            view.rules[0].disable_quick_launch,
            original.disable_quick_launch
        );
    });
    focus_control(&mut surface, "global-blacklist-quick-0");
    release_space(&mut surface);
    view.read_with(&surface, |view, _| {
        assert_eq!(view.rules[0].disable_clipboard, !original.disable_clipboard);
        assert_eq!(
            view.rules[0].disable_quick_launch,
            !original.disable_quick_launch
        );
    });
    let count = view.read_with(&surface, |view, _| view.rules.len());
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.process_input.update(cx, |input, cx| {
                input.set_value(r"X:\Synthetic\QueueEditor.EXE", window, cx)
            });
        })
    });
    focus_control(&mut surface, "global-blacklist-add");
    release_space(&mut surface);
    assert_eq!(
        view.read_with(&surface, |view, _| view.rules.len()),
        count + 1,
        "one released add"
    );
    assert_eq!(std::fs::read(&path).expect("draft disk"), before);
    focus_control(&mut surface, "network-settings-cancel-button");
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("cancel"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("cancel release"),
    });
    surface.run_until_parked();
    assert_eq!(std::fs::read(&path).expect("Cancel disk"), before);
    assert!(cx.read(|cx| application.read(cx).window.is_none()));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn queue09_proxy_form_preserves_inputs_invalid_draft_and_single_atomic_apply(
    cx: &mut TestAppContext,
) {
    let (application, view, mut surface, directory) = fixture(cx);
    let path = directory.join("config.json");
    let before = std::fs::read(&path).expect("original");
    let (host_id, port_id) = view.read_with(&surface, |view, _| {
        (view.host.entity_id(), view.port.entity_id())
    });
    select_page(&mut surface, 1);
    let manual = surface
        .debug_bounds("network-settings-proxy-mode-manual")
        .expect("actual mode button");
    surface.simulate_click(manual.center(), Default::default());
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.host.update(cx, |input, cx| {
                input.set_value("synthetic.proxy.invalid", window, cx)
            });
            view.port
                .update(cx, |input, cx| input.set_value("99999", window, cx));
        })
    });
    surface.refresh().expect("raw invalid proxy draft");
    let save = surface
        .debug_bounds("network-settings-save")
        .expect("Apply");
    surface.simulate_click(save.center(), Default::default());
    surface.run_until_parked();
    assert_eq!(std::fs::read(&path).expect("invalid no save"), before);
    assert!(view.read_with(&surface, |view, _| view.error.is_some() && !view.saving));
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.port
                .update(cx, |input, cx| input.set_value("8888", window, cx));
            assert_eq!(view.host.entity_id(), host_id);
            assert_eq!(view.port.entity_id(), port_id);
        })
    });
    surface.refresh().expect("valid original entity");
    let save = surface
        .debug_bounds("network-settings-save")
        .expect("Apply again");
    surface.simulate_click(save.center(), Default::default());
    surface.run_until_parked();
    let saved = AppConfig::load_from(&path).expect("synthetic committed");
    assert_eq!(saved.network.proxy.mode, NetworkProxyMode::ManualHttp);
    assert_eq!(saved.network.proxy.port, 8888);
    assert_eq!(saved.network.proxy.host, "synthetic.proxy.invalid");
    assert!(cx.read(|cx| application.read(cx).window.is_none()));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn queue09_official_search_filter_clear_never_saves_or_replaces_business_inputs(
    cx: &mut TestAppContext,
) {
    let (application, view, mut surface, directory) = fixture(cx);
    let path = directory.join("config.json");
    let before = std::fs::read(&path).expect("disk");
    let original = view.read_with(&surface, |view, _| view.process_input.entity_id());
    let search = surface
        .update(|window, _| test_support::snapshots(window))
        .into_iter()
        .find(|node| {
            node.role() == Some(gpui_kit::Role::TextInput) && node.bounds().right() <= px(200.0)
        })
        .expect("Settings search");
    surface.simulate_click(search.bounds().center(), Default::default());
    surface.simulate_keystrokes("代理");
    surface.run_until_parked();
    surface.refresh().expect("filter");
    assert!(
        surface
            .debug_bounds("network-settings-proxy-host-field")
            .is_some()
    );
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input("qqq-no-match");
    surface.run_until_parked();
    surface.refresh().expect("no match");
    surface.simulate_keystrokes("ctrl-a backspace");
    surface.run_until_parked();
    surface.refresh().expect("clear");
    select_page(&mut surface, 2);
    assert_eq!(
        view.read_with(&surface, |view, _| view.process_input.entity_id()),
        original
    );
    assert_eq!(std::fs::read(&path).expect("search no save"), before);
    cx.update(|cx| application.update(cx, |application, cx| application.close_window(cx)));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn queue09_failed_commit_keeps_draft_busy_guard_and_retry_uses_original_save(
    cx: &mut TestAppContext,
) {
    let (application, view, mut surface, directory) = fixture(cx);
    let path = directory.join("config.json");
    let before = std::fs::read(&path).expect("original");
    select_page(&mut surface, 2);
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.rules.clear();
            cx.notify();
        })
    });
    std::fs::remove_file(&path).expect("synthetic commit failure");
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.save(window, cx);
            view.save(window, cx);
            assert!(view.saving && view.draft_busy.get());
        })
    });
    surface.run_until_parked();
    surface.refresh().expect("failed commit visible");
    assert!(view.read_with(&surface, |view, _| !view.saving
        && view.error.is_none()
        && view.rules.is_empty()));
    application.read_with(&surface, |app, cx| {
        assert!(
            app.notifications
                .read(cx)
                .items()
                .iter()
                .any(|notice| notice.level == crate::notifications::NotificationLevel::Error)
        )
    });
    assert!(
        surface
            .debug_bounds("network-settings-inline-error")
            .is_none()
    );
    std::fs::write(&path, before).expect("restore temp config");
    let save = surface
        .debug_bounds("network-settings-save")
        .expect("retry Apply");
    surface.simulate_click(save.center(), Default::default());
    surface.run_until_parked();
    assert!(
        AppConfig::load_from(&path)
            .expect("saved empty canonical rules")
            .process_blacklist
            .entries()
            .is_empty()
    );
    assert!(cx.read(|cx| application.read(cx).window.is_none()));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

struct ProxyToggleCandidate {
    changes: Rc<RefCell<Vec<Vec<bool>>>>,
}
impl Render for ProxyToggleCandidate {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let changes = self.changes.clone();
        gpui_kit::component::button::ToggleGroup::new("queue09-toggle-modes")
            .small()
            .segmented()
            .child(
                gpui_kit::component::button::Toggle::new("queue09-direct")
                    .label("直连")
                    .checked(true),
            )
            .child(gpui_kit::component::button::Toggle::new("queue09-system").label("系统代理"))
            .child(gpui_kit::component::button::Toggle::new("queue09-manual").label("手动 HTTP"))
            .on_click(move |requested, _, _| changes.borrow_mut().push(requested.clone()))
    }
}
#[gpui_kit::test]
fn queue09_released_toggle_group_direct_recipe_is_not_exclusive_proxy_selection(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let changes = Rc::new(RefCell::new(vec![]));
    let held = changes.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|_| ProxyToggleCandidate { changes: held });
        Root::new(view, window, cx)
    });
    surface.refresh().expect("released mode candidate");
    let direct = control_bounds(surface, "queue09-direct");
    surface.simulate_click(direct.center(), Default::default());
    assert_eq!(
        changes.borrow().last(),
        Some(&vec![false, false, false]),
        "direct recipe allows no mode"
    );
    let before = changes.borrow().len();
    focus_control(surface, "queue09-direct");
    release_space(surface);
    assert_eq!(
        changes.borrow().len(),
        before,
        "specific group keyboard recipe does not invoke original mode setter"
    );
}

#[gpui_kit::test]
fn queue09_long_error_and_busy_controls_keep_footer_and_draft_at_fixed_budget(
    cx: &mut TestAppContext,
) {
    let (application, view, mut surface, directory) = fixture(cx);
    select_page(&mut surface, 2);
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.error = Some("合成输入校验失败，请检查应用规则。".repeat(40));
            view.saving = true;
            cx.notify();
        })
    });
    surface.refresh().expect("busy long error");
    let footer = surface
        .debug_bounds("network-settings-footer")
        .expect("fixed footer");
    let error = surface
        .debug_bounds("network-settings-inline-error")
        .expect("full error content");
    assert!(error.top() < footer.top() && footer.bottom() <= px(480.0));
    let before = view.read_with(&surface, |view, _| view.rules.clone());
    let scope = control_bounds(&mut surface, "global-blacklist-clipboard-0");
    surface.simulate_click(scope.center(), Default::default());
    let cancel = surface
        .debug_bounds("network-settings-cancel")
        .expect("disabled cancel");
    surface.simulate_click(cancel.center(), Default::default());
    surface.run_until_parked();
    assert_eq!(
        view.read_with(&surface, |view, _| view.rules.clone()),
        before
    );
    assert!(application.read_with(&surface, |application, _| application.window.is_some()));
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.saving = false;
            view.error = None;
            cx.notify();
        })
    });
    cx.update(|cx| application.update(cx, |application, cx| application.close_window(cx)));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn queue09_optional_help_tooltip_and_proxy_keyboard_keep_one_draft_dispatch(
    cx: &mut TestAppContext,
) {
    let (application, view, mut surface, directory) = fixture(cx);
    let help = surface
        .debug_bounds("global-setting-0-help")
        .expect("actual help trigger");
    assert!(
        help.right() <= px(720.0) && help.bottom() <= px(480.0),
        "help={help:?}"
    );
    surface.simulate_mouse_move(
        gpui_kit::point(px(10.0), px(10.0)),
        None,
        Default::default(),
    );
    surface.refresh().expect("initial pointer outside help");
    let button = control_bounds(&mut surface, "global-setting-0-help-button");
    surface.simulate_mouse_move(button.center(), None, Default::default());
    surface.refresh().expect("hover candidate frame");
    surface.run_until_parked();
    surface.refresh().expect("hover timer registered");
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(700));
    surface.run_until_parked();
    surface.refresh().expect("actual Tooltip content");
    let tooltip_nodes = surface.update(|window, _| test_support::snapshots(window));
    assert!(
        tooltip_nodes.iter().any(|node| node
            .path()
            .contains(&"global-setting-0-help-tooltip".into())),
        "help={help:?} button={button:?} paths={:?}",
        tooltip_nodes
            .iter()
            .map(|node| node.path())
            .collect::<Vec<_>>()
    );
    select_page(&mut surface, 1);
    focus_control(&mut surface, "network-settings-proxy-mode-manual");
    release_space(&mut surface);
    assert_eq!(
        view.read_with(&surface, |view, _| view.mode),
        NetworkProxyMode::ManualHttp
    );
    release_space(&mut surface);
    assert_eq!(
        view.read_with(&surface, |view, _| view.mode),
        NetworkProxyMode::ManualHttp,
        "repeat cannot clear sole proxy mode"
    );
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.saving = true;
            cx.notify();
        })
    });
    surface.refresh().expect("busy mode remains inert");
    let direct = surface
        .debug_bounds("network-settings-proxy-mode-direct")
        .expect("direct");
    surface.simulate_click(direct.center(), Default::default());
    surface.run_until_parked();
    assert_eq!(
        view.read_with(&surface, |view, _| view.mode),
        NetworkProxyMode::ManualHttp
    );
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.saving = false;
            cx.notify();
        })
    });
    cx.update(|cx| application.update(cx, |application, cx| application.close_window(cx)));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn feedback_global_switch_has_one_visible_label_and_complete_accessible_name(
    cx: &mut TestAppContext,
) {
    let (application, _, mut surface, directory) = fixture(cx);
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    for (id, label) in [
        ("system-start-on-login", "开机静默启动"),
        ("system-close-to-tray", "关闭到托盘"),
    ] {
        let control = nodes
            .iter()
            .find(|node| node.path().last() == Some(&id.into()))
            .expect("real Switch");
        assert_eq!(control.label(), Some(label));
        assert!(
            control.bounds().size.width <= px(40.0),
            "SettingItem owns visible text; Switch must not duplicate it: {:?}",
            control.bounds()
        );
    }
    cx.update(|cx| application.update(cx, |application, cx| application.close_window(cx)));
    std::fs::remove_dir_all(directory).expect("cleanup synthetic config");
}

#[gpui_kit::test]
fn balance_global_controls_use_a_bounded_lane_and_address_gets_more_space(cx: &mut TestAppContext) {
    let (application, _, mut surface, directory) = fixture(cx);
    let switch = control_bounds(&mut surface, "system-start-on-login");
    assert!(
        switch.right() >= px(640.0),
        "standalone Switch must align to settings right edge: {switch:?}"
    );
    select_page(&mut surface, 1);
    let host = surface
        .debug_bounds("network-settings-proxy-host-field")
        .expect("host");
    let port = surface
        .debug_bounds("network-settings-proxy-port-field")
        .expect("port");
    assert!(
        host.size.width >= port.size.width * 2.0,
        "address requires more space than numeric port: {host:?} {port:?}"
    );
    assert!(port.size.width >= px(80.0) && port.size.width <= px(140.0));
    assert!((host.top() - port.top()).abs() <= px(1.0));
    let mode = surface
        .debug_bounds("network-settings-proxy-mode-manual")
        .expect("proxy option");
    assert_eq!(
        mode.size.height,
        px(32.0),
        "input-adjacent selector uses input tier"
    );
    let title = surface
        .debug_bounds("global-setting-2-title")
        .expect("mode title");
    let help = surface
        .debug_bounds("global-setting-2-help")
        .expect("title-adjacent help");
    assert!(help.left() >= title.right() && help.left() - title.right() <= px(8.0));
    assert!(
        (host.left() - title.left()).abs() <= px(1.0),
        "proxy field starts at mode title edge"
    );
    cx.update(|cx| application.update(cx, |application, cx| application.close_window(cx)));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

fn choose_light_theme(surface: &mut VisualTestContext) {
    select_page(surface, 3);
    let light = surface.update(|window, _| {
        test_support::snapshots(window)
            .into_iter()
            .find(|node| node.label() == Some("默认浅色"))
            .expect("official visual theme Radio")
            .bounds()
    });
    surface.simulate_click(light.center(), Default::default());
    surface.run_until_parked();
}

#[gpui_kit::test]
fn theme_draft_cancel_save_and_reopen_preserve_one_palette_owner(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory) = fixture(cx);
    choose_light_theme(&mut surface);
    assert_eq!(
        view.read_with(&surface, |view, _| view.theme_preference),
        nxtools_core::ThemePreference::Light
    );
    assert_eq!(
        surface.read(Theme::active).canvas,
        Theme::zed_dark().canvas,
        "draft does not apply before Save"
    );
    let cancel = surface
        .debug_bounds("network-settings-cancel")
        .expect("Cancel");
    surface.simulate_click(cancel.center(), Default::default());
    surface.run_until_parked();
    assert_eq!(
        AppConfig::load_from(&directory.join("config.json"))
            .expect("config after Cancel")
            .system
            .theme,
        nxtools_core::ThemePreference::Dark
    );
    cx.update(|cx| application.update(cx, |app, cx| app.open_window(cx)));
    cx.run_until_parked();
    let (handle, view) = application.read_with(cx, |app, _| {
        (
            app.window.expect("reopened window"),
            app.settings_view.clone().expect("reopened draft"),
        )
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(720.0), px(480.0)));
    surface.refresh().expect("reopened settings");
    assert_eq!(
        view.read_with(&surface, |view, _| view.theme_preference),
        nxtools_core::ThemePreference::Dark
    );
    choose_light_theme(&mut surface);
    let save = surface.debug_bounds("network-settings-save").expect("Save");
    surface.simulate_click(save.center(), Default::default());
    surface.run_until_parked();
    assert_eq!(
        AppConfig::load_from(&directory.join("config.json"))
            .expect("config after Save")
            .system
            .theme,
        nxtools_core::ThemePreference::Light
    );
    assert_eq!(cx.read(Theme::active).canvas, Theme::light().canvas);
    cx.update(|cx| application.update(cx, |app, cx| app.open_window(cx)));
    cx.run_until_parked();
    assert_eq!(
        application.read_with(cx, |app, cx| app
            .settings_view
            .as_ref()
            .expect("new draft")
            .read(cx)
            .theme_preference),
        nxtools_core::ThemePreference::Light
    );
    cx.update(|cx| application.update(cx, |app, cx| app.close_window(cx)));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn density_theme_visual_radios_fit_minimum_and_keep_disabled_draft_local(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory) = fixture(cx);
    select_page(&mut surface, 3);
    let footer = surface
        .debug_bounds("network-settings-save")
        .expect("fixed footer");
    assert!(
        surface
            .debug_bounds("global-theme-preview-scroll")
            .is_none()
    );
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    for preference in nxtools_core::ThemePreference::ALL {
        let count = nodes
            .iter()
            .filter(|node| node.label() == Some(preference.label()))
            .count();
        if matches!(
            preference,
            nxtools_core::ThemePreference::CatppuccinLatte
                | nxtools_core::ThemePreference::CatppuccinFrappe
        ) {
            assert_eq!(count, 0, "default aliases have no duplicate Radio");
        } else {
            assert_eq!(
                count, 1,
                "each effective palette is selectable: {preference:?}"
            );
        }
    }
    assert!(footer.bottom() <= px(480.0));
    let original = std::fs::read(directory.join("config.json")).expect("saved config");
    choose_light_theme(&mut surface);
    assert_eq!(
        std::fs::read(directory.join("config.json")).expect("draft only"),
        original
    );
    let checked = surface.update(|window, _| test_support::snapshots(window));
    let selected = checked
        .iter()
        .find(|node| node.label() == Some("默认浅色"))
        .expect("selected Radio");
    assert_eq!(selected.checked(), Some(true));
    let initial_bounds = selected.bounds();
    surface.simulate_mouse_move(initial_bounds.center(), None, Default::default());
    surface.refresh().expect("hover is not selection");
    let hovered = surface.update(|window, _| test_support::snapshots(window));
    assert_eq!(
        hovered
            .iter()
            .find(|node| node.label() == Some("默认浅色"))
            .expect("stable Radio")
            .bounds(),
        initial_bounds
    );
    let system = hovered
        .iter()
        .find(|node| node.label() == Some("跟随系统"))
        .expect("system")
        .bounds();
    view.update(&mut surface, |view, cx| {
        view.saving = true;
        cx.notify();
    });
    surface.refresh().expect("disabled chooser");
    surface.simulate_click(system.center(), Default::default());
    surface.run_until_parked();
    assert_eq!(
        view.read_with(&surface, |view, _| view.theme_preference),
        nxtools_core::ThemePreference::Light
    );
    view.update(&mut surface, |view, cx| {
        view.saving = false;
        cx.notify();
    });
    surface.refresh().expect("enabled chooser");
    let mut reached = false;
    for _ in 0..40 {
        surface.update(|window, cx| window.focus_next(cx));
        surface.refresh().expect("real Tab");
        reached = surface
            .update(|window, _| test_support::snapshots(window))
            .iter()
            .any(|node| node.label() == Some("跟随系统") && node.focused() == Some(true));
        if reached {
            break;
        }
    }
    assert!(reached, "official System Radio reachable by Tab");
    release_space(&mut surface);
    assert_eq!(
        view.read_with(&surface, |view, _| view.theme_preference),
        nxtools_core::ThemePreference::System
    );
    application.update(&mut surface, |app, cx| app.close_window(cx));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn density_theme_failed_save_keeps_active_palette_and_retryable_draft(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory) = fixture(cx);
    choose_light_theme(&mut surface);
    let previous = surface.read(Theme::active).canvas;
    std::fs::write(directory.join("config.json"), b"{broken").expect("failed durable commit");
    let apply = surface
        .debug_bounds("network-settings-save")
        .expect("Apply");
    surface.simulate_click(apply.center(), Default::default());
    surface.run_until_parked();
    assert_eq!(surface.read(Theme::active).canvas, previous);
    view.read_with(&surface, |view, _| {
        assert_eq!(view.theme_preference, nxtools_core::ThemePreference::Light);
        assert!(!view.saving, "failed save remains retryable");
    });
    application.read_with(&surface, |app, cx| {
        assert!(
            app.notifications
                .read(cx)
                .items()
                .iter()
                .any(|notice| notice.title.as_ref() == "全局设置未能保存或应用"),
            "original notification owner reports failure"
        );
    });
    assert_eq!(
        std::fs::read(directory.join("config.json")).expect("no overwrite"),
        b"{broken"
    );
    application.update(&mut surface, |app, cx| app.close_window(cx));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn density_theme_last_option_scrolls_and_uses_official_keyboard_selection(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory) = fixture(cx);
    select_page(&mut surface, 3);
    let preview = surface
        .debug_bounds("theme-preview-默认深色")
        .expect("theme preview");
    let pointer = preview.center();
    let before = surface
        .debug_bounds("global-theme-chooser")
        .expect("page content");
    surface.simulate_mouse_move(pointer, None, Default::default());
    for _ in 0..6 {
        surface.simulate_event(gpui_kit::ScrollWheelEvent {
            position: pointer,
            delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.0), px(-500.0))),
            modifiers: Default::default(),
            touch_phase: gpui_kit::TouchPhase::Moved,
        });
        surface.run_until_parked();
        surface.refresh().expect("theme scroll");
        surface.update(|window, cx| window.draw(cx).clear(cx));
    }
    let last = surface
        .update(|window, _| test_support::snapshots(window))
        .into_iter()
        .find(|node| node.label() == Some("Solarized Dark"))
        .expect("last official Radio")
        .bounds();
    let after = surface
        .debug_bounds("global-theme-chooser")
        .expect("page scrolled");
    assert!(
        after.top() < before.top(),
        "wheel on preview scrolls the SettingPage"
    );
    let content = surface
        .debug_bounds("global-theme-chooser")
        .expect("page content");
    assert!(
        last.top() >= px(0.0) && last.bottom() <= px(480.0),
        "last choice reachable: {last:?} {content:?}"
    );
    surface.simulate_click(last.center(), Default::default());
    surface.run_until_parked();
    assert_eq!(
        view.read_with(&surface, |view, _| view.theme_preference),
        nxtools_core::ThemePreference::SolarizedDark
    );
    let mut reached = false;
    for _ in 0..50 {
        surface.update(|window, cx| window.focus_next(cx));
        surface.refresh().expect("official Tab frame");
        reached = surface
            .update(|window, _| test_support::snapshots(window))
            .iter()
            .any(|node| node.label() == Some("Gruvbox Dark") && node.focused() == Some(true));
        if reached {
            break;
        }
    }
    assert!(reached, "theme option reachable by the original Tab owner");
    release_space(&mut surface);
    assert_eq!(
        view.read_with(&surface, |view, _| view.theme_preference),
        nxtools_core::ThemePreference::GruvboxDark
    );
    let apply = surface
        .debug_bounds("network-settings-save")
        .expect("Apply remains visible");
    assert!(apply.top() >= px(0.0) && apply.bottom() <= px(480.0));
    application.update(&mut surface, |app, cx| app.close_window(cx));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn saved_theme_aliases_select_defaults_without_writing_on_open_or_cancel(cx: &mut TestAppContext) {
    for (alias, selected) in [
        (nxtools_core::ThemePreference::CatppuccinLatte, "默认浅色"),
        (nxtools_core::ThemePreference::CatppuccinFrappe, "默认深色"),
    ] {
        let (application, _, _, directory) = fixture(cx);
        cx.update(|cx| application.update(cx, |app, cx| app.close_window(cx)));
        let path = directory.join("config.json");
        let mut config = AppConfig::load_from(&path).expect("stored config");
        config.system.theme = alias;
        config.save_to(&path).expect("legacy preference fixture");
        let original = std::fs::read(&path).expect("original bytes");
        cx.update(|cx| application.update(cx, |app, cx| app.open_window(cx)));
        cx.run_until_parked();
        let handle = cx.read(|cx| application.read(cx).window.expect("window"));
        let mut surface = VisualTestContext::from_window(handle, cx);
        surface.simulate_resize(size(px(720.0), px(480.0)));
        surface.refresh().expect("alias chooser");
        select_page(&mut surface, 3);
        let nodes = surface.update(|window, _| test_support::snapshots(window));
        assert_eq!(
            nodes
                .iter()
                .find(|node| node.label() == Some(selected))
                .expect("default radio")
                .checked(),
            Some(true)
        );
        assert_eq!(std::fs::read(&path).expect("opened bytes"), original);
        let cancel = surface
            .debug_bounds("network-settings-cancel")
            .expect("Cancel");
        surface.simulate_click(cancel.center(), Default::default());
        surface.run_until_parked();
        assert_eq!(std::fs::read(&path).expect("cancelled bytes"), original);
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[gpui_kit::test]
fn runtime_feedback_blacklist_add_is_before_list(cx: &mut TestAppContext) {
    let (application, _, mut surface, directory) = fixture(cx);
    select_page(&mut surface, 2);
    let add = surface
        .debug_bounds("global-blacklist-new-row")
        .expect("add editor");
    let list = surface
        .debug_bounds("global-blacklist-table")
        .expect("blacklist table");
    assert!(
        add.bottom() <= list.top(),
        "add application must remain above the blacklist"
    );
    cx.update(|cx| application.update(cx, |application, cx| application.close_window(cx)));
    std::fs::remove_dir_all(directory).expect("cleanup synthetic config");
}

#[gpui_kit::test]
fn runtime_feedback_system_proxy_preserves_manual_and_rejects_stale_result(
    cx: &mut TestAppContext,
) {
    let (application, view, mut surface, directory) = fixture(cx);
    let (platform, endpoint) = nxtools_ipc::platform_bridge(16);
    endpoint.set_connected_for_contract_test(true);
    application.update(&mut surface, |owner, _| {
        owner.platform = Some(Rc::new(platform))
    });
    view.update_in(&mut surface, |view, window, cx| {
        view.host
            .update(cx, |input, cx| input.set_value("manual.test", window, cx));
        view.port
            .update(cx, |input, cx| input.set_value("8081", window, cx));
        view.select_proxy_mode(NetworkProxyMode::System, cx);
    });
    surface.run_until_parked();
    let old = endpoint
        .try_receive_command_for_contract_test()
        .expect("snapshot request")
        .id;
    view.update(&mut surface, |view, cx| {
        view.select_proxy_mode(NetworkProxyMode::ManualHttp, cx);
        view.select_proxy_mode(NetworkProxyMode::System, cx);
    });
    surface.run_until_parked();
    let current = endpoint
        .try_receive_command_for_contract_test()
        .expect("replacement snapshot")
        .id;
    let snapshot = nxtools_protocol::system_proxy::SystemProxySnapshot {
        auto_detect: false,
        pac_configured: false,
        endpoints: vec![nxtools_protocol::system_proxy::SystemProxyEndpoint {
            protocol: "all".into(),
            host: "127.0.0.1".into(),
            port: 7890,
        }],
    };
    application.update(&mut surface, |owner, cx| {
        owner.complete_system_proxy(old, Some(snapshot.clone()), cx)
    });
    view.read_with(&surface, |view, _| assert!(view.proxy_loading));
    application.update(&mut surface, |owner, cx| {
        owner.complete_system_proxy(current, Some(snapshot), cx)
    });
    surface.run_until_parked();
    view.read_with(&surface, |view, cx| {
        assert!(!view.proxy_loading && !view.proxy_read_failed);
        assert_eq!(view.proxy_rows[0].1.read(cx).value().as_str(), "127.0.0.1");
        assert_eq!(view.proxy_rows[0].2.read(cx).value().as_str(), "7890");
        assert_eq!(view.host.read(cx).value().as_str(), "manual.test");
        assert_eq!(view.port.read(cx).value().as_str(), "8081");
        assert_eq!(view.mode, NetworkProxyMode::System);
    });
    application.update(&mut surface, |owner, cx| owner.close_window(cx));
    std::fs::remove_dir_all(directory).expect("cleanup synthetic config");
}

#[gpui_kit::test]
fn accessible_name_global_apply_is_actual_button_and_stable_while_saving(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory) = fixture(cx);
    let path = directory.join("config.json");
    let before = std::fs::read(&path).expect("synthetic original config");
    for saving in [false, true] {
        view.update_in(&mut surface, |view, _, cx| {
            view.saving = saving;
            cx.notify();
        });
        surface.refresh().expect("actual global Apply state");
        let button = surface
            .update(|window, _| test_support::snapshots(window))
            .into_iter()
            .find(|node| {
                node.path().last()
                    == Some(&gpui_kit::ElementId::from("network-settings-save-button"))
            })
            .expect("actual Global Apply Button, not footer wrapper");
        assert_eq!(button.role(), Some(gpui_kit::Role::Button));
        assert_eq!(button.label(), Some("应用全局设置"));
        if saving {
            surface.simulate_click(button.bounds().center(), Default::default());
            surface.run_until_parked();
            assert!(view.read_with(&surface, |view, _| view.saving));
            assert_eq!(std::fs::read(&path).expect("busy never saves"), before);
        }
    }
    application.update(&mut surface, |application, cx| application.close_window(cx));
    std::fs::remove_dir_all(directory).expect("remove isolated global fixture");
}

#[gpui_kit::test]
fn audit06_shared_global_settings_minimum_keeps_actual_footer_and_cancel_owner(
    cx: &mut TestAppContext,
) {
    let (application, view, mut surface, directory) = fixture(cx);
    let original = surface.read(|cx| gpui_kit::component::Theme::global(cx).clone());
    let path = directory.join("config.json");
    let before = std::fs::read(&path).expect("owned original config");
    for base in [16.0, 14.0, 18.0] {
        surface.update(|_, cx| {
            gpui_kit::component::Theme::update(cx, |theme| theme.font_size = px(base))
        });
        view.update(&mut surface, |_, cx| cx.notify());
        surface.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
        assert_eq!(surface.update(|window, _| window.rem_size()), px(base));
        let footer = surface
            .debug_bounds("network-settings-footer")
            .expect("fixed original footer");
        assert_eq!(footer.size.height, px(48.0));
        assert_eq!(footer.bottom(), px(480.0));
        for id in [
            "network-settings-cancel-button",
            "network-settings-save-button",
        ] {
            let action = control_bounds(&mut surface, id);
            assert!(
                action.left() >= footer.left()
                    && action.right() <= footer.right()
                    && action.top() >= footer.top()
                    && action.bottom() <= footer.bottom()
            );
            assert!((action.size.height - px(24.0 * base / 16.0)).abs() <= px(0.5));
        }
        assert_eq!(std::fs::read(&path).expect("layout cannot save"), before);
    }
    let cancel = control_bounds(&mut surface, "network-settings-cancel-button");
    surface.simulate_click(cancel.center(), Default::default());
    surface.run_until_parked();
    assert!(application.read_with(&surface, |application, _| application.window.is_none()));
    assert_eq!(std::fs::read(&path).expect("Cancel retains config"), before);
    cx.update(|cx| gpui_kit::component::Theme::update(cx, |theme| *theme = original));
    std::fs::remove_dir_all(directory).expect("remove isolated fixture");
}
