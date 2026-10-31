//! Local presentation on the original browser; EntrySession remains materialization/lease owner.
use super::*;
use crate::archive::preview::{PreviewPayload, PreviewResult};
use crate::ui::markdown_document::{self, DocumentMode, MarkdownDocument};

#[derive(Clone, PartialEq, Eq)]
enum Selection {
    Empty,
    Single(RowId),
    Multiple,
}
#[derive(Clone, PartialEq, Eq)]
struct Fingerprint {
    generation: u64,
    snapshot: Option<Uuid>,
    epoch: u64,
    ready: bool,
    selection: Selection,
}
pub(super) struct PreviewPane {
    pub generation: u64,
    fingerprint: Option<Fingerprint>,
    dispatch: bool,
    result: PreviewResult,
    document: Option<Entity<MarkdownDocument>>,
    source: bool,
    scroll: ScrollHandle,
}
impl Default for PreviewPane {
    fn default() -> Self {
        Self {
            generation: 0,
            fingerprint: None,
            dispatch: false,
            result: PreviewResult::Idle,
            document: None,
            source: false,
            scroll: ScrollHandle::new(),
        }
    }
}
impl ArchiveBrowser {
    pub(super) fn preview_visible(&self) -> bool {
        let fingerprint = self.preview_fingerprint();
        let Selection::Single(RowId::Entry(snapshot, ordinal)) = fingerprint.selection else {
            return false;
        };
        fingerprint.ready
            && self.model.as_ref().is_some_and(|model| {
                model.snapshot == snapshot
                    && model.entries.iter().any(|entry| {
                        entry.ordinal == ordinal
                            && entry.kind == "File"
                            && crate::archive::preview::kind(&entry.name).is_some()
                    })
            })
    }

