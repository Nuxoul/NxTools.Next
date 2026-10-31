use super::FloatingAssistantApplication;
use gpui_kit::Context;
use nxtools_core::{ClickAction, FloatingAssistantSettings, ImageDropAction, TextDropAction};
use nxtools_protocol::{
    ActivateWindowCommand, PasteImageCommand, PlatformCommand, PlatformCommandResult,
    QuickLaunchPanelRequest, SetWindowVisibilityCommand,
    floating_assistant::{FloatingAssistantEvent, FloatingAssistantEventKind},
};
use std::{
    io::Cursor,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Route {
    None,
    Main,
    Download(Option<String>),
    Search(String),
    CopyImage(String, Option<String>),
    Unsupported,
}
fn http_url(value: &str) -> Option<String> {
    let value = value.trim().trim_end_matches(['.', ',', ';', '。', '，']);
    if value.len() > 8192 {
        return None;
    }
    let url = url::Url::parse(value).ok()?;
    (matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none())
    .then(|| value.to_owned())
}
fn text_url(text: &str) -> Option<String> {
    text.split([
        '\r', '\n', '\t', ' ', '"', '\'', '<', '>', '(', ')', '[', ']',
    ])
    .take(4096)
    .find_map(http_url)
}
fn image_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ["png", "jpg", "jpeg", "bmp", "gif", "webp"]
                .iter()
                .any(|known| ext.eq_ignore_ascii_case(known))
        })
}
/// Only actual IMG src attributes qualify; surrounding hyperlinks/alt text do not.
fn html_image_url(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let mut rest = lower.as_str();
    let mut base = 0;
    while let Some(start) = rest.find("<img") {
        let start = base + start;
        let end = lower[start..].find('>')? + start;
        let tag = &html[start + 4..end];
        let tag_lower = &lower[start + 4..end];
        for (src, _) in tag_lower.match_indices("src") {
            let before = tag_lower[..src].chars().next_back();
            if before.is_none_or(char::is_whitespace) {
                let value = tag[src + 3..].trim_start();
                if let Some(value) = value.strip_prefix('=') {
                    let value = value.trim_start();
                    let candidate = if let Some(quote) =
                        value.chars().next().filter(|c| *c == '\'' || *c == '"')
                    {
                        value[1..].split(quote).next().unwrap_or_default()
                    } else {
                        value.split_whitespace().next().unwrap_or_default()
                    };
                    if let Some(url) = http_url(&candidate.replace("&amp;", "&")) {
                        return Some(url);
                    }
                }
            }
        }
        base = end + 1;
        rest = &lower[base..];
    }
    None
}
pub(crate) fn route(settings: &FloatingAssistantSettings, event: &FloatingAssistantEvent) -> Route {
    let click = match event.kind {
        FloatingAssistantEventKind::LeftClick => Some(settings.left_click_action),
        FloatingAssistantEventKind::RightClick => Some(settings.right_click_action),
        _ => None,
    };
    if let Some(action) = click {
        return match action {
            ClickAction::None => Route::None,
            ClickAction::OpenMain => Route::Main,
            ClickAction::AddDownload => Route::Download(None),
        };
    }
    if event.kind != FloatingAssistantEventKind::Drop {
        return Route::None;
    }
    let remote_image = event.html.as_deref().and_then(html_image_url).or_else(|| {
        event
            .url
            .as_deref()
            .filter(|url| {
                event.image_format.is_some()
                    || event.image_path.is_some()
                    || url::Url::parse(url)
                        .ok()
                        .is_some_and(|url| image_path(url.path()))
            })
            .and_then(http_url)
    });
    let local_image = event.files.iter().find(|path| image_path(path)).cloned();
    let image = event.image_path.clone().or(local_image);
    let is_image = image.is_some() || remote_image.is_some() || event.image_format.is_some();
    if is_image {
        return match settings.image_drop_action {
            ImageDropAction::None => Route::None,
            ImageDropAction::Automatic if remote_image.is_some() => Route::Download(remote_image),
            ImageDropAction::Automatic | ImageDropAction::WriteClipboard => image
                .map(|path| Route::CopyImage(path, event.image_format.clone()))
                .unwrap_or(Route::Unsupported),
            ImageDropAction::AddDownload => remote_image
                .map(|url| Route::Download(Some(url)))
                .unwrap_or(Route::Unsupported),
        };
    }
    if !event.files.is_empty() {
        return Route::Unsupported;
    }
    let text = event
        .text
        .as_deref()
        .or(event.url.as_deref())
        .unwrap_or_default();
    let url = event
        .url
        .as_deref()
        .and_then(http_url)
        .or_else(|| text_url(text));
    match settings.text_drop_action {
        TextDropAction::None => Route::None,
        TextDropAction::Automatic if url.is_some() => Route::Download(url),
        TextDropAction::Automatic | TextDropAction::QuickLaunchSearch
            if !text.trim().is_empty() =>
        {
            Route::Search(text.to_owned())
        }
        TextDropAction::AddDownload => url
            .map(|url| Route::Download(Some(url)))
            .unwrap_or(Route::Unsupported),
        _ => Route::Unsupported,
    }
}

