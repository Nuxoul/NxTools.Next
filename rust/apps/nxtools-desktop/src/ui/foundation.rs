use gpui::{AnyElement, App, FontWeight, IntoElement, MouseButton, Window, div, prelude::*, px};
use gpui_component::button::{Button, ButtonCustomVariant};
use gpui_component::{Icon, Sizable};

use crate::icons::MaterialIcon;
use crate::theme::Theme;

pub(crate) const UI_FONT_FAMILY: &str = "Microsoft YaHei UI";
pub(crate) const UI_MONO_FONT_FAMILY: &str = "Consolas";

pub(crate) const TITLEBAR_HEIGHT: f32 = 36.0;
#[allow(dead_code)] // Reserved for compact icon controls as later surfaces migrate.
pub(crate) const CONTROL_HEIGHT_COMPACT: f32 = 18.0;
pub(crate) const CONTROL_HEIGHT_DEFAULT: f32 = 22.0;
pub(crate) const CONTROL_HEIGHT_ACTION: f32 = 28.0;
#[allow(dead_code)] // Reserved for field components as later surfaces migrate.
pub(crate) const CONTROL_HEIGHT_INPUT: f32 = 32.0;
pub(crate) const SETTINGS_ROW_MIN_HEIGHT: f32 = 42.0;

#[allow(dead_code)] // Reserved for shared module/dialog titles.
pub(crate) const TEXT_TITLE: f32 = 14.0;
pub(crate) const TEXT_BODY: f32 = 12.0;
pub(crate) const TEXT_LABEL: f32 = 11.0;
#[allow(dead_code)] // Reserved for actual metadata, never ordinary Chinese descriptions.
pub(crate) const TEXT_META: f32 = 10.0;
#[allow(dead_code)] // Reserved for shared dialog/module titles.
pub(crate) const LINE_HEIGHT_TITLE: f32 = 20.0;
pub(crate) const LINE_HEIGHT_BODY: f32 = 18.0;
pub(crate) const LINE_HEIGHT_LABEL: f32 = 16.0;
#[allow(dead_code)] // Reserved for actual metadata.
pub(crate) const LINE_HEIGHT_META: f32 = 14.0;

#[allow(dead_code)] // Shared motion token for subsequent surface migrations.
pub(crate) const MOTION_ENTER_MS: u64 = 120;
#[allow(dead_code)] // Shared motion token for subsequent surface migrations.
pub(crate) const MOTION_RESIZE_MS: u64 = 160;

pub(crate) const RADIUS_CONTROL: f32 = 4.0;
#[allow(dead_code)] // Reserved for popovers and menus.
pub(crate) const RADIUS_FLOATING: f32 = 6.0;
pub(crate) const SPACING_2: f32 = 2.0;
#[allow(dead_code)]
pub(crate) const SPACING_4: f32 = 4.0;
#[allow(dead_code)]
pub(crate) const SPACING_6: f32 = 6.0;
pub(crate) const SPACING_8: f32 = 8.0;
pub(crate) const SPACING_12: f32 = 12.0;
#[allow(dead_code)]
pub(crate) const SPACING_16: f32 = 16.0;
#[allow(dead_code)]
pub(crate) const SPACING_24: f32 = 24.0;

pub(crate) fn primary_button_variant(theme: Theme, cx: &App) -> ButtonCustomVariant {
    ButtonCustomVariant::new(cx)
        .color(theme.accent.into())
        .foreground(theme.accent_text.into())
        .border(theme.border_active.into())
        .hover(theme.border_active.into())
        .active(theme.accent.into())
}

pub(crate) fn action_button(id: &'static str, label: &'static str) -> Button {
    Button::new(id)
        .label(label)
        .small()
        .h(px(CONTROL_HEIGHT_ACTION))
        .px(px(SPACING_8))
        .rounded(px(RADIUS_CONTROL))
}

pub(crate) fn icon_action_button(
    id: &'static str,
    icon: MaterialIcon,
    label: &'static str,
) -> Button {
    action_button(id, label).icon(Icon::new(icon).size(px(14.0)))
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
    on_toggle: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
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
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(on_toggle)
        .child(div().size(px(16.0)).rounded(px(8.0)).bg(if enabled {
            theme.accent
        } else {
            theme.text_muted
        }))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_scale_matches_the_zed_density_contract() {
        assert_eq!(TITLEBAR_HEIGHT, 36.0);
        assert_eq!(CONTROL_HEIGHT_COMPACT, 18.0);
        assert_eq!(CONTROL_HEIGHT_DEFAULT, 22.0);
        assert_eq!(CONTROL_HEIGHT_ACTION, 28.0);
        assert_eq!(CONTROL_HEIGHT_INPUT, 32.0);
        assert_eq!(RADIUS_CONTROL, 4.0);
        assert_eq!(RADIUS_FLOATING, 6.0);
        assert_eq!(
            [
                SPACING_2, SPACING_4, SPACING_6, SPACING_8, SPACING_12, SPACING_16, SPACING_24,
            ],
            [2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0]
        );
        assert_eq!(
            [TEXT_TITLE, TEXT_BODY, TEXT_LABEL, TEXT_META],
            [14.0, 12.0, 11.0, 10.0]
        );
        assert_eq!(
            [
                LINE_HEIGHT_TITLE,
                LINE_HEIGHT_BODY,
                LINE_HEIGHT_LABEL,
                LINE_HEIGHT_META,
            ],
            [20.0, 18.0, 16.0, 14.0]
        );
        assert_eq!([MOTION_ENTER_MS, MOTION_RESIZE_MS], [120, 160]);
    }
}
