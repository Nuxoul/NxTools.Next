use gpui::{AnyView, App, SharedString, Window, div, prelude::*, px};
use gpui_component::scroll::ScrollableElement;
use gpui_component::tooltip::Tooltip;

use crate::ui::typography::{LINE_HEIGHT_LABEL, TEXT_LABEL, UI_MONO_FONT_FAMILY};

pub(crate) const TOOLTIP_MAX_WIDTH: f32 = 344.0;
pub(crate) const TOOLTIP_MAX_HEIGHT: f32 = 136.0;
pub(crate) const TOOLTIP_SURFACE_MAX_WIDTH: f32 = 360.0;
pub(crate) const TOOLTIP_SURFACE_MAX_HEIGHT: f32 = 160.0;

pub(crate) fn bounded_text_tooltip(
    selector: impl Into<SharedString>,
    message: impl Into<SharedString>,
    monospace: bool,
) -> impl Fn(&mut Window, &mut App) -> AnyView {
    let selector = selector.into();
    let message = message.into();
    move |window, cx| {
        let tooltip_selector = selector.clone();
        let tooltip_message = message.clone();
        Tooltip::element(move |_, _| {
            let selector = tooltip_selector.clone();
            let message = tooltip_message.clone();
            div()
                .debug_selector(move || selector.to_string())
                .max_w(px(TOOLTIP_MAX_WIDTH))
                .max_h(px(TOOLTIP_MAX_HEIGHT))
                .overflow_y_scrollbar()
                .whitespace_normal()
                .when(monospace, |surface| {
                    surface.font_family(UI_MONO_FONT_FAMILY)
                })
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .child(message)
        })
        .max_w(px(TOOLTIP_SURFACE_MAX_WIDTH))
        .max_h(px(TOOLTIP_SURFACE_MAX_HEIGHT))
        .build(window, cx)
    }
}
