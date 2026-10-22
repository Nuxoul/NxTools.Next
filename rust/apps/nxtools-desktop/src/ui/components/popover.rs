use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::{AnyElement, IntoElement, ScrollHandle, div, prelude::*};

pub(crate) fn list_box(
    selector: &'static str,
    scroll: ScrollHandle,
    items: Vec<AnyElement>,
) -> AnyElement {
    div()
        .id(selector)
        .debug_selector(move || selector.into())
        .w_full()
        .flex_1()
        .min_h_0()
        .track_scroll(&scroll)
        .overflow_y_scroll()
        .vertical_scrollbar(&scroll)
        .children(items)
        .into_any_element()
}
