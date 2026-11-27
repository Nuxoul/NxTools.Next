//! One startup barrier on the original dispatcher; no competing notification receiver.
use super::*;
use gpui_kit::App;
use nxtools_protocol::main_window::{PrepareMainWindowCommand, PreparedMainWindow};
use nxtools_protocol::window_material::{
    AppliedWindowMaterial, ApplyWindowMaterialCommand, MaterialSurface,
};
use std::collections::VecDeque;

const MAX_BUFFERED_STARTUP_NOTIFICATIONS: usize = 128;
const MAIN_PREPARATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);
type Ready = Box<dyn FnOnce(&mut App)>;

struct PendingMaterial {
    id: Uuid,
    command: ApplyWindowMaterialCommand,
    applied: Option<AppliedWindowMaterial>,
}

pub(crate) struct MainWindowPreparation {
    command: PrepareMainWindowCommand,
    pending: Option<Uuid>,
    acknowledged: bool,
    material: Option<PendingMaterial>,
    material_window: Option<gpui_kit::AnyWindowHandle>,
    buffered: VecDeque<PlatformNotification>,
    ready: Option<Ready>,
    started: Instant,
    keep_application_on_failure: bool,
}
impl MainWindowPreparation {
    pub(crate) fn new(window_handle: i64, ready: impl FnOnce(&mut App) + 'static) -> Self {
        Self {
            command: PrepareMainWindowCommand {
                window_handle,
                source_process_id: std::process::id(),
                width: crate::MAIN_WINDOW_LOGICAL_WIDTH as i32,
                height: crate::MAIN_WINDOW_LOGICAL_HEIGHT as i32,
            },
            pending: None,
            acknowledged: false,
            material: None,
            material_window: None,
            buffered: VecDeque::new(),
            ready: Some(Box::new(ready)),
            started: Instant::now(),
            keep_application_on_failure: false,
        }
    }
    pub(crate) fn with_material_window(
        mut self,
        window: Option<gpui_kit::AnyWindowHandle>,
    ) -> Self {
        self.material_window = window;
        self
    }
    pub(crate) fn keep_application_on_failure(mut self) -> Self {
        self.keep_application_on_failure = true;
        self
    }
    fn is_ready(&self, material_id: Uuid, cx: &App) -> bool {
        self.acknowledged
            && self.material.as_ref().is_some_and(|pending| {
                pending.id == material_id
                    && crate::ui::material::is_current(&pending.command, cx)
                    && pending
                        .applied
                        .is_some_and(|receipt| receipt.matches(&pending.command))
            })
    }
}
impl PlatformEventDispatcher {
    pub(super) fn intercept_main_preparation(
        &mut self,
        notification: &PlatformNotification,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(preparation) = self.main_preparation.as_mut() else {
            return false;
        };
        if matches!(notification, PlatformNotification::Disconnected { .. }) {
            self.fail_main_preparation("platform_disconnected", None, cx);
            return true;
        }
        if matches!(
            notification,
            PlatformNotification::Event(
                PlatformEvent::SystemMaterialCapabilitiesChanged(_)
                    | PlatformEvent::SystemThemePreferenceChanged(_)
                    | PlatformEvent::SystemMotionPreferenceChanged(_)
            )
        ) {
            // These facts have already been applied by the original dispatcher, outside buffering.
            self.advance_main_material(cx);
            return true;
        }
        if let PlatformNotification::CommandResult {
            correlation_id,
            result,
        } = notification
        {
            if preparation.pending == Some(*correlation_id) && !preparation.acknowledged {
                let valid = result.success
                    && result
                        .data
                        .as_deref()
                        .filter(|data| data.len() <= 1024)
                        .and_then(|data| serde_json::from_str::<PreparedMainWindow>(data).ok())
                        .is_some_and(|readback| {
                            readback.matches(preparation.command.width, preparation.command.height)
                        });
                if !valid {
                    self.fail_main_preparation(
                        "native_result_or_readback",
                        result.native_error_code,
                        cx,
                    );
                    return true;
                }
                preparation.acknowledged = true;
                self.advance_main_material(cx);
                return true;
            }
            if let Some(material) = preparation.material.as_mut()
                && material.id == *correlation_id
            {
                if !crate::ui::material::is_current(&material.command, cx) {
                    self.advance_main_material(cx);
                    return true;
                }
                // A native fact change may arrive immediately after a rejected stale request.
                if !result.success && result.error.as_deref() == Some("stale_material_facts") {
                    return true;
                }
                let receipt = result
                    .success
                    .then_some(result.data.as_deref())
                    .flatten()
                    .filter(|data| data.len() <= 2048)
                    .and_then(|data| serde_json::from_str::<AppliedWindowMaterial>(data).ok())
                    .filter(|receipt| receipt.matches(&material.command));
                let Some(receipt) = receipt else {
                    self.fail_main_preparation(
                        "native_material_result",
                        result.native_error_code,
                        cx,
                    );
                    return true;
                };
                if material.applied.is_some() {
                    return true;
                }
                material.applied = Some(receipt);
                crate::ui::material::paint::publish(material.command, receipt, cx);
                let id = material.id;
                self.schedule_main_ready(id, cx);
                return true;
            }
        }
        if matches!(notification, PlatformNotification::Connected) {
            if preparation.pending.is_some() {
                return true;
            }
            preparation.buffered.push_back(notification.clone());
            let id = Uuid::new_v4();
            preparation.pending = Some(id);
            info!(event="product.main_window.prepare.requested", correlation_id=%id, window_handle=preparation.command.window_handle);
            if self
                .platform
                .send_command_with_id(
                    PlatformCommand::PrepareMainWindow(preparation.command.clone()),
                    id,
                )
                .is_err()
            {
                self.fail_main_preparation("platform_enqueue", None, cx);
            }
            return true;
        }
        if preparation.buffered.len() >= MAX_BUFFERED_STARTUP_NOTIFICATIONS {
            self.fail_main_preparation("startup_notification_budget", None, cx);
        } else {
            preparation.buffered.push_back(notification.clone());
        }
        true
    }

