use gpui::{MouseButton, div, prelude::*, px};

use crate::theme::Theme;

#[derive(Clone, Copy)]
enum WindowAction {
    Minimize,
    Maximize,
    Close,
}

pub fn title_bar(theme: Theme) -> impl IntoElement {
    div()
        .id("title-bar")
        .h(px(42.0))
        .w_full()
        .flex()
        .items_center()
        .border_b_1()
        .border_color(theme.border)
        .bg(theme.surface)
        .on_mouse_down(MouseButton::Left, |_event, window, _cx| {
            window.start_window_move();
        })
        .on_click(|event, window, _cx| {
            if event.click_count() == 2 {
                window.zoom_window();
            }
        })
        .child(
            div()
                .px_3()
                .flex()
                .items_center()
                .gap_2()
                .text_sm()
                .text_color(theme.text)
                .child(
                    div()
                        .size(px(18.0))
                        .rounded_md()
                        .bg(theme.accent)
                        .text_color(theme.canvas)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .child("N"),
                )
                .child("NxTools Next"),
        )
        .child(div().flex_1())
        .child(
            div()
                .px_3()
                .text_xs()
                .text_color(theme.text_muted)
                .child("PLATFORM PROOF"),
        )
        .child(div().flex_1())
        .children([
            window_control("—", "minimize", theme, WindowAction::Minimize),
            window_control("□", "maximize", theme, WindowAction::Maximize),
            window_control("×", "close", theme, WindowAction::Close),
        ])
}

pub fn sidebar(theme: Theme) -> impl IntoElement {
    div()
        .w(px(214.0))
        .h_full()
        .flex_none()
        .flex()
        .flex_col()
        .border_r_1()
        .border_color(theme.border)
        .bg(theme.surface)
        .p_2()
        .gap_1()
        .child(sidebar_item("⌕", "快速查找", false, theme))
        .child(sidebar_item("▣", "剪贴板", true, theme))
        .child(sidebar_item("⇩", "下载", false, theme))
        .child(sidebar_item("✦", "悬浮助手", false, theme))
        .child(div().flex_1())
        .child(
            div()
                .m_1()
                .p_3()
                .rounded_lg()
                .border_1()
                .border_color(theme.border)
                .bg(theme.canvas)
                .text_xs()
                .text_color(theme.text_muted)
                .child("Rust owns product state")
                .child(
                    div()
                        .mt_1()
                        .text_color(theme.success)
                        .child("● IPC server ready"),
                ),
        )
}

fn sidebar_item(
    icon: &'static str,
    label: &'static str,
    selected: bool,
    theme: Theme,
) -> impl IntoElement {
    div()
        .h(px(36.0))
        .px_3()
        .rounded_md()
        .flex()
        .items_center()
        .gap_3()
        .bg(if selected {
            theme.accent_soft
        } else {
            theme.surface
        })
        .text_color(if selected {
            theme.text
        } else {
            theme.text_muted
        })
        .hover(move |style| style.bg(theme.surface_raised).text_color(theme.text))
        .child(
            div()
                .w(px(18.0))
                .text_center()
                .text_color(if selected {
                    theme.accent
                } else {
                    theme.text_muted
                })
                .child(icon),
        )
        .child(label)
}

fn window_control(
    label: &'static str,
    id: &'static str,
    theme: Theme,
    action: WindowAction,
) -> impl IntoElement {
    div()
        .id(id)
        .w(px(46.0))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.text_muted)
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
            cx.stop_propagation();
        })
        .hover(move |style| {
            if id == "close" {
                style.bg(theme.danger).text_color(theme.canvas)
            } else {
                style.bg(theme.surface_raised).text_color(theme.text)
            }
        })
        .on_click(move |_event, window, cx| {
            cx.stop_propagation();
            match action {
                WindowAction::Minimize => window.minimize_window(),
                WindowAction::Maximize => window.zoom_window(),
                WindowAction::Close => window.remove_window(),
            }
        })
        .child(label)
}
