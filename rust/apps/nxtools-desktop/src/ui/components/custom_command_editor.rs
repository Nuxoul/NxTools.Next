use std::rc::Rc;

use gpui::{AnyElement, App, Entity, IntoElement, Window, div, prelude::*, px};
use gpui_component::button::ButtonVariants;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputState};
use gpui_component::{Sizable, Size};
use nxtools_core::CustomCommandSettings;

use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::components::action::{
    action_button, danger_ghost_button_variant, focusable_content, icon_action,
    labelled_icon_action, primary_button_variant,
};
use crate::ui::metrics::{
    CONTROL_HEIGHT_ACTION, CONTROL_HEIGHT_INPUT, RADIUS_CONTROL, SPACING_4, SPACING_8, SPACING_12,
    SPACING_16,
};
use crate::ui::typography::{
    LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL, UI_MONO_FONT_FAMILY,
};

const COMMAND_MASTER_WIDTH: f32 = 224.0;
const COMMAND_EDITOR_HEIGHT: f32 = 360.0;
const COMMAND_MASTER_HEADER_HEIGHT: f32 = 40.0;
const COMMAND_DETAIL_HEADER_HEIGHT: f32 = 48.0;
const COMMAND_MASTER_ROW_HEIGHT: f32 = 40.0;

pub(crate) type SelectHandler =
    Rc<dyn Fn(&(usize, CustomCommandSettings), &mut Window, &mut App) + 'static>;
pub(crate) type ClickHandler = Rc<dyn Fn(&(), &mut Window, &mut App) + 'static>;
pub(crate) type ToggleHandler = Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>;

pub(crate) struct CustomCommandEditorActions {
    pub select: SelectHandler,
    pub add: ClickHandler,
    pub set_enabled: ToggleHandler,
    pub delete: ClickHandler,
    pub pick_program: ClickHandler,
    pub set_run_as_admin: ToggleHandler,
    pub set_run_silently: ToggleHandler,
    pub save: ClickHandler,
}

pub(crate) struct CustomCommandEditorInputs {
    pub keyword: Entity<InputState>,
    pub title: Entity<InputState>,
    pub program: Entity<InputState>,
    pub arguments: Entity<InputState>,
    pub working_directory: Entity<InputState>,
}

pub(crate) struct CustomCommandEditorState<'a> {
    pub commands: &'a [CustomCommandSettings],
    pub selected_index: Option<usize>,
    pub inputs: CustomCommandEditorInputs,
    pub run_as_admin: bool,
    pub run_silently: bool,
}

pub(crate) fn custom_command_editor(
    theme: Theme,
    state: CustomCommandEditorState<'_>,
    actions: CustomCommandEditorActions,
    cx: &App,
) -> AnyElement {
    let CustomCommandEditorState {
        commands,
        selected_index,
        inputs,
        run_as_admin,
        run_silently,
    } = state;
    let mut list = div()
        .id("quick-launch-custom-command-list")
        .debug_selector(|| "quick-launch-custom-command-list".into())
        .min_h_0()
        .flex_1()
        .overflow_y_scroll()
        .flex()
        .flex_col();
    for (index, command) in commands.iter().cloned().enumerate() {
        let on_select = Rc::clone(&actions.select);
        list = list.child(command_master_row(
            theme,
            index,
            command,
            selected_index == Some(index),
            move |command, window, cx| on_select(&(index, command), window, cx),
        ));
    }
    if commands.is_empty() {
        list = list.child(
            div()
                .debug_selector(|| "quick-launch-custom-command-empty".into())
                .h(px(COMMAND_MASTER_ROW_HEIGHT))
                .px(px(SPACING_12))
                .flex()
                .items_center()
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .text_color(theme.text_muted)
                .child("还没有自定义命令"),
        );
    }

    let add = Rc::clone(&actions.add);
    let add_action =
        labelled_icon_action("quick-launch-add-custom-command", MaterialIcon::Add, "新增")
            .ghost()
            .on_click(move |_event, window, cx| add(&(), window, cx));

    let master = div()
        .debug_selector(|| "quick-launch-custom-command-master".into())
        .w(px(COMMAND_MASTER_WIDTH))
        .h_full()
        .min_h_0()
        .flex_none()
        .border_r_1()
        .border_color(theme.border_subtle)
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(COMMAND_MASTER_HEADER_HEIGHT))
                .px(px(SPACING_12))
                .flex_none()
                .border_b_1()
                .border_color(theme.border_subtle)
                .flex()
                .items_center()
                .gap(px(SPACING_8))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .child("自定义命令"),
                )
                .child(bounded_action_slot(
                    "quick-launch-add-custom-command",
                    add_action.into_any_element(),
                )),
        )
        .child(list);

    let detail = if let Some(index) = selected_index {
        let command = commands.get(index).cloned().unwrap_or_default();
        command_detail(
            theme,
            command.enabled,
            inputs,
            run_as_admin,
            run_silently,
            actions,
            cx,
        )
    } else {
        div()
            .debug_selector(|| "quick-launch-custom-command-detail".into())
            .min_w_0()
            .h_full()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(TEXT_LABEL))
            .line_height(px(LINE_HEIGHT_LABEL))
            .text_color(theme.text_muted)
            .child("选择左侧命令后编辑")
            .into_any_element()
    };

    div()
        .debug_selector(|| "quick-launch-custom-command-master-detail".into())
        .w_full()
        .h(px(COMMAND_EDITOR_HEIGHT))
        .overflow_hidden()
        .rounded(px(RADIUS_CONTROL))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .flex()
        .child(master)
        .child(detail)
        .into_any_element()
}

