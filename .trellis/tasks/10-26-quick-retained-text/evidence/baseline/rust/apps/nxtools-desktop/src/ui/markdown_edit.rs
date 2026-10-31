//! Explicit Clipboard editing, with byte-preserving no-edit and source-mode save paths.
use super::*;
use gpui_kit::AnyElement;
use gpui_kit::component::Disableable as _;
use markdown_editor::BlockEditor;

impl ClipboardPanel {
    fn rich_edit_allowed(text: &str) -> bool {
        preview_text::safe_markdown(text).is_some()
    }
    pub(super) fn prepare_markdown_edit(
        &mut self,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        markdown_document::init(cx);
        self.edit_original = text.to_owned();
        self.edit_rich_source = text.to_owned();
        self.edit_source_mode = !Self::rich_edit_allowed(text);
        if self.edit_source_mode {
            self.edit_input.focus_handle(cx).focus(window, cx);
        } else {
            self.edit_rich.update(cx, |state, cx| {
                state
                    .set_source(text, cx)
                    .expect("validated Markdown draft");
            });
            self.edit_rich_baseline = self.edit_rich.read(cx).source_snapshot().text().to_string();
            self.edit_rich.focus_handle(cx).focus(window, cx);
        }
    }
    pub(super) fn markdown_edit_value(&self, cx: &App) -> String {
        if self.edit_source_mode {
            self.edit_input.read(cx).value().to_string()
        } else {
            let canonical = self.edit_rich.read(cx).source_snapshot();
            if canonical.text().as_ref() == self.edit_rich_baseline {
                self.edit_rich_source.clone()
            } else {
                canonical.text().to_string()
            }
        }
    }
    fn switch_markdown_edit_mode(
        &mut self,
        source: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.edit_source_mode == source || self.editing_item_id.is_none() {
            return;
        }
        let draft = self.markdown_edit_value(cx);
        if source {
            self.edit_input
                .update(cx, |state, cx| state.set_value(draft, window, cx));
            self.edit_source_mode = true;
            self.edit_input.focus_handle(cx).focus(window, cx);
        } else if Self::rich_edit_allowed(&draft) {
            self.edit_rich.update(cx, |state, cx| {
                state
                    .set_source(&draft, cx)
                    .expect("validated Markdown draft")
            });
            self.edit_rich_source = draft;
            self.edit_rich_baseline = self.edit_rich.read(cx).source_snapshot().text().to_string();
            self.edit_source_mode = false;
            self.edit_rich.focus_handle(cx).focus(window, cx);
        }
        cx.notify();
    }
    pub(super) fn render_markdown_editor(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let source_mode = self.edit_source_mode;
        let rich_allowed =
            !source_mode || preview_text::within_budget(self.edit_input.read(cx).value().as_ref());
        let source_owner = cx.entity().downgrade();
        let rich_owner = source_owner.clone();
        let height = (window.viewport_size().height.as_f32() - 236.0).max(64.0);
        let body = if source_mode {
            Textarea::new(&self.edit_input)
                .aria_label("剪贴板文本")
                .h(px(height))
                .into_any_element()
        } else {
            let scroll = self.edit_rich.read(cx).scroll_handle();
            div()
                .id("clipboard-rich-editor-scroll")
                .debug_selector(|| "clipboard-rich-editor".into())
                .role(gpui_kit::Role::Group)
                .aria_label("剪贴板文本")
                .w_full()
                .h(px(height))
                .min_h_0()
                .overflow_y_scroll()
                .when_some(scroll, |body, scroll| body.track_scroll(&scroll))
                .child(BlockEditor::new(&self.edit_rich).min_h(px(height)).p_2())
                .into_any_element()
        };
        div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        Button::new("clipboard-edit-rich")
                            .small()
                            .outline()
                            .label("富文本")
                            .disabled(!rich_allowed)
                            .on_click(move |_, window, cx| {
                                let owner = rich_owner.clone();
                                window.defer(cx, move |window, cx| {
                                    let _ = owner.update(cx, |panel, cx| {
                                        panel.switch_markdown_edit_mode(false, window, cx)
                                    });
                                });
                            }),
                    )
                    .child(
                        Button::new("clipboard-edit-source")
                            .small()
                            .outline()
                            .label("原文")
                            .on_click(move |_, window, cx| {
                                let owner = source_owner.clone();
                                window.defer(cx, move |window, cx| {
                                    let _ = owner.update(cx, |panel, cx| {
                                        panel.switch_markdown_edit_mode(true, window, cx)
                                    });
                                });
                            }),
                    ),
            )
            .child(body)
            .into_any_element()
    }
}
