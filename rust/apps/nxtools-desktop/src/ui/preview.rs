use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use gpui::{
    Animation, AnimationExt, Context, ObjectFit, Render, Rgba, WeakEntity, Window, div,
    ease_out_quint, img, prelude::*, px,
};
use gpui_component::scroll::ScrollableElement;
use nxtools_clipboard::{ClipboardItem, ClipboardKind};
use unicode_width::UnicodeWidthStr;

use crate::clipboard::ClipboardApplication;
use crate::theme::{
    COMPACT_RADIUS, META_TEXT_SIZE, RESULT_DETAIL_SIZE, Theme, UI_FONT_FAMILY, UI_MONO_FONT_FAMILY,
};

const TEXT_LINE_HEIGHT: f32 = 20.0;
const META_LINE_HEIGHT: f32 = 16.0;
const WINDOW_HORIZONTAL_CHROME: f32 = 22.0;
const WINDOW_VERTICAL_CHROME: f32 = 22.0;
const CONTENT_META_GAP: f32 = 8.0;
const IMAGE_MAX_CONTENT_WIDTH: f32 = 440.0;
const IMAGE_MAX_CONTENT_HEIGHT: f32 = 340.0;
const PREVIEW_APPEAR_DURATION: Duration = Duration::from_millis(130);

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PreviewLayout {
    pub window_width: f32,
    pub window_height: f32,
    pub body_height: f32,
    pub requires_vertical_scroll: bool,
}

pub(crate) struct ClipboardPreview {
    item: ClipboardItem,
    layout: PreviewLayout,
    theme: Theme,
    application: WeakEntity<ClipboardApplication>,
    appearance_generation: u64,
}

impl ClipboardPreview {
    pub(crate) fn new(
        item: ClipboardItem,
        layout: PreviewLayout,
        application: WeakEntity<ClipboardApplication>,
    ) -> Self {
        Self {
            item,
            layout,
            theme: Theme::zed_dark(),
            application,
            appearance_generation: 0,
        }
    }

    pub(crate) fn set_item(
        &mut self,
        item: ClipboardItem,
        layout: PreviewLayout,
        cx: &mut Context<Self>,
    ) {
        self.item = item;
        self.layout = layout;
        self.appearance_generation = self.appearance_generation.wrapping_add(1);
        cx.notify();
    }
}

impl Render for ClipboardPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let layout = self.layout;
        let application = self.application.clone();
        let appearance_generation = self.appearance_generation;
        let meta = preview_meta(&self.item);
        let content = match self.item.kind {
            ClipboardKind::Text => {
                let body = div()
                    .w_full()
                    .h(px(layout.body_height))
                    .flex_none()
                    .overflow_hidden()
                    .font_family(UI_MONO_FONT_FAMILY)
                    .text_size(px(RESULT_DETAIL_SIZE))
                    .line_height(px(TEXT_LINE_HEIGHT))
                    .whitespace_normal()
                    .child(preview_body(&self.item));
                if layout.requires_vertical_scroll {
                    body.overflow_y_scrollbar().into_any_element()
                } else {
                    body.into_any_element()
                }
            }
            ClipboardKind::Image => {
                let source = existing_image_source(&self.item.image_path);
                div()
                    .w_full()
                    .h(px(layout.body_height))
                    .flex_none()
                    .rounded(px(COMPACT_RADIUS))
                    .overflow_hidden()
                    .bg(theme.surface)
                    .flex()
                    .items_center()
                    .justify_center()
                    .when_some(source, |preview, source| {
                        preview.child(img(source).size_full().object_fit(ObjectFit::Contain))
                    })
                    .when(!Path::new(&self.item.image_path).is_file(), |preview| {
                        preview.child(
                            div()
                                .text_size(px(RESULT_DETAIL_SIZE))
                                .text_color(theme.text_muted)
                                .child("原图片文件已不存在"),
                        )
                    })
                    .into_any_element()
            }
            ClipboardKind::Files => {
                let body = div()
                    .w_full()
                    .h(px(layout.body_height))
                    .flex_none()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(self.item.file_paths.iter().cloned().map(|path| {
                        div()
                            .font_family(UI_MONO_FONT_FAMILY)
                            .text_size(px(RESULT_DETAIL_SIZE))
                            .line_height(px(TEXT_LINE_HEIGHT))
                            .whitespace_normal()
                            .child(path)
                    }));
                if layout.requires_vertical_scroll {
                    body.overflow_y_scrollbar().into_any_element()
                } else {
                    body.into_any_element()
                }
            }
        };

        div()
            .id("clipboard-preview-root")
            .size_full()
            .p(px(10.0))
            .font_family(UI_FONT_FAMILY)
            .overflow_hidden()
            .border_1()
            .border_color(clipboard_preview_border(theme))
            .bg(theme.canvas)
            .text_color(theme.text)
            .flex()
            .flex_col()
            .gap(px(CONTENT_META_GAP))
            .on_hover(move |hovered, _window, cx| {
                let _ = application.update(cx, |application, cx| {
                    application.set_preview_pointer_inside(*hovered, cx);
                });
            })
            .child(content)
            .when(!meta.is_empty(), |preview| {
                preview.child(
                    div()
                        .w_full()
                        .flex_none()
                        .text_size(px(META_TEXT_SIZE))
                        .line_height(px(META_LINE_HEIGHT))
                        .text_color(theme.text_muted)
                        .whitespace_normal()
                        .child(meta),
                )
            })
            .with_animation(
                ("clipboard-preview-appear", appearance_generation),
                Animation::new(PREVIEW_APPEAR_DURATION).with_easing(ease_out_quint()),
                |preview, delta| {
                    let (opacity, vertical_offset) = preview_appearance(delta);
                    preview.opacity(opacity).relative().top(px(vertical_offset))
                },
            )
    }
}

