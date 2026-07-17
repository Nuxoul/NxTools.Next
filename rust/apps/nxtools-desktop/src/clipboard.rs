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
    SetWindowTopmostCommand, SetWindowVisibilityCommand,
};
use tracing::{error, info};

use crate::popup_lifecycle::{PopupLifecycle, TopmostResolution};
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

#[derive(Clone)]
struct PopupWindow {
    window: AnyWindowHandle,
    panel: WeakEntity<ClipboardPanel>,
}

struct PendingVisibility {
    correlation_id: uuid::Uuid,
    window: AnyWindowHandle,
}

#[derive(Default)]
struct PopupSession {
    lifecycle: PopupLifecycle<PopupWindow>,
    pending_visibility: Option<PendingVisibility>,
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

    pub(crate) fn hide_popup(
        &mut self,
        window_handle: i64,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        let command_id = self
            .platform
            .send_command(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand {
                    window_handle,
                    visible: false,
                },
            ))?;
        if let Some(popup) = self.popup.lifecycle.current() {
            self.popup.pending_visibility = Some(PendingVisibility {
                correlation_id: command_id,
                window: popup.window,
            });
        }
        info!(%command_id, window_handle, "clipboard popup hide command sent");
        Ok(command_id)
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
                if self
                    .popup
                    .pending_visibility
                    .as_ref()
                    .is_some_and(|pending| pending.correlation_id == correlation_id)
                {
                    let pending = self.popup.pending_visibility.take().expect("checked above");
                    if result.success {
                        info!(%correlation_id, "clipboard popup hidden without closing its GPUI window");
                    } else {
                        let reason = result
                            .error
                            .as_deref()
                            .unwrap_or("unknown Windows error")
                            .to_owned();
                        error!(
                            %correlation_id,
                            %reason,
                            "failed to hide clipboard popup"
                        );
                        if let Some(popup) = self
                            .popup
                            .lifecycle
                            .current()
                            .filter(|popup| popup.window == pending.window)
                            .cloned()
                        {
                            let _ = popup.window.update(cx, |_, window, app| {
                                let _ = popup.panel.update(app, |panel, cx| {
                                    panel.handle_hide_failure(reason, window, cx);
                                });
                            });
                        }
                    }
                    return;
                }
                let topmost_reason = result
                    .error
                    .as_deref()
                    .unwrap_or("unknown Windows error")
                    .to_owned();
                if let Some(resolution) = self
                    .popup
                    .lifecycle
                    .resolve_topmost(correlation_id, result.success)
                {
                    match resolution {
                        TopmostResolution::Confirmed { previous } => {
                            info!(%correlation_id, "clipboard window is topmost");
                            if let Some(previous) = previous {
                                let _ = previous
                                    .window
                                    .update(cx, |_, window, _| window.remove_window());
                            }
                        }
                        TopmostResolution::Rejected { previous } => {
                            error!(
                                %correlation_id,
                                reason = %topmost_reason,
                                "clipboard window remains open because topmost is nonfatal"
                            );
                            if let Some(previous) = previous {
                                let _ = previous
                                    .window
                                    .update(cx, |_, window, _| window.remove_window());
                            }
                            if let Some(current) = self.popup.lifecycle.current().cloned() {
                                let _ = current.window.update(cx, |_, window, app| {
                                    let _ = current.panel.update(app, |panel, cx| {
                                        panel.handle_topmost_failure(topmost_reason, window, cx);
                                    });
                                });
                            }
                        }
                    }
                    return;
                }
                let Some(popup) = self.popup.lifecycle.current().cloned() else {
                    return;
                };
                let _ = popup.window.update(cx, |_, window, app| {
                    let _ = popup.panel.update(app, |panel, cx| {
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
        if self.popup.lifecycle.replacement_pending() {
            info!(
                "ignored clipboard panel request while a replacement is awaiting topmost confirmation"
            );
            return;
        }

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
                let Some(panel) = panel_slot.borrow_mut().take() else {
                    error!("clipboard popup opened without a panel entity");
                    let _ = window.update(cx, |_, window, _| window.remove_window());
                    return;
                };
                let popup = PopupWindow {
                    window: window.into(),
                    panel,
                };
                let Some(Ok(topmost_command)) = topmost_result else {
                    let reason = match topmost_result {
                        Some(Err(reason)) => reason,
                        _ => "topmost request was not created".to_owned(),
                    };
                    error!(%reason, "clipboard popup topmost request could not be created");
                    if let Some(previous) = self
                        .popup
                        .lifecycle
                        .replace_without_topmost_confirmation(popup)
                    {
                        let _ = previous
                            .window
                            .update(cx, |_, window, _| window.remove_window());
                    }
                    if let Some(current) = self.popup.lifecycle.current().cloned() {
                        let _ = current.window.update(cx, |_, window, app| {
                            let _ = current.panel.update(app, |panel, cx| {
                                panel.handle_topmost_failure(reason, window, cx);
                            });
                        });
                    }
                    return;
                };
                self.popup.lifecycle.stage(popup, topmost_command);
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
