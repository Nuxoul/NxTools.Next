use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use gpui_kit::{
    Bounds, Context, IntoElement, ObjectFit, Pixels, Render, Window, canvas, div, img,
    linear_color_stop, linear_gradient, prelude::*, px, rgba,
};

use crate::ui::motion::{motion_allowed, observe_motion};

const SIZE: f32 = 88.;
// Keep the moving artwork inside the parent scroll viewport, including the raised pose.
const FRAME_SIZE: f32 = 100.;
const FOLLOW_DURATION: Duration = Duration::from_millis(190);
const RETURN_DURATION: Duration = Duration::from_millis(220);

#[derive(Default)]
pub(super) struct Emblem {
    area: Rc<Cell<Option<Bounds<Pixels>>>>,
    pose: Pose,
    transition: Option<Transition>,
}

#[derive(Clone, Copy, Default, PartialEq)]
struct Pose {
    x: f32,
    y: f32,
    lift: f32,
}

struct Transition {
    from: Pose,
    to: Pose,
    started: Instant,
    duration: Duration,
}

impl Emblem {
    fn track(&mut self, position: gpui_kit::Point<Pixels>, cx: &mut Context<Self>) {
        if !motion_allowed(cx) {
            return;
        }
        let Some(bounds) = self.area.get().filter(|bounds| bounds.contains(&position)) else {
            return;
        };
        let offset = position - bounds.center();
        let half = bounds.size.width / 2.;
        let target = Pose {
            x: (offset.x / half).clamp(-1., 1.),
            y: (offset.y / half).clamp(-1., 1.),
            lift: 1.,
        };
        self.aim(target, FOLLOW_DURATION, cx);
    }

    fn aim(&mut self, target: Pose, duration: Duration, cx: &mut Context<Self>) {
        if self
            .transition
            .as_ref()
            .map_or(self.pose, |motion| motion.to)
            != target
        {
            let now = cx.background_executor().now();
            self.advance(now);
            self.transition = (self.pose != target).then_some(Transition {
                from: self.pose,
                to: target,
                started: now,
                duration,
            });
            cx.notify();
        }
    }

    fn advance(&mut self, now: Instant) {
        let Some(motion) = &self.transition else {
            return;
        };
        let elapsed = now.saturating_duration_since(motion.started);
        if elapsed >= motion.duration {
            self.pose = motion.to;
            self.transition = None;
        } else {
            let t = elapsed.as_secs_f32() / motion.duration.as_secs_f32();
            let eased = t * (2. - t);
            let interpolate = |from, to| from + (to - from) * eased;
            self.pose = Pose {
                x: interpolate(motion.from.x, motion.to.x),
                y: interpolate(motion.from.y, motion.to.y),
                lift: interpolate(motion.from.lift, motion.to.lift),
            };
        }
    }
}

impl Render for Emblem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        observe_motion(window, cx);
        if !motion_allowed(cx) {
            self.pose = Pose::default();
            self.transition = None;
        } else {
            self.advance(cx.background_executor().now());
            if self.transition.is_some() {
                window.request_animation_frame();
            }
        }
        let area = self.area.clone();
        div()
            .id("about-emblem")
            .debug_selector(|| "about-emblem".into())
            .relative()
            .size(px(FRAME_SIZE))
            .flex_none()
            .on_mouse_move(
                cx.listener(|view, event: &gpui_kit::MouseMoveEvent, _, cx| {
                    if event.pressed_button.is_none() {
                        view.track(event.position, cx);
                    }
                }),
            )
            .on_hover(cx.listener(|view, inside: &bool, _, cx| {
                if !inside {
                    view.aim(Pose::default(), RETURN_DURATION, cx);
                }
            }))
            .child(
                canvas(move |bounds, _, _| area.set(Some(bounds)), |_, _, _, _| {})
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
            )
            .child(layers(self.pose))
    }
}

fn layers(Pose { x, y, lift }: Pose) -> impl IntoElement {
    // Different depth planes move at different speeds; the hit target never moves.
    let width = SIZE + 4. * lift - 7. * x.abs();
    let height = SIZE + 4. * lift - 7. * y.abs();
    let left = (SIZE - width) / 2. + 4. * x;
    let top = (SIZE - height) / 2. + 4. * y - 3. * lift;
    let face_w = width * 56. / SIZE;
    let face_h = height * 56. / SIZE;
    div()
        .absolute()
        .top(px(7.))
        .left(px((FRAME_SIZE - SIZE) / 2.))
        .size(px(SIZE))
        .child(
            div()
                .absolute()
                .left(px(16. - 5. * x))
                .top(px(76. - 2. * y))
                .w(px(56.))
                .h(px(5.))
                .rounded_full()
                .bg(rgba(0x284B9C38))
                .opacity(lift),
        )
        .child(
            div()
                .debug_selector(|| "about-emblem-plate".into())
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(width))
                .h(px(height))
                .child(
                    img("brand/about-emblem.svg")
                        .size_full()
                        .object_fit(ObjectFit::Fill),
                ),
        )
        .child(
            div()
                .absolute()
                .left(px(left + width * 10. / SIZE))
                .top(px(top + height * 6. / SIZE))
                .w(px(width * 68. / SIZE))
                .h(px(height * 68. / SIZE))
                .rounded(px(17.))
                .opacity(lift)
                .bg(linear_gradient(
                    140. + 42. * x + 27. * y,
                    linear_color_stop(rgba(0xB4D9FF62), 0.),
                    linear_color_stop(rgba(0x60A5FA00), 0.74),
                )),
        )
        .child(
            div()
                .debug_selector(|| "about-emblem-face".into())
                .absolute()
                .left(px(left + (width - face_w) / 2. + 4. * x))
                .top(px(top + height * 13. / SIZE + 4. * y))
                .w(px(face_w))
                .h(px(face_h))
                .child(
                    img("brand/nxtools-128.png")
                        .size_full()
                        .object_fit(ObjectFit::Fill),
                ),
        )
}