    pub(super) fn advance_main_material(&mut self, cx: &mut Context<Self>) {
        let Some(preparation) = self.main_preparation.as_mut() else {
            return;
        };
        if !preparation.acknowledged
            || preparation
                .material
                .as_ref()
                .is_some_and(|pending| crate::ui::material::is_current(&pending.command, cx))
        {
            return;
        }
        let Some(command) = crate::ui::material::command(
            preparation.command.window_handle,
            MaterialSurface::Backdrop,
            cx,
        ) else {
            return;
        };
        let id = Uuid::new_v4();
        if let Some(previous) = preparation.material.take() {
            self.platform.retire_queued_window_material(previous.id);
        }
        preparation.material = Some(PendingMaterial {
            id,
            command,
            applied: None,
        });
        info!(event="product.main_window.material.requested", correlation_id=%id, session_id=%command.session_id,
            facts_revision=command.facts_revision, window_handle=command.window_handle, generation=command.generation);
        if self
            .platform
            .send_command_with_id(PlatformCommand::ApplyWindowMaterial(command), id)
            .is_err()
        {
            self.fail_main_preparation("material_enqueue", None, cx);
        }
    }

    fn schedule_main_ready(&self, material_id: Uuid, cx: &mut Context<Self>) {
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let prepared = owner.update(cx, |dispatcher, cx| {
                let preparation = dispatcher.main_preparation.as_mut().filter(|preparation| preparation.is_ready(material_id, cx))?;
                let ready = preparation.ready.take()?;
                let tracking = preparation.material_window.and_then(|window| {
                    let material = preparation.material.as_ref()?;
                    let state = crate::ui::material::Preparation::confirmed(material.id, material.command, material.applied?, cx)?;
                    Some((window, state))
                });
                Some((ready, tracking, Rc::clone(&dispatcher.platform)))
            }).ok().flatten();
            if let Some((ready, tracking, platform)) = prepared {
                if let Some((window, material)) = tracking {
                    crate::ui::material::runtime::track_prepared(window, platform, material, cx);
                }
                ready(cx);
            }
            let _ = owner.update(cx, |dispatcher, cx| {
                if dispatcher.main_preparation.as_ref().is_none_or(|preparation|
                    !preparation.is_ready(material_id, cx) || preparation.ready.is_some()) { return; }
                let preparation = dispatcher.main_preparation.take().expect("current prepared startup");
                let applied = preparation.material.as_ref().and_then(|material| material.applied).expect("qualified material receipt");
                info!(event="product.main_window.prepare.completed", correlation_id=%material_id,
                    effective=?applied.effective, fallback_reason=?applied.fallback_reason,
                    native_error_code=applied.native_error_code, elapsed_ms=preparation.started.elapsed().as_millis());
                for buffered in preparation.buffered { dispatcher.forward(buffered, cx); }
            });
        });
    }

    pub(super) fn fail_main_preparation(
        &mut self,
        reason: &'static str,
        native_error_code: Option<i32>,
        cx: &mut Context<Self>,
    ) {
        if let Some(preparation) = self.main_preparation.take() {
            if let Some(material) = &preparation.material {
                self.platform.retire_queued_window_material(material.id);
            }
            warn!(event="product.main_window.prepare.failed", command_id=?preparation.pending, reason,
                ?native_error_code, elapsed_ms=preparation.started.elapsed().as_millis());
            if preparation.keep_application_on_failure {
                self.main_window_handle = 0;
                for buffered in preparation.buffered {
                    self.forward(buffered, cx);
                }
                return;
            }
            self.main_preparation_failed = true;
            cx.quit();
        }
    }
    pub(super) fn schedule_main_preparation_deadline(&self, cx: &mut Context<Self>) {
        if self.main_preparation.is_none() {
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(MAIN_PREPARATION_TIMEOUT)
                .await;
            let _ = this.update(cx, |dispatcher, cx| {
                dispatcher.fail_main_preparation("deadline", None, cx)
            });
        })
        .detach();
    }
}

#[cfg(test)]
mod tests;
