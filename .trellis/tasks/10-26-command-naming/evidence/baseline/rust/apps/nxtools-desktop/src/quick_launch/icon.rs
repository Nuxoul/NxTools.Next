use super::*;

#[derive(Default)]
pub(crate) struct IconResourceCache {
    pub(crate) capacity: usize,
    pub(crate) entries: HashMap<String, PathBuf>,
    pub(crate) recency: VecDeque<String>,
}

impl IconResourceCache {
    pub(crate) fn bounded(capacity: usize) -> Self {
        Self {
            capacity,
            ..Self::default()
        }
    }

    pub(crate) fn get(&mut self, key: &str) -> Option<PathBuf> {
        let path = self.entries.get(key)?.clone();
        self.touch(key);
        Some(path)
    }

    pub(crate) fn insert(&mut self, key: String, path: PathBuf) {
        if self.capacity == 0 {
            return;
        }
        self.entries.insert(key.clone(), path);
        self.touch(&key);
        while self.entries.len() > self.capacity {
            let Some(stale) = self.recency.pop_front() else {
                break;
            };
            self.entries.remove(&stale);
        }
    }

    pub(crate) fn touch(&mut self, key: &str) {
        if let Some(position) = self.recency.iter().position(|candidate| candidate == key) {
            self.recency.remove(position);
        }
        self.recency.push_back(key.to_owned());
    }
}

pub(crate) struct PendingIconResource {
    pub(crate) resource_key: String,
    pub(crate) entry_ids: HashSet<String>,
}

pub(crate) fn icon_retry_pending(failed_at: Instant, now: Instant) -> bool {
    now.saturating_duration_since(failed_at) < ICON_RETRY_DELAY
}

pub(crate) fn icon_resource_key(result: &SearchResult, size: u32) -> String {
    match result.entry.source {
        ApplicationSource::IndexedDirectory => format!("{size}:type:folder"),
        ApplicationSource::IndexedFile => {
            let extension = PathBuf::from(&result.entry.target)
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            if !extension.is_empty() && !matches!(extension.as_str(), "exe" | "lnk" | "ico") {
                return format!("{size}:type:.{extension}");
            }
            format!(
                "{size}:target:{}",
                result.entry.target.trim().replace('/', "\\").to_lowercase()
            )
        }
        _ => format!(
            "{size}:target:{}",
            result.entry.target.trim().replace('/', "\\").to_lowercase()
        ),
    }
}
