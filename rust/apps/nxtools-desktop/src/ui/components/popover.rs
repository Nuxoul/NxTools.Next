use gpui::{AnyElement, IntoElement, ScrollHandle, div, prelude::*, px};
use gpui_component::scroll::ScrollableElement;

use crate::theme::Theme;
use crate::ui::metrics::{RADIUS_FLOATING, SPACING_4};

pub(crate) fn popover_shell(
    theme: Theme,
    selector: &'static str,
    width: f32,
    height: f32,
    content: AnyElement,
) -> AnyElement {
    div()
        .id(selector)
        .debug_selector(move || selector.into())
        .w(px(width))
        .h(px(height))
        .rounded(px(RADIUS_FLOATING))
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface_raised)
        .shadow_md()
        .overflow_hidden()
        .child(content)
        .into_any_element()
}

pub(crate) fn list_box(
    selector: &'static str,
    max_height: f32,
    scroll: ScrollHandle,
    items: Vec<AnyElement>,
) -> AnyElement {
    div()
        .id(selector)
        .debug_selector(move || selector.into())
        .w_full()
        .h(px(max_height))
        .min_h_0()
        .p(px(SPACING_4))
        .track_scroll(&scroll)
        .overflow_y_scroll()
        .vertical_scrollbar(&scroll)
        .children(items)
        .into_any_element()
}
