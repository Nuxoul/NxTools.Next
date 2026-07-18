use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::rc::Rc;

use gpui::{
    AnyWindowHandle, AppContext, Context, Entity, Global, Task, WeakEntity,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions, px,
    size,
};
use gpui_component::Root;
use nxtools_clipboard::{
    CaptureSource, ClipboardHistory, ClipboardHistoryError, ClipboardItem, ClipboardKind,
    FilesCapture, ImageCapture, TextCapture,
};
use nxtools_ipc::{PlatformBridge, PlatformBridgeError, PlatformNotification};
use nxtools_protocol::{
    ClipboardPanelRequest, ForegroundContext, PasteFilesCommand, PasteImageCommand,
    PasteTextCommand, PlatformCommand, PlatformEvent, SetWindowTopmostCommand,
    SetWindowVisibilityCommand,
};
use sha2::{Digest, Sha256};
use tracing::{error, info};

use crate::popup_lifecycle::{PopupLifecycle, TopmostResolution};
use crate::ui::{ClipboardPanel, PANEL_HEIGHT, PANEL_WIDTH, native_window_handle, panel_bounds};

const HISTORY_LIMIT: usize = 500;
const MAX_PANEL_REQUEST_AGE_MS: i64 = 2_000;

fn map_source(source: ForegroundContext) -> CaptureSource {
    CaptureSource {
        window_handle: source.window_handle,
        process_id: source.process_id,
        process_name: source.process_name,
        window_title: source.window_title,
    }
}

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
    image_dir: PathBuf,
    platform: Rc<PlatformBridge>,
    popup: PopupSession,
    _platform_task: Task<()>,
}

#[derive(Clone)]
struct PopupWindow {
    window: AnyWindowHandle,
    panel: WeakEntity<ClipboardPanel>,
    native_window_handle: i64,
}

#[derive(Default)]
struct PopupSession {
    lifecycle: PopupLifecycle<PopupWindow>,
    pending_visibility: HashMap<uuid::Uuid, AnyWindowHandle>,
}

