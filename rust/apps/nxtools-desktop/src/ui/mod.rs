mod history;

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    AnyWindowHandle, App, AppContext, Bounds, Context, Entity, Focusable, Global, Render,
    Subscription, Task, WeakEntity, Window, WindowBackgroundAppearance, WindowBounds,
    WindowDecorations, WindowKind, WindowOptions, div, point, prelude::*, px, size, uniform_list,
};
use gpui_component::Root;
use gpui_component::input::{Input, InputEvent, InputState};
use nxtools_clipboard::{CaptureSource, ClipboardHistory, ClipboardItem, TextCapture};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use nxtools_protocol::{
    ClipboardPanelRequest, PasteTextCommand, PlatformCommand, PlatformCommandResult, PlatformEvent,
};
use tracing::{error, info};

use crate::theme::Theme;
use history::history_row;

const HISTORY_LIMIT: usize = 500;
const PANEL_WIDTH: f32 = 340.0;
const PANEL_HEIGHT: f32 = 440.0;
const MAX_PANEL_REQUEST_AGE_MS: i64 = 2_000;

pub struct ClipboardControllerGlobal {
    _controller: Entity<ClipboardController>,
}

impl ClipboardControllerGlobal {
    pub fn new(controller: Entity<ClipboardController>) -> Self {
        Self {
            _controller: controller,
        }
    }
}

impl Global for ClipboardControllerGlobal {}

pub struct ClipboardController {
    history: Rc<RefCell<ClipboardHistory>>,
    platform: Rc<PlatformBridge>,
    popup_window: Option<AnyWindowHandle>,
    popup_panel: Option<WeakEntity<ClipboardPanel>>,
    _platform_task: Task<()>,
}

impl ClipboardController {
    pub fn new(
        history: ClipboardHistory,
        platform: PlatformBridge,
        cx: &mut Context<Self>,
    ) -> Self {
        let platform = Rc::new(platform);
        let platform_notifications = platform.notification_receiver();
        let platform_task = cx.spawn(async move |this, cx| {
            while let Ok(notification) = platform_notifications.recv().await {
                let Some(this) = this.upgrade() else {
                    return;
                };
                if this
                    .update(cx, |this, cx| {
                        this.handle_platform_notification(notification, cx)
                    })
                    .is_err()
                {
                    return;
                }
            }
        });

        Self {
            history: Rc::new(RefCell::new(history)),
            platform,
            popup_window: None,
            popup_panel: None,
            _platform_task: platform_task,
        }
    }

    fn handle_platform_notification(
        &mut self,
        notification: PlatformNotification,
        cx: &mut Context<Self>,
    ) {
        match notification {
            PlatformNotification::Event(event) => self.handle_platform_event(event, cx),
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } => {
                let (Some(window), Some(panel)) =
                    (self.popup_window, self.popup_panel.as_ref().cloned())
                else {
                    return;
                };
                let _ = window.update(cx, |_, window, app| {
                    let _ = panel.update(app, |panel, cx| {
                        panel.handle_command_result(correlation_id, result, window, cx);
                    });
                });
            }
        }
    }

    fn handle_platform_event(&mut self, event: PlatformEvent, cx: &mut Context<Self>) {
        match event {
            PlatformEvent::ClipboardTextCaptured(capture) => {
                if let Err(error) = self.history.borrow_mut().capture(TextCapture {
                    text: capture.text,
                    source: CaptureSource {
                        window_handle: capture.source.window_handle,
                        process_id: capture.source.process_id,
                        process_name: capture.source.process_name,
                        window_title: capture.source.window_title,
                    },
                    captured_at_unix_ms: capture.captured_at_unix_ms,
                }) {
                    error!(%error, "failed to persist clipboard capture");
                }
            }
            PlatformEvent::ClipboardPanelRequested(request) => {
                let request_age =
                    nxtools_ipc::unix_time_ms().saturating_sub(request.requested_at_unix_ms);
                if request_age <= MAX_PANEL_REQUEST_AGE_MS {
                    self.show_panel(request, cx);
                } else {
                    info!(request_age, "ignored stale clipboard panel request");
                }
            }
            PlatformEvent::ForegroundChanged(_) => {}
        }
    }

    fn show_panel(&mut self, request: ClipboardPanelRequest, cx: &mut Context<Self>) {
        if let Some(existing) = self.popup_window.take() {
            let _ = existing.update(cx, |_, window, _| window.remove_window());
        }
        self.popup_panel = None;

        let (bounds, display_id) = panel_bounds(&request, cx);
        let history = Rc::clone(&self.history);
        let platform = Rc::clone(&self.platform);
        let controller = cx.entity().downgrade();
        let target = request.target.clone();
        let panel_slot = Rc::new(RefCell::new(None));
        let opened_panel_slot = Rc::clone(&panel_slot);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                focus: true,
                show: true,
                kind: WindowKind::PopUp,
                is_movable: true,
                is_resizable: false,
                is_minimizable: false,
                display_id,
                window_background: WindowBackgroundAppearance::Transparent,
                app_id: Some("NxTools.Clipboard".into()),
                window_min_size: Some(size(px(280.0), px(280.0))),
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
            },
            move |window, cx| {
                let panel = cx.new(|cx| {
                    ClipboardPanel::new(history, platform, target, controller, window, cx)
                });
                *opened_panel_slot.borrow_mut() = Some(panel.downgrade());
                cx.new(|cx| Root::new(panel, window, cx))
            },
        );

        match opened {
            Ok(window) => {
                self.popup_window = Some(window.into());
                self.popup_panel = panel_slot.borrow_mut().take();
                info!(
                    target = %request.target.process_name,
                    x = request.cursor_x,
                    y = request.cursor_y,
                    "clipboard popup opened"
                );
            }
            Err(error) => error!(%error, "failed to open clipboard popup"),
        }
    }

    fn popup_closed(&mut self) {
        self.popup_window = None;
        self.popup_panel = None;
    }
}

