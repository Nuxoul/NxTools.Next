use std::cell::RefCell;
use std::ops::Range;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyWindowHandle, App, Bounds, Context, Entity, EntityInputHandler,
    Focusable, IntoElement, Render, ScrollStrategy, Subscription, UniformListScrollHandle,
    WeakEntity, Window, WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind,
    WindowOptions, div, ease_out_quint, point, prelude::*, px, size, uniform_list,
};
use gpui_component::Root;
use gpui_component::input::{Input, InputEvent, InputState};
use nxtools_protocol::{
    ActivateWindowCommand, DirectoryNavigationHost, IndexSearchBatch, IndexSearchContext,
    IndexSearchItem, IndexSearchRequest, IndexerCommand, NavigateDirectoryCommand,
    OpenFileInDialogCommand, PlaceFileDialogSearchWindowCommand, PlatformCommand,
    QuickLaunchFileDialogSearchHost, SetWindowOwnerCommand, SetWindowTopmostCommand,
    SetWindowVisibilityCommand,
};
use tracing::{info, warn};

use super::search::is_navigable_directory_target;
use super::{ImeEnterGate, QuickLaunchApplication, schedule_application_update};
use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::components::state::{empty_state, progress_state};
use crate::ui::metrics::{
    COMPACT_RESULT_ICON_RAIL_WIDTH, COMPACT_RESULT_ROW_HEIGHT, COMPACT_RESULT_TRAILING_RAIL_WIDTH,
    ICON_GLYPH_SMALL, SPACING_8, SPACING_12,
};
use crate::ui::native_window_handle;
use crate::ui::typography::{
    LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL, UI_FONT_FAMILY,
};

const SEARCH_WIDTH: f32 = 500.0;
const SEARCH_BAR_HEIGHT: f32 = 36.0;
const DEFAULT_SOURCE_HEIGHT: f32 = 36.0;
const SEARCH_RESULTS_HEIGHT: f32 = 264.0;
const SEARCH_HORIZONTAL_MARGIN: f32 = 16.0;
const SEARCH_RESULT_LIMIT: usize = 120;
const SEARCH_DEBOUNCE: Duration = Duration::from_millis(65);
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
            .saturating_add(default_source_height),
    }
}

fn file_dialog_search_request(
    host: &QuickLaunchFileDialogSearchHost,
    generation: u64,
    query: String,
    limit: usize,
) -> Option<IndexSearchRequest> {
    if query.trim().is_empty() {
        return None;
    }
    Some(IndexSearchRequest {
        generation,
        query,
        limit,
        context: IndexSearchContext {
            directory: None,
            preferred_directory: host.current_directory.clone(),
            include_files: true,
            include_directories: true,
        },
    })
}

