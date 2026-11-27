//! Source locations for the two laid-out panes of the Markdown reader.
use std::cell::Cell;

use gpui_kit::base::text::{
    MarkdownExtensions,
    editor_host::{BlockNode, parse_markdown},
};
use gpui_kit::{AnyElement, ListOffset, Pixels, canvas};

use super::*;
use crate::ui::motion::{self, UiMotionPreference};

struct Projection {
    blocks: Vec<(usize, Range<usize>)>,
    block_count: usize,
    headings: Vec<Heading>,
    fences: Vec<(usize, usize)>,
}

const HEADING_FLASH: Duration = Duration::from_millis(1200);

impl Projection {
    fn new(source: &str, headings: Vec<Heading>) -> Self {
        let parsed =
            parse_markdown(source, Arc::new(MarkdownExtensions::default())).unwrap_or_default();
        let fences = parsed
            .iter()
            .enumerate()
            .filter_map(|(index, block)| match block {
                BlockNode::CodeBlock(code) => code.span.map(|span| (index, span.start)),
                _ => None,
            })
            .collect();
        Self {
            block_count: parsed.len(),
            blocks: parsed
                .iter()
                .enumerate()
                .filter_map(|(index, block)| block.span().map(|span| (index, span.start..span.end)))
                .collect(),
            fences,
            headings: headings
                .into_iter()
                .filter(|heading| !heading.title.trim().is_empty())
                .collect(),
        }
    }

    fn source_at(&self, block: usize) -> Option<usize> {
        let next = self.blocks.partition_point(|(index, _)| *index <= block);
        self.blocks
            .get(next.saturating_sub(1))
            .map(|(_, span)| span.start)
    }

    fn block_at(&self, source: usize) -> Option<usize> {
        let next = self
            .blocks
            .partition_point(|(_, span)| span.start <= source);
        let previous = self.blocks.get(next.saturating_sub(1))?;
        Some(if source >= previous.1.end {
            self.blocks.get(next).unwrap_or(previous).0
        } else {
            previous.0
        })
    }
}

#[derive(Clone, Copy, PartialEq)]
struct PreviewFrame {
    width: Pixels,
    split: bool,
    top: usize,
    within_block: Pixels,
    at_end: bool,
}

#[derive(Default)]
pub(super) struct Navigation {
    projection: Option<(RenderedText, Arc<Projection>)>,
    projection_current: bool,
    project_task: Option<(RenderedText, Task<()>)>,
    preview_probe: Rc<Cell<Option<PreviewFrame>>>,
    source_probe: Rc<Cell<Option<usize>>>,
    preview: Option<PreviewFrame>,
    source_line: Option<usize>,
    follow_preview: bool,
    follow_source: bool,
    pending_preview: Option<usize>,
    pending_source: Option<usize>,
    picked: Option<usize>,
    pub(super) active_heading: Option<usize>,
    pub(super) hovered_code: Option<usize>,
    heading_flash: Option<(usize, Instant, RenderedText)>,
    motion_subscription: Option<Subscription>,
}

impl Navigation {
    fn observe_heading_flash_motion(&mut self, id: usize, cx: &mut Context<Reader>) {
        if self.motion_subscription.is_some() {
            return;
        }
        self.motion_subscription =
            Some(cx.observe_global::<UiMotionPreference>(move |reader, cx| {
                if motion::reduced_motion(cx)
                    && let Some(tab) = reader.tabs.iter_mut().find(|tab| tab.id == id)
                    && tab.navigation.heading_flash.take().is_some()
                {
                    let owner = cx.weak_entity();
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |_, cx| cx.notify());
                    });
                }
            }));
    }

    pub(super) fn headings<'a>(&'a self, document: &'a Document) -> &'a [Heading] {
        self.projection
            .as_ref()
            .map_or(&document.headings, |(_, map)| &map.headings)
    }

    pub(super) fn content_changed(&mut self) {
        self.projection_current = false;
        self.project_task = None;
        self.pending_preview = None;
        self.pending_source = None;
        self.picked = None;
        self.hovered_code = None;
        self.heading_flash = None;
    }

    fn current_map(&self, view: &TextViewState) -> Option<&Projection> {
        let (rendered, map) = self.projection.as_ref()?;
        (self.projection_current
            && view.rendered_text() == *rendered
            && view.list_state().item_count() == map.block_count)
            .then_some(map.as_ref())
    }
}

