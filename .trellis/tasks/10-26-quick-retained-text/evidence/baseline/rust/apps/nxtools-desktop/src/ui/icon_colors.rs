//! RGB-only presentation metadata. IO and palette work stay on the existing storage worker.
use super::*;

impl ClipboardPanel {
    pub(super) fn request_source_icon_colors(
        &mut self,
        paths: Vec<String>,
        cx: &mut Context<Self>,
    ) {
        if !self.is_open || self.icon_colors_pending {
            return;
        }
        let mut unique = HashSet::new();
        let paths = paths
            .into_iter()
            .filter(|path| {
                !path.is_empty()
                    && !self.source_icon_colors.contains_key(path)
                    && unique.insert(path.clone())
            })
            .take(4)
            .collect::<Vec<_>>();
        if paths.is_empty() {
            return;
        }
        self.icon_colors_pending = true;
        let worker = self.history_worker.clone();
        let generation = self.query_dispatch_generation;
        cx.spawn(async move |panel, cx| {
            let result = worker
                .source_icon_colors_async(paths.iter().map(PathBuf::from).collect())
                .await;
            let _ = panel.update(cx, |panel, cx| {
                panel.icon_colors_pending = false;
                if !panel.is_open || panel.query_dispatch_generation != generation {
                    cx.notify();
                    return;
                }
                let active = panel
                    .entries
                    .borrow()
                    .iter()
                    .map(|item| item.source.process_icon_path.clone())
                    .collect::<HashSet<_>>();
                let results = result.unwrap_or_else(|_| {
                    paths
                        .into_iter()
                        .map(|path| (PathBuf::from(path), None))
                        .collect()
                });
                for (path, color) in results {
                    let path = path.to_string_lossy().into_owned();
                    if active.contains(&path) {
                        panel.source_icon_colors.insert(path, color);
                    }
                }
                panel.prune_list_metadata();
                cx.notify();
            });
        })
        .detach();
    }
}