    fn preview_fingerprint(&self) -> Fingerprint {
        let selection =
            self.model
                .as_ref()
                .map_or(Selection::Empty, |model| match model.selected.len() {
                    0 => Selection::Empty,
                    1 => Selection::Single(
                        model
                            .selected
                            .iter()
                            .next()
                            .expect("single selection")
                            .clone(),
                    ),
                    _ => Selection::Multiple,
                });
        Fingerprint {
            generation: self.generation,
            snapshot: self.model.as_ref().map(|model| model.snapshot),
            epoch: self.projection_epoch,
            ready: !self.retired
                && !self.open_pending
                && matches!(self.status, BrowserStatus::Ready),
            selection,
        }
    }
    /// Called from original selection callbacks before any next-turn publication can observe ABA.
    pub(super) fn observe_preview_selection(&mut self) {
        let fingerprint = self.preview_fingerprint();
        if self.preview.fingerprint.as_ref() == Some(&fingerprint) {
            return;
        }
        self.preview.fingerprint = Some(fingerprint.clone());
        self.preview.generation = self.preview.generation.wrapping_add(1).max(1);
        self.preview.document = None;
        self.preview.source = false;
        self.preview
            .scroll
            .set_offset(gpui_kit::point(px(0.), px(0.)));
        self.preview.result = if !fingerprint.ready {
            PreviewResult::Cancelled
        } else {
            match fingerprint.selection {
                Selection::Empty => PreviewResult::Idle,
                Selection::Multiple => PreviewResult::MultipleSelection,
                Selection::Single(_) => PreviewResult::Loading,
            }
        };
        self.preview.dispatch = true;
    }
    pub(crate) fn preview_selection_matches(&self, generation: u64, row: &Option<RowId>) -> bool {
        !self.retired
            && self.preview.generation == generation
            && self
                .preview
                .fingerprint
                .as_ref()
                .is_some_and(|fingerprint| {
                    fingerprint.ready
                        && *fingerprint == self.preview_fingerprint()
                        && match (&fingerprint.selection, row) {
                            (Selection::Single(current), Some(row)) => current == row,
                            (Selection::Empty | Selection::Multiple, None) => true,
                            _ => false,
                        }
                })
    }
    pub(super) fn sync_preview_selection(&mut self, cx: &mut Context<Self>) {
        self.observe_preview_selection();
        if !std::mem::take(&mut self.preview.dispatch) {
            return;
        }
        let generation = self.preview.generation;
        let fingerprint = self
            .preview
            .fingerprint
            .clone()
            .expect("observed selection");
        let row = match &fingerprint.selection {
            Selection::Single(row) => Some(row.clone()),
            _ => None,
        };
        let owner = self.owner.clone();
        let view = cx.entity().downgrade();
        let id = self.window_id;
        cx.defer(move |cx| {
            let current = view
                .read_with(cx, |view, _| {
                    !view.retired
                        && view.preview.generation == generation
                        && view.preview_fingerprint() == fingerprint
                })
                .unwrap_or(false);
            if !current {
                return;
            }
            let _ = owner.update(cx, |app, cx| {
                if fingerprint.ready {
                    app.preview_entry(id, row, generation, cx);
                } else {
                    app.cancel_preview(id, cx);
                }
            });
        });
    }
    pub(super) fn suspend_preview(&mut self, cx: &mut Context<Self>) {
        self.preview.generation = self.preview.generation.wrapping_add(1).max(1);
        self.preview.fingerprint = None;
        self.preview.document = None;
        self.preview.result = PreviewResult::Cancelled;
        self.preview.dispatch = false;
        let owner = self.owner.clone();
        let id = self.window_id;
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| app.cancel_preview(id, cx));
        });
    }
    pub(crate) fn preview_result(
        &mut self,
        generation: u64,
        row: Option<RowId>,
        result: PreviewResult,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.preview_selection_matches(generation, &row) {
            return;
        }
        self.preview.document = match &result {
            PreviewResult::Ready(PreviewPayload::Text { text, mode }) => {
                markdown_document::document(text, *mode, window, cx)
            }
            _ => None,
        };
        self.preview.result =
            if matches!(&result, PreviewResult::Ready(PreviewPayload::Text { .. }))
                && self.preview.document.is_none()
            {
                PreviewResult::DocumentLimit
            } else {
                result
            };
        cx.notify();
    }
    fn preview_source(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let PreviewResult::Ready(PreviewPayload::Text { text, mode }) = &self.preview.result else {
            return;
        };
        self.preview.source = !self.preview.source;
        self.preview.document = markdown_document::document(
            text,
            if self.preview.source {
                DocumentMode::Literal(None)
            } else {
                *mode
            },
            window,
            cx,
        );
        self.preview
            .scroll
            .set_offset(gpui_kit::point(px(0.), px(0.)));
        cx.notify();
    }
    fn retry_preview(&mut self, cx: &mut Context<Self>) {
        self.preview.fingerprint = None;
        self.sync_preview_selection(cx);
        cx.notify();
    }
    pub(super) fn render_preview(&self, cx: &mut Context<Self>) -> AnyElement {
        let body = match &self.preview.result {
            PreviewResult::Ready(PreviewPayload::Image {
                image,
                width,
                height,
            }) => div()
                .size_full()
                .flex()
                .flex_col()
                .min_h_0()
                .child(
                    div().min_h_0().flex_1().w_full().child(
                        div()
                            .debug_selector(|| "archive-preview-image".into())
                            .size_full()
                            .child(
                                gpui_kit::img(image.clone())
                                    .size_full()
                                    .object_fit(gpui_kit::ObjectFit::Contain),
                            ),
                    ),
                )
                .child(
                    div()
                        .flex_none()
                        .text_sm()
                        .child(format!("{width} × {height} · 只读图片")),
                )
                .into_any_element(),
            PreviewResult::Ready(PreviewPayload::Text { .. }) => div()
                .id("archive-preview-document-scroll")
                .debug_selector(|| "archive-preview-document-scroll".into())
                .size_full()
                .min_h_0()
                .overflow_y_scroll()
                .track_scroll(&self.preview.scroll)
                .vertical_scrollbar(&self.preview.scroll)
                .children(self.preview.document.clone())
                .into_any_element(),
            state => {
                let (title, description) = match state {
                    PreviewResult::Idle => (
                        "选择一个文件",
                        "支持图片、文本、代码和 Markdown；不会打开外部程序。",
                    ),
                    PreviewResult::Loading => ("正在准备预览…", "正在读取选中成员。"),
                    PreviewResult::Unsupported => (
                        "不支持此类型",
                        "仅支持图片与 UTF-8 文本／代码／Markdown；不会外部打开。",
                    ),
                    PreviewResult::TooLarge => (
                        "文件超过预览读取大小上限",
                        "成员解压后文本最多64 KiB，图片最多8 MiB；此处不计算压缩包源大小。",
                    ),
                    PreviewResult::ReadLimit => (
                        "压缩包读取资源达到上限",
                        "原生读取或中间解包达到本次请求的资源额度，尚未进入预览解码；不表示选中文件本身超限。",
                    ),
                    PreviewResult::TextLines => ("文本超过预览行数上限", "文本预览最多1200行。"),
                    PreviewResult::ImageDimensions => (
                        "图片尺寸超过预览上限",
                        "图片每边最多8192像素，总像素最多8 Mi。",
                    ),
                    PreviewResult::ImageMemory => (
                        "图片解码内存超过预览上限",
                        "图片解码预算为32 MiB；JPEG/WebP还需在此预算内预留解码工作空间。",
                    ),
                    PreviewResult::DocumentLimit => (
                        "文档超过预览呈现上限",
                        "文本已读取，但只读文档的结构或呈现资源超过限制。",
                    ),
                    PreviewResult::Directory => ("文件夹不可预览", "请选择一个普通文件。"),
                    PreviewResult::MultipleSelection => ("多选不显示预览", "请选择单个文件。"),
                    PreviewResult::PasswordCancelled => {
                        ("密码输入已取消", "点击重试可重新输入密码。")
                    }
                    PreviewResult::SourceChanged => ("源压缩包已更改", "请刷新目录后重新选择。"),
                    PreviewResult::Disconnected => ("原生连接已断开", "恢复连接并刷新后重试。"),
                    PreviewResult::Corrupt => ("无法解码此文件", "文件可能已损坏或编码不受支持。"),
                    PreviewResult::Failed => {
                        ("预览读取失败", "请检查连接并重试；不会自动外部打开。")
                    }
                    _ => ("预览已取消", "重新选择一个文件以继续。"),
                };
                div()
                    .size_full()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .when(
                        matches!(state, PreviewResult::Loading)
                            && !crate::ui::motion::reduced_motion(cx),
                        |body| body.child(gpui_kit::component::spinner::Spinner::new()),
                    )
                    .child(
                        Empty::new().header(
                            EmptyHeader::new()
                                .title(
                                    EmptyTitle::new().child(
                                        div()
                                            .id("archive-preview-state-title")
                                            .role(Role::Label)
                                            .aria_label(title)
                                            .test_support()
                                            .child(title),
                                    ),
                                )
                                .description(EmptyDescription::new().child(description)),
                        ),
                    )
                    .into_any_element()
            }
        };
        let text = matches!(
            self.preview.result,
            PreviewResult::Ready(PreviewPayload::Text { .. })
        );
        let retry = matches!(
            self.preview.result,
            PreviewResult::PasswordCancelled
                | PreviewResult::Cancelled
                | PreviewResult::Corrupt
                | PreviewResult::Failed
        );
        div()
            .id("archive-preview-pane")
            .debug_selector(|| "archive-preview-pane".into())
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_2()
                    .flex_none()
                    .child(div().min_w_0().flex_1().child("只读预览"))
                    .when(text, |bar| {
                        bar.child(
                            Button::new("archive-preview-source")
                                .label(if self.preview.source {
                                    "渲染"
                                } else {
                                    "原文"
                                })
                                .small()
                                .outline()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.preview_source(window, cx)
                                })),
                        )
                    })
                    .when(retry, |bar| {
                        bar.child(
                            Button::new("archive-preview-retry")
                                .label("重试")
                                .small()
                                .outline()
                                .on_click(cx.listener(|this, _, _, cx| this.retry_preview(cx))),
                        )
                    }),
            )
            .child(div().flex_1().min_h_0().min_w_0().p_2().child(body))
            .into_any_element()
    }
    #[cfg(test)]
    pub(crate) fn preview_token_for_test(&self) -> u64 {
        self.preview.generation
    }
    #[cfg(test)]
    pub(crate) fn prepare_preview_for_test(&mut self, cx: &mut Context<Self>) -> u64 {
        self.preview.fingerprint = None;
        self.observe_preview_selection();
        self.preview.dispatch = false;
        cx.notify();
        self.preview.generation
    }
}

#[cfg(test)]
#[path = "browser_preview_tests.rs"]
mod tests;
