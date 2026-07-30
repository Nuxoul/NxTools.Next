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

pub(crate) const TASKBAR_OVERLAY_WIDTH: i32 = 280;
pub(crate) const TASKBAR_OVERLAY_HEIGHT: i32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DownloadTaskbarOverlaySample {
    pub status: DownloadStatus,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DownloadTaskbarOverlayModel {
    Hidden,
    Indeterminate,
    Determinate { completed: u64, total: u64 },
}

impl DownloadTaskbarOverlayModel {
    pub(crate) fn is_visible(self) -> bool {
        !matches!(self, Self::Hidden)
    }
}

pub(crate) fn aggregate_taskbar_progress(
    samples: impl IntoIterator<Item = DownloadTaskbarOverlaySample>,
) -> DownloadTaskbarOverlayModel {
    let mut active_count = 0_u64;
    let mut completed = 0_u64;
    let mut total = 0_u64;
    for sample in samples {
        if !matches!(
            sample.status,
            DownloadStatus::Probing
                | DownloadStatus::Waiting
                | DownloadStatus::Downloading
                | DownloadStatus::Finalizing
                | DownloadStatus::Verifying
        ) {
            continue;
        }
        active_count = active_count.saturating_add(1);
        let Some(sample_total) = sample.total_bytes.filter(|total| *total > 0) else {
            return DownloadTaskbarOverlayModel::Indeterminate;
        };
        completed = completed.saturating_add(sample.downloaded_bytes.min(sample_total));
        total = total.saturating_add(sample_total);
    }
    if active_count == 0 {
        DownloadTaskbarOverlayModel::Hidden
    } else if total == 0 {
        DownloadTaskbarOverlayModel::Indeterminate
    } else {
        DownloadTaskbarOverlayModel::Determinate { completed, total }
    }
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
                this.model = next;
                cx.notify();
                if visibility_changed && this.native_window_handle != 0 {
                    match platform_for_subscription.send_command(
                        PlatformCommand::SetTaskbarProgressOverlayVisibility(
                            SetTaskbarProgressOverlayVisibilityCommand {
                                window_handle: this.native_window_handle,
                                visible: next.is_visible(),
                            },
                        ),
                    ) {
                        Ok(correlation_id) => info!(
                            event = "download.taskbar_overlay.visibility_accepted",
                            %correlation_id,
                            visible = next.is_visible(),
                            "taskbar overlay visibility change accepted"
                        ),
                        Err(error) => warn!(
                            event = "download.taskbar_overlay.visibility_accept_failed",
                            %error,
                            visible = next.is_visible(),
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
            .overflow_hidden();
        match self.model {
            DownloadTaskbarOverlayModel::Hidden => root,
            DownloadTaskbarOverlayModel::Indeterminate => root.bg(theme.border_subtle).child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h_full()
                    .w(px(TASKBAR_OVERLAY_WIDTH as f32 * 0.35))
                    .bg(theme.accent)
                    .opacity(0.78)
                    .with_animation(
                        "download-taskbar-indeterminate",
                        Animation::new(Duration::from_millis(1_400)).repeat(),
                        |segment, delta| {
                            segment.left(px(TASKBAR_OVERLAY_WIDTH as f32 * 0.65 * delta))
                        },
                    ),
            ),
            DownloadTaskbarOverlayModel::Determinate { completed, total } => {
                let ratio = if total == 0 {
                    0.0
                } else {
                    completed as f32 / total as f32
                }
                .clamp(0.0, 1.0);
                root.bg(theme.border_subtle).child(
                    div()
                        .h_full()
                        .w(px(TASKBAR_OVERLAY_WIDTH as f32 * ratio))
                        .bg(theme.accent),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_progress_is_weighted_by_total_bytes() {
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
            DownloadTaskbarOverlayModel::Determinate {
                completed: 70,
                total: 200
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

        assert_eq!(model, DownloadTaskbarOverlayModel::Indeterminate);
    }
}
