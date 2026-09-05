use gpui::{SharedString, div, prelude::*, px};
use gpui_component::Icon;
use gpui_component::menu::{PopupMenu, PopupMenuItem};

use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::metrics::{CONTROL_HEIGHT_ACTION, SPACING_6};
use crate::ui::typography::{LINE_HEIGHT_BODY, TEXT_BODY, UI_FONT_FAMILY};

const MENU_MIN_WIDTH: f32 = 192.0;
const MENU_MAX_WIDTH: f32 = 240.0;
const MENU_CONTENT_MIN_WIDTH: f32 = 160.0;
const MENU_ICON_SIZE: f32 = 14.0;
const MENU_ICON_SLOT_WIDTH: f32 = 14.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum MenuTone {
    #[default]
    Normal,
    Danger,
}

pub(crate) fn context_menu_shell(menu: PopupMenu) -> PopupMenu {
    menu.min_w(px(MENU_MIN_WIDTH)).max_w(px(MENU_MAX_WIDTH))
}

pub(crate) fn context_menu_item(
    theme: Theme,
    selector: &'static str,
    label: impl Into<SharedString>,
    icon: MaterialIcon,
    tone: MenuTone,
) -> PopupMenuItem {
    let label = label.into();
    let foreground = match tone {
        MenuTone::Normal => theme.text,
        MenuTone::Danger => theme.danger,
    };
    let icon_color = match tone {
        MenuTone::Normal => theme.icon_muted,
        MenuTone::Danger => theme.danger,
    };

    PopupMenuItem::element(move |_window, _cx| {
        div()
            .debug_selector(move || selector.into())
            .w_full()
            .min_w(px(MENU_CONTENT_MIN_WIDTH))
            .h(px(CONTROL_HEIGHT_ACTION))
            .flex()
            .items_center()
            .gap(px(SPACING_6))
            .font_family(UI_FONT_FAMILY)
            .text_size(px(TEXT_BODY))
            .line_height(px(LINE_HEIGHT_BODY))
            .text_color(foreground)
            .child(
                div()
                    .w(px(MENU_ICON_SLOT_WIDTH))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        Icon::new(icon)
                            .size(px(MENU_ICON_SIZE))
                            .text_color(icon_color),
                    ),
            )
            .child(div().min_w_0().flex_1().child(label.clone()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_menu_metrics_follow_the_shared_action_tier() {
        assert_eq!(CONTROL_HEIGHT_ACTION, 28.0);
        assert_eq!([MENU_MIN_WIDTH, MENU_MAX_WIDTH], [192.0, 240.0]);
    }
}
