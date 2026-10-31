//! Single-entry materialization and original-window Shell/nested lease ownership.
use super::*;
use crate::archive::model::RowId;
use crate::archive::preview::{self, PreviewKind, PreviewRequest, PreviewResult};
use gpui_kit::ParentElement;
use gpui_kit::component::{
    WindowExt,
    button::Button,
    dialog::{DialogAction, DialogClose, DialogFooter},
};
use nxtools_protocol::{PlatformCommand, archive_browser::*};
use std::sync::atomic::{AtomicBool, Ordering};
#[derive(Clone)]
pub(super) struct PendingEntry {
    pub window_id: Uuid,
    pub operation: Uuid,
    pub generation: u64,
    pub snapshot: Uuid,
    pub nested: bool,
    pub depth: u32,
    pub started: Instant,
    pub cancelled: bool,
}
#[derive(Clone)]
pub(super) struct Lease {
    pub session: Uuid,
    pub generation: u64,
    pub id: Uuid,
    pub operation: Uuid,
}
impl ArchiveApplication {
    pub(crate) fn open_entry(&mut self, id: Uuid, entry: RowId, cx: &mut Context<Self>) {
        self.cancel_preview(id, cx);
        if self.entries.iter().any(|(receipt, pending)| {
            pending.window_id == id && !self.preview_entries.contains_key(receipt)
        }) {
            return;
        }
        let Some(slot) = self.windows.get(&id) else {
            return;
        };
        let Some(browser) = slot.browser.clone() else {
            self.notice(id, "源身份不可用，请重新读取后再打开条目。".into(), cx);
            return;
        };
        let RowId::Entry(snapshot, ordinal) = entry else {
            return;
        };
        if snapshot != browser.snapshot_id {
            return;
        }
        let data = slot.view.upgrade().and_then(|view| {
            view.read(cx).model.as_ref().and_then(|m| {
                m.entries.iter().find(|e| e.ordinal == ordinal).map(|e| {
                    (
                        e.name.clone(),
                        e.kind.clone(),
                        e.warning_codes.clone(),
                        e.size,
                        m.selected_file_ordinals().len(),
                    )
                })
            })
        });
        let Some((name, kind, warnings, size, count)) = data else {
            return;
        };
        if count > 1 {
            self.notice(id, "一次只能打开一个文件，请先取消其他选择。".into(), cx);
            return;
        }
        if kind != "File"
            || warnings.iter().any(|w| {
                matches!(
                    w.as_str(),
                    "UnsafePath" | "OutputCollision" | "LinkNotExtracted"
                )
            })
        {
            self.notice(id, "此条目不是可安全打开的普通文件。".into(), cx);
            return;
        }
        if size.is_some_and(|s| s > ARCHIVE_ENTRY_TEMP_BYTES) {
            self.notice(id, "条目超过 512 MiB 临时打开预算。".into(), cx);
            return;
        }
        let nested = is_archive(&name);
        if nested && slot.depth >= 3 {
            self.notice(id, "嵌套归档最多打开 3 层。".into(), cx);
            return;
        }
        let generation = slot.generation;
        if active_content(&name) {
            let source = slot.window;
            let owner = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = source.update(cx, |_, window, cx| {
                    let owner = owner.clone();
                    window.open_dialog(cx, move |dialog, _, _| {
                        let owner = owner.clone();
                        let close_owner = owner.clone();
                        dialog
                            .title("打开活动内容？")
                            .child("归档中的程序或脚本可能执行代码。仅在确认来源可信时继续。")
                            .footer(
                                DialogFooter::new()
                                    .child(
                                        DialogClose::new().trigger(|button| button.label("取消")),
                                    )
                                    .child(DialogAction::new().child(
                                        Button::new("archive-active-confirm").label("确认打开"),
                                    )),
                            )
                            .on_close(move |_, _, cx| {
                                let owner = close_owner.clone();
                                cx.defer(move |cx| {
                                    let _ = owner.update(cx, |app, cx| app.finish_open(id, cx));
                                });
                            })
                            .on_ok(move |_, _, cx| {
                                let owner = owner.clone();
                                cx.defer(move |cx| {
                                    let _ = owner.update(cx, |app, cx| {
                                        app.materialize(
                                            id, generation, snapshot, ordinal, nested, cx,
                                        )
                                    });
                                });
                                true
                            })
                    });
                });
            });
        } else {
            self.materialize(id, generation, snapshot, ordinal, nested, cx)
        }
    }
    fn materialize(
        &mut self,
        id: Uuid,
        generation: u64,
        snapshot: Uuid,
        ordinal: u64,
        nested: bool,
        cx: &mut Context<Self>,
    ) {
        if self
            .entries
            .iter()
            .any(|(receipt, p)| p.window_id == id && !self.preview_entries.contains_key(receipt))
        {
            return;
        }
        let Some(slot) = self
            .windows
            .get(&id)
            .filter(|slot| slot.generation == generation)
        else {
            return;
        };
        let Some(browser) = slot.browser.clone().filter(|b| b.snapshot_id == snapshot) else {
            return;
        };
        let operation = Uuid::new_v4();
        let receipt = Uuid::new_v4();
        let command = ArchiveEntryCommand {
            version: 1,
            operation_id: operation,
            session_id: id,
            generation,
            action: ArchiveEntryAction::Materialize,
            path: Some(slot.path.to_string_lossy().into_owned()),
            codepage: slot.view.upgrade().map_or(0, |v| v.read(cx).codepage),
            ordinal: Some(ordinal),
            browser: Some(browser),
            lease_id: None,
            depth: slot.depth,
            copy_motw: Some(self.settings.copy_motw),
            password_interaction: true,
            temp_policy: Some(ArchiveTempPolicy {
                base_directory: self.settings.temp.base_directory.clone(),
                retention_hours: self.settings.temp.retention_hours,
                cleanup_on_child_exit: self.settings.temp.cleanup_on_child_exit,
            }),
        };
        self.entries.insert(
            receipt,
            PendingEntry {
                window_id: id,
                operation,
                generation,
                snapshot,
                nested,
                depth: slot.depth,
                started: Instant::now(),
                cancelled: false,
            },
        );
        if self
            .platform
            .send_command_with_id_correlated(
                PlatformCommand::ArchiveEntry(command),
                receipt,
                operation,
            )
            .is_err()
        {
            self.entries.remove(&receipt);
            self.notice(id, "无法提交安全条目打开，请检查原生连接。".into(), cx);
            tracing::warn!(event="archive.entry.finished",correlation_id=%operation,outcome="not_submitted","archive entry submission failed");
            return;
        }
        tracing::info!(event="archive.entry.started",correlation_id=%operation,"archive entry accepted");
    }
    pub(super) fn entry_result(
        &mut self,
        command_id: Uuid,
        result: &nxtools_protocol::PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(pending) = self.entries.remove(&command_id) else {
            return false;
        };
        if let Some(request) = self.preview_entries.remove(&command_id) {
            self.dismiss_preview_password(pending.operation, cx);
            self.preview_materialized(pending, request, result, cx);
            return true;
        }
        self.passwords.remove(&pending.operation);
        self.hide_password_dialog(pending.operation, cx);
        let parsed = result
            .data
            .as_deref()
            .and_then(|data| serde_json::from_str::<ArchiveEntryResult>(data).ok());
        let current = self.windows.get(&pending.window_id).is_some_and(|slot| {
            slot.generation == pending.generation
                && slot
                    .browser
                    .as_ref()
                    .is_some_and(|b| b.snapshot_id == pending.snapshot)
                && !pending.cancelled
        });
        if current {
            self.finish_open(pending.window_id, cx);
        }
        if let Some(value) = parsed {
            let identity = value.operation_id == pending.operation
                && value.session_id == pending.window_id
                && value.generation == pending.generation
                && value.version == 1;
            if result.success
                && identity
                && value.success
                && value.cleanup_complete
                && let (Some(lease_id), Some(path)) = (value.lease_id, value.path)
            {
                let lease = Lease {
                    session: pending.window_id,
                    generation: pending.generation,
                    id: lease_id,
                    operation: pending.operation,
                };
                if !current {
                    self.release(lease);
                    return true;
                }
                if pending.nested {
                    let origin = self.windows.get(&pending.window_id).map(|slot| OpenOrigin {
                        window: slot.window,
                        browser: Some((pending.window_id, pending.generation)),
                    });
                    self.open_at_depth(
                        PathBuf::from(path),
                        pending.depth + 1,
                        vec![lease],
                        origin,
                        cx,
                    )
                } else {
                    let shell_id = Uuid::new_v4();
                    self.shells.insert(
                        shell_id,
                        (pending.window_id, pending.generation, lease.clone()),
                    );
                    if self
                        .platform
                        .send_command_with_id_correlated(
                            PlatformCommand::ArchiveEntry(ArchiveEntryCommand {
                                version: 1,
                                operation_id: lease.operation,
                                session_id: lease.session,
                                generation: lease.generation,
                                action: ArchiveEntryAction::Open,
                                path: None,
                                codepage: 0,
                                ordinal: None,
                                browser: None,
                                lease_id: Some(lease.id),
                                depth: 0,
                                copy_motw: None,
                                password_interaction: false,
                                temp_policy: None,
                            }),
                            shell_id,
                            lease.operation,
                        )
                        .is_err()
                    {
                        self.shells.remove(&shell_id);
                        self.release(lease);
                        self.notice(
                            pending.window_id,
                            "Shell 打开未提交，临时文件已退休。".into(),
                            cx,
                        )
                    }
                }
                tracing::info!(event="archive.entry.finished",correlation_id=%pending.operation,outcome="materialized",elapsed_ms=pending.started.elapsed().as_millis() as u64,"archive entry materialized");
                return true;
            }
            if current && value.code == Some(ArchiveReadError::SourceChanged) {
                self.cancel_preview(pending.window_id, cx);
                if let Some(slot) = self.windows.get_mut(&pending.window_id) {
                    slot.browser = None;
                }
                self.fail_view(
                    pending.window_id,
                    "源文件已改变；保留的视图仅供查看，请刷新后再操作。".into(),
                    cx,
                );
            }
            if let Some(lease_id) = value.lease_id {
                self.release(Lease {
                    session: pending.window_id,
                    generation: pending.generation,
                    id: lease_id,
                    operation: pending.operation,
                });
            }
        }
        if current {
            self.notice(
                pending.window_id,
                "条目解压失败或清理未确认；未启动文件，请刷新后重试。".into(),
                cx,
            )
        }
        tracing::warn!(event="archive.entry.finished",correlation_id=%pending.operation,outcome="failed",native_error_code=result.native_error_code,"archive entry failed");
        true
    }
    pub(super) fn shell_result(
        &mut self,
        id: Uuid,
        result: &nxtools_protocol::PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((window_id, generation, lease)) = self.shells.remove(&id) else {
            return false;
        };
        if let Some(slot) = self
            .windows
            .get_mut(&window_id)
            .filter(|s| s.generation == generation)
        {
            if result.success {
                slot.leases.push(lease)
            } else {
                self.release(lease);
                self.notice(window_id, "Windows 未能打开临时文件。".into(), cx)
            }
        } else {
            self.release(lease)
        }
        true
    }
    pub(super) fn release(&self, lease: Lease) {
        let command = ArchiveEntryCommand {
            version: 1,
            operation_id: lease.operation,
            session_id: lease.session,
            generation: lease.generation,
            action: ArchiveEntryAction::Release,
            path: None,
            codepage: 0,
            ordinal: None,
            browser: None,
            lease_id: Some(lease.id),
            depth: 0,
            copy_motw: None,
            password_interaction: false,
            temp_policy: None,
        };
        let _ = self
            .platform
            .send_command_correlated(PlatformCommand::ArchiveEntry(command), lease.operation);
    }
    pub(super) fn cancel_entries(&mut self, id: Uuid) {
        for pending in self.entries.values_mut().filter(|p| p.window_id == id) {
            pending.cancelled = true;
            let command = ArchiveEntryCommand {
                version: 1,
                operation_id: pending.operation,
                session_id: id,
                generation: pending.generation,
                action: ArchiveEntryAction::Cancel,
                path: None,
                codepage: 0,
                ordinal: None,
                browser: None,
                lease_id: None,
                depth: 0,
                copy_motw: None,
                password_interaction: false,
                temp_policy: None,
            };
            let _ = self
                .platform
                .send_command_correlated(PlatformCommand::ArchiveEntry(command), pending.operation);
        }
    }
    pub(super) fn finish_open(&self, id: Uuid, cx: &mut Context<Self>) {
        if let Some(slot) = self.windows.get(&id) {
            let view = slot.view.clone();
            cx.defer(move |cx| {
                let _ = view.update(cx, |view, cx| view.finish_entry_open(cx));
            });
        }
    }
    pub(super) fn notice(&self, id: Uuid, message: String, cx: &mut Context<Self>) {
        if let Some(slot) = self.windows.get(&id) {
            self.finish_open(id, cx);
            let window = slot.window;
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, cx| {
                    window.open_dialog(cx, move |dialog, _, _| {
                        dialog
                            .title("归档操作")
                            .child(message.clone())
                            .footer(DialogClose::new().trigger(|button| button.label("关闭")))
                    })
                });
            });
        }
    }
}
pub(super) fn active_content(name: &str) -> bool {
    matches!(
        name.rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "exe"
            | "com"
            | "bat"
            | "cmd"
            | "ps1"
            | "psm1"
            | "psd1"
            | "vbs"
            | "vbe"
            | "js"
            | "jse"
            | "wsf"
            | "wsh"
            | "hta"
            | "msi"
            | "msp"
            | "scr"
            | "pif"
            | "lnk"
            | "url"
            | "reg"
            | "cpl"
            | "msc"
            | "chm"
            | "docm"
            | "dotm"
            | "xlsm"
            | "xlam"
            | "pptm"
            | "potm"
            | "ppam"
    )
}
fn is_archive(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        ".zip", ".7z", ".rar", ".tar", ".tgz", ".tbz", ".tbz2", ".txz", ".tzst", ".gz", ".bz2",
        ".xz", ".zst",
    ]
    .iter()
    .any(|ext| name.ends_with(ext))
}