impl TabDocument {
    fn committed_map(&self, cx: &App) -> Option<&Projection> {
        self.navigation.current_map(self.view.read(cx))
    }

    pub(super) fn preview_source(&self, cx: &App) -> Option<usize> {
        let view = self.view.read(cx);
        self.committed_map(cx)?
            .source_at(view.list_state().logical_scroll_top().item_ix)
    }

    pub(super) fn enter_source_at(&mut self, offset: Option<usize>) {
        self.navigation.pending_source = offset;
        self.navigation.source_line = None;
        self.navigation.source_probe.set(None);
    }

    pub(super) fn enter_preview_at(&mut self, offset: Option<usize>) {
        self.navigation.pending_preview = offset;
        self.navigation.preview_probe.set(None);
    }
}

impl Reader {
    pub(super) fn project_navigation(&mut self, id: usize, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) else {
            return;
        };
        let rendered = tab.view.read(cx).rendered_text();
        let known = tab
            .navigation
            .projection
            .as_ref()
            .is_some_and(|(built, _)| *built == rendered);
        if (known && tab.navigation.projection_current)
            || tab
                .navigation
                .project_task
                .as_ref()
                .is_some_and(|(building, _)| *building == rendered)
            || rendered.source() != tab.draft.text(&tab.document.text)
        {
            return;
        }
        if known {
            tab.navigation.projection_current = true;
            tab.navigation.preview_probe.set(None);
            cx.notify();
            return;
        }
        let source = rendered.source().to_owned();
        let path = tab.document.path.clone();
        let headings = (!tab.draft.dirty()).then(|| tab.document.headings.clone());
        let work = cx.background_spawn(async move {
            let headings = headings.unwrap_or_else(|| document::project(&path, &source).0);
            Projection::new(&source, headings)
        });
        let revision = rendered.clone();
        tab.navigation.project_task = Some((
            rendered,
            cx.spawn(async move |owner, cx| {
                let map = work.await;
                let _ = owner.update(cx, |reader, cx| {
                    let Some(tab) = reader.tabs.iter_mut().find(|tab| tab.id == id) else {
                        return;
                    };
                    tab.navigation.project_task = None;
                    if tab.view.read(cx).rendered_text() != revision
                        || revision.source() != tab.draft.text(&tab.document.text)
                    {
                        return;
                    }
                    // Source-only edits may change geometry without changing the tree.
                    let old = tab.navigation.headings(&tab.document);
                    if !old
                        .iter()
                        .map(|h| (&h.anchor, &h.title, h.level))
                        .eq(map.headings.iter().map(|h| (&h.anchor, &h.title, h.level)))
                    {
                        tab.outline.update(cx, |tree, cx| {
                            tree.set_items(outline_items(&map.headings), cx)
                        });
                    }
                    tab.navigation.projection = Some((revision, Arc::new(map)));
                    tab.navigation.projection_current = true;
                    tab.navigation.picked = None;
                    tab.navigation.preview_probe.set(None);
                    cx.notify();
                });
            }),
        ));
    }

    pub(super) fn heading_flash(&self, tab: &TabDocument, cx: &Context<Self>) -> AnyElement {
        let Some((index, started, revision)) = tab.navigation.heading_flash.clone() else {
            return div().absolute().into_any_element();
        };
        let color = Theme::active(cx).accent;
        let view = tab.view.clone();
        let owner = cx.weak_entity();
        let id = tab.id;
        let flash = (index, started, revision.clone());
        canvas(
            move |_, window, cx| {
                let elapsed = cx
                    .background_executor()
                    .now()
                    .saturating_duration_since(started);
                let state = view.read(cx);
                if !motion::motion_allowed(cx)
                    || elapsed >= HEADING_FLASH
                    || state.rendered_text() != revision
                {
                    window.defer(cx, move |_, cx| {
                        let _ = owner.update(cx, |reader, _| {
                            if let Some(tab) = reader.tabs.iter_mut().find(|tab| tab.id == id)
                                && tab.navigation.heading_flash.as_ref() == Some(&flash)
                            {
                                tab.navigation.heading_flash = None;
                            }
                        });
                    });
                    return None;
                }
                let list = state.list_state();
                let rect = list
                    .bounds_for_item(index)?
                    .intersect(&list.viewport_bounds());
                let alpha = 0.14 * (1. - elapsed.as_secs_f32() / HEADING_FLASH.as_secs_f32());
                (rect.size.height > px(0.)).then_some((rect, alpha))
            },
            move |_, frame, window, cx| {
                if motion::motion_allowed(cx)
                    && let Some((rect, alpha)) = frame
                {
                    window.paint_quad(gpui_kit::fill(rect, color.opacity(alpha)));
                    window.request_animation_frame();
                }
            },
        )
        .absolute()
        .inset_0()
        .into_any_element()
    }

    pub(super) fn update_code_hover(
        &mut self,
        position: Option<gpui_kit::Point<Pixels>>,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return;
        };
        let hovered = position.and_then(|position| {
            let map = tab.committed_map(cx)?;
            let state = tab.view.read(cx);
            let list = state.list_state();
            let frame = list.viewport_bounds();
            if !frame.contains(&position) {
                return None;
            }
            let first = list.logical_scroll_top().item_ix;
            map.fences[map.fences.partition_point(|&(index, _)| index < first)..]
                .iter()
                .filter_map(|&(index, span)| {
                    list.bounds_for_item(index).map(|bounds| (span, bounds))
                })
                .take_while(|(_, bounds)| bounds.top() < frame.bottom())
                .find_map(|(span, bounds)| bounds.contains(&position).then_some(span))
        });
        if tab.navigation.hovered_code != hovered {
            tab.navigation.hovered_code = hovered;
            cx.notify();
        }
    }

    pub(super) fn preview_probe(&self, tab: &TabDocument, cx: &Context<Self>) -> AnyElement {
        let id = tab.id;
        let owner = cx.weak_entity();
        let view = tab.view.clone();
        let previous = tab.navigation.preview_probe.clone();
        let split = tab.draft.is_split();
        canvas(
            move |bounds, window, cx| {
                let list = view.read(cx).list_state();
                let offset = list.logical_scroll_top();
                let max = list.max_offset_for_scrollbar().y;
                let frame = PreviewFrame {
                    width: bounds.size.width,
                    split,
                    top: offset.item_ix,
                    within_block: offset.offset_in_item,
                    at_end: max > px(0.) && max + list.scroll_px_offset_for_scrollbar().y <= px(1.),
                };
                if previous.replace(Some(frame)) != Some(frame) {
                    window.defer(cx, move |window, cx| {
                        let _ = owner
                            .update(cx, |this, cx| this.preview_measured(id, frame, window, cx));
                    });
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0()
        .into_any_element()
    }

    pub(super) fn source_probe(&self, tab: &TabDocument, cx: &Context<Self>) -> AnyElement {
        let id = tab.id;
        let owner = cx.weak_entity();
        let source = tab.draft.source().cloned();
        let previous = tab.navigation.source_probe.clone();
        canvas(
            |_, _, _| (),
            move |_, (), window, cx| {
                let Some(line) = source
                    .as_ref()
                    .and_then(|source| edit::top_line(source.read(cx)))
                else {
                    return;
                };
                if previous.replace(Some(line)) != Some(line) {
                    window.defer(cx, move |window, cx| {
                        let _ =
                            owner.update(cx, |this, cx| this.source_measured(id, line, window, cx));
                    });
                }
            },
        )
        .absolute()
        .inset_0()
        .into_any_element()
    }

    fn preview_measured(
        &mut self,
        id: usize,
        frame: PreviewFrame,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self
            .tabs
            .get_mut(self.active)
            .filter(|tab| tab.id == id && tab.draft.shows_preview())
        else {
            return;
        };
        if self.history.is_some() {
            return;
        }
        let previous = tab.navigation.preview.replace(frame);
        let width_changed =
            previous.is_none_or(|prev| (prev.width, prev.split) != (frame.width, frame.split));
        let moved = previous
            .is_some_and(|prev| (prev.top, prev.within_block) != (frame.top, frame.within_block));
        let following = std::mem::take(&mut tab.navigation.follow_preview);
        let Some(map) = tab.committed_map(cx) else {
            if width_changed {
                cx.notify();
            }
            return;
        };
        let top = map.source_at(frame.top);
        let active = if frame.at_end {
            map.headings.len().checked_sub(1)
        } else {
            top.and_then(|top| {
                map.headings
                    .partition_point(|heading| heading.source.start <= top)
                    .checked_sub(1)
            })
        };
        if moved && !following && !width_changed {
            tab.navigation.picked = None;
        }
        let active = tab.navigation.picked.or(active);
        let active_changed = tab.navigation.active_heading != active;
        tab.navigation.active_heading = active;
        if let Some(offset) = tab.navigation.pending_preview.take() {
            Self::scroll_preview(tab, offset, window, cx);
        } else if moved
            && !following
            && !width_changed
            && tab.draft.is_split()
            && let Some(top) = top
        {
            Self::scroll_source(tab, top, window, cx);
        }
        if active_changed || width_changed {
            cx.notify();
        }
    }

    fn source_measured(
        &mut self,
        id: usize,
        line: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self
            .tabs
            .get_mut(self.active)
            .filter(|tab| tab.id == id && tab.draft.shows_source())
        else {
            return;
        };
        if self.history.is_some() {
            return;
        }
        let previous = tab.navigation.source_line.replace(line);
        let following = std::mem::take(&mut tab.navigation.follow_source);
        if let Some(offset) = tab.navigation.pending_source.take() {
            Self::scroll_source(tab, offset, window, cx);
        } else if previous.is_some_and(|old| old != line)
            && !following
            && tab.draft.is_split()
            && let Some(offset) = tab.draft.source_offset_at(line, cx)
        {
            if tab.navigation.picked.take().is_some() {
                tab.navigation.preview_probe.set(None);
                window.refresh();
            }
            if tab.committed_map(cx).is_some() {
                Self::scroll_preview(tab, offset, window, cx);
            } else {
                tab.navigation.pending_preview = Some(offset);
            }
        }
    }

    fn scroll_preview(
        tab: &mut TabDocument,
        offset: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(block) = tab.committed_map(cx).and_then(|map| map.block_at(offset)) else {
            return;
        };
        let view = tab.view.read(cx);
        let list = view.list_state();
        let top = list.logical_scroll_top();
        if top.item_ix != block || top.offset_in_item != px(0.) {
            list.scroll_to(ListOffset {
                item_ix: block,
                offset_in_item: px(0.),
            });
            tab.navigation.follow_preview = true;
            tab.navigation.preview_probe.set(None);
            window.refresh();
        }
    }

    fn scroll_source(
        tab: &mut TabDocument,
        offset: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if tab.draft.align_source(offset, cx) {
            tab.navigation.follow_source = true;
            tab.navigation.source_probe.set(None);
            window.refresh();
        }
    }

    pub(super) fn jump_outline(
        &mut self,
        id: usize,
        anchor: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tabs.get_mut(self.active).filter(|tab| tab.id == id) else {
            return;
        };
        let Some((index, offset)) = tab.committed_map(cx).and_then(|map| {
            map.headings
                .iter()
                .enumerate()
                .find(|(_, h)| h.anchor == anchor)
                .map(|(index, heading)| (index, heading.source.start))
        }) else {
            return;
        };
        tab.navigation.observe_heading_flash_motion(id, cx);
        tab.navigation.picked = Some(index);
        tab.navigation.active_heading = Some(index);
        tab.navigation.heading_flash = if motion::motion_allowed(cx) {
            tab.committed_map(cx)
                .and_then(|map| map.block_at(offset))
                .map(|block| {
                    (
                        block,
                        cx.background_executor().now(),
                        tab.view.read(cx).rendered_text(),
                    )
                })
        } else {
            None
        };
        Self::scroll_preview(tab, offset, window, cx);
        if tab.draft.is_split() {
            Self::scroll_source(tab, offset, window, cx);
        }
        cx.notify();
    }
}
