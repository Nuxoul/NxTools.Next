//! Document-relative chapter rail and its in-pane hover card.
use super::*;
use gpui_kit::component::{list::ListItem, tree::Tree};
use gpui_kit::{
    AnyElement, Bounds, FontWeight, Pixels, RenderOnce, deferred, ease_out_quint, point, size,
};

const DWELL: Duration = Duration::from_millis(200);
const FADE: Duration = Duration::from_millis(160);
const ROW: f32 = 20.;
const STEP: f32 = 8.;
const PAD: f32 = 4.;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Closed,
    Hover,
    Keyboard,
    Closing,
}

#[derive(Default)]
pub(super) struct OutlineOverlay {
    phase: Phase,
    rail: bool,
    card: bool,
    anchor: usize,
    started: Option<Instant>,
    timer: Option<Task<()>>,
}

enum After {
    Dwell,
    Leave,
    Fade,
}

impl Reader {
    pub(super) fn dismiss_outline(&mut self) -> bool {
        let was_visible = self.outline_overlay.phase != Phase::Closed;
        self.outline_overlay = OutlineOverlay::default();
        was_visible
    }

    pub(super) fn outline_headings<'a>(&self, tab: &'a TabDocument) -> &'a [Heading] {
        tab.navigation.headings(&tab.document)
    }

    fn outline_target(&self, id: usize) -> Option<&TabDocument> {
        self.active_tab()
            .filter(|tab| self.history.is_none() && tab.id == id && tab.draft.shows_preview())
    }

    fn begin_outline(
        &mut self,
        id: usize,
        phase: Phase,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.outline_target(id) else {
            return;
        };
        let anchor = tab.navigation.active_heading.unwrap_or(0);
        let count = self.outline_headings(tab).len();
        let tree = tab.outline.clone();
        self.outline_overlay.phase = phase;
        self.outline_overlay.anchor = anchor;
        self.outline_overlay.started = Some(cx.background_executor().now());
        self.outline_overlay.timer = None;
        let rows = visible_rows(window.viewport_size().height, count);
        let start = row_start(anchor, rows, count);
        tree.update(cx, |state, cx| {
            state.scroll_to_item(start, gpui_kit::ScrollStrategy::Top);
            if phase == Phase::Keyboard {
                state.focus(window, cx);
            }
        });
        cx.notify();
    }

    fn outline_timer(
        &mut self,
        id: usize,
        after: After,
        delay: Duration,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let delay = cx.background_executor().timer(delay);
        self.outline_overlay.timer = Some(cx.spawn_in(window, async move |owner, cx| {
            delay.await;
            let _ = owner.update_in(cx, |this, window, cx| {
                this.outline_overlay.timer = None;
                if this.outline_target(id).is_none() {
                    return;
                }
                let inside = this.outline_overlay.rail || this.outline_overlay.card;
                match after {
                    After::Dwell if inside => this.begin_outline(id, Phase::Hover, window, cx),
                    After::Leave if !inside && this.outline_overlay.phase == Phase::Hover => {
                        this.outline_overlay.phase = Phase::Closing;
                        this.outline_overlay.started = Some(cx.background_executor().now());
                        this.outline_timer(id, After::Fade, FADE, window, cx);
                        cx.notify();
                    }
                    After::Fade if this.outline_overlay.phase == Phase::Closing => {
                        this.outline_overlay.phase = Phase::Closed;
                        cx.notify();
                    }
                    _ => {}
                }
            });
        }));
    }

    fn outline_hover(
        &mut self,
        id: usize,
        card: bool,
        inside: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.outline_target(id).is_none() {
            return;
        }
        let state = &mut self.outline_overlay;
        let before = state.rail || state.card;
        if card {
            state.card = inside;
        } else {
            state.rail = inside;
        }
        let after = state.rail || state.card;
        if before == after || state.phase == Phase::Keyboard {
            return;
        }
        state.timer = None;
        match (after, state.phase) {
            (true, Phase::Closed | Phase::Closing) => {
                self.outline_timer(id, After::Dwell, DWELL, window, cx)
            }
            // Both hover callbacks run for one pointer move; defer the leave decision
            // so entering the card from the rail can cancel it without flicker.
            (false, Phase::Hover) => {
                self.outline_timer(id, After::Leave, Duration::from_millis(16), window, cx)
            }
            (false, Phase::Closing) => self.outline_timer(id, After::Fade, FADE, window, cx),
            _ => {}
        }
    }

    pub(super) fn outline_rail(
        &self,
        tab: &TabDocument,
        height: Pixels,
        width: Pixels,
        layout: &presentation::ArticleLayout,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let id = tab.id;
        let headings = self.outline_headings(tab);
        let active = tab.navigation.active_heading;
        let anchor = self.outline_overlay.anchor;
        let phase = self.outline_overlay.phase;
        let shown = headings
            .len()
            .min(((height - layout.inset * 2.).as_f32() / STEP).max(2.) as usize);
        let mut marks = div().flex().flex_col().items_start().gap(px(5.));
        for index in 0..shown {
            marks = marks.child(dash(cx, active == Some(index), index));
        }
        let rail_height = px((shown as f32 * STEP).max(20.));
        let trigger = Button::new(("markdown-outline-trigger", id))
            .debug_selector(|| "markdown-outline-trigger".into())
            .accessibility_label("文档大纲")
            .ghost()
            .xsmall()
            .w(px(presentation::RAIL_WIDTH))
            .h(rail_height)
            .p_0()
            .child(marks)
            .on_click(
                cx.listener(move |_, event: &gpui_kit::ClickEvent, window, cx| {
                    if event.is_keyboard() {
                        cx.defer_in(window, move |this, window, cx| {
                            this.begin_outline(id, Phase::Keyboard, window, cx)
                        });
                    }
                }),
            );
        let root = div().relative().w(px(presentation::RAIL_WIDTH)).child(
            div()
                .id(("markdown-outline-rail", id))
                .debug_selector(|| "markdown-outline-rail".into())
                .opacity(if phase == Phase::Closed { 1. } else { 0. })
                .on_hover(cx.listener(move |this, inside, window, cx| {
                    this.outline_hover(id, false, *inside, window, cx)
                }))
                .child(trigger),
        );
        if phase == Phase::Closed {
            return root;
        }
        let rows = visible_rows(height, headings.len());
        let start = row_start(anchor, rows, headings.len());
        let card_height = px(rows as f32 * ROW + PAD * 2. + 2.);
        let row = anchor.saturating_sub(start).min(rows.saturating_sub(1));
        let anchor_y = anchor.min(shown.saturating_sub(1)) as f32 * STEP + STEP / 2.;
        let offset = px(anchor_y - PAD - (row as f32 + 0.5) * ROW);
        let top = offset
            .min(height - layout.inset - card_height - px(8.))
            .max(px(8.) - layout.inset);
        let card_width = px(320.).min((width - layout.outline_left - px(8.)).max(px(120.)));
        let anchor = active
            .and_then(|index| headings.get(index))
            .map(|heading| heading.anchor.clone());
        let owner = cx.weak_entity();
        let interactive = phase != Phase::Closing;
        let theme = Theme::active(cx);
        let card = div()
            .id(("markdown-outline-panel", id))
            .debug_selector(|| "markdown-outline-panel".into())
            .absolute()
            .w(card_width)
            .h(card_height)
            .rounded(px(9.))
            .border_1()
            .border_color(theme.border_subtle)
            .bg(theme.surface)
            .shadow_md()
            .p(px(PAD))
            .flex()
            .flex_col()
            .when(interactive, |card| {
                card.occlude()
                    .on_mouse_down(gpui_kit::MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation()
                    })
                    .on_hover(cx.listener(move |this, inside, window, cx| {
                        this.outline_hover(id, true, *inside, window, cx)
                    }))
            })
            .child(outline_tree(
                &tab.outline,
                id,
                anchor,
                owner.clone(),
                interactive,
            ))
            .child(
                div()
                    .absolute()
                    .top(px(PAD - 1.))
                    .right(px(PAD - 1.))
                    .child(
                        outline_pin(
                            id,
                            self.outline_visible,
                            presentation::can_pin_outline(width),
                            owner,
                        )
                        .disabled(!interactive),
                    ),
            );
        let bounds = Bounds::new(point(px(-6.), top), size(card_width, card_height));
        let rail = Bounds::new(
            point(px(-6.), px(0.)),
            size(px(presentation::RAIL_WIDTH + 6.), rail_height),
        );
        let reveal = OutlineReveal {
            card: card.into_any_element(),
            bounds,
            seed: bounds.intersect(&rail),
            started: self.outline_overlay.started,
            closing: !interactive,
        };
        root.child(deferred(reveal).with_priority(gpui_kit::base::POPUP_PRIORITY))
    }

    pub(super) fn pinned_outline(&self, tab: &TabDocument, cx: &Context<Self>) -> impl IntoElement {
        let active = tab
            .navigation
            .active_heading
            .and_then(|index| self.outline_headings(tab).get(index));
        div()
            .id(("markdown-outline-pinned", tab.id))
            .debug_selector(|| "markdown-outline-pinned".into())
            .relative()
            .w(px(presentation::SIDEBAR_WIDTH))
            .flex_none()
            .min_h_0()
            .h_full()
            .flex()
            .flex_col()
            .on_mouse_down(gpui_kit::MouseButton::Left, |_, _, cx| {
                cx.stop_propagation()
            })
            .child(outline_tree(
                &tab.outline,
                tab.id,
                active.map(|h| h.anchor.clone()),
                cx.weak_entity(),
                true,
            ))
            .child(div().absolute().top_0().right_0().child(outline_pin(
                tab.id,
                true,
                true,
                cx.weak_entity(),
            )))
    }
}

