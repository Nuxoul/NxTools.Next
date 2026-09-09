//! The "其他工具" page: a grid of tool cards. Clicking a card opens that
//! tool's settings section (inline, below the grid).

use super::*;
use gpui::rgb;
use gpui::{size, WindowOptions, WindowBackgroundAppearance, WindowDecorations, WindowKind, WindowBounds};
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

/// Settings for the tool opened from the grid card.
pub(crate) struct ToolSettingsWindowBody {
    title: &'static str,
    description: &'static str,
    enabled: bool,
    auto_hide_seconds: u64,
    auto_open_browser: bool,
    copy_only: bool,
    clipboard: gpui::WeakEntity<ClipboardApplication>,
}

impl Render for ToolSettingsWindowBody {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::zed_dark();
        let text_secondary = rgb(0xa1a1aa);
        let success = rgb(0x10b981);
        let text_muted = rgb(0x71717a);
        let text_primary = rgb(0xf4f4f5);
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
                                if let Some(clipboard) = this.clipboard.upgrade() {
                                    clipboard.update(cx, |clipboard, cx| {
                                        clipboard.set_link_detector_enabled(this.enabled, cx);
                                    });
                                }
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
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(text_secondary)
                            .child("面板消失时间(秒)"),
                    )
                    .child(
                        Button::new("tool-settings-dwell-minus")
                            .label("-")
                            .small()
                            .outline()
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                this.auto_hide_seconds =
                                    this.auto_hide_seconds.saturating_sub(2).max(2);
                                if let Some(clipboard) = this.clipboard.upgrade() {
                                    clipboard.update(cx, |clipboard, cx| {
                                        clipboard.set_link_panel_auto_hide_ms(
                                            this.auto_hide_seconds * 1_000,
                                            cx,
                                        );
                                    });
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(text_primary)
                            .child(format!("{}", self.auto_hide_seconds)),
                    )
                    .child(
                        Button::new("tool-settings-dwell-plus")
                            .label("+")
                            .small()
                            .outline()
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                this.auto_hide_seconds = (this.auto_hide_seconds + 2).min(60);
                                if let Some(clipboard) = this.clipboard.upgrade() {
                                    clipboard.update(cx, |clipboard, cx| {
                                        clipboard.set_link_panel_auto_hide_ms(
                                            this.auto_hide_seconds * 1_000,
                                            cx,
                                        );
                                    });
                                }
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        Button::new("tool-settings-open-browser")
                            .label(if self.auto_open_browser { "✓" } else { " " })
                            .small()
                            .outline()
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                this.auto_open_browser = !this.auto_open_browser;
                                if this.auto_open_browser {
                                    this.copy_only = false;
                                }
                                if let Some(clipboard) = this.clipboard.upgrade() {
                                    clipboard.update(cx, |clipboard, cx| {
                                        clipboard.set_link_panel_auto_open_browser(
                                            this.auto_open_browser,
                                            cx,
                                        );
                                    });
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(text_secondary)
                            .child("点击后自动在浏览器打开"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        Button::new("tool-settings-copy-only")
                            .label(if self.copy_only { "✓" } else { " " })
                            .small()
                            .outline()
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                this.copy_only = !this.copy_only;
                                if this.copy_only {
                                    this.auto_open_browser = false;
                                }
                                if let Some(clipboard) = this.clipboard.upgrade() {
                                    clipboard.update(cx, |clipboard, cx| {
                                        clipboard.set_link_panel_copy_only(this.copy_only, cx);
                                    });
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(text_secondary)
                            .child("仅复制到剪贴板"),
                    ),
            )
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
    let (enabled, auto_hide_seconds, auto_open_browser, copy_only) = this
        .clipboard
        .upgrade()
        .map(|application| application.read(cx).link_panel_settings())
        .unwrap_or((false, 6, false, true));
    let clipboard = this.clipboard.clone();
    let window_size = size(px(460.0), px(320.0));
    let window_bounds = WindowBounds::centered(window_size, cx);
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(window_bounds),
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
                auto_hide_seconds,
                auto_open_browser,
                copy_only,
                clipboard: clipboard.clone(),
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
