//! Production Foundation gates. Mount as `#[cfg(test)] mod kit_foundation_tests;`
//! in main_window.rs; these fixtures never connect to platform/store/user data.

use super::*;
use gpui_kit::component::Selectable;
use gpui_kit::{KeyUpEvent, Keystroke, Modifiers, TestAppContext, VisualTestContext, size};
use nxtools_core::LinkPanelPosition;

fn main_fixture(cx: &mut TestAppContext) -> (Entity<MainWindow>, VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let retained = Rc::new(RefCell::new(None));
    let retained_view = retained.clone();
    let (_, window) = cx.add_window_view(move |window, cx| {
        let main = cx.new(|cx| {
            MainWindow::new(
                gpui_kit::WeakEntity::<ClipboardApplication>::new_invalid(),
                gpui_kit::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                gpui_kit::WeakEntity::<DownloadApplication>::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            )
        });
        *retained_view.borrow_mut() = Some(main.clone());
        gpui_kit::base::Root::new(main, window, cx)
    });
    let main = retained
        .borrow()
        .as_ref()
        .expect("retained MainWindow")
        .clone();
    let handle = window.update(|window, _| window.window_handle());
    (main, VisualTestContext::from_window(handle, cx))
}

#[gpui_kit::test]
fn kit_foundation_navigation_matches_released_button_geometry(cx: &mut TestAppContext) {
    let (_, mut window) = main_fixture(cx);
    let (_, reference) = cx.add_window_view(|_, _| KitNavigationReference);
    reference.refresh().expect("direct Kit reference");
    let direct = reference
        .debug_bounds("kit-direct-navigation")
        .expect("Kit Button");
    for viewport in [size(px(1060.0), px(650.0)), size(px(920.0), px(480.0))] {
        window.simulate_resize(viewport);
        window.refresh().expect("production main viewport");
        let rail = window.debug_bounds("main-sidebar").expect("Kit Sidebar");
        assert_eq!(rail.size.width, px(176.0));
        for selector in [
            "main-module-下载",
            "main-module-速启",
            "main-module-剪贴板",
            "main-module-扩展",
            "main-module-压缩",
            "main-module-其他工具",
        ] {
            let entry = window.debug_bounds(selector).expect("module Button");
            assert_eq!(
                entry.size.height, direct.size.height,
                "ordinary navigation must retain released Button geometry"
            );
            assert!(entry.left() >= rail.left() && entry.right() <= rail.right());
        }
    }
}

#[gpui_kit::test]
fn extensions_workspace_entry_is_directly_below_clipboard(cx: &mut TestAppContext) {
    let (_, mut window) = main_fixture(cx);
    for viewport in [size(px(1060.0), px(650.0)), size(px(920.0), px(480.0))] {
        window.simulate_resize(viewport);
        window.update(|window, cx| window.draw(cx).clear(cx));
        let clipboard = window
            .debug_bounds("main-module-剪贴板")
            .expect("clipboard workspace entry");
        let extensions = window
            .debug_bounds("main-module-扩展")
            .expect("extensions must be a workspace entry, not inside tools");
        let tools = window
            .debug_bounds("main-module-其他工具")
            .expect("tools entry");
        assert!(clipboard.bottom() <= extensions.top() && extensions.bottom() <= tools.top());
        assert_eq!(clipboard.size.height, extensions.size.height);
        window.simulate_click(extensions.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(window.debug_bounds("tools-extension-manager").is_none());
        assert!(window.debug_bounds("extension-back-tools").is_none());
    }
}

struct KitNavigationReference;
impl Render for KitNavigationReference {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Button::new("kit-direct-navigation")
            .debug_selector(|| "kit-direct-navigation".into())
            .ghost()
            .selected(true)
            .icon(Icon::new(MainWindow::module_icon(MainModule::Downloads)))
            .label(MainModule::Downloads.label())
    }
}

