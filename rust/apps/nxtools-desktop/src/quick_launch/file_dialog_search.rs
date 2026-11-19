use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::base::Root;
use gpui_kit::component::Icon;
use gpui_kit::component::input::{
    Input, InputEvent, InputGroup, InputGroupAddon, InputGroupAddonAlignment, InputState,
};
use gpui_kit::{
    Animation, AnimationExt, AnyWindowHandle, App, Bounds, Context, Entity, EntityInputHandler,
    Focusable, HighlightStyle, IntoElement, ObjectFit, Render, ScrollStrategy, SharedString,
    StyledText, Subscription, UniformListScrollHandle, WeakEntity, Window,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions, div,
    ease_out_quint, img, point, prelude::*, px, size, uniform_list,
};
use nxtools_finder::{ApplicationSource, SearchResult, query_requests_file_index_with_commands};
use nxtools_protocol::{
    ActivateWindowCommand, DirectoryNavigationHost, IndexSearchBatch, IndexSearchCancel,
    IndexSearchRequest, IndexerCommand, NavigateDirectoryCommand, OpenFileInDialogCommand,
    PlaceFileDialogSearchWindowCommand, PlatformCommand, QuickLaunchFileDialogSearchHost,
    SetWindowOwnerCommand, SetWindowTopmostCommand, SetWindowVisibilityCommand,
};
use tracing::{info, warn};

use super::actions::{QuickLaunchActionTrigger, quick_launch_primary_action};
use super::panel::source_icon;
use super::retained_text::QuickLaunchResults;
use super::search::{
    IndexSearchSession, elapsed_microseconds, index_status_has_searchable_snapshot,
    is_navigable_directory_target, quick_launch_match_ranges, should_search_indexer,
    stable_indexed_results,
};
use super::{
    ICON_PREFETCH_LIMIT, INDEX_CANDIDATE_LIMIT, ImeEnterGate, QuickLaunchApplication,
    indexed_search_entries, schedule_application_update,
};
use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::components::state::{empty_state, progress_state};
use crate::ui::metrics::{
    COMPACT_RESULT_ICON_RAIL_WIDTH, COMPACT_RESULT_ROW_HEIGHT, ICON_GLYPH_SMALL, SPACING_8,
    SPACING_12,
};
use crate::ui::native_window_handle;
use crate::ui::typography::{LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL};

