//! Immutable extraction policy. No filesystem/native operations or user-data side effects.
use crate::{ArchiveError, ArchiveSnapshot, EntryId, EntryKind, EntryWarning, Result};
use std::collections::{BTreeSet, HashMap, HashSet};

pub enum ExtractionSelection {
    All,
    Entries(Vec<EntryId>),
}
pub enum TargetStrategy {
    Here,
    Name(String),
    Smart {
        archive_stem: String,
        single_file_folder: bool,
    },
}
#[derive(Clone)]
pub struct PlannedFile {
    id: EntryId,
    relative: String,
    size: Option<u64>,
}
impl PlannedFile {
    pub fn id(&self) -> EntryId {
        self.id
    }
    pub fn relative_path(&self) -> &str {
        &self.relative
    }
    pub fn size(&self) -> Option<u64> {
        self.size
    }
}
pub struct ExtractionPlan {
    files: Vec<PlannedFile>,
    directories: Vec<String>,
    pub(crate) explicit_directories: Vec<EntryId>,
    subfolder: Option<String>,
    total: Option<u64>,
    full_archive: bool,
    skipped_special: u64,
    pub(crate) session: uuid::Uuid,
    /// Share, do not copy, the immutable name projection this plan validated.
    pub(crate) projection: std::sync::Arc<[crate::ArchiveEntry]>,
    pub(crate) isolated_browser_entry: bool,
}
impl ExtractionPlan {
    /// One original ordinal into a fresh private cache lease, never a general extraction target.
    pub(crate) fn browser_entry(
        snapshot: &ArchiveSnapshot,
        entry: &crate::ArchiveEntry,
    ) -> Result<Self> {
        if entry.kind != EntryKind::File || entry.warnings.contains(&EntryWarning::UnsafePath) {
            return Err(ArchiveError::UnsafePath);
        }
        let relative = entry.display_path.replace('\\', "/");
        crate::path_policy::validate_relative(&relative)?;
        let parts = relative.split('/').collect::<Vec<_>>();
        Ok(Self {
            files: vec![PlannedFile {
                id: entry.id,
                relative: relative.clone(),
                size: entry.size,
            }],
            directories: (1..parts.len()).map(|end| parts[..end].join("/")).collect(),
            explicit_directories: Vec::new(),
            subfolder: None,
            total: entry.size,
            full_archive: false,
            skipped_special: 0,
            session: snapshot.session,
            projection: snapshot.entries.clone(),
            isolated_browser_entry: true,
        })
    }
    pub(crate) fn replace_subfolder(&mut self, name: String) -> Result<()> {
        self.subfolder = Some(validate_stem(name)?);
        Ok(())
    }
    pub fn files(&self) -> &[PlannedFile] {
        &self.files
    }
    pub fn directories(&self) -> &[String] {
        &self.directories
    }
    pub fn subfolder(&self) -> Option<&str> {
        self.subfolder.as_deref()
    }
    pub fn total_bytes(&self) -> Option<u64> {
        self.total
    }
    pub fn full_archive(&self) -> bool {
        self.full_archive
    }
    pub fn skipped_special(&self) -> u64 {
        self.skipped_special
    }
    /// Structural precondition only: callers still require full successful commit/test,
    /// per-job consent, source identity and native Recycle Bin outcomes.
    pub fn permits_source_delete_request(&self) -> bool {
        self.full_archive && self.skipped_special == 0 && !self.files.is_empty()
    }
    pub fn build(
        snapshot: &ArchiveSnapshot,
        selection: ExtractionSelection,
        strategy: TargetStrategy,
    ) -> Result<Self> {
        let full_archive = matches!(selection, ExtractionSelection::All);
        let mut selected = HashSet::new();
        if let ExtractionSelection::Entries(ids) = selection {
            let by_id: HashMap<_, _> = snapshot
                .entries
                .iter()
                .map(|entry| (entry.id, entry))
                .collect();
            if ids.len() > snapshot.entries.len() {
                return Err(ArchiveError::SourceChanged);
            }
            for id in ids {
                let entry = by_id.get(&id).ok_or(ArchiveError::SourceChanged)?;
                if id.session != snapshot.session {
                    return Err(ArchiveError::SourceChanged);
                }
                selected.insert(id);
                if entry.kind == EntryKind::Directory {
                    let prefix = format!(
                        "{}/",
                        entry.display_path.replace('\\', "/").trim_end_matches('/')
                    );
                    for candidate in snapshot.entries.iter() {
                        if candidate
                            .display_path
                            .replace('\\', "/")
                            .starts_with(&prefix)
                        {
                            selected.insert(candidate.id);
                        }
                    }
                }
            }
        }
        let mut files = Vec::new();
        let mut directories = BTreeSet::new();
        let mut explicit_directories = Vec::new();
        let mut roots = BTreeSet::new();
        let mut total = Some(0u64);
        let mut skipped_special = 0;
        for entry in snapshot
            .entries
            .iter()
            .filter(|entry| full_archive || selected.contains(&entry.id))
        {
            if entry.warnings.contains(&EntryWarning::UnsafePath) {
                return Err(ArchiveError::UnsafePath);
            }
            if entry.warnings.contains(&EntryWarning::OutputCollision) {
                return Err(ArchiveError::OutputCollision);
            }
            if matches!(entry.kind, EntryKind::Link | EntryKind::Special) {
                skipped_special += 1;
                continue;
            }
            let relative = entry.display_path.replace('\\', "/");
            let relative = relative.trim_end_matches('/');
            crate::path_policy::validate_relative(relative)?;
            let parts: Vec<_> = relative.split('/').collect();
            roots.insert(parts[0].to_owned());
            for count in 1..parts.len() {
                directories.insert(parts[..count].join("/"));
            }
            if entry.kind == EntryKind::Directory {
                explicit_directories.push(entry.id);
                directories.insert(relative.to_owned());
                continue;
            }
            total = match (total, entry.size) {
                (Some(total), Some(size)) => {
                    Some(total.checked_add(size).ok_or(ArchiveError::ResourceLimit)?)
                }
                _ => None,
            };
            files.push(PlannedFile {
                id: entry.id,
                relative: relative.to_owned(),
                size: entry.size,
            });
        }
        let empty = files.is_empty() && directories.is_empty();
        let subfolder = if empty {
            None
        } else {
            match strategy {
                TargetStrategy::Here => None,
                TargetStrategy::Name(name) => Some(validate_stem(name)?),
                TargetStrategy::Smart {
                    archive_stem,
                    single_file_folder,
                } => {
                    let one_directory = roots.len() == 1
                        && directories.contains(roots.iter().next().expect("one root"));
                    if one_directory || roots.len() == 1 && !single_file_folder {
                        None
                    } else {
                        Some(validate_stem(archive_stem)?)
                    }
                }
            }
        };
        Ok(Self {
            files,
            directories: directories.into_iter().collect(),
            explicit_directories,
            subfolder,
            total,
            full_archive,
            skipped_special,
            session: snapshot.session,
            projection: snapshot.entries.clone(),
            isolated_browser_entry: false,
        })
    }
}
fn validate_stem(name: String) -> Result<String> {
    crate::path_policy::validate_relative(&name)?;
    if name.contains(['/', '\\']) {
        return Err(ArchiveError::UnsafePath);
    }
    Ok(name)
}
