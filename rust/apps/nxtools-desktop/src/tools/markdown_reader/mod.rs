//! The independent Markdown reading window and its document lifetimes.
#[cfg(all(test, windows))]
mod color_tests;
mod document;
mod edit;
mod export;
#[cfg(test)]
mod export_tests;
mod foreground;
#[cfg(test)]
mod header_tests;
#[cfg(test)]
mod heading_flash_motion_tests;
mod history;
#[cfg(test)]
mod history_tests;
mod material;
#[cfg(test)]
mod material_tests;
mod outline;
#[cfg(test)]
mod outline_pointer_tests;
mod presentation;
mod settings;
mod split;
#[cfg(test)]
mod split_tests;
pub(crate) use settings::{SettingsView, enabled, install_settings};
#[cfg(test)]
mod tests;
mod viewport;

use std::{
    collections::VecDeque,
    ops::Range,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui_kit::component::{
    Disableable as _, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputGroup, InputState},
    tab::{Tab, TabBar},
    text::{RangeHighlight, RenderedText, TextViewState},
    tree::{TreeItem, TreeState},
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyWindowHandle, App, AppContext as _, Context, Entity, ExternalPaths, FocusHandle,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Task, Window, WindowBounds, div,
    px, size,
};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use nxtools_protocol::{PickMarkdownDocumentCommand, PickMarkdownDocumentResult, PlatformCommand};
use uuid::Uuid;

use crate::{theme::Theme, ui::components::titlebar::caption_window_options};
use document::{Document, Heading};

const MAX_TABS: usize = PickMarkdownDocumentResult::MAX_PATHS;

#[derive(Default)]
pub(crate) struct ReaderWindow(Option<AnyWindowHandle>);
impl gpui_kit::Global for ReaderWindow {}

fn show(platform: Option<Rc<PlatformBridge>>, cx: &mut App) -> Option<AnyWindowHandle> {
    let operation = Uuid::new_v4();
    let handle = show_window(platform, operation, cx)?;
    cx.defer(move |cx| material::present(handle, operation, false, cx));
    Some(handle)
}

fn show_window(
    platform: Option<Rc<PlatformBridge>>,
    correlation: Uuid,
    cx: &mut App,
) -> Option<AnyWindowHandle> {
    if !enabled(cx) {
        return None;
    }
    let current = cx.try_global::<ReaderWindow>().and_then(|state| state.0);
    if let Some(current) = current
        && current.update(cx, |_, _, _| ()).is_ok()
    {
        return Some(current);
    }
    let started = Instant::now();
    let bounds = WindowBounds::centered(size(px(1020.), px(720.)), cx);
    tracing::info!(event = "markdown.reader.started", correlation_id = %correlation);
    let material_platform = platform.clone();
    let material_pending = material_platform.is_some();
    let result = cx.open_window(
        gpui_kit::WindowOptions {
            show: !material_pending,
            focus: !material_pending,
            window_bounds: Some(bounds),
            window_min_size: Some(size(px(680.), px(460.))),
            ..caption_window_options()
        },
        move |window, cx| {
            window.set_window_title("Markdown 阅读器 — NxTools");
            let view = cx.new(|cx| {
                let mut view = Reader::new(platform, window, cx);
                view.material_pending = material_pending;
                view
            });
            let owner = view.downgrade();
            window.on_window_should_close(cx, move |window, cx| {
                let owner = owner.clone();
                window.defer(cx, move |window, cx| {
                    let _ = owner.update(cx, |this, cx| this.request_close(None, window, cx));
                });
                false
            });
            cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
        },
    );
    let handle = result.ok().map(AnyWindowHandle::from);
    if let Some(handle) = handle {
        cx.set_global(ReaderWindow(Some(handle)));
        if let Some(platform) = material_platform {
            material::prepare(handle, platform, cx);
        }
    }
    tracing::info!(event = "markdown.reader.finished", correlation_id = %correlation, success = handle.is_some(), elapsed_ms = started.elapsed().as_millis());
    handle
}

#[cfg(debug_assertions)]
pub(crate) fn open_visual_fixture(path: PathBuf, split: bool, cx: &mut App) -> anyhow::Result<()> {
    let document = document::read(path)
        .map_err(|error| anyhow::anyhow!("Markdown fixture: {}", error.code()))?;
    let handle = show_window(None, Uuid::new_v4(), cx)
        .ok_or_else(|| anyhow::anyhow!("Markdown reader window unavailable"))?;
    handle.update(cx, |root, window, cx| {
        let root = root
            .downcast::<gpui_kit::base::Root>()
            .map_err(|_| anyhow::anyhow!("Markdown fixture root"))?;
        let reader = root
            .read(cx)
            .view()
            .clone()
            .downcast::<Reader>()
            .map_err(|_| anyhow::anyhow!("Markdown fixture view"))?;
        reader.update(cx, |reader, cx| {
            reader.add_document(document, cx);
            if split {
                reader.set_mode(edit::Mode::Split, window, cx);
            }
        });
        window.zoom_window();
        Ok::<_, anyhow::Error>(())
    })?
}