const SEARCH_WIDTH: f32 = 500.0;
const SEARCH_BAR_HEIGHT: f32 = 36.0;
const DEFAULT_SOURCE_HEIGHT: f32 = 36.0;
const SEARCH_RESULTS_HEIGHT: f32 = 264.0;
const SEARCH_FRAME_HEIGHT: f32 = 2.0;
const SEARCH_HORIZONTAL_MARGIN: f32 = 16.0;
const SEARCH_DEBOUNCE: Duration = Duration::from_millis(65);
const INDEX_FIRST_PUBLISH_DEADLINE: Duration = Duration::from_millis(150);
const SEARCH_APPEAR_DURATION: Duration = Duration::from_millis(80);
const FILE_DIALOG_SEARCH_GENERATION_MASK: u64 = 1 << 63;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FileDialogSearchNativeBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileDialogSearchPlacementKey {
    host_window_handle: i64,
    search_window_handle: i64,
    bounds: FileDialogSearchNativeBounds,
    bottom_overlap: i32,
    visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileDialogDefaultSource {
    name: String,
    path: String,
}

fn file_dialog_default_source(
    host: &QuickLaunchFileDialogSearchHost,
) -> Option<FileDialogDefaultSource> {
    let source = host.quick_switch_source.as_ref()?;
    let path = source
        .context
        .current_directory
        .as_deref()
        .filter(|path| Path::new(path).is_absolute())?;
    Some(FileDialogDefaultSource {
        name: source.display_name.clone(),
        path: path.into(),
    })
}

fn file_dialog_search_result_height(host: &QuickLaunchFileDialogSearchHost) -> f32 {
    let scale = host.monitor_dpi.max(96) as f32 / 96.0;
    SEARCH_RESULTS_HEIGHT.min((host.height.max(0) as f32 / scale).floor())
}

pub(super) fn file_dialog_search_bounds(
    host: &QuickLaunchFileDialogSearchHost,
    expanded: bool,
) -> FileDialogSearchNativeBounds {
    let scale = host.monitor_dpi.max(96) as f32 / 96.0;
    let horizontal_margin = (SEARCH_HORIZONTAL_MARGIN * scale).round() as i32;
    let maximum_width = host.width.saturating_sub(horizontal_margin * 2).max(1);
    let desired_width = (SEARCH_WIDTH * scale).round().max(1.0) as i32;
    let width = desired_width.min(maximum_width);
    let bar_height = (SEARCH_BAR_HEIGHT * scale).round().max(1.0) as i32;
    let results_height = if expanded {
        (file_dialog_search_result_height(host) * scale)
            .round()
            .max(1.0) as i32
    } else {
        0
    };
    let default_source_height = if !expanded && file_dialog_default_source(host).is_some() {
        (DEFAULT_SOURCE_HEIGHT * scale).round().max(1.0) as i32
    } else {
        0
    };
    let x = host.left + (host.width - width) / 2;
    let dialog_bottom = host.top.saturating_add(host.height);
    FileDialogSearchNativeBounds {
        x,
        y: dialog_bottom.saturating_sub(results_height),
        width,
        height: bar_height
            .saturating_add(results_height)
            .saturating_add((SEARCH_FRAME_HEIGHT * scale).round().max(1.0) as i32)
            .saturating_add(default_source_height),
    }
}

/// The dialog sends the launcher's ordinary index request. Its current folder only ranks
/// results, `recent:` stays ordinary text, and every generation carries the dialog routing bit.
fn file_dialog_search_request(
    index: &mut IndexSearchSession,
    host: &QuickLaunchFileDialogSearchHost,
    query: &str,
    local_results_empty: bool,
) -> IndexSearchRequest {
    index.generation |= FILE_DIALOG_SEARCH_GENERATION_MASK;
    let mut request = index.begin(query, INDEX_CANDIDATE_LIMIT);
    request.context.preferred_directory = host.current_directory.clone();
    request.context.recent_modified = false;
    request.context.allow_homophone_fallback = local_results_empty;
    request
}

fn same_directory(left: &str, right: &str) -> bool {
    let normalize = |path: &str| {
        path.replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    normalize(left) == normalize(right)
}

/// Indexed rows whose parent is the dialog's current folder move ahead of the other indexed
/// rows. Application, command and calculator rows keep the slots the unified ranking gave them.
fn prefer_dialog_directory(
    results: Vec<SearchResult>,
    directory: Option<&str>,
) -> Vec<SearchResult> {
    let Some(directory) = directory.filter(|directory| !directory.trim().is_empty()) else {
        return results;
    };
    let slots = results
        .iter()
        .enumerate()
        .filter(|(_, result)| {
            matches!(
                result.entry.source,
                ApplicationSource::IndexedFile | ApplicationSource::IndexedDirectory
            )
        })
        .map(|(slot, _)| slot)
        .collect::<Vec<_>>();
    let mut rows = results.into_iter().map(Some).collect::<Vec<_>>();
    let (preferred, others): (Vec<SearchResult>, Vec<SearchResult>) = slots
        .iter()
        .filter_map(|slot| rows[*slot].take())
        .partition(|result| {
            Path::new(&result.entry.target)
                .parent()
                .and_then(|parent| parent.to_str())
                .is_some_and(|parent| same_directory(parent, directory))
        });
    for (slot, result) in slots.into_iter().zip(preferred.into_iter().chain(others)) {
        rows[slot] = Some(result);
    }
    rows.into_iter().flatten().collect()
}

#[derive(Default)]
pub(super) struct FileDialogSearchSession {
    host: Option<QuickLaunchFileDialogSearchHost>,
    window: Option<FileDialogSearchWindow>,
    index: IndexSearchSession,
    pending_index_search: Option<IndexSearchCancel>,
    last_placement: Option<FileDialogSearchPlacementKey>,
}

impl FileDialogSearchSession {
    fn invalidate_index(&mut self) {
        self.index.generation |= FILE_DIALOG_SEARCH_GENERATION_MASK;
        self.index.invalidate();
    }
}

struct FileDialogSearchWindow {
    window: AnyWindowHandle,
    panel: WeakEntity<FileDialogSearchPanel>,
    native_window_handle: i64,
}

pub(super) struct FileDialogSearchPanel {
    theme: Theme,
    application: WeakEntity<QuickLaunchApplication>,
    host_window_handle: i64,
    current_directory: Option<String>,
    default_source: Option<FileDialogDefaultSource>,
    search_input: Entity<InputState>,
    results: QuickLaunchResults,
    committed_query: String,
    icons: HashMap<String, PathBuf>,
    selected: usize,
    search_pending: bool,
    expanded: bool,
    result_height: f32,
    dispatch_generation: u64,
    visible: bool,
    appearance_generation: u64,
    result_scroll: UniformListScrollHandle,
    ime_enter_gate: ImeEnterGate,
    _subscriptions: Vec<Subscription>,
}

impl FileDialogSearchPanel {
    fn new(
        host: &QuickLaunchFileDialogSearchHost,
        application: WeakEntity<QuickLaunchApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索文件、文件夹或应用"));
        let change_subscription = cx.subscribe_in(
            &search_input,
            window,
            |this, state, event: &InputEvent, _window, cx| match event {
                InputEvent::Change => {
                    let query = state.read(cx).value().to_string();
                    let expanded = !query.trim().is_empty();
                    this.ime_enter_gate.observe_change();
                    this.dispatch_generation = this.dispatch_generation.wrapping_add(1).max(1);
                    this.search_pending = expanded;
                    if !expanded {
                        this.results.clear();
                        this.committed_query.clear();
                        this.selected = 0;
                    }
                    if this.expanded != expanded {
                        this.expanded = expanded;
                        let application = this.application.clone();
                        schedule_application_update(
                            application,
                            |application, cx| {
                                application.apply_file_dialog_search_window_placement(cx);
                            },
                            cx,
                        );
                    }
                    this.schedule_search(query, cx);
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => {
                    if this.ime_enter_gate.consume_enter() {
                        this.activate_selected(QuickLaunchActionTrigger::Enter, cx);
                    }
                }
                InputEvent::Focus | InputEvent::Blur => {}
            },
        );
        let composition_subscription =
            cx.observe_in(&search_input, window, |this, state, window, cx| {
                let composing = state.update(cx, |state, cx| {
                    state.marked_text_range(window, cx).is_some()
                });
                this.ime_enter_gate.observe_composition(composing);
            });
        let key_listener = cx.listener(|this, event: &gpui_kit::KeystrokeEvent, window, cx| {
            if !this.visible || !this.search_input.focus_handle(cx).is_focused(window) {
                return;
            }
            let key = event.keystroke.key.as_str();
            let handled = if event.keystroke.modifiers.control && key.eq_ignore_ascii_case("g") {
                this.activate_default_source(cx);
                true
            } else if key.eq_ignore_ascii_case("up") {
                this.move_selection(-1);
                true
            } else if key.eq_ignore_ascii_case("down") {
                this.move_selection(1);
                true
            } else if key.eq_ignore_ascii_case("escape") {
                this.escape(window, cx);
                true
            } else {
                false
            };
            if handled {
                cx.stop_propagation();
                cx.notify();
            }
        });
        let key_subscription = cx.intercept_keystrokes(key_listener);
        Self {
            theme: Theme::zed_dark(),
            application,
            host_window_handle: host.window_handle,
            current_directory: host.current_directory.clone(),
            default_source: file_dialog_default_source(host),
            search_input,
            results: Vec::new().into(),
            committed_query: String::new(),
            icons: HashMap::new(),
            selected: 0,
            search_pending: false,
            expanded: false,
            result_height: file_dialog_search_result_height(host),
            dispatch_generation: 0,
            visible: host.visible,
            appearance_generation: u64::from(host.visible),
            result_scroll: UniformListScrollHandle::new(),
            ime_enter_gate: ImeEnterGate::default(),
            _subscriptions: vec![
                change_subscription,
                composition_subscription,
                key_subscription,
            ],
        }
    }

    fn schedule_search(&mut self, query: String, cx: &mut Context<Self>) {
        let dispatch_generation = self.dispatch_generation;
        let panel = cx.entity().downgrade();
        cx.spawn(async move |_this, cx| {
            cx.background_executor().timer(SEARCH_DEBOUNCE).await;
            let _ = panel.update(cx, |panel, cx| {
                if !panel.visible
                    || panel.dispatch_generation != dispatch_generation
                    || panel.search_input.read(cx).value().as_ref() != query
                {
                    return;
                }
                let application = panel.application.clone();
                schedule_application_update(
                    application,
                    move |application, cx| {
                        application.begin_file_dialog_search(query, cx);
                    },
                    cx,
                );
            });
        })
        .detach();
    }

    fn set_host(
        &mut self,
        host: &QuickLaunchFileDialogSearchHost,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let directory_changed = self.current_directory != host.current_directory;
        self.host_window_handle = host.window_handle;
        self.current_directory = host.current_directory.clone();
        self.default_source = file_dialog_default_source(host);
        self.result_height = file_dialog_search_result_height(host);
        if host.visible && !self.visible {
            self.appearance_generation = self.appearance_generation.wrapping_add(1).max(1);
        }
        self.visible = host.visible;
        if directory_changed {
            self.results.clear();
            self.committed_query.clear();
            self.selected = 0;
            self.search_pending = false;
            self.expanded = false;
            self.dispatch_generation = self.dispatch_generation.wrapping_add(1).max(1);
            self.search_input.update(cx, |state, cx| {
                state.set_value(String::new(), window, cx);
            });
        }
        cx.notify();
    }

    fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if visible && !self.visible {
            self.appearance_generation = self.appearance_generation.wrapping_add(1).max(1);
        }
        self.visible = visible;
        cx.notify();
    }

    fn set_pending(&mut self, cx: &mut Context<Self>) {
        if !self.search_pending {
            self.search_pending = true;
            cx.notify();
        }
    }

    /// Publishes rows for the query still shown in the input; a later merge of the same query
    /// keeps already admitted rows in place, exactly like the launcher panel.
    fn publish_results(
        &mut self,
        query: &str,
        results: Vec<SearchResult>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.search_input.read(cx).value().as_ref() != query {
            return false;
        }
        let same_query = self.committed_query == query;
        let results = if same_query && !self.search_pending {
            stable_indexed_results(&self.results, results)
        } else {
            results
        };
        if !same_query {
            self.selected = 0;
        }
        self.icons
            .retain(|entry_id, _| results.iter().any(|result| result.entry.id == *entry_id));
        self.results = results.into();
        self.committed_query = query.into();
        self.search_pending = false;
        self.selected = self.selected.min(self.results.len().saturating_sub(1));
        self.ensure_selected_visible();
        cx.notify();
        true
    }

    fn set_icons(&mut self, icons: Vec<(String, PathBuf)>, cx: &mut Context<Self>) {
        let mut changed = false;
        for (entry_id, path) in icons {
            if self.icons.get(&entry_id) != Some(&path) {
                self.icons.insert(entry_id, path);
                changed = true;
            }
        }
        if changed {
            cx.notify();
        }
    }

    fn move_selection(&mut self, delta: isize) {
        if self.results.is_empty() {
            self.selected = 0;
            return;
        }
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(self.results.len() - 1);
        self.ensure_selected_visible();
    }

    fn ensure_selected_visible(&self) {
        if !self.results.is_empty() {
            self.result_scroll
                .scroll_to_item(self.selected, ScrollStrategy::Top);
        }
    }

    fn activate_selected(&self, trigger: QuickLaunchActionTrigger, cx: &mut App) {
        if self.search_pending {
            return;
        }
        let Some(result) = self.results.get(self.selected).cloned() else {
            self.activate_default_source(cx);
            return;
        };
        let query = self.committed_query.clone();
        let application = self.application.clone();
        schedule_application_update(
            application,
            move |application, cx| {
                application.activate_file_dialog_search_result(result, query, trigger, cx);
            },
            cx,
        );
    }

    fn activate_default_source(&self, cx: &mut App) {
        let Some(source) = self.default_source.clone() else {
            return;
        };
        let application = self.application.clone();
        schedule_application_update(
            application,
            move |application, cx| {
                application.activate_file_dialog_default_source(source.path, cx);
            },
            cx,
        );
    }

    fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.search_input.read(cx).value().is_empty() {
            self.search_input.update(cx, |state, cx| {
                state.set_value(String::new(), window, cx);
            });
            self.dispatch_generation = self.dispatch_generation.wrapping_add(1).max(1);
            self.ime_enter_gate.reset();
            self.search_pending = false;
            self.results.clear();
            self.selected = 0;
            self.expanded = false;
            schedule_application_update(
                self.application.clone(),
                |application, cx| {
                    application.apply_file_dialog_search_window_placement(cx);
                    application.begin_file_dialog_search(String::new(), cx);
                },
                cx,
            );
            cx.notify();
            return;
        }
        let host_window_handle = self.host_window_handle;
        let application = self.application.clone();
        schedule_application_update(
            application,
            move |application, _cx| {
                let _ = application
                    .platform
                    .send_command(PlatformCommand::ActivateWindow(ActivateWindowCommand {
                        window_handle: host_window_handle,
                    }));
            },
            cx,
        );
    }

    fn default_source_row(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui_kit::AnyElement> {
        let source = self.default_source.clone()?;
        let theme = self.theme;
        let panel = cx.entity().downgrade();
        let escape_panel = panel.clone();
        let row = div()
            .id("file-dialog-default-source")
            .active(move |row| row.bg(theme.surface_pressed))
            .debug_selector(|| "file-dialog-default-source".into())
            .w_full()
            .min_w_0()
            .h(px(DEFAULT_SOURCE_HEIGHT))
            .px(px(SPACING_12))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(SPACING_8))
            .border_t_1()
            .border_color(theme.border_subtle)
            .bg(theme.canvas)
            .hover(move |row| row.bg(theme.surface_hover))
            .cursor_pointer()
            .child(
                div()
                    .debug_selector(|| "file-dialog-default-source-icon".into())
                    .size(px(COMPACT_RESULT_ICON_RAIL_WIDTH))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        gpui_kit::component::Icon::new(MaterialIcon::Folder)
                            .size(px(16.0))
                            .text_color(theme.icon),
                    ),
            )
            .child(
                div()
                    .debug_selector(|| "file-dialog-default-source-title".into())
                    .min_w_0()
                    .flex_1()
                    .max_w(px(192.0))
                    .h(px(LINE_HEIGHT_BODY))
                    .truncate()
                    .text_size(px(TEXT_BODY))
                    .line_height(px(LINE_HEIGHT_BODY))
                    .text_color(theme.text)
                    .child(source.name),
            )
            .child(
                div()
                    .debug_selector(|| "file-dialog-default-source-path".into())
                    .min_w_0()
                    .flex_1()
                    .h(px(LINE_HEIGHT_LABEL))
                    .truncate()
                    .text_size(px(TEXT_BODY))
                    .line_height(px(LINE_HEIGHT_LABEL))
                    .text_color(theme.text_muted)
                    .child(source.path),
            )
            .child(
                div()
                    .debug_selector(|| "file-dialog-default-source-shortcut".into())
                    // Released Ctrl+G Kbd needs52px; borrow4px from the elastic path, not its glyph.
                    .w(px(52.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_end()
                    .text_size(px(TEXT_LABEL))
                    .line_height(px(LINE_HEIGHT_LABEL))
                    .text_color(theme.text_muted)
                    .child(crate::ui::components::shortcut::key("ctrl-g")),
            )
            .into_any_element();
        Some(
            super::panel::quick_launch_focusable_row(
                "file-dialog-default-source-focus",
                theme,
                false,
                row,
                window,
                cx,
                move |_event, _window, cx| {
                    let _ = panel.update(cx, |panel, cx| panel.activate_default_source(cx));
                },
            )
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key != "escape" || event.keystroke.modifiers.modified() {
                    return;
                }
                let _ = escape_panel.update(cx, |panel, cx| {
                    panel.search_input.focus_handle(cx).focus(window, cx);
                    panel.escape(window, cx);
                });
                window.prevent_default();
                cx.stop_propagation();
            })
            .min_w_0()
            .flex_none()
            .into_any_element(),
        )
    }

    fn row(
        &self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let result = &self.results[index];
        let display = self.results.display(index);
        let title = display.title.clone();
        let subtitle = if display.subtitle.is_empty() {
            SharedString::from(result.entry.source.label())
        } else {
            display.subtitle.clone()
        };
        let source = result.entry.source;
        let icon_path = self.icons.get(&result.entry.id).cloned();
        let selected = index == self.selected;
        let theme = self.theme;
        let name_highlights = if result
            .subtitle
            .as_deref()
            .is_some_and(|label| label.contains("同音近似"))
        {
            nxtools_finder::FileQuery::parse(&self.committed_query)
                .homophone_match_ranges(&result.entry.name)
        } else {
            quick_launch_match_ranges(&result.entry.name, &self.committed_query)
        }
        .into_iter()
        .map(|range| {
            (
                range,
                HighlightStyle {
                    color: Some(theme.accent.into()),
                    font_weight: Some(gpui_kit::FontWeight::SEMIBOLD),
                    ..Default::default()
                },
            )
        })
        .collect::<Vec<_>>();
        let panel = cx.entity().downgrade();
        let click_panel = panel.clone();
        let navigation_panel = panel.clone();
        let focus_id = SharedString::from(format!("file-dialog-result-focus-{}", result.entry.id));
        let icon_rail = div()
            .debug_selector(move || format!("file-dialog-search-result-icon-{index}"))
            .size(px(COMPACT_RESULT_ICON_RAIL_WIDTH))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .text_color(if selected {
                theme.icon
            } else {
                theme.icon_muted
            });
        let icon_rail = match icon_path {
            Some(path) => icon_rail.child(
                img(path)
                    .size(px(16.0))
                    .object_fit(ObjectFit::Contain)
                    .with_fallback(move || {
                        Icon::new(source_icon(source))
                            .size(px(16.0))
                            .into_any_element()
                    }),
            ),
            None => icon_rail.child(Icon::new(source_icon(source)).size(px(16.0))),
        };
        let row = div()
            .id(("file-dialog-search-row", index))
            .active(move |row| row.bg(theme.surface_pressed))
            .debug_selector(move || format!("file-dialog-search-result-row-{index}"))
            .w_full()
            .h(px(COMPACT_RESULT_ROW_HEIGHT))
            .px(px(SPACING_12))
            .flex()
            .items_center()
            .gap(px(SPACING_8))
            .cursor_pointer()
            .bg(if selected {
                theme.element_selected
            } else {
                theme.canvas
            })
            .hover(move |row| {
                row.bg(if selected {
                    theme.element_selected
                } else {
                    theme.surface_hover
                })
            })
            .on_mouse_move(move |_event, _window, cx| {
                let _ = panel.update(cx, |panel, cx| {
                    if panel.selected != index {
                        panel.selected = index;
                        cx.notify();
                    }
                });
            })
            .child(icon_rail)
            .child(
                div()
                    .debug_selector(move || format!("file-dialog-search-result-copy-{index}"))
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(crate::ui::metrics::SPACING_4))
                    .child(
                        div()
                            .debug_selector(move || {
                                format!("file-dialog-search-result-title-{index}")
                            })
                            .h(px(LINE_HEIGHT_BODY))
                            .truncate()
                            .text_size(px(TEXT_BODY))
                            .line_height(px(LINE_HEIGHT_BODY))
                            .text_color(theme.text)
                            .child(StyledText::new(title).with_highlights(name_highlights)),
                    )
                    .child(
                        div()
                            .debug_selector(move || {
                                format!("file-dialog-search-result-path-{index}")
                            })
                            .h(px(LINE_HEIGHT_LABEL))
                            .truncate()
                            .text_size(px(TEXT_BODY))
                            .line_height(px(LINE_HEIGHT_LABEL))
                            .text_color(theme.text_muted)
                            .child(subtitle),
                    ),
            )
            .into_any_element();
        super::panel::quick_launch_focusable_row(
            focus_id,
            theme,
            false,
            row,
            window,
            cx,
            move |event, _window, cx| {
                let trigger = if event.is_keyboard() {
                    QuickLaunchActionTrigger::Enter
                } else {
                    QuickLaunchActionTrigger::Pointer
                };
                let _ = click_panel.update(cx, |panel, cx| {
                    panel.selected = index;
                    panel.activate_selected(trigger, cx);
                    cx.notify();
                });
            },
        )
        .on_key_down(move |event, window, cx| {
            if event.keystroke.modifiers.modified()
                || !matches!(event.keystroke.key.as_str(), "up" | "down" | "escape")
            {
                return;
            }
            let _ = navigation_panel.update(cx, |panel, cx| {
                panel.selected = index;
                panel.search_input.focus_handle(cx).focus(window, cx);
                match event.keystroke.key.as_str() {
                    "up" => panel.move_selection(-1),
                    "down" => panel.move_selection(1),
                    _ => panel.escape(window, cx),
                }
                cx.notify();
            });
            window.prevent_default();
            cx.stop_propagation();
        })
        .into_any_element()
    }
}

