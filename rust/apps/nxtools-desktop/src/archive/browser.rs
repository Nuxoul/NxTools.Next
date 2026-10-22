//! Phase C production browser. Component ledger: official Tree + h_resizable (200/160..280),
//! ListItem.selected + public uniform_list (approved details-only exception), one BrowserModel
//! selection/focus/anchor. Toolbar.content deliberately bypasses child ghost/compact adaptation:
//! real Medium outline Buttons. Default Kit palette/type/radius/motion; no application timers.
//! Per-window PopupMenu, stable InputState, StatusBar, Alert and Empty retain their styled owners.
//! The view additionally owns path-free presentation snapshots; ArchiveApplication keeps native
//! read/window/consent/temp/Shell lifecycles and schedules independent state IO.
#[path = "ui_state.rs"]
pub(crate) mod ui_state;
use super::{
    ArchiveApplication,
    model::{self, BrowserModel, Row, RowId, SortKey},
};
use gpui_kit::base::Selectable as _;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::{
    ActiveTheme, Disableable, Sizable, Size, TitleBar, WindowExt,
    alert::Alert,
    button::{Button, ButtonVariants},
    empty::{Empty, EmptyDescription, EmptyHeader, EmptyTitle},
    input::{Input, InputEvent, InputState},
    list::ListItem,
    menu::{ContextMenuExt, DropdownMenu, PopupMenu, PopupMenuItem},
    resizable::{h_resizable, resizable_panel},
    scroll::ScrollableElement,
    status_bar::StatusBar,
    toolbar::Toolbar,
    tree::{Tree, TreeItem, TreeState},
};
use gpui_kit::{
    AnyElement, AnyWindowHandle, App, AppContext, Context, Entity, ExternalPaths, FocusHandle,
    Focusable, IntoElement, KeyDownEvent, MouseButton, ParentElement, Pixels, Render, Role,
    ScrollHandle, ScrollStrategy, Subscription, UniformListScrollHandle, WeakEntity, Window, div,
    prelude::*, px, uniform_list,
};
use nxtools_protocol::archive_job::ArchiveJobModifyAction;
use nxtools_protocol::{archive_browser::ArchiveModifiedTime, archive_read::ArchiveReadEntry};
use std::{collections::BTreeMap, ops::Range, path::PathBuf};
use ui_state::{UiGeometry, UiSortKey, UiState};
use uuid::Uuid;

