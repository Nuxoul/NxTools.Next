//! Immutable extraction policy. No filesystem/native operations or user-data side effects.
use crate::{ArchiveError, ArchiveSnapshot, EntryId, EntryKind, EntryWarning, Result};
use std::collections::BTreeSet;
#[path = "planning_selection.rs"]
mod selection;
pub use selection::{
    CatalogSelection, SelectionEntry, bind_ordinals, expand_selection, matches_directory_prefixes,
    merge_directory_prefixes,
};

/// Aggregate work at an operation's original selection boundary; contains no member data.
/// Catalog visits/normalizations cover membership expansion, not output or collision validation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SelectionWork {
    pub catalog_visits: u64,
    /// Catalog path normalization calls; selected roots and output projection are separate.
    pub path_normalizations: u64,
    pub root_normalizations: u64,
    pub projection_normalizations: u64,
    /// Owned normalization results, not allocator calls or retained bytes.
    pub normalization_allocations: u64,
    /// Directory roots prepared for membership matching (after any prefix merging).
    pub directory_roots: u64,
    /// Component-prefix comparisons executed by membership expansion.
    pub prefix_probes: u64,
    /// Slash-ended membership-prefix strings constructed by expansion.
    pub prefix_allocations: u64,
    /// Catalog records inspected while binding/comparing original IDs, including index construction.
    pub ordinal_probes: u64,
    /// Output-plan traversal, distinct from selection expansion.
    pub plan_visits: u64,
    /// Original metadata/projection records inspected for coexistence accounting.
    #[serde(default)]
    pub metadata_visits: u64,
}

