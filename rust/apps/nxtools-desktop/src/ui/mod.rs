mod chrome;
mod history;

use gpui::{Context, Entity, Render, Window, div, img, prelude::*, px, uniform_list};
use gpui_component::input::{Input, InputState};

use crate::theme::Theme;
use history::{HistoryEntry, history_row, sample_entries};

pub struct DesktopShell {
    theme: Theme,
    show_preview: bool,
    entries: Vec<HistoryEntry>,
    search_input: Entity<InputState>,
}

impl DesktopShell {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("输入关键词、应用、网址或路径…"));
        Self {
            theme: Theme::zed_dark(),
            show_preview: false,
            entries: sample_entries(),
            search_input,
        }
    }

    fn main_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let preview_visible = self.show_preview;
        div()
            .relative()
            .flex_1()
            .h_full()
            .min_w_0()
            .bg(theme.canvas)
            .flex()
            .flex_col()
            .child(
                div()
                    .px_5()
                    .pt_5()
                    .pb_4()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xl()
                                    .text_color(theme.text)
                                    .child("剪贴板技术验证"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.text_muted)
                                    .child("虚拟列表、中文输入、图片比例、独立滚动与悬浮层"),
                            ),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .id("toggle-preview")
                            .px_3()
                            .h(px(32.0))
                            .rounded_md()
                            .border_1()
                            .border_color(if preview_visible {
                                theme.accent
                            } else {
                                theme.border_active
                            })
                            .bg(if preview_visible {
                                theme.accent_soft
                            } else {
                                theme.surface_raised
                            })
                            .text_sm()
                            .text_color(theme.text)
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .hover(move |style| style.bg(theme.accent_soft))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_preview = !this.show_preview;
                                cx.notify();
                            }))
                            .child(if preview_visible {
                                "关闭预览"
                            } else {
                                "打开预览"
                            }),
                    ),
            )
            .child(self.search_box())
            .child(
                div()
                    .mx_5()
                    .mb_5()
                    .flex_1()
                    .min_h_0()
                    .rounded_lg()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.surface)
                    .overflow_hidden()
                    .child(
                        uniform_list(
                            "clipboard-history",
                            self.entries.len(),
                            cx.processor(|this, range: std::ops::Range<usize>, _window, _cx| {
                                range
                                    .map(|index| {
                                        history_row(this.theme, this.entries[index].clone(), index)
                                    })
                                    .collect::<Vec<_>>()
                            }),
                        )
                        .h_full(),
                    ),
            )
            .when(preview_visible, |panel| panel.child(self.preview_popup()))
    }

    fn search_box(&self) -> impl IntoElement {
        let theme = self.theme;
        div()
            .mx_5()
            .mb_3()
            .h(px(38.0))
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .px_3()
            .flex()
            .items_center()
            .gap_2()
            .text_sm()
            .text_color(theme.text_muted)
            .child("⌕")
            .child(
                div().min_w_0().flex_1().text_color(theme.text).child(
                    Input::new(&self.search_input)
                        .appearance(false)
                        .bordered(false)
                        .focus_bordered(false),
                ),
            )
            .child(
                div()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .bg(theme.surface_raised)
                    .text_xs()
                    .child("Ctrl + Shift + V"),
            )
    }

    fn preview_popup(&self) -> impl IntoElement {
        let theme = self.theme;
        div()
            .absolute()
            .top(px(74.0))
            .right(px(22.0))
            .w(px(360.0))
            .max_h(px(420.0))
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(theme.border_active)
            .bg(theme.surface_raised)
            .shadow_lg()
            .text_sm()
            .text_color(theme.text)
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_muted)
                    .mb_2()
                    .child("SNIPPING TOOL · 图片预览"),
            )
            .child(
                div()
                    .w_full()
                    .max_h(px(270.0))
                    .rounded_md()
                    .overflow_hidden()
                    .bg(theme.canvas)
                    .child(img("clipboard-preview.svg").w_full()),
            )
            .child(
                div()
                    .pt_3()
                    .text_color(theme.text_muted)
                    .child("原始比例决定高度，270px 只是最大值。"),
            )
    }
}

impl Render for DesktopShell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(self.theme.canvas)
            .text_color(self.theme.text)
            .flex()
            .flex_col()
            .child(chrome::title_bar(self.theme))
            .child(
                div()
                    .min_h_0()
                    .flex_1()
                    .flex()
                    .child(chrome::sidebar(self.theme))
                    .child(self.main_panel(cx)),
            )
    }
}
