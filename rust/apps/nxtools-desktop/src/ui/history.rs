use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{App, ClickEvent, ObjectFit, Window, div, img, prelude::*, px};
use gpui_component::tooltip::Tooltip;
use nxtools_clipboard::{ClipboardItem, ClipboardKind};

use crate::theme::Theme;

pub const TEXT_ROW_HEIGHT: f32 = 68.0;
pub const IMAGE_PREVIEW_FALLBACK_HEIGHT: f32 = 96.0;
pub const IMAGE_PREVIEW_MAX_HEIGHT: f32 = 270.0;
pub const IMAGE_ROW_CHROME_HEIGHT: f32 = 52.0;
pub const ROW_GAP: f32 = 5.0;
const MATERIAL_SYMBOLS: &str = "Material Symbols Outlined";

pub fn history_row(
    theme: Theme,
    entry: ClipboardItem,
    index: usize,
    available_image_width: f32,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let source = if entry.source.process_name.is_empty() {
        "未知来源".to_owned()
    } else {
        entry.source.process_name.clone()
    };
    let source_icon_path = entry.source.process_icon_path.clone();
    let relative_time = relative_time(entry.captured_at_unix_ms);
    let usage = entry.copy_count.max(1).to_string();
    let card_height = row_card_height(&entry, available_image_width);
    let slot_height = card_height + ROW_GAP;
    let content = match entry.kind {
        ClipboardKind::Text => text_content(theme, &entry),
        ClipboardKind::Image => image_content(theme, &entry, available_image_width),
        ClipboardKind::Files => files_content(theme, &entry),
    };

    div()
        .id(("history-row-slot", index))
        .h(px(slot_height))
        .pb(px(ROW_GAP))
        .child(
            div()
                .id(("history-row", index))
                .h(px(card_height))
                .px_2()
                .py(px(5.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(theme.border_subtle)
                .bg(theme.surface)
                .cursor_pointer()
                .hover(move |style| {
                    style
                        .bg(theme.surface_hover)
                        .border_color(theme.border_active)
                })
                .on_click(on_click)
                .flex()
                .flex_col()
                .gap(px(5.0))
                .child(
                    div()
                        .h(px(18.0))
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .child(usage_badge(theme, usage))
                        .child(source_icon(theme, source, source_icon_path, index))
                        .child(time_label(theme, relative_time))
                        .child(div().flex_1()),
                )
                .child(content),
        )
}

fn text_content(theme: Theme, entry: &ClipboardItem) -> gpui::AnyElement {
    let character_count = entry.text.chars().count();
    div()
        .min_w_0()
        .h(px(23.0))
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .min_w_0()
                .flex_1()
                .text_size(px(12.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.text)
                .truncate()
                .child(entry.text.clone()),
        )
        .child(metric_chip(theme, format!("{character_count} 字符")))
        .into_any_element()
}

fn files_content(theme: Theme, entry: &ClipboardItem) -> gpui::AnyElement {
    let preview = entry
        .file_paths
        .iter()
        .map(|path| {
            PathBuf::from(path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.clone())
        })
        .collect::<Vec<_>>()
        .join("  ·  ");
    div()
        .min_w_0()
        .h(px(23.0))
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .min_w_0()
                .flex_1()
                .text_size(px(12.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.text)
                .truncate()
                .child(preview),
        )
        .child(metric_chip(theme, format!("{} 项", entry.file_paths.len())))
        .into_any_element()
}

fn image_content(
    theme: Theme,
    entry: &ClipboardItem,
    available_image_width: f32,
) -> gpui::AnyElement {
    let preview_height =
        image_preview_height(entry.image_width, entry.image_height, available_image_width);
    div()
        .w_full()
        .h(px(preview_height + 8.0))
        .p(px(4.0))
        .rounded(px(5.0))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.image_well)
        .flex()
        .items_center()
        .justify_center()
        .child(
            img(PathBuf::from(&entry.image_path))
                .w_full()
                .h_full()
                .object_fit(ObjectFit::Contain),
        )
        .into_any_element()
}

fn source_icon(
    theme: Theme,
    source: String,
    process_icon_path: String,
    index: usize,
) -> impl IntoElement {
    let icon = if process_icon_path.is_empty() {
        div()
            .font_family(MATERIAL_SYMBOLS)
            .text_size(px(15.0))
            .text_color(theme.text_secondary)
            .child("apps")
            .into_any_element()
    } else {
        img(PathBuf::from(process_icon_path))
            .size(px(16.0))
            .object_fit(ObjectFit::Contain)
            .into_any_element()
    };
    div()
        .id(("clipboard-source", index))
        .size(px(16.0))
        .flex()
        .items_center()
        .justify_center()
        .tooltip(move |window, cx| Tooltip::new(source.clone()).build(window, cx))
        .child(icon)
}

fn usage_badge(theme: Theme, text: String) -> impl IntoElement {
    div()
        .min_w(px(20.0))
        .h(px(18.0))
        .px(px(6.0))
        .rounded(px(9.0))
        .bg(theme.surface_raised)
        .text_size(px(9.0))
        .text_color(theme.text_secondary)
        .flex()
        .items_center()
        .justify_center()
        .truncate()
        .child(text)
}

fn time_label(theme: Theme, text: String) -> impl IntoElement {
    div()
        .max_w(px(92.0))
        .text_size(px(10.0))
        .text_color(theme.text_muted)
        .truncate()
        .child(text)
}

fn metric_chip(theme: Theme, text: String) -> impl IntoElement {
    div()
        .h(px(18.0))
        .px(px(2.0))
        .text_size(px(9.0))
        .text_color(theme.text_muted)
        .flex()
        .items_center()
        .child(text)
}

fn row_card_height(entry: &ClipboardItem, available_image_width: f32) -> f32 {
    if entry.kind == ClipboardKind::Image {
        image_preview_height(entry.image_width, entry.image_height, available_image_width)
            + IMAGE_ROW_CHROME_HEIGHT
    } else {
        TEXT_ROW_HEIGHT
    }
}

fn image_preview_height(width: u32, height: u32, available_width: f32) -> f32 {
    if width == 0 || height == 0 || available_width <= 0.0 {
        return IMAGE_PREVIEW_FALLBACK_HEIGHT;
    }
    let scale = 1.0_f32
        .min(available_width / width as f32)
        .min(IMAGE_PREVIEW_MAX_HEIGHT / height as f32);
    (height as f32 * scale).max(28.0)
}

fn relative_time(timestamp_ms: i64) -> String {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(timestamp_ms);
    let seconds = now_ms.saturating_sub(timestamp_ms).max(0) / 1_000;
    match seconds {
        0..=59 => "刚刚".into(),
        60..=3_599 => format!("{} 分钟前", seconds / 60),
        3_600..=86_399 => format!("{} 小时前", seconds / 3_600),
        _ => format!("{} 天前", seconds / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_metrics_match_the_reference_panel() {
        assert_eq!(TEXT_ROW_HEIGHT, 68.0);
        assert_eq!(ROW_GAP, 5.0);
        assert_eq!(IMAGE_PREVIEW_MAX_HEIGHT, 270.0);
    }

    #[test]
    fn image_preview_preserves_aspect_ratio_without_upscaling() {
        assert!((image_preview_height(270, 33, 246.0) - 30.07).abs() < 0.01);
        assert_eq!(image_preview_height(100, 120, 246.0), 120.0);
        assert_eq!(image_preview_height(1_000, 2_000, 246.0), 270.0);
    }
}