impl std::ops::AddAssign for SelectionWork {
    fn add_assign(&mut self, other: Self) {
        self.catalog_visits += other.catalog_visits;
        self.path_normalizations += other.path_normalizations;
        self.root_normalizations += other.root_normalizations;
        self.projection_normalizations += other.projection_normalizations;
        self.normalization_allocations += other.normalization_allocations;
        self.directory_roots += other.directory_roots;
        self.prefix_probes += other.prefix_probes;
        self.prefix_allocations += other.prefix_allocations;
        self.ordinal_probes += other.ordinal_probes;
        self.plan_visits += other.plan_visits;
        self.metadata_visits += other.metadata_visits;
    }
}

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
    output_prefix: Option<String>,
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
    /// Retained catalog and extraction projection coexist with every parallel lane.
    pub(crate) fn retained_working_bytes(&self) -> u64 {
        let catalog = self
            .projection
            .iter()
            .map(|entry| {
                std::mem::size_of::<crate::ArchiveEntry>() as u64
                    + entry.raw_name.capacity() as u64
                    + entry.display_path.capacity() as u64
                    + (entry.warnings.capacity() * std::mem::size_of::<EntryWarning>()) as u64
            })
            .sum::<u64>();
        catalog
            + (self.files.capacity() * std::mem::size_of::<PlannedFile>()) as u64
            + self
                .files
                .iter()
                .map(|file| file.relative.capacity() as u64)
                .sum::<u64>()
            + (self.directories.capacity() * std::mem::size_of::<String>()) as u64
            + self
                .directories
                .iter()
                .map(|directory| directory.capacity() as u64)
                .sum::<u64>()
            + (self.explicit_directories.capacity() * std::mem::size_of::<EntryId>()) as u64
            + self
                .subfolder
                .as_ref()
                .map_or(0, |name| name.capacity() as u64)
            + self
                .output_prefix
                .as_ref()
                .map_or(0, |prefix| prefix.capacity() as u64)
    }
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
            output_prefix: None,
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
    /// Project only output names; source IDs and the immutable catalog stay unchanged.
    pub fn strip_output_prefix(&mut self, prefix: &str) -> Result<()> {
        let root = prefix.strip_suffix('/').ok_or(ArchiveError::UnsafePath)?;
        crate::path_policy::validate_relative(root)?;
        for file in &self.files {
            let relative = file
                .relative
                .strip_prefix(prefix)
                .ok_or(ArchiveError::SourceChanged)?;
            crate::path_policy::validate_relative(relative)?;
        }
        for id in &self.explicit_directories {
            let entry = self
                .projection
                .get(id.ordinal as usize)
                .filter(|entry| entry.id == *id)
                .ok_or(ArchiveError::SourceChanged)?;
            let name = entry.display_path.replace('\\', "/");
            let name = name.trim_end_matches('/');
            if name != root && !name.starts_with(prefix) {
                return Err(ArchiveError::SourceChanged);
            }
        }
        for file in &mut self.files {
            file.relative = file.relative[prefix.len()..].to_owned();
        }
        self.directories = self
            .directories
            .iter()
            .filter_map(|name| name.strip_prefix(prefix).map(str::to_owned))
            .collect();
        self.output_prefix = Some(prefix.to_owned());
        Ok(())
    }
    /// The receiver owns the exact selected root; it has no child metadata path.
    pub(crate) fn directory_output_path(
        &self,
        entry: &crate::ArchiveEntry,
    ) -> Result<Option<String>> {
        let name = entry.display_path.replace('\\', "/");
        let name = name.trim_end_matches('/');
        let name = if let Some(prefix) = &self.output_prefix {
            if name == prefix.trim_end_matches('/') {
                return Ok(None);
            }
            name.strip_prefix(prefix)
                .ok_or(ArchiveError::SourceChanged)?
        } else {
            name
        };
        Ok(Some(self.subfolder.as_ref().map_or_else(
            || name.to_owned(),
            |root| format!("{root}/{name}"),
        )))
    }
    pub fn files(&self) -> &[PlannedFile] {
        &self.files
    }
    #[cfg(feature = "isolated-codecs")]
    pub(crate) fn original_file_path(&self, file: &PlannedFile) -> Result<String> {
        if self.output_prefix.is_none() {
            return Ok(file.relative.clone());
        }
        let entry = self
            .projection
            .get(file.id.ordinal as usize)
            .filter(|entry| entry.id == file.id)
            .ok_or(ArchiveError::SourceChanged)?;
        Ok(entry.display_path.replace('\\', "/"))
    }
    #[cfg(feature = "isolated-codecs")]
    pub(crate) fn original_directory_path(&self, relative: &str) -> String {
        self.output_prefix.as_ref().map_or_else(
            || relative.to_owned(),
            |prefix| format!("{prefix}{relative}"),
        )
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
        Self::build_observed(snapshot, selection, strategy, &mut SelectionWork::default())
    }
    /// Same planning boundary with operation-local aggregate observation, including failed work.
    pub fn build_observed(
        snapshot: &ArchiveSnapshot,
        selection: ExtractionSelection,
        strategy: TargetStrategy,
        work: &mut SelectionWork,
    ) -> Result<Self> {
        Self::build_observed_with_limits(
            snapshot,
            selection,
            strategy,
            &crate::ResourceLimits::default(),
            work,
        )
    }
    /// Caller-supplied operation metadata allowance includes the original immutable projection,
    /// selection scratch and the output plan while they coexist.
    pub fn build_observed_with_limits(
        snapshot: &ArchiveSnapshot,
        selection: ExtractionSelection,
        strategy: TargetStrategy,
        limits: &crate::ResourceLimits,
        work: &mut SelectionWork,
    ) -> Result<Self> {
        let full_archive = matches!(selection, ExtractionSelection::All);
        let original_bytes = snapshot.entries.iter().try_fold(0u64, |bytes, entry| {
            work.metadata_visits += 1;
            bytes
                .checked_add(std::mem::size_of::<crate::ArchiveEntry>() as u64)
                .and_then(|bytes| bytes.checked_add(entry.raw_name.len() as u64))
                .and_then(|bytes| bytes.checked_add(entry.display_path.len() as u64))
                .and_then(|bytes| {
                    bytes.checked_add(
                        (entry.warnings.len() * std::mem::size_of::<EntryWarning>()) as u64,
                    )
                })
                .ok_or(ArchiveError::ResourceLimit)
        })?;
        let credit = limits
            .max_metadata_bytes
            .checked_sub(original_bytes)
            .ok_or(ArchiveError::ResourceLimit)?
            .min(limits.max_working_bytes);
        let positions = if let ExtractionSelection::Entries(ids) = selection {
            if ids.len() > snapshot.entries.len()
                || ids.iter().any(|id| id.session != snapshot.session)
            {
                return Err(ArchiveError::SourceChanged);
            }
            let ordinals = ids.iter().map(|id| id.ordinal).collect::<Vec<_>>();
            let positions = bind_ordinals(
                &snapshot.entries,
                &ordinals,
                credit,
                |entry| entry.id.ordinal,
                || Ok(()),
                work,
            )?;
            if positions
                .iter()
                .zip(&ids)
                .any(|(&position, id)| snapshot.entries[position].id != *id)
            {
                return Err(ArchiveError::SourceChanged);
            }
            positions
        } else {
            Vec::new()
        };
        let mut files = Vec::new();
        let mut directories = BTreeSet::new();
        let mut explicit_directories = Vec::new();
        let mut roots = BTreeSet::new();
        let mut total = Some(0u64);
        let mut skipped_special = 0;
        let mut plan_bytes = 0u64;
        let mut collision_candidates = Vec::new();
        let mut projection_work = SelectionWork::default();
        let result = expand_selection(
            &snapshot.entries,
            if full_archive {
                CatalogSelection::All
            } else {
                CatalogSelection::Entries {
                    positions: &positions,
                    directories: &[],
                }
            },
            credit,
            |_, entry| SelectionEntry {
                ordinal: entry.id.ordinal,
                path: &entry.display_path,
                directory: entry.kind == EntryKind::Directory,
            },
            |_, entry, normalized, reservation| {
                projection_work.plan_visits += 1;
                if entry.warnings.contains(&EntryWarning::UnsafePath) {
                    return Err(ArchiveError::UnsafePath);
                }
                if entry.warnings.contains(&EntryWarning::OutputCollision) {
                    if full_archive {
                        return Err(ArchiveError::OutputCollision);
                    }
                    if matches!(entry.kind, EntryKind::File | EntryKind::Directory) {
                        reserve_plan(
                            &mut plan_bytes,
                            2 * std::mem::size_of::<crate::ArchiveEntry>() as u64
                                + entry.raw_name.len() as u64
                                + entry.display_path.len() as u64
                                + (entry.warnings.len() * std::mem::size_of::<EntryWarning>())
                                    as u64,
                            credit,
                            reservation,
                        )?;
                        let mut candidate = entry.clone();
                        candidate
                            .warnings
                            .retain(|warning| *warning != EntryWarning::OutputCollision);
                        collision_candidates.push(candidate);
                    }
                }
                if matches!(entry.kind, EntryKind::Link | EntryKind::Special) {
                    skipped_special += 1;
                    return Ok(plan_bytes);
                }
                let relative = normalized.trim_end_matches('/');
                crate::path_policy::validate_relative(relative)?;
                let parts: Vec<_> = relative.split('/').collect();
                if !roots.contains(parts[0]) {
                    reserve_plan(
                        &mut plan_bytes,
                        96 + parts[0].len() as u64,
                        credit,
                        reservation,
                    )?;
                    roots.insert(parts[0].to_owned());
                }
                for count in 1..=parts.len() {
                    if count == parts.len() && entry.kind != EntryKind::Directory {
                        break;
                    }
                    let name = parts[..count].join("/");
                    if !directories.contains(&name) {
                        reserve_plan(&mut plan_bytes, 96 + name.len() as u64, credit, reservation)?;
                        directories.insert(name);
                    }
                }
                if entry.kind == EntryKind::Directory {
                    reserve_plan(
                        &mut plan_bytes,
                        2 * std::mem::size_of::<EntryId>() as u64,
                        credit,
                        reservation,
                    )?;
                    explicit_directories.push(entry.id);
                    return Ok(plan_bytes);
                }
                total = match (total, entry.size) {
                    (Some(total), Some(size)) => {
                        Some(total.checked_add(size).ok_or(ArchiveError::ResourceLimit)?)
                    }
                    _ => None,
                };
                reserve_plan(
                    &mut plan_bytes,
                    2 * std::mem::size_of::<PlannedFile>() as u64 + relative.len() as u64,
                    credit,
                    reservation,
                )?;
                files.push(PlannedFile {
                    id: entry.id,
                    relative: relative.to_owned(),
                    size: entry.size,
                });
                Ok(plan_bytes)
            },
            || Ok(()),
            work,
        );
        *work += projection_work;
        result?;
        if !collision_candidates.is_empty() {
            crate::path_policy::annotate(
                &mut collision_candidates,
                &crate::OperationControl::with_operation_id(snapshot.session),
                plan_bytes,
                credit,
            )?;
            if collision_candidates
                .iter()
                .any(|entry| entry.warnings.contains(&EntryWarning::OutputCollision))
            {
                return Err(ArchiveError::OutputCollision);
            }
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
            output_prefix: None,
            full_archive,
            skipped_special,
            session: snapshot.session,
            projection: snapshot.entries.clone(),
            isolated_browser_entry: false,
        })
    }
}
fn reserve_plan(bytes: &mut u64, additional: u64, credit: u64, selection: u64) -> Result<()> {
    *bytes = bytes
        .checked_add(additional)
        .ok_or(ArchiveError::ResourceLimit)?;
    if bytes
        .checked_add(selection)
        .is_none_or(|total| total > credit)
    {
        return Err(ArchiveError::ResourceLimit);
    }
    Ok(())
}

fn validate_stem(name: String) -> Result<String> {
    crate::path_policy::validate_relative(&name)?;
    if name.contains(['/', '\\']) {
        return Err(ArchiveError::UnsafePath);
    }
    Ok(name)
}

#[cfg(test)]
#[path = "planning_selection_tests.rs"]
mod selection_tests;
