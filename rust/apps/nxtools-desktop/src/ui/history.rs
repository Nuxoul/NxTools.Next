use gpui::{App, ClickEvent, Window, div, prelude::*, px};
use nxtools_clipboard::ClipboardItem;

use crate::theme::Theme;

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
    let detail = if entry.copy_count > 1 {
        format!("已连续复制 {} 次", entry.copy_count)
    } else if entry.source.window_title.is_empty() {
        "纯文本".to_owned()
    } else {
        entry.source.window_title.clone()
    };

    div()
        .id(("history-row", index))
        .min_h(px(68.0))
        .mx_2()
        .px_3()
        .py_2()
        .flex()
        .items_center()
        .gap_3()
        .border_b_1()
        .border_color(theme.border)
        .cursor_pointer()
        .hover(move |style| style.bg(theme.surface_raised))
        .on_click(on_click)
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
                .child("Tt"),
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
                        .child(entry.text),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.text_muted)
                        .truncate()
                        .child(format!("{source}  ·  {detail}")),
                ),
        )
        .child(div().text_xs().text_color(theme.text_muted).child("粘贴"))
}