struct ClipboardPanel {
    theme: Theme,
    history: Rc<RefCell<ClipboardHistory>>,
    platform: Rc<PlatformBridge>,
    target: nxtools_protocol::ForegroundContext,
    controller: WeakEntity<ClipboardController>,
    entries: Vec<ClipboardItem>,
    status: String,
    pending_paste: Option<uuid::Uuid>,
    search_input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl ClipboardPanel {
    fn new(
        history: Rc<RefCell<ClipboardHistory>>,
        platform: Rc<PlatformBridge>,
        target: nxtools_protocol::ForegroundContext,
        controller: WeakEntity<ClipboardController>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let entries = history
            .borrow()
            .recent(HISTORY_LIMIT)
            .unwrap_or_else(|error| {
                error!(%error, "failed to load clipboard popup history");
                Vec::new()
            });
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("搜索剪贴板历史"));
        search_input.focus_handle(cx).focus(window);

        let search_subscription = cx.subscribe_in(
            &search_input,
            window,
            |this, state, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    let query = state.read(cx).value().to_string();
                    this.reload_entries(&query);
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
            "选择一条记录进行粘贴".into()
        } else {
            format!("将粘贴到 {}", target.process_name)
        };
        Self {
            theme: Theme::zed_dark(),
            history,
            platform,
            target,
            controller,
            entries,
            status,
            pending_paste: None,
            search_input,
            _subscriptions: vec![
                search_subscription,
                activation_subscription,
                keystroke_subscription,
            ],
        }
    }

    fn reload_entries(&mut self, query: &str) {
        let result = if query.is_empty() {
            self.history.borrow().recent(HISTORY_LIMIT)
        } else {
            self.history.borrow().search(query, HISTORY_LIMIT)
        };
        match result {
            Ok(entries) => self.entries = entries,
            Err(error) => {
                error!(%error, "failed to query clipboard popup history");
                self.entries.clear();
                self.status = "查询失败，请查看日志".into();
            }
        }
    }

    fn paste_entry(&mut self, entry: &ClipboardItem, _window: &mut Window, cx: &mut Context<Self>) {
        if self.pending_paste.is_some() {
            self.status = "正在等待粘贴结果…".into();
            cx.notify();
            return;
        }
        if self.target.window_handle == 0 {
            self.status = "原目标窗口已经不可用".into();
            cx.notify();
            return;
        }
        match self
            .platform
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: entry.text.clone(),
                target_window_handle: self.target.window_handle,
                target_focus_window_handle: self.target.focus_window_handle,
            })) {
            Ok(command_id) => {
                self.pending_paste = Some(command_id);
                self.status = "正在粘贴…".into();
                info!(item_id = entry.id, %command_id, target = %self.target.process_name, "paste command sent");
                cx.notify();
            }
            Err(error) => {
                self.status = format!("无法粘贴：{error}");
                cx.notify();
            }
        }
    }

    fn handle_command_result(
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
            self.close(window, cx);
        } else {
            self.status = format!(
                "粘贴失败：{}",
                result.error.unwrap_or_else(|| "Windows 未返回原因".into())
            );
            window.activate_window();
            cx.notify();
        }
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.controller.update(cx, |controller, _| {
            controller.popup_closed();
        });
        window.remove_window();
    }
}

impl Render for ClipboardPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let entries_empty = self.entries.is_empty();
        div()
            .size_full()
            .p_2()
            .rounded_lg()
            .border_1()
            .border_color(theme.border_active)
            .bg(theme.surface)
            .shadow_lg()
            .text_color(theme.text)
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .id("clipboard-panel-header")
                    .h(px(28.0))
                    .flex()
                    .items_center()
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
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(self.status.clone()),
                    ),
            )
            .child(
                div()
                    .h(px(36.0))
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border_active)
                    .bg(theme.canvas)
                    .px_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child("⌕")
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
                    .flex_1()
                    .min_h_0()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
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
                            .h_full(),
                        )
                    }),
            )
            .child(
                div()
                    .h(px(18.0))
                    .px_1()
                    .flex()
                    .items_center()
                    .text_xs()
                    .text_color(theme.text_muted)
                    .child(format!("{} 条结果", self.entries.len()))
                    .child(div().flex_1())
                    .child("Esc 关闭 · 单击粘贴"),
            )
    }
}

fn panel_bounds(
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
