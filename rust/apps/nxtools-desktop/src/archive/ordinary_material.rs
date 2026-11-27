//! Material barriers feed the original job/form presentation owners.
use super::*;
use nxtools_protocol::window_material::MaterialSurface;
use std::time::Duration;
#[cfg(test)]
#[path = "ordinary_material_job_tests.rs"]
mod tests;

impl ArchiveApplication {
    pub(super) fn prepare_job_material(
        &mut self,
        surface: Uuid,
        window: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) {
        self.jobs.material_pending.insert(surface, window);
        let platform = self.platform.clone();
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            crate::ui::material::runtime::when_ready(
                window,
                platform,
                MaterialSurface::Backdrop,
                Duration::from_secs(8),
                move |result, cx| {
                    let _ = owner.update(cx, |app, cx| {
                        app.finish_job_material(surface, window, result, cx)
                    });
                },
                cx,
            );
        });
    }

    fn finish_job_material(
        &mut self,
        surface: Uuid,
        window: AnyWindowHandle,
        result: Result<(), &'static str>,
        cx: &mut Context<Self>,
    ) {
        if self.jobs.material_pending.get(&surface) != Some(&window)
            || !self.job_window_is_current(surface, window)
        {
            return;
        }
        self.jobs.material_pending.remove(&surface);
        if let Err(reason) = result {
            tracing::warn!(event="archive.job.material_failed", correlation_id=%surface, reason);
            self.retire_job_activation(surface, "material_failed");
            if let Some((window, view)) = self
                .jobs
                .records
                .get_mut(&surface)
                .and_then(|record| record.window.take())
            {
                cx.defer(move |cx| {
                    let _ = view.update(cx, |view, _| view.retire_display());
                    let _ = window.update(cx, |_, window, _| window.remove_window());
                });
            }
            return;
        }
        // Batch progression may have selected a different member while material was pending.
        let selected = self
            .jobs
            .records
            .get(&surface)
            .and_then(|record| record.batch)
            .and_then(|batch| self.jobs.batches.get(&batch))
            .and_then(|batch| batch.selected)
            .unwrap_or(surface);
        if !self.jobs.records.contains_key(&selected) || self.job_surface_id(selected) != surface {
            return;
        }
        let _ = window.update(cx, |_, window, _| window.activate_window());
        self.refresh_job(selected, cx);
        self.activate_job(selected, JobPresentation::Show, cx);
    }

    pub(super) fn prepare_form_material(
        &mut self,
        id: Uuid,
        generation: u64,
        window: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) {
        let platform = self.platform.clone();
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            crate::ui::material::runtime::when_ready(
                window, platform, MaterialSurface::Backdrop, Duration::from_secs(8),
                move |result, cx| {
                    let _ = owner.update(cx, |app, cx| {
                        let Some(slot) = app.jobs.forms.get(&id).filter(|slot| slot.window == window && slot.generation == generation) else { return; };
                        let view = slot.view.clone();
                        if let Err(reason) = result {
                            tracing::warn!(event="archive.form.material_failed", correlation_id=%id, generation, reason);
                            let _ = window.update(cx, |_, window, cx| {
                                let _ = view.update(cx, |form, cx| form.retire(window, cx));
                                window.remove_window();
                            });
                            app.retire_form(id);
                            app.refresh_all_batch_views(cx);
                        } else {
                            let _ = window.update(cx, |_, window, _| window.activate_window());
                        }
                    });
                }, cx,
            );
        });
    }
}
