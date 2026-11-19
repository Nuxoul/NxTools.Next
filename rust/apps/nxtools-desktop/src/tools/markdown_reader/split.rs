//! Proportional source/preview layout using the official resize handle.
use super::*;
use gpui_kit::base::InteractiveElementExt as _;
use gpui_kit::component::resizable::resize_handle;
use gpui_kit::{AnyElement, Axis, DragMoveEvent, MouseButton, MouseDownEvent, Pixels, Point, Size};
use std::cell::Cell;

const MIN_PANE: f32 = 240.;
const DIVIDER: f32 = 1.;

#[derive(Default)]
pub(super) struct SplitLayout {
    share: Option<f32>,
    pressed_x: Rc<Cell<Pixels>>,
    gesture: Option<(Uuid, Instant)>,
}

#[derive(Clone)]
struct ResizeSplit {
    tab: usize,
    start: Rc<Cell<Pixels>>,
    left: Pixels,
    width: Pixels,
}

impl Render for ResizeSplit {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl SplitLayout {
    pub(super) fn widths(&self, width: Pixels) -> Option<(Pixels, Pixels)> {
        let available = width - px(DIVIDER);
        if available < px(MIN_PANE * 2.) {
            return None;
        }
        let left = px((available.as_f32() * self.share.unwrap_or(0.5)).round())
            .clamp(px(MIN_PANE), available - px(MIN_PANE));
        Some((left, available - left))
    }

    fn finish(&mut self) {
        if let Some((correlation, started)) = self.gesture.take() {
            tracing::info!(event = "markdown.split_resize.finished", correlation_id = %correlation, elapsed_ms = started.elapsed().as_millis());
        }
    }
}

impl Reader {
    pub(super) fn source_split(
        &self,
        tab: &TabDocument,
        viewport: Size<Pixels>,
        source: AnyElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some((left, right)) = self.split.widths(viewport.width) else {
            return div().flex_1().min_h_0().child(source).into_any_element();
        };
        let id = tab.id;
        let owner = cx.weak_entity();
        let pressed_x = self.split.pressed_x.clone();
        let payload = ResizeSplit {
            tab: id,
            start: pressed_x.clone(),
            left,
            width: viewport.width,
        };
        let handle = resize_handle("markdown-split-handle", Axis::Horizontal)
            .debug_selector(|| "markdown-split-divider".into())
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, _| {
                pressed_x.set(event.position.x);
            })
            .on_drag(payload, move |drag, _: &Point<Pixels>, window, cx| {
                let drag = drag.clone();
                let owner = owner.clone();
                window.defer(cx, move |_, cx| {
                    let _ = owner.update(cx, |this, _| {
                        this.split.finish();
                        let correlation = Uuid::new_v4();
                        this.split.gesture = Some((correlation, Instant::now()));
                        tracing::info!(event = "markdown.split_resize.started", correlation_id = %correlation);
                    });
                });
                cx.new(|_| drag)
            })
            .on_double_click(cx.listener(|this, _, _, cx| {
                this.split.finish();
                let correlation = Uuid::new_v4();
                tracing::info!(event = "markdown.split_reset.started", correlation_id = %correlation);
                this.split.share = None;
                tracing::info!(event = "markdown.split_reset.finished", correlation_id = %correlation, elapsed_ms = 0);
                cx.notify();
            }));
        div()
            .id("markdown-split")
            .relative()
            .flex()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .on_drag_move(
                cx.listener(|this, event: &DragMoveEvent<ResizeSplit>, _, cx| {
                    let drag = event.drag(cx);
                    if !this
                        .active_tab()
                        .is_some_and(|tab| tab.id == drag.tab && tab.draft.is_split())
                    {
                        return;
                    }
                    let available = drag.width - px(DIVIDER);
                    let left = (drag.left + event.event.position.x - drag.start.get())
                        .clamp(px(MIN_PANE), available - px(MIN_PANE));
                    this.split.share = Some(left / available);
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.split.finish()),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.split.finish()),
            )
            .child(div().w(left).h_full().flex_none().min_h_0().child(source))
            .child(
                div()
                    .w(px(DIVIDER))
                    .h_full()
                    .flex_none()
                    .relative()
                    .child(handle),
            )
            .child(
                div()
                    .id("markdown-split-preview")
                    .debug_selector(|| "markdown-split-preview".into())
                    .relative()
                    .w(right)
                    .h_full()
                    .flex_none()
                    .flex()
                    .min_h_0()
                    .child(self.render_document(
                        tab,
                        Size {
                            width: right,
                            height: viewport.height,
                        },
                        cx,
                    )),
            )
            .into_any_element()
    }
}