fn preview_appearance(delta: f32) -> (f32, f32) {
    (0.76 + 0.24 * delta, 4.0 * (1.0 - delta))
}

fn clipboard_preview_border(theme: Theme) -> Rgba {
    theme.window_border
}

pub(crate) fn preview_layout(
    item: &ClipboardItem,
    working_width: f32,
    working_height: f32,
) -> PreviewLayout {
    if item.kind == ClipboardKind::Image
        && item.image_width > 0
        && item.image_height > 0
        && Path::new(&item.image_path).is_file()
    {
        return image_layout(item, working_width, working_height);
    }

    let body = preview_body(item);
    let width = text_width(&body, working_width);
    let content_width = (width - WINDOW_HORIZONTAL_CHROME).max(1.0);
    let desired_body_height = estimated_text_lines(&body, content_width) as f32 * TEXT_LINE_HEIGHT;
    measured_layout(
        width,
        desired_body_height,
        meta_height(item, content_width),
        working_height,
    )
}

fn measured_layout(
    width: f32,
    desired_body_height: f32,
    meta_height: f32,
    working_height: f32,
) -> PreviewLayout {
    let meta_block_height = if meta_height > 0.0 {
        CONTENT_META_GAP + meta_height
    } else {
        0.0
    };
    let measured_height = desired_body_height + meta_block_height + WINDOW_VERTICAL_CHROME;
    let maximum_height = (working_height * 0.7).clamp(180.0, 560.0);
    let window_height = measured_height.clamp(76.0, maximum_height);
    let requires_vertical_scroll = measured_height > maximum_height;
    let body_height =
        (window_height - meta_block_height - WINDOW_VERTICAL_CHROME).max(TEXT_LINE_HEIGHT);
    PreviewLayout {
        window_width: width,
        window_height,
        body_height,
        requires_vertical_scroll,
    }
}

fn text_width(text: &str, working_width: f32) -> f32 {
    let length = text.trim().chars().count();
    let width = match length {
        0..=16 => 190.0,
        17..=36 => 240.0,
        37..=80 => 320.0,
        _ => 460.0_f32.min((working_width * 0.42).max(320.0)),
    };
    width.min((working_width - 32.0).max(190.0))
}

fn estimated_text_lines(text: &str, content_width: f32) -> usize {
    let columns = (content_width / 6.8).floor().max(1.0) as usize;
    text.lines()
        .map(|line| UnicodeWidthStr::width(line).max(1).div_ceil(columns))
        .sum::<usize>()
        .max(1)
}

fn meta_height(item: &ClipboardItem, content_width: f32) -> f32 {
    estimated_text_lines(&preview_meta(item), content_width) as f32 * META_LINE_HEIGHT
}

fn image_layout(item: &ClipboardItem, working_width: f32, working_height: f32) -> PreviewLayout {
    let source_width = item.image_width.max(1) as f32;
    let source_height = item.image_height.max(1) as f32;
    let scale = (IMAGE_MAX_CONTENT_WIDTH / source_width)
        .min(IMAGE_MAX_CONTENT_HEIGHT / source_height)
        .min(1.0);
    let content_width = (source_width * scale).clamp(160.0, IMAGE_MAX_CONTENT_WIDTH);
    let desired_body_height = (source_height * scale).clamp(80.0, IMAGE_MAX_CONTENT_HEIGHT);
    let width = (content_width + WINDOW_HORIZONTAL_CHROME).min((working_width - 24.0).max(182.0));
    measured_layout(
        width,
        desired_body_height,
        meta_height(item, content_width),
        working_height,
    )
}

fn preview_kind_label(kind: ClipboardKind) -> &'static str {
    match kind {
        ClipboardKind::Text => "文本",
        ClipboardKind::Image => "图片",
        ClipboardKind::Files => "文件",
    }
}

fn preview_body(item: &ClipboardItem) -> String {
    let body = match item.kind {
        ClipboardKind::Text => item.text.trim().to_owned(),
        ClipboardKind::Image => "原图片文件已不存在".into(),
        ClipboardKind::Files => item.file_paths.join("\n"),
    };
    if body.is_empty() {
        "(空内容)".into()
    } else {
        body
    }
}

