use gpui_kit::component::alert::Alert;
use gpui_kit::component::empty::{Empty, EmptyContent};
use gpui_kit::component::label::Label;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{Icon, Sizable};
#[cfg(test)]
use gpui_kit::px;
use gpui_kit::{App, ClickEvent, Div, SharedString, Window, div, prelude::*};

use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::metrics::ordinary_rem;

pub(crate) const WORK_STATE_ICON_SIZE: f32 = 18.0;
pub(crate) const WORK_STATE_MIN_HEIGHT: f32 = 72.0;

pub(crate) fn empty_state(
    _theme: Theme,
    selector: &'static str,
    icon: MaterialIcon,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
) -> Div {
    let icon_selector = format!("{selector}-icon");
    let title_selector = format!("{selector}-title");
    let description_selector = format!("{selector}-description");
    let content_selector = format!("{selector}-kit-content");
    div()
        .debug_selector(move || selector.into())
        .w_full()
        .min_h(ordinary_rem(WORK_STATE_MIN_HEIGHT))
        .flex_none()
        .child(
            Empty::new()
                .p_3()
                .gap_0()
                .items_start()
                .text_left()
                .content(
                    EmptyContent::new()
                        .w_full()
                        .max_w_full()
                        .items_start()
                        .child(
                            div()
                                .debug_selector(move || content_selector.clone())
                                .w_full()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .debug_selector(move || icon_selector.clone())
                                        .flex_none()
                                        .child(
                                            Icon::new(icon)
                                                .size(ordinary_rem(WORK_STATE_ICON_SIZE)),
                                        ),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .debug_selector(move || title_selector.clone())
                                                .child(Label::new(title).text_sm()),
                                        )
                                        .child(
                                            div()
                                                .debug_selector(move || {
                                                    description_selector.clone()
                                                })
                                                .child(Label::new(description).text_sm()),
                                        ),
                                ),
                        ),
                ),
        )
}

/// A loading message, never a determinate percentage. The original owner
/// decides when this surface is mounted; there is no new task/timer controller.
pub(crate) fn progress_state(
    _theme: Theme,
    selector: &'static str,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
) -> Div {
    let title_selector = format!("{selector}-title");
    let description_selector = format!("{selector}-description");
    let icon_selector = format!("{selector}-icon");
    div()
        .debug_selector(move || selector.into())
        .w_full()
        .min_h(ordinary_rem(WORK_STATE_MIN_HEIGHT))
        .flex_none()
        .p_3()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .debug_selector(move || icon_selector.clone())
                .flex_none()
                .child(LoadingIndicator),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .debug_selector(move || title_selector.clone())
                        .child(Label::new(title).text_sm()),
                )
                .child(
                    div()
                        .debug_selector(move || description_selector.clone())
                        .child(Label::new(description).text_sm()),
                ),
        )
}

#[derive(gpui_kit::IntoElement)]
struct LoadingIndicator;
impl gpui_kit::RenderOnce for LoadingIndicator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        if crate::ui::motion::motion_allowed(cx) {
            Spinner::new().small().into_any_element()
        } else {
            Icon::new(MaterialIcon::ProgressActivity)
                .small()
                .into_any_element()
        }
    }
}

pub(crate) fn inline_error_state(
    _theme: Theme,
    selector: &'static str,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
) -> Div {
    let alert_selector = format!("{selector}-kit-alert");
    div()
        .debug_selector(move || selector.into())
        .w_full()
        .min_h(ordinary_rem(WORK_STATE_MIN_HEIGHT))
        .flex_none()
        .child(
            div()
                .debug_selector(move || alert_selector.clone())
                .w_full()
                .child(
                    Alert::error(selector, description.into())
                        .xsmall()
                        .title(title),
                ),
        )
}

pub(crate) fn interactive_state(
    selector: &'static str,
    state: Div,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    InteractiveWorkState {
        selector,
        state,
        on_click: Box::new(on_click),
    }
}
#[derive(gpui_kit::IntoElement)]
struct InteractiveWorkState {
    selector: &'static str,
    state: Div,
    on_click: crate::ui::components::action::ClickHandler,
}
impl gpui_kit::RenderOnce for InteractiveWorkState {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            selector,
            state,
            on_click,
        } = self;
        let focus = window
            .use_keyed_state(
                SharedString::from(format!("{selector}-focus")),
                cx,
                |_, cx| cx.focus_handle().tab_stop(true),
            )
            .read(cx)
            .clone();
        let focused = focus.is_focused(window) && window.last_input_was_keyboard();
        crate::ui::components::action::focused_action(
            selector,
            &focus,
            Theme::zed_dark(),
            false,
            window,
            cx,
            on_click,
        )
        .border_0()
        .debug_selector(move || selector.into())
        .w_full()
        .flex_none()
        .min_h(ordinary_rem(WORK_STATE_MIN_HEIGHT))
        .child(state)
        .when(focused, |state| {
            state.child(
                div()
                    .absolute()
                    .inset_0()
                    .border_2()
                    .border_color(Theme::zed_dark().focus_ring),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Context, Render, TestAppContext, size};
    struct States;
    impl Render for States {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w_full()
                .flex()
                .flex_col()
                .child(empty_state(
                    Theme::zed_dark(),
                    "state-empty",
                    MaterialIcon::Search,
                    "没有内容",
                    "调整条件后重试。",
                ))
                .child(progress_state(
                    Theme::zed_dark(),
                    "state-progress",
                    "正在加载",
                    "正在读取有界结果。",
                ))
                .child(inline_error_state(
                    Theme::zed_dark(),
                    "state-error",
                    "加载失败",
                    "请查看诊断信息后重试。",
                ))
        }
    }
    #[gpui_kit::test]
    fn shared_work_states_keep_actual_component_content_inside_original_minimum_budget(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let (_, surface) = cx.add_window_view(|_, _| States);
        surface.simulate_resize(size(px(480.0), px(300.0)));
        surface.refresh().expect("real shared states");
        for id in ["state-empty", "state-progress", "state-error"] {
            let bounds = surface.debug_bounds(id).expect("state");
            assert_eq!(bounds.size.width, px(480.0));
            assert!(bounds.size.height >= px(WORK_STATE_MIN_HEIGHT));
            assert!(bounds.right() <= px(480.0) && bounds.bottom() <= px(300.0));
        }
    }
}