impl ClipboardApplication {
    pub fn new(
        history: ClipboardHistory,
        image_dir: PathBuf,
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
            image_dir,
            platform,
            popup: PopupSession::default(),
            _platform_task: platform_task,
        }
    }

    pub(crate) fn query_entries(
        &self,
        query: &str,
        kind: Option<ClipboardKind>,
    ) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        match (query.is_empty(), kind) {
            (true, None) => self.history.borrow().recent(HISTORY_LIMIT),
            (true, Some(kind)) => self.history.borrow().recent_by_kind(kind, HISTORY_LIMIT),
            (false, None) => self.history.borrow().search(query, HISTORY_LIMIT),
            (false, Some(kind)) => self
                .history
                .borrow()
                .search_by_kind(query, kind, HISTORY_LIMIT),
        }
    }

    pub(crate) fn request_paste(
        &self,
        entry: &ClipboardItem,
        target: &ForegroundContext,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        let command = match entry.kind {
            ClipboardKind::Text => PlatformCommand::PasteText(PasteTextCommand {
                text: entry.text.clone(),
                target_window_handle: target.window_handle,
                target_focus_window_handle: target.focus_window_handle,
            }),
            ClipboardKind::Files => PlatformCommand::PasteFiles(PasteFilesCommand {
                paths: entry.file_paths.clone(),
                target_window_handle: target.window_handle,
                target_focus_window_handle: target.focus_window_handle,
            }),
            ClipboardKind::Image => PlatformCommand::PasteImage(PasteImageCommand {
                image_path: entry.image_path.clone(),
                target_window_handle: target.window_handle,
                target_focus_window_handle: target.focus_window_handle,
            }),
        };
        let command_id = self.platform.send_command(command)?;
        info!(item_id = entry.id, %command_id, target = %target.process_name, "paste command sent");
        Ok(command_id)
    }

    pub(crate) fn hide_popup(
        &mut self,
        popup_window: AnyWindowHandle,
        native_window_handle: i64,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        let command_id = self
            .platform
            .send_command(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand {
                    window_handle: native_window_handle,
                    visible: false,
                },
            ))?;
        self.popup
            .pending_visibility
            .insert(command_id, popup_window);
        info!(%command_id, window_handle = native_window_handle, "clipboard popup hide command sent");
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
                if let Some(pending_window) = self.popup.pending_visibility.remove(&correlation_id)
                {
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
                            .filter(|popup| popup.window == pending_window)
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
            PlatformEvent::ClipboardCaptured(capture) => {
                let source = map_source(capture.source);
                if !capture.paths.is_empty() {
                    if let Some(image) = capture.image {
                        let _ = std::fs::remove_file(image.staging_path);
                    }
                    if let Err(error) = self.history.borrow_mut().capture_files(FilesCapture {
                        paths: capture.paths,
                        source,
                        captured_at_unix_ms: capture.captured_at_unix_ms,
                    }) {
                        error!(%error, "failed to persist clipboard files capture");
                    }
                } else if let Some(image) = capture.image {
                    match self.store_captured_image(&image.staging_path) {
                        Ok(image_path) => {
                            if let Err(error) =
                                self.history.borrow_mut().capture_image(ImageCapture {
                                    image_path,
                                    width: image.width,
                                    height: image.height,
                                    source,
                                    captured_at_unix_ms: capture.captured_at_unix_ms,
                                })
                            {
                                error!(%error, "failed to persist clipboard image capture");
                            }
                        }
                        Err(error) => error!(%error, "failed to ingest staged clipboard image"),
                    }
                } else if let Some(text) = capture.text
                    && !text.is_empty()
                    && let Err(error) = self.history.borrow_mut().capture(TextCapture {
                        text,
                        source,
                        captured_at_unix_ms: capture.captured_at_unix_ms,
                    })
                {
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
            PlatformEvent::ForegroundChanged(foreground) => {
                let Some(popup) = self.popup.lifecycle.current().cloned() else {
                    return;
                };
                let _ = popup.window.update(cx, |_, window, app| {
                    let _ = popup.panel.update(app, |panel, cx| {
                        panel.handle_foreground_changed(
                            popup.native_window_handle,
                            foreground.window_handle,
                            window,
                            cx,
                        );
                    });
                });
            }
        }
    }

    fn store_captured_image(&self, staging_path: &str) -> std::io::Result<String> {
        let staging = std::path::Path::new(staging_path);
        let fingerprint = image_fingerprint(std::fs::File::open(staging)?)?;
        let destination = self.image_dir.join(format!("{fingerprint}.png"));
        if !destination.exists() {
            std::fs::copy(staging, &destination)?;
        }
        let _ = std::fs::remove_file(staging);
        Ok(destination.to_string_lossy().into_owned())
    }

    fn show_panel(&mut self, request: ClipboardPanelRequest, cx: &mut Context<Self>) {
        if self.popup.lifecycle.replacement_pending() {
            info!(
                "ignored clipboard panel request while a replacement is awaiting topmost confirmation"
            );
            return;
        }

        let entries = self.query_entries("", None).unwrap_or_else(|error| {
            error!(%error, "failed to load clipboard popup history");
            Vec::new()
        });
        let (bounds, display_id) = panel_bounds(&request, cx);
        let platform = Rc::clone(&self.platform);
        let application = cx.entity().downgrade();
        let target = request.target.clone();
        let panel_slot = Rc::new(RefCell::new(None));
        let native_window_slot = Rc::new(RefCell::new(None));
        let topmost_slot = Rc::new(RefCell::new(None));
        let opened_panel_slot = Rc::clone(&panel_slot);
        let opened_native_window_slot = Rc::clone(&native_window_slot);
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
                let native_window = native_window_handle(window);
                *opened_native_window_slot.borrow_mut() = native_window;
                let topmost_result = if let Some(window_handle) = native_window {
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
                let native_window_handle = native_window_slot.borrow_mut().take().unwrap_or(0);
                let popup = PopupWindow {
                    window: window.into(),
                    panel,
                    native_window_handle,
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

fn image_fingerprint(mut reader: impl Read) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod image_tests {
    use super::image_fingerprint;

    #[test]
    fn image_fingerprint_depends_on_content_instead_of_staging_path() {
        let first = image_fingerprint(std::io::Cursor::new(b"same image bytes"))
            .expect("fingerprint first image");
        let second = image_fingerprint(std::io::Cursor::new(b"same image bytes"))
            .expect("fingerprint second image");
        let different = image_fingerprint(std::io::Cursor::new(b"different image bytes"))
            .expect("fingerprint different image");

        assert_eq!(first, second);
        assert_ne!(first, different);
    }
}
