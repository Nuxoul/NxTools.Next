use super::*;
use crate::ui::material::runtime;
use nxtools_protocol::window_material::MaterialSurface;

#[derive(Default)]
pub(super) struct MaterialAdmission {
    generation: u64,
    pending: Option<Pending>,
    blocked: bool,
}

#[derive(Clone, Copy)]
struct Pending {
    operation: uuid::Uuid,
    generation: u64,
    window: AnyWindowHandle,
    native: i64,
    host: i64,
    connection: Option<u64>,
    started: Instant,
}

impl QuickLaunchApplication {
    pub(super) fn cancel_file_dialog_material(&mut self) {
        let material = &mut self.file_dialog_search.material;
        material.generation = material.generation.wrapping_add(1);
        if let Some(pending) = material.pending.take() {
            info!(event = "quick_launch.file_dialog_material.retired", correlation_id = %pending.operation,
                elapsed_ms = pending.started.elapsed().as_millis() as u64);
        }
        material.blocked = false;
    }

    pub(super) fn admit_file_dialog_material(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.config.quick_launch.enabled || !self.config.quick_launch.file_dialog_search {
            return false;
        }
        let material = &self.file_dialog_search.material;
        if material.blocked || material.pending.is_some() {
            return false;
        }
        let Some(window) = self.file_dialog_search.window.as_ref() else {
            return false;
        };
        if window.native_window_handle == 0 || !cx.windows().contains(&window.window) {
            return false;
        }
        if runtime::receipt(window.window, cx)
            .is_some_and(|receipt| receipt.window_handle == window.native_window_handle)
            && self.platform.is_connected()
        {
            return true;
        }
        let Some(host) = self
            .file_dialog_search
            .host
            .as_ref()
            .filter(|host| host.visible)
        else {
            return false;
        };
        let pending = Pending {
            operation: uuid::Uuid::new_v4(),
            generation: material.generation,
            window: window.window,
            native: window.native_window_handle,
            host: host.window_handle,
            connection: self.platform.connection_generation(),
            started: Instant::now(),
        };
        self.file_dialog_search.material.pending = Some(pending);
        info!(event = "quick_launch.file_dialog_material.accepted", correlation_id = %pending.operation,
            window_handle = pending.native);
        self.wait_file_dialog_material(pending, cx);
        false
    }

    fn file_dialog_material_is_current(&self, pending: Pending, cx: &App) -> bool {
        self.config.quick_launch.enabled
            && self.config.quick_launch.file_dialog_search
            && self.file_dialog_search.material.generation == pending.generation
            && self
                .file_dialog_search
                .material
                .pending
                .is_some_and(|current| current.operation == pending.operation)
            && self
                .file_dialog_search
                .window
                .as_ref()
                .is_some_and(|window| {
                    window.window == pending.window && window.native_window_handle == pending.native
                })
            && self
                .file_dialog_search
                .host
                .as_ref()
                .is_some_and(|host| host.visible && host.window_handle == pending.host)
            && cx.windows().contains(&pending.window)
    }

    fn wait_file_dialog_material(&self, pending: Pending, cx: &mut Context<Self>) {
        let owner = cx.entity().downgrade();
        let platform = Rc::clone(&self.platform);
        cx.defer(move |cx| {
            if !owner
                .read_with(cx, |owner, cx| {
                    owner.file_dialog_material_is_current(pending, cx)
                })
                .unwrap_or(false)
            {
                return;
            }
            let remaining = Duration::from_secs(3).saturating_sub(pending.started.elapsed());
            runtime::when_ready(
                pending.window,
                platform,
                MaterialSurface::Transient,
                remaining,
                move |result, cx| {
                    let _ = owner.update(cx, |owner, cx| {
                        owner.finish_file_dialog_material(pending, result, cx)
                    });
                },
                cx,
            );
        });
    }

    fn finish_file_dialog_material(
        &mut self,
        pending: Pending,
        mut result: Result<(), &'static str>,
        cx: &mut Context<Self>,
    ) {
        if !self.file_dialog_material_is_current(pending, cx) {
            return;
        }
        if !self.platform.is_connected()
            || pending.connection != self.platform.connection_generation()
        {
            result = Err("material_disconnected");
        } else if pending.started.elapsed() >= Duration::from_secs(3) {
            result = Err("material_timeout");
        }
        if let Err(reason) = result {
            self.file_dialog_search.material.pending = None;
            self.hide_file_dialog_search_window(cx);
            self.file_dialog_search.material.blocked = true;
            warn!(event = "quick_launch.file_dialog_material.completed", correlation_id = %pending.operation,
                success = false, reason, elapsed_ms = pending.started.elapsed().as_millis() as u64);
            return;
        }
        if runtime::receipt(pending.window, cx)
            .is_none_or(|receipt| receipt.window_handle != pending.native)
        {
            self.wait_file_dialog_material(pending, cx);
            return;
        }
        self.file_dialog_search.material.pending = None;
        info!(event = "quick_launch.file_dialog_material.completed", correlation_id = %pending.operation,
            success = true, elapsed_ms = pending.started.elapsed().as_millis() as u64);
        self.apply_file_dialog_search_window_placement(cx);
    }
}
