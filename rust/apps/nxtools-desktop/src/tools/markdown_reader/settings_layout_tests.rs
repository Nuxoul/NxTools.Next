use super::*;
use crate::notifications::NotificationCenter;
use gpui_kit::{Bounds, Modifiers, Pixels, Role, TestAppContext, VisualTestContext};
use nxtools_core::ThemePreference;

fn open_layout(
    cx: &mut TestAppContext,
    platform: Option<Rc<PlatformBridge>>,
) -> (Entity<NotificationCenter>, &mut VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.executor().allow_parking();
    cx.update(|cx| install_settings(&AppConfig::default(), PathBuf::new(), cx));
    let center = cx.new(|_| NotificationCenter::default());
    cx.update(|cx| notifications::register(&center, cx));
    let (_, surface) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| SettingsView::new(platform, window, cx));
        gpui_kit::base::Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(460.), px(320.)));
    surface.run_until_parked();
    surface.update(|_, cx| cx.global_mut::<Settings>().request = None);
    surface.refresh().expect("production compact settings");
    (center, surface)
}

fn bounds(surface: &mut VisualTestContext, selector: &'static str) -> Bounds<Pixels> {
    surface
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("rendered {selector}"))
}

fn scroll(surface: &mut VisualTestContext, dy: f32) {
    let viewport = bounds(surface, "markdown-settings-content");
    surface.simulate_event(gpui_kit::ScrollEvent {
        // The shared viewport margin avoids any child control gesture boundary.
        position: gpui_kit::point(viewport.left() + px(2.), viewport.center().y),
        delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(dy))),
        modifiers: Modifiers::default(),
        touch_phase: gpui_kit::TouchPhase::Moved,
        source: Default::default(),
    });
    surface.run_until_parked();
    surface.refresh().expect("shared settings scroll");
}

fn reveal(surface: &mut VisualTestContext, selector: &'static str) -> Bounds<Pixels> {
    scroll(surface, 2_000.);
    for _ in 0..16 {
        let viewport = bounds(surface, "markdown-settings-content");
        let target = bounds(surface, selector);
        if target.top() >= viewport.top() && target.bottom() <= viewport.bottom() {
            assert!(target.left() >= viewport.left() && target.right() <= viewport.right());
            return target;
        }
        scroll(surface, -48.);
    }
    panic!("action must be fully reachable at 460x320: {selector}");
}

fn semantic_control(
    surface: &mut VisualTestContext,
    label: &str,
    role: Role,
    disabled: bool,
) -> Bounds<Pixels> {
    let nodes = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    let node = nodes
        .iter()
        .find(|node| node.label() == Some(label) && node.role() == Some(role))
        .unwrap_or_else(|| panic!("actual named {role:?}: {label}"));
    assert_eq!(
        node.disabled() == Some(true),
        disabled,
        "availability: {label}"
    );
    node.bounds()
}

fn assert_association_status(
    surface: &mut VisualTestContext,
    extension: &'static str,
    expected: &str,
) {
    let selector = if extension == ".md" {
        "markdown-assoc-md-control"
    } else {
        "markdown-assoc-markdown-control"
    };
    assert!(bounds(surface, selector).size.width > px(0.));
    let label = surface.read(|cx| {
        let state = cx.global::<Settings>();
        association_label(state.association.as_ref().map(|association| {
            if extension == ".md" {
                &association.md
            } else {
                &association.markdown
            }
        }))
    });
    assert_eq!(label, expected, "observed association: {extension}");
}

const ASSOCIATION_ACTIONS: [(&str, &str); 4] = [
    ("markdown-register", "添加打开方式"),
    ("markdown-default-apps", "设置为默认…"),
    ("markdown-unregister", "解除注册"),
    ("markdown-assoc-refresh", "刷新状态"),
];