#[gpui_kit::test]
fn kit_foundation_navigation_preserves_handles_and_release_activation(cx: &mut TestAppContext) {
    let (main, mut window) = main_fixture(cx);
    let transitions = Rc::new(std::cell::Cell::new(0));
    let observed = transitions.clone();
    let current_module = Rc::new(std::cell::Cell::new(MainModule::Downloads));
    let _observer = cx.update(|cx| {
        cx.observe(&main, move |main, cx| {
            let module = main.read(cx).shell.active_module();
            if current_module.replace(module) != module {
                observed.set(observed.get() + 1);
            }
        })
    });
    window.simulate_resize(size(px(920.0), px(480.0)));
    window.refresh().expect("production navigation");
    let download = window.debug_bounds("main-module-下载").expect("Downloads");
    window.simulate_click(download.center(), Modifiers::default());
    window.simulate_keystrokes("tab");
    window.refresh().expect("Quick Find focus");
    assert!(window.update(|window, cx| main.read(cx).module_focus[1].is_focused(window)));
    window.simulate_keystrokes("shift-tab");
    window.refresh().expect("reverse focus");
    assert!(window.update(|window, cx| main.read(cx).module_focus[0].is_focused(window)));
    for (index, module) in MainModule::ALL.into_iter().enumerate().skip(1) {
        window.simulate_keystrokes("tab");
        window.refresh().expect("next navigation focus");
        assert!(window.update(|window, cx| main.read(cx).module_focus[index].is_focused(window)));
        let before = transitions.get();
        let key = if module == MainModule::Tools {
            "space"
        } else {
            "enter"
        };
        window.simulate_keystrokes("ctrl-enter");
        window.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("ctrl-enter").expect("modified Enter"),
        });
        assert_eq!(
            transitions.get(),
            before,
            "modified activation is ineligible"
        );
        window.simulate_keystrokes(key);
        window.simulate_event(KeyDownEvent {
            keystroke: Keystroke::parse(key).expect("activation stroke"),
            is_held: true,
            prefer_character_input: false,
        });
        assert_eq!(transitions.get(), before, "held KeyDown must not dispatch");
        assert_ne!(
            window.read(|cx| main.read(cx).shell.active_module()),
            module,
            "KeyDown must not dispatch"
        );
        window.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse(key).expect("release"),
        });
        window.refresh().expect("release selects module");
        assert_eq!(
            transitions.get(),
            before + 1,
            "one gesture invokes one module transition"
        );
        assert_eq!(
            window.read(|cx| main.read(cx).shell.active_module()),
            module
        );
        assert!(
            window.update(|window, cx| main.read(cx).module_focus[index].is_focused(window)),
            "unmounted page focus must return to its module"
        );
    }
}

fn number_part_bounds(
    window: &mut VisualTestContext,
    id: &'static str,
) -> gpui_kit::Bounds<gpui_kit::Pixels> {
    window.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|snapshot| snapshot.path().last() == Some(&gpui_kit::ElementId::from(id)))
            .unwrap_or_else(|| panic!("actual NumberInput part {id}"))
            .bounds()
    })
}

fn scroll_tool_body_to_bottom(window: &mut VisualTestContext) {
    window.simulate_event(gpui_kit::ScrollEvent {
        position: gpui_kit::point(px(240.0), px(280.0)),
        delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -20.0)),
        ..Default::default()
    });
    window.run_until_parked();
    window
        .refresh()
        .expect("paint scrolled feature settings body");
}