pub(crate) fn open_external(
    path: PathBuf,
    operation: Uuid,
    platform: Option<Rc<PlatformBridge>>,
    cx: &mut App,
) -> bool {
    if !path.is_absolute() || !document::supported(&path) {
        return false;
    }
    let Some(handle) = show_window(platform.clone(), operation, cx) else {
        return false;
    };
    handle
        .update(cx, |root, window, cx| {
            let Ok(root) = root.downcast::<gpui_kit::base::Root>() else {
                return false;
            };
            let Ok(reader) = root.read(cx).view().clone().downcast::<Reader>() else {
                return false;
            };
            reader.update(cx, |reader, cx| {
                if reader.platform.is_none() {
                    reader.platform = platform;
                }
                if reader.load.is_some() {
                    if reader.pending_opens.len() == MAX_TABS {
                        return false;
                    }
                    reader.pending_opens.push_back((path, operation));
                } else {
                    reader.open_paths(vec![path], None, operation, window, cx);
                }
                cx.defer_in(window, move |reader, window, _| {
                    reader.present_when_ready(operation, true, window)
                });
                true
            })
        })
        .unwrap_or(false)
}

pub(crate) fn platform_notification(event: &PlatformNotification, cx: &mut App) {
    settings::platform_notification(event, cx);
    let Some(handle) = cx.try_global::<ReaderWindow>().and_then(|state| state.0) else {
        return;
    };
    let event = match event {
        PlatformNotification::CommandResult { .. } | PlatformNotification::Disconnected { .. } => {
            event.clone()
        }
        _ => return,
    };
    cx.defer(move |cx| {
        let _ = handle.update(cx, |root, window, cx| {
            let Ok(root) = root.downcast::<gpui_kit::base::Root>() else {
                return;
            };
            let Ok(reader) = root.read(cx).view().clone().downcast::<Reader>() else {
                return;
            };
            reader.update(cx, |reader, cx| reader.on_platform_event(event, window, cx));
        });
    });
}

struct TabDocument {
    id: usize,
    document: Document,
    watch_stamp: document::Stamp,
    view: Entity<TextViewState>,
    outline: Entity<TreeState>,
    draft: edit::Draft,
    navigation: viewport::Navigation,
    _content: Subscription,
    problem: Option<&'static str>,
}

struct Find {
    text: RenderedText,
    ranges: Vec<Range<usize>>,
    current: usize,
}

struct Reader {
    platform: Option<Rc<PlatformBridge>>,
    picker: Option<(Uuid, Instant)>,
    foreground: Option<(Uuid, Instant)>,
    material_pending: bool,
    material_presentation: Option<(Uuid, bool)>,
    tabs: Vec<TabDocument>,
    active: usize,
    next_tab: usize,
    focus: FocusHandle,
    find_input: Entity<InputState>,
    find_visible: bool,
    find: Option<Find>,
    outline_visible: bool,
    outline_overlay: outline::OutlineOverlay,
    split: split::SplitLayout,
    font: f32,
    measure: f32,
    saving: Option<Task<()>>,
    export_selection: Option<export::ExportSelection>,
    export_worker: Option<Task<()>>,
    pdf: Option<export::Print>,
    notice: Option<&'static str>,
    history: Option<history::HistoryView>,
    history_work: Option<Task<()>>,
    problem: Option<&'static str>,
    load: Option<Task<()>>,
    pending_opens: VecDeque<(PathBuf, Uuid)>,
    watch: Option<Task<()>>,
    _find_subscription: Subscription,
}

gpui_kit::actions!(
    markdown_reader,
    [
        OpenDocument,
        FindDocument,
        DismissFind,
        SaveDocument,
        JumpOutlineHeading
    ]
);

