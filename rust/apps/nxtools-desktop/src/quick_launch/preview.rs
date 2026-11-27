use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuickLaunchPreviewKind {
    Image,
    Text,
    Metadata,
}

#[derive(Clone)]
pub(crate) enum QuickLaunchPreviewBody {
    Image(Arc<Image>),
    Text { text: String, truncated: bool },
    Metadata { icon: MaterialIcon },
}

#[derive(Clone)]
pub(crate) struct QuickLaunchPreviewPayload {
    pub(crate) entry_id: String,
    pub(crate) name: String,
    pub(crate) target: String,
    pub(crate) detail: String,
    pub(crate) created_at: Option<String>,
    pub(crate) modified_at: Option<String>,
    pub(crate) body: QuickLaunchPreviewBody,
}

pub(crate) fn quick_launch_preview_image_asset(
    payload: &QuickLaunchPreviewPayload,
) -> Option<Arc<Image>> {
    match &payload.body {
        QuickLaunchPreviewBody::Image(image) => Some(Arc::clone(image)),
        QuickLaunchPreviewBody::Text { .. } | QuickLaunchPreviewBody::Metadata { .. } => None,
    }
}

pub(crate) fn quick_launch_preview_height(_payload: &QuickLaunchPreviewPayload) -> f32 {
    QUICK_LAUNCH_PREVIEW_HEIGHT
}

pub(crate) fn quick_launch_preview_kind(path: &Path) -> Option<QuickLaunchPreviewKind> {
    if !path.exists() {
        return None;
    }
    if path.is_dir() {
        return Some(QuickLaunchPreviewKind::Metadata);
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "bmp") {
        return Some(QuickLaunchPreviewKind::Image);
    }
    if matches!(
        extension.as_str(),
        "txt"
            | "md"
            | "markdown"
            | "json"
            | "jsonc"
            | "xml"
            | "yaml"
            | "yml"
            | "toml"
            | "ini"
            | "cfg"
            | "conf"
            | "log"
            | "csv"
            | "rs"
            | "cs"
            | "c"
            | "h"
            | "cpp"
            | "hpp"
            | "py"
            | "js"
            | "jsx"
            | "ts"
            | "tsx"
            | "html"
            | "htm"
            | "css"
            | "scss"
            | "sql"
            | "ps1"
            | "bat"
            | "cmd"
            | "sh"
    ) {
        return Some(QuickLaunchPreviewKind::Text);
    }
    Some(QuickLaunchPreviewKind::Metadata)
}

