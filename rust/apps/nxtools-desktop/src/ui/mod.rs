mod history;

use gpui::{
    App, AppContext, Bounds, Context, Entity, Focusable, Render, ResizeEdge, ScrollStrategy,
    Subscription, UniformListScrollHandle, WeakEntity, Window, div, point, prelude::*, px, size,
    uniform_list,
};
use gpui_component::input::{Input, InputEvent, InputState};
use nxtools_clipboard::ClipboardItem;
use nxtools_protocol::{ClipboardPanelRequest, PlatformCommandResult};
use tracing::{error, info};

use crate::clipboard::ClipboardApplication;
use crate::theme::Theme;
use history::history_row;

pub(crate) const PANEL_WIDTH: f32 = 340.0;
pub(crate) const PANEL_HEIGHT: f32 = 440.0;

#[derive(Debug, Clone, PartialEq, Eq)]
enum PanelStatus {
    Ready(String),
    Pending(&'static str),
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipboardCategory {
    All,
    Text,
}

impl PanelStatus {
    fn label(&self) -> &str {
        match self {
            Self::Ready(label) | Self::Error(label) => label,
            Self::Pending(label) => label,
        }
    }

    fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
}

#[cfg(windows)]
pub(crate) fn native_window_handle(window: &Window) -> Option<i64> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = HasWindowHandle::window_handle(window).ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as i64),
        _ => None,
    }
}

#[cfg(not(windows))]
pub(crate) fn native_window_handle(_window: &Window) -> Option<i64> {
    None
}

pub(crate) struct ClipboardPanel {
    theme: Theme,
    application: WeakEntity<ClipboardApplication>,
    target: nxtools_protocol::ForegroundContext,
    entries: Vec<ClipboardItem>,
    status: PanelStatus,
    pending_paste: Option<uuid::Uuid>,
    is_open: bool,
    active_category: ClipboardCategory,
    list_scroll: UniformListScrollHandle,
    search_input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl ClipboardPanel {
    pub(crate) fn new(
        entries: Vec<ClipboardItem>,
        target: nxtools_protocol::ForegroundContext,
        application: WeakEntity<ClipboardApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("搜索剪贴板历史"));
        search_input.focus_handle(cx).focus(window);

        let search_subscription = cx.subscribe_in(
            &search_input,
            window,
            |this, state, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    let query = state.read(cx).value().to_string();
                    this.reload_entries(&query, cx);
                    cx.notify();
                }
            },
        );
        let activation_subscription = cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() && this.pending_paste.is_none() {
                this.close(window, cx);
            }
        });
        let keystroke_subscription = cx.observe_keystrokes(|this, event, window, cx| {
            if event.keystroke.key.eq_ignore_ascii_case("escape") {
                this.close(window, cx);
            }
        });

        let status = if target.process_name.is_empty() {
            PanelStatus::Ready("选择一条记录进行粘贴".into())
        } else {
            PanelStatus::Ready(format!("将粘贴到 {}", target.process_name))
        };
        Self {
            theme: Theme::zed_dark(),
            application,
            target,
            entries,
            status,
            pending_paste: None,
            is_open: true,
            active_category: ClipboardCategory::All,
            list_scroll: UniformListScrollHandle::new(),
            search_input,
            _subscriptions: vec![
                search_subscription,
                activation_subscription,
                keystroke_subscription,
            ],
        }
    }

    fn reload_entries(&mut self, query: &str, cx: &mut Context<Self>) {
        match self
            .application
            .update(cx, |application, _| application.query_entries(query))
        {
            Ok(Ok(entries)) => self.entries = entries,
            Ok(Err(error)) => {
                error!(%error, "failed to query clipboard popup history");
                self.entries.clear();
                self.status = PanelStatus::Error("查询失败，请查看日志".into());
            }
            Err(error) => {
                error!(%error, "clipboard application became unavailable");
                self.entries.clear();
                self.status = PanelStatus::Error("剪贴板核心不可用".into());
            }
        }
    }

    fn paste_entry(&mut self, entry: &ClipboardItem, _window: &mut Window, cx: &mut Context<Self>) {
        if self.pending_paste.is_some() {
            self.status = PanelStatus::Pending("正在等待粘贴结果…");
            cx.notify();
            return;
        }
        if self.target.window_handle == 0 {
            self.status = PanelStatus::Error("原目标窗口已经不可用".into());
            cx.notify();
            return;
        }
        let target = self.target.clone();
        let entry = entry.clone();
        match self.application.update(cx, |application, _| {
            application.request_paste(&entry, &target)
        }) {
            Ok(Ok(command_id)) => {
                self.pending_paste = Some(command_id);
                self.status = PanelStatus::Pending("正在粘贴…");
                cx.notify();
            }
            Ok(Err(error)) => {
                self.status = PanelStatus::Error(format!("无法粘贴：{error}"));
                cx.notify();
            }
            Err(error) => {
                error!(%error, "clipboard application became unavailable");
                self.status = PanelStatus::Error("无法粘贴：剪贴板核心不可用".into());
                cx.notify();
            }
        }
    }

    pub(crate) fn handle_command_result(
        &mut self,
        correlation_id: uuid::Uuid,
        result: PlatformCommandResult,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_paste != Some(correlation_id) {
            return;
        }
        self.pending_paste = None;
        if result.success {
            info!(%correlation_id, "paste command completed");
            self.close(window, cx);
        } else {
            let reason = result.error.unwrap_or_else(|| "Windows 未返回原因".into());
            error!(%correlation_id, %reason, "paste command failed");
            self.status = PanelStatus::Error(format!("粘贴失败：{}", reason));
            window.activate_window();
            cx.notify();
        }
    }

    pub(crate) fn handle_hide_failure(
        &mut self,
        reason: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_open = true;
        self.status = PanelStatus::Error(format!("隐藏面板失败：{reason}"));
        window.activate_window();
        cx.notify();
    }

    pub(crate) fn handle_topmost_failure(
        &mut self,
        reason: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.status = PanelStatus::Error(format!("面板置顶失败：{reason}"));
        window.activate_window();
        cx.notify();
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_open {
            return;
        }
        let Some(window_handle) = native_window_handle(window) else {
            error!("cannot hide clipboard popup because its native window handle is unavailable");
            self.status = PanelStatus::Error("无法隐藏面板：窗口句柄不可用".into());
            cx.notify();
            return;
        };
        self.is_open = false;
        let application = self.application.clone();
        let panel = cx.entity().downgrade();
        cx.defer(move |cx| {
            let result =
                application.update(cx, |application, _| application.hide_popup(window_handle));
            let reason = match result {
                Ok(Ok(_)) => return,
                Ok(Err(error)) => {
                    error!(%error, "failed to request clipboard popup hide");
                    format!("无法隐藏面板：{error}")
                }
                Err(error) => {
                    error!(%error, "clipboard application became unavailable while hiding popup");
                    "无法隐藏面板：剪贴板核心不可用".into()
                }
            };
            let _ = panel.update(cx, |panel, cx| {
                panel.is_open = true;
                panel.status = PanelStatus::Error(reason);
                cx.notify();
            });
        });
    }
}

