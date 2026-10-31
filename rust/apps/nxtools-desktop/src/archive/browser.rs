//! Phase C production browser. Component ledger: official Tree + h_resizable (200/160..280),
//! ListItem.selected + public uniform_list (approved details-only exception), one BrowserModel
//! selection/focus/anchor. Toolbar.content deliberately bypasses child ghost/compact adaptation:
//! real Medium outline Buttons. Default Kit palette/type/radius/motion; no application timers.
//! Per-window PopupMenu, stable InputState, StatusBar, Alert and Empty retain their styled owners.
//! The view additionally owns path-free presentation snapshots; ArchiveApplication keeps native
//! read/window/consent/temp/Shell lifecycles and schedules independent state IO.
#[path = "browser_drag.rs"]
mod drag;
#[cfg(test)]
#[path = "browser_drag_tests.rs"]
mod drag_tests;
#[path = "browser_icons.rs"]
mod icons;
#[path = "browser_marquee.rs"]
mod marquee;
#[cfg(test)]
#[path = "browser_marquee_tests.rs"]
mod marquee_tests;
#[path = "browser_preview.rs"]
mod preview_pane;
#[path = "ui_state.rs"]
pub(crate) mod ui_state;
use super::{
    ArchiveApplication,
    model::{self, BrowserModel, Row, RowId, SortKey},
};
use crate::icons::MaterialIcon;
use gpui_kit::assets::IconName;
use gpui_kit::base::ElementExt as _;
use gpui_kit::base::Selectable as _;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::Icon;
use gpui_kit::component::{
    ActiveTheme, Disableable, Sizable, Size, WindowExt,
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
use nxtools_core::archive::ArchiveSettings;
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
        Self::Packed,
        Self::Size,
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
    Recent(PathBuf),
    ClearRecent,
    Extract(bool),
    Test,
    UnlockDrag,
    ContinueReclamation,
    Refresh,
    Cancel,
    Close,
    Up,
    Activate,
    Filter,
    ToggleTree,
    ToggleComment,
    ViewMode(bool),
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
    pub(super) recovery: Option<nxtools_protocol::archive_recovery::ArchiveRecoveryInspection>,
    pub(super) recent: Vec<PathBuf>,
    tree: Entity<TreeState>,
    filter_input: Entity<InputState>,
    list_focus: FocusHandle,
    root_focus: FocusHandle,
    scroll: UniformListScrollHandle,
    horizontal: ScrollHandle,
    columns: [bool; 10],
    widths: [f32; 10],
    details_view: bool,
    name_view_width: Pixels,
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
    settings: ArchiveSettings,
    descending: bool,
    restored: bool,
    pub(super) final_snapshot: Option<UiState>,
    retired: bool,
    pub(super) hosted: bool,
    edit_eligibility: Option<Result<(), nxtools_protocol::archive_job::ArchiveModificationRefusal>>,
    pub(super) edit_source: Option<nxtools_protocol::archive_browser::ArchiveSourceBinding>,
    projection_epoch: u64,
    edit_refresh: Option<(u64, String, Vec<String>)>,
    _subscriptions: Vec<Subscription>,
    drag: drag::DragState,
    type_icons: icons::TypeIcons,
    marquee: marquee::MarqueeState,
    preview: preview_pane::PreviewPane,
    #[cfg(test)]
    open_dispatches: Vec<RowId>,
}
impl ArchiveBrowser {
    #[cfg(test)]
    pub(crate) fn new(
        path: PathBuf,
        window_id: Uuid,
        owner: WeakEntity<super::ArchiveApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::new_with_focus(path, window_id, owner, true, window, cx)
    }
    pub(crate) fn new_hosted(
        path: PathBuf,
        window_id: Uuid,
        owner: WeakEntity<super::ArchiveApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::new_with_focus(path, window_id, owner, false, window, cx)
    }
    fn new_with_focus(
        path: PathBuf,
        window_id: Uuid,
        owner: WeakEntity<super::ArchiveApplication>,
        initial_focus: bool,
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
            this.invalidate_drag();
            this.retired = true;
            this.generation = this.generation.wrapping_add(1).max(1);
            let owner = this.owner.clone();
            let id = this.window_id;
            cx.defer(move |cx| {
                let _ = owner.update(cx, |app, cx| app.close_browser(id, cx));
            });
        });
        let list_focus = cx.focus_handle();
        let blur_sub = cx.on_blur(&list_focus, window, |this, _, cx| {
            this.cancel_marquee(true, cx);
        });
        let activation_sub = cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() {
                this.cancel_marquee(true, cx);
            }
        });
        let root_focus = cx.focus_handle();
        if initial_focus {
            root_focus.focus(window, cx);
        }
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
            recovery: None,
            recent: Vec::new(),
            tree,
            filter_input,
            list_focus,
            root_focus,
            scroll: UniformListScrollHandle::new(),
            horizontal: ScrollHandle::new(),
            columns: [
                true, true, true, true, true, false, false, false, false, false,
            ],
            widths: [240., 112., 112., 112., 192., 112., 80., 112., 112., 280.],
            details_view: true,
            name_view_width: px(720.),
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
            settings: ArchiveSettings::default(),
            final_snapshot: None,
            retired: false,
            hosted: false,
            projection_epoch: 0,
            edit_eligibility: None,
            edit_source: None,
            edit_refresh: None,
            _subscriptions: vec![
                filter_sub,
                tree_sub,
                bounds_sub,
                release_sub,
                blur_sub,
                activation_sub,
            ],
            drag: drag::DragState::default(),
            type_icons: icons::TypeIcons::default(),
            marquee: marquee::MarqueeState::default(),
            preview: preview_pane::PreviewPane::default(),
            #[cfg(test)]
            open_dispatches: vec![],
        }
    }
    /// Apply only changed preferences; preserve per-window layout, immutable selection and job owners.
    pub(crate) fn apply_settings(
        &mut self,
        settings: ArchiveSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.retired {
            return;
        }
        if self.settings.show_tree != settings.show_tree {
            self.show_tree = settings.show_tree;
        }
        if self.settings.show_parent_row != settings.show_parent_row {
            if let Some(model) = &mut self.model {
                model.show_parent_row = settings.show_parent_row;
                model.reproject();
            }
            self.projection_changed(window, cx);
        }
        self.settings = settings;
        cx.notify();
    }
    /// Only a complete current-generation snapshot replaces the existing selection.
    pub(crate) fn tab_title(&self) -> String {
        format!(
            "{} — NxTools",
            if self.settings.show_full_path {
                self.path.to_string_lossy().into_owned()
            } else {
                self.path.file_name().map_or_else(
                    || "压缩包".into(),
                    |name| name.to_string_lossy().into_owned(),
                )
            }
        )
    }
    pub(crate) fn suspend_tab(&mut self, cx: &mut Context<Self>) {
        self.suspend_tab_gestures();
        self.suspend_preview(cx);
        self.cancel_marquee(true, cx);
    }
    /// One next-generation presentation hint; never old ordinals or write authority.
    pub(super) fn prepare_edit_refresh(
        &mut self,
        intent: &nxtools_protocol::archive_job::ArchiveJobModify,
        next_generation: u64,
    ) {
        let Some(model) = &self.model else {
            return;
        };
        if model.snapshot != intent.browser.snapshot_id {
            return;
        }
        let old = intent.source_prefix.as_deref().or_else(|| {
            intent.ordinals.first().and_then(|ordinal| {
                model
                    .entries
                    .iter()
                    .find(|entry| entry.ordinal == *ordinal)
                    .map(|entry| entry.name.as_str())
            })
        });
        let old = old.map(super::model::member_path);
        let transform = |path: &str| -> Option<String> {
            if intent.action == ArchiveJobModifyAction::Delete
                && intent.ordinals.iter().any(|ordinal| {
                    model.entries.iter().any(|entry| {
                        let normalized = super::model::member_path(&entry.name);
                        entry.ordinal == *ordinal
                            && (path == normalized.trim_end_matches('/')
                                || entry.kind == "Directory"
                                    && path.starts_with(&format!(
                                        "{}/",
                                        normalized.trim_end_matches('/')
                                    )))
                    })
                })
            {
                return None;
            }
            if intent.action == ArchiveJobModifyAction::Rename
                && let (Some(old), Some(new)) = (old.as_deref(), intent.value.as_deref())
            {
                let old = old.trim_end_matches('/');
                let new = new.trim_end_matches('/');
                if path == old {
                    return Some(new.to_owned());
                }
                if intent
                    .value
                    .as_ref()
                    .is_some_and(|name| name.ends_with('/'))
                    && path.starts_with(&format!("{old}/"))
                {
                    return Some(format!("{new}{}", &path[old.len()..]));
                }
            }
            Some(path.to_owned())
        };
        let folder = transform(&model.folder).unwrap_or_else(|| {
            model
                .folder
                .rsplit_once('/')
                .map_or("", |(parent, _)| parent)
                .to_owned()
        });
        let selected = model
            .visible()
            .iter()
            .filter(|row| !row.parent && model.selected.contains(&row.id))
            .filter_map(|row| transform(&row.path))
            .collect();
        self.edit_refresh = Some((next_generation, folder, selected));
    }
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
        self.invalidate_drag();
        self.recovery = None;
        self.clear_pending_icons();
        let mut model = BrowserModel::new(snapshot, entries);
        model.show_parent_row = self.settings.show_parent_row;
        model.sort = self.sort;
        model.descending = self.descending;
        let edit_refresh = self
            .edit_refresh
            .take()
            .filter(|(generation, _, _)| *generation == self.generation);
        if let Some((_, folder, _)) = &edit_refresh {
            let folders = model.folders();
            let mut folder = folder.as_str();
            while !folder.is_empty() && !folders.iter().any(|candidate| candidate == folder) {
                folder = folder.rsplit_once('/').map_or("", |(parent, _)| parent);
            }
            model.folder = folder.to_owned();
        }
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
        if edit_refresh.is_none()
            && self.settings.auto_enter_single_root
            && model.visible().len() == 1
            && model.visible()[0].directory
            && !model.visible()[0].warning
        {
            let folder = model.visible()[0].path.clone();
            model.enter(&folder);
        }
        if let Some((_, _, selected)) = edit_refresh {
            let rows = model
                .visible()
                .iter()
                .filter(|row| !row.parent && !row.warning && selected.contains(&row.path))
                .map(|row| row.id.clone())
                .collect::<Vec<_>>();
            model.selected.extend(rows.iter().cloned());
            model.focused = rows.first().cloned();
            model.anchor = rows.first().cloned();
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
        self.edit_source = None;
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
        self.invalidate_drag();
        self.recovery = None;
        self.cancel_marquee(false, cx);
        self.read_id = None;
        self.open_pending = false;
        self.status = if self.model.is_some() {
            BrowserStatus::Stale(message)
        } else {
            BrowserStatus::Failed(message)
        };
        self.edit_eligibility = Some(Err(
            nxtools_protocol::archive_job::ArchiveModificationRefusal::SourceChanged,
        ));
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
            details_view: self.details_view,
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
        self.widths[0] = self.widths[0].max(240.);
        self.columns = state.visible;
        self.details_view = state.details_view;
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
        self.invalidate_drag();
        self.cancel_marquee(false, cx);
        self.clear_pending_icons();
        self.retired = true;
        self.edit_refresh = None;
        self.generation = self.generation.wrapping_add(1).max(1);
        self.projection_epoch = self.projection_epoch.wrapping_add(1);
        self.read_id = None;
        self.open_pending = false;
        self.status = BrowserStatus::Cancelled;
        self.observe_preview_selection();
        let owner = self.owner.clone();
        let id = self.window_id;
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| app.cancel_browser(id, cx));
        });
        cx.notify();
    }
    pub(super) fn operation_refusal(
        &self,
        operation: nxtools_protocol::archive_job::ArchiveOperation,
    ) -> Option<nxtools_protocol::archive_job::ArchiveModificationRefusal> {
        use nxtools_protocol::archive_job::ArchiveModificationRefusal as Refusal;
        if !matches!(self.status, BrowserStatus::Ready) || self.retired {
            return Some(if matches!(self.status, BrowserStatus::Stale(_)) {
                Refusal::SourceChanged
            } else {
                Refusal::SnapshotNotReady
            });
        }
        let Some(format) = nxtools_archive::ArchiveFormat::from_browser_name(&self.format) else {
            return Some(Refusal::BackendMissing);
        };
        if !matches!(
            format,
            nxtools_archive::ArchiveFormat::Zip
                | nxtools_archive::ArchiveFormat::SevenZ
                | nxtools_archive::ArchiveFormat::Tar
                | nxtools_archive::ArchiveFormat::TarGz
                | nxtools_archive::ArchiveFormat::TarXz
        ) {
            return Some(Refusal::BackendMissing);
        }
        if self.model.is_none() || self.edit_source.is_none() {
            return Some(Refusal::UnverifiedMetadata);
        }
        let Some(eligibility) = self.edit_eligibility else {
            return Some(Refusal::UnverifiedMetadata);
        };
        let refusal = eligibility.err();
        if format == nxtools_archive::ArchiveFormat::SevenZ
            && let Some(refusal) = refusal
        {
            return Some(refusal);
        }
        nxtools_archive::ArchiveCapabilities::opened(format, refusal)
            .admit(operation)
            .err()
    }
    /// Validate frozen intent members against this snapshot, never current sorted/selected rows.
    pub(super) fn edit_members_current(
        &self,
        action: ArchiveJobModifyAction,
        ordinals: &[u64],
        value: Option<&str>,
        source_prefix: Option<&str>,
    ) -> bool {
        let Some(model) = &self.model else {
            return false;
        };
        let entries = ordinals
            .iter()
            .map(|ordinal| model.entries.iter().find(|entry| entry.ordinal == *ordinal));
        if entries
            .clone()
            .any(|entry| entry.is_none_or(|entry| !entry.warning_codes.is_empty()))
        {
            return false;
        }
        match action {
            ArchiveJobModifyAction::Delete => !ordinals.is_empty(),
            ArchiveJobModifyAction::ReplaceContent => {
                ordinals.len() == 1 && entries.flatten().all(|entry| entry.kind == "File")
            }
            ArchiveJobModifyAction::Rename => {
                if ordinals.len() != 1 {
                    return false;
                }
                let Some(entry) = entries.flatten().next() else {
                    return false;
                };
                let directory = value.is_some_and(|name| name.ends_with('/'));
                if let Some(prefix) = source_prefix {
                    let normalized = super::model::member_path(&entry.name);
                    directory
                        && prefix.ends_with('/')
                        && (normalized.starts_with(prefix)
                            || entry.kind == "Directory"
                                && normalized.trim_end_matches('/') == prefix.trim_end_matches('/'))
                } else {
                    directory == (entry.kind == "Directory")
                }
            }
            _ => ordinals.is_empty(),
        }
    }
    pub(super) fn edit_refusal(
        &self,
    ) -> Option<nxtools_protocol::archive_job::ArchiveModificationRefusal> {
        self.operation_refusal(nxtools_protocol::archive_job::ArchiveOperation::Add)
    }
    fn edit_operation(
        &self,
        action: ArchiveJobModifyAction,
    ) -> nxtools_protocol::archive_job::ArchiveOperation {
        let directory = action == ArchiveJobModifyAction::Rename
            && self.model.as_ref().is_some_and(|model| {
                model
                    .visible()
                    .iter()
                    .find(|row| model.selected.contains(&row.id) && !row.parent)
                    .is_some_and(|row| row.directory)
            });
        action.operation(directory.then_some("/"))
    }
    pub(super) fn edit_reason(&self) -> Option<String> {
        self.edit_refusal().map(|reason| {
            if reason == nxtools_protocol::archive_job::ArchiveModificationRefusal::BackendMissing
                && !matches!(
                    self.format.as_str(),
                    "SevenZ"
                        | "Rar4"
                        | "Rar5"
                        | "Tar"
                        | "TarGz"
                        | "TarBz2"
                        | "TarXz"
                        | "TarZstd"
                        | "Gzip"
                        | "Bzip2"
                        | "Xz"
                        | "Zstd"
                )
            {
                "此格式暂不支持修改压缩包内容。".into()
            } else {
                reason.message().into()
            }
        })
    }
    /// Only completed isolated-worker browser facts can admit 7z; Desktop has no isolated codec.
    pub(super) fn bind_edit_context(
        &mut self,
        browser: &nxtools_protocol::archive_browser::ArchiveBrowserContext,
        cx: &mut Context<Self>,
    ) {
        if self.retired
            || self.format != browser.format
            || self
                .model
                .as_ref()
                .is_none_or(|model| model.snapshot != browser.snapshot_id)
        {
            return;
        }
        self.edit_source = Some(browser.source.clone());
        self.edit_eligibility = match self.format.as_str() {
            "SevenZ" => browser
                .sevenz_edit
                .as_ref()
                .map(|facts| facts.refusal.map_or(Ok(()), Err)),
            "Tar" | "TarGz" | "TarXz" => browser
                .tar_edit
                .as_ref()
                .map(|facts| facts.refusal.map_or(Ok(()), Err)),
            _ => None,
        };
        self.inspect_edit_eligibility(cx);
        cx.notify();
    }
    pub(super) fn inspect_edit_eligibility(&mut self, cx: &mut Context<Self>) {
        if self.format != "Zip" {
            return;
        }
        let Some(expected) = self.edit_source.clone() else {
            return;
        };
        let path = self.path.clone();
        let generation = self.generation;
        let snapshot = self.model.as_ref().map(|m| m.snapshot);
        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    nxtools_archive::ArchiveEngine::default().inspect_modification_source(
                        nxtools_archive::OpenRequest::new(path),
                        &expected,
                        &nxtools_archive::OperationControl::default(),
                    )
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
        if self
            .operation_refusal(self.edit_operation(action))
            .is_some()
        {
            return None;
        }
        let model = self.model.as_ref()?;
        let mut ordinals = std::collections::BTreeSet::new();
        let mut selected = model
            .visible()
            .iter()
            .filter(|r| model.selected.contains(&r.id) && !r.parent);
        let target = if matches!(
            action,
            ArchiveJobModifyAction::Rename | ArchiveJobModifyAction::ReplaceContent
        ) {
            let row = selected.next()?;
            if row.warning
                || selected.next().is_some()
                || action == ArchiveJobModifyAction::ReplaceContent && row.directory
            {
                return None;
            }
            let ordinal = row.ordinal.or_else(|| {
                if !matches!(self.format.as_str(), "SevenZ" | "Tar" | "TarGz" | "TarXz")
                    || action != ArchiveJobModifyAction::Rename
                    || !row.directory
                {
                    return None;
                }
                let prefix = format!("{}/", row.path.trim_end_matches('/'));
                model
                    .entries
                    .iter()
                    .find(|entry| super::model::member_path(&entry.name).starts_with(&prefix))
                    .map(|entry| entry.ordinal)
            })?;
            if action == ArchiveJobModifyAction::ReplaceContent
                && !model.entries.iter().any(|entry| {
                    entry.ordinal == ordinal
                        && entry.kind == "File"
                        && entry.warning_codes.is_empty()
                })
            {
                return None;
            }
            ordinals.insert(ordinal);
            (action == ArchiveJobModifyAction::Rename)
                .then(|| row.path.clone() + if row.directory { "/" } else { "" })
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
                                .filter(|e| super::model::member_path(&e.name).starts_with(&prefix))
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
            Command::ContinueReclamation => {
                let Some(recovery) = self.recovery.as_ref().filter(|record| !record.completed)
                else {
                    return;
                };
                if self.open_pending
                    || !matches!(
                        self.status,
                        BrowserStatus::Failed(_) | BrowserStatus::Stale(_)
                    )
                {
                    return;
                }
                let recovery_id = recovery.recovery_id;
                let owner = self.owner.clone();
                let id = self.window_id;
                let generation = self.generation;
                self.open_pending = true;
                cx.defer(move |cx| {
                    let _ = owner.update(cx, |app, cx| {
                        app.continue_reclamation_browser(id, generation, recovery_id, cx)
                    });
                });
            }
            Command::New => crate::archive::new_from_window(cx),
            Command::Recent(path) => crate::archive::open_recent(path, window.window_handle(), cx),
            Command::ClearRecent => {
                let source = window.window_handle();
                crate::archive::clear_recent(
                    move |result, cx| {
                        if result.is_err() {
                            cx.defer(move |cx| { let _ = source.update(cx, |_, window, cx| {
                            window.open_dialog(cx, |dialog, _, _| dialog.title("无法清除历史记录").child("原记录文件已保留，请检查存储后重试；当前归档不受影响。"));
                        }); });
                        }
                    },
                    cx,
                );
            }
            Command::Edit(action) => {
                if let Some(refusal) = self.operation_refusal(self.edit_operation(action)) {
                    let reason = if self.format == "Zip" {
                        refusal.message().to_owned()
                    } else {
                        self.edit_reason()
                            .unwrap_or_else(|| refusal.message().into())
                    };
                    window.open_dialog(cx, move |dialog, _, _| {
                        dialog.title("无法修改压缩包").child(
                            div()
                                .id("archive-modification-reason")
                                .role(Role::Label)
                                .aria_label(reason.clone())
                                .test_support()
                                .child(
                                    gpui_kit::component::label::Label::new(reason.clone())
                                        .whitespace_normal(),
                                ),
                        )
                    });
                    return;
                }
                if let Some((snapshot, ordinals, value)) = self.edit_selection(action) {
                    let source_prefix =
                        (matches!(self.format.as_str(), "SevenZ" | "Tar" | "TarGz" | "TarXz")
                            && action == ArchiveJobModifyAction::Rename)
                            .then(|| value.as_ref().filter(|value| value.ends_with('/')).cloned())
                            .flatten();
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
                                (value, source_prefix),
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
                                if super::model::member_path(&entry.name).starts_with(&prefix) {
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
            Command::UnlockDrag => self.unlock_drag(cx),
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
                self.invalidate_drag();
                self.cancel_marquee(false, cx);
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
            Command::ViewMode(details) => {
                if self.details_view != details {
                    let correlation_id = Uuid::new_v4();
                    let started = std::time::Instant::now();
                    tracing::info!(event="archive.view_mode.started", %correlation_id, details, "archive view mode accepted");
                    self.details_view = details;
                    self.invalidate_drag();
                    let scroll = self.horizontal.clone();
                    window.defer(cx, move |_, _| {
                        scroll.set_offset(gpui_kit::point(px(0.), px(0.)));
                        tracing::info!(event="archive.view_mode.finished", %correlation_id, details, success=true, elapsed_ms=started.elapsed().as_millis() as u64, "archive view mode updated");
                    });
                }
            }
            Command::Column(kind) => {
                if kind != ColumnKind::Name {
                    self.details_view = true;
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
        self.observe_preview_selection();
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
        self.invalidate_drag();
        self.cancel_marquee(false, cx);
        self.projection_epoch = self.projection_epoch.wrapping_add(1);
        self.update_summary();
        let epoch = self.projection_epoch;
        let folder = self
            .model
            .as_ref()
            .map_or_else(String::new, |m| m.folder.clone());
        let tree = self.tree.downgrade();
        let reveal = self.settings.auto_expand_tree;
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
                if reveal {
                    tree.reveal_item(&folder.clone().into(), ScrollStrategy::Nearest, cx);
                }
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
        self.observe_preview_selection();
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
        if key == "escape" && self.cancel_marquee(true, cx) {
            self.invalidate_drag();
            cx.stop_propagation();
            return;
        }
        if key == "escape" {
            self.invalidate_drag();
        }
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
        if key == "escape"
            && !modifiers.modified()
            && self.settings.escape_closes_window
            && (self.list_focus.is_focused(window) || self.root_focus.is_focused(window))
        {
            if !event.is_held {
                self.command(Command::Close, window, cx);
            }
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
        if !self.details_view {
            return self.name_view_width;
        }
        px(self
            .widths
            .iter()
            .zip(self.columns)
            .filter_map(|(width, shown)| shown.then_some(*width))
            .sum())
    }
    fn shown_column(&self, kind: ColumnKind) -> bool {
        if self.details_view {
            self.columns[kind.index()]
        } else {
            kind == ColumnKind::Name
        }
    }
    fn column_width(&self, kind: ColumnKind) -> Pixels {
        if !self.details_view && kind == ColumnKind::Name {
            self.columns_width()
        } else {
            px(self.widths[kind.index()])
        }
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
    fn file_menu(
        recent: Vec<PathBuf>,
        ready: bool,
        loading: bool,
        menu: PopupMenu,
        weak: WeakEntity<Self>,
        window: &mut Window,
        cx: &mut Context<PopupMenu>,
    ) -> PopupMenu {
        let recent_owner = weak.clone();
        menu.item(Self::menu_item(
            weak.clone(),
            "打开压缩包…  Ctrl+O",
            Command::Pick,
            false,
        ))
        .submenu("最近压缩包", window, cx, move |mut menu, _, _| {
            if recent.is_empty() {
                menu = menu.label("没有历史记录");
            }
            for path in &recent {
                menu = menu.item(Self::menu_item(
                    recent_owner.clone(),
                    path.to_string_lossy().into_owned(),
                    Command::Recent(path.clone()),
                    false,
                ));
            }
            menu.separator().item(Self::menu_item(
                recent_owner.clone(),
                "清除历史记录",
                Command::ClearRecent,
                recent.is_empty(),
            ))
        })
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
            !ready,
        ))
        .item(Self::menu_item(
            weak.clone(),
            "测试压缩包  Ctrl+T",
            Command::Test,
            !ready,
        ))
        .item(Self::menu_item(
            weak.clone(),
            "刷新",
            Command::Refresh,
            loading,
        ))
        .item(Self::menu_item(
            weak.clone(),
            "取消读取",
            Command::Cancel,
            !loading,
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
                Self::menu_item(weak.clone(), "名称列表", Command::ViewMode(false), false)
                    .checked(!self.details_view),
            )
            .item(
                Self::menu_item(weak.clone(), "详细信息", Command::ViewMode(true), false)
                    .checked(self.details_view),
            )
            .separator()
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
        menu = menu.separator().label("文件名编码（选择后重新读取）");
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
        .when(
            matches!(self.format.as_str(), "SevenZ" | "Tar" | "TarGz" | "TarXz"),
            |menu| {
                menu.item(Self::menu_item(
                    weak.clone(),
                    "替换内容…",
                    Command::Edit(ArchiveJobModifyAction::ReplaceContent),
                    self.edit_selection(ArchiveJobModifyAction::ReplaceContent)
                        .is_none(),
                ))
            },
        )
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
            self.operation_refusal(self.edit_operation(ArchiveJobModifyAction::Comment))
                .is_some(),
        ))
    }
    fn render_row(&self, row: &Row, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let id = row.id.clone();
        let right_id = id.clone();
        let drag_id = id.clone();
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
        let mut cells = ColumnKind::ALL
            .into_iter()
            .filter(|kind| self.shown_column(*kind))
            .map(|kind| {
                let cell_selector = format!(
                    "archive-cell-{}-{}",
                    row.ordinal
                        .map_or_else(|| "folder".into(), |n| n.to_string()),
                    kind.index()
                );
                let mut content = div()
                    .debug_selector(move || cell_selector.clone())
                    .flex_none()
                    .w(self.column_width(kind))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .overflow_hidden();
                content = content.when(self.settings.show_grid_lines, |cell| {
                    cell.border_r_1().border_color(cx.theme().border)
                });
                if kind == ColumnKind::Name {
                    let icon_size = px(if row.directory && !row.parent {
                        24.
                    } else {
                        16.
                    });
                    let icon = if let Some(path) = self.icon_path(row) {
                        gpui_kit::img(path.clone())
                            .size(icon_size)
                            .into_any_element()
                    } else {
                        let fallback = if row.parent {
                            Icon::new(MaterialIcon::ArrowUpward)
                        } else if row.directory {
                            Icon::new(MaterialIcon::FolderFilled)
                        } else {
                            Icon::new(IconName::File)
                        };
                        fallback.size(icon_size).into_any_element()
                    };
                    let folder_help = self.folder_icon_description();
                    let kind = if row.parent {
                        "parent"
                    } else if row.directory {
                        "folder"
                    } else {
                        "file"
                    };
                    let marker = format!("archive-kind-{kind}-{}", row.name);
                    content = content.child(
                        div()
                            .id(gpui_kit::SharedString::from(format!(
                                "archive-kind-icon-{:?}",
                                row.id
                            )))
                            .debug_selector(move || marker.clone())
                            .flex_none()
                            .size(icon_size)
                            .when(row.directory && !row.parent, |slot| {
                                slot.tooltip(move |window, cx| {
                                    gpui_kit::component::tooltip::Tooltip::new(folder_help)
                                        .build(window, cx)
                                })
                            })
                            .when(self.icon_path(row).is_some(), |slot| {
                                slot.debug_selector(|| "archive-shell-type-icon".into())
                            })
                            .child(icon),
                    );
                }
                let text = cell(row, kind);
                let tooltip_text = text.clone();
                content
                    .child(
                        div()
                            .id(gpui_kit::SharedString::from(format!(
                                "archive-cell-{:?}-{}",
                                row.id,
                                kind.index()
                            )))
                            .min_w_0()
                            .flex_1()
                            .text_ellipsis()
                            .when(
                                matches!(kind, ColumnKind::Size | ColumnKind::Packed),
                                |cell| cell.text_right(),
                            )
                            .child(text)
                            .map(|cell| {
                                #[cfg(test)]
                                let cell = cell.child(tests::render_repair_font_probe());
                                cell
                            })
                            .tooltip(move |window, cx| {
                                gpui_kit::component::tooltip::Tooltip::new(tooltip_text.clone())
                                    .build(window, cx)
                            }),
                    )
                    .into_any_element()
            })
            .collect::<Vec<_>>();
        let trailing_cells = if self.settings.full_row_select {
            Vec::new()
        } else {
            cells.split_off(1)
        };
        let context = weak.clone();
        let item = ListItem::new(gpui_kit::SharedString::from(format!("archive-row-{id:?}")))
            .when(self.settings.show_grid_lines, |item| {
                item.border_b_1().border_color(cx.theme().border)
            })
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
            .w(if self.settings.full_row_select {
                self.columns_width()
            } else {
                self.column_width(ColumnKind::Name)
            })
            .px_0()
            .child(div().flex().w_full().children(cells))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event, _, cx| {
                    this.marquee.suppress_click = false;
                    this.drag_down(drag_id.clone(), snapshot, event);
                    cx.stop_propagation();
                }),
            )
            .on_click(
                cx.listener(move |this, event: &gpui_kit::ClickEvent, window, cx| {
                    if this.retired
                        || this.drag_click_suppressed()
                        || this.marquee_click_suppressed()
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
                    if event.click_count() == 2
                        || (this.settings.single_click_open && !event.modifiers().modified())
                    {
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
        let item = item.context_menu(move |menu, _, cx| {
            if let Some(view) = context.upgrade() {
                view.read(cx).row_menu(menu, context.clone())
            } else {
                menu
            }
        });
        if self.settings.full_row_select {
            item.into_any_element()
        } else {
            div()
                .flex()
                .items_center()
                .h(px(ROW_HEIGHT))
                .w(self.columns_width())
                .child(item)
                .children(trailing_cells)
                .into_any_element()
        }
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
    value.map_or_else(|| "—".into(), super::progress::format_bytes)
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
        ColumnKind::Size => {
            if row.directory {
                String::new()
            } else {
                bytes(row.size)
            }
        }
        ColumnKind::Packed => {
            if row.directory {
                String::new()
            } else {
                bytes(row.packed_size)
            }
        }
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
        let icon = match command {
            Command::Pick => MaterialIcon::Folder,
            Command::Extract(_) => MaterialIcon::Download,
            Command::New => MaterialIcon::FolderZip,
            Command::Edit(ArchiveJobModifyAction::Add) => MaterialIcon::Add,
            Command::Edit(ArchiveJobModifyAction::Delete) => MaterialIcon::Delete,
            Command::Test => MaterialIcon::Check,
            Command::Up => MaterialIcon::ArrowUpward,
            Command::Refresh => MaterialIcon::Undo,
            Command::Cancel => MaterialIcon::Cancel,
            _ => MaterialIcon::Settings,
        };
        let icon_selector = format!("{id}-icon");
        let label_selector = format!("{id}-label");
        let tooltip = if matches!(&command, Command::Edit(_)) && disabled {
            self.edit_reason()
                .unwrap_or_else(|| "请先选择可修改的条目。".into())
        } else {
            label.to_owned()
        };
        Button::new(id)
            .debug_selector(move || id.into())
            .accessibility_label(label)
            .tooltip(tooltip)
            .outline()
            .with_size(Size::Medium)
            .h(px(64.))
            .min_w(px(56.))
            .px_2()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .debug_selector(move || icon_selector.clone())
                            .size(px(24.))
                            .child(Icon::new(icon).size(px(24.))),
                    )
                    .child(
                        div()
                            .debug_selector(move || label_selector.clone())
                            .child(label),
                    ),
            )
            .disabled(disabled)
            .on_click(
                cx.listener(move |this, _, window, cx| this.command(command.clone(), window, cx)),
            )
    }
    fn split_action(&self, open: bool, ready: bool, cx: &mut Context<Self>) -> AnyElement {
        let weak = cx.entity().downgrade();
        let main = if open {
            self.action("archive-open", "打开…", Command::Pick, false, cx)
        } else {
            self.action(
                "archive-extract-all",
                "解压全部",
                Command::Extract(false),
                !ready,
                cx,
            )
        };
        let trigger = Button::new(if open {
            "archive-open-options"
        } else {
            "archive-extract-options"
        })
        .outline()
        .with_size(Size::Medium)
        .h_16()
        .w_6()
        .px_0()
        .dropdown_caret(true)
        .accessibility_label(if open { "打开选项" } else { "解压选项" })
        .disabled(!open && !ready)
        .dropdown_menu(move |menu, _, cx| {
            let Some(view) = weak.upgrade() else {
                return menu;
            };
            if open {
                menu.item(Self::menu_item(
                    weak.clone(),
                    "打开压缩包…",
                    Command::Pick,
                    false,
                ))
                .item(Self::menu_item(
                    weak.clone(),
                    "新建压缩包…",
                    Command::New,
                    false,
                ))
            } else {
                let view = view.read(cx);
                menu.item(Self::menu_item(
                    weak.clone(),
                    "解压全部…",
                    Command::Extract(false),
                    !matches!(view.status, BrowserStatus::Ready),
                ))
                .item(Self::menu_item(
                    weak.clone(),
                    "解压选定项目…",
                    Command::Extract(true),
                    !matches!(view.status, BrowserStatus::Ready)
                        || view
                            .model
                            .as_ref()
                            .is_none_or(|model| model.selected.is_empty()),
                ))
            }
        });
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(main)
            .child(trigger)
            .into_any_element()
    }
    fn tools_menu_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let weak = cx.entity().downgrade();
        Button::new("archive-tools-menu")
            .label("工具")
            .ghost()
            .with_size(Size::Small)
            .dropdown_menu(move |menu, _, cx| {
                let Some(view) = weak.upgrade() else {
                    return menu;
                };
                let view = view.read(cx);
                menu.item(Self::menu_item(
                    weak.clone(),
                    "测试压缩包  Ctrl+T",
                    Command::Test,
                    !matches!(view.status, BrowserStatus::Ready),
                ))
                .when(view.encrypted == Some(true), |menu| {
                    menu.item(Self::menu_item(
                        weak.clone(),
                        "解锁拖出",
                        Command::UnlockDrag,
                        !matches!(view.status, BrowserStatus::Ready),
                    ))
                })
                .separator()
                .item(Self::menu_item(
                    weak.clone(),
                    "刷新",
                    Command::Refresh,
                    matches!(view.status, BrowserStatus::Loading),
                ))
            })
            .into_any_element()
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
            .ghost()
            .with_size(Size::Small)
            .dropdown_menu(move |menu, window, cx| {
                if let Some(view) = weak.upgrade() {
                    if file {
                        let (recent, ready, loading) = {
                            let view = view.read(cx);
                            (
                                view.recent.clone(),
                                matches!(view.status, BrowserStatus::Ready),
                                matches!(view.status, BrowserStatus::Loading),
                            )
                        };
                        Self::file_menu(recent, ready, loading, menu, weak.clone(), window, cx)
                    } else {
                        view.read(cx).view_menu(menu, weak.clone())
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
            .ghost()
            .with_size(Size::Small)
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
            .filter(|kind| self.shown_column(*kind))
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
        let columns_key = columns_key | (usize::from(self.details_view) << 10);
        let header = h_resizable(("archive-columns", columns_key))
            .children(kinds.into_iter().map(|kind| {
                let marker = sort
                    .filter(|(sort, _)| *sort == kind.sort())
                    .map_or("", |(_, descending)| if descending { " ↓" } else { " ↑" });
                resizable_panel()
                    .size(px(sizes[kind.index()]))
                    .size_range(px(if kind == ColumnKind::Name { 240. } else { 64. })..px(640.))
                    .flex_none()
                    .child(
                        Button::new(("archive-sort", kind.index()))
                            .debug_selector(move || format!("archive-header-{}", kind.index()))
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
                            || !this.details_view
                            || (this
                                .columns
                                .iter()
                                .enumerate()
                                .fold(0usize, |mask, (index, shown)| {
                                    mask | (usize::from(*shown) << index)
                                })
                                | (1 << 10))
                                != columns_key
                        {
                            return;
                        }
                        for (kind, size) in kinds.iter().zip(sizes) {
                            this.widths[kind.index()] = f32::from(size)
                                .clamp(if *kind == ColumnKind::Name { 240. } else { 64. }, 640.);
                        }
                        cx.notify();
                    });
                });
            })
            .into_any_element();
        let header =
            if self.details_view {
                header
            } else {
                Button::new("archive-name-sort")
                    .debug_selector(|| "archive-header-0".into())
                    .accessibility_label("按名称排序")
                    .child(div().w_full().pl_6().child(if self.sort == SortKey::Name {
                        if self.descending {
                            "名称 ↓"
                        } else {
                            "名称 ↑"
                        }
                    } else {
                        "名称"
                    }))
                    .ghost()
                    .with_size(Size::Medium)
                    .w_full()
                    .px_3()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.sort_column(ColumnKind::Name, window, cx)
                    }))
                    .into_any_element()
            };
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
                count + 1,
                cx.processor(move |this, range: Range<usize>, window, cx| {
                    // Only the requested viewport range gets controls, cell strings and callbacks.
                    let rows = this.model.as_ref().map_or_else(Vec::new, |m| {
                        m.visible()[range.start.min(count)..range.end.min(count)].to_vec()
                    });
                    this.queue_type_icons(&rows, cx);
                    let mut rendered = rows
                        .iter()
                        .map(|row| this.render_row(row, window, cx))
                        .collect::<Vec<_>>();
                    if range.end > count {
                        rendered.push(div().w_full().h(px(ROW_HEIGHT)).into_any_element());
                    }
                    rendered
                }),
            )
            .w(width)
            .h_full()
            .track_scroll(&self.scroll)
            .map(|mut list| {
                list.style().restrict_scroll_to_axis = Some(true);
                list
            })
            .into_any_element()
        };
        let border = cx.theme().border;
        let focus = cx.theme().ring;
        let up_disabled = self
            .model
            .as_ref()
            .is_none_or(|model| model.folder.is_empty());
        let loading = matches!(self.status, BrowserStatus::Loading);
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
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("archive-up")
                            .icon(Icon::new(MaterialIcon::ArrowUpward))
                            .outline()
                            .with_size(Size::Small)
                            .tooltip("上一级 · Backspace")
                            .accessibility_label("上一级")
                            .disabled(up_disabled)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.command(Command::Up, window, cx)
                            })),
                    )
                    .child(
                        Button::new("archive-refresh")
                            .icon(Icon::new(IconName::RefreshCw))
                            .outline()
                            .with_size(Size::Small)
                            .tooltip("刷新")
                            .accessibility_label("刷新")
                            .disabled(loading)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.command(Command::Refresh, window, cx)
                            })),
                    )
                    .child(if self.filter_visible {
                        Input::new(&self.filter_input)
                            .w_full()
                            .aria_label("筛选当前文件夹")
                            .into_any_element()
                    } else {
                        div()
                            .min_w_0()
                            .flex_1()
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
                            .restrict_scroll_to_axis()
                            .test_support()
                            .debug_selector(|| "archive-horizontal".into())
                            .size_full()
                            .min_w_0()
                            .overflow_x_scroll()
                            .track_scroll(&self.horizontal)
                            .on_prepaint({
                                let view = cx.entity().downgrade();
                                let viewport = self.marquee.viewport.clone();
                                move |bounds, window, cx| {
                                    let width = bounds.size.width;
                                    viewport.set(bounds);
                                    let changed = view
                                        .read_with(cx, |view, _| {
                                            !view.retired
                                                && !view.details_view
                                                && view.name_view_width != width
                                        })
                                        .unwrap_or(false);
                                    if changed {
                                        let view = view.clone();
                                        window.defer(cx, move |_, cx| {
                                            let _ = view.update(cx, |view, cx| {
                                                if !view.retired
                                                    && !view.details_view
                                                    && view.name_view_width != width
                                                {
                                                    view.name_view_width = width;
                                                    cx.notify();
                                                }
                                            });
                                        });
                                    }
                                }
                            })
                            .child(
                                div()
                                    .w(width)
                                    .h_full()
                                    .flex()
                                    .flex_col()
                                    .child(div().w(width).h(px(32.)).flex_none().child(header))
                                    .child(
                                        div()
                                            .id("archive-selection-body")
                                            .debug_selector(|| "archive-selection-body".into())
                                            .relative()
                                            .w(width)
                                            .flex_1()
                                            .min_h_0()
                                            .on_prepaint({
                                                let bounds = self.marquee.body.clone();
                                                move |current, _, _| bounds.set(current)
                                            })
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, event, window, cx| {
                                                    this.marquee_down(event, window, cx)
                                                }),
                                            )
                                            .child(body),
                                    ),
                            ),
                    )
                    .child(self.marquee_overlay(cx))
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
        self.sync_preview_selection(cx);
        let loading = matches!(self.status, BrowserStatus::Loading);
        let ready = matches!(self.status, BrowserStatus::Ready);
        let title = self.tab_title();
        let menus = div()
            .debug_selector(|| "archive-menu-bar".into())
            .flex_none()
            .flex()
            .items_center()
            .gap_1()
            .p_1()
            .child(self.menu_button("archive-file-menu", "文件", true, cx))
            .child(self.edit_menu_button(cx))
            .child(self.menu_button("archive-view-menu", "查看", false, cx))
            .child(self.tools_menu_button(cx));
        let view_menu_owner = cx.entity().downgrade();
        let toolbar = Toolbar::new("archive-toolbar")
            .with_size(Size::Medium)
            .h_auto()
            .p_0()
            .content(
                div()
                    .debug_selector(|| "archive-icon-toolbar".into())
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_2()
                    .child(self.split_action(true, ready, cx))
                    .child(self.split_action(false, ready, cx))
                    .child(self.action("archive-new", "新建", Command::New, false, cx))
                    .child(gpui_kit::component::separator::Separator::vertical().h(px(48.)))
                    .child(self.action(
                        "archive-add",
                        "添加",
                        Command::Edit(ArchiveJobModifyAction::Add),
                        self.edit_reason().is_some(),
                        cx,
                    ))
                    .when(
                        matches!(self.format.as_str(), "SevenZ" | "Tar" | "TarGz" | "TarXz"),
                        |bar| {
                            bar.child(
                                self.action(
                                    "archive-replace-content",
                                    "替换内容",
                                    Command::Edit(ArchiveJobModifyAction::ReplaceContent),
                                    self.edit_selection(ArchiveJobModifyAction::ReplaceContent)
                                        .is_none(),
                                    cx,
                                ),
                            )
                        },
                    )
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
                    .child(self.action("archive-test", "测试", Command::Test, !ready, cx))
                    .child(gpui_kit::component::separator::Separator::vertical().h(px(48.)))
                    .when(self.encrypted == Some(true), |bar| {
                        bar.child(self.action(
                            "archive-unlock-drag",
                            "解锁拖出",
                            Command::UnlockDrag,
                            !ready,
                            cx,
                        ))
                    })
                    .when(loading, |bar| {
                        bar.child(self.action(
                            "archive-cancel",
                            "取消读取",
                            Command::Cancel,
                            false,
                            cx,
                        ))
                    })
                    .child(
                        Button::new("archive-view")
                            .debug_selector(|| "archive-view".into())
                            .outline()
                            .with_size(Size::Medium)
                            .h(px(64.))
                            .min_w(px(56.))
                            .accessibility_label("查看选项")
                            .tooltip("查看选项")
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap_1()
                                    .child(Icon::new(MaterialIcon::Dashboard).size(px(24.)))
                                    .child("查看 ▾"),
                            )
                            .dropdown_menu(move |menu, _, cx| {
                                if let Some(view) = view_menu_owner.upgrade() {
                                    view.read(cx).view_menu(menu, view_menu_owner.clone())
                                } else {
                                    menu
                                }
                            }),
                    ),
            );
        let content = if self.model.is_some() {
            let details = self.details(window, cx);
            if self.folder_count > 0 {
                self.queue_folder_icon(cx);
            }
            let folder_icon = self.folder_icon_path().cloned();
            let folder_help = self.folder_icon_description();
            let tree_drag_owner = cx.entity().downgrade();
            let tree_drag_snapshot = self.model.as_ref().map(|model| model.snapshot);
            let tree = Tree::new(&self.tree, move |ix, entry, _, _, _| {
                let name = entry.item().label.clone();
                let path = entry.item().id.to_string();
                let label_selector = format!("archive-tree-label-{path}");
                let icon_selector = format!("archive-tree-icon-{path}");
                let disclosure_selector = format!("archive-tree-disclosure-{path}");
                let icon_owner = tree_drag_owner.clone();
                let label_owner = tree_drag_owner.clone();
                let click_owner = tree_drag_owner.clone();
                let icon_path = path.clone();
                let label_path = path.clone();
                ListItem::new(("archive-tree-item", ix))
                    .accessibility_label(if name.is_empty() {
                        "压缩包".into()
                    } else {
                        name.clone()
                    })
                    .w_full()
                    .min_w_0()
                    .h(px(ROW_HEIGHT))
                    .overflow_hidden()
                    .on_click(move |_, _, cx| {
                        if click_owner
                            .read_with(cx, |view, _| view.tree_drag_click())
                            .unwrap_or(true)
                        {
                            cx.stop_propagation();
                        }
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .w_full()
                            .min_w_0()
                            .gap_1()
                            .pl(px(entry.depth() as f32 * 16.))
                            .child(div().flex_none().size(px(16.)).when(
                                entry.is_folder(),
                                |slot| {
                                    slot.debug_selector(move || disclosure_selector.clone())
                                        .child(
                                            Icon::new(if entry.is_expanded() {
                                                MaterialIcon::ExpandMore
                                            } else {
                                                MaterialIcon::ChevronRight
                                            })
                                            .size(px(16.)),
                                        )
                                },
                            ))
                            .child(
                                div()
                                    .debug_selector(move || icon_selector.clone())
                                    .flex_none()
                                    .size(px(20.))
                                    .id(("archive-tree-type", ix))
                                    .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                                        let _ = icon_owner.update(cx, |view, _| {
                                            view.tree_drag_down(
                                                icon_path.clone(),
                                                tree_drag_snapshot,
                                                event,
                                            )
                                        });
                                    })
                                    .tooltip(move |window, cx| {
                                        gpui_kit::component::tooltip::Tooltip::new(folder_help)
                                            .build(window, cx)
                                    })
                                    .child(if entry.is_root() {
                                        gpui_kit::img("brand/archive-40.png")
                                            .size_full()
                                            .flex_none()
                                            .object_fit(gpui_kit::ObjectFit::Contain)
                                            .into_any_element()
                                    } else if let Some(path) = &folder_icon {
                                        gpui_kit::img(path.clone())
                                            .size_full()
                                            .flex_none()
                                            .object_fit(gpui_kit::ObjectFit::Contain)
                                            .into_any_element()
                                    } else {
                                        Icon::new(MaterialIcon::FolderFilled)
                                            .size(px(20.))
                                            .into_any_element()
                                    }),
                            )
                            .child(
                                div()
                                    .id(("archive-tree-label", ix))
                                    .debug_selector(move || label_selector.clone())
                                    .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                                        let _ = label_owner.update(cx, |view, _| {
                                            view.tree_drag_down(
                                                label_path.clone(),
                                                tree_drag_snapshot,
                                                event,
                                            )
                                        });
                                    })
                                    .min_w_0()
                                    .flex_1()
                                    .text_ellipsis()
                                    .child(name.clone())
                                    .tooltip(move |window, cx| {
                                        gpui_kit::component::tooltip::Tooltip::new(name.clone())
                                            .build(window, cx)
                                    }),
                            ),
                    )
            })
            .min_w_0()
            .overflow_hidden();
            h_resizable("archive-browser-split")
                .child(
                    resizable_panel()
                        .size(px(200.))
                        .size_range(px(160.)..px(280.))
                        .fixed()
                        .flex_none()
                        .visible(self.show_tree)
                        .child(
                            div()
                                .debug_selector(|| "archive-tree-pane".into())
                                .size_full()
                                .min_w_0()
                                .min_h_0()
                                .overflow_hidden()
                                .child(tree),
                        ),
                )
                .child(
                    resizable_panel()
                        .size_range(px(160.)..Pixels::MAX)
                        .min_w_0()
                        .w_0()
                        .flex_1()
                        .child(details),
                )
                .when(self.preview_visible(), |this| {
                    this.child(
                        resizable_panel()
                            .size(px(280.))
                            .size_range(px(200.)..px(600.))
                            .fixed()
                            .flex_none()
                            .child(self.render_preview(cx)),
                    )
                })
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
                    if self.recovery.is_some() {
                        div().into_any_element()
                    } else {
                        Alert::error("archive-initial-error", message.clone())
                            .title("无法读取压缩包")
                            .into_any_element()
                    }
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
            BrowserStatus::Stale(_) => "文件已变化 · 只读",
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
            .font_family(crate::ui::typography::UI_FONT_FAMILY)
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
            .capture_any_mouse_down(cx.listener(|this, _, _, _| this.tree_drag_reset()))
            .on_mouse_move(cx.listener(|this, event, window, cx| {
                if !this.marquee_move(event, cx) {
                    this.drag_move(event, window, cx);
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.drag_up();
                    this.marquee_up(cx);
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.drag_up();
                    this.marquee_up(cx);
                }),
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
            .when(!self.hosted, |view| {
                view.child(crate::ui::components::titlebar::client_titlebar(
                    crate::theme::Theme::active(cx),
                    "archive-browser-titlebar",
                    "archive-browser-window-close",
                    title,
                    |_, window, _| window.remove_window(),
                ))
            })
            .child(menus)
            .child(toolbar)
            .when(loading && self.model.is_some(), |view| {
                view.child(
                    Alert::info(
                        "archive-reading",
                        "正在重新读取；原目录暂不可操作，完成后更新。",
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
                            "读取已取消；原目录暂不可操作。点击“刷新”重新读取。",
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
                            format!("{message} · 当前目录仅供查看；请刷新后再操作。"),
                        )
                        .banner(),
                    )
                },
            )
            .when_some(self.recovery.as_ref(), |view, recovery| {
                view.child(div().flex_none().p_3().flex().flex_col().gap_2()
                    .debug_selector(|| "archive-browser-recovery".into())
                    .child(Alert::warning("archive-browser-recovery-status", if recovery.completed {
                        "恢复记录已完成。原包已经过空间回收，不能按普通压缩包浏览；输出文件的当前状态尚未检查。"
                    } else {
                        "此压缩包有未完成的空间回收记录，不能按普通压缩包浏览。可检查原目标后继续解压；这不会还原已回收的原包。"
                    }).title(if recovery.completed { "空间回收解压已完成" } else { "可继续原空间回收任务" }))
                    .when(!recovery.completed, |body| body.child(
                        Button::new("archive-browser-reclamation-continue")
                            .debug_selector(|| "archive-browser-reclamation-continue".into())
                            .label("检查并继续解压…").outline().with_size(Size::Medium)
                            .disabled(self.open_pending)
                            .on_click(cx.listener(|this, _, window, cx| this.command(Command::ContinueReclamation, window, cx))))))
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