#[gpui_kit::test]
fn kit_foundation_tools_form_keeps_fixed_budget_and_single_toggle_actions(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let retained = Rc::new(RefCell::new(None));
    let retained_view = retained.clone();
    let (_, window) = cx.add_window_view(move |window, cx| {
        let body = cx.new(|_| tools_page::ToolSettingsWindowBody::foundation_fixture());
        *retained_view.borrow_mut() = Some(body.clone());
        gpui_kit::base::Root::new(body, window, cx)
    });
    let body = retained
        .borrow()
        .as_ref()
        .expect("Tools production body")
        .clone();
    window.simulate_resize(size(px(460.0), px(320.0)));
    window.refresh().expect("compact real form");
    let root = window
        .debug_bounds("tool-settings-window")
        .expect("fixed Tools client");
    assert_eq!(root.size, size(px(460.0), px(320.0)));
    let minus = number_part_bounds(window, "decrement");
    let plus = number_part_bounds(window, "increment");
    let duration = window
        .debug_bounds("tool-settings-duration")
        .expect("actual NumberInput frame");
    assert_eq!(
        duration.size.height,
        px(32.0),
        "released default NumberInput frame (direct tier comparison below)"
    );
    assert_eq!(plus.size.height, minus.size.height);
    assert!(minus.left() >= duration.left() && plus.right() <= duration.right());
    assert!(
        window.debug_bounds("tool-settings-form").is_some(),
        "actual Form composition"
    );
    let bounds = window
        .debug_bounds("tool-settings-toggle")
        .expect("feature enable remains reachable");
    assert!(bounds.top() >= root.top() && bounds.bottom() <= root.bottom());
    assert!(bounds.left() >= root.left() && bounds.right() <= root.right());
    let enabled = window
        .debug_bounds("tool-settings-toggle")
        .expect("enabled Switch");
    window.simulate_click(enabled.center(), Modifiers::default());
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| body.read(cx).foundation_values()),
        (true, 6, LinkPanelPosition::Center)
    );
    window.simulate_keystrokes("space");
    window.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("space").expect("toggle release"),
    });
    window.run_until_parked();
    assert!(
        !window.read(|cx| body.read(cx).foundation_values().0),
        "one keyboard gesture toggles once"
    );
    window.simulate_click(plus.center(), Modifiers::default());
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| body.read(cx).foundation_values().1),
        8,
        "one action adds exactly two seconds"
    );
    for _ in 0..40 {
        window.simulate_click(plus.center(), Modifiers::default());
        window.run_until_parked();
    }
    assert_eq!(window.read(|cx| body.read(cx).foundation_values().1), 60);
    for _ in 0..40 {
        window.simulate_click(minus.center(), Modifiers::default());
        window.run_until_parked();
    }
    assert_eq!(window.read(|cx| body.read(cx).foundation_values().1), 2);
    let copy = include_str!("tools_page.rs");
    assert!(!copy.contains("tool-settings-copy-only"));
    assert!(!copy.contains("tool-settings-open-browser"));
    window.simulate_event(gpui_kit::ScrollEvent {
        position: gpui_kit::point(px(240.0), px(180.0)),
        delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, 20.0)),
        ..Default::default()
    });
    window.refresh().expect("restore top settings for Select");
    let position = window
        .debug_bounds("tool-settings-position")
        .expect("position dropdown");
    window.simulate_click(position.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("actual Select menu");
    window.simulate_keystrokes("down");
    window.run_until_parked();
    window.simulate_keystrokes("enter");
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| body.read(cx).foundation_values().2),
        LinkPanelPosition::TopRight
    );
}

#[gpui_kit::test]
fn foundation_alignment_navigation_has_fixed_left_icon_and_label_columns(cx: &mut TestAppContext) {
    let (_, mut window) = main_fixture(cx);
    for viewport in [size(px(1060.0), px(650.0)), size(px(920.0), px(480.0))] {
        window.simulate_resize(viewport);
        window.refresh().expect("actual navigation layout");
        let mut icon_left = None;
        let mut label_left = None;
        for (row_id, icon_id, label_id) in [
            (
                "main-module-下载",
                "main-module-下载-icon-slot",
                "main-module-下载-label-slot",
            ),
            (
                "main-module-速启",
                "main-module-速启-icon-slot",
                "main-module-速启-label-slot",
            ),
            (
                "main-module-剪贴板",
                "main-module-剪贴板-icon-slot",
                "main-module-剪贴板-label-slot",
            ),
            (
                "main-module-其他工具",
                "main-module-其他工具-icon-slot",
                "main-module-其他工具-label-slot",
            ),
        ] {
            let row = window.debug_bounds(row_id).expect("navigation row");
            let icon = window.debug_bounds(icon_id).expect("real icon column");
            let label = window
                .debug_bounds(label_id)
                .expect("real left-aligned label column");
            if let Some(left) = icon_left {
                assert_eq!(icon.left(), left);
            }
            if let Some(left) = label_left {
                assert_eq!(label.left(), left);
            }
            icon_left = Some(icon.left());
            label_left = Some(label.left());
            assert!(
                icon.left() - row.left() <= px(12.0),
                "navigation must be start-aligned, not a centered button cluster"
            );
            assert!((icon.center().y - label.center().y).abs() <= px(1.0));
            assert!(icon.right() < label.left() && label.right() <= row.right());
        }
    }
}

