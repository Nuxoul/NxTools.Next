use gpui::{
    Animation, AnimationExt, Context, Entity, IntoElement, Render, Subscription, Window, div,
    ease_in_out, prelude::*, px, transparent_black,
};
use gpui_component::PixelsExt;
use nxtools_core::{DownloadTaskbarProgressMode, DownloadTaskbarProgressOverlaySettings};
use nxtools_download::DownloadStatus;
use nxtools_ipc::PlatformBridge;
use nxtools_protocol::{
    BeginWindowMoveCommand, BeginWindowResizeCommand, PlatformCommand,
    SetTaskbarProgressOverlayVisibilityCommand,
};
use std::{rc::Rc, time::Duration};
use tracing::{info, warn};

use crate::downloads::DownloadApplication;
use crate::theme::Theme;
use crate::ui::metrics::{PROGRESS_TRACK_HEIGHT, RADIUS_CONTROL, SPACING_4, SPACING_8};
use crate::ui::typography::{LINE_HEIGHT_LABEL, TEXT_LABEL, UI_FONT_FAMILY, UI_MONO_FONT_FAMILY};

const TASKBAR_OVERLAY_MAX_LANES: usize = 8;
const TASKBAR_OVERLAY_LANE_HEIGHT: f32 = 2.0;
const TASKBAR_OVERLAY_LANE_GAP: f32 = 1.0;
const TASKBAR_OVERLAY_ENTER_DURATION: Duration = Duration::from_millis(90);
const TASKBAR_OVERLAY_EXIT_DURATION: Duration = Duration::from_millis(120);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TaskbarOverlayVisibilityTransition {
    generation: u64,
    exiting: bool,
}

impl TaskbarOverlayVisibilityTransition {
    fn new(initially_visible: bool) -> Self {
        Self {
            generation: u64::from(initially_visible),
            exiting: false,
        }
    }

    fn begin_enter(&mut self) {
        self.generation = self.generation.wrapping_add(1).max(1);
        self.exiting = false;
    }

    fn begin_exit(&mut self) -> Option<u64> {
        if self.exiting {
            return None;
        }
        self.generation = self.generation.wrapping_add(1).max(1);
        self.exiting = true;
        Some(self.generation)
    }

    fn complete_exit(&mut self, generation: u64) -> bool {
        if !self.exiting || self.generation != generation {
            return false;
        }
        self.exiting = false;
        true
    }
}

