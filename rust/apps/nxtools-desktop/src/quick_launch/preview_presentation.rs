//! Material readiness inside the original passive preview presentation owner.
use super::*;
use crate::ui::material::{UiMaterialCapabilities, runtime};
use nxtools_protocol::window_material::MaterialSurface;

const PREVIEW_MATERIAL_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Copy)]
pub(super) enum PreviewMaterialAction {
    InitialFrame,
    InitialShow,
    ShowOffscreen,
    RevealOnscreen,
}
pub(crate) struct PreviewMaterialWait {
    stage: QuickLaunchPreviewPendingStage,
    action: PreviewMaterialAction,
}

impl QuickLaunchApplication {
    pub(super) fn begin_preview_material_presentation(&mut self, cx: &mut Context<Self>) {
        if let Some((id, started)) = self.preview.material_lifecycle.take() {
            info!(event = "quick_launch.preview.prepare.retired", correlation_id = %id,
                reason = "superseded", elapsed_ms = started.elapsed().as_millis() as u64);
        }
        self.preview.material_wait = None;
        self.preview.pending_stage = None;
        self.preview.material.retire(&self.platform);
        if let Some(preview) = self.preview.window.as_ref()
            && let Some(preparation) = runtime::take_preparation(preview.identity.gpui, cx)
        {
            self.preview.material = preparation;
        }
        if self.preview.material_subscriptions.is_empty() {
            self.preview.material_subscriptions = vec![
                cx.observe_global::<UiMaterialCapabilities>(|_, cx| {
                    let owner = cx.entity().downgrade();
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |owner, cx| owner.progress_preview_material(cx));
                    });
                }),
                cx.observe_global::<crate::theme::UiThemeState>(|_, cx| {
                    let owner = cx.entity().downgrade();
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |owner, cx| owner.progress_preview_material(cx));
                    });
                }),
            ];
        }
        let id = uuid::Uuid::new_v4();
        self.preview.material_lifecycle = Some((id, Instant::now()));
        info!(event = "quick_launch.preview.prepare.started", correlation_id = %id,
            generation = self.preview.generation);
        cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(PREVIEW_MATERIAL_TIMEOUT)
                .await;
            let _ = owner.update(cx, |owner, cx| {
                if owner
                    .preview
                    .material_lifecycle
                    .is_some_and(|(current, _)| current == id)
                {
                    owner.fail_preview_material("timeout", None, cx);
                }
            });
        })
        .detach();
    }

    pub(super) fn preview_stage_is_current(
        &self,
        stage: &QuickLaunchPreviewPendingStage,
        cx: &App,
    ) -> bool {
        self.preview.window.as_ref().is_some_and(|preview| {
            preview.identity == stage.window_identity
                && cx.windows().contains(&preview.identity.gpui)
        }) && self.preview.generation == stage.generation
            && self.preview.hovered_entry_id.as_deref() == Some(stage.entry_id.as_str())
            && self
                .preview
                .material_lifecycle
                .is_some_and(|(_, started)| started.elapsed() < PREVIEW_MATERIAL_TIMEOUT)
    }

    pub(super) fn wait_for_preview_material(
        &mut self,
        stage: QuickLaunchPreviewPendingStage,
        action: PreviewMaterialAction,
        cx: &mut Context<Self>,
    ) {
        if !self.preview_stage_is_current(&stage, cx) {
            return;
        }
        self.preview.material_wait = Some(PreviewMaterialWait { stage, action });
        self.progress_preview_material(cx);
    }

    fn progress_preview_material(&mut self, cx: &mut Context<Self>) {
        let Some(wait) = self.preview.material_wait.as_ref() else {
            return;
        };
        if !self.preview_stage_is_current(&wait.stage, cx) {
            self.fail_preview_material("retired", None, cx);
            return;
        }
        let hwnd = wait.stage.window_identity.native;
        match self
            .preview
            .material
            .ensure(&self.platform, hwnd, MaterialSurface::Transient, cx)
        {
            Ok(false) => return,
            Err(reason) => {
                self.fail_preview_material(reason, None, cx);
                return;
            }
            Ok(true) => {}
        }
        if !self.preview.material.receipt(cx).is_some_and(|receipt| {
            receipt.window_handle == hwnd && receipt.surface == MaterialSurface::Transient
        }) {
            return;
        }
        let PreviewMaterialWait { mut stage, action } =
            self.preview.material_wait.take().expect("material waiter");
        match action {
            PreviewMaterialAction::InitialFrame => self.frame_prepared_preview(stage, cx),
            PreviewMaterialAction::InitialShow | PreviewMaterialAction::ShowOffscreen => {
                stage.phase = if matches!(action, PreviewMaterialAction::InitialShow) {
                    QuickLaunchPreviewStagePhase::InitialShow
                } else {
                    QuickLaunchPreviewStagePhase::ShowOffscreen
                };
                let command = PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                    window_handle: hwnd,
                    visible: true,
                });
                self.send_preview_stage(stage, command, cx);
            }
            PreviewMaterialAction::RevealOnscreen => {
                stage.phase = QuickLaunchPreviewStagePhase::RevealOnscreen;
                let x = (stage.bounds.origin.x.as_f32() * stage.panel_scale_factor).round() as i32;
                let y = (stage.bounds.origin.y.as_f32() * stage.panel_scale_factor).round() as i32;
                self.send_preview_stage(
                    stage,
                    PlatformCommand::SetWindowPosition(SetWindowPositionCommand {
                        window_handle: hwnd,
                        x,
                        y,
                    }),
                    cx,
                );
            }
        }
    }

    pub(super) fn handle_preview_material_result(
        &mut self,
        id: uuid::Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(accepted) = self.preview.material.accept(id, result, cx) else {
            return false;
        };
        match accepted {
            Ok(_) => self.progress_preview_material(cx),
            Err(reason) => self.fail_preview_material(reason, result.native_error_code, cx),
        }
        true
    }

    pub(super) fn send_preview_stage(
        &mut self,
        mut stage: QuickLaunchPreviewPendingStage,
        command: PlatformCommand,
        cx: &mut Context<Self>,
    ) {
        if !self.preview_stage_is_current(&stage, cx) {
            self.fail_preview_material("retired", None, cx);
            return;
        }
        let id = uuid::Uuid::new_v4();
        stage.command_id = id;
        self.preview.pending_stage = Some(stage);
        if self.platform.send_command_with_id(command, id).is_err() {
            self.fail_preview_material("native_enqueue", None, cx);
        }
    }

    pub(super) fn frame_prepared_preview(
        &mut self,
        mut stage: QuickLaunchPreviewPendingStage,
        cx: &mut Context<Self>,
    ) {
        let owner = cx.entity().downgrade();
        let payload = stage.payload.take();
        let image_asset = payload.as_ref().and_then(quick_launch_preview_image_asset);
        let view = self
            .preview
            .window
            .as_ref()
            .map(|preview| preview.view.clone());
        if payload.is_some() {
            self.preview.image_asset = image_asset;
        }
        if stage
            .window_identity
            .gpui
            .update(cx, |_, window, app| {
                if let Some(payload) = payload
                    && let Some(view) = view
                    && view
                        .update(app, |preview, cx| preview.set_payload(payload, cx))
                        .is_err()
                {
                    return;
                }
                window.on_next_frame(move |_, cx| {
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |owner, cx| {
                            owner.wait_for_preview_material(
                                stage,
                                PreviewMaterialAction::InitialShow,
                                cx,
                            );
                        });
                    });
                });
                window.refresh();
            })
            .is_err()
        {
            self.fail_preview_material("window_closed", None, cx);
        }
    }

    pub(super) fn prepare_initial_preview_bounds(
        &mut self,
        mut stage: QuickLaunchPreviewPendingStage,
        cx: &mut Context<Self>,
    ) {
        let scale = stage.panel_scale_factor;
        let command = PlatformCommand::SetWindowBounds(SetWindowBoundsCommand {
            window_handle: stage.window_identity.native,
            x: (stage.bounds.origin.x.as_f32() * scale).round() as i32,
            y: (stage.bounds.origin.y.as_f32() * scale).round() as i32,
            width: (stage.preview_size.width.as_f32() * scale).round().max(1.) as i32,
            height: (stage.preview_size.height.as_f32() * scale).round().max(1.) as i32,
        });
        stage.phase = QuickLaunchPreviewStagePhase::InitialBounds;
        self.send_preview_stage(stage, command, cx);
    }

    pub(super) fn finish_prepared_preview(
        &mut self,
        stage: QuickLaunchPreviewPendingStage,
        cx: &mut Context<Self>,
    ) {
        if !self.preview_stage_is_current(&stage, cx)
            || !self.preview.material.receipt(cx).is_some_and(|receipt| {
                receipt.window_handle == stage.window_identity.native
                    && receipt.surface == MaterialSurface::Transient
            })
        {
            self.fail_preview_material("stale_reveal", None, cx);
            return;
        }
        let _ = self
            .platform
            .send_command(PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                window_handle: stage.window_identity.native,
                enabled: true,
            }));
        self.preview.visible = true;
        self.preview.displayed_entry_id = Some(stage.entry_id.clone());
        if let Some((id, started)) = self.preview.material_lifecycle.take() {
            info!(event = "quick_launch.preview.prepare.completed", correlation_id = %id,
                elapsed_ms = started.elapsed().as_millis() as u64, window_handle = stage.window_identity.native);
        }
        if std::mem::take(&mut self.preview.restore_primary_focus) {
            self.restore_quick_launch_panel_keyboard_focus(cx);
        }
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let transfer = owner
                .update(cx, |owner, _cx| {
                    if owner.preview.generation != stage.generation
                        || owner
                            .preview
                            .window
                            .as_ref()
                            .is_none_or(|window| window.identity != stage.window_identity)
                        || owner.preview.displayed_entry_id.as_deref()
                            != Some(stage.entry_id.as_str())
                        || !owner.preview.visible
                        || owner.preview.material_lifecycle.is_some()
                    {
                        return None;
                    }
                    Some((
                        Rc::clone(&owner.platform),
                        std::mem::take(&mut owner.preview.material),
                    ))
                })
                .ok()
                .flatten();
            if let Some((platform, preparation)) = transfer {
                runtime::track_prepared(stage.window_identity.gpui, platform, preparation, cx);
            }
        });
    }

    fn fail_preview_material(
        &mut self,
        reason: &'static str,
        native_error_code: Option<i32>,
        cx: &mut Context<Self>,
    ) {
        if let Some((id, started)) = self.preview.material_lifecycle {
            warn!(event = "quick_launch.preview.prepare.failed", correlation_id = %id,
                reason, native_error_code, elapsed_ms = started.elapsed().as_millis() as u64);
        }
        self.hide_quick_launch_preview_window(cx);
    }
}

#[cfg(test)]
#[path = "preview_material_tests.rs"]
mod tests;
