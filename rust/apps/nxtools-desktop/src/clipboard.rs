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
    CaptureSource, ClipboardAction, ClipboardHistory, ClipboardHistoryError, ClipboardItem,
    ClipboardKind, ClipboardNativeFormat as StoredClipboardNativeFormat, FilesCapture,
    ImageCapture, TextCapture, action_text_payload,
};
use nxtools_ipc::{PlatformBridge, PlatformBridgeError, PlatformNotification};
use nxtools_protocol::{
    ClipboardNativeFormat as ProtocolClipboardNativeFormat, ClipboardPanelRequest,
    ForegroundContext, PasteFilesCommand, PasteImageCommand, PasteNativeFormatsCommand,
    PasteTextCommand, PlatformCommand, PlatformEvent, SetWindowTopmostCommand,
    SetWindowVisibilityCommand,
};
use sha2::{Digest, Sha256};
use tracing::{error, info};

use crate::popup_lifecycle::{PopupLifecycle, TopmostResolution};
use crate::ui::{ClipboardPanel, PANEL_HEIGHT, PANEL_WIDTH, native_window_handle, panel_bounds};

const HISTORY_LIMIT: usize = 500;
const MAX_PANEL_REQUEST_AGE_MS: i64 = 2_000;
const CF_UNICODETEXT: u32 = 13;
const CF_HDROP: u32 = 15;
const CF_DIB: u32 = 8;
const CF_DIBV5: u32 = 17;

