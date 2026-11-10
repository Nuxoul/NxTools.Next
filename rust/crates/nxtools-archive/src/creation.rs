//! Bounded creation policy. Native source capabilities own object identity and actual reads.
use crate::{ArchiveError, OperationControl, ResourceLimits, Result, path_policy};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

#[derive(Clone)]
pub struct SourceEntry {
    pub id: Uuid,
    pub name: String,
    pub directory: bool,
    pub size: u64,
    /// DOS local wall-clock date/time at two-second precision; absent means unavailable.
    pub dos_modified: Option<(u16, u16)>,
}
/// Operation-scoped native source set. Implementations must bind all reads to the observed
/// no-reparse object, use finite handles, and reject identity/version/member changes.
pub trait CreationSource {
    fn roots(&mut self) -> Result<Vec<SourceEntry>>;
    /// One sequential child at a time; None ends this directory. No interleaved enumerators.
    fn next_child(&mut self, directory: Uuid, offset: u64) -> Result<Option<SourceEntry>>;
    fn open_file(&mut self, id: Uuid) -> Result<()>;
    fn read_file(&mut self, id: Uuid, offset: u64, length: u32) -> Result<Vec<u8>>;
    fn close_file(&mut self, id: Uuid) -> Result<()>;
    fn revalidate(&mut self) -> Result<()>;
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreationOptions {
    pub strip_single_root: bool,
    /// Slash-containing patterns match archive-relative paths; others match basenames.
    /// Windows-style case-insensitive globs, literal slash separators, no negation syntax.
    pub exclude_patterns: Vec<String>,
}
impl Default for CreationOptions {
    fn default() -> Self {
        Self {
            strip_single_root: false,
            exclude_patterns: vec!["Thumbs.db".into(), "desktop.ini".into()],
        }
    }
}
pub struct PlannedSource {
    pub(crate) source: SourceEntry,
    relative: String,
}
impl PlannedSource {
    pub fn relative_path(&self) -> &str {
        &self.relative
    }
    pub fn is_directory(&self) -> bool {
        self.source.directory
    }
    /// Archive payload length, not a native directory's physical index/allocation size.
    pub fn size(&self) -> u64 {
        if self.is_directory() {
            0
        } else {
            self.source.size
        }
    }
}
pub struct CreationPlan {
    pub(crate) entries: Vec<PlannedSource>,
    total: u64,
    excluded: u64,
    roots: Vec<(Uuid, bool)>,
}
/// Add retains every protected input until the worker resolves the complete namespace.
pub(crate) struct IncomingPlan {
    pub entries: Vec<PlannedSource>,
}
impl IncomingPlan {
    pub fn build(
        source: &mut impl CreationSource,
        limits: &ResourceLimits,
        control: &OperationControl,
    ) -> Result<Self> {
        let plan = CreationPlan::collect(
            source,
            &CreationOptions {
                strip_single_root: false,
                exclude_patterns: Vec::new(),
            },
            limits,
            control,
            None,
        )?;
        Ok(Self {
            entries: plan.entries,
        })
    }
}
impl CreationPlan {
    pub fn entries(&self) -> &[PlannedSource] {
        &self.entries
    }
    pub fn total_bytes(&self) -> u64 {
        self.total
    }
    pub fn excluded_entries(&self) -> u64 {
        self.excluded
    }
    /// Whole original roots only. Directory traversal is not an atomic covered snapshot.
    pub(crate) fn record_published_verification(
        &self,
        control: &OperationControl,
        lifecycle: &crate::source_deletion::Lifecycle,
    ) {
        let reason = if self.excluded != 0 {
            "members_excluded"
        } else if self.roots.iter().any(|(_, directory)| *directory) {
            "directory_coverage_unproven"
        } else if self.entries.is_empty() {
            "empty_source_set"
        } else if self.entries.len() != self.roots.len() {
            "coverage_incomplete"
        } else {
            "eligible"
        };
        control.source_delete_finish(
            lifecycle,
            reason,
            self.roots.iter().map(|(id, _)| *id).collect(),
        );
    }
    pub fn build(
        source: &mut impl CreationSource,
        options: &CreationOptions,
        limits: &ResourceLimits,
        control: &OperationControl,
    ) -> Result<Self> {
        Self::collect(source, options, limits, control, Some(HashMap::new()))
    }
    fn collect(
        source: &mut impl CreationSource,
        options: &CreationOptions,
        limits: &ResourceLimits,
        control: &OperationControl,
        mut names: Option<HashMap<String, (String, bool)>>,
    ) -> Result<Self> {
        let filters = Filters::new(options, limits)?;
        control.checkpoint()?;
        let roots = source.roots()?;
        if roots.is_empty() || roots.len() > 32 {
            return Err(ArchiveError::ResourceLimit);
        }
        let strip = options.strip_single_root && roots.len() == 1 && roots[0].directory;
        let mut plan = Self {
            entries: Vec::new(),
            total: 0,
            excluded: 0,
            roots: roots.iter().map(|root| (root.id, root.directory)).collect(),
        };
        let mut queue = VecDeque::new();
        let mut seen = HashSet::new();
        let mut metadata = 4 * 1024 * 1024u64; // bounded glob compiler/matchers and queue reservation
        let mut observed = 0u64;
        for root in roots {
            let prefix = if strip {
                String::new()
            } else {
                root.name.clone()
            };
            admit(
                &root,
                &prefix,
                limits,
                &mut metadata,
                &mut observed,
                &mut seen,
            )?;
            if !strip && filters.matches(&root.name, &prefix) {
                plan.excluded += 1;
                continue;
            }
            if !strip {
                plan.push(root.clone(), prefix.clone(), limits, names.as_mut())?;
            }
            if root.directory {
                queue.push_back((root.id, prefix, 1usize));
            }
        }
        while let Some((directory, prefix, depth)) = queue.pop_front() {
            control.checkpoint()?;
            let mut offset = 0u64;
            while let Some(child) = source.next_child(directory, offset)? {
                control.checkpoint()?;
                offset = offset.checked_add(1).ok_or(ArchiveError::ResourceLimit)?;
                let relative = if prefix.is_empty() {
                    child.name.clone()
                } else {
                    format!("{prefix}/{}", child.name)
                };
                admit(
                    &child,
                    &relative,
                    limits,
                    &mut metadata,
                    &mut observed,
                    &mut seen,
                )?;
                if filters.matches(&child.name, &relative) {
                    plan.excluded += 1;
                    continue;
                }
                if child.directory {
                    if depth >= 64 {
                        return Err(ArchiveError::ResourceLimit);
                    }
                    queue.push_back((child.id, relative.clone(), depth + 1));
                }
                plan.push(child, relative, limits, names.as_mut())?;
            }
        }
        source.revalidate()?;
        Ok(plan)
    }
    fn push(
        &mut self,
        source: SourceEntry,
        mut relative: String,
        limits: &ResourceLimits,
        names: Option<&mut HashMap<String, (String, bool)>>,
    ) -> Result<()> {
        if let Some(names) = names {
            let mut prefix = String::new();
            let components: Vec<_> = relative.split('/').collect();
            for (index, component) in components.iter().enumerate() {
                if !prefix.is_empty() {
                    prefix.push('/');
                }
                prefix.push_str(component);
                let directory = index + 1 != components.len() || source.directory;
                let key = prefix.to_uppercase();
                if let Some((spelling, previous_directory)) = names.get(&key) {
                    if spelling != &prefix
                        || !directory
                        || !previous_directory
                        || index + 1 == components.len()
                    {
                        return Err(ArchiveError::OutputCollision);
                    }
                } else {
                    names.insert(key, (prefix.clone(), directory));
                }
            }
        }
        if source.directory {
            relative.push('/');
        } else {
            self.total = self
                .total
                .checked_add(source.size)
                .ok_or(ArchiveError::ResourceLimit)?;
            if source.size > limits.max_entry_bytes || self.total > limits.max_output_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
        }
        self.entries.push(PlannedSource { source, relative });
        Ok(())
    }
}
fn admit(
    entry: &SourceEntry,
    relative: &str,
    limits: &ResourceLimits,
    metadata: &mut u64,
    count: &mut u64,
    seen: &mut HashSet<Uuid>,
) -> Result<()> {
    if entry.id.is_nil() || entry.name.contains(['/', '\\']) {
        return Err(ArchiveError::UnsafePath);
    }
    path_policy::validate_relative(&entry.name)?;
    if !relative.is_empty() {
        path_policy::validate_relative(relative)?;
    }
    if entry.name.encode_utf16().count() > 255 || relative.len() > 32767 {
        return Err(ArchiveError::ResourceLimit);
    }
    *count = count.checked_add(1).ok_or(ArchiveError::ResourceLimit)?;
    let depth = relative.split('/').count() as u64;
    *metadata = metadata
        .checked_add(2048 + relative.len() as u64 * (24 + depth * 8) + entry.name.len() as u64 * 8)
        .ok_or(ArchiveError::ResourceLimit)?;
    if *count > limits.max_entries || *metadata > limits.max_metadata_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    if !seen.insert(entry.id) {
        return Err(ArchiveError::SourceChanged);
    }
    Ok(())
}
struct Filters {
    path: GlobSet,
    basename: GlobSet,
}
impl Filters {
    fn new(options: &CreationOptions, limits: &ResourceLimits) -> Result<Self> {
        if options.exclude_patterns.len() > 32
            || options
                .exclude_patterns
                .iter()
                .map(String::len)
                .sum::<usize>()
                > 8192
            || limits.max_working_bytes < 64 << 20
        {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut path = GlobSetBuilder::new();
        let mut basename = GlobSetBuilder::new();
        for pattern in &options.exclude_patterns {
            if pattern.is_empty()
                || pattern.len() > 1024
                || pattern.contains(['\\', '\0'])
                || pattern.starts_with(['/', '!'])
                || pattern.split('/').any(|part| matches!(part, "." | ".."))
            {
                return Err(ArchiveError::UnsafePath);
            }
            let glob = GlobBuilder::new(pattern)
                .literal_separator(true)
                .case_insensitive(true)
                .build()
                .map_err(|_| ArchiveError::UnsafePath)?;
            if pattern.contains('/') {
                path.add(glob);
            } else {
                basename.add(glob);
            }
        }
        Ok(Self {
            path: path.build().map_err(|_| ArchiveError::ResourceLimit)?,
            basename: basename.build().map_err(|_| ArchiveError::ResourceLimit)?,
        })
    }
    fn matches(&self, name: &str, relative: &str) -> bool {
        self.basename.is_match(name) || self.path.is_match(relative)
    }
}