fn taskbar_overlay_visibility_motion(delta: f32, exiting: bool) -> (f32, f32) {
    if exiting {
        (1.0 - delta, 2.0 * delta)
    } else {
        (0.45 + 0.55 * delta, 1.5 * (1.0 - delta))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DownloadTaskbarOverlaySample {
    pub status: DownloadStatus,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub bytes_per_second: u64,
    pub active_connections: u16,
    pub segments: Vec<DownloadTaskbarOverlaySegmentSample>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DownloadTaskbarOverlaySegmentSample {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DownloadTaskbarOverlayLane {
    Indeterminate,
    Determinate { completed: u64, total: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DownloadTaskbarOverlayModel {
    Hidden,
    Visible {
        lanes: Vec<DownloadTaskbarOverlayLane>,
        metrics: DownloadTaskbarOverlayMetrics,
        settings: DownloadTaskbarProgressOverlaySettings,
        editing: bool,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DownloadTaskbarOverlayMetrics {
    pub active_tasks: usize,
    pub active_connections: u32,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub bytes_per_second: u64,
}

impl DownloadTaskbarOverlayModel {
    pub(crate) fn is_visible(&self) -> bool {
        !matches!(self, Self::Hidden)
    }

    #[cfg(test)]
    fn visible_lane_count(&self) -> usize {
        match self {
            Self::Hidden => 0,
            Self::Visible { lanes, .. } => lanes.len(),
        }
    }
}

fn taskbar_progress_lane<'a>(
    samples: impl IntoIterator<Item = &'a DownloadTaskbarOverlaySample>,
) -> DownloadTaskbarOverlayLane {
    let mut completed = 0_u64;
    let mut total = 0_u64;
    for sample in samples {
        let Some(sample_total) = sample.total_bytes.filter(|total| *total > 0) else {
            return DownloadTaskbarOverlayLane::Indeterminate;
        };
        completed = completed.saturating_add(sample.downloaded_bytes.min(sample_total));
        total = total.saturating_add(sample_total);
    }
    if total == 0 {
        DownloadTaskbarOverlayLane::Indeterminate
    } else {
        DownloadTaskbarOverlayLane::Determinate { completed, total }
    }
}

fn taskbar_segment_progress_lane<'a>(
    segments: impl IntoIterator<Item = &'a DownloadTaskbarOverlaySegmentSample>,
) -> DownloadTaskbarOverlayLane {
    let mut completed = 0_u64;
    let mut total = 0_u64;
    for segment in segments {
        let Some(segment_total) = segment.total_bytes.filter(|total| *total > 0) else {
            return DownloadTaskbarOverlayLane::Indeterminate;
        };
        completed = completed.saturating_add(segment.downloaded_bytes.min(segment_total));
        total = total.saturating_add(segment_total);
    }
    if total == 0 {
        DownloadTaskbarOverlayLane::Indeterminate
    } else {
        DownloadTaskbarOverlayLane::Determinate { completed, total }
    }
}

fn taskbar_overlay_max_lanes(settings: &DownloadTaskbarProgressOverlaySettings) -> usize {
    let show_text = settings.show_speed
        || settings.show_remaining_time
        || settings.show_active_tasks
        || settings.show_active_connections
        || settings.show_transferred_bytes;
    let text_space = if show_text {
        LINE_HEIGHT_LABEL as u32
            + if settings.show_progress {
                SPACING_4 as u32
            } else {
                0
            }
    } else {
        0
    };
    let available = settings.height.saturating_sub(text_space);
    let lane_stride = (TASKBAR_OVERLAY_LANE_HEIGHT + TASKBAR_OVERLAY_LANE_GAP) as u32;
    let available_with_trailing_gap = available.saturating_add(TASKBAR_OVERLAY_LANE_GAP as u32);
    usize::try_from(
        (available_with_trailing_gap / lane_stride).clamp(1, TASKBAR_OVERLAY_MAX_LANES as u32),
    )
    .unwrap_or(TASKBAR_OVERLAY_MAX_LANES)
}

fn taskbar_overlay_edit_samples() -> Vec<DownloadTaskbarOverlaySample> {
    vec![
        DownloadTaskbarOverlaySample {
            status: DownloadStatus::Downloading,
            downloaded_bytes: 314 * 1024 * 1024,
            total_bytes: Some(1024 * 1024 * 1024),
            bytes_per_second: 10 * 1024 * 1024,
            active_connections: 4,
            segments: vec![
                DownloadTaskbarOverlaySegmentSample {
                    downloaded_bytes: 96,
                    total_bytes: Some(256),
                },
                DownloadTaskbarOverlaySegmentSample {
                    downloaded_bytes: 82,
                    total_bytes: Some(256),
                },
                DownloadTaskbarOverlaySegmentSample {
                    downloaded_bytes: 74,
                    total_bytes: Some(256),
                },
                DownloadTaskbarOverlaySegmentSample {
                    downloaded_bytes: 62,
                    total_bytes: Some(256),
                },
            ],
        },
        DownloadTaskbarOverlaySample {
            status: DownloadStatus::Downloading,
            downloaded_bytes: 420 * 1024 * 1024,
            total_bytes: Some(1024 * 1024 * 1024),
            bytes_per_second: 14 * 1024 * 1024,
            active_connections: 4,
            segments: Vec::new(),
        },
    ]
}

pub(crate) fn aggregate_taskbar_progress(
    settings: &DownloadTaskbarProgressOverlaySettings,
    editing: bool,
    samples: impl IntoIterator<Item = DownloadTaskbarOverlaySample>,
) -> DownloadTaskbarOverlayModel {
    let mut active = samples
        .into_iter()
        .filter(|sample| {
            matches!(
                sample.status,
                DownloadStatus::Probing
                    | DownloadStatus::Waiting
                    | DownloadStatus::Downloading
                    | DownloadStatus::Finalizing
                    | DownloadStatus::Verifying
            )
        })
        .collect::<Vec<_>>();
    if active.is_empty() && !editing {
        return DownloadTaskbarOverlayModel::Hidden;
    }
    if active.is_empty() {
        active = taskbar_overlay_edit_samples();
    }
    let mut total_bytes = Some(0_u64);
    let mut downloaded_bytes = 0_u64;
    let mut bytes_per_second = 0_u64;
    let mut active_connections = 0_u32;
    for sample in &active {
        downloaded_bytes = downloaded_bytes.saturating_add(sample.downloaded_bytes);
        bytes_per_second = bytes_per_second.saturating_add(sample.bytes_per_second);
        active_connections =
            active_connections.saturating_add(u32::from(sample.active_connections));
        total_bytes = total_bytes
            .zip(sample.total_bytes)
            .map(|(left, right)| left.saturating_add(right));
    }
    let metrics = DownloadTaskbarOverlayMetrics {
        active_tasks: active.len(),
        active_connections,
        downloaded_bytes,
        total_bytes,
        bytes_per_second,
    };
    let max_lanes = taskbar_overlay_max_lanes(settings);
    let lanes = match settings.progress_mode {
        DownloadTaskbarProgressMode::Aggregate => vec![taskbar_progress_lane(active.iter())],
        DownloadTaskbarProgressMode::PerTask => {
            let split_at = active.len().min(max_lanes.saturating_sub(1));
            let mut lanes = active[..split_at]
                .iter()
                .map(|sample| taskbar_progress_lane([sample]))
                .collect::<Vec<_>>();
            if split_at < active.len() {
                lanes.push(taskbar_progress_lane(active[split_at..].iter()));
            }
            lanes
        }
        DownloadTaskbarProgressMode::PerSegment => {
            let segments = active
                .first()
                .map(|sample| sample.segments.as_slice())
                .unwrap_or_default();
            let split_at = segments.len().min(max_lanes.saturating_sub(1));
            let mut lanes = segments[..split_at]
                .iter()
                .map(|segment| taskbar_segment_progress_lane([segment]))
                .collect::<Vec<_>>();
            if split_at < segments.len() {
                lanes.push(taskbar_segment_progress_lane(segments[split_at..].iter()));
            }
            if lanes.is_empty() {
                lanes.push(taskbar_progress_lane(active.iter()));
            }
            lanes
        }
    };
    DownloadTaskbarOverlayModel::Visible {
        lanes,
        metrics,
        settings: settings.clone(),
        editing,
    }
}

pub(crate) struct DownloadTaskbarOverlayView {
    theme: Theme,
    model: DownloadTaskbarOverlayModel,
    application: Entity<DownloadApplication>,
    platform: Rc<PlatformBridge>,
    native_window_handle: i64,
    _subscription: Subscription,
    _bounds_subscription: Subscription,
    edit_reference: Option<TaskbarEditGeometryReference>,
    edit_gesture_active: bool,
    visibility: TaskbarOverlayVisibilityTransition,
}

#[derive(Clone, Copy)]
struct TaskbarEditGeometryReference {
    anchor: nxtools_core::DownloadTaskbarOverlayAnchor,
    taskbar_edge_x: f32,
    taskbar_top: f32,
    correlation_id: uuid::Uuid,
}

impl DownloadTaskbarOverlayView {
    pub(crate) fn new(
        application: Entity<DownloadApplication>,
        platform: Rc<PlatformBridge>,
        native_window_handle: i64,
        initial_model: DownloadTaskbarOverlayModel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let observed_application = application.clone();
        let observed_view = cx.entity();
        let subscription = cx.observe(&application, move |_, _, cx| {
            let application = observed_application.clone();
            let view = observed_view.clone();
            cx.defer(move |cx| {
                let (next, correlation_id) = {
                    let application = application.read(cx);
                    (
                        application.taskbar_overlay_model(),
                        application.taskbar_overlay_diagnostic_correlation_id(),
                    )
                };
                view.update(cx, |this, cx| {
                    this.apply_model(next, correlation_id, cx);
                });
            });
        });
        let bounds_application = application.clone();
        let bounds_subscription = cx.observe_window_bounds(window, move |this, window, cx| {
            let DownloadTaskbarOverlayModel::Visible {
                settings,
                editing: true,
                ..
            } = &this.model
            else {
                this.edit_reference = None;
                this.edit_gesture_active = false;
                return;
            };
            if !this.edit_gesture_active
                || !bounds_application
                    .read(cx)
                    .taskbar_overlay_edit_gesture_active()
            {
                this.edit_reference = None;
                this.edit_gesture_active = false;
                return;
            }
            let bounds = window.bounds();
            let x = bounds.origin.x.as_f32();
            let y = bounds.origin.y.as_f32();
            let width = bounds.size.width.as_f32();
            let height = bounds.size.height.as_f32();
            if this
                .edit_reference
                .is_some_and(|reference| reference.anchor != settings.anchor)
            {
                this.edit_reference = None;
                return;
            }
            let reference =
                *this
                    .edit_reference
                    .get_or_insert_with(|| TaskbarEditGeometryReference {
                        anchor: settings.anchor,
                        taskbar_edge_x: match settings.anchor {
                            nxtools_core::DownloadTaskbarOverlayAnchor::Left => {
                                x - settings.offset_x as f32
                            }
                            nxtools_core::DownloadTaskbarOverlayAnchor::Right => {
                                x + width + settings.offset_x as f32
                            }
                        },
                        taskbar_top: y - settings.offset_y as f32,
                        correlation_id: uuid::Uuid::new_v4(),
                    });
            let mut next = settings.clone();
            next.width = width.round().max(0.0) as u32;
            next.height = height.round().max(0.0) as u32;
            next.offset_x = match settings.anchor {
                nxtools_core::DownloadTaskbarOverlayAnchor::Left => {
                    (x - reference.taskbar_edge_x).round() as i32
                }
                nxtools_core::DownloadTaskbarOverlayAnchor::Right => {
                    (reference.taskbar_edge_x - x - width).round() as i32
                }
            };
            next.offset_y = (y - reference.taskbar_top).round() as i32;
            if &next == settings {
                return;
            }
            let application = bounds_application.clone();
            cx.defer(move |cx| {
                application.update(cx, |application, cx| {
                    application.record_taskbar_overlay_edit_bounds(
                        next,
                        reference.correlation_id,
                        cx,
                    );
                });
            });
        });
        let visibility = TaskbarOverlayVisibilityTransition::new(initial_model.is_visible());
        Self {
            theme: Theme::zed_dark(),
            model: initial_model,
            application,
            platform,
            native_window_handle,
            _subscription: subscription,
            _bounds_subscription: bounds_subscription,
            edit_reference: None,
            edit_gesture_active: false,
            visibility,
        }
    }

    fn apply_model(
        &mut self,
        next: DownloadTaskbarOverlayModel,
        correlation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        if self.model == next && !(next.is_visible() && self.visibility.exiting) {
            return;
        }
        match (self.model.is_visible(), next.is_visible()) {
            (false, false) => self.model = next,
            (false, true) => {
                self.send_native_visibility(true, correlation_id);
                self.model = next;
                self.visibility.begin_enter();
                cx.notify();
            }
            (true, true) => {
                self.model = next;
                if self.visibility.exiting {
                    self.visibility.begin_enter();
                }
                cx.notify();
            }
            (true, false) => {
                let Some(generation) = self.visibility.begin_exit() else {
                    return;
                };
                info!(
                    event = "download.taskbar_overlay.visibility_exit_started",
                    %correlation_id,
                    generation,
                    "taskbar overlay exit transition started"
                );
                cx.notify();
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(TASKBAR_OVERLAY_EXIT_DURATION)
                        .await;
                    let _ = this.update(cx, |this, cx| {
                        this.complete_visibility_exit(generation, correlation_id, cx);
                    });
                })
                .detach();
            }
        }
    }

    fn complete_visibility_exit(
        &mut self,
        generation: u64,
        correlation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        if !self.visibility.complete_exit(generation) {
            return;
        }
        self.model = DownloadTaskbarOverlayModel::Hidden;
        cx.notify();
        self.send_native_visibility(false, correlation_id);
    }

    fn send_native_visibility(&self, visible: bool, correlation_id: uuid::Uuid) {
        if self.native_window_handle == 0 {
            return;
        }
        match self.platform.send_command_correlated(
            PlatformCommand::SetTaskbarProgressOverlayVisibility(
                SetTaskbarProgressOverlayVisibilityCommand {
                    window_handle: self.native_window_handle,
                    visible,
                },
            ),
            correlation_id,
        ) {
            Ok(_) => info!(
                event = "download.taskbar_overlay.visibility_accepted",
                %correlation_id,
                visible,
                "taskbar overlay visibility change accepted"
            ),
            Err(error) => warn!(
                event = "download.taskbar_overlay.visibility_accept_failed",
                %correlation_id,
                %error,
                visible,
                "taskbar overlay visibility change could not be queued"
            ),
        }
    }

    fn begin_edit_gesture(&mut self, window: &Window, correlation_id: uuid::Uuid) {
        if let DownloadTaskbarOverlayModel::Visible {
            settings,
            editing: true,
            ..
        } = &self.model
        {
            let bounds = window.bounds();
            let x = bounds.origin.x.as_f32();
            let y = bounds.origin.y.as_f32();
            let width = bounds.size.width.as_f32();
            self.edit_reference = Some(TaskbarEditGeometryReference {
                anchor: settings.anchor,
                taskbar_edge_x: match settings.anchor {
                    nxtools_core::DownloadTaskbarOverlayAnchor::Left => {
                        x - settings.offset_x as f32
                    }
                    nxtools_core::DownloadTaskbarOverlayAnchor::Right => {
                        x + width + settings.offset_x as f32
                    }
                },
                taskbar_top: y - settings.offset_y as f32,
                correlation_id,
            });
            self.edit_gesture_active = true;
        }
    }

    fn finish_edit_gesture(&mut self) {
        self.edit_gesture_active = false;
        self.edit_reference = None;
    }
}

impl Render for DownloadTaskbarOverlayView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::ui::motion::observe_motion(window, cx);
        let theme = self.theme;
        let visibility_generation = self.visibility.generation;
        let visibility_exiting = self.visibility.exiting;
        let mut root = div()
            .debug_selector(|| "download-taskbar-overlay-root".into())
            .size_full()
            .relative()
            .bg(transparent_black())
            .overflow_hidden();
        match &self.model {
            DownloadTaskbarOverlayModel::Hidden => root.into_any_element(),
            DownloadTaskbarOverlayModel::Visible {
                lanes,
                metrics,
                settings,
                editing,
            } => {
                let width = settings.width as f32;
                let show_text = settings.show_speed
                    || settings.show_remaining_time
                    || settings.show_active_tasks
                    || settings.show_active_connections
                    || settings.show_transferred_bytes;
                let move_platform = Rc::clone(&self.platform);
                let move_handle = self.native_window_handle;
                root = root
                    .when(*editing, |root| {
                        root.cursor_move().on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, _event, window, cx| {
                                cx.stop_propagation();
                                if move_handle == 0 {
                                    return;
                                }
                                let correlation_id = this
                                    .application
                                    .read(cx)
                                    .taskbar_overlay_diagnostic_correlation_id();
                                this.begin_edit_gesture(window, correlation_id);
                                match move_platform.send_command_correlated(
                                    PlatformCommand::BeginWindowMove(BeginWindowMoveCommand {
                                        window_handle: move_handle,
                                    }),
                                    correlation_id,
                                ) {
                                    Ok(command_id) => {
                                        let application = this.application.clone();
                                        cx.defer(move |cx| {
                                            application.update(cx, |application, _| {
                                                application
                                                    .set_taskbar_overlay_edit_gesture_active(true);
                                                application.register_taskbar_overlay_edit_gesture(
                                                    command_id,
                                                );
                                            });
                                        });
                                        info!(
                                            event = "download.taskbar_overlay.move_accepted",
                                            %correlation_id,
                                            %command_id,
                                            "taskbar overlay move gesture accepted"
                                        );
                                    }
                                    Err(error) => {
                                        this.finish_edit_gesture();
                                        let application = this.application.clone();
                                        cx.defer(move |cx| {
                                            application.update(cx, |application, _| {
                                                application
                                                    .set_taskbar_overlay_edit_gesture_active(false);
                                            });
                                        });
                                        warn!(
                                            event = "download.taskbar_overlay.move_accept_failed",
                                            %correlation_id,
                                            %error,
                                            "taskbar overlay move gesture could not be queued"
                                        );
                                    }
                                }
                            }),
                        )
                    })
                    .child(
                        div()
                            .debug_selector(|| "download-taskbar-overlay-content".into())
                            .size_full()
                            .flex()
                            .flex_col()
                            .justify_center()
                            .gap(px(if show_text && settings.show_progress {
                                SPACING_4
                            } else {
                                0.0
                            }))
                            .when(show_text, |content| {
                                content.child(taskbar_metrics_element(theme, *metrics, settings))
                            })
                            .when(settings.show_progress, |content| {
                                content.child(
                                    div()
                                        .debug_selector(|| {
                                            "download-taskbar-overlay-progress".into()
                                        })
                                        .w_full()
                                        .flex()
                                        .flex_col()
                                        .gap(px(TASKBAR_OVERLAY_LANE_GAP))
                                        .children(lanes.iter().copied().enumerate().map(
                                            |(index, lane)| {
                                                taskbar_progress_lane_element(
                                                    theme,
                                                    width,
                                                    index,
                                                    lane,
                                                    crate::ui::motion::motion_allowed(cx),
                                                )
                                            },
                                        )),
                                )
                            }),
                    );
                if *editing {
                    let resize_platform = Rc::clone(&self.platform);
                    let resize_handle = self.native_window_handle;
                    let resize_listener = cx.listener(move |this, _event, window, cx| {
                        cx.stop_propagation();
                        if resize_handle == 0 {
                            return;
                        }
                        let correlation_id = this
                            .application
                            .read(cx)
                            .taskbar_overlay_diagnostic_correlation_id();
                        this.begin_edit_gesture(window, correlation_id);
                        match resize_platform.send_command_correlated(
                            PlatformCommand::BeginWindowResize(BeginWindowResizeCommand {
                                window_handle: resize_handle,
                            }),
                            correlation_id,
                        ) {
                            Ok(command_id) => {
                                let application = this.application.clone();
                                cx.defer(move |cx| {
                                    application.update(cx, |application, _| {
                                        application.set_taskbar_overlay_edit_gesture_active(true);
                                        application
                                            .register_taskbar_overlay_edit_gesture(command_id);
                                    });
                                });
                                info!(
                                    event = "download.taskbar_overlay.resize_accepted",
                                    %correlation_id,
                                    %command_id,
                                    "taskbar overlay resize gesture accepted"
                                );
                            }
                            Err(error) => {
                                this.finish_edit_gesture();
                                let application = this.application.clone();
                                cx.defer(move |cx| {
                                    application.update(cx, |application, _| {
                                        application.set_taskbar_overlay_edit_gesture_active(false);
                                    });
                                });
                                warn!(
                                    event = "download.taskbar_overlay.resize_accept_failed",
                                    %correlation_id,
                                    %error,
                                    "taskbar overlay resize gesture could not be queued"
                                );
                            }
                        }
                    });
                    root = root
                        .child(
                            div()
                                .debug_selector(|| "download-taskbar-overlay-edit-border".into())
                                .absolute()
                                .size_full()
                                .border_1()
                                .border_color(theme.accent)
                                .rounded(px(RADIUS_CONTROL)),
                        )
                        .child(
                            div()
                                .debug_selector(|| "download-taskbar-overlay-resize-handle".into())
                                .absolute()
                                .right_0()
                                .bottom_0()
                                .w(px(12.0))
                                .h(px(12.0))
                                .cursor_e_resize()
                                .on_mouse_down(gpui::MouseButton::Left, resize_listener)
                                .child(
                                    div()
                                        .absolute()
                                        .right(px(2.0))
                                        .bottom(px(2.0))
                                        .w(px(5.0))
                                        .h(px(PROGRESS_TRACK_HEIGHT))
                                        .border_r_1()
                                        .border_b_1()
                                        .border_color(theme.text_muted),
                                ),
                        );
                }
                if crate::ui::motion::motion_allowed(cx) {
                    root.with_animation(
                    gpui::SharedString::from(format!(
                        "download-taskbar-visibility-{visibility_generation}-{visibility_exiting}"
                    )),
                    Animation::new(if visibility_exiting {
                        TASKBAR_OVERLAY_EXIT_DURATION
                    } else {
                        TASKBAR_OVERLAY_ENTER_DURATION
                    })
                    .with_easing(ease_in_out),
                    move |root, delta| {
                        let (opacity, vertical_offset) =
                            taskbar_overlay_visibility_motion(delta, visibility_exiting);
                        root.opacity(opacity).relative().top(px(vertical_offset))
                    },
                ).into_any_element()
                } else {
                    root.into_any_element()
                }
            }
        }
    }
}

