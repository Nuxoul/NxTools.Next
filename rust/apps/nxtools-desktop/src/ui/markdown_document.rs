//! One bounded document presentation backend for Clipboard and Quick Launch.
//! Original action/save payloads never come from this disposable view projection.
pub(crate) use super::preview_text::TextBudget;
use super::preview_text::{self, TextKind};
use gpui_kit::{
    App, AppContext as _, Bounds, Context, Entity, IntoElement, ParentElement as _, Pixels, Render,
    Styled as _, Window, div,
};
use markdown_editor::{BlockEditor, BlockEditorState, ImagePolicy};
use std::{cell::Cell, rc::Rc};

struct EditorInitialized;
impl gpui_kit::Global for EditorInitialized {}
pub(crate) fn init(cx: &mut App) {
    if cx.try_global::<EditorInitialized>().is_none() {
        markdown_editor::init(cx);
        cx.set_global(EditorInitialized);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DocumentMode {
    Markdown,
    Literal(Option<&'static str>),
}

pub(crate) fn literal_source(text: &str, language: Option<&str>) -> String {
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    format!("{fence}{}\n{text}\n{fence}", language.unwrap_or(""))
}

pub(crate) fn prepared_source(text: &str, mode: DocumentMode) -> Option<String> {
    match mode {
        DocumentMode::Markdown => preview_text::safe_markdown(text),
        DocumentMode::Literal(language) => {
            preview_text::within_budget(text).then(|| literal_source(text, language))
        }
    }
}

pub(crate) fn prepared_source_with_budget(
    text: &str,
    mode: DocumentMode,
    budget: TextBudget,
) -> Option<String> {
    match mode {
        DocumentMode::Markdown => preview_text::safe_markdown_with_budget(text, budget),
        DocumentMode::Literal(language) => {
            budget.allows(text).then(|| literal_source(text, language))
        }
    }
}

pub(crate) fn automatic_mode(text: &str) -> DocumentMode {
    match preview_text::classify(text) {
        TextKind::Code(language) => DocumentMode::Literal(Some(language)),
        _ => DocumentMode::Markdown,
    }
}

pub(crate) struct MarkdownDocument {
    pub(crate) editor: Entity<BlockEditorState>,
    #[cfg(test)]
    mode: DocumentMode,
    #[cfg(test)]
    original: String,
    #[cfg(test)]
    parsed_text: String,
    bounds: Rc<Cell<Bounds<Pixels>>>,
}
impl MarkdownDocument {
    #[cfg(test)]
    pub(crate) fn new(
        original: &str,
        mode: DocumentMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Self> {
        let source = prepared_source(original, mode)?;
        Some(Self::from_prepared(original, mode, &source, window, cx))
    }
    fn from_prepared(
        original: &str,
        mode: DocumentMode,
        source: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        init(cx);
        #[cfg(not(test))]
        let _ = (original, mode);
        let editor = cx.new(|cx| {
            let mut state = BlockEditorState::new(window, cx)
                .with_image_policy(ImagePolicy::Placeholder)
                .with_source(source);
            state.set_readonly(true, cx);
            state
        });
        #[cfg(test)]
        let parsed_text = editor
            .read(cx)
            .model()
            .document()
            .leaves()
            .filter_map(|leaf| leaf.text().map(str::to_owned))
            .collect::<Vec<_>>()
            .join("\n");
        Self {
            editor,
            #[cfg(test)]
            mode,
            #[cfg(test)]
            parsed_text,
            #[cfg(test)]
            original: original.to_owned(),
            bounds: Rc::new(Cell::new(Bounds::default())),
        }
    }
    pub(crate) fn bounds(&self) -> Bounds<Pixels> {
        self.bounds.get()
    }
    /// Inspect the actual parsed editor document, not the incoming Markdown source.
    #[cfg(test)]
    pub(crate) fn rendered_text(&self) -> &String {
        &self.parsed_text
    }
    #[cfg(test)]
    pub(crate) fn language(&self) -> Option<&'static str> {
        match self.mode {
            DocumentMode::Literal(language) => language,
            DocumentMode::Markdown => None,
        }
    }
    #[cfg(test)]
    pub(crate) fn original(&self) -> &str {
        &self.original
    }
}
impl Render for MarkdownDocument {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let bounds = self.bounds.clone();
        div()
            .relative()
            .w_full()
            .min_w_0()
            .child(BlockEditor::new(&self.editor).passive(true))
            .child(
                gpui_kit::canvas(move |current, _, _| bounds.set(current), |_, _, _, _| {})
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
            )
    }
}

pub(crate) fn document(
    original: &str,
    mode: DocumentMode,
    window: &mut Window,
    cx: &mut App,
) -> Option<Entity<MarkdownDocument>> {
    let source = prepared_source(original, mode)?;
    Some(cx.new(|cx| MarkdownDocument::from_prepared(original, mode, &source, window, cx)))
}

pub(crate) fn document_with_budget(
    original: &str,
    mode: DocumentMode,
    budget: TextBudget,
    window: &mut Window,
    cx: &mut App,
) -> Option<Entity<MarkdownDocument>> {
    let source = prepared_source_with_budget(original, mode, budget)?;
    Some(cx.new(|cx| MarkdownDocument::from_prepared(original, mode, &source, window, cx)))
}

struct CachedDocument {
    source: String,
    mode: DocumentMode,
    view: Option<Entity<MarkdownDocument>>,
}
pub(crate) fn cached_view(
    key: gpui_kit::SharedString,
    source: &str,
    mode: DocumentMode,
    window: &mut Window,
    cx: &mut App,
) -> gpui_kit::AnyElement {
    let cached = window.use_keyed_state(key, cx, |window, cx| CachedDocument {
        source: source.to_owned(),
        mode,
        view: document(source, mode, window, cx),
    });
    if cached.read(cx).source != source || cached.read(cx).mode != mode {
        cached.update(cx, |cached, cx| {
            cached.source = source.to_owned();
            cached.mode = mode;
            cached.view = document(source, mode, window, cx);
        });
    }
    match cached.read(cx).view.clone() {
        Some(view) => view.into_any_element(),
        None => div()
            .w_full()
            .whitespace_normal()
            .child(source.to_owned())
            .into_any_element(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::px;
    #[test]
    fn literal_projection_does_not_interpret_markdown_or_break_fences() {
        let original = "**not bold**\n```\n# raw\n";
        let projected = literal_source(original, Some("rust"));
        assert!(projected.starts_with("````rust\n"));
        assert!(projected.contains(original));
    }
    #[gpui_kit::test]
    fn shared_document_uses_real_editor_model_and_preserves_original(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let source = "# Heading\n\n**strong** ![blocked](https://example.invalid/x.png)";
        let (view, surface) = cx.add_window_view(|window, cx| {
            MarkdownDocument::new(source, DocumentMode::Markdown, window, cx).expect("document")
        });
        surface.simulate_resize(gpui_kit::size(px(320.), px(440.)));
        surface.refresh().expect("real editor document");
        view.read_with(surface, |view, cx| {
            let rendered = view.rendered_text();
            assert!(
                rendered.contains("Heading")
                    && rendered.contains("strong")
                    && rendered.contains("未加载")
            );
            assert!(
                !rendered.contains("**strong**") && !rendered.contains("https://example.invalid")
            );
            assert_eq!(view.original(), source);
            assert!(view.editor.read(cx).is_readonly());
        });
    }
}
