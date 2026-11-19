use super::*;
use gpui_kit::component::{
    WindowExt as _,
    button::{Toggle, ToggleGroup, ToggleVariants as _},
    dialog::{DialogAction, DialogClose, DialogFooter},
    input::{Editor, EditorState, RopeExt as _},
};
use gpui_kit::{AnyElement, Focusable as _};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Mode {
    #[default]
    Preview,
    Code,
    Split,
}

#[derive(Default)]
pub(super) struct Draft {
    mode: Mode,
    modified: Option<String>,
    source: Option<Entity<EditorState>>,
    source_changes: Option<Subscription>,
}

impl Draft {
    pub fn dirty(&self) -> bool {
        self.modified.is_some()
    }

    pub(super) fn is_split(&self) -> bool {
        self.mode == Mode::Split
    }

    pub(super) fn shows_preview(&self) -> bool {
        self.mode != Mode::Code
    }

    pub(super) fn shows_source(&self) -> bool {
        self.mode != Mode::Preview
    }

    pub(super) fn source(&self) -> Option<&Entity<EditorState>> {
        self.source.as_ref()
    }

    #[cfg(test)]
    pub(super) fn source_top_line(&self, cx: &App) -> Option<usize> {
        top_line(self.source()?.read(cx))
    }

    pub(super) fn source_offset_at(&self, line: usize, cx: &App) -> Option<usize> {
        let text = self.source()?.read(cx).text();
        (line < text.lines_len()).then(|| text.line_start_offset(line))
    }

    pub(super) fn align_source(&self, offset: usize, cx: &mut App) -> bool {
        let Some(source) = self.source() else {
            return false;
        };
        let state = source.read(cx);
        let Some(height) = state.line_height().filter(|height| *height > px(0.)) else {
            return false;
        };
        let text = state.text();
        let line = text.offset_to_point(offset.min(text.len())).row;
        let row = state.display_map().buffer_line_to_display_row(line);
        let current = state.scroll_offset();
        let target = gpui_kit::point(current.x, -height * row as f32);
        if target == current {
            return false;
        }
        source.update(cx, |state, cx| state.set_scroll_offset(target, cx));
        true
    }

    pub(super) fn text<'a>(&'a self, committed: &'a str) -> &'a str {
        self.modified.as_deref().unwrap_or(committed)
    }

    pub fn refresh_clean(&self, text: &str, window: &mut Window, cx: &mut App) {
        if let Some(source) = &self.source
            && source.read(cx).value().as_ref() != text
        {
            source.update(cx, |source, cx| source.set_value(text, window, cx));
        }
    }
}