#[derive(IntoElement)]
struct OutlineReveal {
    card: AnyElement,
    bounds: Bounds<Pixels>,
    seed: Bounds<Pixels>,
    started: Option<Instant>,
    closing: bool,
}

impl RenderOnce for OutlineReveal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = self
            .started
            .filter(|_| !cx.reduce_motion())
            .map_or(1., |started| {
                let elapsed = cx
                    .background_executor()
                    .now()
                    .saturating_duration_since(started);
                (elapsed.as_secs_f32() / FADE.as_secs_f32()).min(1.)
            });
        if t < 1. {
            window.request_animation_frame();
        }
        let eased = ease_out_quint()(t);
        let progress = if self.closing { 1. - eased } else { eased };
        let mix = |from, to| from + (to - from) * progress;
        let left = mix(self.seed.left(), self.bounds.left());
        let top = mix(self.seed.top(), self.bounds.top());
        let right = mix(self.seed.right(), self.bounds.right());
        let bottom = mix(self.seed.bottom(), self.bounds.bottom());
        div()
            .id("markdown-outline-reveal")
            .debug_selector(|| "markdown-outline-reveal".into())
            .absolute()
            .left(left)
            .top(top)
            .w(right - left)
            .h(bottom - top)
            .overflow_hidden()
            .opacity(progress)
            .child(
                div()
                    .absolute()
                    .left(self.bounds.left() - left)
                    .top(self.bounds.top() - top)
                    .w(self.bounds.size.width)
                    .h(self.bounds.size.height)
                    .child(self.card),
            )
    }
}