pub(crate) fn load_quick_launch_preview(
    result: &SearchResult,
) -> Option<QuickLaunchPreviewPayload> {
    let path = PathBuf::from(result.entry.target.trim());
    let metadata = match std::fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(_) => {
            return Some(QuickLaunchPreviewPayload {
                entry_id: result.entry.id.clone(),
                name: result.entry.name.clone(),
                target: result.entry.target.clone(),
                detail: "目标当前不可访问，仅显示索引信息".into(),
                created_at: None,
                modified_at: None,
                body: QuickLaunchPreviewBody::Metadata {
                    icon: if result.entry.source == ApplicationSource::IndexedDirectory {
                        MaterialIcon::Folder
                    } else {
                        MaterialIcon::Info
                    },
                },
            });
        }
    };
    let kind = quick_launch_preview_kind(&path).unwrap_or(QuickLaunchPreviewKind::Metadata);
    let created_at = metadata.created().ok().map(format_preview_timestamp);
    let modified_at = metadata.modified().ok().map(format_preview_timestamp);
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_uppercase)
        .filter(|extension| !extension.is_empty());
    let mut detail = if metadata.is_dir() {
        "文件夹".to_owned()
    } else {
        let kind_label = extension
            .map(|extension| format!("{extension} 文件"))
            .unwrap_or_else(|| "文件".into());
        format!("{kind_label}  ·  {}", format_preview_bytes(metadata.len()))
    };
    let body = match kind {
        QuickLaunchPreviewKind::Image => {
            if metadata.len() > QUICK_LAUNCH_IMAGE_FILE_LIMIT {
                detail.push_str("  ·  图片过大，仅显示信息");
                QuickLaunchPreviewBody::Metadata {
                    icon: MaterialIcon::Image,
                }
            } else {
                let mut bytes = Vec::with_capacity(
                    metadata
                        .len()
                        .min(1024 * 1024)
                        .try_into()
                        .unwrap_or_default(),
                );
                let read = File::open(&path).and_then(|file| {
                    file.take(QUICK_LAUNCH_IMAGE_FILE_LIMIT + 1)
                        .read_to_end(&mut bytes)
                });
                let format = quick_launch_image_format(&path);
                match (read, format, imagesize::blob_size(&bytes)) {
                    (Ok(_), Some(format), Ok(size))
                        if quick_launch_image_within_budget(
                            bytes.len() as u64,
                            size.width,
                            size.height,
                        ) =>
                    {
                        detail.push_str(&format!("  ·  {} × {}", size.width, size.height));
                        QuickLaunchPreviewBody::Image(Arc::new(Image::from_bytes(format, bytes)))
                    }
                    (Ok(_), Some(_), Ok(_))
                        if bytes.len() as u64 > QUICK_LAUNCH_IMAGE_FILE_LIMIT =>
                    {
                        detail.push_str("  ·  图片过大，仅显示信息");
                        QuickLaunchPreviewBody::Metadata {
                            icon: MaterialIcon::Image,
                        }
                    }
                    (Ok(_), Some(_), Ok(_)) => {
                        detail.push_str("  ·  图片尺寸过大，仅显示信息");
                        QuickLaunchPreviewBody::Metadata {
                            icon: MaterialIcon::Image,
                        }
                    }
                    _ => {
                        detail.push_str("  ·  无法读取图片内容");
                        QuickLaunchPreviewBody::Metadata {
                            icon: MaterialIcon::Image,
                        }
                    }
                }
            }
        }
        QuickLaunchPreviewKind::Text => {
            let mut bytes = Vec::with_capacity(QUICK_LAUNCH_TEXT_PREVIEW_LIMIT + 1);
            let read = File::open(&path).and_then(|mut file| {
                file.by_ref()
                    .take((QUICK_LAUNCH_TEXT_PREVIEW_LIMIT + 1) as u64)
                    .read_to_end(&mut bytes)
            });
            match read {
                Ok(_) => {
                    let truncated = metadata.len() > QUICK_LAUNCH_TEXT_PREVIEW_LIMIT as u64
                        || bytes.len() > QUICK_LAUNCH_TEXT_PREVIEW_LIMIT;
                    bytes.truncate(QUICK_LAUNCH_TEXT_PREVIEW_LIMIT);
                    if let Some(text) = decode_preview_text(&bytes) {
                        QuickLaunchPreviewBody::Text { text, truncated }
                    } else {
                        QuickLaunchPreviewBody::Metadata {
                            icon: MaterialIcon::Info,
                        }
                    }
                }
                Err(_) => {
                    detail.push_str("  ·  内容当前不可读取");
                    QuickLaunchPreviewBody::Metadata {
                        icon: MaterialIcon::Info,
                    }
                }
            }
        }
        QuickLaunchPreviewKind::Metadata => QuickLaunchPreviewBody::Metadata {
            icon: if metadata.is_dir() {
                MaterialIcon::Folder
            } else {
                MaterialIcon::Info
            },
        },
    };
    Some(QuickLaunchPreviewPayload {
        entry_id: result.entry.id.clone(),
        name: result.entry.name.clone(),
        target: result.entry.target.clone(),
        detail,
        created_at,
        modified_at,
        body,
    })
}

pub(crate) fn format_preview_timestamp(timestamp: SystemTime) -> String {
    DateTime::<Local>::from(timestamp)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

pub(crate) fn quick_launch_image_format(path: &Path) -> Option<ImageFormat> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "bmp" => Some(ImageFormat::Bmp),
        _ => None,
    }
}

pub(crate) fn quick_launch_image_within_budget(file_len: u64, width: usize, height: usize) -> bool {
    if file_len > QUICK_LAUNCH_IMAGE_FILE_LIMIT || width == 0 || height == 0 {
        return false;
    }
    let Ok(width) = u64::try_from(width) else {
        return false;
    };
    let Ok(height) = u64::try_from(height) else {
        return false;
    };
    width
        .checked_mul(height)
        .is_some_and(|pixels| pixels <= QUICK_LAUNCH_IMAGE_PIXEL_LIMIT)
}

pub(crate) fn decode_preview_text(bytes: &[u8]) -> Option<String> {
    let mut text = if bytes.starts_with(&[0xff, 0xfe]) {
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&units)
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|unit| u16::from_be_bytes([unit[0], unit[1]]))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&units)
    } else {
        if bytes.contains(&0) {
            return None;
        }
        String::from_utf8_lossy(bytes).into_owned()
    };
    while text.len() > QUICK_LAUNCH_TEXT_PREVIEW_LIMIT {
        text.pop();
    }
    Some(text)
}

pub(crate) fn format_preview_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