pub(super) fn top_line(state: &EditorState) -> Option<usize> {
    let height = state.line_height().filter(|height| *height > px(0.))?;
    let row = ((-state.scroll_offset().y / height) + 0.001).max(0.) as usize;
    Some(state.display_map().display_row_to_buffer_line(row))
}
impl Reader {
    pub(super) fn set_mode(&mut self, mode: Mode, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return;
        };
        if tab.draft.mode == mode && self.history.is_none() {
            return;
        }
        let started = Instant::now();
        let correlation = Uuid::new_v4();
        let previous = tab.draft.mode;
        let preview_position = (previous == Mode::Preview)
            .then(|| tab.preview_source(cx))
            .flatten();
        let source_position = (previous == Mode::Code)
            .then(|| {
                top_line(tab.draft.source()?.read(cx))
                    .and_then(|line| tab.draft.source_offset_at(line, cx))
            })
            .flatten();
        tracing::info!(event = "markdown.mode.started", correlation_id = %correlation, from = ?tab.draft.mode, to = ?mode);
        self.history = None;
        if previous == Mode::Preview && mode != Mode::Preview {
            tab.enter_source_at(preview_position);
        }
        if previous == Mode::Code && mode != Mode::Code {
            tab.enter_preview_at(source_position);
        }
        tab.draft.mode = mode;
        if mode == Mode::Preview {
            self.focus.focus(window, cx);
        } else {
            if tab.draft.source.is_none() {
                let text = tab.draft.text(&tab.document.text);
                let id = tab.id;
                let source = cx.new(|cx| {
                    EditorState::new(window, cx)
                        .language(gpui_kit::component::highlighter::Language::Markdown)
                        .line_number(true)
                        .default_value(text)
                });
                tab.draft.source_changes = Some(cx.subscribe(
                    &source,
                    move |_, state, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            let text = state.read(cx).value().to_string();
                            let entity = state.entity_id();
                            let owner = cx.weak_entity();
                            cx.defer(move |cx| {
                                let _ = owner.update(cx, |this, cx| {
                                    this.draft_changed(id, entity, text, cx)
                                });
                            });
                        }
                    },
                ));
                tab.draft.source = Some(source);
            }
            tab.draft
                .refresh_clean(tab.draft.text(&tab.document.text), window, cx);
            if let Some(source) = &tab.draft.source {
                source.read(cx).focus_handle(cx).focus(window, cx);
            }
        }
        self.dismiss_outline();
        tracing::info!(event = "markdown.mode.finished", correlation_id = %correlation, mode = ?mode, elapsed_ms = started.elapsed().as_millis());
        cx.notify();
    }

    fn draft_changed(
        &mut self,
        id: usize,
        entity: gpui_kit::EntityId,
        text: String,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) else {
            return;
        };
        if tab
            .draft
            .source
            .as_ref()
            .is_none_or(|source| source.entity_id() != entity)
        {
            return;
        }
        if text == tab.draft.text(&tab.document.text) {
            return;
        }
        tab.navigation.content_changed();
        tab.view.update(cx, |state, cx| state.set_text(&text, cx));
        tab.draft.modified = (text != tab.document.text).then_some(text);
        cx.notify();
    }

    pub(super) fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return;
        };
        if self.saving.is_some() || !tab.draft.dirty() {
            return;
        }
        let id = tab.id;
        let path = tab.document.path.clone();
        let before = tab.document.original();
        let text = tab.draft.text(&tab.document.text).to_owned();
        let correlation = Uuid::new_v4();
        let started = Instant::now();
        tracing::info!(event = "markdown.save.started", correlation_id = %correlation);
        let result = cx.background_spawn(async move {
            document::save(path, &before, &text).map(|saved| {
                let recorded = history::observe(&saved, "saved", correlation);
                (saved, recorded)
            })
        });
        self.saving = Some(cx.spawn_in(window, async move |owner, cx| {
            let result = result.await;
            let _ = owner.update_in(cx, |this, _, cx| {
                this.saving = None;
                let Some(tab) = this.tabs.iter_mut().find(|tab| tab.id == id) else { return };
                match result {
                    Ok((saved, recorded)) => {
                        let current = tab.draft.text(&tab.document.text);
                        tab.draft.modified = (current != saved.text).then(|| current.to_owned());
                        tab.outline.update(cx, |outline, cx| outline.set_items(outline_items(&saved.headings), cx));
                        tab.watch_stamp = saved.stamp;
                        tab.document = saved;
                        tab.problem = (!recorded).then_some("文件已保存，但新版本未能写入历史。");
                        tracing::info!(event = "markdown.save.finished", correlation_id = %correlation, outcome = "saved", elapsed_ms = started.elapsed().as_millis());
                    }
                    Err(error) => {
                        tab.problem = Some(if matches!(error, document::ReadError::Changed) { "文件已被外部修改，未覆盖。草稿仍保留。" } else { "保存失败。请检查文件权限后重试；草稿仍保留。" });
                        tracing::warn!(event = "markdown.save.finished", correlation_id = %correlation, outcome = error.code(), native_error = ?error.native_code(), elapsed_ms = started.elapsed().as_millis());
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    pub(super) fn request_close(
        &mut self,
        tab_id: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.saving.is_some() || self.pdf.is_some() || self.export_worker.is_some() {
            return;
        }
        let dirty = self
            .tabs
            .iter()
            .any(|tab| tab_id.is_none_or(|id| tab.id == id) && tab.draft.dirty());
        if !dirty {
            if let Some(id) = tab_id {
                self.close_tab(id, cx);
            } else {
                window.remove_window();
            }
            return;
        }
        if window.has_active_dialog(cx) {
            return;
        }
        let owner = cx.weak_entity();
        window.open_dialog(cx, move |dialog, _, _| {
            let owner = owner.clone();
            dialog
                .title("放弃未保存的更改？")
                .child("关闭后，未保存的草稿将丢失。")
                .footer(
                    DialogFooter::new()
                        .child(
                            DialogClose::new().trigger(|button| button.label("继续编辑").outline()),
                        )
                        .child(
                            DialogAction::new().child(
                                Button::new("markdown-discard-draft")
                                    .label("放弃草稿")
                                    .danger()
                                    .on_click(|event, window, cx| {
                                        if event.is_keyboard() {
                                            window.dispatch_action(
                                                Box::new(gpui_kit::component::dialog::Confirm {
                                                    secondary: false,
                                                }),
                                                cx,
                                            );
                                        }
                                    }),
                            ),
                        ),
                )
                .on_ok(move |_, window, cx| {
                    let owner = owner.clone();
                    window.defer(cx, move |window, cx| {
                        let _ = owner.update(cx, |this, cx| {
                            if let Some(id) = tab_id {
                                this.close_tab(id, cx);
                            } else {
                                window.remove_window();
                            }
                        });
                    });
                    true
                })
        });
    }

    pub(super) fn mode_actions(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tab = self.active_tab();
        let active = tab.map_or(Mode::Preview, |tab| tab.draft.mode);
        let disabled = tab.is_none() || self.saving.is_some();
        let modes = [
            ("markdown-mode-code", "代码", Mode::Code),
            ("markdown-mode-split", "分栏", Mode::Split),
            ("markdown-mode-preview", "预览", Mode::Preview),
        ];
        let owner = cx.weak_entity();
        let selector = ToggleGroup::new("markdown-reader-mode")
            .segmented()
            .outline()
            .small()
            .disabled(disabled)
            .children(modes.map(|(id, label, mode)| {
                Toggle::new(id)
                    .label(label)
                    .tooltip(label)
                    .checked(mode == active)
            }))
            .on_click(move |next, window, cx| {
                let Some((_, _, requested)) = modes
                    .iter()
                    .zip(next)
                    .find_map(|(mode, &checked)| (checked && mode.2 != active).then_some(*mode))
                else {
                    return;
                };
                let owner = owner.clone();
                window.defer(cx, move |window, cx| {
                    let _ = owner.update(cx, |this, cx| this.set_mode(requested, window, cx));
                });
            });
        div()
            .flex()
            .flex_none()
            .gap_2()
            .child(
                div()
                    .id("markdown-mode-selector-bounds")
                    .debug_selector(|| "markdown-mode-selector".into())
                    .child(selector),
            )
            .child(
                Button::new("markdown-save")
                    .label("保存")
                    .small()
                    .outline()
                    .disabled(!tab.is_some_and(|tab| tab.draft.dirty()) || self.saving.is_some())
                    .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
            )
    }

    pub(super) fn render_mode(
        &self,
        tab: &TabDocument,
        viewport: gpui_kit::Size<gpui_kit::Pixels>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match tab.draft.mode {
            Mode::Preview => self.render_document(tab, viewport, cx).into_any_element(),
            Mode::Code | Mode::Split => {
                let source = tab.draft.source.as_ref().expect("active source editor");
                let content = div()
                    .id("markdown-source-editor")
                    .debug_selector(|| "markdown-source-editor".into())
                    .relative()
                    .size_full()
                    .min_h_0()
                    .child(
                        Editor::new(source)
                            .bordered(false)
                            .rounded_none()
                            .bg(presentation::paper(cx).background)
                            .text_size(px(self.font))
                            .line_height(gpui_kit::relative(1.5))
                            .h(gpui_kit::relative(1.))
                            .aria_label("Markdown 源码"),
                    )
                    .child(self.source_probe(tab, cx));
                if tab.draft.mode == Mode::Code {
                    return div().flex_1().min_h_0().child(content).into_any_element();
                }
                self.source_split(tab, viewport, content.into_any_element(), cx)
            }
        }
    }
}