fn dash(cx: &App, active: bool, index: usize) -> impl IntoElement {
    let theme = Theme::active(cx);
    div()
        .id(("markdown-outline-mark", index))
        .flex_none()
        .when(active, |mark| {
            mark.debug_selector(move || format!("markdown-outline-active-{index}"))
        })
        .w(px(if active { 18. } else { 14. }))
        .h(px(if active { 3. } else { 2. }))
        .rounded_full()
        .bg(if active {
            theme.accent
        } else {
            theme.text_secondary.opacity(0.55)
        })
}

fn visible_rows(height: Pixels, count: usize) -> usize {
    count
        .min(((height.as_f32() - 70.) / ROW).floor().max(1.) as usize)
        .max(1)
}

fn row_start(anchor: usize, rows: usize, count: usize) -> usize {
    anchor
        .saturating_sub(rows / 2)
        .min(count.saturating_sub(rows))
}

fn outline_pin(id: usize, pinned: bool, fits: bool, owner: gpui_kit::WeakEntity<Reader>) -> Button {
    let label = if pinned { "取消固定" } else { "固定" };
    Button::new(("markdown-outline-pin", id))
        .debug_selector(|| "markdown-outline-pin".into())
        .icon(gpui_kit::component::Icon::new(crate::icons::MaterialIcon::Keep))
        .tooltip(label).accessibility_label(label).xsmall().ghost()
        .on_click(move |event, window, cx| {
            cx.stop_propagation();
            let keyboard = event.is_keyboard();
            let owner = owner.clone();
            window.defer(cx, move |window, cx| {
                let _ = owner.update(cx, |this, cx| {
                    if this.outline_target(id).is_none() { return; }
                    let correlation = Uuid::new_v4();
                    let started = Instant::now();
                    tracing::info!(event = "markdown.outline_pin.started", correlation_id = %correlation, pinned = !pinned);
                    this.outline_visible = !pinned;
                    if pinned && fits {
                        this.outline_overlay.card = true;
                        this.begin_outline(id, if keyboard { Phase::Keyboard } else { Phase::Hover }, window, cx);
                    } else if fits {
                        this.dismiss_outline();
                    }
                    tracing::info!(event = "markdown.outline_pin.finished", correlation_id = %correlation, pinned = !pinned, elapsed_ms = started.elapsed().as_millis());
                    cx.notify();
                });
            });
        })
}

