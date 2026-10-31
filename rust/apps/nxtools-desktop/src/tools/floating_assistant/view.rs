use super::{FloatingAssistantApplication, media::DecodedMedia, playback::Playback};
use gpui_kit::component::progress::ProgressCircle;
use gpui_kit::{
    Context, IntoElement, ObjectFit, Render, RenderImage, Task, WeakEntity, Window, div, img,
    prelude::*, px,
};
use std::{sync::Arc, time::Instant};
use uuid::Uuid;

pub(super) struct FloatingView {
    pub(super) expected_frame: Option<(Uuid, u32, u32)>,
    images: Vec<Arc<RenderImage>>,
    playback: Playback,
    visible: bool,
    sync_speed: bool,
    downloading: usize,
    bytes_per_second: u64,
    progress: Option<f32>,
    active_count: usize,
    timer: Option<Task<()>>,
    owner: WeakEntity<FloatingAssistantApplication>,
}
impl FloatingView {
    pub(super) fn new(owner: WeakEntity<FloatingAssistantApplication>) -> Self {
        Self {
            expected_frame: None,
            images: Vec::new(),
            playback: Playback::new(&[], 24, Instant::now()),
            visible: false,
            sync_speed: false,
            downloading: 0,
            bytes_per_second: 0,
            progress: None,
            active_count: 0,
            timer: None,
            owner,
        }
    }
    pub(super) fn replace(
        &mut self,
        media: Option<DecodedMedia>,
        fps: u32,
        cx: &mut Context<Self>,
    ) {
        self.timer = None;
        self.images.clear();
        let mut durations = Vec::new();
        if let Some(media) = media {
            for frame in media.frames {
                durations.push(frame.duration);
                let mut bgra = frame.rgba;
                for pixel in bgra.chunks_exact_mut(4) {
                    pixel.swap(0, 2);
                }
                if let Some(buffer) = image::RgbaImage::from_raw(frame.width, frame.height, bgra) {
                    self.images
                        .push(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])));
                }
            }
        }
        self.playback.reset(&durations, fps, Instant::now());
        self.schedule(cx);
        cx.notify();
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn activity(
        &mut self,
        visible: bool,
        fps: u32,
        sync: bool,
        count: usize,
        speed: u64,
        progress: Option<f32>,
        active_count: usize,
        cx: &mut Context<Self>,
    ) {
        self.visible = visible;
        self.sync_speed = sync;
        self.downloading = count;
        self.bytes_per_second = speed;
        self.progress = progress;
        self.active_count = active_count;
        self.playback.set_fps(fps, Instant::now());
        self.schedule(cx);
        cx.notify();
    }
    fn schedule(&mut self, cx: &mut Context<Self>) {
        self.timer = None;
        let now = Instant::now();
        self.playback.set_activity(
            true,
            self.visible,
            crate::ui::motion::motion_allowed(cx),
            now,
        );
        self.playback.set_download_speed(
            self.sync_speed,
            self.downloading,
            self.bytes_per_second,
            now,
        );
        let Some(deadline) = self.playback.next_wake(now) else {
            return;
        };
        self.timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(deadline.saturating_duration_since(Instant::now()))
                .await;
            let _ = this.update(cx, |view, cx| {
                view.timer = None;
                if view.playback.poll(Instant::now()).is_some() {
                    cx.notify();
                }
                view.schedule(cx);
            });
        }));
    }
    #[cfg(test)]
    pub(super) fn timer_active(&self) -> bool {
        self.timer.is_some()
    }
    #[cfg(test)]
    pub(super) fn image_count(&self) -> usize {
        self.images.len()
    }
}
impl Render for FloatingView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::theme::observe_theme(window, cx);
        crate::ui::motion::observe_motion(window, cx);
        if crate::ui::motion::reduced_motion(cx) && self.timer.is_some() {
            self.timer = None;
            self.playback
                .set_activity(true, self.visible, false, Instant::now());
        }
        if let Some((id, width, height)) = self.expected_frame {
            let viewport = window.viewport_size();
            let scale = window.scale_factor();
            if (viewport.width.as_f32() * scale - width as f32).abs() <= 1.0
                && (viewport.height.as_f32() * scale - height as f32).abs() <= 1.0
            {
                self.expected_frame = None;
                let owner = self.owner.clone();
                cx.defer(move |cx| {
                    let _ = owner.update(cx, |owner, cx| owner.frame_ready(id, cx));
                });
            }
        }
        let theme = crate::theme::Theme::active(cx);
        let side = window
            .viewport_size()
            .width
            .min(window.viewport_size().height);
        let mut ball = div()
            .debug_selector(|| "floating-assistant-ball".into())
            .size_full()
            .rounded_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center();
        if let Some(image) = self.images.get(self.playback.displayed()) {
            ball = ball.child(
                img(image.clone())
                    .size_full()
                    .object_fit(ObjectFit::Contain),
            );
        } else {
            ball = ball.bg(theme.surface).text_color(theme.text).child(
                div()
                    .text_size(px((side.as_f32() * 0.28).clamp(10.0, 48.0)))
                    .child("NX"),
            );
        }
        div()
            .debug_selector(|| "floating-assistant-renderer".into())
            .size_full()
            .relative()
            .child(ball)
            .children(self.progress.filter(|_| self.visible).map(|value| {
                div()
                    .debug_selector(|| "floating-download-ring".into())
                    .size(side)
                    .absolute()
                    .top_0()
                    .left_0()
                    .child(
                        ProgressCircle::new("floating-download-progress")
                            .value(value)
                            .size_full()
                            .accessibility_label(format!("{} 个活动下载", self.active_count)),
                    )
            }))
            .when(self.active_count > 0 && self.visible, |this| {
                this.child(
                    div()
                        .debug_selector(|| "floating-download-count".into())
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom(px(side.as_f32() * 0.12))
                        .flex()
                        .justify_center()
                        .child(
                            div()
                                .rounded_full()
                                .px(px(2.0))
                                .bg(theme.surface)
                                .text_color(theme.text)
                                .text_size(px((side.as_f32() * 0.18).clamp(6.0, 12.0)))
                                .child(if self.active_count > 99 {
                                    "99+".into()
                                } else {
                                    self.active_count.to_string()
                                }),
                        ),
                )
            })
    }
}
