//! Type-anywhere fallback delegates platform text/IME to the existing search InputState.
use super::*;
use gpui_kit::{ElementInputHandler, InputHandler, UTF16Selection};
use std::ops::Range;

pub(super) struct SearchInputFallback {
    input: ElementInputHandler<InputState>,
    panel: WeakEntity<ClipboardPanel>,
}
impl SearchInputFallback {
    pub(super) fn new(
        bounds: Bounds<Pixels>,
        input: Entity<InputState>,
        panel: WeakEntity<ClipboardPanel>,
    ) -> Self {
        Self {
            input: ElementInputHandler::new(bounds, input),
            panel,
        }
    }
    fn restore_search(&self, window: &mut Window, cx: &mut App) -> bool {
        self.panel
            .update(cx, |panel, cx| {
                if !panel.search_input_ready(window, cx) {
                    return false;
                }
                panel.focus_search_input(window, cx);
                panel.dismiss_preview_for_input("search_input", cx);
                true
            })
            .unwrap_or(false)
    }
}
impl InputHandler for SearchInputFallback {
    fn selected_text_range(
        &mut self,
        ignore_disabled_input: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<UTF16Selection> {
        self.input
            .selected_text_range(ignore_disabled_input, window, cx)
    }
    fn marked_text_range(&mut self, window: &mut Window, cx: &mut App) -> Option<Range<usize>> {
        self.input.marked_text_range(window, cx)
    }
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<String> {
        self.input.text_for_range(range, adjusted_range, window, cx)
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut App,
    ) {
        if !text.is_empty()
            && !text.chars().all(char::is_control)
            && self.restore_search(window, cx)
        {
            self.input.replace_text_in_range(range, text, window, cx);
        }
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selection: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.restore_search(window, cx) {
            self.input
                .replace_and_mark_text_in_range(range, text, selection, window, cx);
        }
    }
    fn unmark_text(&mut self, window: &mut Window, cx: &mut App) {
        self.input.unmark_text(window, cx);
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Bounds<Pixels>> {
        self.input.bounds_for_range(range, window, cx)
    }
    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<usize> {
        self.input.character_index_for_point(position, window, cx)
    }
    fn set_selected_text_range(&mut self, range: Range<usize>, window: &mut Window, cx: &mut App) {
        self.input.set_selected_text_range(range, window, cx);
    }
    fn element_bounds(&mut self, window: &mut Window, cx: &mut App) -> Option<Bounds<Pixels>> {
        self.input.element_bounds(window, cx)
    }
    fn text_length_utf16(&mut self, window: &mut Window, cx: &mut App) -> Option<usize> {
        self.input.text_length_utf16(window, cx)
    }
    fn accepts_text_input(&mut self, window: &mut Window, cx: &mut App) -> bool {
        self.panel
            .update(cx, |panel, cx| panel.search_input_ready(window, cx))
            .unwrap_or(false)
    }
    fn prefers_ime_for_printable_keys(&mut self, window: &mut Window, cx: &mut App) -> bool {
        self.input.prefers_ime_for_printable_keys(window, cx)
    }
    fn text_input_configuration(
        &mut self,
        window: &mut Window,
        cx: &mut App,
    ) -> gpui_kit::TextInputConfiguration {
        self.input.text_input_configuration(window, cx)
    }
    fn text_input_editable_range(
        &mut self,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Range<usize>> {
        self.input.text_input_editable_range(window, cx)
    }
}

impl ClipboardPanel {
    pub(super) fn search_input_ready(&self, window: &mut Window, cx: &mut App) -> bool {
        self.is_open
            && window.window_handle() == self.notification_source
            && self.category_editor.is_none()
            && self.editing_item_id.is_none()
            && self.pending_action.is_none()
            && self.full_item_request.is_none()
            && !window.has_active_dialog(cx)
            && !components::native_context_menu::active(window, cx)
    }
    pub(super) fn dismiss_preview_for_input(&self, reason: &'static str, cx: &mut Context<Self>) {
        schedule_entity_update(
            self.application.clone(),
            move |owner, cx| owner.dismiss_preview_for_interaction(reason, cx),
            cx,
        );
    }
    pub(super) fn render_search_input_fallback(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let panel = cx.entity().downgrade();
        let input = self.search_input.clone();
        gpui_kit::canvas(
            |_, _, _| (),
            move |_, (), window, cx| {
                let available = panel
                    .update(cx, |panel, cx| panel.search_input_ready(window, cx))
                    .unwrap_or(false);
                if available
                    && !input.focus_handle(cx).is_focused(window)
                    && !window.has_focused_input(cx)
                    && let Some(focus) = window.focused(cx)
                {
                    let bounds = input.read(cx).input_bounds();
                    window.handle_input(&focus, SearchInputFallback::new(bounds, input, panel), cx);
                }
            },
        )
        .absolute()
        .size_0()
    }
}
