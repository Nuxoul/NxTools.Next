//! One presentation slot for the original native drag. No extractor, execution queue or OLE owner.
use super::*;
use crate::archive::progress::JobProgressView;
use nxtools_protocol::{
    PlatformCommand, PlatformCommandResult, SetWindowBoundsCommand, SetWindowNoActivateCommand,
    SetWindowVisibilityCommand, archive_virtual_drag::ArchiveVirtualDragProgress,
};
pub(super) struct DragPresentation {
    pub(super) operation: Uuid,
    generation: u64,
    session: Uuid,
    ended: bool,
    latest: ArchiveVirtualDragProgress,
    members: Rc<[Option<Uuid>]>,
    started: Instant,
    window: AnyWindowHandle,
    view: WeakEntity<JobProgressView>,
    hidden: bool,
    prepare: Option<(Uuid, u8)>,
    hwnd: i64,
    bounds: SetWindowBoundsCommand,
}
impl ArchiveApplication {
    pub(super) fn drag_progress_event(
        &mut self,
        event: ArchiveVirtualDragProgress,
        cx: &mut Context<Self>,
    ) {
        if event.validate().is_err() {
            return;
        }
        let members = if let Some(slot) = self
            .jobs
            .drag_progress
            .as_ref()
            .filter(|slot| slot.operation == event.operation_id)
        {
            if slot.ended
                || slot.session != event.session_id
                || slot.generation != event.generation
                || slot
                    .latest
                    .sequence
                    .is_some_and(|prior| event.sequence.is_none_or(|next| next <= prior))
                || event.bytes_done < slot.latest.bytes_done
                || event.files_done < slot.latest.files_done
                || slot
                    .latest
                    .bytes_processed
                    .zip(event.bytes_processed)
                    .is_some_and(|(prior, next)| next < prior)
                || (event.current_request_id.is_some()
                    && event.current_request_id == slot.latest.current_request_id
                    && slot
                        .latest
                        .current_bytes_done
                        .zip(event.current_bytes_done)
                        .is_some_and(|(prior, next)| next < prior))
            {
                return;
            }
            slot.members.clone()
        } else {
            let Some(members) = self
                .windows
                .get(&event.session_id)
                .filter(|slot| slot.generation == event.generation)
                .and_then(|slot| {
                    slot.view
                        .read_with(cx, |view, _| {
                            view.drag_progress_members(event.operation_id, event.generation)
                        })
                        .ok()
                        .flatten()
                })
            else {
                return;
            };
            members
        };
        if let Some(index) = event.current_index {
            let Some(expected) = members.get(index as usize).copied().flatten() else {
                return;
            };
            if event
                .current_request_id
                .is_some_and(|request| request != expected)
            {
                return;
            }
        }
        if self
            .jobs
            .drag_progress
            .as_ref()
            .is_some_and(|slot| slot.operation != event.operation_id)
        {
            let old = self
                .jobs
                .drag_progress
                .take()
                .expect("one old presentation");
            cx.defer(move |cx| {
                let _ = old.window.update(cx, |_, window, _| window.remove_window());
            });
        }
        let terminal = matches!(event.phase.as_str(), "finished" | "cancelled" | "failed");
        if self.jobs.drag_progress.is_none() {
            if terminal {
                return;
            }
            let operation = event.operation_id;
            let owner = cx.entity().downgrade();
            let closing = owner.clone();
            let capture = Rc::new(std::cell::RefCell::new(None));
            let captured = capture.clone();
            let initial = event.clone();
            let parent = self.windows.get(&event.session_id).map(|slot| slot.window);
            let placement = crate::ui::monitor_placement::owner_monitor_placement(None, parent, cx);
            let bounds = placement.centered(size(px(560.), px(300.)));
            let opened = cx.open_window(
                WindowOptions {
                    focus: false,
                    show: false,
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    display_id: placement.display_id,
                    window_min_size: Some(size(px(480.), px(260.))),
                    app_id: Some(format!("NxTools.Archive.Drag.{operation}")),
                    ..crate::ui::components::titlebar::caption_window_options()
                },
                move |window, cx| {
                    window.set_window_title(crate::archive::progress::DRAG_TITLE);
                    let hwnd = crate::ui::native_window_handle(window).unwrap_or(0);
                    let scale = placement.scale;
                    let native_bounds = SetWindowBoundsCommand {
                        window_handle: hwnd,
                        x: (bounds.origin.x.as_f32() * scale).round() as i32,
                        y: (bounds.origin.y.as_f32() * scale).round() as i32,
                        width: (bounds.size.width.as_f32() * scale).round() as i32,
                        height: (bounds.size.height.as_f32() * scale).round() as i32,
                    };
                    let view = cx.new(|cx| JobProgressView::new_drag(initial, owner, cx));
                    *captured.borrow_mut() = Some((view.downgrade(), hwnd, native_bounds));
                    window.on_window_should_close(cx, move |_, cx| {
                        let owner = closing.clone();
                        cx.defer(move |cx| {
                            let _ = owner
                                .update(cx, |app, cx| app.background_drag_progress(operation, cx));
                        });
                        false
                    });
                    cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
                },
            );
            if let (Ok(window), Some((view, hwnd, bounds))) = (opened, capture.borrow_mut().take())
            {
                self.jobs.drag_progress = Some(DragPresentation {
                    operation,
                    generation: event.generation,
                    session: event.session_id,
                    ended: false,
                    latest: event.clone(),
                    members,
                    started: Instant::now(),
                    window: window.into(),
                    view,
                    hidden: false,
                    prepare: None,
                    hwnd,
                    bounds,
                });
                self.prepare_drag_presentation(0, cx);
                tracing::info!(event="archive.drag_progress.opened",correlation_id=%operation,"native demand presentation created hidden");
            }
        }
        let Some(slot) = self.jobs.drag_progress.as_mut().filter(|slot| {
            slot.operation == event.operation_id
                && slot.generation == event.generation
                && !slot.ended
        }) else {
            return;
        };
        slot.latest = event.clone();
        slot.ended = terminal;
        let window = slot.window;
        let view = slot.view.clone();
        let elapsed = slot.started.elapsed().as_millis().min(u64::MAX as u128) as u64;
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, _, cx| {
                let _ = view.update(cx, |view, cx| view.apply_drag(event, elapsed, cx));
            });
        });
    }
    fn prepare_drag_presentation(&mut self, stage: u8, cx: &mut Context<Self>) {
        let Some(slot) = self.jobs.drag_progress.as_mut().filter(|slot| !slot.hidden) else {
            return;
        };
        if slot.hwnd <= 0 {
            return;
        } // TestWindow has no native HWND; no fake native acceptance.
        let receipt = Uuid::new_v4();
        slot.prepare = Some((receipt, stage));
        let command = match stage {
            0 => PlatformCommand::SetWindowNoActivate(SetWindowNoActivateCommand {
                window_handle: slot.hwnd,
                enabled: true,
            }),
            1 => PlatformCommand::SetWindowBounds(slot.bounds.clone()),
            _ => PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                window_handle: slot.hwnd,
                visible: true,
            }),
        };
        if self
            .platform
            .send_command_with_id_correlated(command, receipt, slot.operation)
            .is_err()
        {
            slot.prepare = None;
            slot.hidden = true;
        }
        let _ = cx;
    }
    pub(super) fn drag_presentation_result(
        &mut self,
        receipt: Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(slot) = self
            .jobs
            .drag_progress
            .as_mut()
            .filter(|slot| slot.prepare.is_some_and(|pending| pending.0 == receipt))
        else {
            return false;
        };
        let (_, stage) = slot.prepare.take().expect("matched presentation receipt");
        if !result.success || slot.hidden {
            slot.hidden = true;
            return true;
        }
        if stage < 2 {
            self.prepare_drag_presentation(stage + 1, cx);
        }
        true
    }
    pub(super) fn drag_progress_disconnected(&mut self, cx: &mut Context<Self>) {
        if let Some(slot) = self.jobs.drag_progress.as_mut().filter(|slot| !slot.ended) {
            slot.ended = true;
            slot.prepare = None;
            let mut event = slot.latest.clone();
            event.phase = "failed".into();
            event.cleanup_complete = Some(false);
            event.code = Some("disconnected".into());
            let window = slot.window;
            let view = slot.view.clone();
            let elapsed = slot.started.elapsed().as_millis().min(u64::MAX as u128) as u64;
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, _, cx| {
                    let _ = view.update(cx, |view, cx| view.apply_drag(event, elapsed, cx));
                });
            });
        }
    }
    pub(in crate::archive) fn background_drag_progress(
        &mut self,
        operation: Uuid,
        cx: &mut Context<Self>,
    ) {
        let Some(slot) = self
            .jobs
            .drag_progress
            .as_mut()
            .filter(|slot| slot.operation == operation)
        else {
            return;
        };
        slot.hidden = true;
        slot.prepare = None;
        let window = slot.window;
        let hwnd = slot.hwnd;
        if hwnd > 0 {
            let _ = self.platform.send_command_correlated(
                PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                    window_handle: hwnd,
                    visible: false,
                }),
                operation,
            );
        } else {
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, _| window.remove_window());
            });
        }
        tracing::info!(event="archive.drag_progress.hidden",correlation_id=%operation,"presentation hidden without native cancellation");
    }
}
