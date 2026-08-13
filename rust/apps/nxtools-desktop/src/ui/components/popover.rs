use gpui::{AnyElement, IntoElement, div, prelude::*, px};
use gpui_component::scroll::ScrollableElement;

use crate::theme::Theme;
use crate::ui::metrics::{RADIUS_FLOATING, SPACING_4};

#[allow(dead_code)]
pub(crate) fn popover_shell(
    theme: Theme,
    selector: &'static str,
    max_height: f32,
    content: AnyElement,
) -> AnyElement {
    div()
        .debug_selector(move || selector.into())
        .max_h(px(max_height))
        .rounded(px(RADIUS_FLOATING))
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface_raised)
        .shadow_md()
        .overflow_hidden()
        .child(content)
        .into_any_element()
}

#[allow(dead_code)]
pub(crate) fn list_box(content: AnyElement) -> AnyElement {
    div()
        .w_full()
        .max_h_full()
        .p(px(SPACING_4))
        .overflow_y_scrollbar()
        .child(content)
        .into_any_element()
}
