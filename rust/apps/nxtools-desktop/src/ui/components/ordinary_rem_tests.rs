//! Queue07: real shared components under production Root, not helper arithmetic.
use super::*;
use crate::icons::MaterialIcon;
use crate::theme::Theme;
use gpui_kit::component::Theme as ComponentTheme;
use gpui_kit::component::form::{Field, Form};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{Sizable, Size};
use gpui_kit::{
    AppContext, Context, Entity, IntoElement, Modifiers, Render, TestAppContext, VisualTestContext,
    Window, div, prelude::*, px, size,
};
use std::cell::Cell;
use std::rc::Rc;

fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
}
fn base(surface: &mut VisualTestContext, value: f32) {
    surface.update(|window, cx| {
        ComponentTheme::update(cx, |theme| theme.font_size = px(value));
        window.refresh();
    });
    draw(surface);
    assert_eq!(surface.update(|window, _| window.rem_size()), px(value));
}
fn near(actual: gpui_kit::Pixels, expected: f32, context: &str) {
    assert!(
        (actual - px(expected)).abs() <= px(0.5),
        "{context}: {actual:?} != {expected}"
    );
}
fn key(surface: &mut VisualTestContext, stroke: &'static str) {
    surface.simulate_keystrokes(stroke);
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse(stroke).expect("static activation"),
    });
    draw(surface);
}
struct FormSurface {
    input: Entity<InputState>,
    calls: Rc<Cell<usize>>,
    long: bool,
}
impl Render for FormSurface {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let calls = self.calls.clone();
        let path = field::path_picker_shell(
            "shared-rem-path",
            field::field_shell(
                Theme::zed_dark(),
                "shared-rem-input",
                Input::new(&self.input).aria_label("合成路径"),
            ),
            action::icon_action("shared-rem-picker", MaterialIcon::Folder, "合成选择")
                .with_size(Size::Medium)
                .on_click(move |_, _, _| calls.set(calls.get() + 1))
                .into_any_element(),
        );
        let apply = self.calls.clone();
        settings::settings_shell(
            Theme::zed_dark(),
            settings::SettingsShellIds {
                root: "shared-rem-shell",
                content: "shared-rem-body",
                footer: "shared-rem-footer",
            },
            titlebar::client_titlebar(
                Theme::zed_dark(),
                "shared-rem-titlebar",
                "shared-rem-close",
                "合成共享表单",
                |_, _, _| {},
            ),
            div()
                .id("shared-rem-scroll")
                .debug_selector(|| "shared-rem-scroll".into())
                .size_full()
                .overflow_y_scroll()
                .p_3()
                .child(Form::new().child(Field::new().label("路径").child(path)))
                .child(field::inline_error(
                    Theme::zed_dark(),
                    "shared-rem-validation",
                    if self.long {
                        "合成完整错误需要在原内容滚动区域中可达。\n".repeat(40)
                    } else {
                        "合成错误：保留输入与原动作。".into()
                    },
                ))
                .child(
                    div()
                        .debug_selector(|| "shared-rem-last".into())
                        .child("最后一行"),
                )
                .into_any_element(),
            settings::dialog_footer(
                action::action_button("shared-rem-apply", "合成应用")
                    .on_click(move |_, _, _| apply.set(apply.get() + 1))
                    .into_any_element(),
            ),
        )
    }
}
#[gpui_kit::test]
fn audit06_shared_form_path_tracks_actual_input_and_fixed_footer(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let calls = Rc::new(Cell::new(0));
    let retained = calls.clone();
    let slot = Rc::new(std::cell::RefCell::new(None));
    let held = slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| FormSurface {
            input: cx.new(|cx| InputState::new(window, cx)),
            calls: retained,
            long: false,
        });
        *held.borrow_mut() = Some(view.clone());
        gpui_kit::base::Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(380.0), px(240.0)));
    let original = surface.read(|cx| ComponentTheme::global(cx).clone());
    let view = slot.borrow().as_ref().expect("retained form").clone();
    let identity = view.read_with(surface, |view, _| view.input.entity_id());
    for value in [16.0, 14.0, 18.0] {
        view.update(surface, |_, cx| cx.notify());
        base(surface, value);
        let path = surface
            .debug_bounds("shared-rem-path")
            .expect("actual path composition");
        let input = surface
            .debug_bounds("shared-rem-input")
            .expect("actual Input");
        let picker = surface
            .debug_bounds("shared-rem-picker")
            .expect("actual Button");
        near(
            path.size.height,
            32.0 * value / 16.0,
            "path rail follows Input",
        );
        near(input.size.height, 32.0 * value / 16.0, "Medium Input");
        near(picker.size.height, 32.0 * value / 16.0, "Medium Button");
        near(
            picker.left() - input.right(),
            8.0 * value / 16.0,
            "ordinary gap",
        );
        assert!(input.top() >= path.top() && input.bottom() <= path.bottom());
        let footer = surface
            .debug_bounds("shared-rem-footer")
            .expect("fixed footer");
        let apply = surface
            .debug_bounds("shared-rem-apply")
            .expect("original footer action");
        assert_eq!(footer.size.height, px(48.0));
        assert_eq!(footer.bottom(), px(240.0));
        assert!(apply.top() >= footer.top() && apply.bottom() <= footer.bottom());
        surface.simulate_click(picker.center(), Modifiers::default());
        draw(surface);
    }
    assert_eq!(calls.get(), 3, "one pointer dispatch per shared action");
    let input = view.read_with(surface, |view, _| view.input.clone());
    surface.update(|window, cx| input.update(cx, |state, cx| state.focus(window, cx)));
    surface.simulate_input("合成路径尾部");
    draw(surface);
    assert_eq!(
        input.read_with(surface, |state, _| state.value().to_string()),
        "合成路径尾部"
    );
    assert_eq!(input.entity_id(), identity);
    key(surface, "tab");
    let focused = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.focused() == Some(true))
            .expect("actual focused picker")
    });
    assert_eq!(focused.label(), Some("合成选择"));
    key(surface, "space");
    assert_eq!(calls.get(), 4, "same Button release owner");
    view.update(surface, |view, cx| {
        view.long = true;
        cx.notify();
    });
    draw(surface);
    let viewport = surface
        .debug_bounds("shared-rem-scroll")
        .expect("original scroll viewport");
    let before = surface
        .debug_bounds("shared-rem-last")
        .expect("long error end");
    assert!(before.bottom() > viewport.bottom());
    surface.simulate_event(gpui_kit::ScrollEvent {
        position: viewport.center(),
        delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.0), px(-5000.0))),
        ..Default::default()
    });
    draw(surface);
    let end = surface
        .debug_bounds("shared-rem-last")
        .expect("reachable complete error end");
    assert!(
        end.top() >= viewport.top() && end.bottom() <= viewport.bottom(),
        "{end:?} vs {viewport:?}"
    );
    let apply = surface
        .debug_bounds("shared-rem-apply")
        .expect("fixed action after scroll");
    surface.simulate_click(apply.center(), Modifiers::default());
    draw(surface);
    assert_eq!(calls.get(), 5);
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
}
struct States;
impl Render for States {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .flex()
            .flex_col()
            .child(state::empty_state(
                Theme::zed_dark(),
                "shared-rem-empty",
                MaterialIcon::Search,
                "无内容",
                "调整条件",
            ))
            .child(state::progress_state(
                Theme::zed_dark(),
                "shared-rem-progress",
                "加载中",
                "不是百分比",
            ))
            .child(state::inline_error_state(
                Theme::zed_dark(),
                "shared-rem-error",
                "读取失败",
                "可重试",
            ))
    }
}
#[gpui_kit::test]
fn audit06_shared_empty_error_ordinary_geometry_follows_root(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, surface) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| States);
        gpui_kit::base::Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(480.0), px(320.0)));
    let original = surface.read(|cx| ComponentTheme::global(cx).clone());
    for value in [16.0, 14.0, 18.0] {
        base(surface, value);
        let empty = surface
            .debug_bounds("shared-rem-empty")
            .expect("actual Empty");
        near(
            empty.size.height,
            72.0 * value / 16.0,
            "ordinary state minimum",
        );
        near(
            surface
                .debug_bounds("shared-rem-empty-icon")
                .expect("ordinary vector icon")
                .size
                .width,
            18.0 * value / 16.0,
            "ordinary icon",
        );
        for selector in ["shared-rem-progress", "shared-rem-error"] {
            let bounds = surface.debug_bounds(selector).expect("actual state");
            assert!(bounds.size.height >= px(72.0 * value / 16.0) && bounds.bottom() <= px(320.0));
        }
    }
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
}
struct Rows;
impl Render for Rows {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .flex()
            .flex_col()
            .child(volume_scope::volume_scope_row(
                Theme::zed_dark(),
                cx,
                "shared-rem-volume",
                "合成卷",
                "合成状态",
                volume_scope::VolumeScopeRowState {
                    maintenance: None,
                    checked: false,
                    show_divider: false,
                },
                |_, _, _| {},
            ))
            .child(quick_launch_diagnostics::quick_launch_diagnostics(
                Theme::zed_dark(),
                quick_launch_diagnostics::QuickLaunchDiagnosticsData {
                    ranked_items: 1,
                    total_activations: 2,
                    query_associations: 3,
                    recent_directories: 4,
                    local_search_elapsed: "10 µs".into(),
                    index_search_elapsed: "20 µs".into(),
                    log_path: "合成日志路径".into(),
                    log_available: false,
                },
                quick_launch_diagnostics::QuickLaunchDiagnosticsActions {
                    clear_history: Rc::new(|_, _, _| {}),
                    open_logs: Rc::new(|_, _, _| {}),
                },
                cx,
            ))
    }
}
#[gpui_kit::test]
fn audit06_shared_volume_and_diagnostics_scale_without_clipping(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, surface) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| Rows);
        gpui_kit::base::Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(720.0), px(480.0)));
    let original = surface.read(|cx| ComponentTheme::global(cx).clone());
    for value in [16.0, 14.0, 18.0] {
        base(surface, value);
        near(
            surface
                .debug_bounds("shared-rem-volume")
                .expect("actual ListItem")
                .size
                .height,
            40.0 * value / 16.0,
            "volume row",
        );
        near(
            surface
                .debug_bounds("shared-rem-volume-title")
                .expect("actual type line")
                .size
                .height,
            20.0 * value / 16.0,
            "ordinary workbench line",
        );
        near(
            surface
                .debug_bounds("quick-launch-diagnostics-log-row")
                .expect("actual diagnostic row")
                .size
                .height,
            48.0 * value / 16.0,
            "diagnostic row",
        );
        let row = surface
            .debug_bounds("quick-launch-diagnostics-log-row")
            .expect("log row");
        let path = surface
            .debug_bounds("quick-launch-diagnostics-log-path")
            .expect("full path owner");
        assert!(path.top() >= row.top() && path.bottom() <= row.bottom());
    }
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
}
struct Header;
impl Render for Header {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        titlebar::ModuleHeader::new(
            "shared-rem-header",
            "shared-rem-header-title",
            "shared-rem-header-meta",
            "合成标题",
            "合成元数据",
        )
        .trailing(action::action_button("shared-rem-header-action", "合成操作").into_any_element())
        .render(Theme::zed_dark())
    }
}
#[gpui_kit::test]
fn audit06_shared_module_header_type_and_action_share_ordinary_scale(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, surface) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| Header);
        gpui_kit::base::Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(480.0), px(100.0)));
    let original = surface.read(|cx| ComponentTheme::global(cx).clone());
    for value in [16.0, 14.0, 18.0] {
        base(surface, value);
        let header = surface
            .debug_bounds("shared-rem-header")
            .expect("real module header");
        near(
            header.size.height,
            48.0 * value / 16.0,
            "ordinary module rail",
        );
        near(
            surface
                .debug_bounds("shared-rem-header-title")
                .expect("actual title line")
                .size
                .height,
            22.0 * value / 16.0,
            "title role",
        );
        let action = surface
            .debug_bounds("shared-rem-header-action")
            .expect("original styled action");
        assert!(action.top() >= header.top() && action.bottom() <= header.bottom());
    }
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
}