fn command_master_row(
    theme: Theme,
    index: usize,
    command: CustomCommandSettings,
    selected: bool,
    on_select: impl Fn(CustomCommandSettings, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let selector = format!("quick-launch-command-{index}");
    let title_selector = format!("quick-launch-command-title-{index}");
    let context_selector = format!("quick-launch-command-context-{index}");
    let selection_selector = format!("quick-launch-command-selection-{index}");
    let title = if command.title.is_empty() {
        "未命名命令".to_owned()
    } else {
        command.title.clone()
    };
    let keyword = if command.keyword.is_empty() {
        "未设置关键词".to_owned()
    } else {
        command.keyword.clone()
    };
    let context = if command.enabled {
        keyword
    } else {
        format!("{keyword} · 已停用")
    };
    let selected_command = command.clone();
    let content = div()
        .id(("quick-launch-command", index))
        .debug_selector(move || selector.clone())
        .w_full()
        .h(px(COMMAND_MASTER_ROW_HEIGHT))
        .flex_none()
        .relative()
        .border_b_1()
        .border_color(theme.border_subtle)
        .bg(if selected {
            theme.element_selected
        } else {
            theme.canvas
        })
        .cursor_pointer()
        .px(px(SPACING_12))
        .flex()
        .items_center()
        .when(!selected, |row| {
            row.hover(move |style| style.bg(theme.surface_hover))
        })
        .when(selected, |row| {
            row.child(
                div()
                    .debug_selector(move || selection_selector.clone())
                    .absolute()
                    .left_0()
                    .top_0()
                    .h(px(COMMAND_MASTER_ROW_HEIGHT))
                    .w(px(2.0))
                    .bg(theme.accent),
            )
        })
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(SPACING_4))
                .child(
                    div()
                        .debug_selector(move || title_selector.clone())
                        .h(px(LINE_HEIGHT_BODY))
                        .truncate()
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .text_color(if command.enabled {
                            theme.text
                        } else {
                            theme.text_muted
                        })
                        .child(title),
                )
                .child(
                    div()
                        .debug_selector(move || context_selector.clone())
                        .h(px(LINE_HEIGHT_LABEL))
                        .truncate()
                        .font_family(UI_MONO_FONT_FAMILY)
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child(context),
                ),
        )
        .into_any_element();
    focusable_content(
        ("quick-launch-command-action", index),
        theme,
        false,
        content,
        move |_, window, cx| on_select(selected_command.clone(), window, cx),
    )
}

fn command_detail(
    theme: Theme,
    enabled: bool,
    inputs: CustomCommandEditorInputs,
    run_as_admin: bool,
    run_silently: bool,
    actions: CustomCommandEditorActions,
    cx: &App,
) -> AnyElement {
    let set_enabled = Rc::clone(&actions.set_enabled);
    let enabled_control = command_checkbox(
        "quick-launch-command-enabled",
        "启用",
        enabled,
        move |checked, window, cx| set_enabled(&checked, window, cx),
    );
    let delete = Rc::clone(&actions.delete);
    let delete_action = action_button("quick-launch-delete-custom-command", "删除")
        .custom(danger_ghost_button_variant(theme, cx))
        .on_click(move |_event, window, cx| delete(&(), window, cx));

    let pick_program = Rc::clone(&actions.pick_program);
    let picker_action = icon_action(
        "quick-launch-pick-command-program",
        MaterialIcon::Folder,
        "选择程序",
    )
    .ghost()
    .on_click(move |_event, window, cx| pick_program(&(), window, cx));

    let set_admin = Rc::clone(&actions.set_run_as_admin);
    let admin = command_checkbox(
        "quick-launch-command-admin",
        "管理员运行",
        run_as_admin,
        move |checked, window, cx| set_admin(&checked, window, cx),
    );
    let set_silent = Rc::clone(&actions.set_run_silently);
    let silent = command_checkbox(
        "quick-launch-command-silent",
        "静默运行",
        run_silently,
        move |checked, window, cx| set_silent(&checked, window, cx),
    );
    let save = Rc::clone(&actions.save);
    let save_action = action_button("quick-launch-save-custom-command", "保存")
        .custom(primary_button_variant(theme, cx))
        .on_click(move |_event, window, cx| save(&(), window, cx));

    div()
        .debug_selector(|| "quick-launch-custom-command-detail".into())
        .min_w_0()
        .h_full()
        .flex_1()
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(COMMAND_DETAIL_HEADER_HEIGHT))
                .px(px(SPACING_16))
                .flex_none()
                .border_b_1()
                .border_color(theme.border_subtle)
                .flex()
                .items_center()
                .gap(px(SPACING_8))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .child("命令详情"),
                )
                .child(enabled_control)
                .child(bounded_action_slot(
                    "quick-launch-delete-custom-command",
                    delete_action.into_any_element(),
                )),
        )
        .child(
            div()
                .min_h_0()
                .flex_1()
                .p(px(SPACING_16))
                .flex()
                .flex_col()
                .gap(px(SPACING_8))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .gap(px(SPACING_8))
                        .child(command_input_field(
                            theme,
                            "quick-launch-command-field-keyword",
                            "关键词",
                            &inputs.keyword,
                        ))
                        .child(command_input_field(
                            theme,
                            "quick-launch-command-field-title",
                            "显示名称",
                            &inputs.title,
                        )),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_end()
                        .gap(px(SPACING_8))
                        .child(command_input_field(
                            theme,
                            "quick-launch-command-field-program",
                            "程序",
                            &inputs.program,
                        ))
                        .child(bounded_action_slot(
                            "quick-launch-pick-command-program",
                            picker_action.into_any_element(),
                        )),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .gap(px(SPACING_8))
                        .child(command_input_field(
                            theme,
                            "quick-launch-command-field-arguments",
                            "参数模板",
                            &inputs.arguments,
                        ))
                        .child(command_input_field(
                            theme,
                            "quick-launch-command-field-working-directory",
                            "工作目录",
                            &inputs.working_directory,
                        )),
                )
                .child(
                    div()
                        .h(px(CONTROL_HEIGHT_ACTION))
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(SPACING_16))
                        .child(admin)
                        .child(silent)
                        .child(div().flex_1())
                        .child(bounded_action_slot(
                            "quick-launch-save-custom-command",
                            save_action.into_any_element(),
                        )),
                ),
        )
        .into_any_element()
}