fn taskbar_progress_lane_element(
    theme: Theme,
    width: f32,
    index: usize,
    lane: DownloadTaskbarOverlayLane,
    motion_allowed: bool,
) -> gpui::AnyElement {
    let track = div()
        .relative()
        .w_full()
        .h(px(TASKBAR_OVERLAY_LANE_HEIGHT))
        .overflow_hidden()
        .bg(theme.border_subtle);
    match lane {
        DownloadTaskbarOverlayLane::Indeterminate => {
            let segment = div()
                .absolute()
                .top_0()
                .left_0()
                .h_full()
                .w(px(width * 0.35))
                .bg(theme.accent)
                .opacity(0.78);
            let segment = if motion_allowed {
                segment
                    .with_animation(
                        ("download-taskbar-indeterminate", index),
                        Animation::new(Duration::from_millis(1_400)).repeat(),
                        move |segment, delta| segment.left(px(width * 0.65 * delta)),
                    )
                    .into_any_element()
            } else {
                segment.into_any_element()
            };
            track.child(segment).into_any_element()
        }
        DownloadTaskbarOverlayLane::Determinate { completed, total } => {
            let ratio = if total == 0 {
                0.0
            } else {
                completed as f32 / total as f32
            }
            .clamp(0.0, 1.0);
            track
                .child(div().h_full().w(px(width * ratio)).bg(theme.accent))
                .into_any_element()
        }
    }
}

