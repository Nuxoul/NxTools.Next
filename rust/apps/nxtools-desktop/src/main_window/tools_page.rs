//! The "其他工具" page: a grid of tool cards. Clicking a card opens that
//! tool's settings section (inline, below the grid).

use super::*;
use gpui::rgb;
use gpui::{point, size, Bounds, WindowOptions, WindowBackgroundAppearance, WindowDecorations, WindowKind, WindowBounds};
use crate::ui::metrics::{RADIUS_CONTROL, SPACING_6, SPACING_8, SPACING_12};
use crate::ui::typography::{LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL, TEXT_META};

/// One card in the tools grid.
#[derive(Clone, Copy)]
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
                    // Clicking a card opens the tool's settings in a small
                    // dedicated window (user's explicit requirement).
                    this.tools_open_tool = Some(card.id);
                    open_tool_settings_window(this, card, cx);
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
        .into_any_element()
}

/// Inline settings for the tool selected from the grid.
pub(crate) struct ToolSettingsWindowBody {
    title: &'static str,
    description: &'static str,
    enabled: bool,
    toggle_clicked: bool,
}

impl Render for ToolSettingsWindowBody {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::zed_dark();
        let text_secondary = rgb(0xa1a1aa);
        let success = rgb(0x10b981);
        let text_muted = rgb(0x71717a);
        div()
            .size_full()
            .bg(theme.canvas)
            .text_color(theme.text)
            .p(px(16.0))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(self.title),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .line_height(px(16.0))
                    .text_color(text_secondary)
                    .child(self.description),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        Button::new("tool-settings-toggle")
                            .label(if self.enabled { "停用" } else { "启用" })
                            .small()
                            .outline()
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                this.enabled = !this.enabled;
                                this.toggle_clicked = true;
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(if self.enabled { success } else { text_secondary })
                            .child(if self.enabled { "运行中" } else { "已停用" }),
                    ),
            )
            .when(self.toggle_clicked, |body| {
                body.child(
                    div()
                        .text_size(px(10.0))
                        .text_color(text_secondary)
                        .child("设置已更新；关闭此窗口即可。"),
                )
            })
            .child(div().flex_1())
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(text_muted)
                    .child("其他工具 / 链接识别"),
            )
    }
}

fn open_tool_settings_window(
    this: &mut MainWindow,
    card: ToolCard,
    cx: &mut Context<MainWindow>,
) {
    let enabled = this
        .clipboard
        .upgrade()
        .map(|application| application.read(cx).link_detector_enabled())
        .unwrap_or_default();
    let window_size = size(px(460.0), px(190.0));
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(point(px(0.0), px(0.0)), window_size))),
            titlebar: None,
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_movable: true,
            is_resizable: false,
            is_minimizable: true,
            display_id: None,
            window_background: WindowBackgroundAppearance::Opaque,
            app_id: Some(format!("NxTools.ToolSettings.{}", card.id)),
            window_min_size: Some(window_size),
            window_decorations: Some(WindowDecorations::Client),
            tabbing_identifier: None,
        },
        move |window, cx| {
            let body = cx.new(|_| ToolSettingsWindowBody {
                title: "链接识别",
                description: "复制的内容包含网址（http/https）或存在的本地路径时，在屏幕上方浮出识别面板；点击条目复制，面板稍后自动消失。",
                enabled,
                toggle_clicked: false,
            });
            cx.new(|cx| gpui_component::Root::new(body, window, cx))
        },
    );
    if let Ok(window) = opened {
        this.tool_settings_window = Some(window.into());
    } else {
        tracing::error!(tool = card.id, "failed to open tool settings window");
    }
}