fn command_input_field(
    theme: Theme,
    selector: &'static str,
    label: &'static str,
    input: &Entity<InputState>,
) -> AnyElement {
    div()
        .min_w_0()
        .flex_1()
        .flex()
        .flex_col()
        .gap(px(SPACING_4))
        .child(
            div()
                .h(px(LINE_HEIGHT_LABEL))
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .text_color(theme.text_muted)
                .child(label),
        )
        .child(
            div()
                .debug_selector(move || selector.into())
                .h(px(CONTROL_HEIGHT_INPUT))
                .rounded(px(RADIUS_CONTROL))
                .border_1()
                .border_color(theme.border)
                .bg(theme.canvas)
                .px(px(SPACING_8))
                .flex()
                .items_center()
                .child(
                    Input::new(input)
                        .appearance(false)
                        .bordered(false)
                        .focus_bordered(false),
                ),
        )
        .into_any_element()
}

fn command_checkbox(
    selector: &'static str,
    label: &'static str,
    checked: bool,
    on_click: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .debug_selector(move || selector.into())
        .h(px(CONTROL_HEIGHT_ACTION))
        .flex()
        .items_center()
        .child(
            Checkbox::new(selector)
                .checked(checked)
                .with_size(Size::Small)
                .h_full()
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .child(label)
                .on_click(move |checked, window, cx| on_click(*checked, window, cx)),
        )
        .into_any_element()
}

fn bounded_action_slot(selector: &'static str, action: AnyElement) -> AnyElement {
    div()
        .debug_selector(move || selector.into())
        .h(px(CONTROL_HEIGHT_ACTION))
        .flex_none()
        .flex()
        .items_center()
        .child(action)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    struct CommandFixture {
        selected: Rc<Cell<bool>>,
    }
    impl gpui::Render for CommandFixture {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let selected = self.selected.clone();
            div().w(px(COMMAND_MASTER_WIDTH)).child(command_master_row(
                Theme::zed_dark(),
                0,
                CustomCommandSettings::default(),
                false,
                move |_, _, _| selected.set(true),
            ))
        }
    }
    #[gpui::test]
    fn custom_command_master_keyboard_selects_without_executing(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_component::init);
        cx.update(crate::theme::apply_component_theme);
        let selected = Rc::new(Cell::new(false));
        let rendered_selected = selected.clone();
        let (_, window) = cx.add_window_view(move |_, _| CommandFixture {
            selected: rendered_selected,
        });
        window
            .refresh()
            .expect("render actual custom command master row");
        let row = window
            .debug_bounds("quick-launch-command-0")
            .expect("master hit bounds");
        window.update(|window, _| window.focus_next());
        window.refresh().expect("focus command row");
        window.simulate_event(gpui::KeyDownEvent {
            keystroke: gpui::Keystroke::parse("enter").expect("select key"),
            is_held: false,
        });
        assert!(!selected.get(), "selection waits for release");
        window.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").expect("select release"),
        });
        assert!(
            selected.get(),
            "keyboard must select the editor row, not run a command"
        );
        window
            .refresh()
            .expect("focused command row remains bounded");
        assert_eq!(window.debug_bounds("quick-launch-command-0"), Some(row));
        assert_eq!(row.size.height, px(40.0));
    }
}
