use gpui::{
    Animation, AnimationExt, Context, Entity, IntoElement, Render, Subscription, Window, div,
    prelude::*, px, transparent_black,
};
use nxtools_download::DownloadStatus;
use nxtools_ipc::PlatformBridge;
use nxtools_protocol::{PlatformCommand, SetTaskbarProgressOverlayVisibilityCommand};
use std::{rc::Rc, time::Duration};
use tracing::{info, warn};

use crate::downloads::DownloadApplication;
use crate::theme::Theme;

pub(crate) const TASKBAR_OVERLAY_WIDTH: i32 = 132;
pub(crate) const TASKBAR_OVERLAY_HEIGHT: i32 = 8;
const TASKBAR_OVERLAY_MAX_LANES: usize = 3;
const TASKBAR_OVERLAY_LANE_HEIGHT: f32 = 2.0;
const _: () = {
    assert!(TASKBAR_OVERLAY_WIDTH <= 160);
    assert!(TASKBAR_OVERLAY_HEIGHT <= 8);
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DownloadTaskbarOverlaySample {
    pub status: DownloadStatus,
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
    },
}

impl DownloadTaskbarOverlayModel {
    pub(crate) fn is_visible(&self) -> bool {
        !matches!(self, Self::Hidden)
    }

    #[cfg(test)]
    fn visible_lane_count(&self) -> usize {
        match self {
            Self::Hidden => 0,
            Self::Visible { lanes } => lanes.len(),
        }
    }
}

fn taskbar_progress_lane(
    samples: impl IntoIterator<Item = DownloadTaskbarOverlaySample>,
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

pub(crate) fn aggregate_taskbar_progress(
    samples: impl IntoIterator<Item = DownloadTaskbarOverlaySample>,
) -> DownloadTaskbarOverlayModel {
    let active = samples
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
    if active.is_empty() {
        return DownloadTaskbarOverlayModel::Hidden;
    }
    let split_at = active
        .len()
        .min(TASKBAR_OVERLAY_MAX_LANES.saturating_sub(1));
    let mut lanes = active[..split_at]
        .iter()
        .copied()
        .map(|sample| taskbar_progress_lane([sample]))
        .collect::<Vec<_>>();
    if split_at < active.len() {
        lanes.push(taskbar_progress_lane(active[split_at..].iter().copied()));
    }
    DownloadTaskbarOverlayModel::Visible { lanes }
}

pub(crate) struct DownloadTaskbarOverlayView {
    theme: Theme,
    model: DownloadTaskbarOverlayModel,
    native_window_handle: i64,
    _subscription: Subscription,
}

impl DownloadTaskbarOverlayView {
    pub(crate) fn new(
        application: Entity<DownloadApplication>,
        platform: Rc<PlatformBridge>,
        native_window_handle: i64,
        initial_model: DownloadTaskbarOverlayModel,
        cx: &mut Context<Self>,
    ) -> Self {
        let platform_for_subscription = Rc::clone(&platform);
        let subscription = cx.observe(&application, move |this, application, cx| {
            let next = application.read(cx).taskbar_overlay_model();
            if this.model != next {
                let visibility_changed = this.model.is_visible() != next.is_visible();
                let next_visible = next.is_visible();
                this.model = next;
                cx.notify();
                if visibility_changed && this.native_window_handle != 0 {
                    match platform_for_subscription.send_command(
                        PlatformCommand::SetTaskbarProgressOverlayVisibility(
                            SetTaskbarProgressOverlayVisibilityCommand {
                                window_handle: this.native_window_handle,
                                visible: next_visible,
                            },
                        ),
                    ) {
                        Ok(correlation_id) => info!(
                            event = "download.taskbar_overlay.visibility_accepted",
                            %correlation_id,
                            visible = next_visible,
                            "taskbar overlay visibility change accepted"
                        ),
                        Err(error) => warn!(
                            event = "download.taskbar_overlay.visibility_accept_failed",
                            %error,
                            visible = next_visible,
                            "taskbar overlay visibility change could not be queued"
                        ),
                    }
                }
            }
        });
        Self {
            theme: Theme::zed_dark(),
            model: initial_model,
            native_window_handle,
            _subscription: subscription,
        }
    }
}

impl Render for DownloadTaskbarOverlayView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let root = div()
            .size_full()
            .relative()
            .bg(transparent_black())
            .overflow_hidden()
            .flex()
            .flex_col()
            .justify_center()
            .gap(px(1.0));
        match &self.model {
            DownloadTaskbarOverlayModel::Hidden => root,
            DownloadTaskbarOverlayModel::Visible { lanes } => root.children(
                lanes
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(index, lane)| taskbar_progress_lane_element(theme, index, lane)),
            ),
        }
    }
}

