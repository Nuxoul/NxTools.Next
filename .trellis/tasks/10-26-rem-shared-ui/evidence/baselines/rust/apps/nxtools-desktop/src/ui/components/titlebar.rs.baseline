use gpui_kit::component::button::ButtonVariants;
use gpui_kit::{
    AnyElement, IntoElement, MouseButton, SharedString, WindowControlArea, div, prelude::*, px,
};

use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::components::action::icon_action;
use crate::ui::metrics::{MODULE_HEADER_HEIGHT, SPACING_8, SPACING_12, TITLEBAR_HEIGHT};
use crate::ui::typography::{
    LINE_HEIGHT_LABEL, LINE_HEIGHT_PAGE_TITLE, TEXT_BODY, TEXT_LABEL, TEXT_PAGE_TITLE,
    UI_FONT_FAMILY, UI_MONO_FONT_FAMILY,
};

pub(crate) struct ModuleHeader {
    selector: &'static str,
    title_selector: &'static str,
    metadata_selector: &'static str,
    title: SharedString,
    metadata: SharedString,
    metadata_monospace: bool,
    trailing: Option<AnyElement>,
}

impl ModuleHeader {
    pub(crate) fn new(
        selector: &'static str,
        title_selector: &'static str,
        metadata_selector: &'static str,
        title: impl Into<SharedString>,
        metadata: impl Into<SharedString>,
    ) -> Self {
        Self {
            selector,
            title_selector,
            metadata_selector,
            title: title.into(),
            metadata: metadata.into(),
            metadata_monospace: false,
            trailing: None,
        }
    }

    pub(crate) fn metadata_monospace(mut self) -> Self {
        self.metadata_monospace = true;
        self
    }

    pub(crate) fn trailing(mut self, trailing: AnyElement) -> Self {
        self.trailing = Some(trailing);
        self
    }

    pub(crate) fn render(self, theme: Theme) -> AnyElement {
        let Self {
            selector,
            title_selector,
            metadata_selector,
            title,
            metadata,
            metadata_monospace,
            trailing,
        } = self;
        div()
            .id(selector)
            .debug_selector(move || selector.into())
            .h(px(MODULE_HEADER_HEIGHT))
            .w_full()
            .flex_none()
            .px(px(SPACING_12))
            .border_b_1()
            .border_color(theme.border_subtle)
            .bg(theme.canvas)
            .font_family(UI_FONT_FAMILY)
            .flex()
            .items_center()
            .gap(px(SPACING_8))
            .child(
                div()
                    .id(title_selector)
                    .debug_selector(move || title_selector.into())
                    .flex_none()
                    .h(px(LINE_HEIGHT_PAGE_TITLE))
                    .text_size(px(TEXT_PAGE_TITLE))
                    .line_height(px(LINE_HEIGHT_PAGE_TITLE))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(
                div()
                    .id(metadata_selector)
                    .debug_selector(move || metadata_selector.into())
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .h(px(LINE_HEIGHT_LABEL))
                    .text_size(px(TEXT_LABEL))
                    .line_height(px(LINE_HEIGHT_LABEL))
                    .text_color(theme.text_secondary)
                    .when(metadata_monospace, |metadata| {
                        metadata.font_family(UI_MONO_FONT_FAMILY)
                    })
                    .child(metadata),
            )
            .child(div().flex_1().min_w_0())
            .when_some(trailing, |header, trailing| header.child(trailing))
            .into_any_element()
    }
}

pub(crate) fn client_titlebar(
    theme: Theme,
    selector: &'static str,
    close_id: &'static str,
    title: &'static str,
    on_close: impl Fn(&gpui_kit::ClickEvent, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static,
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
                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
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
                    div().debug_selector(move || close_id.into()).child(
                        icon_action(close_id, MaterialIcon::Close, "关闭")
                            .ghost()
                            .on_click(on_close),
                    ),
                ),
        )
        .into_any_element()
}