impl Render for FileDialogSearchPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::ui::motion::observe_motion(window, cx);
        crate::theme::observe_theme(window, cx);
        self.theme = Theme::active(cx);
        let theme = self.theme;
        let expanded = self.expanded;
        let appearance_generation = self.appearance_generation;
        let result_height = self.result_height;
        let result_count = self.results.len();
        let default_source = if expanded {
            None
        } else {
            self.default_source_row(window, cx)
        };
        let list = uniform_list(
            "file-dialog-search-results",
            result_count,
            cx.processor(|panel, range: Range<usize>, window, cx| {
                range.map(|index| panel.row(index, window, cx)).collect()
            }),
        )
        .size_full()
        .track_scroll(&self.result_scroll);
        let root = div()
            .id("file-dialog-search-window")
            .debug_selector(|| "file-dialog-search-window".into())
            .size_full()
            .overflow_hidden()
            .bg(theme.canvas)
            .border_1()
            .border_color(theme.window_border)
            .flex()
            .flex_col()
            .when(expanded, |root| {
                root.child(
                    div()
                        .h(px(result_height))
                        .flex_none()
                        .border_b_1()
                        .border_color(theme.border_subtle)
                        .bg(theme.canvas)
                        .when(result_count == 0, |surface| {
                            if self.search_pending {
                                surface.child(progress_state(
                                    theme,
                                    "file-dialog-search-progress",
                                    "正在搜索",
                                    "正在合并应用、文件与文件夹结果。",
                                ))
                            } else {
                                surface.child(empty_state(
                                    theme,
                                    "file-dialog-search-empty",
                                    MaterialIcon::Search,
                                    super::QUICK_LAUNCH_EMPTY_RESULTS_LABEL,
                                    "尝试缩短关键词，或输入应用、文件与文件夹名称。",
                                ))
                            }
                        })
                        .when(result_count > 0, |surface| surface.child(list)),
                )
            })
            .child(
                div()
                    .debug_selector(|| "file-dialog-search-header".into())
                    .h(px(SEARCH_BAR_HEIGHT))
                    .px(px(SPACING_12))
                    .flex_none()
                    .bg(theme.canvas)
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .debug_selector(|| "file-dialog-search-input".into())
                            .min_w_0()
                            .flex_1()
                            .child(
                                InputGroup::new("file-dialog-search-group")
                                    .aria_label("附着文件对话框搜索")
                                    .input(
                                        Input::new(&self.search_input)
                                            .aria_label("搜索文件、文件夹或应用"),
                                    )
                                    .addon(
                                        InputGroupAddon::new("file-dialog-search-icon")
                                            .child(Icon::new(MaterialIcon::Search)),
                                    )
                                    .when(self.search_pending, |group| {
                                        group.addon(
                                            InputGroupAddon::new("file-dialog-search-pending")
                                                .align(InputGroupAddonAlignment::InlineEnd)
                                                .child(
                                                    Icon::new(MaterialIcon::ProgressActivity)
                                                        .size(px(ICON_GLYPH_SMALL)),
                                                ),
                                        )
                                    }),
                            ),
                    ),
            )
            .when_some(default_source, |root, row| root.child(row));
        if crate::ui::motion::motion_allowed(cx) {
            root.with_animation(
                ("file-dialog-search-appear", appearance_generation),
                Animation::new(SEARCH_APPEAR_DURATION).with_easing(ease_out_quint()),
                |surface, delta| {
                    surface
                        .relative()
                        .top(px(1.0 * (1.0 - delta)))
                        .opacity(0.94 + 0.06 * delta)
                },
            )
            .into_any_element()
        } else {
            root.into_any_element()
        }
    }
}