#[gpui_kit::test]
fn foundation_normalized_tool_fields_share_label_help_and_control_lanes(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, window) = cx.add_window_view(|window, cx| {
        let body = cx.new(|_| tools_page::ToolSettingsWindowBody::foundation_fixture());
        gpui_kit::base::Root::new(body, window, cx)
    });
    window.simulate_resize(size(px(460.0), px(320.0)));
    window.refresh().expect("actual normalized tool form");
    let mut label_start = None;
    let mut control_start = None;
    let mut previous_bottom = None;
    for (title_id, help_id, control_id) in [
        ("tool-dwell-label", "tool-dwell-help", "tool-dwell-control"),
        (
            "tool-position-label",
            "tool-position-help",
            "tool-position-control",
        ),
    ] {
        let title = window
            .debug_bounds(title_id)
            .expect("real field label lane");
        let help = window
            .debug_bounds(help_id)
            .expect("real field supporting lane");
        let control = window
            .debug_bounds(control_id)
            .expect("real field control lane");
        if let Some(left) = label_start {
            assert_eq!(title.left(), left);
        }
        if let Some(left) = control_start {
            assert_eq!(control.left(), left);
        }
        label_start = Some(title.left());
        control_start = Some(control.left());
        assert!(help.left() >= title.right(), "help is beside its title");
        assert!(
            help.right() < control.left(),
            "help remains in the information column, separate from controls"
        );
        assert!(
            control.size.width > px(140.0),
            "remaining form width supports a complete NumberInput/Select"
        );
        assert_eq!(
            control.right(),
            px(448.0),
            "control slot ends on the content edge"
        );
        assert!((title.center().y - control.center().y).abs() <= px(1.0));
        assert!((help.center().y - title.center().y).abs() <= px(1.0));
        if let Some(bottom) = previous_bottom {
            assert!(title.top() >= bottom + px(6.0));
        }
        previous_bottom = Some(help.bottom());
        assert!(help.left() >= px(12.0) && help.right() <= px(448.0));
    }
    let before_scroll = window
        .debug_bounds("tool-position-help")
        .expect("last help before scroll");
    scroll_tool_body_to_bottom(window);
    let last_help = window
        .debug_bounds("tool-position-help")
        .expect("last help after actual wheel");
    assert!(
        last_help.top() <= before_scroll.top(),
        "compact body remains stable or scrolls when needed"
    );
    assert!(
        last_help.top() >= px(48.0) && last_help.bottom() <= px(308.0),
        "last setting help is fully reachable inside fixed client: {last_help:?}"
    );
    let slot = window
        .debug_bounds("tool-dwell-control")
        .expect("dwell slot");
    let duration = window
        .debug_bounds("tool-settings-duration")
        .expect("Kit numeric frame");
    let minus = number_part_bounds(window, "decrement");
    let plus = number_part_bounds(window, "increment");
    assert_eq!(
        duration.size, slot.size,
        "complete NumberInput owns its one frame"
    );
    assert!(minus.left() >= duration.left() && minus.right() < plus.left());
    assert!(plus.right() <= duration.right() && plus.bottom() <= duration.bottom());
    let position = window
        .debug_bounds("tool-settings-position")
        .expect("position select");
    let position_slot = window
        .debug_bounds("tool-position-control")
        .expect("position slot");
    assert_eq!(position.left(), position_slot.left());
    assert_eq!(position.size.width, position_slot.size.width);
}

