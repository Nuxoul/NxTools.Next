//! Shared Markdown editor presentation inside the original passive HWND.
use super::{ClipboardPreview, preview_body};
use crate::ui::markdown_document::{self, DocumentMode, MarkdownDocument};
use crate::ui::preview_text::{self, TextKind};
use gpui_kit::component::{Sizable as _, button::Button};
use gpui_kit::{
    Context, Entity, IntoElement, ParentElement as _, Styled as _, Window, div, prelude::*, px,
};
use nxtools_clipboard::{ClipboardItem, ClipboardKind};

/// Locked0.7.1 TSX has only TS-specific captures; C# has no captures. Supplement via public API.
pub(super) fn ensure_preview_grammars() {
    static REGISTERED: std::sync::Once = std::sync::Once::new();
    REGISTERED.call_once(|| {
        let registry = gpui_kit::component::highlighter::LanguageRegistry::singleton();
        if let (Some(mut tsx), Some(typescript)) = (registry.language("tsx"), registry.language("typescript")) {
            tsx.highlights = format!("{}\n{}\n(jsx_opening_element name: (identifier) @tag)\n(jsx_closing_element name: (identifier) @tag)\n(jsx_self_closing_element name: (identifier) @tag)",typescript.highlights,tsx.highlights).into();
            registry.register("tsx", &tsx);
        }
        if let (Some(mut cpp), Some(c)) = (registry.language("cpp"), registry.language("c")) {
            cpp.highlights = format!("{}\n{}", c.highlights, cpp.highlights).into();
            registry.register("cpp", &cpp);
        }
        if let Some(mut csharp) = registry.language("csharp") {
            csharp.highlights = r#"
                (comment) @comment
                (string_literal) @string
                (character_literal) @string
                (integer_literal) @number
                (real_literal) @number
                (boolean_literal) @boolean
                (predefined_type) @type
                (class_declaration name: (identifier) @type)
                (interface_declaration name: (identifier) @type)
                (method_declaration name: (identifier) @function)
                ["using" "namespace" "class" "interface" "public" "private" "static" "return" "new" "async" "await" "var" "if" "else" "for" "while"] @keyword
            "#.into();
            registry.register("csharp", &csharp);
        }
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ViewingMode {
    Auto,
    Source,
}

pub(super) struct PreparedText {
    pub mode: ViewingMode,
    pub markdown: Option<Entity<MarkdownDocument>>,
    pub code: Option<Entity<MarkdownDocument>>,
}
impl PreparedText {
    pub fn new(
        item: &ClipboardItem,
        mode: ViewingMode,
        window: &mut Window,
        cx: &mut Context<ClipboardPreview>,
    ) -> Self {
        let started = std::time::Instant::now();
        ensure_preview_grammars();
        let original = if item.kind == ClipboardKind::Text {
            item.text.as_str()
        } else {
            ""
        };
        let budget = preview_text::within_budget(original);
        let kind = if item.kind != ClipboardKind::Text || !budget || mode == ViewingMode::Source {
            TextKind::Plain
        } else {
            match preview_text::classify(original) {
                TextKind::Code(language) => TextKind::Code(language),
                _ => TextKind::Markdown,
            }
        };
        let presentation = match kind {
            TextKind::Code(language) => DocumentMode::Literal(Some(language)),
            TextKind::Markdown => DocumentMode::Markdown,
            TextKind::Plain => DocumentMode::Literal(None),
        };
        let prepared = (item.kind == ClipboardKind::Text)
            .then(|| markdown_document::document(original, presentation, window, cx))
            .flatten();
        let (markdown, code) = if matches!(kind, TextKind::Code(_)) {
            (None, prepared)
        } else {
            (prepared, None)
        };
        if item.kind == ClipboardKind::Text {
            tracing::info!(event="clipboard.preview.text_prepared",correlation_id=%uuid::Uuid::new_v4(),item_id=item.id,
                mode=?mode,kind=?kind,budget_fallback=!budget,elapsed_ms=started.elapsed().as_millis(),"local preview presentation prepared");
        }
        Self {
            mode,
            markdown,
            code,
        }
    }
}

impl ClipboardPreview {
    pub(super) fn change_viewing_mode(
        &mut self,
        mode: ViewingMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.rich = PreparedText::new(&self.item, mode, window, cx);
        self.text_scroll
            .set_offset(gpui_kit::point(px(0.0), px(0.0)));
        cx.notify();
    }
    pub(super) fn rich_text_body(&self, _cx: &gpui_kit::App) -> Option<gpui_kit::AnyElement> {
        let state = self.rich.markdown.as_ref().or(self.rich.code.as_ref())?;
        let is_code = self.rich.code.is_some();
        Some(
            div()
                .id("clipboard-preview-document-scroll")
                .debug_selector(move || {
                    if is_code {
                        "clipboard-preview-code".into()
                    } else {
                        "clipboard-preview-markdown".into()
                    }
                })
                .w_full()
                .h(px(self.body_viewport_height()))
                .flex_none()
                .overflow_y_scroll()
                .track_scroll(&self.text_scroll)
                .child(state.clone())
                .into_any_element(),
        )
    }
    pub(super) fn text_controls(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let source = cx.entity().downgrade();
        let mode = self.rich.mode;
        let generation = self.appearance_generation;
        Button::new("clipboard-preview-source")
            .small()
            .outline()
            .debug_selector(|| "clipboard-preview-source".into())
            .label(if mode == ViewingMode::Source {
                "渲染"
            } else {
                "原文"
            })
            .on_click(move |_, window, cx| {
                let source = source.clone();
                window.defer(cx, move |window, cx| {
                    let _ = source.update(cx, |view, cx| {
                        if view.appearance_generation == generation {
                            view.change_viewing_mode(
                                if mode == ViewingMode::Source {
                                    ViewingMode::Auto
                                } else {
                                    ViewingMode::Source
                                },
                                window,
                                cx,
                            );
                        }
                    });
                });
            })
    }
    pub(super) fn literal_text(&self) -> String {
        if self.item.kind == ClipboardKind::Text {
            self.item.text.clone()
        } else {
            preview_body(&self.item)
        }
    }
}