fn outline_tree(
    tree: &Entity<TreeState>,
    id: usize,
    active: Option<String>,
    owner: gpui_kit::WeakEntity<Reader>,
    interactive: bool,
) -> impl IntoElement {
    let confirm_tree = tree.clone();
    let confirm_owner = owner.clone();
    let rows = Tree::new(tree, move |index, entry, _, _, cx| {
        let anchor = entry.item().id.clone();
        let title = entry.item().label.clone();
        let tooltip_title = title.clone();
        let current = active.as_deref() == Some(anchor.as_ref());
        let owner = owner.clone();
        let indent = px(entry.depth().min(4) as f32 * 6.);
        ListItem::new(anchor.clone())
            .h(px(ROW))
            .py_0()
            .px_1()
            .text_sm()
            .line_height(px(18.))
            .rounded(px(5.))
            .accessibility_label(if current {
                format!("{title}，当前章节").into()
            } else {
                title.clone()
            })
            .tooltip(move |window, cx| {
                gpui_kit::component::tooltip::Tooltip::new(tooltip_title.clone()).build(window, cx)
            })
            .child(
                div()
                    .min_w_0()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .pr(px(if index == 0 { 20. } else { 2. }))
                    .child(dash(cx, current, index))
                    .child(
                        div()
                            .id(("markdown-outline-title", index))
                            .min_w_0()
                            .truncate()
                            .ml(indent)
                            .when(current, |label| label.font_weight(FontWeight::SEMIBOLD))
                            .debug_selector(move || format!("markdown-outline-title-{index}"))
                            .child(title),
                    ),
            )
            .when(interactive, |row| {
                row.on_click(move |_, window, cx| {
                    let owner = owner.clone();
                    let anchor = anchor.clone();
                    window.defer(cx, move |window, cx| {
                        let _ =
                            owner.update(cx, |this, cx| this.jump_outline(id, &anchor, window, cx));
                    });
                })
            })
    });
    div()
        .id(("markdown-outline-tree-container", id))
        .flex_1()
        .min_h_0()
        .key_context("MarkdownOutline")
        .on_action(move |_: &JumpOutlineHeading, window, cx| {
            if !interactive {
                return;
            }
            let Some(anchor) = confirm_tree
                .read(cx)
                .selected_item()
                .map(|item| item.id.clone())
            else {
                return;
            };
            cx.stop_propagation();
            let owner = confirm_owner.clone();
            window.defer(cx, move |window, cx| {
                let _ = owner.update(cx, |this, cx| this.jump_outline(id, &anchor, window, cx));
            });
        })
        .child(rows)
}
