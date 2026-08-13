use gpui::{AnyElement, IntoElement, MouseButton, WindowControlArea, div, prelude::*, px};
use gpui_component::button::ButtonVariants;

use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::components::action::icon_action;
use crate::ui::metrics::{CONTROL_HEIGHT_ACTION, SPACING_12, TITLEBAR_HEIGHT};
use crate::ui::typography::{TEXT_BODY, UI_FONT_FAMILY};

pub(crate) fn client_titlebar(
    theme: Theme,
    selector: &'static str,
    close_id: &'static str,
    title: &'static str,
    on_close: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    div()
        .debug_selector(move || selector.into())
        .h(px(TITLEBAR_HEIGHT))
        .w_full()
        .flex_none()
        .pl(px(SPACING_12))
        .border_b_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .font_family(UI_FONT_FAMILY)
        .window_control_area(WindowControlArea::Drag)
        .flex()
        .items_center()
        .child(
            div()
                .flex_1()
                .text_size(px(TEXT_BODY))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .w(px(TITLEBAR_HEIGHT))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .debug_selector(move || close_id.into())
                        .size(px(CONTROL_HEIGHT_ACTION))
                        .rounded(px(crate::ui::metrics::RADIUS_CONTROL))
                        .hover(move |style| style.bg(theme.danger).text_color(gpui::white()))
                        .child(
                            icon_action(close_id, MaterialIcon::Close, "关闭")
                                .ghost()
                                .size(px(CONTROL_HEIGHT_ACTION))
                                .on_click(on_close),
                        ),
                ),
        )
        .into_any_element()
}
