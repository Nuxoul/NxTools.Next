use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::component::{Icon, label::Label};
use gpui_kit::{SharedString, div, prelude::*, px};

use crate::icons::MaterialIcon;
use crate::theme::Theme;

const MENU_MIN_WIDTH: f32 = 192.0;
const MENU_MAX_WIDTH: f32 = 240.0;
const MENU_CONTENT_MIN_WIDTH: f32 = 160.0;

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
            .text_color(foreground)
            .child(Label::new(label.clone()).text_sm())
    })
    .icon(Icon::new(icon).text_color(icon_color))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_menu_preserves_only_bounded_shell_metrics() {
        assert_eq!([MENU_MIN_WIDTH, MENU_MAX_WIDTH], [192.0, 240.0]);
    }
}
