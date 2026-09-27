//! The "其他工具" page: a grid of tool cards. Clicking a card opens that
//! tool's settings section (inline, below the grid).

use super::*;
use crate::ui::components::settings::{
    FeatureSettingsWindowIds, FeatureSettingsWindowSpec, feature_settings_window, settings_row,
    settings_section, settings_toggle,
};
use crate::ui::metrics::{RADIUS_CONTROL, SPACING_6, SPACING_8, SPACING_12};
use crate::ui::typography::{
    LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL, TEXT_META,
};
use gpui::{
    Corner, WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions,
    size,
};
use gpui_component::menu::{DropdownMenu, PopupMenuItem};
use nxtools_core::LinkPanelPosition;

/// One card in the tools grid.
#[derive(Clone, Copy)]
struct ToolCard {
    id: &'static str,
    title: &'static str,
    description: &'static str,
    icon: MaterialIcon,
    enabled: bool,
}
fn link_panel_position_label(position: LinkPanelPosition) -> &'static str {
    match position {
        LinkPanelPosition::TopLeft => "左上角",
        LinkPanelPosition::Center => "中间",
        LinkPanelPosition::TopRight => "右上角",
    }
}

fn link_panel_position_selector(position: LinkPanelPosition) -> &'static str {
    match position {
        LinkPanelPosition::TopLeft => "tool-settings-position-top-left",
        LinkPanelPosition::Center => "tool-settings-position-center",
        LinkPanelPosition::TopRight => "tool-settings-position-top-right",
    }
}
pub(crate) fn render_tools(window: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let theme = window.theme;
    let link_detector_enabled = window
        .clipboard
        .upgrade()
        .map(|application| application.read(cx).link_detector_enabled())
        .unwrap_or_default();

    let cards = [ToolCard {
        id: "tool-link-detector",
        title: "链接识别",
        description: "复制到链接或本地路径时在光标附近浮出识别面板",
        icon: MaterialIcon::Language,
        enabled: link_detector_enabled,
    }];

    let mut grid = div()
        .debug_selector(|| "tools-grid".into())
        .w_full()
        .flex()
        .flex_wrap()
        .gap(px(SPACING_12));
    for card in cards {
        grid =
            grid.child(
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
                        open_tool_settings_window(this, card, cx);
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(SPACING_8))
                            .child(Icon::new(card.icon).size(px(18.0)).text_color(
                                if card.enabled {
                                    theme.accent
                                } else {
                                    theme.text_muted
                                },
                            ))
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
                            .child(if card.enabled {
                                "已启用"
                            } else {
                                "已停用"
                            }),
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
    position: LinkPanelPosition,
    clipboard: gpui::WeakEntity<ClipboardApplication>,
}

impl ToolSettingsWindowBody {
    fn defer_clipboard_update(
        clipboard: gpui::WeakEntity<ClipboardApplication>,
        cx: &mut Context<Self>,
        update: impl FnOnce(&mut ClipboardApplication, &mut Context<ClipboardApplication>) + 'static,
    ) {
        cx.defer(move |cx| {
            if let Some(clipboard) = clipboard.upgrade() {
                clipboard.update(cx, update);
            }
        });
    }
}

impl Render for ToolSettingsWindowBody {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::zed_dark();
        let enabled_toggle = cx.listener(|this, _event, _window, cx| {
            this.enabled = !this.enabled;
            let enabled = this.enabled;
            Self::defer_clipboard_update(this.clipboard.clone(), cx, move |clipboard, cx| {
                clipboard.set_link_detector_enabled(enabled, cx);
            });
            cx.notify();
        });

        let seconds_minus = cx.listener(|this, _event, _window, cx| {
            this.auto_hide_seconds = this.auto_hide_seconds.saturating_sub(2).max(2);
            let auto_hide_ms = this.auto_hide_seconds * 1_000;
            Self::defer_clipboard_update(this.clipboard.clone(), cx, move |clipboard, cx| {
                clipboard.set_link_panel_auto_hide_ms(auto_hide_ms, cx);
            });
            cx.notify();
        });
        let seconds_plus = cx.listener(|this, _event, _window, cx| {
            this.auto_hide_seconds = (this.auto_hide_seconds + 2).min(60);
            let auto_hide_ms = this.auto_hide_seconds * 1_000;
            Self::defer_clipboard_update(this.clipboard.clone(), cx, move |clipboard, cx| {
                clipboard.set_link_panel_auto_hide_ms(auto_hide_ms, cx);
            });
            cx.notify();
        });
        let auto_hide_control = div()
            .flex()
            .items_center()
            .gap(px(SPACING_8))
            .child(
                Button::new("tool-settings-dwell-minus")
                    .debug_selector(|| "tool-settings-dwell-minus".into())
                    .label("-")
                    .small()
                    .outline()
                    .size(px(28.0))
                    .on_click(seconds_minus),
            )
            .child(
                div()
                    .debug_selector(|| "tool-settings-dwell-value".into())
                    .w(px(48.0))
                    .text_size(px(TEXT_BODY))
                    .line_height(px(LINE_HEIGHT_BODY))
                    .text_color(theme.text)
                    .text_center()
                    .child(format!("{} 秒", self.auto_hide_seconds)),
            )
            .child(
                Button::new("tool-settings-dwell-plus")
                    .debug_selector(|| "tool-settings-dwell-plus".into())
                    .label("+")
                    .small()
                    .outline()
                    .size(px(28.0))
                    .on_click(seconds_plus),
            )
            .into_any_element();

        let open_browser_toggle = cx.listener(|this, _event, _window, cx| {
            this.auto_open_browser = !this.auto_open_browser;
            if this.auto_open_browser {
                this.copy_only = false;
            }
            let auto_open_browser = this.auto_open_browser;
            let copy_only = this.copy_only;
            Self::defer_clipboard_update(this.clipboard.clone(), cx, move |clipboard, cx| {
                clipboard.set_link_panel_auto_open_browser(auto_open_browser, cx);
                if auto_open_browser {
                    clipboard.set_link_panel_copy_only(copy_only, cx);
                }
            });
            cx.notify();
        });
        let copy_only_toggle = cx.listener(|this, _event, _window, cx| {
            this.copy_only = !this.copy_only;
            if this.copy_only {
                this.auto_open_browser = false;
            }
            let auto_open_browser = this.auto_open_browser;
            let copy_only = this.copy_only;
            Self::defer_clipboard_update(this.clipboard.clone(), cx, move |clipboard, cx| {
                clipboard.set_link_panel_copy_only(copy_only, cx);
                if copy_only {
                    clipboard.set_link_panel_auto_open_browser(auto_open_browser, cx);
                }
            });
            cx.notify();
        });
        let position_view = cx.entity().downgrade();
        let selected_position = self.position;
        let position_control = Button::new("tool-settings-position")
            .debug_selector(|| "tool-settings-position".into())
            .label(link_panel_position_label(self.position))
            .small()
            .outline()
            .w(px(124.0))
            .dropdown_menu_with_anchor(Corner::TopRight, move |menu, _window, _cx| {
                LinkPanelPosition::ALL
                    .into_iter()
                    .fold(menu.min_w(px(124.0)), |menu, position| {
                        let position_view = position_view.clone();
                        let selector = link_panel_position_selector(position);
                        let label = link_panel_position_label(position);
                        menu.item(
                            PopupMenuItem::element(move |_, _| {
                                div()
                                    .id(selector)
                                    .debug_selector(move || selector.into())
                                    .w_full()
                                    .child(label)
                            })
                            .checked(position == selected_position)
                            .on_click(move |_, window, cx| {
                                let position_view = position_view.clone();
                                window.defer(cx, move |_, cx| {
                                    let _ = position_view.update(cx, |view, cx| {
                                        view.position = position;
                                        let clipboard = view.clipboard.clone();
                                        Self::defer_clipboard_update(
                                            clipboard,
                                            cx,
                                            move |clipboard, cx| {
                                                clipboard.set_link_panel_position(position, cx);
                                            },
                                        );
                                        cx.notify();
                                    });
                                });
                            }),
                        )
                    })
            });

        let content = settings_section(
            theme,
            "tool-settings-panel-section",
            "面板行为",
            vec![
                settings_row(
                    theme,
                    "自动关闭时间",
                    "识别面板显示后等待关闭的时间。",
                    auto_hide_control,
                ),
                settings_row(
                    theme,
                    "悬浮窗位置",
                    "选择识别面板在屏幕上的显示位置。",
                    position_control.into_any_element(),
                ),
                settings_row(
                    theme,
                    "自动打开浏览器",
                    "点击识别结果后直接打开对应网页。",
                    settings_toggle(
                        theme,
                        "tool-settings-open-browser",
                        self.auto_open_browser,
                        open_browser_toggle,
                    ),
                ),
                settings_row(
                    theme,
                    "仅复制",
                    "点击识别结果时只复制，不打开目标。",
                    settings_toggle(
                        theme,
                        "tool-settings-copy-only",
                        self.copy_only,
                        copy_only_toggle,
                    ),
                ),
            ],
        );
        let on_close = cx.listener(|_, _event, window, _cx| window.remove_window());
        feature_settings_window(
            theme,
            FeatureSettingsWindowSpec {
                ids: FeatureSettingsWindowIds {
                    root: "tool-settings-window",
                    title: "tool-settings-title",
                    description: "tool-settings-description",
                    status: "tool-settings-enabled-status",
                    enabled_toggle: "tool-settings-toggle",
                    close: "tool-settings-close",
                    content: "tool-settings-content",
                },
                title: self.title.into(),
                description: self.description.into(),
                enabled: self.enabled,
            },
            enabled_toggle,
            on_close,
            content,
        )
    }
}

fn open_tool_settings_window(this: &mut MainWindow, card: ToolCard, cx: &mut Context<MainWindow>) {
    if let Some(existing) = this.tool_settings_windows.get(card.id).copied() {
        if existing
            .update(cx, |_, window, _| {
                window.activate_window();
                window.refresh();
            })
            .is_ok()
        {
            return;
        }
        this.tool_settings_windows.remove(card.id);
    }

    let (enabled, auto_hide_seconds, auto_open_browser, copy_only, position) = this
        .clipboard
        .upgrade()
        .map(|application| application.read(cx).link_panel_settings())
        .unwrap_or((false, 6, false, true, LinkPanelPosition::Center));
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
                title: card.title,
                description: card.description,
                enabled,
                auto_hide_seconds,
                auto_open_browser,
                copy_only,
                position,
                clipboard: clipboard.clone(),
            });
            cx.new(|cx| gpui_component::Root::new(body, window, cx))
        },
    );
    if let Ok(window) = opened {
        this.tool_settings_windows.insert(card.id, window.into());
    } else {
        tracing::error!(tool = card.id, "failed to open tool settings window");
    }
}
