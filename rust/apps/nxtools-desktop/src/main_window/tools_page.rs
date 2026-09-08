//! The "其他工具" page: a grid of tool cards. Clicking a card opens that
//! tool's settings section (inline, below the grid).

use super::*;
use crate::ui::metrics::{RADIUS_CONTROL, SPACING_6, SPACING_8, SPACING_12};
use crate::ui::typography::{LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL, TEXT_META};

/// One card in the tools grid.
struct ToolCard {
    id: &'static str,
    title: &'static str,
    description: &'static str,
    icon: MaterialIcon,
    enabled: bool,
}

pub(crate) fn render_tools(window: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let theme = window.theme;
    let link_detector_enabled = window
        .clipboard
        .upgrade()
        .map(|application| application.read(cx).link_detector_enabled())
        .unwrap_or_default();

    let cards = [
        ToolCard {
            id: "tool-link-detector",
            title: "链接识别",
            description: "复制到链接或本地路径时在光标附近浮出识别面板",
            icon: MaterialIcon::Language,
            enabled: link_detector_enabled,
        },
    ];

    let mut grid = div()
        .debug_selector(|| "tools-grid".into())
        .w_full()
        .flex()
        .flex_wrap()
        .gap(px(SPACING_12));
    for card in cards {
        grid = grid.child(
            div()
                .id(card.id)
                .debug_selector(move || format!("{}-card", card.id))
                .w(px(240.0))
                .cursor_pointer()
                .rounded(px(RADIUS_CONTROL))
                .border_1()
                .border_color(theme.border_subtle)
                .bg(theme.surface)
                .p(px(SPACING_12))
                .flex()
                .flex_col()
                .gap(px(SPACING_8))
                .hover(move |style| style.bg(theme.surface_hover))
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    // Clicking a card jumps to that tool's settings section.
                    this.tools_open_tool = Some(card.id);
                    cx.notify();
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SPACING_8))
                        .child(
                            Icon::new(card.icon)
                                .size(px(18.0))
                                .text_color(if card.enabled {
                                    theme.accent
                                } else {
                                    theme.text_muted
                                }),
                        )
                        .child(
                            div()
                                .text_size(px(TEXT_BODY))
                                .line_height(px(LINE_HEIGHT_BODY))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .child(card.title),
                        ),
                )
                .child(
                    div()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child(card.description),
                )
                .child(
                    div()
                        .text_size(px(TEXT_META))
                        .text_color(if card.enabled {
                            theme.success
                        } else {
                            theme.text_muted
                        })
                        .child(if card.enabled { "已启用" } else { "已停用" }),
                ),
        );
    }

    div()
        .id("tools-page")
        .debug_selector(|| "tools-page".into())
        .size_full()
        .min_h_0()
        .overflow_y_scroll()
        .track_scroll(&window.tools_scroll)
        .px(px(20.0))
        .py(px(16.0))
        .flex()
        .flex_col()
        .gap(px(SPACING_6))
        .child(
            div()
                .text_size(px(crate::ui::typography::TEXT_TITLE))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child("其他工具"),
        )
        .child(grid)
        .when(window.tools_open_tool.is_some(), |page| {
            page.child(tools_settings_section(window, cx))
        })
        .into_any_element()
}

/// Inline settings for the tool selected from the grid.
fn tools_settings_section(window: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let theme = window.theme;
    let enabled = window
        .clipboard
        .upgrade()
        .map(|application| application.read(cx).link_detector_enabled())
        .unwrap_or_default();
    div()
        .debug_selector(|| "tools-settings".into())
        .w_full()
        .rounded(px(RADIUS_CONTROL))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .p(px(SPACING_12))
        .flex()
        .flex_col()
        .gap(px(SPACING_6))
        .child(
            div()
                .text_size(px(TEXT_BODY))
                .font_weight(gpui::FontWeight::MEDIUM)
                .child("链接识别"),
        )
        .child(
            div()
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .text_color(theme.text_muted)
                .child("复制的内容包含网址（http/https）或存在的本地路径时，在鼠标附近浮出识别面板；点击条目复制，面板稍后自动消失。"),
        )
        .child(
            div()
                .id("tool-link-detector-toggle")
                .flex()
                .items_center()
                .gap(px(SPACING_8))
                .child(
                    Button::new("tool-link-detector-toggle-button")
                        .label(if enabled { "停用" } else { "启用" })
                        .small()
                        .outline()
                        .on_click({
                            let clipboard = window.clipboard.clone();
                            move |_, _, cx| {
                                let clipboard = clipboard.clone();
                                cx.defer(move |cx| {
                                    let _ = clipboard.update(cx, |clipboard, cx| {
                                        clipboard.set_link_detector_enabled(!enabled, cx);
                                    });
                                });
                            }
                        }),
                )
                .child(
                    div()
                        .text_size(px(TEXT_LABEL))
                        .text_color(if enabled { theme.success } else { theme.text_muted })
                        .child(if enabled { "运行中" } else { "已停用" }),
                ),
        )
        .into_any_element()
}