fn taskbar_metrics_element(
    theme: Theme,
    metrics: DownloadTaskbarOverlayMetrics,
    settings: &DownloadTaskbarProgressOverlaySettings,
) -> gpui::AnyElement {
    let eta = metrics
        .total_bytes
        .filter(|total| metrics.bytes_per_second > 0 && *total > metrics.downloaded_bytes)
        .map(|total| {
            format_compact_duration(
                total.saturating_sub(metrics.downloaded_bytes) / metrics.bytes_per_second,
            )
        })
        .unwrap_or_else(|| "—".to_owned());
    div()
        .debug_selector(|| "download-taskbar-overlay-metrics".into())
        .w_full()
        .min_w_0()
        .h(px(LINE_HEIGHT_LABEL))
        .flex()
        .items_center()
        .gap(px(SPACING_8))
        .overflow_hidden()
        .whitespace_nowrap()
        .font_family(UI_FONT_FAMILY)
        .text_size(px(TEXT_LABEL))
        .line_height(px(LINE_HEIGHT_LABEL))
        .text_color(theme.text_secondary)
        .when(settings.show_speed, |row| {
            row.child(taskbar_metric_item(
                "download-taskbar-overlay-speed",
                None,
                format!("{}/s", format_compact_bytes(metrics.bytes_per_second)),
                None,
            ))
        })
        .when(settings.show_remaining_time, |row| {
            row.child(taskbar_metric_item(
                "download-taskbar-overlay-remaining",
                Some("剩"),
                eta,
                None,
            ))
        })
        .when(settings.show_active_tasks, |row| {
            row.child(taskbar_metric_item(
                "download-taskbar-overlay-active-tasks",
                None,
                metrics.active_tasks.to_string(),
                Some("任务"),
            ))
        })
        .when(settings.show_active_connections, |row| {
            row.child(taskbar_metric_item(
                "download-taskbar-overlay-active-connections",
                None,
                metrics.active_connections.to_string(),
                Some("连接"),
            ))
        })
        .when(settings.show_transferred_bytes, |row| {
            row.child(taskbar_metric_item(
                "download-taskbar-overlay-transferred",
                None,
                match metrics.total_bytes {
                    Some(total) => format!(
                        "{} / {}",
                        format_compact_bytes(metrics.downloaded_bytes),
                        format_compact_bytes(total)
                    ),
                    None => format_compact_bytes(metrics.downloaded_bytes),
                },
                None,
            ))
        })
        .into_any_element()
}