#[gpui_kit::test]
fn foundation_stepper_disables_at_original_bounds(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let retained = Rc::new(RefCell::new(None));
    let retained_view = retained.clone();
    let (_, window) = cx.add_window_view(move |window, cx| {
        let body = cx.new(|_| tools_page::ToolSettingsWindowBody::foundation_fixture());
        *retained_view.borrow_mut() = Some(body.clone());
        gpui_kit::base::Root::new(body, window, cx)
    });
    let body = retained.borrow().as_ref().expect("body").clone();
    window.simulate_resize(size(px(460.0), px(320.0)));
    window.refresh().expect("stepper frame");
    let minus = number_part_bounds(window, "decrement");
    for _ in 0..8 {
        window.simulate_click(minus.center(), Modifiers::default());
        window.run_until_parked();
    }
    assert_eq!(window.read(|cx| body.read(cx).foundation_values().1), 2);
    window.simulate_keystrokes("up");
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| body.read(cx).foundation_values().1),
        4,
        "numeric keyboard step remains two seconds"
    );
}

#[gpui_kit::test]
fn foundation_sidebar_icon_column_aligns_with_group_label_inset(cx: &mut TestAppContext) {
    let (_, mut window) = main_fixture(cx);
    for viewport in [size(px(1060.0), px(650.0)), size(px(920.0), px(480.0))] {
        window.simulate_resize(viewport);
        window.refresh().expect("sidebar frame");
        let rail = window.debug_bounds("main-sidebar").expect("rail");
        let icon = window
            .debug_bounds("main-module-下载-icon-slot")
            .expect("icon slot");
        // Kit Sidebar inner px_3 (12) + SidebarGroup label px_2 (8).
        assert_eq!(
            icon.left(),
            rail.left() + px(24.0),
            "navigation icon column must share the Kit group label edge"
        );
    }
}

#[gpui_kit::test]
fn foundation_tool_entry_aligns_status_and_action_with_title_line(cx: &mut TestAppContext) {
    let (main, mut window) = main_fixture(cx);
    window.update(|_, cx| {
        main.update(cx, |main, cx| {
            main.shell.activate(MainModule::Tools);
            cx.notify();
        })
    });
    for viewport in [size(px(1060.0), px(650.0)), size(px(920.0), px(480.0))] {
        window.simulate_resize(viewport);
        window.refresh().expect("tool entry frame");
        let title = window
            .debug_bounds("tool-link-detector-title")
            .expect("title");
        assert!(
            window
                .debug_bounds("tool-link-detector-description")
                .is_none(),
            "tool explanation is a Tooltip, not a second row"
        );
        let icon = window
            .debug_bounds("tool-link-detector-icon")
            .expect("tile icon");
        let status = window
            .debug_bounds("tool-link-detector-status")
            .expect("tile status");
        let configure = window
            .debug_bounds("tool-link-detector-configure")
            .expect("tile action");
        assert!(icon.bottom() <= title.top() && title.bottom() <= status.top());
        assert!(status.bottom() <= configure.top());
        assert!((icon.center().x - title.center().x).abs() <= px(1.0));
        assert!((status.center().x - title.center().x).abs() <= px(1.0));
    }
}

#[test]
fn foundation_dedicated_controls_are_real_kit_number_input_and_select() {
    let source = include_str!("tools_page.rs");
    let render = source
        .split("impl Render for ToolSettingsWindowBody {")
        .nth(1)
        .and_then(|part| part.split("fn open_tool_settings_window(").next())
        .expect("production tool settings render");
    assert!(
        render.contains("NumberInput::new("),
        "duration must use the styled numeric component"
    );
    assert!(
        render.contains("Select::new("),
        "position must use the styled select component"
    );
    assert!(
        !render.contains("dropdown_menu_with_anchor("),
        "do not reconstruct a select from a button menu"
    );
}