fn map_source(source: ForegroundContext, process_icon_path: String) -> CaptureSource {
    CaptureSource {
        window_handle: source.window_handle,
        process_id: source.process_id,
        process_name: source.process_name,
        process_icon_path,
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
    clipboard_asset_dir: PathBuf,
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
        clipboard_asset_dir: PathBuf,
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
            clipboard_asset_dir,
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

    pub(crate) fn set_item_favorite(
        &self,
        item_id: i64,
        value: bool,
    ) -> Result<ClipboardItem, ClipboardHistoryError> {
        self.history.borrow_mut().set_favorite(item_id, value)
    }

    pub(crate) fn set_item_pinned(
        &self,
        item_id: i64,
        value: bool,
    ) -> Result<ClipboardItem, ClipboardHistoryError> {
        self.history.borrow_mut().set_pinned(item_id, value)
    }

    pub(crate) fn delete_item(&self, item_id: i64) -> Result<(), ClipboardHistoryError> {
        self.history
            .borrow_mut()
            .soft_delete(item_id, nxtools_ipc::unix_time_ms())
    }

    pub(crate) fn restore_item(
        &self,
        item_id: i64,
    ) -> Result<ClipboardItem, ClipboardHistoryError> {
        self.history.borrow_mut().restore(item_id)
    }

    pub(crate) fn update_item_text(
        &self,
        item_id: i64,
        text: &str,
    ) -> Result<ClipboardItem, ClipboardHistoryError> {
        self.history.borrow_mut().update_text(item_id, text)
    }

    pub(crate) fn request_clipboard_action(
        &self,
        entry: &ClipboardItem,
        target: &ForegroundContext,
        action: ClipboardAction,
    ) -> anyhow::Result<uuid::Uuid> {
        let native_formats = if matches!(
            action,
            ClipboardAction::OriginalPaste | ClipboardAction::CopyOnly
        ) {
            self.history.borrow().native_formats(entry.id)?
        } else {
            Vec::new()
        };
        let command = command_for_action(entry, target, action, native_formats)?;
        let command_id = self.platform.send_command(command)?;
        info!(item_id = entry.id, ?action, %command_id, target = %target.process_name, "clipboard action command sent");
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
}

fn command_for_action(
    entry: &ClipboardItem,
    target: &ForegroundContext,
    action: ClipboardAction,
    native_formats: Vec<StoredClipboardNativeFormat>,
) -> anyhow::Result<PlatformCommand> {
    if action == ClipboardAction::WithoutLineBreaks && entry.kind != ClipboardKind::Text {
        anyhow::bail!("去换行仅适用于文本条目");
    }
    let paste_after_write = action != ClipboardAction::CopyOnly;
    if native_formats_restore_primary_kind(entry.kind, &native_formats) {
        return Ok(PlatformCommand::PasteNativeFormats(
            PasteNativeFormatsCommand {
                formats: native_formats
                    .into_iter()
                    .map(|format| ProtocolClipboardNativeFormat {
                        format_id: format.format_id,
                        format_name: format.format_name,
                        data_path: format.data_path,
                        size_bytes: format.size_bytes,
                        sort_order: format.sort_order,
                    })
                    .collect(),
                paste_after_write,
                target_window_handle: target.window_handle,
                target_focus_window_handle: target.focus_window_handle,
            },
        ));
    }
    if let Some(text) = action_text_payload(entry, action) {
        return Ok(PlatformCommand::PasteText(PasteTextCommand {
            text,
            paste_after_write,
            target_window_handle: target.window_handle,
            target_focus_window_handle: target.focus_window_handle,
        }));
    }
    Ok(match entry.kind {
        ClipboardKind::Text => PlatformCommand::PasteText(PasteTextCommand {
            text: entry.text.clone(),
            paste_after_write,
            target_window_handle: target.window_handle,
            target_focus_window_handle: target.focus_window_handle,
        }),
        ClipboardKind::Files => PlatformCommand::PasteFiles(PasteFilesCommand {
            paths: entry.file_paths.clone(),
            paste_after_write,
            target_window_handle: target.window_handle,
            target_focus_window_handle: target.focus_window_handle,
        }),
        ClipboardKind::Image => PlatformCommand::PasteImage(PasteImageCommand {
            image_path: entry.image_path.clone(),
            paste_after_write,
            target_window_handle: target.window_handle,
            target_focus_window_handle: target.focus_window_handle,
        }),
    })
}

fn native_formats_restore_primary_kind(
    kind: ClipboardKind,
    formats: &[StoredClipboardNativeFormat],
) -> bool {
    match kind {
        ClipboardKind::Text => formats
            .iter()
            .any(|format| format.format_id == CF_UNICODETEXT),
        ClipboardKind::Files => formats.iter().any(|format| format.format_id == CF_HDROP),
        ClipboardKind::Image => formats
            .iter()
            .any(|format| matches!(format.format_id, CF_DIB | CF_DIBV5)),
    }
}

impl ClipboardApplication {
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
                let native_formats = self.ingest_native_formats(capture.native_formats);
                let process_icon_path = capture
                    .source_icon_staging_path
                    .as_deref()
                    .and_then(|path| self.store_source_icon(path).ok())
                    .unwrap_or_default();
                let source = map_source(capture.source, process_icon_path);
                if !capture.paths.is_empty() {
                    if let Some(image) = capture.image {
                        let _ = std::fs::remove_file(image.staging_path);
                    }
                    if let Err(error) = self.history.borrow_mut().capture_files(FilesCapture {
                        paths: capture.paths,
                        native_formats,
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
                                    native_formats,
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
                        native_formats,
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
        self.store_captured_asset(staging_path, "", true)
    }

    fn ingest_native_formats(
        &self,
        formats: Vec<ProtocolClipboardNativeFormat>,
    ) -> Vec<StoredClipboardNativeFormat> {
        formats
            .into_iter()
            .filter_map(|format| match self.store_native_format(&format.data_path) {
                Ok(data_path) => Some(StoredClipboardNativeFormat {
                    format_id: format.format_id,
                    format_name: format.format_name,
                    data_path,
                    size_bytes: format.size_bytes,
                    sort_order: format.sort_order,
                }),
                Err(error) => {
                    error!(
                        %error,
                        format_id = format.format_id,
                        size_bytes = format.size_bytes,
                        "failed to ingest staged native clipboard format"
                    );
                    let _ = std::fs::remove_file(&format.data_path);
                    None
                }
            })
            .collect()
    }

    fn store_native_format(&self, staging_path: &str) -> std::io::Result<String> {
        let staging = std::path::Path::new(staging_path);
        let fingerprint = content_fingerprint(std::fs::File::open(staging)?)?;
        let format_dir = self.clipboard_asset_dir.join("native-formats");
        std::fs::create_dir_all(&format_dir)?;
        let destination = format_dir.join(format!("{fingerprint}.bin"));
        if !destination.exists() {
            std::fs::copy(staging, &destination)?;
        }
        let _ = std::fs::remove_file(staging);
        Ok(destination.to_string_lossy().into_owned())
    }

    fn store_source_icon(&self, staging_path: &str) -> std::io::Result<String> {
        self.store_captured_asset(staging_path, "source-", false)
    }

    fn store_captured_asset(
        &self,
        staging_path: &str,
        file_name_prefix: &str,
        remove_source: bool,
    ) -> std::io::Result<String> {
        let staging = std::path::Path::new(staging_path);
        let fingerprint = content_fingerprint(std::fs::File::open(staging)?)?;
        let destination = self
            .clipboard_asset_dir
            .join(format!("{file_name_prefix}{fingerprint}.png"));
        if !destination.exists() {
            std::fs::copy(staging, &destination)?;
        }
        if remove_source {
            let _ = std::fs::remove_file(staging);
        }
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

fn content_fingerprint(mut reader: impl Read) -> std::io::Result<String> {
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
mod asset_tests {
    use super::*;

    #[test]
    fn content_fingerprint_depends_on_bytes_instead_of_staging_path() {
        let first = content_fingerprint(std::io::Cursor::new(b"same image bytes"))
            .expect("fingerprint first image");
        let second = content_fingerprint(std::io::Cursor::new(b"same image bytes"))
            .expect("fingerprint second image");
        let different = content_fingerprint(std::io::Cursor::new(b"different image bytes"))
            .expect("fingerprint different image");

        assert_eq!(first, second);
        assert_ne!(first, different);
    }

    fn fixture_target() -> ForegroundContext {
        ForegroundContext {
            window_handle: 42,
            focus_window_handle: 43,
            process_id: 7,
            process_name: "editor".into(),
            window_title: "draft".into(),
        }
    }

    fn fixture_text() -> ClipboardItem {
        ClipboardItem {
            id: 1,
            kind: ClipboardKind::Text,
            text: "first\r\nsecond".into(),
            image_path: String::new(),
            image_width: 0,
            image_height: 0,
            file_paths: Vec::new(),
            source: CaptureSource {
                window_handle: 1,
                process_id: 2,
                process_name: "browser".into(),
                process_icon_path: String::new(),
                window_title: "source".into(),
            },
            captured_at_unix_ms: 10,
            copy_count: 1,
            is_favorite: false,
            is_pinned: false,
            deleted_at_unix_ms: None,
        }
    }

    fn unicode_native_format() -> StoredClipboardNativeFormat {
        StoredClipboardNativeFormat {
            format_id: 13,
            format_name: String::new(),
            data_path: "C:/formats/unicode.bin".into(),
            size_bytes: 28,
            sort_order: 0,
        }
    }

    #[test]
    fn four_clipboard_actions_build_the_expected_platform_outcomes() {
        let entry = fixture_text();
        let target = fixture_target();

        let original = command_for_action(
            &entry,
            &target,
            ClipboardAction::OriginalPaste,
            vec![unicode_native_format()],
        )
        .expect("original action");
        let copy = command_for_action(
            &entry,
            &target,
            ClipboardAction::CopyOnly,
            vec![unicode_native_format()],
        )
        .expect("copy action");
        let plain = command_for_action(&entry, &target, ClipboardAction::PlainText, Vec::new())
            .expect("plain action");
        let without_breaks = command_for_action(
            &entry,
            &target,
            ClipboardAction::WithoutLineBreaks,
            Vec::new(),
        )
        .expect("without line breaks action");

        let fixture_commands =
            include_str!("../../../../tests/fixtures/clipboard_action_commands.jsonl")
                .lines()
                .map(|line| {
                    nxtools_protocol::Envelope::decode_line(line)
                        .expect("decode shared action fixture")
                })
                .map(|envelope| match envelope.message {
                    nxtools_protocol::Message::PlatformCommand(command) => command,
                    message => panic!("unexpected shared fixture message: {message:?}"),
                })
                .collect::<Vec<_>>();
        assert_eq!(
            vec![
                original.clone(),
                copy.clone(),
                plain.clone(),
                without_breaks.clone()
            ],
            fixture_commands
        );

        assert!(matches!(
            original,
            PlatformCommand::PasteNativeFormats(PasteNativeFormatsCommand {
                paste_after_write: true,
                ..
            })
        ));
        assert!(matches!(
            copy,
            PlatformCommand::PasteNativeFormats(PasteNativeFormatsCommand {
                paste_after_write: false,
                ..
            })
        ));
        assert!(matches!(
            plain,
            PlatformCommand::PasteText(PasteTextCommand {
                ref text,
                paste_after_write: true,
                ..
            }) if text == "first\r\nsecond"
        ));
        assert!(matches!(
            without_breaks,
            PlatformCommand::PasteText(PasteTextCommand {
                ref text,
                paste_after_write: true,
                ..
            }) if text == "firstsecond"
        ));
    }

    #[test]
    fn image_original_uses_dib_bundle_and_shift_rejects_non_text() {
        let mut image = fixture_text();
        image.kind = ClipboardKind::Image;
        image.text.clear();
        image.image_path = "C:/images/capture.png".into();
        image.image_width = 640;
        image.image_height = 480;
        let dib = StoredClipboardNativeFormat {
            format_id: 8,
            format_name: String::new(),
            data_path: "C:/formats/image-dib.bin".into(),
            size_bytes: 1024,
            sort_order: 0,
        };

        assert!(matches!(
            command_for_action(
                &image,
                &fixture_target(),
                ClipboardAction::OriginalPaste,
                vec![dib]
            )
            .expect("image original action"),
            PlatformCommand::PasteNativeFormats(_)
        ));
        assert!(
            command_for_action(
                &image,
                &fixture_target(),
                ClipboardAction::WithoutLineBreaks,
                Vec::new()
            )
            .is_err()
        );
    }
}
