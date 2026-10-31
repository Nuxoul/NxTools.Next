use gpui_kit::{AnyElement, IntoElement, div, prelude::*, px};

use crate::theme::Theme;
use crate::ui::metrics::{SETTINGS_FOOTER_HEIGHT, TITLEBAR_HEIGHT};
use crate::ui::typography::UI_FONT_FAMILY;

pub(crate) struct DialogShellIds {
    pub root: &'static str,
    pub titlebar: &'static str,
    pub body: &'static str,
    pub footer: &'static str,
}

pub(crate) fn dialog_shell(
    theme: Theme,
    ids: DialogShellIds,
    titlebar: AnyElement,
    body: AnyElement,
    footer: AnyElement,
) -> AnyElement {
    div()
        .debug_selector(move || ids.root.into())
        .size_full()
        .bg(theme.canvas)
        .font_family(UI_FONT_FAMILY)
        .text_color(theme.text)
        .flex()
        .flex_col()
        .child(
            div()
                .debug_selector(move || ids.titlebar.into())
                .h(px(TITLEBAR_HEIGHT))
                .w_full()
                .flex_none()
                .child(titlebar),
        )
        .child(
            div()
                .debug_selector(move || ids.body.into())
                .w_full()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .child(body),
        )
        .child(
            div()
                .debug_selector(move || ids.footer.into())
                .h(px(SETTINGS_FOOTER_HEIGHT))
                .w_full()
                .flex_none()
                .border_t_1()
                .border_color(theme.border_subtle)
                .bg(theme.surface)
                .child(footer),
        )
        .into_any_element()
}