const ROW_HEIGHT: f32 = 32.;
const CODEPAGES: &[(u32, &str)] = &[
    (0, "自动检测"),
    (65001, "UTF-8"),
    (936, "GBK (936)"),
    (950, "Big5 (950)"),
    (932, "Shift-JIS (932)"),
    (949, "Korean (949)"),
    (437, "CP437"),
    (1250, "CP1250"),
    (1251, "CP1251"),
    (1252, "CP1252"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BrowserStatus {
    Loading,
    Ready,
    Cancelled,
    Failed(String),
    Stale(String),
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ColumnKind {
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
impl ColumnKind {
    const ALL: [Self; 10] = [
        Self::Name,
        Self::Size,
        Self::Packed,
        Self::Type,
        Self::Modified,
        Self::Method,
        Self::Encrypted,
        Self::Crc,
        Self::Attributes,
        Self::Path,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::Name => "名称",
            Self::Size => "原始大小",
            Self::Packed => "压缩后大小",
            Self::Type => "类型",
            Self::Modified => "修改日期",
            Self::Method => "压缩方法",
            Self::Encrypted => "加密",
            Self::Crc => "CRC",
            Self::Attributes => "属性",
            Self::Path => "路径",
        }
    }
    fn sort(self) -> SortKey {
        match self {
            Self::Name => SortKey::Name,
            Self::Size => SortKey::Size,
            Self::Packed => SortKey::Packed,
            Self::Type => SortKey::Type,
            Self::Modified => SortKey::Modified,
            Self::Method => SortKey::Method,
            Self::Encrypted => SortKey::Encrypted,
            Self::Crc => SortKey::Crc,
            Self::Attributes => SortKey::Attributes,
            Self::Path => SortKey::Path,
        }
    }
    fn index(self) -> usize {
        self as usize
    }
}
#[derive(Clone)]
enum Command {
    Pick,
    New,
    Extract(bool),
    Test,
    Refresh,
    Cancel,
    Close,
    Up,
    Activate,
    Filter,
    ToggleTree,
    ToggleComment,
    Column(ColumnKind),
    Encoding(u32),
    SelectAll,
    CopyPath,
    Edit(ArchiveJobModifyAction),
}

pub(crate) struct ArchiveBrowser {
    pub(super) path: PathBuf,
    pub(super) window_id: Uuid,
    pub(super) owner: WeakEntity<ArchiveApplication>,
    pub(super) model: Option<BrowserModel>,
    pub(super) status: BrowserStatus,
    pub(super) generation: u64,
    pub(super) codepage: u32,
    pub(super) read_id: Option<Uuid>,
    pub(super) open_pending: bool,
    tree: Entity<TreeState>,
    filter_input: Entity<InputState>,
    list_focus: FocusHandle,
    root_focus: FocusHandle,
    scroll: UniformListScrollHandle,
    horizontal: ScrollHandle,
    columns: [bool; 10],
    widths: [f32; 10],
    show_tree: bool,
    show_comment: bool,
    filter_visible: bool,
    // Persisted state deliberately contains no archive/source path; `path` and `format` are runtime-only.
    format: String,
    comment: String,
    source_size: Option<u64>,
    solid: Option<bool>,
    encrypted: Option<bool>,
    file_count: usize,
    folder_count: usize,
    summary: String,
    observed_geometry: UiGeometry,
    sort: SortKey,
    descending: bool,
    restored: bool,
    pub(super) final_snapshot: Option<UiState>,
    retired: bool,
    edit_eligibility: Option<Result<(), String>>,
    projection_epoch: u64,
    _subscriptions: Vec<Subscription>,
    #[cfg(test)]
    open_dispatches: Vec<RowId>,
}
impl ArchiveBrowser {
    pub(crate) fn new(
        path: PathBuf,
        window_id: Uuid,
        owner: WeakEntity<super::ArchiveApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let filter_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("筛选当前文件夹 · Ctrl+F"));
        let tree = cx.new(|cx| TreeState::new(cx));
        let filter_sub = cx.subscribe_in(
            &filter_input,
            window,
            |this, input, event: &InputEvent, window, cx| {
                if !matches!(event, InputEvent::Change) || this.retired {
                    return;
                }
                let query = input.read(cx).value().to_string();
                if let Some(model) = &mut this.model {
                    if model.filter == query {
                        return;
                    }
                    model.filter = query;
                    model.reproject();
                    this.projection_changed(window, cx);
                }
            },
        );
        // Read only an immutable ID while the Tree callback owns its entity. No writeback here.
        let tree_sub = cx.observe_in(&tree, window, |this, tree, window, cx| {
            let id = tree
                .read(cx)
                .selected_item()
                .map(|item| item.id.to_string());
            let Some(id) = id else {
                return;
            };
            if this.retired || this.model.as_ref().is_none_or(|m| m.folder == id) {
                return;
            }
            let epoch = this.projection_epoch;
            let tree = tree.downgrade();
            cx.defer_in(window, move |this, window, cx| {
                let still_selected = tree
                    .read_with(cx, |tree, _| {
                        tree.selected_item()
                            .is_some_and(|item| item.id.as_ref() == id)
                    })
                    .unwrap_or(false);
                if !this.retired && this.projection_epoch == epoch && still_selected {
                    this.navigate(id, window, cx);
                }
            });
        });
        let bounds_sub = cx.observe_window_bounds(window, |this, window, _cx| {
            this.observe_window_state(window);
        });
        let release_sub = cx.on_release(|this, cx| {
            if this.retired {
                return;
            }
            this.final_snapshot = Some(this.snapshot_from_observed());
            this.retired = true;
            this.generation = this.generation.wrapping_add(1).max(1);
            let owner = this.owner.clone();
            let id = this.window_id;
            cx.defer(move |cx| {
                let _ = owner.update(cx, |app, cx| app.close_browser(id, cx));
            });
        });
        let root_focus = cx.focus_handle();
        root_focus.focus(window, cx);
        Self {
            path,
            window_id,
            owner,
            model: None,
            status: BrowserStatus::Loading,
            generation: 0,
            codepage: 0,
            read_id: None,
            open_pending: false,
            tree,
            filter_input,
            list_focus: cx.focus_handle(),
            root_focus,
            scroll: UniformListScrollHandle::new(),
            horizontal: ScrollHandle::new(),
            columns: [
                true, true, true, true, true, false, false, false, false, false,
            ],
            widths: [240., 112., 112., 112., 192., 112., 80., 112., 112., 280.],
            show_tree: true,
            show_comment: false,
            filter_visible: false,
            format: String::new(),
            comment: String::new(),
            source_size: None,
            solid: None,
            encrypted: None,
            file_count: 0,
            folder_count: 0,
            summary: "正在读取压缩包…".into(),
            observed_geometry: UiGeometry::default(),
            sort: SortKey::Name,
            descending: false,
            restored: false,
            final_snapshot: None,
            retired: false,
            projection_epoch: 0,
            edit_eligibility: None,
            _subscriptions: vec![filter_sub, tree_sub, bounds_sub, release_sub],
            #[cfg(test)]
            open_dispatches: vec![],
        }
    }
    /// Parent must match operation ID/generation before calling. This method never accepts a
    /// streaming partial list as success; the completed immutable snapshot replaces selection.
    pub(crate) fn install(
        &mut self,
        snapshot: Uuid,
        entries: Vec<ArchiveReadEntry>,
        format: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.retired {
            return;
        }
        let mut model = BrowserModel::new(snapshot, entries);
        model.sort = self.sort;
        model.descending = self.descending;
        model.reproject();
        self.file_count = model
            .entries
            .iter()
            .filter(|e| e.kind != "Directory")
            .count();
        let folders = model.folders();
        self.folder_count = folders.len();
        let encrypted_known = model.entries.iter().all(|e| e.encrypted.is_some());
        self.encrypted = if model.entries.iter().any(|e| e.encrypted == Some(true)) {
            Some(true)
        } else if encrypted_known && !model.entries.is_empty() {
            Some(false)
        } else {
            None
        };
        self.source_size = None;
        self.comment.clear();
        self.solid = None;
        // Approved default: auto-enter exactly one safe root directory, never a single file.
        if model.visible().len() == 1 && model.visible()[0].directory && !model.visible()[0].warning
        {
            let folder = model.visible()[0].path.clone();
            model.enter(&folder);
        }
        let items = folder_tree(
            self.path
                .file_name()
                .map_or_else(|| "压缩包".into(), |s| s.to_string_lossy().into_owned()),
            &folders,
        );
        self.model = Some(model);
        self.status = BrowserStatus::Ready;
        self.read_id = None;
        self.open_pending = false;
        self.edit_eligibility = None;
        self.format = format;
        let tree = self.tree.downgrade();
        let view = cx.entity().downgrade();
        let folder = self
            .model
            .as_ref()
            .map_or_else(String::new, |m| m.folder.clone());
        cx.defer(move |cx| {
            let live = view
                .read_with(cx, |v, _| {
                    !v.retired && v.model.as_ref().is_some_and(|m| m.snapshot == snapshot)
                })
                .unwrap_or(false);
            if live {
                let _ = tree.update(cx, |tree, cx| {
                    tree.set_items(items, cx);
                    tree.set_selected_item(Some(&TreeItem::new(folder.clone(), folder)), cx);
                });
            }
        });
        self.sync_filter(window, cx);
        self.projection_changed(window, cx);
        self.inspect_edit_eligibility(cx);
    }
    pub(crate) fn set_failure(
        &mut self,
        message: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.retired {
            return;
        }
        self.read_id = None;
        self.open_pending = false;
        self.status = if self.model.is_some() {
            BrowserStatus::Stale(message)
        } else {
            BrowserStatus::Failed(message)
        };
        self.edit_eligibility = Some(Err("旧快照不能编辑；请刷新。".into()));
        self.update_summary();
        cx.notify();
    }
    /// ArchiveApplication clears this after the matching open/warning/native terminal, including
    /// cancel and failure. This view owns duplicate suppression, not native success policy.
    pub(crate) fn finish_entry_open(&mut self, cx: &mut Context<Self>) {
        if self.retired {
            return;
        }
        self.open_pending = false;
        cx.notify();
    }
    /// Optional metadata seam avoids coupling this view to a concurrent protocol context shape.
    pub(crate) fn set_metadata(
        &mut self,
        source_size: Option<u64>,
        comment: String,
        solid: Option<bool>,
        encrypted: Option<bool>,
        cx: &mut Context<Self>,
    ) {
        if self.retired {
            return;
        }
        self.source_size = source_size;
        self.comment = comment
            .chars()
            .filter(|ch| !ch.is_control() || matches!(*ch, '\n' | '\r' | '\t'))
            .take(65536)
            .collect();
        self.solid = solid;
        if encrypted.is_some() {
            self.encrypted = encrypted;
        }
        self.update_summary();
        cx.notify();
    }
    pub(crate) fn observe_window_state(&mut self, window: &Window) {
        let bounds = window.window_bounds().get_bounds();
        let candidate = UiGeometry {
            x: f32::from(bounds.origin.x).round() as i32,
            y: f32::from(bounds.origin.y).round() as i32,
            width: f32::from(bounds.size.width).round() as i32,
            height: f32::from(bounds.size.height).round() as i32,
            maximized: window.is_maximized(),
        };
        if !candidate.maximized {
            let mut validated = self.snapshot_from_observed();
            validated.geometry = candidate;
            if validated.validate().is_ok() {
                self.observed_geometry = candidate;
            }
            self.observed_geometry.maximized = false;
        } else {
            self.observed_geometry.maximized = true;
        }
    }
    /// Application close integration reads this after the view has captured F4/native-close state.
    pub(crate) fn final_snapshot(&self) -> Option<UiState> {
        self.final_snapshot.clone()
    }
    /// Capture the current local presentation without touching another entity.
    pub(crate) fn snapshot(&mut self, window: &Window) -> UiState {
        self.observe_window_state(window);
        self.snapshot_from_observed()
    }
    pub(crate) fn snapshot_from_observed(&self) -> UiState {
        UiState {
            geometry: self.observed_geometry,
            widths: self.widths,
            visible: self.columns,
            show_tree: self.show_tree,
            show_comment: self.show_comment,
            sort: UiSortKey::from(self.sort),
            descending: self.descending,
            ..UiState::default()
        }
    }
    pub(crate) fn apply_restored_state(
        &mut self,
        state: UiState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.restored || state.validate().is_err() {
            return false;
        }
        self.widths = state.widths;
        self.columns = state.visible;
        self.show_tree = state.show_tree;
        self.show_comment = state.show_comment;
        self.sort = state.sort.into();
        self.descending = state.descending;
        self.observed_geometry = state.geometry;
        self.restored = true;
        if let Some(model) = &mut self.model {
            model.sort = self.sort;
            model.descending = self.descending;
            model.reproject();
            self.projection_changed(window, cx);
        } else {
            cx.notify();
        }
        true
    }
    pub(crate) fn retire(&mut self, cx: &mut Context<Self>) {
        if self.retired {
            return;
        }
        self.retired = true;
        self.generation = self.generation.wrapping_add(1).max(1);
        self.projection_epoch = self.projection_epoch.wrapping_add(1);
        self.read_id = None;
        self.open_pending = false;
        self.status = BrowserStatus::Cancelled;
        let owner = self.owner.clone();
        let id = self.window_id;
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| app.cancel_browser(id, cx));
        });
        cx.notify();
    }
    pub(super) fn edit_reason(&self) -> Option<String> {
        if !matches!(self.status, BrowserStatus::Ready) {
            return Some("仅完整的就绪快照可编辑。".into());
        }
        if self.format != "Zip" {
            return Some("仅普通 ZIP 可编辑；此格式只读。".into());
        }
        if !self
            .path
            .extension()
            .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case("zip"))
        {
            return Some("应用容器或分卷不能编辑；仅普通 .zip 可编辑。".into());
        }
        self.edit_eligibility
            .clone()
            .unwrap_or_else(|| Err("正在确认 ZIP 源结构…".into()))
            .err()
    }
    fn inspect_edit_eligibility(&mut self, cx: &mut Context<Self>) {
        let path = self.path.clone();
        let generation = self.generation;
        let snapshot = self.model.as_ref().map(|m| m.snapshot);
        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let sibling = path.with_extension("z01");
                    let upper = path.with_extension("Z01");
                    if sibling.exists() || upper.exists() {
                        return Err("分卷 ZIP 只读，不能编辑。".to_owned());
                    }
                    let mut file = std::fs::File::open(&path)
                        .map_err(|_| "源不可访问，请刷新。".to_owned())?;
                    use std::io::Read as _;
                    let mut header = [0u8; 4];
                    file.read_exact(&mut header)
                        .map_err(|_| "ZIP 源文件已损坏或过短。".to_owned())?;
                    if !matches!(&header, b"PK\x03\x04" | b"PK\x05\x06") {
                        return Err("自解压、非标准或损坏的 ZIP 不能编辑。".to_owned());
                    }
                    // Signed/application ZIP detection and complete structural preflight
                    // remain authoritative at the worker before any candidate is written.
                    Ok(())
                })
                .await;
            let _ = view.update(cx, |view, cx| {
                if !view.retired
                    && view.generation == generation
                    && view.model.as_ref().map(|m| m.snapshot) == snapshot
                {
                    view.edit_eligibility = Some(result);
                    cx.notify();
                }
            });
        })
        .detach();
    }
    fn edit_selection(
        &self,
        action: ArchiveJobModifyAction,
    ) -> Option<(Uuid, Vec<u64>, Option<String>)> {
        if self.edit_reason().is_some() {
            return None;
        }
        let model = self.model.as_ref()?;
        let mut ordinals = std::collections::BTreeSet::new();
        let mut selected = model
            .visible()
            .iter()
            .filter(|r| model.selected.contains(&r.id) && !r.parent);
        let target = if action == ArchiveJobModifyAction::Rename {
            let row = selected.next()?;
            if row.warning || selected.next().is_some() {
                return None;
            }
            let ordinal = row.ordinal?;
            ordinals.insert(ordinal);
            Some(row.path.clone() + if row.directory { "/" } else { "" })
        } else {
            if action == ArchiveJobModifyAction::Delete {
                for row in selected {
                    if row.warning {
                        return None;
                    }
                    if let Some(ordinal) = row.ordinal {
                        ordinals.insert(ordinal);
                    }
                    if row.directory {
                        let prefix = format!("{}/", row.path.trim_end_matches('/'));
                        ordinals.extend(
                            model
                                .entries
                                .iter()
                                .filter(|e| e.name.starts_with(&prefix))
                                .map(|e| e.ordinal),
                        );
                    }
                }
                if ordinals.is_empty() || ordinals.len() > 4096 {
                    return None;
                }
            }
            match action {
                ArchiveJobModifyAction::Add => {
                    (!model.folder.is_empty()).then(|| format!("{}/", model.folder))
                }
                ArchiveJobModifyAction::Comment => Some(self.comment.clone()),
                ArchiveJobModifyAction::NewDirectory => Some(format!(
                    "{}新建文件夹/",
                    if model.folder.is_empty() {
                        String::new()
                    } else {
                        format!("{}/", model.folder)
                    }
                )),
                _ => None,
            }
        };
        Some((model.snapshot, ordinals.into_iter().collect(), target))
    }
    fn command(&mut self, command: Command, window: &mut Window, cx: &mut Context<Self>) {
        if self.retired {
            return;
        }
        match command {
            Command::New => crate::archive::new_from_window(cx),
            Command::Edit(action) => {
                if let Some((snapshot, ordinals, value)) = self.edit_selection(action) {
                    let owner = self.owner.clone();
                    let id = self.window_id;
                    let generation = self.generation;
                    let codepage = self.codepage;
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |app, cx| {
                            app.modify_browser(
                                (id, generation, snapshot),
                                action,
                                ordinals,
                                value,
                                codepage,
                                cx,
                            )
                        });
                    });
                }
            }
            Command::Extract(selected) => {
                if !matches!(self.status, BrowserStatus::Ready) {
                    return;
                }
                let Some(model) = self.model.as_ref() else {
                    return;
                };
                let mut ordinals = std::collections::BTreeSet::new();
                if selected {
                    for row in model
                        .visible()
                        .iter()
                        .filter(|row| model.selected.contains(&row.id) && !row.parent)
                    {
                        if let Some(ordinal) = row.ordinal {
                            ordinals.insert(ordinal);
                        }
                        if row.directory {
                            let prefix = format!("{}/", row.path.trim_end_matches('/'));
                            for entry in &model.entries {
                                if entry.name.starts_with(&prefix) {
                                    ordinals.insert(entry.ordinal);
                                }
                            }
                        }
                    }
                    if ordinals.is_empty() {
                        return;
                    }
                }
                let owner = self.owner.clone();
                let id = self.window_id;
                let generation = self.generation;
                let snapshot = model.snapshot;
                cx.defer(move |cx| {
                    let _ = owner.update(cx, |app, cx| {
                        app.extract_browser(
                            id,
                            generation,
                            snapshot,
                            selected.then(|| ordinals.into_iter().collect()),
                            cx,
                        )
                    });
                });
            }
            Command::Test => {
                if !matches!(self.status, BrowserStatus::Ready) {
                    return;
                }
                let owner = self.owner.clone();
                let id = self.window_id;
                let generation = self.generation;
                cx.defer(move |cx| {
                    let _ = owner.update(cx, |app, cx| app.test_browser(id, generation, cx));
                });
            }
            Command::Pick => {
                let owner = self.owner.clone();
                let source = window.window_handle();
                cx.defer(move |cx| {
                    let _ = owner.update(cx, |app, cx| app.pick_archive(source, cx));
                });
            }
            Command::Refresh | Command::Encoding(_) => {
                if matches!(self.status, BrowserStatus::Loading) {
                    return;
                }
                if let Command::Encoding(value) = command {
                    self.codepage = value;
                }
                self.status = BrowserStatus::Loading;
                self.open_pending = false;
                self.update_summary();
                let owner = self.owner.clone();
                let id = self.window_id;
                let codepage = self.codepage;
                cx.defer(move |cx| {
                    let _ = owner.update(cx, |app, cx| app.refresh_browser(id, codepage, cx));
                });
            }
            Command::Cancel => {
                if !matches!(self.status, BrowserStatus::Loading) {
                    return;
                }
                self.read_id = None;
                self.status = BrowserStatus::Cancelled;
                self.update_summary();
                let owner = self.owner.clone();
                let id = self.window_id;
                cx.defer(move |cx| {
                    let _ = owner.update(cx, |app, cx| app.cancel_browser(id, cx));
                });
            }
            Command::Close => {
                let owner = self.owner.clone();
                let id = self.window_id;
                self.final_snapshot = Some(self.snapshot(window));
                self.retire(cx);
                cx.defer(move |cx| {
                    let _ = owner.update(cx, |app, cx| app.close_browser(id, cx));
                });
            }
            Command::Up => {
                if let Some(m) = &mut self.model {
                    m.up();
                    self.sync_filter(window, cx);
                    self.projection_changed(window, cx);
                }
            }
            Command::Activate => self.activate(window, cx),
            Command::Filter => {
                self.filter_visible = true;
                let input = self.filter_input.clone();
                cx.defer_in(window, move |this, window, cx| {
                    if !this.retired {
                        input.focus_handle(cx).focus(window, cx);
                    }
                });
            }
            Command::ToggleTree => self.show_tree = !self.show_tree,
            Command::ToggleComment => self.show_comment = !self.show_comment,
            Command::Column(kind) => {
                if kind != ColumnKind::Name {
                    self.columns[kind.index()] = !self.columns[kind.index()];
                }
            }
            Command::SelectAll => {
                if let Some(m) = &mut self.model {
                    m.select_all();
                    self.update_summary();
                }
            }
            Command::CopyPath => {
                if let Some(m) = &self.model {
                    let text = m
                        .visible()
                        .iter()
                        .filter(|r| m.selected.contains(&r.id) && !r.parent)
                        .map(|r| r.path.as_str())
                        .collect::<Vec<_>>()
                        .join("\r\n");
                    if !text.is_empty() {
                        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text));
                    }
                }
            }
        }
        cx.notify();
    }
    fn activate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(model) = &self.model else {
            return;
        };
        let Some(row) = model
            .focused
            .as_ref()
            .and_then(|id| model.visible().iter().find(|r| &r.id == id))
            .cloned()
        else {
            return;
        };
        if row.directory && !row.warning {
            self.navigate(row.path, window, cx);
            return;
        }
        if !matches!(self.status, BrowserStatus::Ready)
            || self.open_pending
            || row.warning
            || row.kind != "File"
            || !model.selected.contains(&row.id)
        {
            return;
        }
        // Parent owns the visible warning for multi-file activation, and must never fan it out.
        self.open_pending = true;
        #[cfg(test)]
        self.open_dispatches.push(row.id.clone());
        let owner = self.owner.clone();
        let id = self.window_id;
        let entry = row.id;
        let view = cx.entity().downgrade();
        let source = window.window_handle();
        let generation = self.generation;
        cx.defer(move |cx| {
            let live = view
                .read_with(cx, |view, _| {
                    !view.retired
                        && view.generation == generation
                        && matches!(view.status, BrowserStatus::Ready)
                        && view.model.as_ref().is_some_and(|m| {
                            m.focused.as_ref() == Some(&entry) && m.selected.contains(&entry)
                        })
                })
                .unwrap_or(false);
            if !live {
                return;
            }
            let _ = source.update(cx, |_, _, cx| {
                let _ = owner.update(cx, |app, cx| app.open_entry(id, entry, cx));
            });
        });
        cx.notify();
    }
    fn navigate(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.retired {
            return;
        }
        let Some(m) = &mut self.model else {
            return;
        };
        if m.folder == path {
            return;
        }
        m.enter(&path);
        self.sync_filter(window, cx);
        self.projection_changed(window, cx);
    }
    fn sync_filter(&self, window: &mut Window, cx: &mut Context<Self>) {
        let value = self
            .model
            .as_ref()
            .map_or_else(String::new, |m| m.filter.clone());
        let input = self.filter_input.downgrade();
        let view = cx.entity().downgrade();
        window.defer(cx, move |window, cx| {
            let live = view
                .read_with(cx, |v, _| {
                    !v.retired && v.model.as_ref().is_some_and(|m| m.filter == value)
                })
                .unwrap_or(false);
            if !live {
                return;
            }
            let _ = input.update(cx, |input, cx| {
                if input.value().as_ref() != value {
                    input.set_value(value, window, cx);
                }
            });
        });
    }
    fn projection_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.projection_epoch = self.projection_epoch.wrapping_add(1);
        self.update_summary();
        let epoch = self.projection_epoch;
        let folder = self
            .model
            .as_ref()
            .map_or_else(String::new, |m| m.folder.clone());
        let tree = self.tree.downgrade();
        let scroll = self.scroll.clone();
        let view = cx.entity().downgrade();
        window.defer(cx, move |_, cx| {
            let live = view
                .read_with(cx, |v, _| !v.retired && v.projection_epoch == epoch)
                .unwrap_or(false);
            if !live {
                return;
            }
            scroll.scroll_to_item(0, ScrollStrategy::Top);
            let _ = tree.update(cx, |tree, cx| {
                if tree
                    .selected_item()
                    .is_none_or(|item| item.id.as_ref() != folder)
                {
                    tree.set_selected_item(Some(&TreeItem::new(folder.clone(), folder)), cx);
                }
            });
        });
        cx.notify();
    }
    fn sort_column(&mut self, kind: ColumnKind, window: &mut Window, cx: &mut Context<Self>) {
        let key = kind.sort();
        if self.sort == key {
            self.descending = !self.descending;
        } else {
            self.sort = key;
            self.descending = false;
        }
        if let Some(m) = &mut self.model {
            m.sort = self.sort;
            m.descending = self.descending;
            m.reproject();
            self.projection_changed(window, cx);
        }
    }
    fn update_summary(&mut self) {
        if let Some(m) = &self.model {
            self.summary = if m.selected.is_empty() {
                format!(
                    "文件: {}  文件夹: {}  压缩包大小: {}",
                    self.file_count,
                    self.folder_count,
                    bytes(self.source_size)
                )
            } else {
                let mut files = 0;
                let mut folders = 0;
                let mut total = Some(0u64);
                for row in m.visible().iter().filter(|r| m.selected.contains(&r.id)) {
                    if row.directory {
                        folders += 1;
                    } else {
                        files += 1;
                        total =
                            total.and_then(|sum| row.size.and_then(|size| sum.checked_add(size)));
                    }
                }
                format!(
                    "已选 文件: {files}  文件夹: {folders}  原始大小: {}",
                    bytes(total)
                )
            };
        } else {
            self.summary = match self.status {
                BrowserStatus::Loading => "正在读取压缩包…",
                BrowserStatus::Cancelled => "已取消读取",
                _ => "尚未读取压缩包",
            }
            .into();
        }
    }
    fn keyboard(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.retired || window.has_active_dialog(cx) {
            return;
        }
        let key = event.keystroke.key.to_ascii_lowercase();
        let modifiers = event.keystroke.modifiers;
        if self.filter_input.focus_handle(cx).is_focused(window)
            && modifiers.control
            && matches!(key.as_str(), "e" | "t")
        {
            return;
        }
        if modifiers.control
            && !modifiers.alt
            && !modifiers.shift
            && matches!(key.as_str(), "o" | "f" | "n" | "e" | "t")
        {
            if !event.is_held {
                self.command(
                    match key.as_str() {
                        "o" => Command::Pick,
                        "n" => Command::New,
                        "e" => Command::Extract(
                            self.model
                                .as_ref()
                                .is_some_and(|model| !model.selected.is_empty()),
                        ),
                        "t" => Command::Test,
                        _ => Command::Filter,
                    },
                    window,
                    cx,
                );
            }
            cx.stop_propagation();
            return;
        }
        if key == "f4" && !modifiers.modified() {
            if !event.is_held {
                self.command(Command::Close, window, cx);
            }
            cx.stop_propagation();
            return;
        }
        let filter_focused = self.filter_input.focus_handle(cx).is_focused(window);
        if key == "escape" && !modifiers.modified() && filter_focused {
            self.filter_visible = false;
            if let Some(model) = &mut self.model {
                model.filter.clear();
                model.reproject();
            }
            self.sync_filter(window, cx);
            self.projection_changed(window, cx);
            self.list_focus.focus(window, cx);
            cx.stop_propagation();
            return;
        }
        if !self.list_focus.is_focused(window) || modifiers.alt || modifiers.platform {
            return;
        }
        if modifiers.control && key == "a" {
            self.command(Command::SelectAll, window, cx);
            cx.stop_propagation();
            return;
        }
        if modifiers.control {
            return;
        }
        if matches!(key.as_str(), "f2" | "delete") && !modifiers.modified() {
            if !event.is_held {
                self.command(
                    Command::Edit(if key == "f2" {
                        ArchiveJobModifyAction::Rename
                    } else {
                        ArchiveJobModifyAction::Delete
                    }),
                    window,
                    cx,
                );
            }
            cx.stop_propagation();
            return;
        }
        match key.as_str() {
            "enter" | "space" => {
                if !event.is_held {
                    self.activate(window, cx);
                }
            }
            "backspace" => {
                if !event.is_held {
                    self.command(Command::Up, window, cx);
                }
            }
            "up" | "down" | "home" | "end" | "pageup" | "pagedown" => {
                let page = ((f32::from(window.viewport_size().height) - 192.) / ROW_HEIGHT)
                    .floor()
                    .max(1.) as isize;
                if let Some(m) = &mut self.model {
                    if key == "home" || key == "end" {
                        let index = if key == "home" {
                            0
                        } else {
                            m.visible().len().saturating_sub(1)
                        };
                        if let Some(row) = m.visible().get(index) {
                            m.select(row.id.clone(), false, modifiers.shift);
                        }
                    } else {
                        let delta = match key.as_str() {
                            "up" => -1,
                            "down" => 1,
                            "pageup" => -page,
                            _ => page,
                        };
                        m.move_focus(delta, modifiers.shift);
                    }
                    if let Some(index) = m
                        .focused
                        .as_ref()
                        .and_then(|id| m.visible().iter().position(|r| &r.id == id))
                    {
                        self.scroll.scroll_to_item(index, ScrollStrategy::Nearest);
                    }
                    self.update_summary();
                    cx.notify();
                }
            }
            _ => return,
        }
        cx.stop_propagation();
    }
    fn columns_width(&self) -> Pixels {
        px(self
            .widths
            .iter()
            .zip(self.columns)
            .filter_map(|(width, shown)| shown.then_some(*width))
            .sum())
    }
    fn menu_item(
        view: WeakEntity<Self>,
        label: impl Into<gpui_kit::SharedString>,
        command: Command,
        disabled: bool,
    ) -> PopupMenuItem {
        PopupMenuItem::new(label)
            .disabled(disabled)
            .on_click(move |_, window, cx| {
                // Menu is a different entity; always release its callback lease before touching view.
                let view = view.clone();
                let command = command.clone();
                let source = window.window_handle();
                let snapshot = view
                    .read_with(cx, |v, _| v.model.as_ref().map(|m| m.snapshot))
                    .ok();
                cx.defer(move |cx| {
                    let _ = source.update(cx, |_, window, cx| {
                        let _ = view.update(cx, |this, cx| {
                            let row_command = matches!(
                                command,
                                Command::Activate
                                    | Command::Up
                                    | Command::SelectAll
                                    | Command::CopyPath
                                    | Command::Edit(_)
                            );
                            if row_command
                                && snapshot != Some(this.model.as_ref().map(|m| m.snapshot))
                            {
                                return;
                            }
                            this.command(command, window, cx);
                        });
                    });
                });
            })
    }
    fn file_menu(&self, menu: PopupMenu, weak: WeakEntity<Self>) -> PopupMenu {
        menu.item(Self::menu_item(
            weak.clone(),
            "打开压缩包…  Ctrl+O",
            Command::Pick,
            false,
        ))
        .item(Self::menu_item(
            weak.clone(),
            "新建压缩包…  Ctrl+N",
            Command::New,
            false,
        ))
        .item(Self::menu_item(
            weak.clone(),
            "解压全部…  Ctrl+E",
            Command::Extract(false),
            !matches!(self.status, BrowserStatus::Ready),
        ))
        .item(Self::menu_item(
            weak.clone(),
            "测试压缩包  Ctrl+T",
            Command::Test,
            !matches!(self.status, BrowserStatus::Ready),
        ))
        .item(Self::menu_item(
            weak.clone(),
            "刷新",
            Command::Refresh,
            matches!(self.status, BrowserStatus::Loading),
        ))
        .item(Self::menu_item(
            weak.clone(),
            "取消读取",
            Command::Cancel,
            !matches!(self.status, BrowserStatus::Loading),
        ))
        .separator()
        .item(Self::menu_item(
            weak,
            "关闭压缩包  F4",
            Command::Close,
            false,
        ))
    }
    fn view_menu(&self, mut menu: PopupMenu, weak: WeakEntity<Self>) -> PopupMenu {
        menu = menu
            .item(
                Self::menu_item(weak.clone(), "文件夹树", Command::ToggleTree, false)
                    .checked(self.show_tree),
            )
            .item(
                Self::menu_item(weak.clone(), "注释窗格", Command::ToggleComment, false)
                    .checked(self.show_comment),
            )
            .item(Self::menu_item(
                weak.clone(),
                "筛选当前文件夹  Ctrl+F",
                Command::Filter,
                self.model.is_none(),
            ))
            .separator()
            .label("详细信息列");
        for kind in ColumnKind::ALL {
            menu = menu.item(
                Self::menu_item(
                    weak.clone(),
                    kind.label(),
                    Command::Column(kind),
                    kind == ColumnKind::Name,
                )
                .checked(self.columns[kind.index()]),
            );
        }
        menu = menu.separator().label("文件名编码 · 刷新后替换快照");
        for &(value, label) in CODEPAGES {
            menu = menu.item(
                Self::menu_item(
                    weak.clone(),
                    label,
                    Command::Encoding(value),
                    matches!(self.status, BrowserStatus::Loading),
                )
                .checked(self.codepage == value),
            );
        }
        menu
    }
    fn row_menu(&self, menu: PopupMenu, weak: WeakEntity<Self>) -> PopupMenu {
        let eligible = self.model.as_ref().is_some_and(|m| {
            m.focused
                .as_ref()
                .and_then(|id| m.visible().iter().find(|r| &r.id == id))
                .is_some_and(|r| {
                    !r.warning
                        && (r.directory
                            || (matches!(self.status, BrowserStatus::Ready)
                                && !self.open_pending
                                && m.selected.len() == 1
                                && r.kind == "File"))
                })
        });
        menu.item(Self::menu_item(
            weak.clone(),
            "打开  Enter",
            Command::Activate,
            !eligible,
        ))
        .item(Self::menu_item(
            weak.clone(),
            "解压选中条目…",
            Command::Extract(true),
            !matches!(self.status, BrowserStatus::Ready)
                || self.model.as_ref().is_none_or(|m| m.selected.is_empty()),
        ))
        .item(Self::menu_item(
            weak.clone(),
            "解压全部…",
            Command::Extract(false),
            !matches!(self.status, BrowserStatus::Ready),
        ))
        .item(Self::menu_item(
            weak.clone(),
            "上一级  Backspace",
            Command::Up,
            self.model.as_ref().is_none_or(|m| m.folder.is_empty()),
        ))
        .separator()
        .item(Self::menu_item(
            weak.clone(),
            "全选  Ctrl+A",
            Command::SelectAll,
            self.model.is_none(),
        ))
        .item(Self::menu_item(
            weak.clone(),
            "复制相对路径",
            Command::CopyPath,
            self.model.as_ref().is_none_or(|m| m.selected.is_empty()),
        ))
        .separator()
        .item(Self::menu_item(
            weak.clone(),
            "添加文件…",
            Command::Edit(ArchiveJobModifyAction::Add),
            self.edit_reason().is_some(),
        ))
        .item(Self::menu_item(
            weak.clone(),
            "删除  Del",
            Command::Edit(ArchiveJobModifyAction::Delete),
            self.edit_selection(ArchiveJobModifyAction::Delete)
                .is_none(),
        ))
        .item(Self::menu_item(
            weak.clone(),
            "重命名  F2",
            Command::Edit(ArchiveJobModifyAction::Rename),
            self.edit_selection(ArchiveJobModifyAction::Rename)
                .is_none(),
        ))
        .item(Self::menu_item(
            weak.clone(),
            "新建文件夹",
            Command::Edit(ArchiveJobModifyAction::NewDirectory),
            self.edit_reason().is_some(),
        ))
        .item(Self::menu_item(
            weak,
            "归档注释",
            Command::Edit(ArchiveJobModifyAction::Comment),
            self.edit_reason().is_some(),
        ))
    }
    fn render_row(&self, row: &Row, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let id = row.id.clone();
        let right_id = id.clone();
        let weak = cx.entity().downgrade();
        let snapshot = self.model.as_ref().map(|m| m.snapshot);
        let selected = self
            .model
            .as_ref()
            .is_some_and(|m| m.selected.contains(&id));
        let focused = self
            .model
            .as_ref()
            .is_some_and(|m| m.focused.as_ref() == Some(&id));
        let selector = match &id {
            RowId::Entry(_, ordinal) => format!("archive-entry-{ordinal}"),
            RowId::Folder(_, path) => format!("archive-folder-{path}"),
            RowId::Parent(_, _) => "archive-parent".into(),
        };
        let cells = ColumnKind::ALL
            .into_iter()
            .filter(|kind| self.columns[kind.index()])
            .map(|kind| {
                div()
                    .flex_none()
                    .w(px(self.widths[kind.index()]))
                    .px_3()
                    .text_ellipsis()
                    .child(cell(row, kind))
                    .into_any_element()
            })
            .collect::<Vec<_>>();
        let context = weak.clone();
        let item = ListItem::new(gpui_kit::SharedString::from(format!("archive-row-{id:?}")))
            .debug_selector(move || selector.clone())
            .role(Role::ListBoxOption)
            .aria_selected(selected)
            .when(focused, |item| item.aria_active_descendant())
            .accessibility_label(if row.warning {
                format!("{}；包含安全警告，不可打开", row.name)
            } else {
                row.name.clone()
            })
            .selected(selected)
            .secondary_selected(
                focused && self.list_focus.is_focused(window) && window.last_input_was_keyboard(),
            )
            .h(px(ROW_HEIGHT))
            .w(self.columns_width())
            .px_0()
            .child(div().flex().w_full().children(cells))
            .on_click(
                cx.listener(move |this, event: &gpui_kit::ClickEvent, window, cx| {
                    if this.retired
                        || this.model.as_ref().map(|m| m.snapshot) != snapshot
                        || this
                            .model
                            .as_ref()
                            .is_none_or(|m| !m.visible().iter().any(|r| r.id == id))
                    {
                        return;
                    }
                    cx.stop_propagation();
                    this.list_focus.focus(window, cx);
                    if let Some(m) = &mut this.model {
                        m.select(
                            id.clone(),
                            event.modifiers().control,
                            event.modifiers().shift,
                        );
                    }
                    this.update_summary();
                    if event.click_count() == 2 {
                        this.activate(window, cx);
                    }
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, _, window, cx| {
                    if this.retired
                        || this.model.as_ref().map(|m| m.snapshot) != snapshot
                        || this
                            .model
                            .as_ref()
                            .is_none_or(|m| !m.visible().iter().any(|r| r.id == right_id))
                    {
                        return;
                    }
                    this.list_focus.focus(window, cx);
                    if let Some(m) = &mut this.model {
                        m.right_select(right_id.clone());
                    }
                    this.update_summary();
                    cx.notify();
                }),
            );
        item.context_menu(move |menu, _, cx| {
            if let Some(view) = context.upgrade() {
                view.read(cx).row_menu(menu, context.clone())
            } else {
                menu
            }
        })
        .into_any_element()
    }
}