#[gpui_kit::test]
fn markdown_settings_compact_horizontal_light_dark_bounds_and_scroll(cx: &mut TestAppContext) {
    let (_, surface) = open_layout(cx, None);
    for theme in [ThemePreference::Light, ThemePreference::Dark] {
        surface.update(|_, cx| crate::theme::set_theme_preference(theme, cx));
        surface.refresh().expect("requested settings theme");
        scroll(surface, 2_000.);
        let root = bounds(surface, "markdown-settings");
        assert_eq!(root.size, size(px(460.), px(320.)));
        let viewport = bounds(surface, "markdown-settings-content");
        let title = bounds(surface, "markdown-settings-title");
        let enable = bounds(surface, "markdown-settings-enable-row");
        let reading = bounds(surface, "markdown-settings-reading");
        let association = bounds(surface, "markdown-settings-association");
        let preview = bounds(surface, "markdown-settings-preview");
        assert!(title.bottom() <= enable.top());
        assert!(enable.bottom() <= reading.top());
        assert!(reading.bottom() <= association.top());
        assert!(association.bottom() <= preview.top());
        assert!(
            preview.bottom() > viewport.bottom(),
            "content must scroll, not compress"
        );

        let mut right = None;
        for (selector, title_id, control_id) in [
            (
                "markdown-default-font",
                "markdown-default-font-title",
                "markdown-default-font-control",
            ),
            (
                "markdown-default-wide",
                "markdown-default-wide-title",
                "markdown-default-wide-control",
            ),
            (
                "markdown-default-outline",
                "markdown-default-outline-title",
                "markdown-default-outline-control",
            ),
        ] {
            let label = bounds(surface, title_id);
            let control = bounds(surface, control_id);
            assert!(
                label.right() <= control.left(),
                "horizontal field: {selector}"
            );
            assert!((label.center().y - control.center().y).abs() <= px(1.));
            assert_eq!(control.size.height, px(32.));
            assert_eq!(control.size.width, px(168.));
            assert!(control.right() <= viewport.right());
            if let Some(right) = right {
                assert_eq!(control.right(), right, "shared control edge");
            }
            right = Some(control.right());
        }
        for label in ["宽版正文", "宽窗默认固定大纲"] {
            let switch = semantic_control(surface, label, Role::Switch, false);
            assert_eq!(switch.size, size(px(36.), px(20.)));
            assert_eq!(Some(switch.right()), right);
        }
        for selector in [
            "markdown-font-default-smaller",
            "markdown-font-default-larger",
        ] {
            assert_eq!(bounds(surface, selector).size.height, px(24.));
        }
        let caption = bounds(surface, "markdown-settings-close");
        scroll(surface, -2_000.);
        assert_eq!(bounds(surface, "markdown-settings-close"), caption);
        for (selector, label) in ASSOCIATION_ACTIONS {
            let action = reveal(surface, selector);
            assert_eq!(action.size.height, px(24.));
            semantic_control(surface, label, Role::Button, true);
        }
        let preview = reveal(surface, "markdown-settings-preview");
        assert_eq!(preview.size.height, px(24.));
        semantic_control(surface, "打开预览…", Role::Button, false);
        assert_eq!(bounds(surface, "markdown-settings-content"), viewport);
        assert_association_status(surface, ".md", "未核实");
        assert_association_status(surface, ".markdown", "未核实");
    }
}

