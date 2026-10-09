use gpui::{App, ClickEvent, Div, SharedString, Window, div, prelude::*, px};
use gpui_component::Icon;

use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::metrics::{SPACING_4, SPACING_8, SPACING_12};
use crate::ui::typography::{LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL};

pub(crate) const WORK_STATE_ICON_SIZE: f32 = 18.0;
pub(crate) const WORK_STATE_MIN_HEIGHT: f32 = 72.0;

#[derive(Clone, Copy)]
enum WorkStateTone {
    Neutral,
    Progress,
    Error,
}

pub(crate) fn empty_state(
    theme: Theme,
    selector: &'static str,
    icon: MaterialIcon,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
) -> Div {
    work_state(
        theme,
        selector,
        icon,
        title.into(),
        description.into(),
        WorkStateTone::Neutral,
    )
}

pub(crate) fn progress_state(
    theme: Theme,
    selector: &'static str,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
) -> Div {
    work_state(
        theme,
        selector,
        MaterialIcon::ProgressActivity,
        title.into(),
        description.into(),
        WorkStateTone::Progress,
    )
}

pub(crate) fn inline_error_state(
    theme: Theme,
    selector: &'static str,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
) -> Div {
    work_state(
        theme,
        selector,
        MaterialIcon::Info,
        title.into(),
        description.into(),
        WorkStateTone::Error,
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

#[derive(gpui::IntoElement)]
struct InteractiveWorkState {
    selector: &'static str,
    state: Div,
    on_click: crate::ui::components::action::ClickHandler,
}

impl gpui::RenderOnce for InteractiveWorkState {
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
        let focused = focus.is_focused(window);
        crate::ui::components::action::focused_action(
            selector,
            &focus,
            Theme::zed_dark(),
            false,
            window,
            on_click,
        )
        .border_0()
        .debug_selector(move || selector.into())
        .w_full()
        .flex_none()
        .min_h(px(WORK_STATE_MIN_HEIGHT))
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

fn work_state(
    theme: Theme,
    selector: &'static str,
    icon: MaterialIcon,
    title: SharedString,
    description: SharedString,
    tone: WorkStateTone,
) -> Div {
    let (semantic_color, title_color, shows_error_edge) = match tone {
        WorkStateTone::Neutral => (theme.icon_muted, theme.text_secondary, false),
        WorkStateTone::Progress => (theme.accent, theme.text_secondary, false),
        WorkStateTone::Error => (theme.danger, theme.danger, true),
    };
    let icon_selector = format!("{selector}-icon");
    let title_selector = format!("{selector}-title");
    let description_selector = format!("{selector}-description");
    let marker_selector = format!("{selector}-marker");

    div()
        .debug_selector(move || selector.into())
        .relative()
        .w_full()
        .min_h(px(WORK_STATE_MIN_HEIGHT))
        .flex_none()
        .px(px(SPACING_12))
        .py(px(SPACING_12))
        .when(shows_error_edge, |state| {
            state.child(
                div()
                    .debug_selector(move || marker_selector.clone())
                    .absolute()
                    .left_0()
                    .top(px(SPACING_12))
                    .bottom(px(SPACING_12))
                    .w(px(2.0))
                    .bg(theme.danger),
            )
        })
        .flex()
        .items_center()
        .gap(px(SPACING_8))
        .child(
            div()
                .debug_selector(move || icon_selector.clone())
                .size(px(WORK_STATE_ICON_SIZE))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    Icon::new(icon)
                        .size(px(WORK_STATE_ICON_SIZE))
                        .text_color(semantic_color),
                ),
        )
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
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .text_color(title_color)
                        .child(title),
                )
                .child(
                    div()
                        .debug_selector(move || description_selector.clone())
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child(description),
                ),
        )
}

#[cfg(test)]
mod tests {
    use gpui::{Context, IntoElement, Render, TestAppContext, Window, div, point, px, size};

    use super::*;

    struct StateFixture;

    impl Render for StateFixture {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::zed_dark();
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(empty_state(
                    theme,
                    "state-empty",
                    MaterialIcon::Search,
                    "没有内容",
                    "调整条件后重试。",
                ))
                .child(progress_state(
                    theme,
                    "state-progress",
                    "正在加载",
                    "正在读取有界结果。",
                ))
                .child(inline_error_state(
                    theme,
                    "state-error",
                    "加载失败",
                    "请查看诊断信息后重试。",
                ))
        }
    }

    #[gpui::test]
    fn shared_work_states_keep_one_icon_and_typography_geometry(cx: &mut TestAppContext) {
        let window = cx.add_empty_window();
        window.draw(
            point(px(0.0), px(0.0)),
            size(px(480.0), px(WORK_STATE_MIN_HEIGHT * 3.0)),
            |_window, cx| cx.new(|_| StateFixture),
        );

        let mut origins = Vec::new();
        for selector in ["state-empty", "state-progress", "state-error"] {
            let state = window.debug_bounds(selector).expect("shared state surface");
            let icon = window
                .debug_bounds(Box::leak(format!("{selector}-icon").into_boxed_str()))
                .expect("shared state icon slot");
            let title = window
                .debug_bounds(Box::leak(format!("{selector}-title").into_boxed_str()))
                .expect("shared state title");
            let description = window
                .debug_bounds(Box::leak(
                    format!("{selector}-description").into_boxed_str(),
                ))
                .expect("shared state description");

            assert_eq!(state.size.width, px(480.0));
            assert_eq!(state.size.height, px(WORK_STATE_MIN_HEIGHT));
            assert_eq!(
                icon.size,
                size(px(WORK_STATE_ICON_SIZE), px(WORK_STATE_ICON_SIZE))
            );
            assert_eq!(title.size.height, px(LINE_HEIGHT_BODY));
            assert_eq!(description.size.height, px(LINE_HEIGHT_LABEL));
            assert_eq!(title.left(), description.left());
            origins.push((icon.left(), title.left(), description.left()));
        }
        assert!(origins.windows(2).all(|pair| pair[0] == pair[1]));
    }
}