// Builds the folder hierarchy once per completed snapshot, not an O(n²) per-frame prefix scan.
fn folder_tree(label: String, folders: &[String]) -> Vec<TreeItem> {
    // Parent paths sort before descendants. Build bottom-up without recursive application
    // stack growth or cloning whole subtrees; Tree virtualizes only the expanded rows.
    let mut nodes: BTreeMap<String, Vec<TreeItem>> = BTreeMap::new();
    for path in folders.iter().rev() {
        let mut nested = nodes.remove(path).unwrap_or_default();
        nested.reverse();
        let label = path.rsplit('/').next().unwrap_or(path).to_string();
        let node = TreeItem::new(path.clone(), label).children(nested);
        let parent = path.rsplit_once('/').map_or("", |(parent, _)| parent);
        nodes.entry(parent.into()).or_default().push(node);
    }
    let mut roots = nodes.remove("").unwrap_or_default();
    roots.reverse();
    vec![TreeItem::new("", label).expanded(true).children(roots)]
}
fn bytes(value: Option<u64>) -> String {
    value.map_or_else(|| "—".into(), |v| format!("{v} B"))
}
fn cell(row: &Row, kind: ColumnKind) -> String {
    match kind {
        ColumnKind::Name => {
            if row.warning {
                format!("⚠ {}", row.name)
            } else {
                row.name.clone()
            }
        }
        ColumnKind::Size => bytes(row.size),
        ColumnKind::Packed => bytes(row.packed_size),
        ColumnKind::Type => {
            if row.directory {
                "文件夹".into()
            } else if row.kind != "File" {
                row.kind.clone()
            } else {
                row.name.rsplit_once('.').map_or_else(
                    || "文件".into(),
                    |(_, ext)| format!("{} 文件", ext.to_uppercase()),
                )
            }
        }
        ColumnKind::Modified => modified(row.modified.as_ref()),
        ColumnKind::Method => row.method.map_or_else(
            || "—".into(),
            |method| match method {
                0 => "Store".into(),
                8 => "Deflate".into(),
                9 => "Deflate64".into(),
                12 => "BZip2".into(),
                14 => "LZMA".into(),
                93 => "Zstd".into(),
                95 => "XZ".into(),
                _ => format!("{method}"),
            },
        ),
        ColumnKind::Encrypted => row
            .encrypted
            .map_or("—", |v| if v { "是" } else { "否" })
            .into(),
        ColumnKind::Crc => row.crc32.map_or_else(|| "—".into(), |v| format!("{v:08X}")),
        ColumnKind::Attributes => row.attributes.as_ref().map_or_else(
            || "—".into(),
            |v| {
                format!(
                    "{}{}",
                    if v.read_only { "R" } else { "-" },
                    if v.hidden { "H" } else { "-" }
                )
            },
        ),
        ColumnKind::Path => row.path.clone(),
    }
}
fn modified(value: Option<&ArchiveModifiedTime>) -> String {
    let Some(value) = value else {
        return "—".into();
    };
    let Some((seconds, nanos)) = model::modified_key(value) else {
        return "—".into();
    };
    let Some(time) = chrono::DateTime::from_timestamp(seconds, nanos) else {
        return "—".into();
    };
    let suffix = if matches!(value, ArchiveModifiedTime::DosLocal { .. }) {
        " (本地)"
    } else {
        " UTC"
    };
    format!("{}{suffix}", time.format("%Y-%m-%d %H:%M:%S"))
}