fn preview_meta(item: &ClipboardItem) -> String {
    let source = if item.source.process_name.trim().is_empty() {
        "未知来源"
    } else {
        item.source.process_name.trim()
    };
    let detail = match item.kind {
        ClipboardKind::Text => format!("{} 字符", item.text.chars().count()),
        ClipboardKind::Image => format!("{} × {}", item.image_width, item.image_height),
        ClipboardKind::Files => format!("{} 项", item.file_paths.len()),
    };
    format!(
        "{source}  ·  {}  ·  {detail}",
        preview_kind_label(item.kind)
    )
}

fn existing_image_source(path: &str) -> Option<PathBuf> {
    let path = PathBuf::from(path);
    path.is_file().then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nxtools_clipboard::CaptureSource;

    fn image_item(width: u32, height: u32) -> ClipboardItem {
        ClipboardItem {
            id: 1,
            kind: ClipboardKind::Image,
            text: String::new(),
            image_path: "C:/image.png".into(),
            image_width: width,
            image_height: height,
            file_paths: Vec::new(),
            source: CaptureSource {
                window_handle: 1,
                process_id: 2,
                process_name: "fixture".into(),
                process_icon_path: String::new(),
                window_title: String::new(),
            },
            captured_at_unix_ms: 1,
            copy_count: 1,
            is_favorite: false,
            is_pinned: false,
            deleted_at_unix_ms: None,
        }
    }

    fn text_item(text: &str) -> ClipboardItem {
        let mut item = image_item(0, 0);
        item.kind = ClipboardKind::Text;
        item.text = text.into();
        item.image_path.clear();
        item
    }

    #[test]
    fn long_cjk_preview_uses_reference_width_and_never_underallocates_body() {
        let text = "重新装备时基础属性存在Roll值，玩家可以使用特定材料对装备进行重铸操作，重铸会重置装备的Roll值。".repeat(5);
        let layout = preview_layout(&text_item(&text), 1920.0, 1080.0);

        assert_eq!(layout.window_width, 460.0);
        assert!(layout.body_height >= 160.0);
        assert!(!layout.requires_vertical_scroll);
        assert!(layout.window_height >= layout.body_height + 40.0);
    }

    #[test]
    fn oversized_text_is_bounded_to_seventy_percent_and_scrolls() {
        let item = text_item(&"很长的中文预览内容".repeat(600));
        let layout = preview_layout(&item, 1920.0, 720.0);

        assert_eq!(layout.window_height, 504.0);
        assert!(layout.requires_vertical_scroll);
    }

    #[test]
    fn short_text_uses_minimum_height_slack_above_the_bottom_metadata() {
        let mut item = text_item("1234");
        item.source.process_name = "N".into();
        let layout = preview_layout(&item, 1920.0, 1080.0);
        let content_width = layout.window_width - WINDOW_HORIZONTAL_CHROME;
        let metadata = meta_height(&item, content_width);

        assert_eq!(layout.window_height, 76.0);
        assert_eq!(
            layout.body_height + CONTENT_META_GAP + metadata + WINDOW_VERTICAL_CHROME,
            layout.window_height,
        );
    }

    #[test]
    fn image_preview_preserves_reference_aspect_ratio_and_does_not_upscale() {
        let wide = image_layout(&image_item(400, 120), 1920.0, 1080.0);
        let tall = image_layout(&image_item(120, 400), 1920.0, 1080.0);

        assert_eq!(wide.body_height, 120.0);
        assert_eq!(tall.body_height, 340.0);
    }

    #[test]
    fn missing_image_is_not_sent_to_the_gpui_asset_loader() {
        assert!(existing_image_source("Z:/definitely-missing/nxtools-image.png").is_none());
        let layout = preview_layout(&image_item(4_000, 3_000), 1920.0, 1080.0);
        assert_eq!(layout.window_width, 190.0);
        assert!(layout.window_height < 120.0);
    }

    #[test]
    fn preview_appearance_is_short_generation_scoped_and_never_repeats() {
        assert_eq!(
            PREVIEW_APPEAR_DURATION,
            std::time::Duration::from_millis(130)
        );
        assert_eq!(preview_appearance(0.0), (0.76, 4.0));
        assert_eq!(preview_appearance(1.0), (1.0, 0.0));

        let source = include_str!("preview.rs");
        let render = source
            .split("impl Render for ClipboardPreview")
            .nth(1)
            .expect("preview render implementation");
        assert!(render.contains("(\"clipboard-preview-appear\", appearance_generation)"));
        assert!(render.contains(".with_animation("));
        assert!(!render.contains(".repeat()"));

        let outer_shell = render
            .split(".id(\"clipboard-preview-root\")")
            .nth(1)
            .and_then(|source| source.split(".with_animation(").next())
            .expect("preview outer shell source");
        assert!(
            !outer_shell.contains(".rounded("),
            "a rounded transparent preview root exposes the rectangular HWND corners"
        );
    }

    #[test]
    fn clipboard_preview_uses_the_same_neutral_window_border_as_launcher() {
        let theme = Theme::zed_dark();
        assert_eq!(clipboard_preview_border(theme), theme.window_border);
    }
}