impl QuickLaunchApplication {
    pub(super) fn update_file_dialog_search_directory(
        &mut self,
        window_handle: i64,
        directory: String,
        cx: &mut Context<Self>,
    ) {
        let Some(host) = self.file_dialog_search.host.as_mut() else {
            return;
        };
        if host.window_handle != window_handle
            || host
                .current_directory
                .as_deref()
                .is_some_and(|current| current.eq_ignore_ascii_case(&directory))
        {
            return;
        }
        host.current_directory = Some(directory);
        let host = host.clone();
        self.cancel_file_dialog_index_search("directory_changed", cx);
        self.file_dialog_search.invalidate_index();
        if let Some(search_window) = self.file_dialog_search.window.as_ref() {
            let panel = search_window.panel.clone();
            let window = search_window.window;
            let _ = window.update(cx, |_, window, app| {
                let _ = panel.update(app, |panel, cx| {
                    panel.set_host(&host, window, cx);
                });
            });
        }
        self.apply_file_dialog_search_window_placement(cx);
    }

    pub(super) fn handle_file_dialog_search_host(
        &mut self,
        host: QuickLaunchFileDialogSearchHost,
        cx: &mut Context<Self>,
    ) {
        if !self.config.quick_launch.enabled || !self.config.quick_launch.file_dialog_search {
            return;
        }
        self.file_dialog_search.host = Some(host.clone());
        if !host.visible {
            self.hide_file_dialog_search_window(cx);
            return;
        }
        if self.file_dialog_search.window.is_none() {
            self.open_file_dialog_search_window(&host, cx);
            let application = cx.entity().downgrade();
            schedule_application_update(
                application,
                |application, cx| {
                    application.initialize_file_dialog_search_window(cx);
                },
                cx,
            );
            return;
        } else {
            let window = self
                .file_dialog_search
                .window
                .as_ref()
                .map(|window| window.window);
            let panel = self
                .file_dialog_search
                .window
                .as_ref()
                .map(|window| window.panel.clone());
            if let (Some(window), Some(panel)) = (window, panel) {
                let _ = window.update(cx, |_, window, app| {
                    let _ = panel.update(app, |panel, cx| {
                        panel.set_host(&host, window, cx);
                    });
                });
            }
        }
        self.apply_file_dialog_search_window_placement(cx);
    }