#[gpui_kit::test]
fn foundation_dedicated_numeric_edits_and_select_cancel_keep_valid_setting(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let retained = Rc::new(RefCell::new(None));
    let retained_view = retained.clone();
    let (_, window) = cx.add_window_view(move |window, cx| {
        let body = cx.new(|_| tools_page::ToolSettingsWindowBody::foundation_fixture());
        *retained_view.borrow_mut() = Some(body.clone());
        gpui_kit::base::Root::new(body, window, cx)
    });
    let body = retained.borrow().as_ref().expect("production body").clone();
    window.simulate_resize(size(px(460.0), px(320.0)));
    // GPUI excludes inactive windows from focus-event paths, unlike is_focused.
    window.update(|window, _| window.activate_window());
    window.run_until_parked();
    assert!(window.update(|window, _| window.is_window_active()));
    window.refresh().expect("initial dedicated controls");
    let input = window.read(|cx| body.read(cx).dwell_input());
    let selection = window.read(|cx| body.read(cx).position_input());
    assert_eq!(window.read(|cx| input.read(cx).value().to_string()), "6");
    assert_eq!(
        window.read(|cx| selection.read(cx).selected_value().copied()),
        Some(LinkPanelPosition::Center)
    );
    window.update(|window, cx| input.update(cx, |state, cx| state.replace_all("12", window, cx)));
    window.run_until_parked();
    assert_eq!(window.read(|cx| body.read(cx).foundation_values().1), 12);
    for invalid in ["", "1", "99", "abc", "1.5"] {
        window.update(|window, cx| {
            input.update(cx, |state, cx| state.replace_all(invalid, window, cx))
        });
        window.run_until_parked();
        assert_eq!(
            window.read(|cx| body.read(cx).foundation_values().1),
            12,
            "invalid/intermediate input is not committed"
        );
    }
    window.update(|window, cx| {
        input.update(cx, |state, cx| {
            state.focus(window, cx);
            state.replace_all("99", window, cx);
        });
    });
    window.run_until_parked();
    window
        .refresh()
        .expect("paint focused numeric editor before blur");
    assert!(
        window.update(|window, cx| {
            gpui_kit::Focusable::focus_handle(input.read(cx), cx).is_focused(window)
        }),
        "numeric input owns actual focus before leaving"
    );
    assert_eq!(window.read(|cx| input.read(cx).value().to_string()), "99");
    let outside = window
        .debug_bounds("tool-settings-position")
        .expect("other real control");
    window.simulate_click(outside.center(), Modifiers::default());
    window
        .refresh()
        .expect("paint actual input-to-Select focus transition");
    window.run_until_parked();
    assert!(
        !window.update(|window, cx| {
            gpui_kit::Focusable::focus_handle(input.read(cx), cx).is_focused(window)
        }),
        "focus leaves numeric input for the real Select"
    );
    assert_eq!(
        window.read(|cx| input.read(cx).value().to_string()),
        "12",
        "real blur restores the last valid seconds before any cancel key"
    );
    window.simulate_keystrokes("escape");
    window.run_until_parked();
    let position = window
        .debug_bounds("tool-settings-position")
        .expect("Select trigger");
    window.simulate_click(position.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("actual open select");
    window.simulate_keystrokes("down");
    window.simulate_keystrokes("escape");
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| body.read(cx).foundation_values().2),
        LinkPanelPosition::Center,
        "cursor change without confirm is not persisted"
    );
    assert_eq!(
        window.read(|cx| selection.read(cx).selected_value().copied()),
        Some(LinkPanelPosition::Center)
    );
}

struct ToolControlTierReference {
    input: Entity<InputState>,
    position: Entity<gpui_kit::component::select::SelectState<Vec<tools_page::PositionChoice>>>,
}