struct Editors {
    inputs: custom_command_editor::CustomCommandEditorInputs,
}
impl Render for Editors {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use custom_command_editor::{
            CustomCommandEditorActions, CustomCommandEditorInputs, CustomCommandEditorState,
        };
        let commands = [nxtools_core::CustomCommandSettings {
            title: "合成命令".into(),
            keyword: "test".into(),
            ..Default::default()
        }];
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(hotkey::hotkey_field(
                Theme::zed_dark(),
                "shared-rem-hotkey",
                "Ctrl+Space",
                "合成状态",
                hotkey::HotkeyFieldState::Applied,
                None,
                |_, _, _| {},
            ))
            .child(path_list::path_exclusion_setting(
                Theme::zed_dark(),
                &[r"C:\synthetic\合成路径".into()],
                None,
                |_, _, _| {},
                |_, _, _, _| {},
                |_, _, _| {},
                cx,
            ))
            .child(custom_command_editor::custom_command_editor(
                Theme::zed_dark(),
                CustomCommandEditorState {
                    commands: &commands,
                    selected_index: Some(0),
                    inputs: CustomCommandEditorInputs {
                        keyword: self.inputs.keyword.clone(),
                        title: self.inputs.title.clone(),
                        program: self.inputs.program.clone(),
                        arguments: self.inputs.arguments.clone(),
                        working_directory: self.inputs.working_directory.clone(),
                    },
                    run_as_admin: false,
                    run_silently: false,
                },
                CustomCommandEditorActions {
                    select: Rc::new(|_, _, _| {}),
                    add: Rc::new(|_, _, _| {}),
                    set_enabled: Rc::new(|_, _, _| {}),
                    delete: Rc::new(|_, _, _| {}),
                    pick_program: Rc::new(|_, _, _| {}),
                    set_run_as_admin: Rc::new(|_, _, _| {}),
                    set_run_silently: Rc::new(|_, _, _| {}),
                    save: Rc::new(|_, _, _| {}),
                },
                cx,
            ))
    }
}
#[gpui_kit::test]
fn audit06_shared_editors_ordinary_roles_keep_fixed_viewport_and_delete_hit(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, surface) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| Editors {
            inputs: custom_command_editor::CustomCommandEditorInputs {
                keyword: cx.new(|cx| InputState::new(window, cx)),
                title: cx.new(|cx| InputState::new(window, cx)),
                program: cx.new(|cx| InputState::new(window, cx)),
                arguments: cx.new(|cx| InputState::new(window, cx)),
                working_directory: cx.new(|cx| InputState::new(window, cx)),
            },
        });
        gpui_kit::base::Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(720.0), px(600.0)));
    let original = surface.read(|cx| ComponentTheme::global(cx).clone());
    for value in [16.0, 14.0, 18.0] {
        base(surface, value);
        near(
            surface
                .debug_bounds("shared-rem-hotkey")
                .expect("actual recorder")
                .size
                .width,
            260.0 * value / 16.0,
            "ordinary hotkey width",
        );
        near(
            surface
                .debug_bounds("quick-launch-exclusion-path-0")
                .expect("actual path line")
                .size
                .height,
            20.0 * value / 16.0,
            "ordinary path line",
        );
        assert_eq!(
            surface
                .debug_bounds("quick-launch-exclusion-delete-0")
                .expect("approved delete hit")
                .size,
            size(px(40.0), px(40.0))
        );
        near(
            surface
                .debug_bounds("quick-launch-custom-command-master")
                .expect("actual master")
                .size
                .width,
            224.0 * value / 16.0,
            "ordinary master rail",
        );
        assert_eq!(
            surface
                .debug_bounds("quick-launch-custom-command-master-detail")
                .expect("bounded editor")
                .size
                .height,
            px(360.0)
        );
        near(
            surface
                .debug_bounds("quick-launch-command-field-keyword")
                .expect("actual Form Input")
                .size
                .height,
            32.0 * value / 16.0,
            "original command Input",
        );
    }
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
}