    fn open_file_dialog_search_window(
        &mut self,
        host: &QuickLaunchFileDialogSearchHost,
        cx: &mut Context<Self>,
    ) {
        let native_bounds = file_dialog_search_bounds(host, false);
        let scale = host.monitor_dpi.max(96) as f32 / 96.0;
        let logical_bounds = Bounds::new(
            point(
                px(native_bounds.x as f32 / scale),
                px(native_bounds.y as f32 / scale),
            ),
            size(
                px(native_bounds.width as f32 / scale),
                px(native_bounds.height as f32 / scale),
            ),
        );
        let application = cx.entity().downgrade();
        let host_for_panel = host.clone();
        let panel_slot = Rc::new(RefCell::new(None));
        let native_slot = Rc::new(RefCell::new(0_i64));
        let opened_panel_slot = Rc::clone(&panel_slot);
        let opened_native_slot = Rc::clone(&native_slot);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(logical_bounds)),
                titlebar: None,
                focus: false,
                show: false,
                kind: WindowKind::PopUp,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                display_id: None,
                window_background: WindowBackgroundAppearance::Opaque,
                app_id: Some("NxTools.FileDialogSearch".into()),
                window_min_size: None,
                window_decorations: Some(WindowDecorations::Client),
                app_owns_titlebar_drag: false,
                inactive_frame_interval: None,
                icon: None,
                tabbing_identifier: None,
            },
            move |window, cx| {
                *opened_native_slot.borrow_mut() = native_window_handle(window).unwrap_or_default();
                let panel = cx
                    .new(|cx| FileDialogSearchPanel::new(&host_for_panel, application, window, cx));
                *opened_panel_slot.borrow_mut() = Some(panel.downgrade());
                cx.new(|cx| Root::new(panel, window, cx))
            },
        );
        match opened {
            Ok(window) => {
                let Some(panel) = panel_slot.borrow_mut().take() else {
                    let _ = window.update(cx, |_, window, _| window.remove_window());
                    return;
                };
                self.file_dialog_search.window = Some(FileDialogSearchWindow {
                    window: window.into(),
                    panel,
                    native_window_handle: *native_slot.borrow(),
                });
                self.file_dialog_search.last_placement = None;
                info!(
                    host_window_handle = host.window_handle,
                    "file-dialog search window opened"
                );
            }
            Err(error) => warn!(%error, "could not open file-dialog search window"),
        }
    }

    fn apply_file_dialog_search_window_placement(&mut self, cx: &mut Context<Self>) {
        let Some(host) = self.file_dialog_search.host.clone() else {
            return;
        };
        let Some(search_window) = self.file_dialog_search.window.as_ref() else {
            return;
        };
        let expanded = search_window
            .panel
            .update(cx, |panel, cx| {
                panel.visible && !panel.search_input.read(cx).value().trim().is_empty()
            })
            .unwrap_or(false);
        let bounds = file_dialog_search_bounds(&host, expanded);
        let scale = host.monitor_dpi.max(96) as f32 / 96.0;
        let results_height = if expanded {
            (file_dialog_search_result_height(&host) * scale)
                .round()
                .max(1.0) as i32
        } else {
            0
        };
        let native_window_handle = search_window.native_window_handle;
        if native_window_handle == 0 {
            return;
        }
        let placement_key = FileDialogSearchPlacementKey {
            host_window_handle: host.window_handle,
            search_window_handle: native_window_handle,
            bounds,
            bottom_overlap: results_height,
            visible: host.visible,
        };
        if self.file_dialog_search.last_placement == Some(placement_key) {
            return;
        }
        for command in [
            PlatformCommand::SetWindowOwner(SetWindowOwnerCommand {
                window_handle: native_window_handle,
                owner_window_handle: host.window_handle,
            }),
            PlatformCommand::PlaceFileDialogSearchWindow(PlaceFileDialogSearchWindowCommand {
                window_handle: native_window_handle,
                owner_window_handle: host.window_handle,
                visible_width: bounds.width,
                visible_height: bounds.height,
                bottom_overlap: results_height,
            }),
            PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                window_handle: native_window_handle,
                visible: host.visible,
            }),
        ] {
            if let Err(error) = self.platform.send_command(command) {
                warn!(%error, "could not place file-dialog search window");
            }
        }
        self.file_dialog_search.last_placement = Some(placement_key);
        if let Some(search_window) = self.file_dialog_search.window.as_ref() {
            let _ = search_window
                .panel
                .update(cx, |panel, cx| panel.set_visible(host.visible, cx));
        }
    }

    fn initialize_file_dialog_search_window(&mut self, cx: &mut Context<Self>) {
        let Some(search_window) = self.file_dialog_search.window.as_ref() else {
            return;
        };
        if search_window.native_window_handle == 0 {
            return;
        }
        if let Err(error) = self
            .platform
            .send_command(PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                window_handle: search_window.native_window_handle,
                enabled: true,
            }))
        {
            warn!(%error, "could not initialize file-dialog search window as topmost");
        }
        self.apply_file_dialog_search_window_placement(cx);
    }

    pub(super) fn hide_file_dialog_search_window(&mut self, cx: &mut Context<Self>) {
        let Some(search_window) = self.file_dialog_search.window.as_ref() else {
            return;
        };
        let _ = search_window
            .panel
            .update(cx, |panel, cx| panel.set_visible(false, cx));
        if search_window.native_window_handle != 0 {
            let _ = self
                .platform
                .send_command(PlatformCommand::SetWindowVisibility(
                    SetWindowVisibilityCommand {
                        window_handle: search_window.native_window_handle,
                        visible: false,
                    },
                ));
        }
        self.file_dialog_search.last_placement = None;
    }
    /// The dialog bar runs the launcher's ordinary query: local catalog sources, then the
    /// indexed provider under the same admission, first-merge deadline and Finder ranking.
    pub(super) fn begin_file_dialog_search(&mut self, query: String, cx: &mut Context<Self>) {
        let Some(host) = self.file_dialog_search.host.clone() else {
            return;
        };
        let started = Instant::now();
        if query.trim().is_empty() {
            self.cancel_file_dialog_index_search("cleared", cx);
            self.file_dialog_search.invalidate_index();
            self.publish_file_dialog_search_results(String::new(), Vec::new(), cx);
            self.apply_file_dialog_search_window_placement(cx);
            return;
        }
        let local_results = self.unified_catalog_results(&query, &[], true);
        let request = file_dialog_search_request(
            &mut self.file_dialog_search.index,
            &host,
            &query,
            local_results.is_empty(),
        );
        let generation = request.generation;
        let indexer_connected = self.indexer.is_connected();
        let index_available = index_status_has_searchable_snapshot(self.index_status.as_ref());
        let search_indexer = should_search_indexer(indexer_connected, index_available)
            && query_requests_file_index_with_commands(
                &query,
                &self.config.quick_launch.custom_commands,
            );
        let search_queued = if search_indexer {
            self.dispatch_file_dialog_index_search(request)
        } else {
            self.cancel_file_dialog_index_search("provider_not_requested", cx);
            false
        };
        self.file_dialog_search.index.complete = !search_queued;
        info!(
            event = "quick_launch.file_dialog_search.accepted",
            generation,
            query_length = query.chars().count(),
            indexer_connected,
            index_available,
            search_queued,
            local_count = local_results.len(),
            elapsed_us = elapsed_microseconds(started),
            "file-dialog search accepted"
        );
        if search_queued {
            self.set_file_dialog_search_pending(cx);
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(INDEX_FIRST_PUBLISH_DEADLINE)
                    .await;
                let _ = this.update(cx, |application, cx| {
                    let index = &mut application.file_dialog_search.index;
                    if index.generation != generation || index.complete || index.first_published {
                        return;
                    }
                    index.deadline_elapsed = true;
                    application.refresh_file_dialog_search_results(cx);
                });
            })
            .detach();
            return;
        }
        let results = self.file_dialog_combined_results();
        self.publish_file_dialog_search_results(query, results, cx);
    }

    fn dispatch_file_dialog_index_search(&mut self, request: IndexSearchRequest) -> bool {
        let request_id = uuid::Uuid::new_v4();
        let previous = self
            .file_dialog_search
            .pending_index_search
            .replace(IndexSearchCancel {
                request_id,
                generation: request.generation,
            });
        match self
            .indexer
            .send_command_with_id(IndexerCommand::Search(request), request_id)
        {
            Ok(_) => true,
            Err(error) => {
                self.file_dialog_search.pending_index_search = previous;
                warn!(%error, "could not queue file-dialog indexed search");
                false
            }
        }
    }

    fn cancel_file_dialog_index_search(&mut self, reason: &'static str, cx: &mut Context<Self>) {
        let Some(cancel) = self.file_dialog_search.pending_index_search.take() else {
            return;
        };
        let indexer = Rc::clone(&self.indexer);
        info!(
            event = "quick_launch.file_dialog_search.cancel_requested",
            correlation_id = %cancel.request_id,
            generation = cancel.generation,
            reason,
            "file-dialog indexed search retired locally"
        );
        cx.spawn(async move |_, _| {
            let queued = indexer.cancel_search(cancel).await.is_ok();
            info!(
                event = "quick_launch.file_dialog_search.cancel_dispatched",
                correlation_id = %cancel.request_id,
                generation = cancel.generation,
                reason,
                queued,
                "file-dialog indexed search cancellation dispatch ended"
            );
        })
        .detach();
    }

    pub(super) fn handle_file_dialog_search_batch(
        &mut self,
        batch: &IndexSearchBatch,
        cx: &mut Context<Self>,
    ) -> bool {
        if batch.generation & FILE_DIALOG_SEARCH_GENERATION_MASK == 0 {
            return false;
        }
        if batch.complete
            && self
                .file_dialog_search
                .pending_index_search
                .is_some_and(|pending| pending.generation == batch.generation)
        {
            self.file_dialog_search.pending_index_search = None;
        }
        if !self.file_dialog_search.index.apply(batch.clone()) {
            return true;
        }
        if batch.complete
            || (!batch.items.is_empty() && self.file_dialog_search.index.deadline_elapsed)
        {
            self.refresh_file_dialog_search_results(cx);
        }
        true
    }

    pub(super) fn handle_file_dialog_search_failure(
        &mut self,
        generation: u64,
        cx: &mut Context<Self>,
    ) -> bool {
        if generation & FILE_DIALOG_SEARCH_GENERATION_MASK == 0 {
            return false;
        }
        if self
            .file_dialog_search
            .pending_index_search
            .is_some_and(|pending| pending.generation == generation)
        {
            self.file_dialog_search.pending_index_search = None;
        }
        if generation == self.file_dialog_search.index.generation {
            warn!(
                event = "quick_launch.file_dialog_search.provider_failed",
                generation, "file-dialog indexed search failed; local results remain"
            );
            self.file_dialog_search.index.items.clear();
            self.file_dialog_search.index.complete = true;
            self.refresh_file_dialog_search_results(cx);
        }
        true
    }

    pub(super) fn handle_file_dialog_search_disconnect(&mut self, cx: &mut Context<Self>) {
        self.file_dialog_search.pending_index_search = None;
        let query = self.file_dialog_search.index.query.clone();
        if query.trim().is_empty() {
            return;
        }
        // Re-run the visible query so local sources stay available without the indexer.
        self.begin_file_dialog_search(query, cx);
    }

    pub(super) fn retry_file_dialog_search_after_ready(&mut self, cx: &mut Context<Self>) {
        let query = self.file_dialog_search.index.query.clone();
        let visible = self
            .file_dialog_search
            .host
            .as_ref()
            .is_some_and(|host| host.visible);
        if !visible || query.trim().is_empty() {
            return;
        }
        info!(
            event = "quick_launch.file_dialog_search.retry_after_ready",
            query_length = query.chars().count(),
            "retrying visible file-dialog query after indexer became ready"
        );
        self.begin_file_dialog_search(query, cx);
    }

    /// Same publication rule as the launcher panel: one bounded first merge, then the terminal
    /// merge, with intermediate provider batches folded into those two publications.
    fn refresh_file_dialog_search_results(&mut self, cx: &mut Context<Self>) {
        let index = &self.file_dialog_search.index;
        if index.terminal_published
            || (index.first_published && !index.complete)
            || (!index.complete && !index.deadline_elapsed)
            || index.query.trim().is_empty()
        {
            return;
        }
        let complete = index.complete;
        let generation = index.generation;
        let query = index.query.clone();
        let results = self.file_dialog_combined_results();
        if results.is_empty() && !complete {
            return;
        }
        let indexed_visible = results
            .iter()
            .filter(|result| {
                matches!(
                    result.entry.source,
                    ApplicationSource::IndexedFile | ApplicationSource::IndexedDirectory
                )
            })
            .count();
        let result_count = results.len();
        if self.publish_file_dialog_search_results(query, results, cx) {
            self.file_dialog_search.index.first_published = true;
            self.file_dialog_search.index.terminal_published = complete;
            info!(
                event = "quick_launch.file_dialog_search.published",
                generation,
                complete,
                indexed_visible,
                result_count,
                "file-dialog merged results published"
            );
        }
    }

    fn file_dialog_combined_results(&self) -> Vec<SearchResult> {
        let index = &self.file_dialog_search.index;
        if index.query.trim().is_empty() {
            return Vec::new();
        }
        let indexed = indexed_search_entries(&index.items);
        let results = self.unified_catalog_results(&index.query, &indexed, !index.complete);
        prefer_dialog_directory(
            results,
            self.file_dialog_search
                .host
                .as_ref()
                .and_then(|host| host.current_directory.as_deref()),
        )
    }

    fn publish_file_dialog_search_results(
        &mut self,
        query: String,
        results: Vec<SearchResult>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(panel) = self
            .file_dialog_search
            .window
            .as_ref()
            .map(|window| window.panel.clone())
        else {
            return false;
        };
        let prefetch = results[..results.len().min(ICON_PREFETCH_LIMIT)].to_vec();
        let published = panel
            .update(cx, |panel, cx| panel.publish_results(&query, results, cx))
            .unwrap_or(false);
        if published {
            self.request_icons(&prefetch, cx);
        }
        published
    }

    fn set_file_dialog_search_pending(&mut self, cx: &mut Context<Self>) {
        if let Some(window) = self.file_dialog_search.window.as_ref() {
            let _ = window.panel.update(cx, |panel, cx| panel.set_pending(cx));
        }
    }

    pub(super) fn deliver_file_dialog_search_icons(
        &self,
        icons: Vec<(String, PathBuf)>,
        cx: &mut Context<Self>,
    ) {
        let Some(panel) = self
            .file_dialog_search
            .window
            .as_ref()
            .map(|window| window.panel.clone())
        else {
            return;
        };
        cx.defer(move |cx| {
            let _ = panel.update(cx, |panel, cx| panel.set_icons(icons, cx));
        });
    }

    pub(super) fn activate_file_dialog_search_result(
        &mut self,
        result: SearchResult,
        query: String,
        trigger: QuickLaunchActionTrigger,
        cx: &mut Context<Self>,
    ) {
        let Some(host_window_handle) = self
            .file_dialog_search
            .host
            .as_ref()
            .map(|host| host.window_handle)
        else {
            return;
        };
        // Files and folders stay inside the dialog; they never open a second Explorer.
        let command = match result.entry.source {
            ApplicationSource::IndexedDirectory => Some(PlatformCommand::NavigateDirectory(
                NavigateDirectoryCommand {
                    host: DirectoryNavigationHost::FileDialog,
                    window_handle: host_window_handle,
                    target_directory: result.entry.target.clone(),
                    host_context_id: None,
                },
            )),
            ApplicationSource::IndexedFile if Path::new(&result.entry.target).is_absolute() => {
                Some(PlatformCommand::OpenFileInDialog(OpenFileInDialogCommand {
                    window_handle: host_window_handle,
                    file_path: result.entry.target.clone(),
                }))
            }
            ApplicationSource::IndexedFile => return,
            _ => None,
        };
        if let Some(command) = command {
            if let Err(error) = self.platform.send_command(command) {
                warn!(%error, "could not activate file-dialog search result");
            }
            return;
        }
        // Applications, commands, calculator, environment variables and extension rows use
        // the launcher's own action pipeline.
        let action = quick_launch_primary_action(&result, false, false);
        self.execute(result, action, query, trigger, cx);
    }

    fn activate_file_dialog_default_source(
        &mut self,
        target_directory: String,
        _cx: &mut Context<Self>,
    ) {
        let Some(host) = self.file_dialog_search.host.as_ref() else {
            return;
        };
        if !is_navigable_directory_target(&target_directory) {
            return;
        }
        if let Err(error) = self
            .platform
            .send_command(PlatformCommand::NavigateDirectory(
                NavigateDirectoryCommand {
                    host: DirectoryNavigationHost::FileDialog,
                    window_handle: host.window_handle,
                    target_directory,
                    host_context_id: None,
                },
            ))
        {
            warn!(%error, "could not activate file-dialog default directory");
        }
    }
}