impl Reader {
    fn new(
        platform: Option<Rc<PlatformBridge>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.bind_keys([
            gpui_kit::KeyBinding::new("ctrl-o", OpenDocument, Some("MarkdownReader")),
            gpui_kit::KeyBinding::new("ctrl-s", SaveDocument, Some("MarkdownReader")),
            gpui_kit::KeyBinding::new("ctrl-f", FindDocument, Some("MarkdownReader")),
            gpui_kit::KeyBinding::new("escape", DismissFind, Some("MarkdownReader")),
            gpui_kit::KeyBinding::new("enter", JumpOutlineHeading, Some("MarkdownOutline")),
        ]);
        let focus = cx.focus_handle().tab_stop(true);
        focus.focus(window, cx);
        let find_input = cx.new(|cx| InputState::new(window, cx).placeholder("查找正文"));
        let subscription = cx.subscribe_in(&find_input, window, |_, _, event, window, cx| {
            let event = match event {
                InputEvent::Change => 0,
                InputEvent::PressEnter { shift: false, .. } => 1,
                InputEvent::PressEnter { shift: true, .. } => -1,
                _ => return,
            };
            cx.defer_in(window, move |this, _, cx| {
                if event == 0 {
                    this.update_find(true, cx);
                } else {
                    this.step_find(event, cx);
                }
            });
        });
        let defaults = settings::preferences(cx);
        Self {
            platform,
            picker: None,
            foreground: None,
            material_pending: false,
            material_presentation: None,
            tabs: Vec::new(),
            active: 0,
            next_tab: 0,
            focus,
            find_input,
            find_visible: false,
            find: None,
            outline_visible: defaults.show_outline,
            outline_overlay: outline::OutlineOverlay::default(),
            split: split::SplitLayout::default(),
            font: f32::from(defaults.font_size),
            measure: if defaults.wide_body {
                presentation::WIDE_MEASURE
            } else {
                presentation::STANDARD_MEASURE
            },
            problem: None,
            saving: None,
            export_selection: None,
            export_worker: None,
            pdf: None,
            notice: None,
            history: None,
            history_work: None,
            load: None,
            pending_opens: VecDeque::new(),
            watch: None,
            _find_subscription: subscription,
        }
    }

