//! First native preview presentation. One acknowledged chain in the existing owner.
use super::*;
use gpui_kit::App;
const PRESENTATION_STEP_TIMEOUT: Duration = Duration::from_secs(3);

pub(super) struct PendingPreviewMaterial {
    token: uuid::Uuid,
    stage: PendingPreviewStage,
    initial: bool,
    started: Instant,
}

impl ClipboardApplication {
    fn wait_for_preview_material(
        &mut self,
        stage: PendingPreviewStage,
        initial: bool,
        cx: &mut Context<Self>,
    ) {
        let material = std::mem::take(&mut self.preview.material);
        self.retire_preview_material("superseded");
        self.preview.material = material;
        self.preview.pending_stage = None;
        if !self.preview.material_observers_installed {
            self.preview.material_observers_installed = true;
            cx.observe_global::<crate::ui::material::UiMaterialCapabilities>(|_, cx| {
                Self::defer_preview_material(cx);
            })
            .detach();
            cx.observe_global::<crate::theme::UiThemeState>(|_, cx| {
                Self::defer_preview_material(cx);
            })
            .detach();
        }
        let token = uuid::Uuid::new_v4();
        self.preview.material_presenting = true;
        self.preview.pending_material = Some(PendingPreviewMaterial {
            token,
            stage,
            initial,
            started: Instant::now(),
        });
        info!(event="clipboard.preview.material_started", correlation_id=%token,
            window_handle=self.preview.native_window_handle, generation=self.preview.generation);
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = owner.update(cx, |owner, cx| {
                if owner
                    .preview
                    .pending_material
                    .as_ref()
                    .is_none_or(|pending| pending.token != token)
                {
                    return;
                }
                owner.drive_preview_material(cx);
            });
        });
        cx.spawn(async move |owner, cx| {
            cx.background_executor().timer(PRESENTATION_STEP_TIMEOUT).await;
            let _ = owner.update(cx, |owner, _| {
                if owner.preview.pending_material.as_ref().is_some_and(|pending| pending.token == token) {
                    warn!(event="clipboard.preview.material_failed", correlation_id=%token, reason="timeout");
                    owner.hide_preview_window();
                }
            });
        }).detach();
    }

    fn defer_preview_material(cx: &mut Context<Self>) {
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = owner.update(cx, |owner, cx| {
                if owner.preview.pending_material.is_some() {
                    owner.drive_preview_material(cx);
                } else if owner.preview.material_presenting && !owner.preview_material_ready(cx) {
                    owner.hide_preview_window();
                }
            });
        });
    }

    fn preview_material_stage_is_current(&self, stage: &PendingPreviewStage) -> bool {
        self.config.clipboard.enabled
            && preview_callback_is_current(
                self.preview.generation,
                stage.generation,
                self.preview.hovered_item_id,
                stage.item.id,
                self.preview.scroll_suppressed,
                self.preview.native_window_handle,
                stage.native_window_handle,
            )
    }

    pub(super) fn preview_material_ready(&self, cx: &App) -> bool {
        self.config.clipboard.enabled
            && self.preview.material_presenting
            && self.preview.material.receipt(cx).is_some_and(|receipt| {
                receipt.window_handle == self.preview.native_window_handle
                    && receipt.surface
                        == nxtools_protocol::window_material::MaterialSurface::Transient
            })
    }

    fn drive_preview_material(&mut self, cx: &mut Context<Self>) {
        let Some(pending) = self.preview.pending_material.as_ref() else {
            return;
        };
        if pending.stage.native_window_handle != self.preview.native_window_handle {
            self.retire_preview_material("stale_window");
            return;
        }
        if pending.started.elapsed() >= PRESENTATION_STEP_TIMEOUT {
            self.hide_preview_window();
            return;
        }
        if !self.preview_material_stage_is_current(&pending.stage) {
            self.hide_preview_window();
            return;
        }
        if let Some(window) = self.preview.window.as_ref().map(|preview| preview.window)
            && let Some(preparation) = crate::ui::material::runtime::take_preparation(window, cx)
        {
            self.preview.material.retire(&self.platform);
            self.preview.material = preparation;
        }
        let ready = self.preview.material.ensure(
            &self.platform,
            self.preview.native_window_handle,
            nxtools_protocol::window_material::MaterialSurface::Transient,
            cx,
        );
        match ready {
            Err(reason) => {
                warn!(event = "clipboard.preview.material_failed", reason);
                self.hide_preview_window();
            }
            Ok(false) => {
                // A reused visible preview cannot stage its old frame with stale material.
                if self.preview.visible {
                    if self
                        .platform
                        .send_command(PlatformCommand::SetWindowVisibility(
                            SetWindowVisibilityCommand {
                                window_handle: self.preview.native_window_handle,
                                visible: false,
                            },
                        ))
                        .is_err()
                    {
                        self.hide_preview_window();
                        return;
                    }
                    self.preview.visible = false;
                    self.preview.displayed_item_id = None;
                }
            }
            Ok(true) => {
                let pending = self
                    .preview
                    .pending_material
                    .take()
                    .expect("current material barrier");
                let receipt = self
                    .preview
                    .material
                    .receipt(cx)
                    .expect("qualified preview receipt");
                info!(event="clipboard.preview.material_completed", correlation_id=%pending.token,
                    generation=pending.stage.generation, effective=?receipt.effective,
                    fallback_reason=?receipt.fallback_reason, native_error_code=receipt.native_error_code,
                    elapsed_ms=pending.started.elapsed().as_millis() as u64);
                if pending.initial {
                    self.queue_initial_preview_step(pending.stage, cx);
                } else {
                    let command =
                        PlatformCommand::SetWindowNoActivate(SetWindowNoActivateCommand {
                            window_handle: pending.stage.native_window_handle,
                            enabled: true,
                        });
                    self.dispatch_preview_presentation_step(pending.stage, command, cx);
                }
            }
        }
    }

    pub(super) fn finish_preview_material(
        &mut self,
        id: uuid::Uuid,
        result: &nxtools_protocol::PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(accepted) = self.preview.material.accept(id, result, cx) else {
            return false;
        };
        match accepted {
            Ok(true) => {
                info!(event="clipboard.preview.material_receipt", command_id=%id,
                    generation=self.preview.generation, window_handle=self.preview.native_window_handle);
                Self::defer_preview_material(cx);
            }
            Ok(false) => Self::defer_preview_material(cx),
            Err(reason) => {
                warn!(event="clipboard.preview.material_failed", command_id=%id, reason,
                    native_error_code=?result.native_error_code);
                self.hide_preview_window();
            }
        }
        true
    }

    pub(super) fn retire_preview_material(&mut self, reason: &'static str) {
        self.preview.material.retire(&self.platform);
        self.preview.material_presenting = false;
        if let Some(pending) = self.preview.pending_material.take() {
            info!(event="clipboard.preview.material_cancelled", correlation_id=%pending.token,
                reason, elapsed_ms=pending.started.elapsed().as_millis() as u64);
        }
    }

    fn transfer_preview_material(&mut self, cx: &mut Context<Self>) {
        let Some(window) = self.preview.window.as_ref().map(|preview| preview.window) else {
            return;
        };
        let generation = self.preview.generation;
        let native = self.preview.native_window_handle;
        let preparation = std::mem::take(&mut self.preview.material);
        let platform = Rc::clone(&self.platform);
        let owner = cx.entity().downgrade();
        self.preview.material_presenting = false;
        cx.defer(move |cx| {
            let current = owner
                .update(cx, |owner, _| {
                    if owner.preview.generation != generation
                        || owner.preview.native_window_handle != native
                        || owner
                            .preview
                            .window
                            .as_ref()
                            .is_none_or(|preview| preview.window != window)
                        || !owner.preview.visible
                        || owner.preview.material_presenting
                        || owner.preview.pending_material.is_some()
                    {
                        return false;
                    }
                    true
                })
                .unwrap_or(false);
            if current {
                crate::ui::material::runtime::track_prepared(window, platform, preparation, cx);
            } else {
                let mut preparation = preparation;
                preparation.retire(&platform);
            }
        });
    }

    pub(super) fn arm_preview_frame_timeout(
        &mut self,
        token: uuid::Uuid,
        native: i64,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |owner, cx| {
            cx.background_executor().timer(PRESENTATION_STEP_TIMEOUT).await;
            let _ = owner.update(cx, |owner, _| {
                if owner.preview.native_window_handle == native
                    && owner.preview.pending_stage.as_ref().is_some_and(|stage| stage.command_id == token) {
                    warn!(event="clipboard.preview.presentation_failed", command_id=%token, reason="frame_timeout");
                    owner.hide_preview_window();
                }
            });
        }).detach();
    }

    pub(super) fn preview_frame_is_current(
        &mut self,
        token: uuid::Uuid,
        stage: &PendingPreviewStage,
        cx: &App,
    ) -> bool {
        if self
            .preview
            .pending_stage
            .as_ref()
            .is_none_or(|pending| pending.command_id != token)
            || !self.preview_material_stage_is_current(stage)
        {
            return false;
        }
        if !self.preview_material_ready(cx) {
            self.hide_preview_window();
            return false;
        }
        true
    }

    pub(super) fn reveal_reused_preview(
        &mut self,
        token: uuid::Uuid,
        stage: PendingPreviewStage,
        cx: &mut Context<Self>,
    ) {
        if !self.preview_frame_is_current(token, &stage, cx) {
            return;
        }
        self.preview.pending_stage = None;
        let (x, y) = preview_device_origin(stage.bounds, stage.panel_scale_factor);
        if self
            .platform
            .send_command(PlatformCommand::SetWindowPosition(
                SetWindowPositionCommand {
                    window_handle: stage.native_window_handle,
                    x,
                    y,
                },
            ))
            .is_err()
        {
            self.hide_preview_window();
            return;
        }
        if self
            .platform
            .send_command(PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                window_handle: stage.native_window_handle,
                enabled: true,
            }))
            .is_err()
        {
            warn!(event="clipboard.preview.presentation_failed", command_id=%token, reason="reuse_topmost_enqueue");
        }
        self.preview.visible = true;
        self.preview.displayed_item_id = Some(stage.item.id);
        if let Some(timing) = self.preview.timing.take() {
            info!(event="clipboard.preview.reuse_reveal_queued", correlation_id=%timing.correlation_id,
                generation=stage.generation, elapsed_ms=timing.started_at.elapsed().as_millis() as u64,
                native_elapsed_ms=?timing.native_started_at.map(|started| started.elapsed().as_millis() as u64));
        }
        self.transfer_preview_material(cx);
    }
}

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
                        cx.defer(move |cx| {
                            let _ = callback_owner.update(cx, |owner, cx| {
                                owner.complete_preview_content_measurement(token, layout, cx)
                            });
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
        self.wait_for_preview_material(stage, false, cx);
    }
    pub(super) fn begin_initial_preview_presentation(
        &mut self,
        mut stage: PendingPreviewStage,
        cx: &mut Context<Self>,
    ) {
        if self.preview.native_window_handle == 0 {
            return;
        }
        stage.native_window_handle = self.preview.native_window_handle;
        self.wait_for_preview_material(stage, true, cx);
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
    pub(super) fn dispatch_preview_presentation_step(
        &mut self,
        mut stage: PendingPreviewStage,
        command: PlatformCommand,
        cx: &mut Context<Self>,
    ) {
        if !self.preview_material_stage_is_current(&stage) || !self.preview_material_ready(cx) {
            self.hide_preview_window();
            return;
        }
        let phase = stage.phase;
        let window_handle = stage.native_window_handle;
        let command_id = uuid::Uuid::new_v4();
        if let Some(timing) = self.preview.timing.as_mut() {
            timing.step_started_at = Some(Instant::now());
        }
        let show = matches!(
            phase,
            PreviewStagePhase::InitialShow | PreviewStagePhase::ShowOffscreen
        );
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
                self.transfer_preview_material(cx);
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
            self.arm_preview_frame_timeout(token, expected_window, cx);
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
                    if !owner.preview_material_ready(cx) {
                        owner.hide_preview_window();
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

#[cfg(test)]
mod material_tests {
    use super::*;
    use crate::ui::material::UiMaterialCapabilities;
    use gpui_kit::TestAppContext;
    use nxtools_protocol::window_material::*;
    use nxtools_protocol::{Envelope, Message, PlatformCommandResult};

    struct Fixture {
        owner: Entity<ClipboardApplication>,
        endpoint: nxtools_ipc::PlatformServerEndpoint,
        worker: crate::clipboard_worker::ClipboardCaptureWorkerHandle,
        _directory: tempfile::TempDir,
    }
    fn setup(cx: &mut TestAppContext) -> Fixture {
        cx.update(|cx| {
            cx.set_global(UiMaterialCapabilities::default());
            cx.set_global(crate::theme::UiThemeState::default());
        });
        let directory = tempfile::tempdir().expect("synthetic preview store");
        let config_path = directory.path().join("config.json");
        AppConfig::default()
            .save_to(&config_path)
            .expect("fixture config");
        let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
            directory.path().join("history.db"),
            directory.path().join("assets"),
            ClipboardRetentionPolicy {
                max_items: 1000,
                retention_days: 30,
            },
            1000,
        )
        .expect("temporary worker");
        let (bridge, endpoint) = nxtools_ipc::platform_bridge(32);
        let _ = endpoint.open_connection_for_contract_test();
        let owner = cx.new(|cx| {
            let notifications = cx.new(|_| NotificationCenter::default());
            ClipboardApplication::new(
                worker.clone(),
                Rc::new(bridge),
                AppConfig::default(),
                config_path,
                notifications,
                cx,
            )
        });
        while endpoint.try_receive_command_for_contract_test().is_ok() {}
        owner.update(cx, |owner, _| {
            owner.preview.native_window_handle = 88;
            owner.preview.generation = 4;
            owner.preview.hovered_item_id = Some(7);
        });
        Fixture {
            owner,
            endpoint,
            worker,
            _directory: directory,
        }
    }
    fn facts(cx: &mut App, revision: u64) {
        let mut state = cx
            .try_global::<UiMaterialCapabilities>()
            .copied()
            .unwrap_or_default();
        state.connected();
        let session_id = state
            .snapshot()
            .map_or_else(uuid::Uuid::new_v4, |facts| facts.session_id);
        assert!(state.apply(SystemMaterialCapabilitiesChanged {
            correlation_id: uuid::Uuid::new_v4(),
            session_id,
            revision,
            supported: true,
            transparency_enabled: true,
            high_contrast: false,
            energy_saver: false,
            remote_session: false,
            native_error_code: 0,
            system_backdrop: true,
            rounded_corners: true,
        }));
        cx.set_global(state);
    }
    fn stage() -> PendingPreviewStage {
        let item = nxtools_clipboard::ClipboardItem {
            id: 7,
            kind: ClipboardKind::Files,
            text: String::new(),
            image_path: String::new(),
            image_width: 0,
            image_height: 0,
            file_paths: Vec::new(),
            source: nxtools_clipboard::CaptureSource {
                window_handle: 0,
                process_id: 0,
                process_name: String::new(),
                process_icon_path: String::new(),
                window_title: String::new(),
            },
            captured_at_unix_ms: 1,
            last_used_at_unix_ms: 1,
            copy_count: 1,
            is_favorite: false,
            is_pinned: false,
            deleted_at_unix_ms: None,
        };
        PendingPreviewStage {
            command_id: uuid::Uuid::nil(),
            native_window_handle: 88,
            phase: PreviewStagePhase::InitialNoActivate,
            generation: 4,
            item,
            layout: PreviewLayout {
                window_width: 190.0,
                window_height: 76.0,
                body_height: 30.0,
                requires_vertical_scroll: false,
                maximum_height: 560.0,
                image: None,
            },
            preview_size: size(px(190.0), px(76.0)),
            bounds: Bounds::new(point(px(120.0), px(140.0)), size(px(190.0), px(76.0))),
            panel_scale_factor: 1.25,
            content_measured: true,
        }
    }
    fn next(fixture: &Fixture) -> Envelope {
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .expect("original command queue")
    }
    fn receipt(command: ApplyWindowMaterialCommand) -> PlatformCommandResult {
        PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: Some(
                serde_json::to_string(&AppliedWindowMaterial {
                    session_id: command.session_id,
                    facts_revision: command.facts_revision,
                    window_handle: command.window_handle,
                    generation: command.generation,
                    style: command.style,
                    surface: command.surface,
                    dark: command.dark,
                    effective: match command.style {
                        WindowMaterialStyle::Solid => EffectiveWindowMaterial::Solid,
                        _ => EffectiveWindowMaterial::Acrylic,
                    },
                    fallback_reason: MaterialFallbackReason::None,
                    native_error_code: 0,
                })
                .expect("typed receipt"),
            ),
        }
    }
    fn deliver(
        fixture: &Fixture,
        envelope: &Envelope,
        result: PlatformCommandResult,
        cx: &mut TestAppContext,
    ) {
        fixture
            .endpoint
            .route_command_result_for_contract_test(envelope.id, result.clone())
            .expect("original result route");
        fixture.owner.update(cx, |owner, cx| {
            owner.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: envelope.id,
                    result,
                },
                cx,
            )
        });
        cx.run_until_parked();
    }
    fn qualify(fixture: &Fixture, cx: &mut TestAppContext) {
        cx.run_until_parked();
        let envelope = next(fixture);
        let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) =
            envelope.message
        else {
            panic!("material must precede native presentation");
        };
        assert_eq!(command.surface, MaterialSurface::Transient);
        deliver(fixture, &envelope, receipt(command), cx);
    }
    fn ack(fixture: &Fixture, envelope: &Envelope, cx: &mut TestAppContext) {
        deliver(
            fixture,
            envelope,
            PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: None,
            },
            cx,
        );
    }

    #[gpui_kit::test]
    fn passive_material_first_show_requires_current_receipt_and_original_native_chain(
        cx: &mut TestAppContext,
    ) {
        let fixture = setup(cx);
        cx.update(|cx| facts(cx, 1));
        fixture.owner.update(cx, |owner, cx| {
            owner.begin_initial_preview_presentation(stage(), cx)
        });
        qualify(&fixture, cx);
        for expected in ["passive", "bounds", "topmost", "show"] {
            let envelope = next(&fixture);
            assert!(match (&envelope.message, expected) {
                (
                    Message::PlatformCommand(PlatformCommand::SetWindowNoActivate(command)),
                    "passive",
                ) => command.enabled,
                (Message::PlatformCommand(PlatformCommand::SetWindowBounds(command)), "bounds") =>
                    command.x == 150
                        && command.y == 175
                        && command.width == 238
                        && command.height == 95,
                (
                    Message::PlatformCommand(PlatformCommand::SetWindowTopmost(command)),
                    "topmost",
                ) => command.enabled,
                (
                    Message::PlatformCommand(PlatformCommand::SetWindowVisibility(command)),
                    "show",
                ) => command.visible,
                _ => false,
            });
            ack(&fixture, &envelope, cx);
        }
        fixture.owner.read_with(cx, |owner, _| {
            assert_eq!(owner.preview.displayed_item_id, Some(7))
        });
        fixture.worker.shutdown().expect("temporary worker closed");
    }

    #[gpui_kit::test]
    fn passive_material_missing_facts_dismiss_disconnect_and_timeout_never_start_show(
        cx: &mut TestAppContext,
    ) {
        for terminal in ["dismiss", "disconnect", "timeout"] {
            let fixture = setup(cx);
            fixture.owner.update(cx, |owner, cx| {
                owner.begin_initial_preview_presentation(stage(), cx)
            });
            cx.run_until_parked();
            assert!(
                fixture
                    .endpoint
                    .try_receive_command_for_contract_test()
                    .is_err()
            );
            match terminal {
                "dismiss" => fixture
                    .owner
                    .update(cx, |owner, cx| owner.dismiss_preview(cx)),
                "disconnect" => fixture.owner.update(cx, |owner, cx| {
                    owner.handle_platform_notification(
                        PlatformNotification::Disconnected {
                            generation: 1,
                            pending_command_ids: Vec::new(),
                        },
                        cx,
                    )
                }),
                _ => {
                    cx.executor().advance_clock(PRESENTATION_STEP_TIMEOUT);
                    cx.run_until_parked();
                }
            }
            fixture.owner.read_with(cx, |owner, _| {
                assert!(owner.preview.pending_material.is_none());
                assert!(!owner.preview.visible);
            });
            fixture.worker.shutdown().expect("temporary worker closed");
        }
    }

    #[gpui_kit::test]
    fn passive_material_changed_facts_replace_receipt_without_resetting_hover(
        cx: &mut TestAppContext,
    ) {
        let fixture = setup(cx);
        cx.update(|cx| facts(cx, 1));
        fixture.owner.update(cx, |owner, cx| {
            owner.begin_reused_preview_presentation(stage(), cx)
        });
        cx.run_until_parked();
        let old = next(&fixture);
        let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) = old.message
        else {
            panic!("material")
        };
        cx.update(|cx| facts(cx, 2));
        cx.run_until_parked();
        deliver(&fixture, &old, receipt(command), cx);
        let fresh = next(&fixture);
        let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(current)) = fresh.message
        else {
            panic!("latest material")
        };
        assert_eq!(current.facts_revision, 2);
        assert!(
            fixture
                .endpoint
                .try_receive_command_for_contract_test()
                .is_err()
        );
        deliver(&fixture, &fresh, receipt(current), cx);
        assert!(matches!(
            next(&fixture).message,
            Message::PlatformCommand(PlatformCommand::SetWindowNoActivate(_))
        ));
        fixture
            .owner
            .read_with(cx, |owner, _| assert_eq!(owner.preview.generation, 4));
        fixture.worker.shutdown().expect("temporary worker closed");
    }

    #[gpui_kit::test]
    fn passive_material_reuse_offscreen_show_rejects_stale_appearance(cx: &mut TestAppContext) {
        for change in ["facts", "style", "dark", "session"] {
            let fixture = setup(cx);
            cx.update(|cx| facts(cx, 1));
            fixture.owner.update(cx, |owner, cx| {
                owner.begin_reused_preview_presentation(stage(), cx)
            });
            qualify(&fixture, cx);
            for _ in 0..2 {
                let envelope = next(&fixture);
                ack(&fixture, &envelope, cx);
            }
            let bounds = next(&fixture);
            assert!(matches!(
                bounds.message,
                Message::PlatformCommand(PlatformCommand::SetWindowBounds(_))
            ));
            cx.update(|cx| match change {
                "facts" => facts(cx, 2),
                "style" => {
                    cx.default_global::<crate::theme::UiThemeState>()
                        .material_style = nxtools_core::MaterialStyle::Acrylic
                }
                "dark" => {
                    cx.default_global::<crate::theme::UiThemeState>()
                        .system_dark = false
                }
                _ => {
                    cx.default_global::<UiMaterialCapabilities>().disconnected();
                    facts(cx, 1);
                }
            });
            ack(&fixture, &bounds, cx);
            while let Ok(envelope) = fixture.endpoint.try_receive_command_for_contract_test() {
                assert!(!matches!(
                    envelope.message,
                    Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                        SetWindowVisibilityCommand { visible: true, .. }
                    ))
                ));
            }
            fixture
                .owner
                .read_with(cx, |owner, _| assert!(!owner.preview.visible));
            fixture.worker.shutdown().expect("temporary worker closed");
        }
    }

    #[gpui_kit::test]
    fn passive_material_invalid_or_dismissed_receipt_never_reveals(cx: &mut TestAppContext) {
        for invalid in ["window", "generation", "surface", "dismiss"] {
            let fixture = setup(cx);
            cx.update(|cx| facts(cx, 1));
            fixture.owner.update(cx, |owner, cx| {
                owner.begin_initial_preview_presentation(stage(), cx)
            });
            cx.run_until_parked();
            let envelope = next(&fixture);
            let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(mut command)) =
                envelope.message
            else {
                panic!("material")
            };
            match invalid {
                "window" => command.window_handle = 99,
                "generation" => command.generation += 1,
                "surface" => command.surface = MaterialSurface::Transparent,
                _ => fixture
                    .owner
                    .update(cx, |owner, cx| owner.dismiss_preview(cx)),
            }
            deliver(&fixture, &envelope, receipt(command), cx);
            assert!(
                fixture
                    .endpoint
                    .try_receive_command_for_contract_test()
                    .is_err()
            );
            fixture.owner.read_with(cx, |owner, _| {
                assert!(!owner.preview.visible && owner.preview.pending_material.is_none())
            });
            fixture.worker.shutdown().expect("temporary worker closed");
        }
    }

    #[gpui_kit::test]
    fn passive_material_frame_cannot_reveal_after_timeout_or_stale_receipt(
        cx: &mut TestAppContext,
    ) {
        for terminal in ["timeout", "facts", "hover", "dismiss"] {
            let fixture = setup(cx);
            cx.update(|cx| facts(cx, 1));
            fixture.owner.update(cx, |owner, cx| {
                owner.begin_reused_preview_presentation(stage(), cx)
            });
            qualify(&fixture, cx);
            let _ = next(&fixture);
            let token = uuid::Uuid::new_v4();
            let mut frame = stage();
            frame.command_id = token;
            fixture.owner.update(cx, |owner, cx| {
                owner.preview.visible = true;
                owner.preview.pending_stage = Some(frame.clone());
                owner.arm_preview_frame_timeout(token, 88, cx);
            });
            match terminal {
                "timeout" => {
                    cx.run_until_parked();
                    cx.executor().advance_clock(PRESENTATION_STEP_TIMEOUT);
                    cx.run_until_parked();
                }
                "facts" => cx.update(|cx| facts(cx, 2)),
                "hover" => fixture
                    .owner
                    .update(cx, |owner, _| owner.preview.generation += 1),
                _ => fixture
                    .owner
                    .update(cx, |owner, cx| owner.dismiss_preview(cx)),
            }
            fixture.owner.update(cx, |owner, cx| {
                owner.reveal_reused_preview(token, frame, cx)
            });
            while let Ok(envelope) = fixture.endpoint.try_receive_command_for_contract_test() {
                assert!(matches!(
                    envelope.message,
                    Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                        SetWindowVisibilityCommand { visible: false, .. }
                    ))
                ));
            }
            fixture.owner.read_with(cx, |owner, _| {
                assert!(owner.preview.displayed_item_id.is_none())
            });
            fixture.worker.shutdown().expect("temporary worker closed");
        }
    }

    #[gpui_kit::test]
    fn passive_material_original_reveal_transfers_and_reuse_reclaims_same_preparation(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let fixture = setup(cx);
        let view_slot = Rc::new(RefCell::new(None));
        let opened_slot = Rc::clone(&view_slot);
        let owner = fixture.owner.downgrade();
        let window: AnyWindowHandle = cx
            .add_window(move |window, cx| {
                let stage = stage();
                let preview =
                    cx.new(|cx| ClipboardPreview::new(stage.item, stage.layout, owner, window, cx));
                *opened_slot.borrow_mut() = Some(preview.downgrade());
                Root::new(preview, window, cx)
            })
            .into();
        fixture.owner.update(cx, |owner, _| {
            owner.preview.window = Some(PreviewWindow {
                window,
                view: view_slot
                    .borrow_mut()
                    .take()
                    .expect("actual preview entity"),
            })
        });
        cx.update(|cx| facts(cx, 1));
        fixture.owner.update(cx, |owner, cx| {
            owner.begin_initial_preview_presentation(stage(), cx)
        });
        qualify(&fixture, cx);
        for _ in 0..4 {
            let envelope = next(&fixture);
            ack(&fixture, &envelope, cx);
        }
        let original = cx.update(|cx| {
            crate::ui::material::runtime::receipt(window, cx).expect("deferred runtime handoff")
        });
        fixture.owner.update(cx, |owner, cx| {
            assert!(owner.preview.material.receipt(cx).is_none());
            owner.begin_reused_preview_presentation(stage(), cx);
            assert!(
                owner.preview.pending_stage.is_none(),
                "cache-hit continuation must remain deferred"
            );
        });
        cx.run_until_parked();
        assert!(matches!(
            next(&fixture).message,
            Message::PlatformCommand(PlatformCommand::SetWindowNoActivate(_))
        ));
        assert!(
            fixture
                .endpoint
                .try_receive_command_for_contract_test()
                .is_err(),
            "no duplicate material request"
        );
        cx.update(|cx| assert!(crate::ui::material::runtime::receipt(window, cx).is_none()));
        fixture.owner.read_with(cx, |owner, cx| {
            assert_eq!(owner.preview.material.receipt(cx), Some(original));
            assert_eq!(owner.preview.generation, 4);
            assert_eq!(
                owner
                    .preview
                    .window
                    .as_ref()
                    .expect("retained preview")
                    .window,
                window
            );
        });
        fixture
            .owner
            .update(cx, |owner, cx| owner.dismiss_preview(cx));
        cx.update(|cx| {
            let _ = window.update(cx, |_, window, _| window.remove_window());
        });
        fixture.worker.shutdown().expect("temporary worker closed");
    }

    #[gpui_kit::test]
    fn passive_material_deferred_handoff_reapplies_new_facts_without_hiding(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let fixture = setup(cx);
        let slot = Rc::new(RefCell::new(None));
        let opened = slot.clone();
        let owner = fixture.owner.downgrade();
        let window: AnyWindowHandle = cx
            .add_window(move |window, cx| {
                let stage = stage();
                let preview =
                    cx.new(|cx| ClipboardPreview::new(stage.item, stage.layout, owner, window, cx));
                *opened.borrow_mut() = Some(preview.downgrade());
                Root::new(preview, window, cx)
            })
            .into();
        fixture.owner.update(cx, |owner, _| {
            owner.preview.window = Some(PreviewWindow {
                window,
                view: slot.borrow_mut().take().expect("retained preview"),
            });
        });
        cx.update(|cx| facts(cx, 1));
        fixture.owner.update(cx, |owner, cx| {
            owner.begin_initial_preview_presentation(stage(), cx)
        });
        qualify(&fixture, cx);
        for _ in 0..3 {
            let envelope = next(&fixture);
            ack(&fixture, &envelope, cx);
        }
        let show = next(&fixture);
        assert!(matches!(
            show.message,
            Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand { visible: true, .. }
            ))
        ));
        fixture.owner.update(cx, |owner, cx| {
            assert!(owner.handle_preview_stage_result(show.id, true, None, cx));
            facts(cx, 2);
        });
        cx.run_until_parked();
        let update = next(&fixture);
        let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) =
            update.message
        else {
            panic!("runtime only reapplies material");
        };
        assert_eq!(command.facts_revision, 2);
        fixture.owner.read_with(cx, |owner, _| {
            assert!(owner.preview.visible);
            assert_eq!(owner.preview.generation, 4);
            assert_eq!(
                owner.preview.window.as_ref().expect("same preview").window,
                window
            );
        });
        assert!(
            fixture
                .endpoint
                .try_receive_command_for_contract_test()
                .is_err()
        );
        fixture.worker.shutdown().expect("temporary worker closed");
    }
}