fn taskbar_progress_lane_element(
    theme: Theme,
    index: usize,
    lane: DownloadTaskbarOverlayLane,
) -> gpui::AnyElement {
    let track = div()
        .relative()
        .w_full()
        .h(px(TASKBAR_OVERLAY_LANE_HEIGHT))
        .overflow_hidden()
        .bg(theme.border_subtle);
    match lane {
        DownloadTaskbarOverlayLane::Indeterminate => track
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h_full()
                    .w(px(TASKBAR_OVERLAY_WIDTH as f32 * 0.35))
                    .bg(theme.accent)
                    .opacity(0.78)
                    .with_animation(
                        ("download-taskbar-indeterminate", index),
                        Animation::new(Duration::from_millis(1_400)).repeat(),
                        |segment, delta| {
                            segment.left(px(TASKBAR_OVERLAY_WIDTH as f32 * 0.65 * delta))
                        },
                    ),
            )
            .into_any_element(),
        DownloadTaskbarOverlayLane::Determinate { completed, total } => {
            let ratio = if total == 0 {
                0.0
            } else {
                completed as f32 / total as f32
            }
            .clamp(0.0, 1.0);
            track
                .child(
                    div()
                        .h_full()
                        .w(px(TASKBAR_OVERLAY_WIDTH as f32 * ratio))
                        .bg(theme.accent),
                )
                .into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_downloads_preserve_their_individual_progress() {
        let model = aggregate_taskbar_progress([
            DownloadTaskbarOverlaySample {
                status: DownloadStatus::Downloading,
                downloaded_bytes: 20,
                total_bytes: Some(100),
            },
            DownloadTaskbarOverlaySample {
                status: DownloadStatus::Waiting,
                downloaded_bytes: 50,
                total_bytes: Some(100),
            },
        ]);

        assert_eq!(
            model,
            DownloadTaskbarOverlayModel::Visible {
                lanes: vec![
                    DownloadTaskbarOverlayLane::Determinate {
                        completed: 20,
                        total: 100,
                    },
                    DownloadTaskbarOverlayLane::Determinate {
                        completed: 50,
                        total: 100,
                    },
                ]
            }
        );
    }

    #[test]
    fn aggregate_progress_hides_without_active_tasks() {
        let model = aggregate_taskbar_progress([
            DownloadTaskbarOverlaySample {
                status: DownloadStatus::Paused,
                downloaded_bytes: 20,
                total_bytes: Some(100),
            },
            DownloadTaskbarOverlaySample {
                status: DownloadStatus::Completed,
                downloaded_bytes: 100,
                total_bytes: Some(100),
            },
        ]);

        assert_eq!(model, DownloadTaskbarOverlayModel::Hidden);
    }

    #[test]
    fn aggregate_progress_is_indeterminate_when_an_active_length_is_unknown() {
        let model = aggregate_taskbar_progress([
            DownloadTaskbarOverlaySample {
                status: DownloadStatus::Downloading,
                downloaded_bytes: 20,
                total_bytes: Some(100),
            },
            DownloadTaskbarOverlaySample {
                status: DownloadStatus::Probing,
                downloaded_bytes: 0,
                total_bytes: None,
            },
        ]);

        assert_eq!(
            model,
            DownloadTaskbarOverlayModel::Visible {
                lanes: vec![
                    DownloadTaskbarOverlayLane::Determinate {
                        completed: 20,
                        total: 100,
                    },
                    DownloadTaskbarOverlayLane::Indeterminate,
                ]
            }
        );
    }

    #[test]
    fn separate_active_downloads_keep_separate_taskbar_progress_lanes() {
        let model = aggregate_taskbar_progress([
            DownloadTaskbarOverlaySample {
                status: DownloadStatus::Downloading,
                downloaded_bytes: 25,
                total_bytes: Some(100),
            },
            DownloadTaskbarOverlaySample {
                status: DownloadStatus::Waiting,
                downloaded_bytes: 150,
                total_bytes: Some(300),
            },
        ]);

        assert_eq!(model.visible_lane_count(), 2);
    }

    #[test]
    fn overflow_downloads_share_the_last_weighted_lane() {
        let model = aggregate_taskbar_progress([
            DownloadTaskbarOverlaySample {
                status: DownloadStatus::Downloading,
                downloaded_bytes: 10,
                total_bytes: Some(100),
            },
            DownloadTaskbarOverlaySample {
                status: DownloadStatus::Downloading,
                downloaded_bytes: 20,
                total_bytes: Some(100),
            },
            DownloadTaskbarOverlaySample {
                status: DownloadStatus::Downloading,
                downloaded_bytes: 30,
                total_bytes: Some(100),
            },
            DownloadTaskbarOverlaySample {
                status: DownloadStatus::Waiting,
                downloaded_bytes: 150,
                total_bytes: Some(300),
            },
        ]);

        assert_eq!(
            model,
            DownloadTaskbarOverlayModel::Visible {
                lanes: vec![
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
            }
        );
    }
}