#[derive(Default)]
pub(super) struct FileDialogSearchSession {
    host: Option<QuickLaunchFileDialogSearchHost>,
    window: Option<FileDialogSearchWindow>,
    generation: u64,
    query: String,
    items: Vec<IndexSearchItem>,
    last_placement: Option<FileDialogSearchPlacementKey>,
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
    results: Vec<IndexSearchItem>,
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
                        this.activate_selected(cx);
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
        let key_listener = cx.listener(|this, event: &gpui::KeystrokeEvent, window, cx| {
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
            results: Vec::new(),
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

    fn set_results(
        &mut self,
        results: Vec<IndexSearchItem>,
        pending: bool,
        cx: &mut Context<Self>,
    ) {
        self.results = results;
        self.search_pending = pending;
        self.selected = self.selected.min(self.results.len().saturating_sub(1));
        self.ensure_selected_visible();
        cx.notify();
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

    fn activate_selected(&self, cx: &mut App) {
        let Some(result) = self.results.get(self.selected).cloned() else {
            self.activate_default_source(cx);
            return;
        };
        let application = self.application.clone();
        schedule_application_update(
            application,
            move |application, cx| {
                application.activate_file_dialog_search_result(result, cx);
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
    ) -> Option<gpui::AnyElement> {
        let source = self.default_source.clone()?;
        let theme = self.theme;
        let panel = cx.entity().downgrade();
        let escape_panel = panel.clone();
        let row = div()
            .id("file-dialog-default-source")
            .active(move |row| row.bg(theme.surface_pressed))
            .debug_selector(|| "file-dialog-default-source".into())
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
                        gpui_component::Icon::new(MaterialIcon::Folder)
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
                    .text_size(px(TEXT_LABEL))
                    .line_height(px(LINE_HEIGHT_LABEL))
                    .text_color(theme.text_muted)
                    .child(source.path),
            )
            .child(
                div()
                    .debug_selector(|| "file-dialog-default-source-shortcut".into())
                    .w(px(COMPACT_RESULT_TRAILING_RAIL_WIDTH))
                    .h(px(LINE_HEIGHT_LABEL))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_end()
                    .text_size(px(TEXT_LABEL))
                    .line_height(px(LINE_HEIGHT_LABEL))
                    .text_color(theme.text_muted)
                    .child("Ctrl+G"),
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
                    panel.search_input.focus_handle(cx).focus(window);
                    panel.escape(window, cx);
                });
                window.prevent_default();
                cx.stop_propagation();
            })
            .flex_none()
            .into_any_element(),
        )
    }

