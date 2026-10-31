//! Immutable-ID browse projection and its single selection owner. No IO or GPUI state.
#[path = "model_drag.rs"]
mod drag;
use nxtools_protocol::archive_browser::{ArchiveAttributes, ArchiveModifiedTime};
use nxtools_protocol::archive_read::ArchiveReadEntry;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RowId {
    Entry(Uuid, u64),
    Folder(Uuid, String),
    Parent(Uuid, String),
}
#[derive(Clone)]
pub(crate) struct Row {
    pub id: RowId,
    pub name: String,
    pub path: String,
    pub directory: bool,
    pub parent: bool,
    pub size: Option<u64>,
    pub packed_size: Option<u64>,
    pub ordinal: Option<u64>,
    pub encrypted: Option<bool>,
    pub warning: bool,
    pub kind: String,
    pub modified: Option<ArchiveModifiedTime>,
    pub method: Option<u16>,
    pub crc32: Option<u32>,
    pub attributes: Option<ArchiveAttributes>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SortKey {
    Name,
    Size,
    Packed,
    Type,
    Modified,
    Method,
    Encrypted,
    Crc,
    Attributes,
    Path,
}
pub(crate) struct BrowserModel {
    pub snapshot: Uuid,
    pub entries: Vec<ArchiveReadEntry>,
    pub folder: String,
    pub filter: String,
    pub sort: SortKey,
    pub descending: bool,
    pub selected: BTreeSet<RowId>,
    pub focused: Option<RowId>,
    pub anchor: Option<RowId>,
    pub show_parent_row: bool,
    rows: Vec<Row>,
}
impl BrowserModel {
    pub fn new(snapshot: Uuid, entries: Vec<ArchiveReadEntry>) -> Self {
        let mut m = Self {
            snapshot,
            entries,
            folder: String::new(),
            filter: String::new(),
            sort: SortKey::Name,
            descending: false,
            selected: BTreeSet::new(),
            focused: None,
            anchor: None,
            show_parent_row: true,
            rows: vec![],
        };
        m.reproject();
        m
    }
    pub fn visible(&self) -> &[Row] {
        &self.rows
    }
    pub fn folders(&self) -> Vec<String> {
        let mut dirs = BTreeSet::new();
        for entry in &self.entries {
            let parts = entry
                .name
                .trim_end_matches('/')
                .split('/')
                .collect::<Vec<_>>();
            let count = parts
                .len()
                .saturating_sub(usize::from(entry.kind != "Directory"));
            for end in 1..=count {
                let path = parts[..end].join("/");
                if !path.starts_with('/')
                    && !parts[..end].iter().any(|p| matches!(*p, "" | "." | ".."))
                {
                    dirs.insert(path);
                }
            }
        }
        dirs.into_iter().collect()
    }
    pub fn reproject(&mut self) {
        let prefix = if self.folder.is_empty() {
            String::new()
        } else {
            format!("{}/", self.folder)
        };
        let mut dirs = BTreeMap::new();
        let mut rows = vec![];
        for e in &self.entries {
            let path = e.name.trim_end_matches('/');
            let Some(rest) = path.strip_prefix(&prefix) else {
                continue;
            };
            if rest.is_empty() {
                continue;
            }
            let unsafe_name =
                rest.starts_with('/') || rest.split('/').any(|p| matches!(p, "." | ".." | ""));
            if !unsafe_name && let Some((dir, _)) = rest.split_once('/') {
                let path = format!("{prefix}{dir}");
                dirs.entry(path.clone()).or_insert(Row {
                    id: RowId::Folder(self.snapshot, path.clone()),
                    name: dir.into(),
                    path,
                    directory: true,
                    parent: false,
                    size: None,
                    packed_size: None,
                    ordinal: None,
                    encrypted: None,
                    warning: false,
                    kind: "Directory".into(),
                    modified: None,
                    method: None,
                    crc32: None,
                    attributes: None,
                });
                continue;
            }
            let directory = e.kind == "Directory";
            let row = Row {
                id: if directory {
                    RowId::Folder(self.snapshot, path.into())
                } else {
                    RowId::Entry(self.snapshot, e.ordinal)
                },
                name: rest.into(),
                path: path.into(),
                directory,
                parent: false,
                size: e.size,
                packed_size: e.packed_size,
                ordinal: Some(e.ordinal),
                encrypted: e.encrypted,
                warning: unsafe_name
                    || e.warning_codes.iter().any(|code| {
                        matches!(
                            code.as_str(),
                            "UnsafePath" | "OutputCollision" | "LinkNotExtracted"
                        )
                    }),
                kind: e.kind.clone(),
                modified: e.modified.clone(),
                method: e.method,
                crc32: e.crc32,
                attributes: e.attributes.clone(),
            };
            if directory && !unsafe_name {
                dirs.insert(path.into(), row);
            } else {
                rows.push(row)
            }
        }
        rows.extend(dirs.into_values());
        let query = self.filter.to_lowercase();
        rows.retain(|r| query.is_empty() || r.name.to_lowercase().contains(&query));
        rows.sort_by(|a, b| {
            let grouping = b.directory.cmp(&a.directory);
            if !grouping.is_eq() {
                return grouping;
            }
            let order = match self.sort {
                SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortKey::Modified => a
                    .modified
                    .as_ref()
                    .and_then(modified_key)
                    .cmp(&b.modified.as_ref().and_then(modified_key)),
                SortKey::Size => a.size.cmp(&b.size),
                SortKey::Packed => a.packed_size.cmp(&b.packed_size),
                SortKey::Type => extension(&a.name).cmp(extension(&b.name)),
                SortKey::Method => a.method.cmp(&b.method),
                SortKey::Encrypted => a.encrypted.cmp(&b.encrypted),
                SortKey::Crc => a.crc32.cmp(&b.crc32),
                SortKey::Attributes => a
                    .attributes
                    .as_ref()
                    .map(|v| (v.read_only, v.hidden))
                    .cmp(&b.attributes.as_ref().map(|v| (v.read_only, v.hidden))),
                SortKey::Path => a.path.cmp(&b.path),
            }
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.id.cmp(&b.id));
            if self.descending {
                order.reverse()
            } else {
                order
            }
        });
        if self.show_parent_row && !self.folder.is_empty() {
            rows.insert(
                0,
                Row {
                    id: RowId::Parent(self.snapshot, self.folder.clone()),
                    name: "..".into(),
                    path: parent(&self.folder),
                    directory: true,
                    parent: true,
                    size: None,
                    packed_size: None,
                    ordinal: None,
                    encrypted: None,
                    warning: false,
                    kind: "Directory".into(),
                    modified: None,
                    method: None,
                    crc32: None,
                    attributes: None,
                },
            )
        }
        let visible = rows.iter().map(|r| r.id.clone()).collect::<BTreeSet<_>>();
        self.selected.retain(|id| visible.contains(id));
        if self
            .focused
            .as_ref()
            .is_some_and(|id| !visible.contains(id))
        {
            self.focused = None
        }
        if self.anchor.as_ref().is_some_and(|id| !visible.contains(id)) {
            self.anchor = None
        }
        self.rows = rows;
    }
    pub fn enter(&mut self, path: &str) {
        self.folder = path.into();
        self.filter.clear();
        self.selected.clear();
        self.focused = None;
        self.anchor = None;
        self.reproject()
    }
    pub fn up(&mut self) {
        let path = parent(&self.folder);
        self.enter(&path)
    }
    pub fn select(&mut self, id: RowId, control: bool, shift: bool) {
        let Some(index) = self.rows.iter().position(|r| r.id == id) else {
            return;
        };
        if self.rows[index].parent {
            self.selected.clear();
            self.focused = Some(id);
            self.anchor = self.focused.clone();
            return;
        }
        if shift {
            let anchor = self
                .anchor
                .as_ref()
                .and_then(|id| self.rows.iter().position(|r| &r.id == id))
                .unwrap_or(index);
            if self.anchor.is_none() {
                self.anchor = Some(self.rows[anchor].id.clone());
            }
            if !control {
                self.selected.clear()
            }
            for row in &self.rows[anchor.min(index)..=anchor.max(index)] {
                if !row.parent {
                    self.selected.insert(row.id.clone());
                }
            }
        } else {
            if !control {
                self.selected.clear()
            }
            if control && self.selected.contains(&id) {
                self.selected.remove(&id);
            } else {
                self.selected.insert(id.clone());
            }
            self.anchor = Some(id.clone())
        }
        self.focused = Some(id)
    }
    pub fn right_select(&mut self, id: RowId) {
        if !self.selected.contains(&id) {
            self.select(id, false, false)
        } else {
            self.focused = Some(id)
        }
    }
    pub fn move_focus(&mut self, delta: isize, shift: bool) {
        if self.rows.is_empty() {
            return;
        }
        let current = self
            .focused
            .as_ref()
            .and_then(|id| self.rows.iter().position(|r| &r.id == id))
            .unwrap_or_else(|| if delta < 0 { self.rows.len() - 1 } else { 0 });
        let index = if self.focused.is_none() {
            current
        } else {
            current
                .saturating_add_signed(delta)
                .min(self.rows.len() - 1)
        };
        let id = self.rows[index].id.clone();
        self.select(id, false, shift)
    }
    pub fn select_all(&mut self) {
        self.selected = self
            .rows
            .iter()
            .filter(|r| !r.parent)
            .map(|r| r.id.clone())
            .collect();
    }
    pub fn selected_file_ordinals(&self) -> Vec<u64> {
        self.rows
            .iter()
            .filter(|r| self.selected.contains(&r.id) && !r.directory)
            .filter_map(|r| r.ordinal)
            .collect()
    }
}
/// A comparable value, not a claim that timezone-less DOS stamps are UTC.
pub(crate) fn modified_key(value: &ArchiveModifiedTime) -> Option<(i64, u32)> {
    match value {
        ArchiveModifiedTime::UnixUtc {
            seconds,
            nanoseconds,
            ..
        } => chrono::DateTime::from_timestamp(*seconds, *nanoseconds)
            .map(|v| (v.timestamp(), v.timestamp_subsec_nanos())),
        ArchiveModifiedTime::FiletimeUtc { ticks } => {
            let nanos = i128::from(*ticks) * 100 - 11_644_473_600i128 * 1_000_000_000;
            let seconds = i64::try_from(nanos.div_euclid(1_000_000_000)).ok()?;
            Some((seconds, nanos.rem_euclid(1_000_000_000) as u32))
        }
        ArchiveModifiedTime::DosLocal { date, time } => {
            let day = chrono::NaiveDate::from_ymd_opt(
                1980 + i32::from(date >> 9),
                u32::from((date >> 5) & 15),
                u32::from(date & 31),
            )?;
            let stamp = day.and_hms_opt(
                u32::from(time >> 11),
                u32::from((time >> 5) & 63),
                u32::from(time & 31) * 2,
            )?;
            Some((stamp.and_utc().timestamp(), 0))
        }
        ArchiveModifiedTime::Invalid => None,
    }
}
fn extension(name: &str) -> &str {
    name.rsplit_once('.').map_or("", |(_, ext)| ext)
}
fn parent(path: &str) -> String {
    path.rsplit_once('/')
        .map_or_else(String::new, |(p, _)| p.into())
}
#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
