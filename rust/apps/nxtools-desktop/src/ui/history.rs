use gpui::{div, prelude::*, px};

use crate::theme::Theme;

#[derive(Clone)]
pub struct HistoryEntry {
    source: &'static str,
    summary: &'static str,
    detail: &'static str,
    kind: EntryKind,
}

#[derive(Clone, Copy)]
enum EntryKind {
    Text,
    Image,
    Link,
}

pub fn sample_entries() -> Vec<HistoryEntry> {
    let templates = [
        HistoryEntry {
            source: "Visual Studio Code",
            summary: "Rust 与 C# 只通过版本化 IPC 通信",
            detail: "刚刚 · 142 字符",
            kind: EntryKind::Text,
        },
        HistoryEntry {
            source: "Snipping Tool",
            summary: "图片按原始比例预览，不制造空白区域",
            detail: "2 分钟前 · 1280 × 720",
            kind: EntryKind::Image,
        },
        HistoryEntry {
            source: "Microsoft Edge",
            summary: "https://gpui.rs — GPUI documentation",
            detail: "5 分钟前 · 可直接打开",
            kind: EntryKind::Link,
        },
    ];
    (0..90)
        .map(|index| templates[index % templates.len()].clone())
        .collect()
}

pub fn history_row(theme: Theme, entry: HistoryEntry, index: usize) -> impl IntoElement + use<> {
    let kind = match entry.kind {
        EntryKind::Text => "Tt",
        EntryKind::Image => "▧",
        EntryKind::Link => "↗",
    };

    div()
        .id(("history-row", index))
        .h(px(68.0))
        .mx_2()
        .px_3()
        .flex()
        .items_center()
        .gap_3()
        .border_b_1()
        .border_color(theme.border)
        .cursor_pointer()
        .hover(move |style| style.bg(theme.surface_raised))
        .child(
            div()
                .size(px(34.0))
                .rounded_md()
                .bg(theme.accent_soft)
                .text_color(theme.accent)
                .text_sm()
                .flex()
                .items_center()
                .justify_center()
                .child(kind),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.text)
                        .truncate()
                        .child(entry.summary),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.text_muted)
                        .child(format!("{}  ·  {}", entry.source, entry.detail)),
                ),
        )
        .child(div().text_xs().text_color(theme.text_muted).child("⋯"))
}
