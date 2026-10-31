//! Reuse Downloads/QuickLaunch's native ShellIconExtractor and shared IconCache.
//! Only synthetic type targets: never materialize/inspect archive members or shortcuts.
use super::*;
use nxtools_protocol::{ExtractShellIconCommand, PlatformCommand, PlatformCommandResult};
use std::{collections::BTreeSet, rc::Rc, time::Instant};

const MAX_TYPES: usize = 128;
const MAX_PENDING: usize = 8;
const FOLDER_TYPE: &str = "nxtools-folder-type:directory";
#[derive(Default)]
pub(super) struct TypeIcons {
    platform: Option<Rc<nxtools_ipc::PlatformBridge>>,
    cache: BTreeMap<String, PathBuf>,
    failed: BTreeSet<String>,
    pending: BTreeMap<Uuid, Pending>,
    epoch: u64,
}
struct Pending {
    key: String,
    snapshot: Uuid,
    generation: u64,
    epoch: u64,
    validating: bool,
    started: Instant,
    _deadline: gpui_kit::Task<()>,
}
fn key(row: &Row) -> Option<String> {
    if row.parent || row.warning {
        return None;
    }
    if row.directory {
        return Some(FOLDER_TYPE.into());
    }
    if row.kind != "File" {
        return None;
    }
    let (_, extension) = row.name.rsplit_once('.')?;
    (extension.len() <= 16
        && !extension.is_empty()
        && extension.bytes().all(|b| b.is_ascii_alphanumeric()))
    .then(|| extension.to_ascii_lowercase())
}
impl ArchiveBrowser {
    pub(crate) fn configure_icons(&mut self, platform: Rc<nxtools_ipc::PlatformBridge>) {
        self.clear_pending_icons();
        self.type_icons.platform = Some(platform);
    }
    pub(super) fn clear_pending_icons(&mut self) {
        self.type_icons.epoch = self.type_icons.epoch.wrapping_add(1);
        self.type_icons.pending.clear();
        self.type_icons.failed.clear();
    }
    pub(super) fn icon_path(&self, row: &Row) -> Option<&PathBuf> {
        self.type_icons.cache.get(&key(row)?)
    }
    /// One shared folder PNG for Tree and directory rows. None keeps the official fallback.
    pub(super) fn folder_icon_path(&self) -> Option<&PathBuf> {
        self.type_icons.cache.get(FOLDER_TYPE)
    }
    pub(super) fn folder_icon_description(&self) -> &'static str {
        if self.folder_icon_path().is_some() {
            "文件夹"
        } else if self.type_icons.failed.contains(FOLDER_TYPE) {
            "文件夹 · 系统图标加载失败，显示备用图标"
        } else if self
            .type_icons
            .platform
            .as_ref()
            .is_none_or(|platform| !platform.is_connected())
        {
            "文件夹 · 离线备用图标"
        } else {
            "文件夹 · 正在加载系统图标"
        }
    }
    pub(super) fn queue_folder_icon(&mut self, cx: &mut Context<Self>) {
        self.queue_icon_keys(BTreeSet::from([FOLDER_TYPE.into()]), cx);
    }
    // Rendering only queues work. All entity mutations/IPC occur on the next turn.
    pub(super) fn queue_type_icons(&mut self, rows: &[Row], cx: &mut Context<Self>) {
        self.queue_icon_keys(rows.iter().filter_map(key).collect(), cx);
    }
    fn queue_icon_keys(&mut self, keys: BTreeSet<String>, cx: &mut Context<Self>) {
        let Some(snapshot) = self.model.as_ref().map(|m| m.snapshot) else {
            return;
        };
        if keys.is_empty()
            || self.type_icons.platform.is_none()
            || self.retired
            || !matches!(self.status, BrowserStatus::Ready)
        {
            return;
        }
        let generation = self.generation;
        let epoch = self.type_icons.epoch;
        let view = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = view.update(cx, |this, cx| {
                if this.retired
                    || this.generation != generation
                    || this.type_icons.epoch != epoch
                    || this.model.as_ref().is_none_or(|m| m.snapshot != snapshot)
                    || !matches!(this.status, BrowserStatus::Ready)
                {
                    return;
                }
                this.request_type_icons(keys, snapshot, cx);
            });
        });
    }
    fn request_type_icons(
        &mut self,
        keys: BTreeSet<String>,
        snapshot: Uuid,
        cx: &mut Context<Self>,
    ) {
        let Some(platform) = self
            .type_icons
            .platform
            .clone()
            .filter(|p| p.is_connected())
        else {
            return;
        };
        for key in keys {
            if self.type_icons.pending.len() >= MAX_PENDING
                || self.type_icons.cache.len()
                    + self.type_icons.failed.len()
                    + self.type_icons.pending.len()
                    >= MAX_TYPES
            {
                break;
            }
            if self.type_icons.cache.contains_key(&key)
                || self.type_icons.failed.contains(&key)
                || self.type_icons.pending.values().any(|p| p.key == key)
            {
                continue;
            }
            let id = Uuid::new_v4();
            let generation = self.generation;
            let epoch = self.type_icons.epoch;
            let deadline = cx.spawn(async move |this, cx| {
                cx.background_executor().timer(std::time::Duration::from_secs(15)).await;
                let _ = this.update(cx, |this, cx| {
                    if let Some(pending) = this.type_icons.pending.remove(&id) {
                        this.type_icons.failed.insert(pending.key);
                        tracing::warn!(event="archive.type_icon.finished", correlation_id=%id, outcome="timeout", elapsed_ms=pending.started.elapsed().as_millis() as u64, "archive type icon finished");
                        cx.notify();
                    }
                });
            });
            self.type_icons.pending.insert(
                id,
                Pending {
                    key: key.clone(),
                    snapshot,
                    generation,
                    epoch,
                    validating: false,
                    started: Instant::now(),
                    _deadline: deadline,
                },
            );
            let command = PlatformCommand::ExtractShellIcon(ExtractShellIconCommand {
                target: if key == FOLDER_TYPE {
                    FOLDER_TYPE.into()
                } else {
                    format!("nxtools-file-type:.{key}")
                },
                size: 32,
            });
            if platform.send_command_with_id(command, id).is_err() {
                self.type_icons.pending.remove(&id);
                self.type_icons.failed.insert(key);
                tracing::warn!(event="archive.type_icon.finished", correlation_id=%id, outcome="not_submitted", "archive type icon finished");
            } else {
                tracing::info!(event="archive.type_icon.started", correlation_id=%id, "archive type icon accepted");
            }
        }
    }
    pub(crate) fn icon_matches(&self, id: Uuid) -> bool {
        self.type_icons.pending.contains_key(&id)
    }
    pub(crate) fn icon_result(
        &mut self,
        id: Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) {
        let Some(pending) = self.type_icons.pending.get_mut(&id) else {
            return;
        };
        if pending.validating {
            return;
        }
        pending.validating = true;
        let path = result
            .success
            .then(|| {
                result
                    .data
                    .as_deref()
                    .filter(|s| s.len() <= 32768)
                    .map(PathBuf::from)
            })
            .flatten();
        let native_error_code = result.native_error_code;
        let validation = cx
            .background_executor()
            .spawn(async move { path.filter(|p| crate::icons::shell_icon_path_is_renderable(p)) });
        cx.spawn(async move |this, cx| {
            let path = validation.await;
            let _ = this.update(cx, |this, cx| {
                let Some(pending) = this.type_icons.pending.remove(&id) else { return; };
                if this.retired || this.generation != pending.generation || this.type_icons.epoch != pending.epoch
                    || this.model.as_ref().is_none_or(|m| m.snapshot != pending.snapshot)
                    || this.type_icons.platform.as_ref().is_none_or(|p| !p.is_connected()) { return; }
                let success = path.is_some();
                if let Some(path) = path { this.type_icons.cache.insert(pending.key, path); }
                else { this.type_icons.failed.insert(pending.key); }
                tracing::info!(event="archive.type_icon.finished", correlation_id=%id, success, native_error_code,
                    elapsed_ms=pending.started.elapsed().as_millis() as u64, "archive type icon finished");
                cx.notify();
            });
        }).detach();
    }
    pub(crate) fn icons_disconnected(&mut self) {
        self.clear_pending_icons();
    }
}

#[cfg(test)]
#[path = "browser_icons_tests.rs"]
mod tests;
