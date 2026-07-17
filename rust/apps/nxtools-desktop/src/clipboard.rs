use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    AnyWindowHandle, AppContext, Context, Entity, Global, Task, WeakEntity,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions, px,
    size,
};
use gpui_component::Root;
use nxtools_clipboard::{
    CaptureSource, ClipboardHistory, ClipboardHistoryError, ClipboardItem, TextCapture,
};
use nxtools_ipc::{PlatformBridge, PlatformBridgeError, PlatformNotification};
use nxtools_protocol::{
    ClipboardPanelRequest, ForegroundContext, PasteTextCommand, PlatformCommand, PlatformEvent,
    SetWindowTopmostCommand,
};
use tracing::{error, info};

use crate::ui::{ClipboardPanel, PANEL_HEIGHT, PANEL_WIDTH, native_window_handle, panel_bounds};

const HISTORY_LIMIT: usize = 500;
const MAX_PANEL_REQUEST_AGE_MS: i64 = 2_000;

pub struct ClipboardApplicationGlobal {
    _application: Entity<ClipboardApplication>,
}

impl ClipboardApplicationGlobal {
    pub fn new(application: Entity<ClipboardApplication>) -> Self {
        Self {
            _application: application,
        }
    }
}

impl Global for ClipboardApplicationGlobal {}

pub struct ClipboardApplication {
    history: Rc<RefCell<ClipboardHistory>>,
    platform: Rc<PlatformBridge>,
    popup: PopupSession,
    _platform_task: Task<()>,
}

#[derive(Default)]
struct PopupSession {
    window: Option<AnyWindowHandle>,
    panel: Option<WeakEntity<ClipboardPanel>>,
    pending_topmost: Option<uuid::Uuid>,
}

impl PopupSession {
    fn clear(&mut self) {
        self.window = None;
        self.panel = None;
        self.pending_topmost = None;
    }
}

impl ClipboardApplication {
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
            popup: PopupSession::default(),
            _platform_task: platform_task,
        }
    }

    pub(crate) fn query_entries(
        &self,
        query: &str,
    ) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        if query.is_empty() {
            self.history.borrow().recent(HISTORY_LIMIT)
        } else {
            self.history.borrow().search(query, HISTORY_LIMIT)
        }
    }

    pub(crate) fn request_paste(
        &self,
        entry: &ClipboardItem,
        target: &ForegroundContext,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        let command_id =
            self.platform
                .send_command(PlatformCommand::PasteText(PasteTextCommand {
                    text: entry.text.clone(),
                    target_window_handle: target.window_handle,
                    target_focus_window_handle: target.focus_window_handle,
                }))?;
        info!(item_id = entry.id, %command_id, target = %target.process_name, "paste command sent");
        Ok(command_id)
    }

    pub(crate) fn popup_closed(&mut self) {
        self.popup.clear();
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
                if self.popup.pending_topmost == Some(correlation_id) {
                    self.popup.pending_topmost = None;
                    if result.success {
                        info!(%correlation_id, "clipboard window is topmost");
                    } else {
                        error!(
                            %correlation_id,
                            reason = result.error.as_deref().unwrap_or("unknown Windows error"),
                            "failed to make clipboard window topmost"
                        );
                        let window = self.popup.window.take();
                        self.popup.clear();
                        if let Some(window) = window {
                            let _ = window.update(cx, |_, window, _| window.remove_window());
                        }
                    }
                    return;
                }
                let (Some(window), Some(panel)) =
                    (self.popup.window, self.popup.panel.as_ref().cloned())
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
        if let Some(existing) = self.popup.window.take() {
            let _ = existing.update(cx, |_, window, _| window.remove_window());
        }
        self.popup.clear();

        let entries = self.query_entries("").unwrap_or_else(|error| {
            error!(%error, "failed to load clipboard popup history");
            Vec::new()
        });
        let (bounds, display_id) = panel_bounds(&request, cx);
        let platform = Rc::clone(&self.platform);
        let application = cx.entity().downgrade();
        let target = request.target.clone();
        let panel_slot = Rc::new(RefCell::new(None));
        let topmost_slot = Rc::new(RefCell::new(None));
        let opened_panel_slot = Rc::clone(&panel_slot);
        let opened_topmost_slot = Rc::clone(&topmost_slot);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                focus: true,
                show: true,
                kind: WindowKind::PopUp,
                is_movable: true,
                is_resizable: true,
                is_minimizable: false,
                display_id,
                window_background: WindowBackgroundAppearance::Transparent,
                app_id: Some("NxTools.Clipboard".into()),
                window_min_size: Some(size(px(280.0), px(280.0))),
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
            },
            move |window, cx| {
                let topmost_result = if let Some(window_handle) = native_window_handle(window) {
                    platform
                        .send_command(PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                            window_handle,
                            enabled: true,
                        }))
                        .map_err(|error| error.to_string())
                } else {
                    Err("GPUI did not expose a native Windows handle".to_owned())
                };
                *opened_topmost_slot.borrow_mut() = Some(topmost_result);
                let panel =
                    cx.new(|cx| ClipboardPanel::new(entries, target, application, window, cx));
                *opened_panel_slot.borrow_mut() = Some(panel.downgrade());
                cx.new(|cx| Root::new(panel, window, cx))
            },
        );

        match opened {
            Ok(window) => {
                let topmost_result = topmost_slot.borrow_mut().take();
                let Some(Ok(topmost_command)) = topmost_result else {
                    let reason = match topmost_result {
                        Some(Err(reason)) => reason,
                        _ => "topmost request was not created".to_owned(),
                    };
                    error!(%reason, "clipboard popup closed because topmost could not be requested");
                    let _ = window.update(cx, |_, window, _| window.remove_window());
                    return;
                };
                self.popup.window = Some(window.into());
                self.popup.panel = panel_slot.borrow_mut().take();
                self.popup.pending_topmost = Some(topmost_command);
                info!(
                    target = %request.target.process_name,
                    x = request.cursor_x,
                    y = request.cursor_y,
                    width = PANEL_WIDTH,
                    height = PANEL_HEIGHT,
                    "clipboard popup opened"
                );
            }
            Err(error) => error!(%error, "failed to open clipboard popup"),
        }
    }
}