impl Render for ToolControlTierReference {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        use gpui_kit::component::input::NumberInput;
        use gpui_kit::component::select::Select;
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .debug_selector(|| "reference-number-default".into())
                    .w(px(160.0))
                    .child(NumberInput::new(&self.input).suffix("秒")),
            )
            .child(
                div()
                    .debug_selector(|| "reference-number-small".into())
                    .w(px(160.0))
                    .child(NumberInput::new(&self.input).small().suffix("秒")),
            )
            .child(
                div()
                    .debug_selector(|| "reference-select-default".into())
                    .w(px(160.0))
                    .child(Select::new(&self.position)),
            )
            .child(
                div()
                    .debug_selector(|| "reference-select-small".into())
                    .w(px(160.0))
                    .child(Select::new(&self.position).small()),
            )
    }
}

#[gpui_kit::test]
fn foundation_tool_controls_keep_released_default_tiers_without_slot_compression(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, reference) = cx.add_window_view(|window, cx| {
        let input = cx.new(|cx| InputState::new(window, cx));
        let position = cx.new(|cx| {
            gpui_kit::component::select::SelectState::new(
                Vec::<tools_page::PositionChoice>::new(),
                None,
                window,
                cx,
            )
        });
        let view = cx.new(|_| ToolControlTierReference { input, position });
        gpui_kit::base::Root::new(view, window, cx)
    });
    reference
        .refresh()
        .expect("direct released tier references");
    let reference_tiers = [
        ("reference-number-default", "reference-number-small"),
        ("reference-select-default", "reference-select-small"),
    ]
    .map(|(default, small)| {
        (
            reference
                .debug_bounds(default)
                .expect("released default tier")
                .size
                .height,
            reference
                .debug_bounds(small)
                .expect("released Small tier")
                .size
                .height,
        )
    });
    let _ = reference;
    let (_, production) = cx.add_window_view(|window, cx| {
        let body = cx.new(|_| tools_page::ToolSettingsWindowBody::foundation_fixture());
        gpui_kit::base::Root::new(body, window, cx)
    });
    production.simulate_resize(size(px(460.0), px(320.0)));
    production
        .refresh()
        .expect("production tool form tier comparison");
    for ((control, slot), (default, small)) in [
        ("tool-settings-duration", "tool-dwell-control"),
        ("tool-settings-position", "tool-position-control"),
    ]
    .into_iter()
    .zip(reference_tiers)
    {
        let actual = production
            .debug_bounds(control)
            .expect("production component");
        let slot = production
            .debug_bounds(slot)
            .expect("production control lane");
        assert!(default > small, "comparison distinguishes official tiers");
        assert_eq!(
            actual.size.height, default,
            "{control} retains the released default tier"
        );
        assert!(
            slot.size.height >= actual.size.height,
            "layout must not compress the component"
        );
        assert!(actual.left() >= slot.left() && actual.right() <= slot.right());
        assert!(actual.top() >= slot.top() && actual.bottom() <= slot.bottom());
    }
}

#[gpui_kit::test]
fn foundation_tool_header_separates_feature_state_from_window_close(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, window) = cx.add_window_view(|window, cx| {
        let body = cx.new(|_| tools_page::ToolSettingsWindowBody::foundation_fixture());
        gpui_kit::base::Root::new(body, window, cx)
    });
    window.simulate_resize(size(px(460.0), px(320.0)));
    window
        .refresh()
        .expect("actual feature header role grouping");
    let status = window
        .debug_bounds("tool-settings-enabled-status")
        .expect("feature state");
    let toggle = window
        .debug_bounds("tool-settings-toggle")
        .expect("feature switch");
    let close = window
        .debug_bounds("tool-settings-close")
        .expect("window close");
    let description = window
        .debug_bounds("tool-settings-description")
        .expect("body description");
    assert!(
        close.bottom() <= description.top(),
        "description belongs below the titlebar divider"
    );
    assert!(
        (description.center().y - toggle.center().y).abs() <= px(1.0),
        "feature description trigger belongs beside enable label"
    );
    assert!(
        toggle.top() > close.bottom(),
        "feature toggle must not remain in window chrome"
    );
    assert!((status.center().y - toggle.center().y).abs() <= px(1.0));
}