impl ArchiveApplication {
    /// Browser callbacks must defer this call until their own entity lease has ended.
    pub(crate) fn preview_entry(
        &mut self,
        id: Uuid,
        row: Option<RowId>,
        selection_generation: u64,
        cx: &mut Context<Self>,
    ) {
        let Some(slot) = self.windows.get(&id) else {
            return;
        };
        // UI token is mandatory: a delayed A request must not survive A -> B -> A (ABA).
        if !slot
            .view
            .read_with(cx, |view, _| {
                view.preview_selection_matches(selection_generation, &row)
            })
            .unwrap_or(false)
        {
            return;
        }
        if self.previews.get(&id).is_some_and(|state| {
            selection_generation <= state.selection_generation && !state.retired
        }) {
            return;
        }
        let generation = slot.generation;
        let browser = slot.browser.clone();
        let data = slot
            .view
            .read_with(cx, |view, _| {
                view.model.as_ref().map(|model| {
                    (
                        model.snapshot,
                        model.selected.clone(),
                        row.as_ref().and_then(|row| match row {
                            RowId::Entry(_, ordinal) => model
                                .entries
                                .iter()
                                .find(|entry| entry.ordinal == *ordinal)
                                .cloned(),
                            _ => None,
                        }),
                    )
                })
            })
            .ok()
            .flatten();
        self.cancel_preview(id, cx);
        let state = self.previews.entry(id).or_default();
        state.retired = false;
        state.selection_generation = selection_generation;
        state.row = row.clone();
        let mut result = PreviewResult::Idle;
        if let Some((snapshot, selected, entry)) = data {
            if selected.len() > 1 {
                result = PreviewResult::MultipleSelection;
            } else if let Some(row) = row {
                if !selected.contains(&row) {
                    result = PreviewResult::Cancelled;
                } else if !matches!(row, RowId::Entry(_, _)) {
                    result = PreviewResult::Directory;
                } else if let (Some(browser), Some(entry)) = (browser, entry) {
                    if browser.snapshot_id != snapshot
                        || row != RowId::Entry(snapshot, entry.ordinal)
                    {
                        result = PreviewResult::SourceChanged;
                    } else if entry.kind == "Directory" {
                        result = PreviewResult::Directory;
                    } else if entry.kind != "File"
                        || entry.warning_codes.iter().any(|code| {
                            matches!(
                                code.as_str(),
                                "UnsafePath" | "OutputCollision" | "LinkNotExtracted"
                            )
                        })
                    {
                        result = PreviewResult::Unsupported;
                    } else if let Some(kind) = preview::kind(&entry.name) {
                        if entry.size.is_some_and(|size| size > kind.limit()) {
                            result = PreviewResult::TooLarge;
                        } else {
                            state.latest = Some(PreviewRequest {
                                window_id: id,
                                generation,
                                browser,
                                row,
                                selection_generation,
                                kind,
                                started: Instant::now(),
                            });
                            result = PreviewResult::Loading;
                        }
                    } else {
                        result = PreviewResult::Unsupported;
                    }
                } else {
                    result = PreviewResult::SourceChanged;
                }
            }
        }
        if state.blocked {
            state.latest = None;
            result = PreviewResult::Failed;
        }
        self.publish_preview(id, result, cx);
        self.start_preview(id, cx);
    }
    pub(crate) fn cancel_preview(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let Some(state) = self.previews.get_mut(&id) else {
            return;
        };
        state.cancel.store(true, Ordering::Release);
        state.latest = None;
        state.retired = true;
        let running = state.running;
        let blocked = state.blocked;
        if let Some(operation) = running {
            for pending in self
                .entries
                .values_mut()
                .filter(|pending| pending.operation == operation)
            {
                if !pending.cancelled {
                    pending.cancelled = true;
                    let command = preview_control(
                        pending.window_id,
                        pending.generation,
                        operation,
                        ArchiveEntryAction::Cancel,
                        None,
                    );
                    let _ = self
                        .platform
                        .send_command_correlated(PlatformCommand::ArchiveEntry(command), operation);
                }
            }
            self.dismiss_preview_password(operation, cx);
        } else if !blocked {
            self.previews.remove(&id);
        }
    }
    pub(super) fn publish_preview(&self, id: Uuid, result: PreviewResult, cx: &mut Context<Self>) {
        let (Some(state), Some(slot)) = (self.previews.get(&id), self.windows.get(&id)) else {
            return;
        };
        let selection_generation = state.selection_generation;
        let row = state.row.clone();
        let generation = slot.generation;
        let browser = slot.browser.clone();
        let window = slot.window;
        let view = slot.view.clone();
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let current = owner
                .read_with(cx, |app, _| {
                    app.windows.get(&id).is_some_and(|slot| {
                        slot.generation == generation && slot.browser == browser
                    }) && app.previews.get(&id).is_some_and(|state| {
                        !state.retired
                            && state.selection_generation == selection_generation
                            && state.row == row
                    })
                })
                .unwrap_or(false);
            if current {
                let _ = window.update(cx, |_, window, cx| {
                    let _ = view.update(cx, |view, cx| {
                        if view.preview_selection_matches(selection_generation, &row) {
                            view.preview_result(selection_generation, row, result, window, cx);
                        }
                    });
                });
            }
        });
    }
    fn preview_current(&self, request: &PreviewRequest, cx: &App) -> bool {
        self.previews.get(&request.window_id).is_some_and(|state| {
            !state.retired
                && !state.cancel.load(Ordering::Acquire)
                && state.selection_generation == request.selection_generation
                && state.row.as_ref() == Some(&request.row)
        }) && self.windows.get(&request.window_id).is_some_and(|slot| {
            slot.generation == request.generation
                && slot.browser.as_ref() == Some(&request.browser)
                && slot
                    .view
                    .read_with(cx, |view, _| {
                        view.preview_selection_matches(
                            request.selection_generation,
                            &Some(request.row.clone()),
                        ) && view.model.as_ref().is_some_and(|model| {
                            model.snapshot == request.browser.snapshot_id
                                && model.selected.len() == 1
                                && model.selected.contains(&request.row)
                        })
                    })
                    .unwrap_or(false)
        })
    }
    pub(super) fn start_preview(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let Some(request) = self
            .previews
            .get(&id)
            .filter(|state| state.running.is_none() && !state.blocked)
            .and_then(|state| state.latest.clone())
        else {
            return;
        };
        // A cancelled predecessor's flag is not the new request's cancellation flag.
        if let Some(state) = self.previews.get_mut(&id) {
            state.cancel = Arc::new(AtomicBool::new(false));
        }
        if !self.preview_current(&request, cx) {
            return;
        }
        let RowId::Entry(snapshot, ordinal) = request.row.clone() else {
            return;
        };
        let Some(slot) = self.windows.get(&id) else {
            return;
        };
        let origin = slot
            .view
            .read_with(cx, |view, _| view.preview_connection())
            .unwrap_or(0);
        let operation = Uuid::new_v4();
        let receipt = Uuid::new_v4();
        let action = match request.kind {
            PreviewKind::Image => ArchiveEntryAction::Preview,
            PreviewKind::Text(_) => ArchiveEntryAction::PreviewText,
        };
        let command = ArchiveEntryCommand {
            version: 1,
            operation_id: operation,
            session_id: id,
            generation: request.generation,
            action,
            path: Some(slot.path.to_string_lossy().into_owned()),
            codepage: slot.view.upgrade().map_or(0, |view| view.read(cx).codepage),
            ordinal: Some(ordinal),
            browser: Some(request.browser.clone()),
            lease_id: None,
            depth: slot.depth,
            copy_motw: Some(self.settings.copy_motw),
            password_interaction: true,
            temp_policy: Some(ArchiveTempPolicy {
                base_directory: self.settings.temp.base_directory.clone(),
                retention_hours: self.settings.temp.retention_hours,
                cleanup_on_child_exit: self.settings.temp.cleanup_on_child_exit,
            }),
        };
        if command.validate().is_err() {
            self.publish_preview(id, PreviewResult::Failed, cx);
            return;
        }
        self.previews.get_mut(&id).expect("preview state").running = Some(operation);
        self.entries.insert(
            receipt,
            PendingEntry {
                window_id: id,
                operation,
                generation: request.generation,
                snapshot,
                nested: false,
                depth: 0,
                started: request.started,
                cancelled: false,
            },
        );
        self.preview_entries.insert(receipt, request.clone());
        tracing::info!(event="archive.preview.accepted",correlation_id=%operation,"archive passive preview accepted");
        if self
            .platform
            .send_archive_preview(command, receipt, origin)
            .is_err()
        {
            self.entries.remove(&receipt);
            self.preview_entries.remove(&receipt);
            self.complete_preview(
                operation,
                request,
                PreviewResult::Disconnected,
                "not_submitted",
                cx,
            );
        }
    }
    fn preview_materialized(
        &mut self,
        pending: PendingEntry,
        request: PreviewRequest,
        outer: &nxtools_protocol::PlatformCommandResult,
        cx: &mut Context<Self>,
    ) {
        let parsed = outer
            .data
            .as_deref()
            .and_then(|data| serde_json::from_str::<ArchiveEntryResult>(data).ok());
        let current = !pending.cancelled && self.preview_current(&request, cx);
        if let Some(value) = parsed {
            let identity = value.version == 1
                && value.operation_id == pending.operation
                && value.session_id == pending.window_id
                && value.generation == pending.generation;
            let lease = value.lease_id.filter(|id| !id.is_nil()).map(|id| Lease {
                session: pending.window_id,
                generation: pending.generation,
                operation: pending.operation,
                id,
            });
            if outer.success
                && value.success
                && identity
                && value.cleanup_complete
                && current
                && let (Some(lease), Some(path)) = (lease.clone(), value.path)
            {
                tracing::info!(event="archive.preview.materialized",correlation_id=%pending.operation,elapsed_ms=pending.started.elapsed().as_millis() as u64,"archive preview materialized");
                self.decode_preview(pending.operation, request, lease, PathBuf::from(path), cx);
                return;
            }
            if let Some(lease) = lease {
                self.release(lease);
            }
            let result = if !current {
                PreviewResult::Cancelled
            } else if identity {
                match value.code {
                    Some(ArchiveReadError::Cancelled) => PreviewResult::PasswordCancelled,
                    Some(ArchiveReadError::ResourceLimit) => PreviewResult::ReadLimit,
                    Some(ArchiveReadError::SourceChanged) => PreviewResult::SourceChanged,
                    Some(
                        ArchiveReadError::CorruptArchive
                        | ArchiveReadError::AuthenticationOrDataError,
                    ) => PreviewResult::Corrupt,
                    Some(
                        ArchiveReadError::UnsupportedFormat
                        | ArchiveReadError::UnsupportedMethod
                        | ArchiveReadError::UnsupportedEncryption
                        | ArchiveReadError::UnsupportedOperation,
                    ) => PreviewResult::Unsupported,
                    Some(ArchiveReadError::Disconnected) => PreviewResult::Disconnected,
                    _ => PreviewResult::Failed,
                }
            } else {
                PreviewResult::Failed
            };
            tracing::info!(event="archive.preview.native_terminal",correlation_id=%pending.operation,native_error_code=outer.native_error_code,cleanup_complete=value.cleanup_complete,elapsed_ms=pending.started.elapsed().as_millis() as u64,"archive preview native terminal");
            if !value.cleanup_complete || !identity {
                // Keep a fail-closed reservation but no running thread/closed-window map leak.
                if let Some(state) = self.previews.get_mut(&pending.window_id) {
                    state.running = None;
                    state.latest = None;
                    state.blocked = true;
                }
                if current {
                    self.publish_preview(pending.window_id, result, cx);
                }
                if !self.windows.contains_key(&pending.window_id) {
                    self.previews.remove(&pending.window_id);
                }
                tracing::warn!(event="archive.preview.terminal",correlation_id=%pending.operation,outcome="cleanup_unconfirmed",elapsed_ms=pending.started.elapsed().as_millis() as u64,"archive preview replacement blocked");
                return;
            }
            let source_changed = identity
                && value.code == Some(ArchiveReadError::SourceChanged)
                && self.windows.get(&pending.window_id).is_some_and(|slot| {
                    slot.generation == request.generation
                        && slot.browser.as_ref() == Some(&request.browser)
                });
            if source_changed {
                if let Some(slot) = self.windows.get_mut(&pending.window_id) {
                    slot.browser = None;
                }
                if let Some(state) = self.previews.get_mut(&pending.window_id) {
                    state.latest = None;
                }
                self.publish_preview(pending.window_id, PreviewResult::SourceChanged, cx);
                self.fail_view(
                    pending.window_id,
                    "源文件已改变；保留的视图仅供查看，请刷新后再操作。".into(),
                    cx,
                );
            }
            let outcome = match &result {
                PreviewResult::PasswordCancelled => "password_cancelled",
                PreviewResult::TooLarge => "too_large",
                PreviewResult::ReadLimit => "archive_read_limit",
                PreviewResult::SourceChanged => "source_changed",
                PreviewResult::Corrupt => "corrupt",
                PreviewResult::Unsupported => "unsupported",
                PreviewResult::Disconnected => "disconnected",
                PreviewResult::Cancelled => "superseded",
                _ => "native_failure",
            };
            self.complete_preview(pending.operation, request, result, outcome, cx);
        } else {
            if let Some(state) = self.previews.get_mut(&pending.window_id) {
                state.running = None;
                state.latest = None;
                state.blocked = true;
            }
            if !self.windows.contains_key(&pending.window_id) {
                self.previews.remove(&pending.window_id);
            }
            if current {
                self.publish_preview(pending.window_id, PreviewResult::Failed, cx);
            }
            tracing::warn!(event="archive.preview.terminal",correlation_id=%pending.operation,outcome="invalid_reply_cleanup_unconfirmed",elapsed_ms=pending.started.elapsed().as_millis() as u64,"archive preview retired without replacement");
        }
    }
    fn decode_preview(
        &mut self,
        operation: Uuid,
        request: PreviewRequest,
        lease: Lease,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let cancellation = self
            .previews
            .get(&request.window_id)
            .expect("current preview")
            .cancel
            .clone();
        let kind = request.kind;
        let platform = self.platform.clone();
        #[cfg(test)]
        let barrier = self.preview_decode_barrier.take();
        let task = cx.background_executor().spawn(async move {
            let _permit = preview::acquire().await;
            #[cfg(test)]
            if let Some(barrier) = barrier { let _ = barrier.recv().await; }
            tracing::info!(event="archive.preview.decode_started",correlation_id=%operation,"archive preview decode started");
            preview::prepare(&path, kind, &cancellation)
        });
        cx.spawn(async move |this, cx| {
            let decoded = task.await;
            // Detached from the Browser: lease release still runs after Browser/app retirement.
            let release = preview_control(
                lease.session,
                lease.generation,
                lease.operation,
                ArchiveEntryAction::Release,
                Some(lease.id),
            );
            let _ =
                platform.send_command_correlated(PlatformCommand::ArchiveEntry(release), operation);
            let outcome = match &decoded {
                Ok(_) => "ready",
                Err(preview::PreviewFailure::TooLarge) => "too_large",
                Err(preview::PreviewFailure::TextLines) => "text_line_limit",
                Err(preview::PreviewFailure::ImageDimensions) => "image_dimension_limit",
                Err(preview::PreviewFailure::ImageMemory) => "image_memory_limit",
                Err(preview::PreviewFailure::DocumentLimit) => "document_limit",
                Err(preview::PreviewFailure::Unsupported) => "unsupported",
                Err(preview::PreviewFailure::Corrupt) => "corrupt",
                Err(preview::PreviewFailure::Cancelled) => "cancelled",
                Err(preview::PreviewFailure::Io) => "io_failure",
            };
            let result = match decoded {
                Ok(payload) => PreviewResult::Ready(payload),
                Err(failure) => failure.into(),
            };
            let _ = this.update(cx, |app, cx| {
                app.complete_preview(operation, request, result, outcome, cx)
            });
        })
        .detach();
    }
    fn complete_preview(
        &mut self,
        operation: Uuid,
        request: PreviewRequest,
        result: PreviewResult,
        outcome: &'static str,
        cx: &mut Context<Self>,
    ) {
        let current = self.preview_current(&request, cx);
        if let Some(state) = self.previews.get_mut(&request.window_id) {
            if state.running != Some(operation) {
                return;
            }
            state.running = None;
            if state
                .latest
                .as_ref()
                .is_some_and(|latest| latest.selection_generation == request.selection_generation)
            {
                state.latest = None;
            }
        } else {
            return;
        }
        tracing::info!(event="archive.preview.terminal",correlation_id=%operation,outcome=if current { outcome } else { "superseded" },elapsed_ms=request.started.elapsed().as_millis() as u64,"archive passive preview finished");
        if current {
            self.publish_preview(request.window_id, result, cx);
        }
        if !self.windows.contains_key(&request.window_id)
            || self
                .previews
                .get(&request.window_id)
                .is_some_and(|state| state.retired)
        {
            self.previews.remove(&request.window_id);
        } else {
            self.start_preview(request.window_id, cx);
        }
    }
    pub(super) fn previews_disconnected(&mut self, cx: &mut Context<Self>) {
        let operations = self.preview_prompts.keys().copied().collect::<Vec<_>>();
        for operation in operations {
            self.dismiss_preview_password(operation, cx);
        }
        for (id, state) in &mut self.previews {
            state.cancel.store(true, Ordering::Release);
            state.latest = None;
            if let Some(slot) = self.windows.get(id) {
                let view = slot.view.clone();
                let window = slot.window;
                let row = state.row.clone();
                let token = state.selection_generation;
                cx.defer(move |cx| {
                    let _ = window.update(cx, |_, window, cx| {
                        let _ = view.update(cx, |view, cx| {
                            if view.preview_selection_matches(token, &row) {
                                view.preview_result(
                                    token,
                                    row,
                                    PreviewResult::Disconnected,
                                    window,
                                    cx,
                                );
                            }
                        });
                    });
                });
            }
            state.retired = true;
        }
        // Native reservations are dead with the pipe; decode reservations retain their real worker.
        let native = self
            .preview_entries
            .values()
            .map(|request| request.window_id)
            .collect::<Vec<_>>();
        for id in native {
            self.previews.remove(&id);
        }
        self.preview_entries.clear();
        self.previews.retain(|_, state| state.running.is_some());
    }
}
fn preview_control(
    session: Uuid,
    generation: u64,
    operation: Uuid,
    action: ArchiveEntryAction,
    lease_id: Option<Uuid>,
) -> ArchiveEntryCommand {
    ArchiveEntryCommand {
        version: 1,
        operation_id: operation,
        session_id: session,
        generation,
        action,
        path: None,
        codepage: 0,
        ordinal: None,
        browser: None,
        lease_id,
        depth: 0,
        copy_motw: None,
        password_interaction: false,
        temp_policy: None,
    }
}

