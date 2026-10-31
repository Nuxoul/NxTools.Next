//! Transient gesture only. BrowserModel remains the sole selection/focus/anchor owner.
use super::*;
use gpui_kit::{Bounds, MouseDownEvent, MouseMoveEvent, Point, point, size};
use std::{cell::Cell, collections::BTreeSet, rc::Rc};

#[derive(Default)]
pub(super) struct MarqueeState {
    pub body: Rc<Cell<Bounds<Pixels>>>,
    pub viewport: Rc<Cell<Bounds<Pixels>>>,
    gesture: Option<Gesture>,
    pub(super) suppress_click: bool,
}
struct Gesture {
    origin: Point<Pixels>,
    pointer: Point<Pixels>,
    baseline: BTreeSet<RowId>,
    focus: Option<RowId>,
    anchor: Option<RowId>,
    control: bool,
    snapshot: Uuid,
    generation: u64,
    epoch: u64,
    correlation: Uuid,
    started: std::time::Instant,
}
impl ArchiveBrowser {
    pub(super) fn marquee_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.marquee.suppress_click = false;
        if self.retired
            || !matches!(self.status, BrowserStatus::Ready)
            || self.open_pending
            || event.click_count != 1
            || event.modifiers.alt
            || event.modifiers.platform
            || event.modifiers.shift
        {
            return;
        }
        let body = self.marquee.body.get();
        let viewport = self.marquee.viewport.get();
        // Scrollbar bands remain owned by the official bars, never rectangle selection.
        if !body.contains(&event.position)
            || event.position.x >= viewport.right() - px(12.)
            || event.position.y >= body.bottom() - px(12.)
        {
            return;
        }
        let Some(model) = self.model.as_ref() else {
            return;
        };
        let origin = self.marquee_content_point(event.position);
        self.marquee.gesture = Some(Gesture {
            origin,
            pointer: event.position,
            baseline: model.selected.clone(),
            focus: model.focused.clone(),
            anchor: model.anchor.clone(),
            control: event.modifiers.control,
            snapshot: model.snapshot,
            generation: self.generation,
            epoch: self.projection_epoch,
            correlation: Uuid::new_v4(),
            started: std::time::Instant::now(),
        });
        if let Some(gesture) = &self.marquee.gesture {
            tracing::info!(event="archive.marquee.started",correlation_id=%gesture.correlation,control=gesture.control,"archive rectangle selection accepted");
        }
        self.drag_up();
        self.list_focus.focus(window, cx);
        if !event.modifiers.control
            && let Some(model) = &mut self.model
        {
            model.clear_selection();
        }
        self.update_summary();
        cx.stop_propagation();
        cx.notify();
    }
    fn marquee_content_point(&self, pointer: Point<Pixels>) -> Point<Pixels> {
        let body = self.marquee.body.get();
        let offset = self.scroll.0.borrow().base_handle.offset();
        point(pointer.x - body.left(), pointer.y - body.top() - offset.y)
    }
    pub(super) fn marquee_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) -> bool {
        if self.marquee.gesture.is_none() {
            return false;
        }
        if event.pressed_button != Some(MouseButton::Left) {
            self.cancel_marquee(true, cx);
            return true;
        }
        if self.retired
            || !matches!(self.status, BrowserStatus::Ready)
            || self.marquee.gesture.as_ref().is_some_and(|g| {
                g.generation != self.generation
                    || g.epoch != self.projection_epoch
                    || self.model.as_ref().is_none_or(|m| m.snapshot != g.snapshot)
            })
        {
            self.cancel_marquee(false, cx);
            return true;
        }
        let row_width = if self.settings.full_row_select {
            self.columns_width()
        } else {
            self.column_width(ColumnKind::Name)
        };
        let body = self.marquee.body.get();
        let viewport = self.marquee.viewport.get();
        let pointer = point(
            event
                .position
                .x
                .clamp(viewport.left(), viewport.right() - px(12.)),
            event.position.y.clamp(body.top(), body.bottom() - px(12.)),
        );
        let current = self.marquee_content_point(pointer);
        let gesture = self.marquee.gesture.as_mut().expect("current gesture");
        gesture.pointer = pointer;
        self.marquee.suppress_click = true;
        let left = gesture.origin.x.min(current.x);
        let right = gesture.origin.x.max(current.x);
        let top = gesture.origin.y.min(current.y);
        let bottom = gesture.origin.y.max(current.y);
        if let Some(model) = &mut self.model {
            let hits = model
                .visible()
                .iter()
                .enumerate()
                .filter(|(index, row)| {
                    !row.parent
                        && right > px(0.)
                        && left < row_width
                        && bottom > px(*index as f32 * ROW_HEIGHT)
                        && top < px((*index + 1) as f32 * ROW_HEIGHT)
                })
                .map(|(_, row)| row.id.clone())
                .collect::<BTreeSet<_>>();
            model.select_rectangle(&gesture.baseline, hits, gesture.control);
        }
        self.update_summary();
        cx.notify();
        true
    }
    pub(super) fn marquee_up(&mut self, cx: &mut Context<Self>) {
        if let Some(gesture) = self.marquee.gesture.take() {
            tracing::info!(event="archive.marquee.finished",correlation_id=%gesture.correlation,outcome="selected",elapsed_ms=gesture.started.elapsed().as_millis() as u64,"archive rectangle selection finished");
            self.update_summary();
            cx.notify();
        }
    }
    pub(super) fn cancel_marquee(&mut self, restore: bool, cx: &mut Context<Self>) -> bool {
        let Some(gesture) = self.marquee.gesture.take() else {
            return false;
        };
        tracing::info!(event="archive.marquee.finished",correlation_id=%gesture.correlation,outcome=if restore {"cancelled"} else {"retired"},elapsed_ms=gesture.started.elapsed().as_millis() as u64,"archive rectangle selection finished");
        if restore
            && let Some(model) = self.model.as_mut().filter(|model| {
                model.snapshot == gesture.snapshot
                    && self.generation == gesture.generation
                    && self.projection_epoch == gesture.epoch
            })
        {
            model.restore_selection(gesture.baseline, gesture.focus, gesture.anchor);
        }
        self.marquee.suppress_click = true;
        self.update_summary();
        cx.notify();
        true
    }
    pub(super) fn marquee_click_suppressed(&self) -> bool {
        self.marquee.suppress_click || self.marquee.gesture.is_some()
    }
    pub(super) fn marquee_overlay(&self, cx: &App) -> AnyElement {
        let Some(gesture) = &self.marquee.gesture else {
            return div().into_any_element();
        };
        let origin = gesture.origin;
        let pointer = gesture.pointer;
        let body = self.marquee.body.clone();
        let viewport = self.marquee.viewport.clone();
        let scroll = self.scroll.clone();
        let border = cx.theme().ring;
        let mut fill = cx.theme().list_active;
        fill.a = 0.18;
        gpui_kit::canvas(
            move |_, _, _| {
                let body = body.get();
                let viewport = viewport.get();
                let offset = scroll.0.borrow().base_handle.offset();
                let start = point(body.left() + origin.x, body.top() + origin.y + offset.y);
                let left = start.x.min(pointer.x).max(viewport.left());
                let right = start.x.max(pointer.x).min(viewport.right() - px(12.));
                let top = start.y.min(pointer.y).max(body.top());
                let bottom = start.y.max(pointer.y).min(body.bottom() - px(12.));
                Bounds::new(
                    point(left, top),
                    size((right - left).max(px(0.)), (bottom - top).max(px(0.))),
                )
            },
            move |_, bounds, window, _| {
                if bounds.size.width > px(0.) && bounds.size.height > px(0.) {
                    window.paint_quad(gpui_kit::fill(bounds, fill));
                    window.paint_quad(gpui_kit::outline(
                        bounds,
                        border,
                        gpui_kit::BorderStyle::Solid,
                    ));
                }
            },
        )
        .absolute()
        .size_full()
        .into_any_element()
    }
}
