use super::*;
use crate::ui::material::runtime;
use gpui_kit::App;
use nxtools_protocol::window_material::MaterialSurface;

#[derive(Clone, Copy)]
struct MaterialWait {
    id: Uuid,
    phase: Phase,
    window: AnyWindowHandle,
    hwnd: i64,
    session: Uuid,
    generation: u64,
    connection: Option<u64>,
}

impl FloatingAssistantApplication {
    pub(super) fn material_admitted(
        &mut self,
        id: Uuid,
        operation: FloatingAssistantOperation,
        phase: Phase,
        cx: &mut Context<Self>,
    ) -> bool {
        if !matches!(phase, Phase::Prepare | Phase::Update | Phase::Show) {
            return true;
        }
        let Some(surface) = self.surface.as_ref() else {
            return false;
        };
        let window = surface.window;
        let hwnd = surface.hwnd;
        if runtime::receipt(window, cx).is_some_and(|receipt| {
            receipt.window_handle == hwnd && receipt.surface == MaterialSurface::Transparent
        }) {
            self.material_wait = None;
            return true;
        }
        self.material_wait = Some(id);
        let wait = MaterialWait {
            id,
            phase,
            window,
            hwnd,
            session: self.session,
            generation: self.generation,
            connection: self.connection,
        };
        let owner = cx.entity().downgrade();
        let platform = self.platform.clone();
        cx.defer(move |cx| {
            let current = owner.upgrade().is_some_and(|owner| owner.read(cx).current_material_wait(wait, cx));
            if !current { return; }
            runtime::when_ready(window, platform, MaterialSurface::Transparent, Duration::from_secs(5), move |result, cx| {
                let _ = owner.update(cx, |owner, cx| {
                    if !owner.current_material_wait(wait, cx) { return; }
                    owner.material_wait = None;
                    if let Err(reason) = result {
                        owner.lifecycle.pending = None;
                        tracing::warn!(event = "floating_assistant.material.failed", correlation_id = %id,
                            session_id = %wait.session, generation = wait.generation, reason);
                        owner.fail("悬浮窗透明宿主尚未准备完成，请重试。", cx);
                        owner.emergency_hide(cx);
                        return;
                    }
                    if !owner.desired_visible() {
                        owner.lifecycle.pending = None;
                        owner.sync(cx);
                    } else if owner.material_admitted(id, operation, phase, cx) {
                        owner.dispatch_pending(id, operation, phase, cx);
                    }
                });
            }, cx);
        });
        false
    }

    fn current_material_wait(&self, wait: MaterialWait, cx: &App) -> bool {
        self.material_wait == Some(wait.id)
            && self.lifecycle.pending == Some((wait.id, wait.phase))
            && self.session == wait.session
            && self.generation == wait.generation
            && self.connection == wait.connection
            && self.platform.floating_assistant_connection() == wait.connection
            && self
                .surface
                .as_ref()
                .is_some_and(|surface| surface.window == wait.window && surface.hwnd == wait.hwnd)
            && cx.windows().contains(&wait.window)
            && cx.windows().contains(&self.source)
    }
}