#[cfg(test)]
pub(super) fn file_dialog_test_result(target: &str, source: ApplicationSource) -> SearchResult {
    let name = Path::new(target)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(target)
        .to_owned();
    SearchResult {
        entry: nxtools_finder::ApplicationEntry {
            id: format!("test:{}", target.to_lowercase()),
            name,
            target: target.into(),
            source,
        },
        action: nxtools_finder::SearchResultAction::OpenTarget,
        subtitle: None,
        score: 0,
        activation_count: 0,
        last_activated_at_unix_ms: None,
        query_activation_count: 0,
        last_query_activation_at_unix_ms: None,
    }
}

#[cfg(test)]
pub(super) fn polish_keyboard_fixture(
    window: &mut Window,
    cx: &mut Context<FileDialogSearchPanel>,
) -> FileDialogSearchPanel {
    let host = QuickLaunchFileDialogSearchHost {
        window_handle: 77,
        current_directory: Some(r"C:\synthetic".into()),
        left: 100,
        top: 200,
        width: 900,
        height: 640,
        monitor_dpi: 96,
        visible: true,
        changed_at_unix_ms: 1,
        quick_switch_source: None,
    };
    let mut panel = FileDialogSearchPanel::new(&host, WeakEntity::new_invalid(), window, cx);
    panel.expanded = true;
    panel.results = ["first", "second"]
        .into_iter()
        .map(|name| {
            file_dialog_test_result(
                &format!(r"C:\synthetic\{name}.txt"),
                ApplicationSource::IndexedFile,
            )
        })
        .collect::<Vec<_>>()
        .into();
    panel
}