    fn row(&self, index: usize, window: &mut Window, cx: &mut Context<Self>) -> gpui::AnyElement {
        let item = self.results[index].clone();
        let selected = index == self.selected;
        let theme = self.theme;
        let panel = cx.entity().downgrade();
        let click_panel = panel.clone();
        let navigation_panel = panel.clone();
        let focus_id = gpui::SharedString::from(format!("file-dialog-result-focus-{}", item.path));
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
            .child(
                div()
                    .debug_selector(move || format!("file-dialog-search-result-icon-{index}"))
                    .size(px(COMPACT_RESULT_ICON_RAIL_WIDTH))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        gpui_component::Icon::new(if item.is_directory {
                            MaterialIcon::Folder
                        } else {
                            MaterialIcon::TextFields
                        })
                        .size(px(16.0))
                        .text_color(if selected {
                            theme.icon
                        } else {
                            theme.icon_muted
                        }),
                    ),
            )
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
                            .child(item.name),
                    )
                    .child(
                        div()
                            .debug_selector(move || {
                                format!("file-dialog-search-result-path-{index}")
                            })
                            .h(px(LINE_HEIGHT_LABEL))
                            .truncate()
                            .text_size(px(TEXT_LABEL))
                            .line_height(px(LINE_HEIGHT_LABEL))
                            .text_color(theme.text_muted)
                            .child(item.path),
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
            move |_event, _window, cx| {
                let _ = click_panel.update(cx, |panel, cx| {
                    panel.selected = index;
                    panel.activate_selected(cx);
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
                panel.search_input.focus_handle(cx).focus(window);
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
        .track_scroll(self.result_scroll.clone());
        let root = div()
            .id("file-dialog-search-window")
            .debug_selector(|| "file-dialog-search-window".into())
            .size_full()
            .overflow_hidden()
            .font_family(UI_FONT_FAMILY)
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
                                    "正在合并本地文件与文件夹结果。",
                                ))
                            } else {
                                surface.child(empty_state(
                                    theme,
                                    "file-dialog-search-empty",
                                    MaterialIcon::Search,
                                    super::QUICK_LAUNCH_EMPTY_RESULTS_LABEL,
                                    "尝试缩短关键词，或输入文件与文件夹名称。",
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
                    .gap(px(SPACING_8))
                    .child(
                        div()
                            .debug_selector(|| "file-dialog-search-icon".into())
                            .size(px(18.0))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                gpui_component::Icon::new(MaterialIcon::Search)
                                    .size(px(16.0))
                                    .text_color(theme.icon_muted),
                            ),
                    )
                    .child(
                        div()
                            .debug_selector(|| "file-dialog-search-input".into())
                            .h(px(LINE_HEIGHT_BODY))
                            .min_w_0()
                            .flex_1()
                            .child(
                                Input::new(&self.search_input)
                                    .appearance(false)
                                    .bordered(false)
                                    .focus_bordered(false)
                                    .text_size(px(TEXT_BODY))
                                    .line_height(px(LINE_HEIGHT_BODY)),
                            ),
                    )
                    .when(self.search_pending, |bar| {
                        bar.child(
                            gpui_component::Icon::new(MaterialIcon::ProgressActivity)
                                .size(px(ICON_GLYPH_SMALL))
                                .text_color(theme.icon_muted),
                        )
                    }),
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
        self.file_dialog_search.generation =
            self.file_dialog_search.generation.wrapping_add(1).max(1)
                | FILE_DIALOG_SEARCH_GENERATION_MASK;
        self.file_dialog_search.query.clear();
        self.file_dialog_search.items.clear();
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
    pub(super) fn begin_file_dialog_search(&mut self, query: String, cx: &mut Context<Self>) {
        let Some(host) = self.file_dialog_search.host.as_ref() else {
            return;
        };
        self.file_dialog_search.generation =
            self.file_dialog_search.generation.wrapping_add(1).max(1)
                | FILE_DIALOG_SEARCH_GENERATION_MASK;
        self.file_dialog_search.query = query.clone();
        self.file_dialog_search.items.clear();
        if query.trim().is_empty() {
            self.update_file_dialog_search_results(false, cx);
            self.apply_file_dialog_search_window_placement(cx);
            return;
        }
        let Some(request) = file_dialog_search_request(
            host,
            self.file_dialog_search.generation,
            query,
            SEARCH_RESULT_LIMIT,
        ) else {
            return;
        };
        if let Err(error) = self.indexer.send_command(IndexerCommand::Search(request)) {
            warn!(%error, "could not send file-dialog search request");
            self.update_file_dialog_search_results(false, cx);
        }
    }

    pub(super) fn handle_file_dialog_search_batch(
        &mut self,
        batch: &IndexSearchBatch,
        cx: &mut Context<Self>,
    ) -> bool {
        if batch.generation & FILE_DIALOG_SEARCH_GENERATION_MASK == 0 {
            return false;
        }
        if batch.generation != self.file_dialog_search.generation {
            return true;
        }
        self.file_dialog_search.items.extend(batch.items.clone());
        self.update_file_dialog_search_results(!batch.complete, cx);
        true
    }

    pub(super) fn handle_file_dialog_search_disconnect(&mut self, cx: &mut Context<Self>) {
        if self.file_dialog_search.query.trim().is_empty()
            && self.file_dialog_search.items.is_empty()
        {
            return;
        }
        self.file_dialog_search.generation =
            self.file_dialog_search.generation.wrapping_add(1).max(1)
                | FILE_DIALOG_SEARCH_GENERATION_MASK;
        self.file_dialog_search.items.clear();
        self.update_file_dialog_search_results(false, cx);
    }

    fn update_file_dialog_search_results(&mut self, pending: bool, cx: &mut Context<Self>) {
        let Some(search_window) = self.file_dialog_search.window.as_ref() else {
            return;
        };
        let results = self.file_dialog_search.items.clone();
        let _ = search_window
            .panel
            .update(cx, |panel, cx| panel.set_results(results, pending, cx));
    }

    pub(super) fn activate_file_dialog_search_result(
        &mut self,
        result: IndexSearchItem,
        _cx: &mut Context<Self>,
    ) {
        let Some(host) = self.file_dialog_search.host.as_ref() else {
            return;
        };
        let target_directory = if result.is_directory {
            result.path.clone()
        } else {
            String::new()
        };
        let command = if result.is_directory {
            PlatformCommand::NavigateDirectory(NavigateDirectoryCommand {
                host: DirectoryNavigationHost::FileDialog,
                window_handle: host.window_handle,
                target_directory,
                host_context_id: None,
            })
        } else if Path::new(&result.path).is_absolute() {
            PlatformCommand::OpenFileInDialog(OpenFileInDialogCommand {
                window_handle: host.window_handle,
                file_path: result.path,
            })
        } else {
            return;
        };
        if let Err(error) = self.platform.send_command(command) {
            warn!(%error, "could not activate file-dialog search result");
        }
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
        .map(|name| IndexSearchItem {
            path: format!(r"C:\synthetic\{name}.txt"),
            name: name.into(),
            is_directory: false,
            size_bytes: None,
            modified_at_unix_ms: None,
            score: 0,
        })
        .collect();
    panel
}

#[cfg(test)]
impl FileDialogSearchPanel {
    pub(super) fn polish_focus_search(&self, window: &mut Window, cx: &App) {
        self.search_input.focus_handle(cx).focus(window);
    }
    pub(super) fn polish_selected(&self) -> usize {
        self.selected
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

    #[test]
    fn collapsed_search_bar_stays_centered_below_the_dialog() {
        assert_eq!(
            file_dialog_search_bounds(&host(96), false),
            FileDialogSearchNativeBounds {
                x: 300,
                y: 840,
                width: 500,
                height: 36,
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
                height: 300,
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
                height: 45,
            }
        );
        assert_eq!(
            file_dialog_search_bounds(&scaled_host(144), false),
            FileDialogSearchNativeBounds {
                x: 450,
                y: 1_260,
                width: 750,
                height: 54,
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

    #[gpui::test]
    fn production_file_dialog_search_uses_shared_input_and_result_rails(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let test_host = host(96);
        let (_root, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(
                px(SEARCH_WIDTH),
                px(SEARCH_RESULTS_HEIGHT + SEARCH_BAR_HEIGHT),
            ));
            let panel = cx.new(|cx| {
                let mut panel = FileDialogSearchPanel::new(
                    &test_host,
                    WeakEntity::<QuickLaunchApplication>::new_invalid(),
                    window,
                    cx,
                );
                panel.expanded = true;
                panel.results = vec![IndexSearchItem {
                    path: r"C:\Work\report.txt".into(),
                    name: "report.txt".into(),
                    is_directory: false,
                    size_bytes: Some(42),
                    modified_at_unix_ms: None,
                    score: 1,
                }];
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
        let search_icon = window
            .debug_bounds("file-dialog-search-icon")
            .expect("search icon rail");
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
        assert_eq!(search_icon.size.width, px(18.0));
        assert_eq!(input.size.height, px(LINE_HEIGHT_BODY));
        assert_eq!(row.size.height, px(COMPACT_RESULT_ROW_HEIGHT));
        assert_eq!(icon.size, size(px(26.0), px(26.0)));
        assert_eq!(title.size.height, px(LINE_HEIGHT_BODY));
        assert_eq!(path.size.height, px(LINE_HEIGHT_LABEL));
        assert_eq!(title.left(), path.left());
    }

    #[gpui::test]
    fn production_file_dialog_default_source_uses_the_same_compact_rails(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_component::init);
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
                px(SEARCH_BAR_HEIGHT + DEFAULT_SOURCE_HEIGHT),
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

        assert_eq!(row.size.height, px(DEFAULT_SOURCE_HEIGHT));
        assert_eq!(icon.size, size(px(26.0), px(26.0)));
        assert_eq!(title.size.height, px(LINE_HEIGHT_BODY));
        assert!(title.size.width > px(0.0));
        assert_eq!(path.size.height, px(LINE_HEIGHT_LABEL));
        assert!(path.size.width > px(0.0));
        assert!(title.right() <= path.left());
        assert_eq!(shortcut.size.width, px(48.0));
        assert_eq!(shortcut.size.height, px(LINE_HEIGHT_LABEL));
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
                height: 36,
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

        let request = file_dialog_search_request(&host, 9, "gram".into(), 120)
            .expect("global dialog search request");
        assert_eq!(request.context.directory, None);
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
                height: 72,
            }
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
