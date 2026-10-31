//! First native preview presentation. One acknowledged chain in the existing owner.
use super::*;
const PRESENTATION_STEP_TIMEOUT: Duration = Duration::from_secs(3);

impl ClipboardApplication {
    pub(super) fn begin_preview_content_measurement(
        &mut self,
        mut stage: PendingPreviewStage,
        initial: bool,
        cx: &mut Context<Self>,
    ) {
        if stage.item.kind != ClipboardKind::Text {
            stage.content_measured = true;
            if initial {
                self.begin_initial_preview_presentation(stage, cx);
            } else {
                stage.phase = PreviewStagePhase::ReuseBounds;
                self.queue_reused_preview_bounds(stage, cx);
            }
            return;
        }
        let Some(preview) = self.preview.window.as_ref() else {
            return;
        };
        let view = preview.view.clone();
        let window = preview.window;
        let token = uuid::Uuid::new_v4();
        stage.native_window_handle = self.preview.native_window_handle;
        info!(event="clipboard.preview.content_measurement_started", correlation_id=%token, generation=stage.generation);
        stage.command_id = token;
        stage.phase = if initial {
            PreviewStagePhase::MeasureInitial
        } else {
            PreviewStagePhase::MeasureReuse
        };
        let item = stage.item.clone();
        let layout = stage.layout;
        self.preview.pending_stage = Some(stage);
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            if owner.upgrade().is_none_or(|owner| {
                owner
                    .read(cx)
                    .preview
                    .pending_stage
                    .as_ref()
                    .is_none_or(|stage| stage.command_id != token)
            }) {
                return;
            }
            let _ = window.update(cx, |_, window, cx| {
                let callback_owner = owner.clone();
                let _ = view.update(cx, |view, cx| {
                    if !initial {
                        view.set_item(item, layout, window, cx);
                    }
                    view.on_measured_layout(move |layout, cx| {
                        let _ = callback_owner.update(cx, |owner, cx| {
                            owner.complete_preview_content_measurement(token, layout, cx)
                        });
                    });
                    cx.notify();
                });
                window.draw(cx).clear(cx);
            });
        });
        let expected_window = self.preview.native_window_handle;
        cx.spawn(async move |owner, cx| {
            cx.background_executor().timer(PRESENTATION_STEP_TIMEOUT).await;
            let _ = owner.update(cx, |owner, _| {
                if owner.preview.pending_stage.as_ref().is_some_and(|stage| stage.command_id == token) {
                    if owner.preview.native_window_handle != expected_window {
                        owner.preview.pending_stage = None;
                        return;
                    }
                    warn!(event="clipboard.preview.presentation_failed", correlation_id=%token, reason="content_measurement_timeout");
                    owner.hide_preview_window();
                }
            });
        }).detach();
    }
    fn complete_preview_content_measurement(
        &mut self,
        token: uuid::Uuid,
        layout: PreviewLayout,
        cx: &mut Context<Self>,
    ) {
        if self
            .preview
            .pending_stage
            .as_ref()
            .is_none_or(|stage| stage.command_id != token)
        {
            return;
        }
        let mut stage = self
            .preview
            .pending_stage
            .take()
            .expect("matching measured preview");
        if !preview_callback_is_current(
            self.preview.generation,
            stage.generation,
            self.preview.hovered_item_id,
            stage.item.id,
            self.preview.scroll_suppressed,
            self.preview.native_window_handle,
            stage.native_window_handle,
        ) {
            return;
        }
        stage.layout = layout;
        stage.preview_size = size(px(layout.window_width), px(layout.window_height));
        let screen = cx
            .displays()
            .into_iter()
            .find(|display| display.bounds().contains(&stage.bounds.origin))
            .or_else(|| cx.primary_display())
            .map(|display| display.bounds());
        if let Some(screen) = screen {
            if let Some(candidate) = self.preview.candidate.as_ref() {
                stage.bounds = preview_bounds(
                    candidate.panel_bounds,
                    stage.preview_size,
                    screen,
                    candidate.pointer_position,
                );
            } else {
                stage.bounds.size = stage.preview_size;
                stage.bounds.origin.y = stage
                    .bounds
                    .origin
                    .y
                    .min(screen.bottom() - stage.preview_size.height)
                    .max(screen.top());
            }
        } else {
            stage.bounds.size = stage.preview_size;
        }
        stage.content_measured = true;
        info!(event="clipboard.preview.content_measured", correlation_id=%token, generation=stage.generation, height=layout.window_height, scroll=layout.requires_vertical_scroll);
        if stage.phase == PreviewStagePhase::MeasureInitial {
            stage.phase = PreviewStagePhase::InitialNoActivate;
            self.begin_initial_preview_presentation(stage, cx);
        } else {
            stage.phase = PreviewStagePhase::ReuseBounds;
            self.queue_reused_preview_bounds(stage, cx);
        }
    }
    pub(super) fn begin_reused_preview_presentation(
        &mut self,
        mut stage: PendingPreviewStage,
        cx: &mut Context<Self>,
    ) {
        let window_handle = self.preview.native_window_handle;
        if window_handle == 0 {
            return;
        }
        stage.native_window_handle = window_handle;
        stage.phase = PreviewStagePhase::ReuseNoActivate;
        self.dispatch_preview_presentation_step(
            stage,
            PlatformCommand::SetWindowNoActivate(SetWindowNoActivateCommand {
                window_handle,
                enabled: true,
            }),
            cx,
        );
    }
    pub(super) fn begin_initial_preview_presentation(
        &mut self,
        stage: PendingPreviewStage,
        cx: &mut Context<Self>,
    ) {
        if self.preview.native_window_handle == 0 {
            return;
        }
        self.queue_initial_preview_step(stage, cx);
    }
    pub(super) fn queue_reused_preview_bounds(
        &mut self,
        stage: PendingPreviewStage,
        cx: &mut Context<Self>,
    ) {
        let coordinate = PREVIEW_STAGING_COORDINATE;
        let command = PlatformCommand::SetWindowBounds(SetWindowBoundsCommand {
            window_handle: stage.native_window_handle,
            x: coordinate,
            y: coordinate,
            width: (stage.preview_size.width.as_f32() * stage.panel_scale_factor)
                .round()
                .max(1.0) as i32,
            height: (stage.preview_size.height.as_f32() * stage.panel_scale_factor)
                .round()
                .max(1.0) as i32,
        });
        self.dispatch_preview_presentation_step(stage, command, cx);
    }
    fn queue_initial_preview_step(
        &mut self,
        mut stage: PendingPreviewStage,
        cx: &mut Context<Self>,
    ) {
        let window_handle = self.preview.native_window_handle;
        stage.native_window_handle = window_handle;
        let command = match stage.phase {
            PreviewStagePhase::InitialNoActivate => {
                PlatformCommand::SetWindowNoActivate(SetWindowNoActivateCommand {
                    window_handle,
                    enabled: true,
                })
            }
            PreviewStagePhase::InitialBounds => {
                let (x, y) = preview_device_origin(stage.bounds, stage.panel_scale_factor);
                PlatformCommand::SetWindowBounds(SetWindowBoundsCommand {
                    window_handle,
                    x,
                    y,
                    width: (stage.preview_size.width.as_f32() * stage.panel_scale_factor)
                        .round()
                        .max(1.0) as i32,
                    height: (stage.preview_size.height.as_f32() * stage.panel_scale_factor)
                        .round()
                        .max(1.0) as i32,
                })
            }
            PreviewStagePhase::InitialTopmost => {
                PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                    window_handle,
                    enabled: true,
                })
            }
            PreviewStagePhase::InitialShow => {
                PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                    window_handle,
                    visible: true,
                })
            }
            _ => return,
        };
        self.dispatch_preview_presentation_step(stage, command, cx);
    }
    fn dispatch_preview_presentation_step(
        &mut self,
        mut stage: PendingPreviewStage,
        command: PlatformCommand,
        cx: &mut Context<Self>,
    ) {
        let phase = stage.phase;
        let window_handle = stage.native_window_handle;
        let command_id = uuid::Uuid::new_v4();
        if let Some(timing) = self.preview.timing.as_mut() {
            timing.step_started_at = Some(Instant::now());
        }
        let show = phase == PreviewStagePhase::InitialShow;
        stage.command_id = command_id;
        // Install identity before dispatch; a native result can be immediate.
        self.preview.pending_stage = Some(stage);
        if show {
            self.preview.visible = true;
        } // cancellation must queue hide behind show
        if self
            .platform
            .send_command_with_id(command, command_id)
            .is_err()
        {
            warn!(event="clipboard.preview.presentation_failed", %command_id, ?phase, reason="queue_rejected", "preview presentation rejected");
            self.hide_preview_window();
            return;
        }
        info!(event="clipboard.preview.presentation_started", %command_id, ?phase,
            correlation_id = ?self.preview.timing.as_ref().map(|timing| timing.correlation_id),
            "preview presentation step accepted");
        cx.spawn(async move |this,cx| {
            cx.background_executor().timer(PRESENTATION_STEP_TIMEOUT).await;
            let _=this.update(cx,|application,_| {
                if application.preview.pending_stage.as_ref().is_some_and(|stage|stage.command_id==command_id) {
                    if application.preview.native_window_handle != window_handle {
                        application.preview.pending_stage = None;
                        return;
                    }
                    warn!(event="clipboard.preview.presentation_failed", %command_id, ?phase, reason="timeout", elapsed_ms=PRESENTATION_STEP_TIMEOUT.as_millis(), "preview presentation expired");
                    application.hide_preview_window();
                }
            });
        }).detach();
    }
    pub(super) fn advance_initial_preview_presentation(
        &mut self,
        mut stage: PendingPreviewStage,
        cx: &mut Context<Self>,
    ) {
        info!(event="clipboard.preview.presentation_completed", command_id=%stage.command_id, phase=?stage.phase,
            correlation_id = ?self.preview.timing.as_ref().map(|timing| timing.correlation_id),
            step_elapsed_ms = ?self.preview.timing.as_ref().and_then(|timing| timing.step_started_at).map(|started| started.elapsed().as_millis() as u64),
            "native preview step confirmed");
        stage.phase = match stage.phase {
            PreviewStagePhase::InitialNoActivate => PreviewStagePhase::InitialBounds,
            PreviewStagePhase::InitialBounds => PreviewStagePhase::InitialTopmost,
            PreviewStagePhase::InitialTopmost => PreviewStagePhase::InitialShow,
            PreviewStagePhase::InitialShow => {
                self.preview.visible = true;
                self.preview.displayed_item_id = Some(stage.item.id);
                if let Some(timing) = self.preview.timing.take() {
                    info!(event = "clipboard.preview.first_show_confirmed", correlation_id = %timing.correlation_id,
                        command_id = %stage.command_id, generation = stage.generation,
                        elapsed_ms = timing.started_at.elapsed().as_millis() as u64,
                        native_elapsed_ms = ?timing.native_started_at.map(|started| started.elapsed().as_millis() as u64));
                }
                return;
            }
            _ => return,
        };
        if stage.phase == PreviewStagePhase::InitialTopmost
            && stage.content_measured
            && let Some(preview) = self.preview.window.as_ref()
        {
            let source = preview.window;
            let token = uuid::Uuid::new_v4();
            stage.command_id = token;
            let expected_window = stage.native_window_handle;
            self.preview.pending_stage = Some(stage);
            let owner = cx.entity().downgrade();
            cx.defer(move |cx| {
                if owner.upgrade().is_none_or(|owner| {
                    owner
                        .read(cx)
                        .preview
                        .pending_stage
                        .as_ref()
                        .is_none_or(|stage| stage.command_id != token)
                }) {
                    return;
                }
                if source
                    .update(cx, |_, window, cx| window.draw(cx).clear(cx))
                    .is_err()
                {
                    return;
                }
                let _ = owner.update(cx, |owner, cx| {
                    if owner.preview.native_window_handle != expected_window {
                        return;
                    }
                    if owner
                        .preview
                        .pending_stage
                        .as_ref()
                        .is_some_and(|stage| stage.command_id == token)
                    {
                        let stage = owner
                            .preview
                            .pending_stage
                            .take()
                            .expect("current prepared frame");
                        owner.queue_initial_preview_step(stage, cx);
                    }
                });
            });
            return;
        }
        self.queue_initial_preview_step(stage, cx);
    }
}

#[cfg(test)]
#[path = "clipboard_preview_presentation_tests.rs"]
mod tests;