pub(super) struct PreviewPrompt {
    window: AnyWindowHandle,
    alive: Rc<std::cell::Cell<bool>>,
    scope: Rc<std::cell::RefCell<Option<gpui_kit::FocusHandle>>>,
    input: Entity<gpui_kit::component::input::InputState>,
}
impl ArchiveApplication {
    // Same private ticket/reply owner, with explicit dialog focus ownership. Never pop an
    // unrelated top dialog just because a password map entry was removed.
    pub(super) fn preview_password_prompt(
        &mut self,
        prompt: nxtools_protocol::archive_password::ArchivePasswordPrompt,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(request) = self.preview_entries.iter().find_map(|(receipt, request)| {
            self.entries
                .get(receipt)
                .filter(|pending| pending.operation == prompt.operation_id)
                .map(|_| request.clone())
        }) else {
            return false;
        };
        if !self.preview_current(&request, cx)
            || prompt.session_id != request.window_id
            || prompt.generation != request.generation
        {
            return true;
        }
        if self.passwords.get(&prompt.operation_id) == Some(&prompt)
            && self.preview_prompts.contains_key(&prompt.operation_id)
        {
            return true;
        }
        self.dismiss_preview_password(prompt.operation_id, cx);
        self.passwords.insert(prompt.operation_id, prompt.clone());
        if !self.tab_is_active(prompt.session_id, cx) {
            return true;
        }
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let window = owner
                .read_with(cx, |app, cx| {
                    (app.passwords.get(&prompt.operation_id) == Some(&prompt)
                        && app.preview_current(&request, cx)
                        && app.tab_is_active(prompt.session_id, cx))
                    .then(|| app.windows.get(&request.window_id).map(|slot| slot.window))
                    .flatten()
                })
                .ok()
                .flatten();
            let Some(source) = window else {
                return;
            };
            let alive = Rc::new(std::cell::Cell::new(true));
            let scope = Rc::new(std::cell::RefCell::new(None::<gpui_kit::FocusHandle>));
            let input_for_retirement = source.update(cx, |_, window, cx| {
                let input = cx
                    .new(|cx| gpui_kit::component::input::InputState::new(window, cx).masked(true));
                let retire_input = input.clone();
                let dialog_alive = alive.clone();
                let dialog_scope = scope.clone();
                let dialog_owner = owner.clone();
                let dialog_prompt = prompt.clone();
                window.open_dialog(cx, move |dialog, window, cx| {
                    // A covered cancelled prompt closes only when its own scope is top/focused.
                    if !dialog_alive.get()
                        && dialog_scope
                            .borrow()
                            .as_ref()
                            .is_some_and(|scope| scope.contains_focused(window, cx))
                    {
                        let scope = dialog_scope.clone();
                        window.defer(cx, move |window, cx| {
                            if scope
                                .borrow()
                                .as_ref()
                                .is_some_and(|scope| scope.contains_focused(window, cx))
                                && window.has_active_dialog(cx)
                            {
                                window.close_dialog(cx);
                            }
                        });
                    }
                    let confirm_owner = dialog_owner.clone();
                    let cancel_owner = dialog_owner.clone();
                    let confirm_prompt = dialog_prompt.clone();
                    let cancel_prompt = dialog_prompt.clone();
                    let confirm_input = input.clone();
                    let cancel_input = input.clone();
                    let confirm_alive = dialog_alive.clone();
                    let cancel_alive = dialog_alive.clone();
                    let close_owner = dialog_owner.clone();
                    let close_prompt = dialog_prompt.clone();
                    let close_input = input.clone();
                    let close_alive = dialog_alive.clone();
                    dialog
                        .title(if dialog_prompt.error_code == "WrongPassword" {
                            "密码错误，请重试"
                        } else {
                            "归档预览需要密码"
                        })
                        .child(
                            gpui_kit::component::input::Input::new(&input).aria_label("归档密码"),
                        )
                        .footer(
                            DialogFooter::new()
                                .child(DialogClose::new().trigger(|button| button.label("取消")))
                                .child(DialogAction::new().child(
                                    Button::new("archive-preview-password-ok").label("确定"),
                                )),
                        )
                        .on_ok(move |_, window, cx| {
                            if !confirm_alive.replace(false) {
                                return true;
                            }
                            let value = confirm_input.read(cx).value();
                            let secret = nxtools_protocol::archive_password::PrivatePassword::new(
                                value.as_ref(),
                            );
                            confirm_input.update(cx, |input, cx| input.set_value("", window, cx));
                            let owner = confirm_owner.clone();
                            let prompt = confirm_prompt.clone();
                            cx.defer(move |cx| {
                                let _ = owner.update(cx, |app, cx| {
                                    app.preview_password_reply(prompt, Some(secret), cx)
                                });
                            });
                            true
                        })
                        .on_cancel(move |_, window, cx| {
                            if !cancel_alive.replace(false) {
                                return true;
                            }
                            cancel_input.update(cx, |input, cx| input.set_value("", window, cx));
                            let owner = cancel_owner.clone();
                            let prompt = cancel_prompt.clone();
                            cx.defer(move |cx| {
                                let _ = owner.update(cx, |app, cx| {
                                    app.preview_password_reply(prompt, None, cx)
                                });
                            });
                            true
                        })
                        .on_close(move |_, window, cx| {
                            if close_alive.replace(false) {
                                close_input.update(cx, |input, cx| input.set_value("", window, cx));
                                let owner = close_owner.clone();
                                let prompt = close_prompt.clone();
                                cx.defer(move |cx| {
                                    let _ = owner.update(cx, |app, cx| {
                                        app.preview_password_reply(prompt, None, cx)
                                    });
                                });
                            }
                        })
                });
                *scope.borrow_mut() = window.focused(cx);
                retire_input
            });
            let Ok(input) = input_for_retirement else {
                return;
            };
            let _ = owner.update(cx, |app, _| {
                app.preview_prompts.insert(
                    prompt.operation_id,
                    PreviewPrompt {
                        window: source,
                        alive,
                        scope,
                        input,
                    },
                );
            });
        });
        true
    }
    fn preview_password_reply(
        &mut self,
        prompt: nxtools_protocol::archive_password::ArchivePasswordPrompt,
        secret: Option<nxtools_protocol::archive_password::PrivatePassword>,
        cx: &mut Context<Self>,
    ) {
        self.preview_prompts.remove(&prompt.operation_id);
        let current = self
            .preview_entries
            .iter()
            .find_map(|(receipt, request)| {
                self.entries
                    .get(receipt)
                    .filter(|pending| {
                        pending.operation == prompt.operation_id && !pending.cancelled
                    })
                    .map(|_| request)
            })
            .is_some_and(|request| self.preview_current(request, cx));
        if current {
            self.password_reply(prompt, secret, cx);
        } else {
            self.passwords.remove(&prompt.operation_id);
        }
    }
    pub(super) fn suspend_preview_password(&mut self, operation: Uuid, cx: &mut Context<Self>) {
        let ticket = self.passwords.get(&operation).cloned();
        self.dismiss_preview_password(operation, cx);
        if let Some(ticket) = ticket {
            self.passwords.insert(operation, ticket);
        }
    }
    fn dismiss_preview_password(&mut self, operation: Uuid, cx: &mut Context<Self>) {
        self.passwords.remove(&operation);
        let Some(prompt) = self.preview_prompts.remove(&operation) else {
            return;
        };
        prompt.alive.set(false);
        cx.defer(move |cx| {
            let _ = prompt.window.update(cx, |_, window, cx| {
                prompt
                    .input
                    .update(cx, |input, cx| input.set_value("", window, cx));
                if prompt
                    .scope
                    .borrow()
                    .as_ref()
                    .is_some_and(|scope| scope.contains_focused(window, cx))
                    && window.has_active_dialog(cx)
                {
                    window.close_dialog(cx);
                }
            });
        });
    }
}