impl Render for ClipboardPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let entries_empty = self.entries.is_empty();
        let status_is_error = self.status.is_error();
        let status_label = self.status.label().to_owned();
        let summary = if self.target.process_name.is_empty() {
            format!("{} 条记录", self.entries.len())
        } else {
            format!("{} 条 · {}", self.entries.len(), self.target.process_name)
        };
        let is_scrolled = self.list_scroll.0.borrow().base_handle.offset().y < px(-1.0);
        let scroll_to_top = self.list_scroll.clone();
        let panel = cx.entity().downgrade();
        let all_category_panel = cx.entity().downgrade();
        let text_category_panel = cx.entity().downgrade();
        div()
            .relative()
            .size_full()
            .p_2()
            .rounded(px(8.0))
            .border_1()
            .border_color(theme.border)
            .bg(theme.canvas)
            .shadow_lg()
            .text_color(theme.text)
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .id("clipboard-panel-header")
                    .h(px(26.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .cursor_default()
                    .on_mouse_down(gpui::MouseButton::Left, |_event, window, _cx| {
                        window.start_window_move();
                    })
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("剪贴板"),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .truncate()
                            .child(summary),
                    )
                    .child(
                        div()
                            .max_w(px(150.0))
                            .h(px(21.0))
                            .px_2()
                            .rounded(px(5.0))
                            .border_1()
                            .border_color(if status_is_error {
                                theme.danger
                            } else {
                                theme.border
                            })
                            .bg(theme.surface_raised)
                            .text_xs()
                            .text_color(if status_is_error {
                                theme.danger
                            } else {
                                theme.text_secondary
                            })
                            .flex()
                            .items_center()
                            .truncate()
                            .child(status_label),
                    ),
            )
            .child(
                div()
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .gap_1()
                    .overflow_hidden()
                    .child(category_tab(
                        theme,
                        "全部",
                        self.active_category == ClipboardCategory::All,
                        move |_event, _window, app| {
                            let _ = all_category_panel.update(app, |panel, cx| {
                                panel.active_category = ClipboardCategory::All;
                                cx.notify();
                            });
                        },
                    ))
                    .child(category_tab(
                        theme,
                        "文本",
                        self.active_category == ClipboardCategory::Text,
                        move |_event, _window, app| {
                            let _ = text_category_panel.update(app, |panel, cx| {
                                panel.active_category = ClipboardCategory::Text;
                                cx.notify();
                            });
                        },
                    ))
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child("当前阶段仅迁移文本"),
                    ),
            )
            .child(
                div()
                    .h(px(30.0))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .h(px(30.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.surface)
                            .px_2()
                            .flex()
                            .items_center()
                            .child(
                                div().min_w_0().flex_1().text_color(theme.text).child(
                                    Input::new(&self.search_input)
                                        .appearance(false)
                                        .bordered(false)
                                        .focus_bordered(false),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .flex()
                            .items_center()
                            .child(format!("{} 条", self.entries.len())),
                    ),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .bg(theme.canvas)
                    .overflow_hidden()
                    .when(entries_empty, |list| {
                        list.child(
                            div()
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_sm()
                                .text_color(theme.text_muted)
                                .child("暂无纯文本历史"),
                        )
                    })
                    .when(!entries_empty, |list| {
                        list.child(
                            uniform_list(
                                "clipboard-popup-history",
                                self.entries.len(),
                                cx.processor(|this, range: std::ops::Range<usize>, _window, cx| {
                                    range
                                        .filter_map(|index| {
                                            let entry = this.entries.get(index)?.clone();
                                            let paste_entry = entry.clone();
                                            let panel = cx.entity().downgrade();
                                            let on_click = move |
                                                _event: &gpui::ClickEvent,
                                                window: &mut Window,
                                                app: &mut App,
                                            | {
                                                let _ = panel.update(app, |this, cx| {
                                                    this.paste_entry(&paste_entry, window, cx);
                                                });
                                            };
                                            Some(history_row(this.theme, entry, index, on_click))
                                        })
                                        .collect::<Vec<_>>()
                                }),
                            )
                            .w_full()
                            .h_full()
                            .track_scroll(self.list_scroll.clone()),
                        )
                    })
                    .when(is_scrolled, |list| {
                        list.child(
                            div()
                                .absolute()
                                .bottom_2()
                                .left_0()
                                .right_0()
                                .flex()
                                .justify_center()
                                .child(
                                    div()
                                        .id("back-to-top")
                                        .h(px(28.0))
                                        .px_3()
                                        .rounded(px(5.0))
                                        .border_1()
                                        .border_color(theme.border)
                                        .bg(theme.surface_raised)
                                        .shadow_md()
                                        .cursor_pointer()
                                        .hover(move |style| style.bg(theme.surface_hover))
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .flex()
                                        .items_center()
                                        .child("↑  回到开头")
                                        .on_click(move |_event, _window, app| {
                                            scroll_to_top
                                                .scroll_to_item_strict(0, ScrollStrategy::Top);
                                            let _ = panel.update(app, |_, cx| cx.notify());
                                        }),
                                ),
                        )
                    }),
            )
            .child(
                div()
                    .absolute()
                    .right_0()
                    .bottom_0()
                    .size(px(16.0))
                    .cursor_e_resize()
                    .text_xs()
                    .text_color(theme.text_muted)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child("⌟")
                    .on_mouse_down(gpui::MouseButton::Left, |_event, window, _cx| {
                        window.start_window_resize(ResizeEdge::BottomRight);
                    }),
            )
    }
}