fn taskbar_metric_item(
    selector: &'static str,
    leading_label: Option<&'static str>,
    value: String,
    trailing_label: Option<&'static str>,
) -> gpui::AnyElement {
    div()
        .debug_selector(move || selector.into())
        .flex_none()
        .flex()
        .items_center()
        .gap(px(SPACING_4))
        .when_some(leading_label, |item, label| item.child(label))
        .child(div().font_family(UI_MONO_FONT_FAMILY).child(value))
        .when_some(trailing_label, |item, label| item.child(label))
        .into_any_element()
}

fn format_compact_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.1}G", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.1}M", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.0}K", bytes / KIB)
    } else {
        format!("{bytes:.0}B")
    }
}

fn format_compact_duration(seconds: u64) -> String {
    if seconds >= 3600 {
        format!("{}h{}m", seconds / 3600, seconds % 3600 / 60)
    } else if seconds >= 60 {
        format!("{}m{}s", seconds / 60, seconds % 60)
    } else {
        format!("{seconds}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> DownloadTaskbarProgressOverlaySettings {
        DownloadTaskbarProgressOverlaySettings::default()
    }

    fn sample(
        status: DownloadStatus,
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
    ) -> DownloadTaskbarOverlaySample {
        DownloadTaskbarOverlaySample {
            status,
            downloaded_bytes,
            total_bytes,
            bytes_per_second: 1_024,
            active_connections: 2,
            segments: Vec::new(),
        }
    }

    fn lanes(model: &DownloadTaskbarOverlayModel) -> &[DownloadTaskbarOverlayLane] {
        match model {
            DownloadTaskbarOverlayModel::Visible { lanes, .. } => lanes,
            DownloadTaskbarOverlayModel::Hidden => &[],
        }
    }

    #[test]
    fn active_downloads_preserve_their_individual_progress() {
        let model = aggregate_taskbar_progress(
            &settings(),
            false,
            [
                sample(DownloadStatus::Downloading, 20, Some(100)),
                sample(DownloadStatus::Waiting, 50, Some(100)),
            ],
        );

        assert_eq!(
            lanes(&model),
            &[
                DownloadTaskbarOverlayLane::Determinate {
                    completed: 20,
                    total: 100,
                },
                DownloadTaskbarOverlayLane::Determinate {
                    completed: 50,
                    total: 100,
                },
            ]
        );
    }

    #[test]
    fn aggregate_progress_hides_without_active_tasks() {
        let model = aggregate_taskbar_progress(
            &settings(),
            false,
            [
                sample(DownloadStatus::Paused, 20, Some(100)),
                sample(DownloadStatus::Completed, 100, Some(100)),
            ],
        );

        assert_eq!(model, DownloadTaskbarOverlayModel::Hidden);
    }

    #[test]
    fn aggregate_progress_is_indeterminate_when_an_active_length_is_unknown() {
        let model = aggregate_taskbar_progress(
            &settings(),
            false,
            [
                sample(DownloadStatus::Downloading, 20, Some(100)),
                sample(DownloadStatus::Probing, 0, None),
            ],
        );

        assert_eq!(
            lanes(&model),
            &[
                DownloadTaskbarOverlayLane::Determinate {
                    completed: 20,
                    total: 100,
                },
                DownloadTaskbarOverlayLane::Indeterminate,
            ]
        );
    }

    #[test]
    fn separate_active_downloads_keep_separate_taskbar_progress_lanes() {
        let model = aggregate_taskbar_progress(
            &settings(),
            false,
            [
                sample(DownloadStatus::Downloading, 25, Some(100)),
                sample(DownloadStatus::Waiting, 150, Some(300)),
            ],
        );

        assert_eq!(model.visible_lane_count(), 2);
    }

    #[test]
    fn overflow_downloads_share_the_last_weighted_lane() {
        let model = aggregate_taskbar_progress(
            &settings(),
            false,
            [
                sample(DownloadStatus::Downloading, 10, Some(100)),
                sample(DownloadStatus::Downloading, 20, Some(100)),
                sample(DownloadStatus::Downloading, 30, Some(100)),
                sample(DownloadStatus::Waiting, 150, Some(300)),
            ],
        );

        assert_eq!(
            lanes(&model),
            &[
                DownloadTaskbarOverlayLane::Determinate {
                    completed: 10,
                    total: 100,
                },
                DownloadTaskbarOverlayLane::Determinate {
                    completed: 20,
                    total: 100,
                },
                DownloadTaskbarOverlayLane::Determinate {
                    completed: 180,
                    total: 400,
                },
            ]
        );
    }

    #[test]
    fn edit_mode_stays_visible_with_deterministic_preview_data() {
        let model = aggregate_taskbar_progress(&settings(), true, []);

        assert!(model.is_visible());
        assert_eq!(model.visible_lane_count(), 2);
        assert!(matches!(
            model,
            DownloadTaskbarOverlayModel::Visible { editing: true, .. }
        ));
    }

    #[test]
    fn edit_preview_reflects_each_progress_mode_without_active_tasks() {
        let mut settings = settings();
        settings.progress_mode = DownloadTaskbarProgressMode::Aggregate;
        let aggregate = aggregate_taskbar_progress(&settings, true, []);
        settings.progress_mode = DownloadTaskbarProgressMode::PerTask;
        let per_task = aggregate_taskbar_progress(&settings, true, []);
        settings.progress_mode = DownloadTaskbarProgressMode::PerSegment;
        let per_segment = aggregate_taskbar_progress(&settings, true, []);

        assert_eq!(aggregate.visible_lane_count(), 1);
        assert_eq!(per_task.visible_lane_count(), 2);
        assert_eq!(per_segment.visible_lane_count(), 3);
        assert_ne!(lanes(&per_task), lanes(&per_segment));
    }

    #[test]
    fn aggregate_mode_uses_one_weighted_lane() {
        let mut settings = settings();
        settings.progress_mode = DownloadTaskbarProgressMode::Aggregate;
        let model = aggregate_taskbar_progress(
            &settings,
            false,
            [
                sample(DownloadStatus::Downloading, 20, Some(100)),
                sample(DownloadStatus::Downloading, 150, Some(300)),
            ],
        );

        assert_eq!(
            lanes(&model),
            &[DownloadTaskbarOverlayLane::Determinate {
                completed: 170,
                total: 400,
            }]
        );
    }

    #[test]
    fn per_segment_mode_uses_real_segment_lengths() {
        let mut settings = settings();
        settings.height = 20;
        settings.progress_mode = DownloadTaskbarProgressMode::PerSegment;
        let mut active = sample(DownloadStatus::Downloading, 30, Some(100));
        active.segments = vec![
            DownloadTaskbarOverlaySegmentSample {
                downloaded_bytes: 10,
                total_bytes: Some(25),
            },
            DownloadTaskbarOverlaySegmentSample {
                downloaded_bytes: 20,
                total_bytes: Some(75),
            },
            DownloadTaskbarOverlaySegmentSample {
                downloaded_bytes: 0,
                total_bytes: None,
            },
        ];

        let model = aggregate_taskbar_progress(&settings, false, [active]);

        assert_eq!(
            lanes(&model),
            &[
                DownloadTaskbarOverlayLane::Determinate {
                    completed: 10,
                    total: 25,
                },
                DownloadTaskbarOverlayLane::Determinate {
                    completed: 20,
                    total: 75,
                },
                DownloadTaskbarOverlayLane::Indeterminate,
            ]
        );
    }

    #[test]
    fn per_segment_mode_aggregates_overflow_instead_of_dropping_it() {
        let mut settings = settings();
        settings.progress_mode = DownloadTaskbarProgressMode::PerSegment;
        let mut active = sample(DownloadStatus::Downloading, 150, Some(500));
        active.segments = (0..5)
            .map(|index| DownloadTaskbarOverlaySegmentSample {
                downloaded_bytes: (index + 1) * 10,
                total_bytes: Some(100),
            })
            .collect();

        let model = aggregate_taskbar_progress(&settings, false, [active]);

        assert_eq!(
            lanes(&model),
            &[
                DownloadTaskbarOverlayLane::Determinate {
                    completed: 10,
                    total: 100,
                },
                DownloadTaskbarOverlayLane::Determinate {
                    completed: 20,
                    total: 100,
                },
                DownloadTaskbarOverlayLane::Determinate {
                    completed: 120,
                    total: 300,
                },
            ]
        );
    }

    #[test]
    fn readable_metrics_keep_a_stable_lane_budget_before_rendering() {
        let mut settings = settings();
        settings.height = 32;
        settings.show_speed = true;

        assert_eq!(taskbar_overlay_max_lanes(&settings), 4);
    }

    #[test]
    fn overlay_metrics_aggregate_all_active_tasks() {
        let model = aggregate_taskbar_progress(
            &settings(),
            false,
            [
                sample(DownloadStatus::Downloading, 20, Some(100)),
                sample(DownloadStatus::Waiting, 40, Some(200)),
            ],
        );

        let DownloadTaskbarOverlayModel::Visible { metrics, .. } = model else {
            panic!("active downloads should keep the overlay visible");
        };
        assert_eq!(metrics.active_tasks, 2);
        assert_eq!(metrics.active_connections, 4);
        assert_eq!(metrics.downloaded_bytes, 60);
        assert_eq!(metrics.total_bytes, Some(300));
        assert_eq!(metrics.bytes_per_second, 2_048);
    }

    #[test]
    fn visibility_motion_has_distinct_enter_and_exit_endpoints() {
        assert_eq!(taskbar_overlay_visibility_motion(0.0, false), (0.45, 1.5));
        assert_eq!(taskbar_overlay_visibility_motion(1.0, false), (1.0, 0.0));
        assert_eq!(taskbar_overlay_visibility_motion(0.0, true), (1.0, 0.0));
        assert_eq!(taskbar_overlay_visibility_motion(1.0, true), (0.0, 2.0));
    }

    #[test]
    fn a_new_enter_invalidates_a_pending_native_hide() {
        let mut visibility = TaskbarOverlayVisibilityTransition::new(true);
        let exit_generation = visibility.begin_exit().expect("begin exit");

        visibility.begin_enter();

        assert!(!visibility.complete_exit(exit_generation));
        assert!(!visibility.exiting);
    }

    #[test]
    fn native_hide_becomes_eligible_only_after_the_matching_exit() {
        let mut visibility = TaskbarOverlayVisibilityTransition::new(true);
        let exit_generation = visibility.begin_exit().expect("begin exit");

        assert!(visibility.complete_exit(exit_generation));
        assert!(!visibility.exiting);
        assert!(!visibility.complete_exit(exit_generation));
    }
}