pub(super) struct PendingAction {
    pub(super) started: Instant,
    pub(super) drop_id: Option<Uuid>,
    pub(super) image: Option<OwnedImage>,
    session: Uuid,
    correlation: Uuid,
    dispatched: bool,
    cancelled: bool,
    follow_up: Option<PlatformCommand>,
}
impl PendingAction {
    fn new(session: Uuid, correlation: Uuid, drop_id: Option<Uuid>) -> Self {
        Self {
            started: Instant::now(),
            drop_id,
            image: None,
            session,
            correlation,
            dispatched: false,
            cancelled: false,
            follow_up: None,
        }
    }
}
pub(super) struct OwnedImage {
    path: PathBuf,
    directory: PathBuf,
}
impl Drop for OwnedImage {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_dir(&self.directory);
    }
}
fn prepare_image(path: &str, format: Option<&str>) -> Result<OwnedImage, ()> {
    let file = std::fs::File::open(path).map_err(|_| ())?;
    let metadata = file.metadata().map_err(|_| ())?;
    if !metadata.is_file() || metadata.len() > 16 * 1024 * 1024 {
        return Err(());
    }
    let mut bytes = Vec::new();
    std::io::Read::take(file, 16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if matches!(format, Some("dib" | "dib_v5")) {
        if bytes.len() < 40 {
            return Err(());
        }
        let header = u32::from_le_bytes(bytes[0..4].try_into().map_err(|_| ())?) as usize;
        if header < 40 || header > bytes.len() {
            return Err(());
        }
        let bits = u16::from_le_bytes(bytes[14..16].try_into().map_err(|_| ())?);
        let compression = u32::from_le_bytes(bytes[16..20].try_into().map_err(|_| ())?);
        let colors = u32::from_le_bytes(bytes[32..36].try_into().map_err(|_| ())?) as usize;
        let palette = if colors > 0 {
            colors
        } else if bits <= 8 {
            1_usize << bits
        } else {
            0
        };
        let masks = if header == 40 && compression == 3 {
            12
        } else {
            0
        };
        let offset = 14_usize
            .checked_add(header)
            .and_then(|v| v.checked_add(palette.checked_mul(4)?))
            .and_then(|v| v.checked_add(masks))
            .ok_or(())?;
        if offset > bytes.len() + 14 {
            return Err(());
        }
        let mut bmp = b"BM".to_vec();
        bmp.extend_from_slice(&((bytes.len() + 14) as u32).to_le_bytes());
        bmp.extend_from_slice(&[0; 4]);
        bmp.extend_from_slice(&(offset as u32).to_le_bytes());
        bmp.extend(bytes);
        bytes = bmp;
    }
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| ())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|_| ())?;
    if u64::from(decoded.width()) * u64::from(decoded.height()) > 16_000_000 {
        return Err(());
    }
    let directory = std::env::temp_dir().join(format!("nxtools-floating-copy-{}", Uuid::new_v4()));
    std::fs::create_dir(&directory).map_err(|_| ())?;
    let owned = OwnedImage {
        path: directory.join("image.png"),
        directory,
    };
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&owned.path)
        .map_err(|_| ())?;
    decoded
        .write_to(&mut std::io::BufWriter::new(file), image::ImageFormat::Png)
        .map_err(|_| ())?;
    Ok(owned)
}
use std::io::Read as _;

