use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{App, ClickEvent, ObjectFit, SharedString, Window, div, img, prelude::*, px};
use nxtools_clipboard::{ClipboardItem, ClipboardKind};

use crate::theme::Theme;

pub const TEXT_ROW_HEIGHT: f32 = 86.0;
pub const ROW_GAP: f32 = 5.0;
pub const LIST_ITEM_HEIGHT: f32 = TEXT_ROW_HEIGHT + ROW_GAP;

pub fn history_row(
    theme: Theme,
    entry: ClipboardItem,
    index: usize,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let source = if entry.source.process_name.is_empty() {
        "未知来源".to_owned()
    } else {
        entry.source.process_name.clone()
    };
    let group: SharedString = format!("clipboard-row-{index}").into();
    let character_count = entry.text.chars().count();
    let copied = (entry.copy_count > 1).then(|| format!("{}×", entry.copy_count));
    let relative_time = relative_time(entry.captured_at_unix_ms);
    let kind_label = match entry.kind {
        ClipboardKind::Text => "文本",
        ClipboardKind::Image => "图片",
        ClipboardKind::Files => "文件",
    };
    let content = match entry.kind {
        ClipboardKind::Text => div()
            .min_w_0()
            .flex_1()
            .text_sm()
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(theme.text)
            .truncate()
            .child(entry.text.clone())
            .into_any_element(),
        ClipboardKind::Image => div()
            .min_w_0()
            .flex_1()
            .h(px(48.0))
            .flex()
            .items_center()
            .gap_2()
            .child(
                img(PathBuf::from(&entry.image_path))
                    .w(px(72.0))
                    .h(px(48.0))
                    .object_fit(ObjectFit::Contain),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .child(format!("{} × {}", entry.image_width, entry.image_height)),
            )
            .into_any_element(),
        ClipboardKind::Files => div()
            .min_w_0()
            .flex_1()
            .text_sm()
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(theme.text)
            .truncate()
            .child(
                entry
                    .file_paths
                    .iter()
                    .map(|path| {
                        PathBuf::from(path)
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.clone())
                    })
                    .collect::<Vec<_>>()
                    .join("  ·  "),
            )
            .into_any_element(),
    };

    div()
        .id(("history-row-slot", index))
        .h(px(LIST_ITEM_HEIGHT))
        .pb(px(ROW_GAP))
        .child(
            div()
                .id(("history-row", index))
                .group(group.clone())
                .h(px(TEXT_ROW_HEIGHT))
                .px_2()
                .py(px(6.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(theme.border)
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
                        .gap_1()
                        .when_some(copied, |row, copied| {
                            row.child(metadata_chip(theme, copied, true))
                        })
                        .child(metadata_chip(theme, kind_label.to_owned(), true))
                        .child(metadata_chip(theme, source, false))
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.text_muted)
                                .child(relative_time),
                        )
                        .child(div().flex_1())
                        .child(
                            div()
                                .invisible()
                                .group_hover(group, |style| style.visible())
                                .h(px(18.0))
                                .px_2()
                                .rounded(px(4.0))
                                .border_1()
                                .border_color(theme.border_active)
                                .bg(theme.surface_raised)
                                .text_xs()
                                .text_color(theme.accent_text)
                                .flex()
                                .items_center()
                                .child("粘贴"),
                        ),
                )
                .child(
                    div()
                        .min_w_0()
                        .h(px(24.0))
                        .flex()
                        .items_center()
                        .child(content)
                        .when(entry.kind == ClipboardKind::Text, |row| {
                            row.child(
                                div()
                                    .ml_2()
                                    .px_2()
                                    .h(px(18.0))
                                    .rounded(px(9.0))
                                    .border_1()
                                    .border_color(theme.border)
                                    .text_xs()
                                    .text_color(theme.text_muted)
                                    .flex()
                                    .items_center()
                                    .child(format!("{character_count} 字符")),
                            )
                        }),
                ),
        )
}

fn metadata_chip(theme: Theme, text: String, accented: bool) -> impl IntoElement {
    div()
        .max_w(px(104.0))
        .h(px(18.0))
        .px_2()
        .rounded(px(9.0))
        .border_1()
        .border_color(if accented { theme.accent } else { theme.border })
        .bg(if accented {
            theme.accent_soft
        } else {
            theme.surface_raised
        })
        .text_xs()
        .text_color(if accented {
            theme.accent_text
        } else {
            theme.text_secondary
        })
        .flex()
        .items_center()
        .truncate()
        .child(text)
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
        assert_eq!(TEXT_ROW_HEIGHT, 86.0);
        assert_eq!(ROW_GAP, 5.0);
        assert_eq!(LIST_ITEM_HEIGHT, 91.0);
    }
}