#[cfg(test)]
impl FileDialogSearchPanel {
    pub(super) fn polish_focus_search(&self, window: &mut Window, cx: &mut App) {
        self.search_input.focus_handle(cx).focus(window, cx);
    }
    pub(super) fn polish_selected(&self) -> usize {
        self.selected
    }
    pub(super) fn polish_search_input(&self) -> Entity<InputState> {
        self.search_input.clone()
    }
    pub(super) fn polish_search_state(&mut self, expanded: bool, pending: bool) {
        self.expanded = expanded;
        self.search_pending = pending;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::typography::{LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL};

    fn host(dpi: u32) -> QuickLaunchFileDialogSearchHost {
        QuickLaunchFileDialogSearchHost {
            window_handle: 77,
            current_directory: Some(r"C:\Work".into()),
            left: 100,
            top: 200,
            width: 900,
            height: 640,
            monitor_dpi: dpi,
            visible: true,
            changed_at_unix_ms: 1,
            quick_switch_source: None,
        }
    }

    fn scaled_host(dpi: u32) -> QuickLaunchFileDialogSearchHost {
        let scale = dpi as f32 / 96.0;
        QuickLaunchFileDialogSearchHost {
            left: (100.0 * scale).round() as i32,
            top: (200.0 * scale).round() as i32,
            width: (900.0 * scale).round() as i32,
            height: (640.0 * scale).round() as i32,
            ..host(dpi)
        }
    }

    #[gpui_kit::test]
    fn w4_file_dialog_recent_prefix_keeps_ordinary_routing_and_finishes_pending(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (_temporary, application, _endpoint) =
            super::super::tests::quick_launch_application_fixture(cx);
        let test_host = host(96);
        let mut index = IndexSearchSession::default();
        let request = file_dialog_search_request(&mut index, &test_host, "recent:", false);
        let generation = request.generation;
        assert_ne!(generation & FILE_DIALOG_SEARCH_GENERATION_MASK, 0);
        assert!(!request.context.recent_modified);
        assert!(!request.context.allow_homophone_fallback);
        assert_eq!(request.query, "recent:");
        let retained = Rc::new(RefCell::new(None));
        let slot = retained.clone();
        let owner = application.downgrade();
        let (_root, window) = cx.add_window_view(move |window, cx| {
            let panel = cx.new(|cx| FileDialogSearchPanel::new(&test_host, owner, window, cx));
            panel.update(cx, |panel, cx| {
                panel
                    .search_input
                    .update(cx, |state, cx| state.set_value("recent:", window, cx));
                panel.search_pending = true;
            });
            *slot.borrow_mut() = Some(panel.clone());
            Root::new(panel, window, cx)
        });
        let panel = retained.borrow().clone().expect("panel");
        window.update(|window, cx| {
            application.update(cx, |app, _| {
                app.file_dialog_search.index.generation = generation;
                app.file_dialog_search.index.query = "recent:".into();
                app.file_dialog_search.window = Some(FileDialogSearchWindow {
                    window: window.window_handle(),
                    panel: panel.downgrade(),
                    native_window_handle: 1,
                });
            })
        });
        for batch_generation in [generation - 1, generation] {
            window.update(|_, cx| {
                application.update(cx, |app, cx| {
                    app.handle_indexer_notification(
                        crate::runtime::IndexerNotification::Event(
                            nxtools_protocol::IndexerEvent::SearchBatch(IndexSearchBatch {
                                generation: batch_generation,
                                items: Vec::new(),
                                complete: true,
                                elapsed_us: 1,
                            }),
                        ),
                        cx,
                    );
                })
            });
            window.read(|cx| {
                assert_eq!(
                    panel.read(cx).search_pending,
                    batch_generation != generation
                )
            });
        }
    }

    #[test]
    fn collapsed_search_bar_stays_centered_below_the_dialog() {
        assert_eq!(
            file_dialog_search_bounds(&host(96), false),
            FileDialogSearchNativeBounds {
                x: 300,
                y: 840,
                width: 500,
                height: 38,
            }
        );
    }

    #[test]
    fn expanded_results_open_above_the_bottom_anchored_search_bar() {
        assert_eq!(
            file_dialog_search_bounds(&host(96), true),
            FileDialogSearchNativeBounds {
                x: 300,
                y: 576,
                width: 500,
                height: 302,
            }
        );
    }

    #[test]
    fn search_bar_keeps_the_same_logical_geometry_at_125_and_150_percent_dpi() {
        assert_eq!(
            file_dialog_search_bounds(&scaled_host(120), false),
            FileDialogSearchNativeBounds {
                x: 375,
                y: 1_050,
                width: 625,
                height: 48,
            }
        );
        assert_eq!(
            file_dialog_search_bounds(&scaled_host(144), false),
            FileDialogSearchNativeBounds {
                x: 450,
                y: 1_260,
                width: 750,
                height: 57,
            }
        );
    }

    #[test]
    fn result_overlap_and_visible_width_stay_inside_the_host_at_release_dpis() {
        for dpi in [96, 120, 144] {
            let host = scaled_host(dpi);
            let bounds = file_dialog_search_bounds(&host, true);
            let scale = dpi as f32 / 96.0;
            let result_overlap = (SEARCH_RESULTS_HEIGHT * scale).round() as i32;

            assert!(bounds.x >= host.left);
            assert!(bounds.x + bounds.width <= host.left + host.width);
            assert!(bounds.y >= host.top);
            assert_eq!(bounds.y + result_overlap, host.top + host.height);
        }
    }

    #[test]
    fn short_hosts_clamp_the_result_overlap_at_every_release_dpi() {
        for dpi in [96, 120, 144] {
            let scale = dpi as f32 / 96.0;
            let mut host = scaled_host(dpi);
            host.height = (160.0 * scale).round() as i32;
            let bounds = file_dialog_search_bounds(&host, true);
            let result_overlap = (file_dialog_search_result_height(&host) * scale).round() as i32;

            assert_eq!(bounds.y, host.top);
            assert_eq!(result_overlap, host.height);
            assert_eq!(bounds.y + result_overlap, host.top + host.height);
        }
    }

    #[gpui_kit::test]
    fn kit_silent_query_clear_retires_file_dialog_search_generation(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (panel, window) = cx.add_window_view(|window, cx| {
            FileDialogSearchPanel::new(&host(96), WeakEntity::new_invalid(), window, cx)
        });
        window.update(|window, cx| {
            panel.update(cx, |panel, cx| {
                panel
                    .search_input
                    .update(cx, |state, cx| state.set_value("synthetic", window, cx));
                panel.expanded = true;
                panel.search_pending = true;
                let generation = panel.dispatch_generation;
                panel.escape(window, cx);
                assert!(panel.search_input.read(cx).value().is_empty());
                assert!(panel.dispatch_generation > generation);
                assert!(!panel.expanded && !panel.search_pending);
                assert!(panel.results.is_empty());
            })
        });
    }

    #[gpui_kit::test]
    fn production_file_dialog_search_uses_shared_input_and_result_rails(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let test_host = host(96);
        let (_root, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(
                px(SEARCH_WIDTH),
                px(SEARCH_RESULTS_HEIGHT + SEARCH_BAR_HEIGHT + SEARCH_FRAME_HEIGHT),
            ));
            let panel = cx.new(|cx| {
                let mut panel = FileDialogSearchPanel::new(
                    &test_host,
                    WeakEntity::<QuickLaunchApplication>::new_invalid(),
                    window,
                    cx,
                );
                panel.expanded = true;
                panel.results = vec![file_dialog_test_result(
                    r"C:\Work\report.txt",
                    ApplicationSource::IndexedFile,
                )]
                .into();
                panel
            });
            Root::new(panel, window, cx)
        });

        window
            .refresh()
            .expect("render production file-dialog search");
        let search = window
            .debug_bounds("file-dialog-search-header")
            .expect("shared search header");
        let snapshots = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
        let search_icon = snapshots
            .iter()
            .find(|snapshot| {
                snapshot.path().last()
                    == Some(&gpui_kit::ElementId::from("file-dialog-search-icon"))
            })
            .expect("official search addon")
            .bounds();
        let group = snapshots
            .iter()
            .find(|snapshot| {
                snapshot.path().last()
                    == Some(&gpui_kit::ElementId::from("file-dialog-search-group"))
            })
            .expect("real Kit search frame")
            .bounds();
        let input = window
            .debug_bounds("file-dialog-search-input")
            .expect("search input rail");
        let row = window
            .debug_bounds("file-dialog-search-result-row-0")
            .expect("shared result row");
        let icon = window
            .debug_bounds("file-dialog-search-result-icon-0")
            .expect("result icon rail");
        let title = window
            .debug_bounds("file-dialog-search-result-title-0")
            .expect("result title");
        let path = window
            .debug_bounds("file-dialog-search-result-path-0")
            .expect("result path");

