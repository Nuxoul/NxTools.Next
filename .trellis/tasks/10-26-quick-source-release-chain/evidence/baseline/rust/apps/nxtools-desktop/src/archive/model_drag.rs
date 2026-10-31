//! Bounded metadata-only expansion on the ORIGINAL immutable browser snapshot.
use super::*;
use nxtools_protocol::archive_virtual_drag::{
    ARCHIVE_DRAG_MAX_BYTES, ARCHIVE_DRAG_MAX_DESCRIPTORS, ARCHIVE_DRAG_MAX_FILES,
    ARCHIVE_DRAG_MAX_ROOTS,
};

pub(crate) struct FrozenDragItem {
    pub name: String,
    pub directory: bool,
    pub ordinal: Option<u64>,
    pub size: Option<u64>,
}
impl BrowserModel {
    pub fn freeze_drag_selection(&self) -> Result<Vec<FrozenDragItem>, &'static str> {
        self.freeze_drag_selection_authorized(false)
    }
    pub fn freeze_drag_selection_authorized(
        &self,
        authorized: bool,
    ) -> Result<Vec<FrozenDragItem>, &'static str> {
        const INVALID: &str = "选择包含不安全、冲突或不支持的条目；未拖出任何项目。";
        const LIMIT: &str =
            "拖出最多32个根项目、128个目录项及32个文件，总大小最多512MiB；未拖出任何项目。";
        if self.selected.is_empty() || self.selected.len() > ARCHIVE_DRAG_MAX_ROOTS {
            return Err(LIMIT);
        }
        let mut roots = Vec::new();
        for id in &self.selected {
            let (path, directory) = match id {
                RowId::Entry(snapshot, ordinal) if *snapshot == self.snapshot => {
                    let entry = self
                        .entries
                        .iter()
                        .find(|e| e.ordinal == *ordinal)
                        .ok_or(INVALID)?;
                    (
                        entry.name.trim_end_matches('/').to_owned(),
                        entry.kind == "Directory",
                    )
                }
                RowId::Folder(snapshot, path) if *snapshot == self.snapshot => (path.clone(), true),
                _ => return Err(INVALID),
            };
            roots.push((path, directory));
        }
        let prefix = if self.folder.is_empty() {
            String::new()
        } else {
            format!("{}/", self.folder)
        };
        let mut frozen = BTreeMap::<String, FrozenDragItem>::new();
        let mut ordinals = BTreeSet::new();
        for entry in &self.entries {
            let path = entry.name.trim_end_matches('/');
            let folded = path.to_lowercase();
            if !roots.iter().any(|(root, directory)| {
                folded == root.to_lowercase()
                    || (*directory && folded.starts_with(&format!("{}/", root.to_lowercase())))
            }) {
                continue;
            }
            if !matches!(entry.kind.as_str(), "File" | "Directory")
                || entry.encrypted.is_none()
                || entry.encrypted == Some(true) && !authorized
                || entry.warning_codes.iter().any(|c| {
                    matches!(
                        c.as_str(),
                        "UnsafePath" | "OutputCollision" | "LinkNotExtracted"
                    )
                })
                || !ordinals.insert(entry.ordinal)
            {
                return Err(INVALID);
            }
            let raw = if entry.kind == "Directory" {
                entry.name.strip_suffix('/').unwrap_or(&entry.name)
            } else {
                &entry.name
            };
            if raw
                .split('/')
                .any(|part| !nxtools_protocol::archive_job::valid_job_name(part))
            {
                return Err(INVALID);
            }
            let name = path.strip_prefix(&prefix).ok_or(INVALID)?;
            if name.is_empty()
                || name.encode_utf16().count() >= 260
                || name
                    .split('/')
                    .any(|part| !nxtools_protocol::archive_job::valid_job_name(part))
            {
                return Err(INVALID);
            }
            let parts = name.split('/').collect::<Vec<_>>();
            for end in 1..=parts.len() {
                let name = parts[..end].join("/");
                let directory = end < parts.len() || entry.kind == "Directory";
                let ordinal = (end == parts.len()).then_some(entry.ordinal);
                let key = name.to_lowercase();
                if let Some(existing) = frozen.get_mut(&key) {
                    if existing.name != name
                        || !existing.directory
                        || !directory
                        || (ordinal.is_some() && existing.ordinal.is_some())
                    {
                        return Err(INVALID);
                    }
                    if ordinal.is_some() {
                        existing.ordinal = ordinal;
                    }
                } else {
                    frozen.insert(
                        key,
                        FrozenDragItem {
                            name,
                            directory,
                            ordinal,
                            size: if directory { None } else { entry.size },
                        },
                    );
                    if frozen.len() > ARCHIVE_DRAG_MAX_DESCRIPTORS {
                        return Err(LIMIT);
                    }
                }
            }
        }
        // A folder identity must resolve to at least an explicit empty directory or a descendant.
        for (root, directory) in roots {
            let name = root.strip_prefix(&prefix).ok_or(INVALID)?;
            if frozen
                .get(&name.to_lowercase())
                .is_none_or(|item| item.directory != directory)
            {
                return Err(INVALID);
            }
        }
        let mut files = 0usize;
        let mut total = 0u64;
        for item in frozen.values().filter(|item| !item.directory) {
            files += 1;
            total = total.checked_add(item.size.unwrap_or(0)).ok_or(LIMIT)?;
            if files > ARCHIVE_DRAG_MAX_FILES || total > ARCHIVE_DRAG_MAX_BYTES {
                return Err(LIMIT);
            }
        }
        Ok(frozen.into_values().collect())
    }
}
