use crate::theme::Theme;
use crate::ui::metrics::{
    CONTROL_HEIGHT_DEFAULT, RADIUS_CONTROL, SETTINGS_FOOTER_HEIGHT, SETTINGS_ROW_MIN_HEIGHT,
    SPACING_2, SPACING_8, SPACING_12,
};
use crate::ui::typography::{
    LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL, UI_FONT_FAMILY,
};
use gpui::{AnyElement, FontWeight, IntoElement, div, prelude::*, px};

const TOGGLE_KNOB_SIZE: f32 = 16.0;

pub(crate) struct SettingsShellIds {
    pub root: &'static str,
    pub content: &'static str,
    pub footer: &'static str,
}

pub(crate) struct SettingsSplitSurfaceIds {
    pub surface: &'static str,
    pub primary: &'static str,
    pub divider: &'static str,
    pub secondary: &'static str,
}

pub(crate) fn settings_split_surface(
    theme: Theme,
    ids: SettingsSplitSurfaceIds,
    secondary_width: f32,
    primary: AnyElement,
    secondary: AnyElement,
) -> AnyElement {
    div()
        .id(ids.surface)
        .debug_selector(move || ids.surface.into())
        .w_full()
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .flex()
        .child(
            div()
                .id(ids.primary)
                .debug_selector(move || ids.primary.into())
                .flex_1()
                .min_w_0()
                .px(px(SPACING_12))
                .child(primary),
        )
        .child(
            div()
                .id(ids.divider)
                .debug_selector(move || ids.divider.into())
                .w(px(1.0))
                .flex_none()
                .bg(theme.border_subtle),
        )
        .child(
            div()
                .id(ids.secondary)
                .debug_selector(move || ids.secondary.into())
                .w(px(secondary_width))
                .flex_none()
                .px(px(SPACING_12))
                .child(secondary),
        )
        .into_any_element()
}

pub(crate) fn settings_shell(
    theme: Theme,
    ids: SettingsShellIds,
    titlebar: AnyElement,
    content: AnyElement,
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
        .child(titlebar)
        .child(
            div()
                .debug_selector(move || ids.content.into())
                .w_full()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .child(content),
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

pub(crate) fn settings_section(
    theme: Theme,
    selector: &'static str,
    title: &'static str,
    rows: Vec<AnyElement>,
) -> AnyElement {
    div()
        .debug_selector(move || selector.into())
        .w_full()
        .rounded(px(RADIUS_CONTROL))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(26.0))
                .px(px(SPACING_8))
                .border_b_1()
                .border_color(theme.border_subtle)
                .flex()
                .items_center()
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_secondary)
                .child(title),
        )
        .children(rows)
        .into_any_element()
}

pub(crate) fn settings_row(
    theme: Theme,
    title: &'static str,
    description: impl Into<gpui::SharedString>,
    control: AnyElement,
) -> AnyElement {
    div()
        .min_h(px(SETTINGS_ROW_MIN_HEIGHT))
        .px(px(SPACING_8))
        .border_b_1()
        .border_color(theme.border_subtle)
        .flex()
        .items_center()
        .gap(px(SPACING_12))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(SPACING_2))
                .child(
                    div()
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child(description.into()),
                ),
        )
        .child(control)
        .into_any_element()
}

pub(crate) fn settings_toggle(
    theme: Theme,
    id: &'static str,
    enabled: bool,
    on_toggle: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    div()
        .id(id)
        .debug_selector(move || id.into())
        .w(px(40.0))
        .h(px(CONTROL_HEIGHT_DEFAULT))
        .p(px(2.0))
        .rounded(px(CONTROL_HEIGHT_DEFAULT / 2.0))
        .border_1()
        .border_color(if enabled {
            theme.border_active
        } else {
            theme.border_subtle
        })
        .bg(if enabled {
            theme.accent_soft
        } else {
            theme.canvas
        })
        .cursor_pointer()
        .flex()
        .when(enabled, |toggle| toggle.justify_end())
        .on_click(on_toggle)
        .child(
            div()
                .size(px(TOGGLE_KNOB_SIZE))
                .rounded(px(TOGGLE_KNOB_SIZE / 2.0))
                .bg(if enabled {
                    theme.accent
                } else {
                    theme.text_muted
                }),
        )
        .into_any_element()
}

pub(crate) fn dialog_footer(content: AnyElement) -> AnyElement {
    div()
        .size_full()
        .px(px(SPACING_12))
        .flex()
        .items_center()
        .justify_end()
        .gap(px(SPACING_8))
        .child(content)
        .into_any_element()
}