    fn pick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !enabled(cx)
            || self.picker.is_some()
            || self.load.is_some()
            || self.export_selection.is_some()
        {
            return;
        }
        let id = Uuid::new_v4();
        let started = Instant::now();
        let sent = self
            .platform
            .as_ref()
            .zip(crate::ui::native_window_handle(window))
            .is_some_and(|(platform, hwnd)| {
                platform
                    .send_command_with_id(
                        PlatformCommand::PickMarkdownDocument(PickMarkdownDocumentCommand {
                            source_window_handle: hwnd as u64,
                            source_process_id: std::process::id(),
                        }),
                        id,
                    )
                    .is_ok()
            });
        if sent {
            self.picker = Some((id, started));
            tracing::info!(event = "markdown.picker.started", correlation_id = %id);
        } else {
            self.problem = Some("文件选择器不可用。可以将 Markdown 文件拖到此窗口。");
            tracing::warn!(event = "markdown.picker.finished", correlation_id = %id, outcome = "not_submitted");
        }
        cx.notify();
    }

    fn on_platform_event(
        &mut self,
        event: PlatformNotification,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.export_receipt(&event, window, cx);
        self.foreground_receipt(&event);
        let Some((id, started)) = self.picker else {
            return;
        };
        let (picked, native_error) = match event {
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } if correlation_id == id => {
                let parsed = if result.success {
                    result
                        .data
                        .as_deref()
                        .and_then(|data| {
                            serde_json::from_str::<PickMarkdownDocumentResult>(data).ok()
                        })
                        .filter(PickMarkdownDocumentResult::selection_valid)
                        .map(|value| {
                            value
                                .paths
                                .into_iter()
                                .map(PathBuf::from)
                                .collect::<Vec<_>>()
                        })
                } else {
                    None
                };
                (parsed, result.native_error_code)
            }
            PlatformNotification::Disconnected {
                pending_command_ids,
                ..
            } if pending_command_ids.contains(&id) => (None, None),
            _ => return,
        };
        self.picker = None;
        let outcome = match &picked {
            Some(paths) if paths.is_empty() => "cancelled",
            Some(_) => "selected",
            None => "failed",
        };
        tracing::info!(event = "markdown.picker.finished", correlation_id = %id, outcome, ?native_error, elapsed_ms = started.elapsed().as_millis());
        match picked {
            Some(paths) if !paths.is_empty() => self.open_paths(paths, None, id, window, cx),
            Some(_) => {}
            None => self.problem = Some("未能打开文件选择器，请重试或拖入文件。"),
        }
        cx.notify();
    }

    fn open_paths(
        &mut self,
        paths: Vec<PathBuf>,
        target: Option<String>,
        correlation: Uuid,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.load.is_some() || paths.is_empty() {
            return;
        }
        if !enabled(cx) {
            self.problem = Some("Markdown 阅读器已停用，请在其他工具中启用。");
            tracing::info!(event = "markdown.read.finished", correlation_id = %correlation, outcome = "disabled");
            cx.notify();
            return;
        }
        if paths.len() > MAX_TABS {
            self.problem = Some("一次最多打开 32 个 Markdown 文件。");
            cx.notify();
            return;
        }
        self.problem = None;
        let started = Instant::now();
        tracing::info!(event = "markdown.read.started", correlation_id = %correlation, count = paths.len());
        let worker = cx.background_spawn(async move {
            paths
                .into_iter()
                .map(|path| {
                    document::read(path).map(|document| {
                        let recorded = history::observe(&document, "opened", correlation);
                        (document, recorded)
                    })
                })
                .collect::<Vec<_>>()
        });
        self.load = Some(cx.spawn_in(window, async move |owner, cx| {
            let results = worker.await;
            let _ = owner.update_in(cx, |this, window, cx| {
                this.load = None;
                let mut opened = 0;
                let mut failed = 0;
                for result in results {
                    match result {
                        Ok((document, recorded)) => {
                            if !recorded { this.problem = Some("文档已打开，但未能写入版本历史。"); }
                            if this.add_document(document, cx) { opened += 1; }
                            else { failed += 1; }
                        }
                        Err(error) => {
                            failed += 1;
                            tracing::warn!(event = "markdown.read.failed", correlation_id = %correlation, outcome = error.code(), native_error = ?error.native_code());
                            this.problem = Some(error.message());
                        }
                    }
                }
                if let Some(anchor) = target { this.reveal_heading(&anchor, cx); }
                this.update_find(false, cx);
                this.start_watch(window, cx);
                tracing::info!(event = "markdown.read.finished", correlation_id = %correlation, opened, failed, elapsed_ms = started.elapsed().as_millis());
                if enabled(cx) {
                    if let Some((path, operation)) = this.pending_opens.pop_front() {
                        this.open_paths(vec![path], None, operation, window, cx);
                    }
                } else {
                    for (_, operation) in this.pending_opens.drain(..) {
                        tracing::info!(event = "markdown.read.finished", correlation_id = %operation, outcome = "disabled");
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn add_document(&mut self, document: Document, cx: &mut Context<Self>) -> bool {
        if let Some(index) = self
            .tabs
            .iter()
            .position(|tab| tab.document.path == document.path)
        {
            self.activate(index, cx);
            return true;
        }
        if self.tabs.len() == MAX_TABS {
            self.problem = Some("已打开 32 个标签页，请先关闭不使用的文档。");
            return false;
        }
        let view = cx.new(|cx| TextViewState::markdown(&document.text, cx));
        let outline = cx.new(|cx| TreeState::new(cx).items(outline_items(&document.headings)));
        let id = self.next_tab;
        self.next_tab += 1;
        let subscription = cx.observe(&view, move |_, _, cx| {
            let owner = cx.weak_entity();
            cx.defer(move |cx| {
                let _ = owner.update(cx, |this, cx| {
                    this.project_navigation(id, cx);
                    if this.active_tab().is_some_and(|tab| tab.id == id) && this.find_visible {
                        this.update_find(false, cx);
                    }
                });
            });
        });
        self.tabs.push(TabDocument {
            id,
            watch_stamp: document.stamp,
            document,
            view,
            outline,
            problem: None,
            draft: edit::Draft::default(),
            navigation: viewport::Navigation::default(),
            _content: subscription,
        });
        self.project_navigation(id, cx);
        self.activate(self.tabs.len() - 1, cx);
        true
    }

    fn active_tab(&self) -> Option<&TabDocument> {
        self.tabs.get(self.active)
    }

    fn activate(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.tabs.len() {
            if self.active != index
                && self.find_visible
                && let Some(tab) = self.active_tab()
            {
                tab.view
                    .update(cx, |view, cx| view.clear_range_highlights(cx));
            }
            if self.active != index {
                self.history = None;
                self.dismiss_outline();
            }
            self.active = index;
            self.update_find(false, cx);
            cx.notify();
        }
    }

    fn close_tab(&mut self, id: usize, cx: &mut Context<Self>) {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        if index == self.active {
            self.history = None;
            self.dismiss_outline();
        }
        self.tabs.remove(index);
        if index < self.active {
            self.active -= 1;
        }
        self.active = self.active.min(self.tabs.len().saturating_sub(1));
        if self.tabs.is_empty() {
            self.watch = None;
        }
        self.update_find(false, cx);
        cx.notify();
    }

    fn start_watch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.watch.is_some() || self.tabs.is_empty() {
            return;
        }
        let background = cx.background_executor().clone();
        self.watch = Some(cx.spawn_in(window, async move |owner, cx| {
            loop {
                background.timer(Duration::from_secs(1)).await;
                let Ok(sources) = owner.update_in(cx, |this, _, _| this.tabs.iter().map(|tab| (tab.id, tab.document.path.clone(), tab.watch_stamp)).collect::<Vec<_>>()) else { break };
                if sources.is_empty() { break; }
                let results = background.spawn(async move {
                    sources.into_iter().filter_map(|(id, path, base)| {
                        if document::stamp(&path).ok() == Some(base) { return None; }
                        let correlation = Uuid::new_v4();
                        let result = document::read(path).map(|document| {
                            let recorded = history::observe(&document, "external", correlation);
                            (document, recorded)
                        });
                        Some((id, base, correlation, result))
                    }).collect::<Vec<_>>()
                }).await;
                if results.is_empty() { continue; }
                if owner.update_in(cx, |this, window, cx| {
                    for (id, before, correlation, result) in results {
                        let Some(tab) = this.tabs.iter_mut().find(|tab| tab.id == id && tab.watch_stamp == before) else { continue };
                        match result {
                            Ok((document, recorded)) => {
                                if this.saving.is_some() { continue; }
                                tab.watch_stamp = document.stamp;
                                if tab.draft.dirty() {
                                    let problem = "文件已被外部修改，草稿仍保留。";
                                    if this.saving.is_none() && tab.problem != Some(problem) {
                                        tab.problem = Some(problem);
                                        tracing::info!(event = "markdown.watch.changed", correlation_id = %correlation, outcome = "conflict");
                                        cx.notify();
                                    }
                                    continue;
                                }
                                if document.text != tab.document.text {
                                    tab.navigation.content_changed();
                                }
                                tab.draft.refresh_clean(&document.text, window, cx);
                                let position = tab.view.read(cx).list_state().logical_scroll_top();
                                tab.view.update(cx, |view, cx| {
                                    // The same viewport and selection owner survives a file refresh.
                                    view.set_text(&document.text, cx);
                                    view.list_state().scroll_to(position);
                                });
                                tab.outline.update(cx, |outline, cx| outline.set_items(outline_items(&document.headings), cx));
                                tab.document = document;
                                tab.problem = (!recorded).then_some("文档已刷新，但未能写入版本历史。");
                                tracing::info!(event = "markdown.watch.changed", correlation_id = %correlation, outcome = "reloaded");
                                cx.notify();
                            }
                            Err(error) if tab.problem != Some(error.message()) => {
                                tab.problem = Some(error.message());
                                tracing::warn!(event = "markdown.watch.changed", correlation_id = %correlation, outcome = error.code(), native_error = ?error.native_code());
                                cx.notify();
                            }
                            Err(_) => {},
                        }
                    }
                }).is_err() { break; }
            }
        }));
    }

    fn set_find_visible(&mut self, visible: bool, window: &mut Window, cx: &mut Context<Self>) {
        use gpui_kit::Focusable as _;
        self.find_visible = visible;
        if visible {
            self.find_input.read(cx).focus_handle(cx).focus(window, cx);
            self.update_find(false, cx);
        } else {
            self.find = None;
            if let Some(view) = self.active_tab().map(|tab| tab.view.clone()) {
                view.update(cx, |view, cx| view.clear_range_highlights(cx));
            }
            self.focus.focus(window, cx);
        }
        cx.notify();
    }

    fn update_find(&mut self, query_changed: bool, cx: &mut Context<Self>) {
        let Some(view) = self
            .active_tab()
            .map(|tab| tab.view.clone())
            .filter(|_| self.find_visible)
        else {
            self.find = None;
            return;
        };
        let text = view.read(cx).rendered_text();
        if !query_changed && self.find.as_ref().is_some_and(|find| find.text == text) {
            return;
        }
        let query = self.find_input.read(cx).value();
        let ranges = if query.is_empty() {
            Vec::new()
        } else {
            text.as_str()
                .match_indices(query.as_str())
                .map(|(start, value)| start..start + value.len())
                .collect::<Vec<_>>()
        };
        let current = if query_changed {
            0
        } else {
            self.find
                .as_ref()
                .map_or(0, |find| find.current.min(ranges.len().saturating_sub(1)))
        };
        self.find = Some(Find {
            text,
            ranges,
            current,
        });
        self.paint_find(query_changed, cx);
    }

    fn step_find(&mut self, direction: isize, cx: &mut Context<Self>) {
        if let Some(find) = &mut self.find
            && !find.ranges.is_empty()
        {
            find.current =
                (find.current as isize + direction).rem_euclid(find.ranges.len() as isize) as usize;
            self.paint_find(true, cx);
        }
    }

    fn paint_find(&self, reveal: bool, cx: &mut Context<Self>) {
        let (Some(tab), Some(find)) = (self.active_tab(), &self.find) else {
            return;
        };
        let color = Theme::active(cx).warning;
        tab.view.update(cx, |view, cx| {
            if view.rendered_text() != find.text {
                return;
            }
            let _ = view.set_range_highlights(
                find.ranges.iter().enumerate().map(|(index, range)| {
                    RangeHighlight::new(
                        range.clone(),
                        color.opacity(if index == find.current { 0.55 } else { 0.24 }),
                    )
                }),
                cx,
            );
            if reveal && let Some(range) = find.ranges.get(find.current) {
                let _ = view.reveal_range(range.clone(), cx);
            }
        });
        cx.notify();
    }

    fn reveal_heading(&self, anchor: &str, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else { return };
        let Some(heading) = tab
            .document
            .headings
            .iter()
            .find(|heading| heading.anchor == anchor)
        else {
            return;
        };
        tab.view.update(cx, |view, cx| {
            let rendered = view.rendered_text();
            if rendered.source() == tab.document.text
                && let Some(range) = rendered.range_for_source(heading.source.clone())
            {
                let _ = view.reveal_range(range, cx);
            }
        });
    }
}

fn outline_items(headings: &[Heading]) -> Vec<TreeItem> {
    fn branch(headings: &[Heading], index: &mut usize, ancestor: u8) -> Vec<TreeItem> {
        let mut items = Vec::new();
        while let Some(heading) = headings
            .get(*index)
            .filter(|heading| heading.level > ancestor)
        {
            *index += 1;
            items.push(
                TreeItem::new(heading.anchor.clone(), heading.title.clone())
                    .children(branch(headings, index, heading.level))
                    .expanded(true),
            );
        }
        items
    }
    branch(headings, &mut 0, 0)
}

impl Render for Reader {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        let close_owner = cx.weak_entity();
        let close = move |_: &gpui_kit::ClickEvent, window: &mut Window, cx: &mut App| {
            let owner = close_owner.clone();
            window.defer(cx, move |window, cx| {
                let _ = owner.update(cx, |this, cx| this.request_close(None, window, cx));
            });
        };
        let tabs = self
            .tabs
            .iter()
            .map(|tab| {
                let id = tab.id;
                let label = if tab.draft.dirty() {
                    format!("{} ●", file_name(&tab.document))
                } else {
                    file_name(&tab.document).to_string()
                };
                Tab::new().label(label).suffix(
                    Button::new(("markdown-close-tab", id))
                        .debug_selector(move || format!("markdown-close-tab-{id}"))
                        .icon(IconName::Close)
                        .xsmall()
                        .ghost()
                        .accessibility_label("关闭标签页")
                        .on_click(cx.listener(move |_, _, window, cx| {
                            cx.stop_propagation();
                            cx.defer_in(window, move |this, window, cx| {
                                this.request_close(Some(id), window, cx)
                            });
                        })),
                )
            })
            .collect::<Vec<_>>();
        let page = presentation::paper(cx);
        let owner = cx.weak_entity();
        let focused = self.active_tab().is_some();
        div()
            .id("markdown-reader")
            .debug_selector(|| "markdown-reader".into())
            .track_focus(&self.focus)
            .key_context("MarkdownReader")
            .on_action(cx.listener(|this, _: &OpenDocument, window, cx| this.pick(window, cx)))
            .on_action(cx.listener(|this, _: &SaveDocument, window, cx| this.save(window, cx)))
            .on_action(cx.listener(|this, _: &FindDocument, window, cx| {
                if this.active_tab().is_some() {
                    this.set_find_visible(true, window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &DismissFind, window, cx| {
                if this.dismiss_outline() {
                    cx.notify();
                } else if this.find_visible {
                    this.set_find_visible(false, window, cx);
                } else {
                    cx.propagate();
                }
            }))
            .on_drop(move |paths: &ExternalPaths, window, cx| {
                let paths = paths.paths().to_vec();
                let owner = owner.clone();
                window.defer(cx, move |window, cx| {
                    let _ = owner.update(cx, |this, cx| {
                        this.open_paths(paths, None, Uuid::new_v4(), window, cx)
                    });
                });
            })
            .size_full()
            .flex()
            .flex_col()
            .bg(page.background)
            .text_color(page.ink)
            .child(
                crate::ui::components::titlebar::slotted_controlled_titlebar(
                    "markdown-reader-caption",
                    "markdown-reader-close",
                    div()
                        .id("markdown-preview-title")
                        .debug_selector(|| "markdown-preview-title".into())
                        .h_full()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap_1()
                        .px_2()
                        .bg(page.background)
                        .when(tabs.is_empty(), |bar| bar.child("Markdown 阅读器"))
                        .when(!tabs.is_empty(), |bar| {
                            bar.child(
                                div()
                                    .id("markdown-file-tabs")
                                    .debug_selector(|| "markdown-file-tabs".into())
                                    .min_w_0()
                                    .max_w(gpui_kit::relative(0.5))
                                    .child(
                                        TabBar::new("markdown-reader-tabs")
                                            .folder()
                                            .small()
                                            .max_width(px(220.))
                                            .selected_index(self.active)
                                            .children(tabs)
                                            .on_click(cx.listener(|this, index: &usize, _, cx| {
                                                this.activate(*index, cx)
                                            })),
                                    ),
                            )
                        })
                        .child(
                            div()
                                .id("caption-drag-region")
                                .debug_selector(|| "caption-drag-region".into())
                                .h_full()
                                .min_w(px(12.))
                                .flex_1()
                                .window_control_area(gpui_kit::WindowControlArea::Drag),
                        )
                        .child(self.mode_actions(cx))
                        .child(
                            Button::new("markdown-font-smaller")
                                .debug_selector(|| "markdown-font-smaller".into())
                                .label("A−")
                                .small()
                                .outline()
                                .disabled(!focused || self.font <= 12.)
                                .accessibility_label("减小字号")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.font -= 1.;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("markdown-font-larger")
                                .debug_selector(|| "markdown-font-larger".into())
                                .label("A+")
                                .small()
                                .outline()
                                .disabled(!focused || self.font >= 28.)
                                .accessibility_label("增大字号")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.font += 1.;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("markdown-measure")
                                .label(if self.measure == presentation::STANDARD_MEASURE {
                                    "宽版"
                                } else {
                                    "窄版"
                                })
                                .small()
                                .outline()
                                .disabled(!focused)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.measure = if this.measure == presentation::STANDARD_MEASURE
                                    {
                                        presentation::WIDE_MEASURE
                                    } else {
                                        presentation::STANDARD_MEASURE
                                    };
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("markdown-find")
                                .debug_selector(|| "markdown-find".into())
                                .label("查找")
                                .small()
                                .outline()
                                .disabled(!focused)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.set_find_visible(!this.find_visible, window, cx)
                                })),
                        )
                        .child(self.export_menu(cx))
                        .child(self.history_button(cx)),
                    close,
                ),
            )
            .when(self.find_visible, |body| body.child(self.render_find(cx)))
            .when_some(
                self.notice.filter(|_| self.problem.is_none()),
                |body, notice| {
                    body.child(
                        gpui_kit::component::alert::Alert::success(
                            "markdown-reader-notice",
                            notice,
                        )
                        .small(),
                    )
                },
            )
            .when_some(
                self.problem
                    .or_else(|| self.active_tab().and_then(|tab| tab.problem)),
                |body, problem| {
                    body.child(
                        gpui_kit::component::alert::Alert::error("markdown-reader-error", problem)
                            .small(),
                    )
                },
            )
            .child(if let Some(history) = &self.history {
                self.render_history(history, cx).into_any_element()
            } else if let Some(tab) = self.active_tab() {
                self.render_mode(tab, viewport, cx)
            } else {
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(gpui_kit::component::empty::Empty::new().child(
                        gpui_kit::component::empty::EmptyTitle::new().child(
                            if self.load.is_some() {
                                "正在读取 Markdown…"
                            } else {
                                "按 Ctrl+O 打开，或将 Markdown 文件拖入此处"
                            },
                        ),
                    ))
                    .into_any_element()
            })
            .when_some(self.active_tab(), |body, tab| {
                body.child(
                    div()
                        .px_2()
                        .py_1()
                        .text_xs()
                        .text_color(page.muted)
                        .truncate()
                        .child(tab.document.path.to_string_lossy().into_owned()),
                )
            })
    }
}

impl Reader {
    fn render_find(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (current, count) = self
            .find
            .as_ref()
            .filter(|find| !find.ranges.is_empty())
            .map_or((0, 0), |find| (find.current + 1, find.ranges.len()));
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .child(
                InputGroup::new("markdown-find-group")
                    .input(Input::new(&self.find_input).aria_label("查找正文")),
            )
            .child(format!("{current} / {count}"))
            .child(
                Button::new("markdown-find-prev")
                    .label("上一处")
                    .small()
                    .outline()
                    .disabled(count == 0)
                    .on_click(cx.listener(|this, _, _, cx| this.step_find(-1, cx))),
            )
            .child(
                Button::new("markdown-find-next")
                    .label("下一处")
                    .small()
                    .outline()
                    .disabled(count == 0)
                    .on_click(cx.listener(|this, _, _, cx| this.step_find(1, cx))),
            )
    }

    fn render_document(
        &self,
        tab: &TabDocument,
        viewport: gpui_kit::Size<gpui_kit::Pixels>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let link_owner = cx.weak_entity();
        let width = viewport.width;
        let has_outline = self.outline_headings(tab).len() >= 2;
        let layout =
            presentation::article_layout(width, self.measure, has_outline, self.outline_visible);
        let prose = presentation::prose_style(self.font, cx);
        let id = tab.id;
        let document_path = tab.document.path.clone();
        let images = Arc::clone(&tab.document.images);
        let hovered_code = tab.navigation.hovered_code;
        div()
            .relative()
            .flex()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .when(layout.pinned, |body| {
                body.child(
                    div()
                        .absolute()
                        .left(layout.outline_left)
                        .top(layout.inset)
                        .bottom(px(16.))
                        .flex()
                        .child(self.pinned_outline(tab, cx)),
                )
            })
            .when(has_outline && !layout.pinned, |body| {
                body.child(
                    div()
                        .absolute()
                        .left(layout.outline_left)
                        .top(layout.inset)
                        .child(self.outline_rail(tab, viewport.height, width, &layout, cx)),
                )
            })
            .child(
                div()
                    .id("markdown-prose-column")
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .on_mouse_move(cx.listener(
                        |reader, event: &gpui_kit::MouseMoveEvent, _, cx| {
                            reader.update_code_hover(Some(event.position), cx);
                        },
                    ))
                    .on_hover(cx.listener(|reader, hovered: &bool, _, cx| {
                        if !hovered {
                            reader.update_code_hover(None, cx);
                        }
                    }))
                    .left(layout.left)
                    .w(layout.width)
                    .debug_selector(|| "markdown-prose-viewport".into())
                    .child(
                        gpui_kit::base::TextView::new(&tab.view)
                            .w_full()
                            .h_full()
                            .min_w_0()
                            .py(layout.inset)
                            .text_size(px(self.font))
                            .line_height(gpui_kit::relative(1.5))
                            .style(prose)
                            .code_block_actions(move |block, window, cx| {
                                presentation::code_actions(block, hovered_code, window, cx)
                            })
                            .selectable(true)
                            .scrollable(true)
                            .image_source(move |uri| {
                                images.get(uri.as_ref()).map_or_else(
                                    || {
                                        gpui_kit::ImageSource::from(
                                            |_: &mut Window, _: &mut App| None,
                                        )
                                    },
                                    |path| path.clone().into(),
                                )
                            })
                            .on_link_click(move |link, event, window, cx| {
                                if event.is_right_click() {
                                    return;
                                }
                                let owner = link_owner.clone();
                                let path = document_path.clone();
                                let link = link.to_string();
                                window.defer(cx, move |window, cx| {
                                    let _ = owner.update(cx, |this, cx| {
                                        if this.active_tab().is_none_or(|tab| tab.id != id) {
                                            return;
                                        }
                                        if let Some(anchor) = link.strip_prefix('#') {
                                            this.reveal_heading(anchor, cx);
                                        } else if link.starts_with("https://")
                                            || link.starts_with("http://")
                                        {
                                            cx.open_url(&link);
                                        } else if let Some(target) =
                                            document::relative_path(&path, &link)
                                        {
                                            this.open_paths(
                                                vec![target],
                                                link.split_once('#')
                                                    .map(|(_, anchor)| anchor.to_owned()),
                                                Uuid::new_v4(),
                                                window,
                                                cx,
                                            );
                                        }
                                    });
                                });
                            }),
                    ),
            )
            .child(self.heading_flash(tab, cx))
            .child(self.preview_probe(tab, cx))
    }
}

fn file_name(document: &Document) -> SharedString {
    document
        .path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
        .into()
}