fn category_tab(
    theme: Theme,
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(label)
        .h(px(26.0))
        .px_2()
        .rounded(px(5.0))
        .border_1()
        .border_color(if selected {
            theme.accent
        } else {
            gpui::rgba(0x00000000)
        })
        .bg(if selected {
            theme.accent_soft
        } else {
            gpui::rgba(0x00000000)
        })
        .text_xs()
        .text_color(if selected {
            theme.accent_text
        } else {
            theme.text_secondary
        })
        .flex()
        .items_center()
        .cursor_pointer()
        .hover(move |style| style.bg(theme.surface_hover))
        .on_click(on_click)
        .child(label)
}

pub(crate) fn panel_bounds(
    request: &ClipboardPanelRequest,
    cx: &App,
) -> (Bounds<gpui::Pixels>, Option<gpui::DisplayId>) {
    let panel_size = size(px(PANEL_WIDTH), px(PANEL_HEIGHT));
    let displays = cx.displays();
    let display = displays
        .iter()
        .find(|display| u32::from(display.id()) == request.cursor_display_index)
        .cloned()
        .or_else(|| displays.first().cloned());
    let scale = (request.cursor_monitor_dpi.max(96) as f32) / 96.0;
    let cursor = point(
        px(request.cursor_x as f32 / scale),
        px(request.cursor_y as f32 / scale),
    );
    let Some(display) = display else {
        return (
            Bounds {
                origin: point(cursor.x + px(12.0), cursor.y + px(12.0)),
                size: panel_size,
            },
            None,
        );
    };

    let screen = display.bounds();
    let max_x = screen.right() - panel_size.width;
    let max_y = screen.bottom() - panel_size.height;
    let origin = point(
        clamp_pixels(cursor.x + px(12.0), screen.left(), max_x),
        clamp_pixels(cursor.y + px(12.0), screen.top(), max_y),
    );
    (
        Bounds {
            origin,
            size: panel_size,
        },
        Some(display.id()),
    )
}

fn clamp_pixels(value: gpui::Pixels, minimum: gpui::Pixels, maximum: gpui::Pixels) -> gpui::Pixels {
    if maximum < minimum || value < minimum {
        minimum
    } else if value > maximum {
        maximum
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_status_uses_typed_error_state() {
        assert!(!PanelStatus::Ready("ready".into()).is_error());
        assert!(!PanelStatus::Pending("pending").is_error());
        assert!(PanelStatus::Error("failed".into()).is_error());
    }

    #[test]
    fn panel_metrics_match_the_reference_window() {
        assert_eq!(PANEL_WIDTH, 340.0);
        assert_eq!(PANEL_HEIGHT, 440.0);
    }
}