impl FloatingAssistantApplication {
    pub(super) fn dispatch_event(&mut self, event: FloatingAssistantEvent, cx: &mut Context<Self>) {
        let route = route(self.settings(), &event);
        let id = event.correlation_id;
        let drop_id = event.drop_id;
        tracing::info!(event="floating_assistant.action.accepted",correlation_id=%id,"floating action admitted");
        if matches!(route, Route::None | Route::Unsupported) {
            if route == Route::Unsupported {
                self.notice(
                    "拖入内容没有可用的下载链接或图片数据，无法执行所选动作。",
                    true,
                    cx,
                );
            }
            self.release_drop(drop_id);
            tracing::info!(event="floating_assistant.action.terminal",correlation_id=%id,success=route == Route::None,reason="no_action",elapsed_ms=0,"floating action terminated");
            return;
        }
        if !self.pending_actions.is_empty() {
            self.notice("已有悬浮窗操作正在处理，请稍后重试。", true, cx);
            self.release_drop(drop_id);
            tracing::info!(event="floating_assistant.action.terminal",correlation_id=%id,success=false,reason="busy",elapsed_ms=0,"floating action rejected");
            return;
        }
        self.pending_actions
            .insert(id, PendingAction::new(self.session, id, drop_id));
        match route {
            Route::Main => {
                self.pending_actions
                    .get_mut(&id)
                    .expect("registered action")
                    .follow_up = Some(PlatformCommand::ActivateWindow(ActivateWindowCommand {
                    window_handle: self.source_hwnd,
                }));
                self.send_pending_action(
                    PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                        window_handle: self.source_hwnd,
                        visible: true,
                    }),
                    id,
                    cx,
                );
            }
            Route::Download(url) => {
                let downloads = self.downloads.clone();
                let owner = cx.entity().downgrade();
                cx.defer(move |cx| {
                    let Some(current) = owner.upgrade() else {
                        return;
                    };
                    if !current.read(cx).action_is_current(id) {
                        return;
                    }
                    let success = matches!(
                        downloads.update(cx, |downloads, cx| downloads
                            .open_floating_download(url, cx)),
                        Ok(Ok(()))
                    );
                    let _ = owner.update(cx, |owner, cx| {
                        owner.finish_routed_action(id, success);
                        if !success {
                            owner.notice(
                                "无法打开添加下载窗口，请先完成现有下载确认后重试。",
                                true,
                                cx,
                            );
                        }
                    });
                });
            }
            Route::Search(text) => {
                let quick = self.quick_launch.clone();
                let owner = cx.entity().downgrade();
                let request = QuickLaunchPanelRequest {
                    target: self.foreground.clone(),
                    directory_navigation_context: None,
                    cursor_x: event.x,
                    cursor_y: event.y,
                    cursor_display_index: 0,
                    cursor_monitor_dpi: 96,
                    requested_at_unix_ms: nxtools_ipc::unix_time_ms(),
                    correlation_id: Some(id),
                };
                cx.defer(move |cx| {
                    let Some(current) = owner.upgrade() else {
                        return;
                    };
                    if !current.read(cx).action_is_current(id) {
                        return;
                    }
                    let success = quick
                        .update(cx, |quick, cx| {
                            quick.show_floating_search(text, request, cx)
                        })
                        .is_ok();
                    let _ = owner.update(cx, |owner, cx| {
                        owner.finish_routed_action(id, success);
                        if !success {
                            owner.notice("速启搜索当前不可用，请重试。", true, cx);
                        }
                    });
                });
            }
            Route::CopyImage(path, format) => {
                cx.spawn(async move |this, cx| {
                    let result = cx
                        .background_executor()
                        .spawn(async move { prepare_image(&path, format.as_deref()) })
                        .await;
                    let _ = this.update(cx, |owner, cx| {
                        if !owner.action_is_current(id) {
                            return;
                        }
                        match result {
                            Ok(image) => {
                                let command = PlatformCommand::PasteImage(PasteImageCommand {
                                    image_path: image.path.to_string_lossy().into_owned(),
                                    include_file_drop_fallback: false,
                                    paste_after_write: false,
                                    target_window_handle: 0,
                                    target_focus_window_handle: 0,
                                });
                                owner
                                    .pending_actions
                                    .get_mut(&id)
                                    .expect("current action")
                                    .image = Some(image);
                                owner.send_pending_action(command, id, cx);
                            }
                            Err(()) => {
                                owner.finish_routed_action(id, false);
                                owner.notice(
                                    "图片无法读取、已损坏或超过资源限制，未写入剪切板。",
                                    true,
                                    cx,
                                );
                            }
                        }
                    });
                })
                .detach();
            }
            Route::None | Route::Unsupported => unreachable!("handled before admission"),
        }
    }
    fn action_is_current(&self, id: Uuid) -> bool {
        self.settings().enabled
            && !self.settings().click_through
            && self
                .pending_actions
                .get(&id)
                .is_some_and(|pending| pending.session == self.session && !pending.cancelled)
    }
    fn finish_routed_action(&mut self, id: Uuid, success: bool) {
        if let Some(pending) = self.pending_actions.remove(&id) {
            self.release_drop(pending.drop_id);
            tracing::info!(event="floating_assistant.action.terminal",correlation_id=%pending.correlation,success,elapsed_ms=pending.started.elapsed().as_millis(),"floating routed action terminated");
        }
    }
    /// Preparation/deferred work is cancellable. An admitted Windows copy still owns
    /// its normalized image until its real reply/deadline; cancellation cannot undo it.
    pub(super) fn cancel_actions(&mut self, retiring: bool) {
        let mut releases = Vec::new();
        self.pending_actions.retain(|_, pending| {
            pending.cancelled = true;
            pending.follow_up = None;
            if !pending.dispatched || retiring {
                releases.push(pending.drop_id.take());
            }
            if !pending.dispatched {
                tracing::info!(event="floating_assistant.action.terminal",correlation_id=%pending.correlation,success=false,reason="cancelled",elapsed_ms=pending.started.elapsed().as_millis(),"floating action cancelled before dispatch");
            }
            pending.dispatched
        });
        for drop_id in releases {
            self.release_drop(drop_id);
        }
    }
    fn send_pending_action(&mut self, command: PlatformCommand, id: Uuid, cx: &mut Context<Self>) {
        let pending = self
            .pending_actions
            .get_mut(&id)
            .expect("registered native action");
        pending.dispatched = true;
        let correlation = pending.correlation;
        if self
            .platform
            .send_command_with_id_correlated(command, id, correlation)
            .is_err()
        {
            self.finish_routed_action(id, false);
            self.notice("Windows 平台未连接，操作未执行。", true, cx);
        } else {
            self.action_deadline(id, cx);
        }
    }
    fn action_deadline(&mut self, id: Uuid, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(15)).await;
            let _ = this.update(cx, |owner, cx| {
                if let Some(pending) = owner.pending_actions.remove(&id) {
                    owner.release_drop(pending.drop_id);
                    tracing::warn!(event="floating_assistant.action.terminal",correlation_id=%pending.correlation,success=false,reason="timeout",elapsed_ms=pending.started.elapsed().as_millis(),"floating action timed out");
                    if !pending.cancelled { owner.notice("悬浮窗操作超时，请重试。", true, cx); }
                }
            });
        }).detach();
    }
    pub(super) fn complete_action(
        &mut self,
        id: Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(mut pending) = self.pending_actions.remove(&id) else {
            return false;
        };
        if result.success
            && !pending.cancelled
            && pending.session == self.session
            && self.settings().enabled
            && let Some(command) = pending.follow_up.take()
        {
            let next = Uuid::new_v4();
            self.pending_actions.insert(next, pending);
            self.send_pending_action(command, next, cx);
            return true;
        }
        self.release_drop(pending.drop_id);
        tracing::info!(event="floating_assistant.action.terminal",correlation_id=%pending.correlation,success=result.success,cancelled=pending.cancelled,native_error_code=?result.native_error_code,elapsed_ms=pending.started.elapsed().as_millis(),"floating action terminated");
        if !pending.cancelled && pending.session == self.session && self.settings().enabled {
            if !result.success {
                self.notice("Windows 未完成悬浮窗操作，请重试。", true, cx);
            } else if pending.image.is_some() {
                self.notice("图片已写入剪切板。", false, cx);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event() -> FloatingAssistantEvent {
        FloatingAssistantEvent {
            version: 1,
            session_id: Uuid::new_v4(),
            generation: 1,
            window_handle: 1,
            correlation_id: Uuid::new_v4(),
            sequence: 1,
            kind: FloatingAssistantEventKind::Drop,
            x: 0,
            y: 0,
            delta: 0,
            fullscreen: false,
            text: None,
            url: None,
            html: None,
            image_path: None,
            image_format: None,
            files: vec![],
            drop_id: Some(Uuid::new_v4()),
        }
    }
    #[test]
    fn automatic_text_is_download_or_whole_search_never_both() {
        let settings = FloatingAssistantSettings::default();
        let mut value = event();
        value.text = Some("查找 中文".into());
        assert_eq!(route(&settings, &value), Route::Search("查找 中文".into()));
        value.text = Some("链接 https://example.test/file.zip".into());
        assert_eq!(
            route(&settings, &value),
            Route::Download(Some("https://example.test/file.zip".into()))
        );
    }
    #[test]
    fn image_metadata_wins_and_local_image_is_copy_not_fake_download() {
        let mut settings = FloatingAssistantSettings::default();
        let mut value = event();
        value.html =
            Some("<a href='https://other.test/'><img src='https://image.test/a.png'></a>".into());
        value.image_path = Some("staged.png".into());
        value.image_format = Some("png".into());
        assert_eq!(
            route(&settings, &value),
            Route::Download(Some("https://image.test/a.png".into()))
        );
        settings.image_drop_action = ImageDropAction::WriteClipboard;
        assert_eq!(
            route(&settings, &value),
            Route::CopyImage("staged.png".into(), Some("png".into()))
        );
        value.html = None;
        value.image_path = None;
        value.image_format = None;
        value.files = vec!["C:/image.gif".into()];
        settings.image_drop_action = ImageDropAction::Automatic;
        assert_eq!(
            route(&settings, &value),
            Route::CopyImage("C:/image.gif".into(), None)
        );
        settings.image_drop_action = ImageDropAction::AddDownload;
        assert_eq!(route(&settings, &value), Route::Unsupported);
    }
    #[test]
    fn every_explicit_none_and_click_choice_is_single_route() {
        let mut settings = FloatingAssistantSettings::default();
        let mut value = event();
        value.text = Some("hello".into());
        settings.text_drop_action = TextDropAction::None;
        assert_eq!(route(&settings, &value), Route::None);
        settings.text_drop_action = TextDropAction::AddDownload;
        assert_eq!(route(&settings, &value), Route::Unsupported);
        for (action, expected) in [
            (ClickAction::None, Route::None),
            (ClickAction::OpenMain, Route::Main),
            (ClickAction::AddDownload, Route::Download(None)),
        ] {
            settings.left_click_action = action;
            settings.right_click_action = action;
            value.kind = FloatingAssistantEventKind::LeftClick;
            assert_eq!(route(&settings, &value), expected);
            value.kind = FloatingAssistantEventKind::RightClick;
            assert_eq!(route(&settings, &value), expected);
        }
    }
    #[test]
    fn image_url_without_extension_uses_remote_source_and_none_remains_inert() {
        let mut settings = FloatingAssistantSettings::default();
        let mut value = event();
        value.url = Some("https://image.test/render?id=7".into());
        value.image_path = Some("staged.png".into());
        value.image_format = Some("png".into());
        assert_eq!(
            route(&settings, &value),
            Route::Download(Some("https://image.test/render?id=7".into()))
        );
        settings.image_drop_action = ImageDropAction::None;
        assert_eq!(route(&settings, &value), Route::None);
    }
    #[test]
    fn html_image_source_ignores_data_src_and_srcset_before_real_src() {
        assert_eq!(
            html_image_url(
                "<img data-src='https://wrong.test/a' srcset='https://wrong.test/b 2x' src='https://right.test/render?a=1&amp;b=2'>"
            ),
            Some("https://right.test/render?a=1&b=2".into())
        );
    }
}