        assert_eq!(search.size.height, px(SEARCH_BAR_HEIGHT));
        assert!(search_icon.left() >= group.left() && search_icon.right() <= group.right());
        assert_eq!(group.size.height, px(32.0));
        assert_eq!(input.size.height, group.size.height);
        assert!(group.top() >= search.top() && group.bottom() <= search.bottom());
        assert_eq!(row.size.height, px(COMPACT_RESULT_ROW_HEIGHT));
        assert_eq!(icon.size, size(px(26.0), px(26.0)));
        assert_eq!(title.size.height, px(LINE_HEIGHT_BODY));
        assert_eq!(path.size.height, px(LINE_HEIGHT_LABEL));
        assert_eq!(title.left(), path.left());
    }

    #[gpui_kit::test]
    fn production_file_dialog_default_source_uses_the_same_compact_rails(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let test_host = QuickLaunchFileDialogSearchHost {
            quick_switch_source: Some(nxtools_protocol::QuickLaunchDirectorySource {
                context: nxtools_protocol::DirectoryNavigationContext {
                    host: DirectoryNavigationHost::ThirdPartyFileManager,
                    window_handle: 88,
                    current_directory: Some(r"D:\Workspace".into()),
                    host_context_id: Some("dopus:lister-88".into()),
                },
                display_name: "Workspace (Directory Opus)".into(),
            }),
            ..host(96)
        };
        let (_root, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(
                px(288.0),
                px(SEARCH_BAR_HEIGHT + DEFAULT_SOURCE_HEIGHT + SEARCH_FRAME_HEIGHT),
            ));
            let panel = cx.new(|cx| {
                FileDialogSearchPanel::new(
                    &test_host,
                    WeakEntity::<QuickLaunchApplication>::new_invalid(),
                    window,
                    cx,
                )
            });
            Root::new(panel, window, cx)
        });
        window.simulate_resize(size(
            px(288.0),
            px(SEARCH_BAR_HEIGHT + DEFAULT_SOURCE_HEIGHT + SEARCH_FRAME_HEIGHT),
        ));

        window
            .refresh()
            .expect("render default Quick Switch source");
        let row = window
            .debug_bounds("file-dialog-default-source")
            .expect("default source row");
        let icon = window
            .debug_bounds("file-dialog-default-source-icon")
            .expect("default source icon rail");
        let title = window
            .debug_bounds("file-dialog-default-source-title")
            .expect("default source title");
        let path = window
            .debug_bounds("file-dialog-default-source-path")
            .expect("default source path");
        let shortcut = window
            .debug_bounds("file-dialog-default-source-shortcut")
            .expect("default source shortcut rail");

        let root = window
            .debug_bounds("file-dialog-search-window")
            .expect("actual root");
        assert!(
            row.left() >= root.left() + px(1.0) && row.right() <= root.right() - px(1.0),
            "whole source row width fits root: row={row:?} root={root:?}"
        );
        assert!(row.top() >= root.top() + px(1.0));
        assert!(
            row.bottom() <= root.bottom() - px(1.0),
            "whole source row must fit inside border: row={row:?} root={root:?}"
        );
        assert_eq!(row.size.height, px(DEFAULT_SOURCE_HEIGHT));
        assert_eq!(icon.size, size(px(26.0), px(26.0)));
        assert_eq!(title.size.height, px(LINE_HEIGHT_BODY));
        assert!(title.size.width > px(0.0));
        assert_eq!(path.size.height, px(LINE_HEIGHT_LABEL));
        assert!(path.size.width > px(0.0));
        assert!(title.right() <= path.left());
        assert_eq!(shortcut.size.width, px(52.0));
        let key = window
            .debug_bounds("kbd:ctrl-g")
            .expect("official Ctrl+G Kbd");
        assert!(
            key.left() >= shortcut.left() && key.right() <= shortcut.right(),
            "Kbd fits adapted52px rail: {key:?} rail={shortcut:?}"
        );
        assert!(
            key.top() >= row.top() && key.bottom() <= row.bottom(),
            "official key height fits original compact row"
        );
        assert!(path.right() <= shortcut.left());
        assert!(shortcut.right() <= row.right());
    }

    #[test]
    fn narrow_dialog_clamps_search_width_without_losing_symmetric_insets() {
        let mut narrow = host(96);
        narrow.width = 320;
        assert_eq!(
            file_dialog_search_bounds(&narrow, false),
            FileDialogSearchNativeBounds {
                x: 116,
                y: 840,
                width: 288,
                height: 38,
            }
        );
    }

    #[test]
    fn fixed_dialog_search_is_global_and_keeps_the_latest_directory_source_visible() {
        let host = QuickLaunchFileDialogSearchHost {
            quick_switch_source: Some(nxtools_protocol::QuickLaunchDirectorySource {
                context: nxtools_protocol::DirectoryNavigationContext {
                    host: DirectoryNavigationHost::ThirdPartyFileManager,
                    window_handle: 88,
                    current_directory: Some(r"D:\Workspace".into()),
                    host_context_id: Some("dopus:lister-88".into()),
                },
                display_name: "Workspace (Directory Opus)".into(),
            }),
            ..host(96)
        };

        let mut index = IndexSearchSession::default();
        let request = file_dialog_search_request(&mut index, &host, "gram", false);
        assert_eq!(request.context.directory, None);
        assert_eq!(
            request.context.preferred_directory.as_deref(),
            Some(r"C:\Work")
        );
        assert_eq!(
            file_dialog_default_source(&host),
            Some(FileDialogDefaultSource {
                name: "Workspace (Directory Opus)".into(),
                path: r"D:\Workspace".into(),
            })
        );
        assert_eq!(
            file_dialog_search_bounds(&host, false),
            FileDialogSearchNativeBounds {
                x: 300,
                y: 840,
                width: 500,
                height: 74,
            }
        );
    }

    #[test]
    fn dialog_request_matches_the_launcher_index_request() {
        let mut launcher = IndexSearchSession::default();
        let launcher_request = launcher.begin("report", INDEX_CANDIDATE_LIMIT);
        let mut dialog = IndexSearchSession::default();
        let first = file_dialog_search_request(&mut dialog, &host(96), "report", true);
        assert_eq!(first.limit, launcher_request.limit);
        assert_eq!(
            first.context.retain_kinds_apart,
            launcher_request.context.retain_kinds_apart
        );
        assert!(first.context.allow_homophone_fallback);
        let next = file_dialog_search_request(&mut dialog, &host(96), "report", false);
        assert_ne!(next.generation & FILE_DIALOG_SEARCH_GENERATION_MASK, 0);
        assert!(next.generation > first.generation);
        assert!(!next.context.allow_homophone_fallback);
    }

    #[test]
    fn dialog_folder_ranks_indexed_rows_without_moving_launcher_rows() {
        let ranked = vec![
            file_dialog_test_result(r"D:\Other\report.txt", ApplicationSource::IndexedFile),
            file_dialog_test_result(r"C:\Apps\Report.lnk", ApplicationSource::StartMenu),
            file_dialog_test_result(r"C:\Work\report.txt", ApplicationSource::IndexedFile),
            file_dialog_test_result(r"C:\Work\reports", ApplicationSource::IndexedDirectory),
        ];
        fn targets(rows: &[SearchResult]) -> Vec<String> {
            rows.iter().map(|row| row.entry.target.clone()).collect()
        }
        assert_eq!(
            targets(&prefer_dialog_directory(ranked.clone(), Some(r"C:\Work\"))),
            vec![
                r"C:\Work\report.txt",
                r"C:\Apps\Report.lnk",
                r"C:\Work\reports",
                r"D:\Other\report.txt",
            ]
        );
        assert_eq!(
            targets(&prefer_dialog_directory(ranked.clone(), None)),
            targets(&ranked)
        );
    }

    #[test]
    fn fixed_dialog_search_header_does_not_advertise_unimplemented_actions() {
        let source = include_str!("file_dialog_search.rs");
        for icon in ["Star", "History", "ManageSearch"] {
            let usage = format!("MaterialIcon::{icon}");
            assert!(!source.contains(&usage));
        }
    }

    #[test]
    fn initial_hidden_gpui_window_defers_native_placement_and_visibility() {
        let source = include_str!("file_dialog_search.rs");
        let handler = source
            .split("pub(super) fn handle_file_dialog_search_host")
            .nth(1)
            .and_then(|source| source.split("fn open_file_dialog_search_window").next())
            .expect("file-dialog host handler source");
        let create_branch = handler
            .split("if self.file_dialog_search.window.is_none()")
            .nth(1)
            .and_then(|source| source.split("} else {").next())
            .expect("initial file-dialog search window branch");

        assert!(
            create_branch.contains("schedule_application_update"),
            "a GPUI window created with show:false must finish its first app turn before native placement and visibility are applied"
        );
        assert!(
            create_branch.contains("return;"),
            "the initial host update must not fall through to immediate native placement"
        );
    }

    #[test]
    fn csharp_host_visibility_is_the_only_fixed_bar_dismissal_authority() {
        let event_router = include_str!("../quick_launch.rs");
        assert!(
            !event_router.contains("self.handle_file_dialog_search_foreground(&foreground, cx);"),
            "the generic foreground event must not race the C# file-dialog host visibility event"
        );
    }
}
