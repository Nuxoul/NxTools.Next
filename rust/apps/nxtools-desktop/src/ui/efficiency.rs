//! Small input additions on the original history selection/action owner.
use super::*;
use gpui_kit::{EntityInputHandler as _, KeyDownEvent, KeyUpEvent};

pub(super) struct ArmedNumberKey {
    key: String,
    item_id: i64,
    generation: u64,
    correlation_id: uuid::Uuid,
    started_at: Instant,
    index: usize,
}

impl ClipboardPanel {
    pub(super) fn history_keyboard_ready(&self, window: &mut Window, cx: &mut App) -> bool {
        self.is_open
            && window.window_handle() == self.notification_source
            && self.category_editor.is_none()
            && self.editing_item_id.is_none()
            && !self.search_pending
            && self.query_error.is_none()
            && self.pending_action.is_none()
            && self.full_item_request.is_none()
            && self.pending_action_started_at.is_none()
            && !window.has_active_dialog(cx)
            && !components::native_context_menu::active(window, cx)
    }
    pub(super) fn search_is_composing(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.search_input.update(cx, |state, cx| {
            state.marked_text_range(window, cx).is_some()
        })
    }
    pub(super) fn move_page(
        &mut self,
        direction: isize,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let entries = self.entries.borrow();
        if entries.is_empty() {
            return false;
        }
        let mut height = 0.0_f32;
        let budget = (f32::from(window.viewport_size().height) - 120.0).max(64.0);
        let current = self.selected_entry_index.unwrap_or(0);
        let mut count = 0_usize;
        while count < entries.len() {
            let index = if direction > 0 {
                current.saturating_add(count).min(entries.len() - 1)
            } else {
                current.saturating_sub(count)
            };
            height += self
                .list_scroll
                .bounds_for_item(index)
                .map(|bounds| bounds.size.height.as_f32())
                .unwrap_or_else(|| {
                    history::row_card_height(&entries[index], self.list_width.max(200.0))
                        + history::ROW_GAP
                });
            if height > budget {
                break;
            }
            count += 1;
            if (direction > 0 && index + 1 == entries.len()) || (direction < 0 && index == 0) {
                break;
            }
        }
        drop(entries);
        self.move_selection(direction * count.max(1) as isize, cx)
    }
    fn number_shortcut_index(key: &gpui_kit::Keystroke) -> Option<usize> {
        let modifiers = key.modifiers;
        if !modifiers.control
            || modifiers.alt
            || modifiers.shift
            || modifiers.platform
            || modifiers.function
        {
            return None;
        }
        key.key
            .parse::<usize>()
            .ok()
            .filter(|number| *number <= 9)
            .map(|number| if number == 0 { 9 } else { number - 1 })
    }
    fn shortcut_block_reason(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<&'static str> {
        if !self.is_open {
            Some("panel_closed")
        } else if self.category_editor.is_some() || self.editing_item_id.is_some() {
            Some("editor_active")
        } else if window.has_active_dialog(cx)
            || components::native_context_menu::active(window, cx)
        {
            Some("overlay_active")
        } else if self.search_pending {
            Some("query_pending")
        } else if self.query_error.is_some() {
            Some("query_failed")
        } else if self.pending_action.is_some()
            || self.pending_action_started_at.is_some()
            || self.full_item_request.is_some()
        {
            Some("action_pending")
        } else if self.search_is_composing(window, cx) {
            Some("ime_composing")
        } else {
            None
        }
    }
    pub(super) fn log_rejected_number_shortcut(
        &mut self,
        key: &gpui_kit::Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = Self::number_shortcut_index(key)
            && let Some(reason) = self.shortcut_block_reason(window, cx)
        {
            self.reject_number_shortcut(index, reason);
        }
    }
    fn reject_number_shortcut(&mut self, index: usize, reason: &'static str) {
        if self
            .last_shortcut_rejection
            .is_some_and(|(last, at)| last == reason && at.elapsed() < Duration::from_secs(1))
        {
            return;
        }
        self.last_shortcut_rejection = Some((reason, Instant::now()));
        info!(event="clipboard.shortcut.rejected", session_id=%self.session_id, index=index+1, reason, generation=self.query_dispatch_generation);
    }
    pub(super) fn number_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = Self::number_shortcut_index(&event.keystroke) else {
            self.armed_number_key = None;
            return;
        };
        if let Some(reason) = self.shortcut_block_reason(window, cx) {
            self.armed_number_key = None;
            if !event.is_held {
                self.reject_number_shortcut(index, reason);
            }
            return;
        }
        if !self.history_keyboard_ready(window, cx) {
            return;
        }
        if !event.is_held {
            let item_id = self.entries.borrow().get(index).map(|entry| entry.id);
            self.armed_number_key = item_id.map(|item_id| ArmedNumberKey {
                key: event.keystroke.key.clone(),
                item_id,
                index,
                generation: self.query_dispatch_generation,
                correlation_id: uuid::Uuid::new_v4(),
                started_at: Instant::now(),
            });
            if let Some(armed) = &self.armed_number_key {
                info!(event="clipboard.shortcut.accepted", correlation_id=%armed.correlation_id,
                    session_id=%self.session_id, index=index+1, generation=armed.generation);
                self.dismiss_preview_for_input("number_shortcut", cx);
            } else {
                self.reject_number_shortcut(index, "index_unavailable");
            }
        }
        cx.stop_propagation();
    }
    pub(super) fn number_key_up(
        &mut self,
        event: &KeyUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .armed_number_key
            .as_ref()
            .is_none_or(|armed| event.keystroke.key != armed.key)
        {
            return;
        }
        let armed = self
            .armed_number_key
            .take()
            .expect("matching armed shortcut");
        let modifiers = event.keystroke.modifiers;
        let reason = if modifiers.alt || modifiers.shift || modifiers.platform || modifiers.function
        {
            Some("modifiers_changed")
        } else if armed.generation != self.query_dispatch_generation {
            Some("query_changed")
        } else {
            self.shortcut_block_reason(window, cx)
        };
        if let Some(reason) = reason {
            info!(event="clipboard.shortcut.cancelled", correlation_id=%armed.correlation_id, session_id=%self.session_id,
                reason, elapsed_ms=armed.started_at.elapsed().as_millis() as u64);
            return;
        }
        if !self.history_keyboard_ready(window, cx) {
            return;
        }
        let entry = self
            .entries
            .borrow()
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.id == armed.item_id)
            .map(|(index, entry)| (index, entry.clone()));
        if let Some((index, entry)) = entry {
            self.selected_entry_index = Some(index);
            self.activate_entry_correlated(
                &entry,
                ClipboardAction::OriginalPaste,
                window,
                armed.correlation_id,
                cx,
            );
            let accepted = self
                .pending_action
                .is_some_and(|(id, _, _)| id == armed.correlation_id);
            info!(event="clipboard.shortcut.dispatched", correlation_id=%armed.correlation_id,
                session_id=%self.session_id, index=armed.index+1, accepted,
                elapsed_ms=armed.started_at.elapsed().as_millis() as u64);
            cx.stop_propagation();
        } else {
            info!(event="clipboard.shortcut.cancelled", correlation_id=%armed.correlation_id,
                session_id=%self.session_id, reason="item_retired", elapsed_ms=armed.started_at.elapsed().as_millis() as u64);
        }
    }

    pub(crate) fn unpack_summary_page(
        &mut self,
        page: nxtools_clipboard::ClipboardSummaryPage,
    ) -> ClipboardHistoryPage {
        let items = page
            .items
            .into_iter()
            .map(|summary| {
                self.summary_counts
                    .insert(summary.item.id, summary.text_character_count);
                summary.item
            })
            .collect();
        ClipboardHistoryPage {
            items,
            total: page.total,
            has_more: page.has_more,
        }
    }

    pub(crate) fn with_summary_counts(mut self, counts: HashMap<i64, usize>) -> Self {
        self.summary_counts = counts;
        self
    }

    pub(super) fn load_full_item_for_management(
        &mut self,
        item_id: i64,
        action: ClipboardItemAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let operation_id = uuid::Uuid::new_v4();
        if self.full_item_request.is_some()
            || self.pending_action.is_some()
            || self.search_pending
            || !self.is_open
        {
            return;
        }
        self.set_operation_status_for(operation_id, PanelStatus::Pending("正在读取完整记录…"), cx);
        self.full_item_request = Some(operation_id);
        let generation = self.query_dispatch_generation;
        let worker = self.history_worker.clone();
        let source = window.window_handle();
        cx.spawn(async move |panel, cx| {
            let result = worker.active_item_async(item_id).await;
            // Window and entity updates occur in a new application turn after worker completion.
            let _ = source.update(cx, |_, window, cx| {
                let _ = panel.update(cx, |panel, cx| {
                    if panel.full_item_request != Some(operation_id) {
                        return;
                    }
                    panel.full_item_request = None;
                    panel.status = PanelStatus::Ready(String::new());
                    if !panel.is_open
                        || panel.query_dispatch_generation != generation
                        || panel.search_pending
                        || !panel.entries.borrow().iter().any(|item| item.id == item_id)
                    {
                        return;
                    }
                    if panel.pending_action.is_some() || panel.category_editor.is_some() {
                        return;
                    }
                    match result {
                        Ok(item) => {
                            let count = panel.summary_counts.remove(&item_id);
                            panel.handle_item_action(&item, action, window, cx);
                            if let Some(count) = count {
                                panel.summary_counts.insert(item_id, count);
                            }
                        }
                        Err(_) => panel.set_operation_status_for(
                            operation_id,
                            PanelStatus::Error("记录已删除或暂时无法读取，请刷新后重试".into()),
                            cx,
                        ),
                    }
                    cx.notify();
                });
            });
        })
        .detach();
    }
}