impl ArchiveBrowser {
    fn action(
        &self,
        id: &'static str,
        label: &'static str,
        command: Command,
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(id)
            .label(label)
            .outline()
            .with_size(Size::Medium)
            .disabled(disabled)
            .on_click(
                cx.listener(move |this, _, window, cx| this.command(command.clone(), window, cx)),
            )
    }
    fn menu_button(
        &self,
        id: &'static str,
        label: &'static str,
        file: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let weak = cx.entity().downgrade();
        Button::new(id)
            .label(label)
            .outline()
            .with_size(Size::Medium)
            .dropdown_menu(move |menu, _, cx| {
                if let Some(view) = weak.upgrade() {
                    let view = view.read(cx);
                    if file {
                        view.file_menu(menu, weak.clone())
                    } else {
                        view.view_menu(menu, weak.clone())
                    }
                } else {
                    menu
                }
            })
            .into_any_element()
    }
    fn edit_menu_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let weak = cx.entity().downgrade();
        Button::new("archive-edit-menu")
            .label("编辑")
            .outline()
            .with_size(Size::Medium)
            .dropdown_menu(move |menu, _, cx| {
                if let Some(view) = weak.upgrade() {
                    view.read(cx).row_menu(menu, weak.clone())
                } else {
                    menu
                }
            })
            .into_any_element()
    }
    fn details(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let width = self.columns_width();
        let kinds = ColumnKind::ALL
            .into_iter()
            .filter(|kind| self.columns[kind.index()])
            .collect::<Vec<_>>();
        let sizes = self.widths;
        let sort = Some((self.sort, self.descending));
        let weak = cx.entity().downgrade();
        let resize_kinds = kinds.clone();
        let columns_key = self
            .columns
            .iter()
            .enumerate()
            .fold(0usize, |mask, (index, shown)| {
                mask | (usize::from(*shown) << index)
            });
        let header = h_resizable(("archive-columns", columns_key))
            .children(kinds.into_iter().map(|kind| {
                let marker = sort
                    .filter(|(sort, _)| *sort == kind.sort())
                    .map_or("", |(_, descending)| if descending { " ↓" } else { " ↑" });
                resizable_panel()
                    .size(px(sizes[kind.index()]))
                    .size_range(px(64.)..px(640.))
                    .flex_none()
                    .child(
                        Button::new(("archive-sort", kind.index()))
                            .label(format!("{}{marker}", kind.label()))
                            .ghost()
                            .with_size(Size::Medium)
                            .w_full()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.sort_column(kind, window, cx)
                            })),
                    )
            }))
            .on_resize(move |state, _, cx| {
                // Snapshot public sizes while the resize entity is borrowed; defer view writes.
                let sizes = state.read(cx).sizes().clone();
                let weak = weak.clone();
                let kinds = resize_kinds.clone();
                cx.defer(move |cx| {
                    let _ = weak.update(cx, |this, cx| {
                        if this.retired
                            || this
                                .columns
                                .iter()
                                .enumerate()
                                .fold(0usize, |mask, (index, shown)| {
                                    mask | (usize::from(*shown) << index)
                                })
                                != columns_key
                        {
                            return;
                        }
                        for (kind, size) in kinds.iter().zip(sizes) {
                            this.widths[kind.index()] = f32::from(size).clamp(64., 640.);
                        }
                        cx.notify();
                    });
                });
            });
        let count = self.model.as_ref().map_or(0, |m| m.visible().len());
        let body = if count == 0 {
            let filtered = self.model.as_ref().is_some_and(|m| !m.filter.is_empty());
            Empty::new()
                .header(
                    EmptyHeader::new()
                        .title(EmptyTitle::new().child(if filtered {
                            "没有匹配的项目"
                        } else {
                            "此文件夹为空"
                        }))
                        .description(EmptyDescription::new().child(if filtered {
                            "更改或清除筛选条件。"
                        } else {
                            "压缩包中没有可显示的项目。"
                        })),
                )
                .into_any_element()
        } else {
            uniform_list(
                "archive-details-list",
                count,
                cx.processor(|this, range: Range<usize>, window, cx| {
                    // Only the requested viewport range gets controls, cell strings and callbacks.
                    let rows = this
                        .model
                        .as_ref()
                        .map_or_else(Vec::new, |m| m.visible()[range].to_vec());
                    rows.iter()
                        .map(|row| this.render_row(row, window, cx))
                        .collect::<Vec<_>>()
                }),
            )
            .w(width)
            .h_full()
            .track_scroll(&self.scroll)
            .into_any_element()
        };
        let border = cx.theme().border;
        let focus = cx.theme().ring;
        div()
            .flex()
            .flex_col()
            .size_full()
            .min_w_0()
            .min_h_0()
            .child(
                div()
                    .flex_none()
                    .min_w_0()
                    .p_2()
                    .child(if self.filter_visible {
                        Input::new(&self.filter_input)
                            .aria_label("筛选当前文件夹")
                            .into_any_element()
                    } else {
                        div()
                            .text_ellipsis()
                            .child(self.model.as_ref().map_or_else(
                                || "压缩包".into(),
                                |m| {
                                    if m.folder.is_empty() {
                                        "/".into()
                                    } else {
                                        format!("/{}", m.folder)
                                    }
                                },
                            ))
                            .into_any_element()
                    }),
            )
            .child(
                div()
                    .id("archive-details-focus")
                    .test_support()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .relative()
                    .track_focus(&self.list_focus)
                    .tab_index(0)
                    .role(Role::ListBox)
                    .aria_label("压缩包详细信息；Ctrl 与 Shift 多选")
                    .border_1()
                    .border_color(border)
                    .focus_visible(move |style| style.border_color(focus))
                    .child(
                        div()
                            .id("archive-horizontal")
                            .test_support()
                            .debug_selector(|| "archive-horizontal".into())
                            .size_full()
                            .min_w_0()
                            .overflow_x_scroll()
                            .track_scroll(&self.horizontal)
                            .child(
                                div()
                                    .w(width)
                                    .h_full()
                                    .flex()
                                    .flex_col()
                                    .child(div().w(width).h(px(32.)).flex_none().child(header))
                                    .child(div().w(width).flex_1().min_h_0().child(body)),
                            ),
                    )
                    .horizontal_scrollbar(&self.horizontal)
                    .vertical_scrollbar(&self.scroll),
            )
            .when(self.show_comment, |pane| {
                pane.child(
                    div()
                        .id("archive-comment")
                        .debug_selector(|| "archive-comment".into())
                        .h(px(80.))
                        .flex_none()
                        .border_t_1()
                        .border_color(border)
                        .p_2()
                        .overflow_y_scrollbar()
                        .child(if self.comment.is_empty() {
                            "注释: —".into()
                        } else {
                            self.comment.clone()
                        }),
                )
            })
            .into_any_element()
    }
}
impl Focusable for ArchiveBrowser {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.root_focus.clone()
    }
}
impl Render for ArchiveBrowser {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.observe_window_state(window);
        let loading = matches!(self.status, BrowserStatus::Loading);
        let up_disabled = self.model.as_ref().is_none_or(|m| m.folder.is_empty());
        let title = format!(
            "{} - NxTools",
            self.path.file_name().map_or_else(
                || "压缩包".into(),
                |name| name.to_string_lossy().into_owned()
            )
        );
        let menus = div()
            .flex()
            .items_center()
            .gap_1()
            .p_1()
            .child(self.menu_button("archive-file-menu", "文件", true, cx))
            .child(self.edit_menu_button(cx))
            .child(self.menu_button("archive-view-menu", "查看", false, cx));
        let toolbar = Toolbar::new("archive-toolbar")
            .with_size(Size::Medium)
            .content(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_2()
                    .child(self.action("archive-open", "打开…", Command::Pick, false, cx))
                    .child(self.action(
                        "archive-extract-all",
                        "解压…",
                        Command::Extract(false),
                        !matches!(self.status, BrowserStatus::Ready),
                        cx,
                    ))
                    .child(self.action("archive-new", "新建", Command::New, false, cx))
                    .child(self.action(
                        "archive-test",
                        "测试",
                        Command::Test,
                        !matches!(self.status, BrowserStatus::Ready),
                        cx,
                    ))
                    .child(self.action(
                        "archive-add",
                        "添加",
                        Command::Edit(ArchiveJobModifyAction::Add),
                        self.edit_reason().is_some(),
                        cx,
                    ))
                    .child(
                        self.action(
                            "archive-delete",
                            "删除",
                            Command::Edit(ArchiveJobModifyAction::Delete),
                            self.edit_selection(ArchiveJobModifyAction::Delete)
                                .is_none(),
                            cx,
                        ),
                    )
                    .child(self.action("archive-up", "上一级", Command::Up, up_disabled, cx))
                    .child(self.action("archive-refresh", "刷新", Command::Refresh, loading, cx))
                    .when(loading, |bar| {
                        bar.child(self.action(
                            "archive-cancel",
                            "取消读取",
                            Command::Cancel,
                            false,
                            cx,
                        ))
                    })
                    .child(self.menu_button("archive-view", "查看 ▾", false, cx)),
            );
        let content = if self.model.is_some() {
            let details = self.details(window, cx);
            let tree = Tree::new(&self.tree, |ix, entry, _, _, _| {
                ListItem::new(("archive-tree-item", ix))
                    .accessibility_label(if entry.item().label.is_empty() {
                        "压缩包".into()
                    } else {
                        entry.item().label.clone()
                    })
                    .child(entry.item().label.clone())
            });
            h_resizable("archive-browser-split")
                .child(
                    resizable_panel()
                        .size(px(200.))
                        .size_range(px(160.)..px(280.))
                        .flex_none()
                        .visible(self.show_tree)
                        .child(tree),
                )
                .child(
                    resizable_panel()
                        .size_range(px(160.)..Pixels::MAX)
                        .min_w_0()
                        .child(details),
                )
                .into_any_element()
        } else {
            match &self.status {
                BrowserStatus::Loading => Empty::new()
                    .header(
                        EmptyHeader::new()
                            .title(EmptyTitle::new().child("正在读取压缩包…"))
                            .description(
                                EmptyDescription::new()
                                    .child("读取完成后显示目录；可取消本次读取。"),
                            ),
                    )
                    .into_any_element(),
                BrowserStatus::Failed(message) => {
                    Alert::error("archive-initial-error", message.clone())
                        .title("无法读取压缩包")
                        .into_any_element()
                }
                _ => Empty::new()
                    .header(
                        EmptyHeader::new()
                            .title(EmptyTitle::new().child("已取消读取"))
                            .description(
                                EmptyDescription::new()
                                    .child("点击刷新重新读取，或打开另一个压缩包。"),
                            ),
                    )
                    .into_any_element(),
            }
        };
        let state_label = match self.status {
            BrowserStatus::Loading => "读取中",
            BrowserStatus::Ready => "就绪",
            BrowserStatus::Cancelled => "已取消",
            BrowserStatus::Failed(_) => "读取失败",
            BrowserStatus::Stale(_) => "旧快照 · 只读",
        };
        let details_status = format!(
            "{}  固实: {}  加密: {}  注释: {}",
            if self.format.is_empty() {
                "—"
            } else {
                &self.format
            },
            known(self.solid),
            known(self.encrypted),
            if self.comment.is_empty() {
                "—"
            } else {
                "有"
            }
        );
        let status_text = if f32::from(window.viewport_size().width) < 800. {
            format!(
                "{} 固:{} 密:{} 注:{}",
                if self.format.is_empty() {
                    "—"
                } else {
                    &self.format
                },
                known(self.solid),
                known(self.encrypted),
                if self.comment.is_empty() {
                    "—"
                } else {
                    "有"
                }
            )
        } else if self.open_pending {
            format!("正在打开条目… · {details_status}")
        } else {
            format!("{state_label} · {details_status}")
        };
        let owner = self.owner.clone();
        let id = self.window_id;
        div()
            .id("archive-browser")
            .size_full()
            .flex()
            .flex_col()
            .min_w_0()
            .min_h_0()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .track_focus(&self.root_focus)
            .capture_key_down(
                cx.listener(|this, event, window, cx| this.keyboard(event, window, cx)),
            )
            .on_drop(move |paths: &ExternalPaths, window, cx| {
                if paths.paths().is_empty() {
                    return;
                }
                let paths = paths.paths().iter().take(256).cloned().collect::<Vec<_>>();
                let owner = owner.clone();
                let source: AnyWindowHandle = window.window_handle();
                cx.defer(move |cx| {
                    let _ = owner.update(cx, |app, cx| app.handle_drop(id, paths, source, cx));
                });
            })
            .child(TitleBar::new().child(title))
            .child(menus)
            .child(toolbar)
            .when(loading && self.model.is_some(), |view| {
                view.child(
                    Alert::info(
                        "archive-reading",
                        "正在重新读取；当前显示上一次快照，暂不可打开文件。",
                    )
                    .banner(),
                )
            })
            .when(
                matches!(self.status, BrowserStatus::Cancelled) && self.model.is_some(),
                |view| {
                    view.child(
                        Alert::warning(
                            "archive-cancelled",
                            "读取已取消；当前显示上一次快照。请刷新后再打开文件。",
                        )
                        .banner(),
                    )
                },
            )
            .when_some(
                if let BrowserStatus::Stale(message) = &self.status {
                    Some(message.clone())
                } else {
                    None
                },
                |view, message| {
                    view.child(
                        Alert::warning(
                            "archive-stale",
                            format!("{message} · 保留上一次目录，只读；请显式刷新。"),
                        )
                        .banner(),
                    )
                },
            )
            .when_some(self.edit_reason(), |view, reason| {
                view.child(Alert::info("archive-edit-reason", reason).banner())
            })
            .child(div().flex_1().min_h_0().min_w_0().child(content))
            .child(
                div()
                    .id("archive-status")
                    .test_support()
                    .debug_selector(|| "archive-status".into())
                    .flex_none()
                    .w_full()
                    .child(StatusBar::new().left(self.summary.clone()).right(
                        div().id("archive-status-info").child(status_text).tooltip(
                            move |window, cx| {
                                gpui_kit::component::tooltip::Tooltip::new(format!(
                                    "{state_label} · {details_status}"
                                ))
                                .build(window, cx)
                            },
                        ),
                    )),
            )
    }
}
fn known(value: Option<bool>) -> &'static str {
    value.map_or("—", |value| if value { "是" } else { "否" })
}

impl From<SortKey> for UiSortKey {
    fn from(value: SortKey) -> Self {
        match value {
            SortKey::Name => Self::Name,
            SortKey::Size => Self::Size,
            SortKey::Packed => Self::Packed,
            SortKey::Type => Self::Type,
            SortKey::Modified => Self::Modified,
            SortKey::Method => Self::Method,
            SortKey::Encrypted => Self::Encrypted,
            SortKey::Crc => Self::Crc,
            SortKey::Attributes => Self::Attributes,
            SortKey::Path => Self::Path,
        }
    }
}

impl From<UiSortKey> for SortKey {
    fn from(value: UiSortKey) -> Self {
        match value {
            UiSortKey::Name => Self::Name,
            UiSortKey::Size => Self::Size,
            UiSortKey::Packed => Self::Packed,
            UiSortKey::Type => Self::Type,
            UiSortKey::Modified => Self::Modified,
            UiSortKey::Method => Self::Method,
            UiSortKey::Encrypted => Self::Encrypted,
            UiSortKey::Crc => Self::Crc,
            UiSortKey::Attributes => Self::Attributes,
            UiSortKey::Path => Self::Path,
        }
    }
}

#[cfg(test)]
#[path = "browser_tests.rs"]
mod tests;