#[gpui_kit::test]
fn feature_descriptions_use_compact_kit_tooltip_triggers_not_inline_copy(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, window) = cx.add_window_view(|window, cx| {
        let body = cx.new(|_| tools_page::ToolSettingsWindowBody::foundation_fixture());
        gpui_kit::base::Root::new(body, window, cx)
    });
    window.simulate_resize(size(px(460.0), px(320.0)));
    window.refresh().expect("actual feature help presentation");
    for help in ["tool-dwell-help", "tool-position-help"] {
        let trigger = window
            .debug_bounds(help)
            .expect("discoverable help trigger");
        assert!(
            trigger.size.width <= px(24.0) && trigger.size.height <= px(24.0),
            "{help} is an information trigger, not a permanent text row: {trigger:?}"
        );
    }
}

#[gpui_kit::test]
fn installed_feedback_global_action_is_centered_inside_titlebar(cx: &mut TestAppContext) {
    let (_, mut surface) = main_fixture(cx);
    surface.simulate_resize(size(px(1060.0), px(650.0)));
    surface.refresh().expect("production titlebar");
    let bar = surface.debug_bounds("main-titlebar").expect("titlebar");
    let action = surface
        .debug_bounds("global-network-settings-button")
        .expect("settings button");
    assert!(
        (bar.center().y - action.center().y).abs() <= px(1.0),
        "settings button and hover frame must be vertically centered: {bar:?} {action:?}"
    );
}

#[test]
fn installed_feedback_tool_and_notification_surfaces_have_distinct_roles() {
    let tools = include_str!("tools_page.rs");
    let tile = tools
        .split("ListItem::new(\"tool-link-detector-item\")")
        .nth(1)
        .expect("tool tile");
    assert!(
        tile.contains(".bg(theme.surface)"),
        "tile default must not use hover color"
    );
    assert!(
        !tile.contains("Tag::success()"),
        "enabled state is subordinate text/dot, not vivid tag"
    );
    let notify = include_str!("../notifications/presentation.rs");
    assert!(
        notify.contains("NotificationList::new(window, cx)")
            && notify.contains("Notification::new()"),
        "released official card/list own notification surfaces"
    );
    assert!(
        !notify.contains("GroupBox::new()") && !notify.contains("Alert::new("),
        "no hand-composed replacement notification shell"
    );
}

#[gpui_kit::test]
fn recurrence_light_sidebar_paints_active_selection_not_dark_fill(cx: &mut TestAppContext) {
    let (_, mut window) = main_fixture(cx);
    window.simulate_resize(size(px(1060.0), px(650.0)));
    for preference in [
        nxtools_core::ThemePreference::Light,
        nxtools_core::ThemePreference::Dark,
    ] {
        window.update(|_, cx| crate::theme::set_theme_preference(preference, cx));
        window.run_until_parked();
        window
            .refresh()
            .expect("production sidebar theme transition");
        let row = window
            .debug_bounds("main-module-下载")
            .expect("selected production row");
        let expected = window.update(|_, cx| {
            gpui_kit::component::Theme::global(cx)
                .colors
                .secondary_active
        });
        let paints_selected = window.update(|window, _| {
            let scale = window.scale_factor();
            window.painted_quads().iter().any(|quad| {
                quad.background == gpui_kit::solid_background(expected)
                    && (quad.bounds.origin.x.as_f32() / scale - row.origin.x.as_f32()).abs() < 1.0
                    && (quad.bounds.origin.y.as_f32() / scale - row.origin.y.as_f32()).abs() < 1.0
                    && (quad.bounds.size.width.as_f32() / scale - row.size.width.as_f32()).abs()
                        < 1.0
            })
        });
        assert!(
            paints_selected,
            "selected module must paint {preference:?} official Ghost selected fill, never hardcoded dark"
        );
    }
}