pub(crate) fn quick_launch_preview_bounds(
    panel_bounds: Bounds<Pixels>,
    preview_size: Size<Pixels>,
    screen: Bounds<Pixels>,
) -> Bounds<Pixels> {
    let gap = px(QUICK_LAUNCH_PREVIEW_GAP);
    let right = panel_bounds.right() + gap;
    let x = if right + preview_size.width <= screen.right() {
        right
    } else {
        (panel_bounds.origin.x - gap - preview_size.width).max(screen.origin.x)
    };
    let max_x = (screen.right() - preview_size.width).max(screen.origin.x);
    let max_y = (screen.bottom() - preview_size.height).max(screen.origin.y);
    Bounds::new(
        point(
            x.clamp(screen.origin.x, max_x),
            panel_bounds.origin.y.clamp(screen.origin.y, max_y),
        ),
        preview_size,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuickLaunchPreviewOpenAction {
    Create,
    Reuse,
    Ignore,
}

pub(crate) fn quick_launch_preview_open_action(
    window_exists: bool,
    visible: bool,
    displayed_entry_id: Option<&str>,
    requested_entry_id: &str,
) -> QuickLaunchPreviewOpenAction {
    if !window_exists {
        QuickLaunchPreviewOpenAction::Create
    } else if visible && displayed_entry_id == Some(requested_entry_id) {
        QuickLaunchPreviewOpenAction::Ignore
    } else {
        QuickLaunchPreviewOpenAction::Reuse
    }
}

pub(crate) fn quick_launch_preview_target_changed(
    visible: bool,
    displayed_entry_id: Option<&str>,
    requested_entry_id: &str,
) -> bool {
    visible && displayed_entry_id.is_some() && displayed_entry_id != Some(requested_entry_id)
}

#[derive(Default)]
pub(crate) struct QuickLaunchPreviewSession {
    pub(crate) window: Option<QuickLaunchPreviewWindow>,
    pub(crate) visible: bool,
    pub(crate) displayed_entry_id: Option<String>,
    pub(crate) generation: u64,
    pub(crate) hovered_entry_id: Option<String>,
    pub(crate) pointer_inside: bool,
    pub(crate) pending_stage: Option<QuickLaunchPreviewPendingStage>,
    pub(crate) image_asset: Option<Arc<Image>>,
    pub(crate) exit_generation: u64,
    pub(crate) material: crate::ui::material::Preparation,
    pub(crate) material_wait: Option<super::preview_presentation::PreviewMaterialWait>,
    pub(crate) material_lifecycle: Option<(uuid::Uuid, Instant)>,
    pub(crate) material_subscriptions: Vec<Subscription>,
    pub(crate) no_activate_ready: bool,
    pub(crate) restore_primary_focus: bool,
}

#[derive(Clone)]
pub(crate) struct QuickLaunchPreviewCandidate {
    pub(crate) result: SearchResult,
    pub(crate) panel_bounds: Bounds<Pixels>,
    pub(crate) panel_scale_factor: f32,
}

#[derive(Clone)]
pub(crate) struct QuickLaunchPreviewWindow {
    pub(crate) identity: QuickLaunchPreviewWindowIdentity,
    pub(crate) view: WeakEntity<QuickLaunchPreview>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct QuickLaunchPreviewWindowIdentity {
    pub(crate) gpui: AnyWindowHandle,
    pub(crate) native: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuickLaunchPreviewStagePhase {
    InitialNoActivate,
    InitialBounds,
    InitialShow,
    MoveOffscreen,
    ShowOffscreen,
    RevealOnscreen,
}

pub(crate) struct QuickLaunchPreviewPendingStage {
    pub(crate) command_id: uuid::Uuid,
    pub(crate) phase: QuickLaunchPreviewStagePhase,
    pub(crate) window_identity: QuickLaunchPreviewWindowIdentity,
    pub(crate) generation: u64,
    pub(crate) entry_id: String,
    pub(crate) payload: Option<QuickLaunchPreviewPayload>,
    pub(crate) preview_size: Size<Pixels>,
    pub(crate) bounds: Bounds<Pixels>,
    pub(crate) panel_scale_factor: f32,
}

pub(crate) struct QuickLaunchPreview {
    pub(crate) payload: QuickLaunchPreviewPayload,
    pub(crate) application: WeakEntity<QuickLaunchApplication>,
    pub(crate) text_scroll: ScrollHandle,
    pub(crate) appearance_generation: u64,
    pub(crate) is_exiting: bool,
    pub(crate) motion_suppressed: bool,
}

impl QuickLaunchPreview {
    pub(crate) fn new(
        payload: QuickLaunchPreviewPayload,
        application: WeakEntity<QuickLaunchApplication>,
    ) -> Self {
        Self {
            payload,
            application,
            text_scroll: ScrollHandle::new(),
            appearance_generation: 1,
            is_exiting: false,
            motion_suppressed: false,
        }
    }

    pub(crate) fn set_payload(
        &mut self,
        payload: QuickLaunchPreviewPayload,
        cx: &mut Context<Self>,
    ) {
        self.release_image_asset(cx);
        self.payload = payload;
        self.text_scroll.set_offset(point(px(0.0), px(0.0)));
        self.begin_enter(cx);
    }

    pub(crate) fn begin_enter(&mut self, cx: &mut Context<Self>) {
        self.is_exiting = false;
        self.motion_suppressed = false;
        self.appearance_generation = self.appearance_generation.wrapping_add(1).max(1);
        cx.notify();
    }

    pub(crate) fn cancel_exit(&mut self, cx: &mut Context<Self>) {
        if self.is_exiting {
            self.is_exiting = false;
            self.motion_suppressed = true;
            cx.notify();
        }
    }

    pub(crate) fn begin_exit(&mut self, cx: &mut Context<Self>) {
        if !self.is_exiting {
            self.is_exiting = true;
            self.motion_suppressed = false;
            self.appearance_generation = self.appearance_generation.wrapping_add(1).max(1);
            cx.notify();
        }
    }

    pub(crate) fn clear_image_asset(&mut self, cx: &mut Context<Self>) {
        if matches!(self.payload.body, QuickLaunchPreviewBody::Image(_)) {
            self.release_image_asset(cx);
            self.payload.body = QuickLaunchPreviewBody::Metadata {
                icon: MaterialIcon::Image,
            };
            cx.notify();
        }
    }

    pub(crate) fn release_image_asset(&self, cx: &mut Context<Self>) {
        if let QuickLaunchPreviewBody::Image(image) = &self.payload.body {
            ImageSource::from(Arc::clone(image)).remove_asset(cx);
        }
    }
}

impl Render for QuickLaunchPreview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::ui::motion::observe_motion(window, cx);
        crate::theme::observe_theme(window, cx);
        let theme = Theme::active(cx);
        let application = self.application.clone();
        let appearance_generation = self.appearance_generation;
        let is_exiting = self.is_exiting;
        let motion_suppressed = self.motion_suppressed || crate::ui::motion::reduced_motion(cx);
        let has_timestamps =
            self.payload.created_at.is_some() || self.payload.modified_at.is_some();
        let body = match &self.payload.body {
            QuickLaunchPreviewBody::Image(image) => preview_media_surface(theme)
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    img(Arc::clone(image))
                        .size_full()
                        .object_fit(ObjectFit::Contain),
                )
                .into_any_element(),
            QuickLaunchPreviewBody::Text { text, truncated } => preview_media_surface(theme)
                .id("quick-launch-preview-text")
                .size_full()
                .track_scroll(&self.text_scroll)
                .overflow_y_scroll()
                .vertical_scrollbar(&self.text_scroll)
                .p(px(SPACING_12))
                .min_w_0()
                .child(
                    div()
                        .debug_selector(|| "quick-launch-preview-text-copy".into())
                        .w_full()
                        .min_w_0()
                        .child(crate::ui::markdown_document::cached_view(
                            "quick-launch-preview-document".into(),
                            text,
                            if Path::new(&self.payload.target)
                                .extension()
                                .and_then(|extension| extension.to_str())
                                .is_some_and(|extension| {
                                    extension.eq_ignore_ascii_case("md")
                                        || extension.eq_ignore_ascii_case("markdown")
                                })
                            {
                                crate::ui::markdown_document::DocumentMode::Markdown
                            } else {
                                match crate::ui::markdown_document::automatic_mode(text) {
                                    crate::ui::markdown_document::DocumentMode::Literal(
                                        language,
                                    ) => crate::ui::markdown_document::DocumentMode::Literal(
                                        language,
                                    ),
                                    _ => crate::ui::markdown_document::DocumentMode::Literal(None),
                                }
                            },
                            window,
                            cx,
                        )),
                )
                .when(*truncated, |content| {
                    content.child(
                        div()
                            .mt(px(SPACING_8))
                            .pt(px(SPACING_8))
                            .border_t_1()
                            .border_color(theme.border_subtle)
                            .text_size(px(TEXT_LABEL))
                            .line_height(px(LINE_HEIGHT_LABEL))
                            .text_color(theme.text_muted)
                            .child("仅显示文件开头的 96 KB"),
                    )
                })
                .into_any_element(),
            QuickLaunchPreviewBody::Metadata { icon } => preview_media_surface(theme)
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(empty_state(
                    theme,
                    "quick-launch-preview-unavailable",
                    *icon,
                    "没有轻量预览",
                    "仍可直接打开、定位或执行操作。",
                ))
                .into_any_element(),
        };

        let preview = preview_shell(theme)
            .id("quick-launch-preview-root")
            .w(px(QUICK_LAUNCH_PREVIEW_WIDTH))
            .h(px(QUICK_LAUNCH_PREVIEW_HEIGHT))
            .on_hover(move |hovered, _window, cx| {
                let inside = *hovered;
                schedule_application_update(
                    application.clone(),
                    move |application, cx| {
                        application.set_quick_launch_preview_pointer_inside(inside, cx)
                    },
                    cx,
                );
            })
            .child(
                div()
                    .debug_selector(|| "quick-launch-preview-header".into())
                    .h(px(QUICK_LAUNCH_PREVIEW_HEADER_HEIGHT))
                    .flex_none()
                    .px(px(SPACING_12))
                    .border_b_1()
                    .border_color(theme.border_subtle)
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap(px(SPACING_4))
                    .child(
                        div()
                            .debug_selector(|| "quick-launch-preview-title".into())
                            .w_full()
                            .min_w_0()
                            .h(px(LINE_HEIGHT_BODY))
                            .text_size(px(TEXT_BODY))
                            .line_height(px(LINE_HEIGHT_BODY))
                            .text_color(theme.text)
                            .truncate()
                            .child(self.payload.name.clone()),
                    )
                    .child(
                        div()
                            .debug_selector(|| "quick-launch-preview-detail".into())
                            .w_full()
                            .min_w_0()
                            .h(px(LINE_HEIGHT_BODY))
                            .text_size(px(TEXT_BODY))
                            .line_height(px(LINE_HEIGHT_BODY))
                            .text_color(theme.text_secondary)
                            .truncate()
                            .child(self.payload.detail.clone()),
                    )
                    .when(has_timestamps, |header| {
                        header.child(
                            div()
                                .debug_selector(|| "quick-launch-preview-timestamps".into())
                                .w_full()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(SPACING_12))
                                .text_size(px(TEXT_LABEL))
                                .line_height(px(LINE_HEIGHT_LABEL))
                                .text_color(theme.text_muted)
                                .when_some(self.payload.created_at.clone(), |timestamps, value| {
                                    timestamps.child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .truncate()
                                            .child(format!("创建 {value}")),
                                    )
                                })
                                .when_some(
                                    self.payload.modified_at.clone(),
                                    |timestamps, value| {
                                        timestamps.child(
                                            div()
                                                .min_w_0()
                                                .flex_1()
                                                .truncate()
                                                .child(format!("修改 {value}")),
                                        )
                                    },
                                ),
                        )
                    }),
            )
            .child(
                div()
                    .debug_selector(|| "quick-launch-preview-content".into())
                    .min_h_0()
                    .flex_1()
                    .overflow_hidden()
                    .p(px(SPACING_8))
                    .child(body),
            )
            .child(
                div()
                    .debug_selector(|| "quick-launch-preview-footer".into())
                    .h(px(QUICK_LAUNCH_PREVIEW_FOOTER_HEIGHT))
                    .flex_none()
                    .px(px(SPACING_12))
                    .border_t_1()
                    .border_color(theme.border_subtle)
                    .min_w_0()
                    .flex()
                    .items_center()
                    .text_size(px(TEXT_BODY))
                    .line_height(px(LINE_HEIGHT_BODY))
                    .text_color(theme.text_muted)
                    .truncate()
                    .child(self.payload.target.clone()),
            );
        if motion_suppressed {
            preview.into_any_element()
        } else {
            preview
                .with_animation(
                    ("quick-launch-preview-motion", appearance_generation),
                    Animation::new(if is_exiting {
                        QUICK_LAUNCH_PREVIEW_DISAPPEAR_DURATION
                    } else {
                        PREVIEW_APPEAR_DURATION
                    })
                    .with_easing(ease_out_quint()),
                    move |preview, delta| {
                        if is_exiting {
                            preview
                                .relative()
                                .top(px(-2.0 * delta))
                                .opacity(1.0 - 0.1 * delta)
                        } else {
                            preview
                                .relative()
                                .top(px(3.0 * (1.0 - delta)))
                                .opacity(0.9 + 0.1 * delta)
                        }
                    },
                )
                .into_any_element()
        }
    }
}