#[gpui_kit::test]
fn markdown_settings_association_availability_and_busy_states(cx: &mut TestAppContext) {
    let (platform, _endpoint) = nxtools_ipc::platform_bridge(32);
    let (center, surface) = open_layout(cx, Some(Rc::new(platform)));
    for theme in [ThemePreference::Light, ThemePreference::Dark] {
        surface.update(|_, cx| crate::theme::set_theme_preference(theme, cx));
        surface.refresh().expect("requested settings theme");
        scroll(surface, 2_000.);
        let original_font_row = bounds(surface, "markdown-default-font-control");
        for (enabled, registered, is_default, saving, requesting) in [
            (true, false, false, false, false),
            (false, true, false, false, false),
            (true, true, true, false, false),
            (true, true, false, true, false),
            (true, true, false, false, true),
        ] {
            scroll(surface, 2_000.);
            surface.update(|window, cx| {
                let state = cx.global_mut::<Settings>();
                state.applied.enabled = enabled;
                state.saving = saving.then(Uuid::new_v4);
                state.request = requesting.then(|| Pending {
                    id: Uuid::new_v4(),
                    intent: Intent::Refresh,
                    started: Instant::now(),
                    source: window.window_handle(),
                });
                state.association = Some(MarkdownAssociationState {
                    md: MarkdownAssociationExtensionState {
                        registered,
                        is_default,
                    },
                    markdown: MarkdownAssociationExtensionState {
                        registered: false,
                        is_default: false,
                    },
                });
            });
            surface.refresh().expect("real availability state");
            let font_row = bounds(surface, "markdown-default-font-control");
            let busy = saving || requesting;
            assert_eq!(
                font_row, original_font_row,
                "busy must not move preference fields"
            );
            assert_eq!(
                surface.debug_bounds("markdown-settings-busy").is_some(),
                busy
            );
            for (selector, label) in [
                ("markdown-font-default-smaller", "减小默认字号"),
                ("markdown-font-default-larger", "增大默认字号"),
            ] {
                let target = reveal(surface, selector);
                semantic_control(surface, label, Role::Button, busy);
                if busy {
                    surface.simulate_click(target.center(), Modifiers::default());
                    surface.run_until_parked();
                }
            }
            for label in ["宽版正文", "宽窗默认固定大纲"] {
                let target = semantic_control(surface, label, Role::Switch, busy);
                if busy {
                    surface.simulate_click(target.center(), Modifiers::default());
                    surface.run_until_parked();
                }
            }
            for ((selector, label), disabled) in ASSOCIATION_ACTIONS.into_iter().zip([
                busy || !enabled,
                busy || !registered,
                busy || !registered,
                busy,
            ]) {
                let target = reveal(surface, selector);
                semantic_control(surface, label, Role::Button, disabled);
                if disabled {
                    surface.simulate_click(target.center(), Modifiers::default());
                    surface.run_until_parked();
                }
            }
            reveal(surface, "markdown-settings-preview");
            semantic_control(surface, "打开预览…", Role::Button, !enabled);
            assert_association_status(
                surface,
                ".md",
                if is_default {
                    "当前默认"
                } else if registered {
                    "已注册，非默认"
                } else {
                    "未注册"
                },
            );
            assert_association_status(surface, ".markdown", "未注册");
            scroll(surface, 2_000.);
            assert_eq!(bounds(surface, "markdown-default-font-control"), font_row);
            surface.read(|cx| {
                let state = cx.global::<Settings>();
                assert_eq!(state.saving.is_some(), saving);
                assert_eq!(state.request.is_some(), requesting);
                assert_eq!(
                    state.applied,
                    MarkdownReaderSettings {
                        enabled,
                        ..Default::default()
                    }
                );
            });
            assert!(center.read_with(surface, |center, _| center.items().is_empty()));
        }
    }
}

#[gpui_kit::test]
fn markdown_settings_unavailable_actions_and_font_limits_do_not_dispatch(cx: &mut TestAppContext) {
    let (center, surface) = open_layout(cx, None);
    for (selector, label) in ASSOCIATION_ACTIONS {
        let target = reveal(surface, selector);
        semantic_control(surface, label, Role::Button, true);
        surface.simulate_click(target.center(), Modifiers::default());
        surface.run_until_parked();
    }
    for (font, selector, label) in [
        (
            MarkdownReaderSettings::MIN_FONT_SIZE,
            "markdown-font-default-smaller",
            "减小默认字号",
        ),
        (
            MarkdownReaderSettings::MAX_FONT_SIZE,
            "markdown-font-default-larger",
            "增大默认字号",
        ),
    ] {
        surface.update(|_, cx| cx.global_mut::<Settings>().applied.font_size = font);
        surface.refresh().expect("font boundary");
        let target = reveal(surface, selector);
        semantic_control(surface, label, Role::Button, true);
        surface.simulate_click(target.center(), Modifiers::default());
        surface.run_until_parked();
        surface.read(|cx| assert_eq!(preferences(cx).font_size, font));
    }
    surface.update(|_, cx| cx.global_mut::<Settings>().applied.enabled = false);
    surface.refresh().expect("disabled preview");
    let preview = reveal(surface, "markdown-settings-preview");
    semantic_control(surface, "打开预览…", Role::Button, true);
    surface.simulate_click(preview.center(), Modifiers::default());
    surface.run_until_parked();
    surface.read(|cx| {
        let state = cx.global::<Settings>();
        assert!(state.saving.is_none() && state.request.is_none());
        assert!(
            cx.try_global::<ReaderWindow>()
                .is_none_or(|state| state.0.is_none())
        );
    });
    assert!(center.read_with(surface, |center, _| center.items().is_empty()));
}
