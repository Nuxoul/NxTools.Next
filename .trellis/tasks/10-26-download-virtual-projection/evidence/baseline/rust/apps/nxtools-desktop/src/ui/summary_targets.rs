//! Resolve full-body targets on the storage worker, keeping only bounded metadata in the list.
use super::*;
use nxtools_clipboard::ClipboardOpenTarget;

impl ClipboardPanel {
    pub(super) fn prune_list_metadata(&mut self) {
        let entries = self.entries.borrow();
        let ids = entries.iter().map(|item| item.id).collect::<HashSet<_>>();
        let images = entries
            .iter()
            .map(|item| item.image_path.as_str())
            .collect::<HashSet<_>>();
        self.summary_counts.retain(|id, _| ids.contains(id));
        self.open_targets.retain(|id, _| ids.contains(id));
        self.list_thumbnails
            .retain(|path, _| images.contains(path.as_str()));
        let icons = entries
            .iter()
            .map(|item| item.source.process_icon_path.as_str())
            .collect::<HashSet<_>>();
        self.source_icon_colors
            .retain(|path, _| icons.contains(path.as_str()));
    }
    pub(super) fn request_open_targets(&mut self, item_id: i64, cx: &mut Context<Self>) {
        if !self.is_open || self.open_targets_pending || self.open_targets.contains_key(&item_id) {
            return;
        }
        let ids = vec![item_id];
        self.open_targets_pending = true;
        let worker = self.history_worker.clone();
        let generation = self.query_dispatch_generation;
        cx.spawn(async move |panel, cx| {
            let result = worker.open_targets_async(ids.clone()).await;
            let _ = panel.update(cx, |panel, cx| {
                panel.open_targets_pending = false;
                if panel.query_dispatch_generation != generation || !panel.is_open {
                    cx.notify();
                    return;
                }
                let resolved =
                    result.unwrap_or_else(|_| ids.into_iter().map(|id| (id, Vec::new())).collect());
                for (id, targets) in resolved {
                    panel.open_targets.insert(id, targets);
                }
                panel.prune_list_metadata();
                cx.notify();
            });
        })
        .detach();
    }
    pub(super) fn open_full_item_target(
        &mut self,
        item_id: i64,
        target: ClipboardOpenTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_action.is_some() || self.search_pending || !self.is_open {
            return;
        }
        let token = uuid::Uuid::new_v4();
        self.full_item_request = Some(token);
        let generation = self.query_dispatch_generation;
        let worker = self.history_worker.clone();
        let source = window.window_handle();
        cx.spawn(async move |panel, cx| {
            let result = worker.open_targets_async(vec![item_id]).await;
            let _ = source.update(cx, |_, _, cx| {
                let _ = panel.update(cx, |panel, cx| {
                    if panel.full_item_request != Some(token) {
                        return;
                    }
                    panel.full_item_request = None;
                    if !panel.is_open
                        || panel.query_dispatch_generation != generation
                        || panel.search_pending
                        || panel.pending_action.is_some()
                        || !panel.entries.borrow().iter().any(|item| item.id == item_id)
                    {
                        return;
                    }
                    if result.is_ok_and(|rows| {
                        rows.into_iter()
                            .any(|(id, targets)| id == item_id && targets.contains(&target))
                    }) {
                        schedule_entity_update(
                            panel.application.clone(),
                            move |owner, _| owner.open_quick_target(&target),
                            cx,
                        );
                    } else {
                        panel.set_operation_status_for(
                            token,
                            PanelStatus::Error("记录或打开目标已变化，请刷新后重试".into()),
                            cx,
                        );
                    }
                });
            });
        })
        .detach();
    }
}
