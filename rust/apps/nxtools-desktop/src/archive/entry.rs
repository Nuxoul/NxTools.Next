//! Single-entry materialization and original-window Shell/nested lease ownership.
use super::*;
use crate::archive::model::RowId;
use gpui_kit::ParentElement;
use gpui_kit::component::{
    WindowExt,
    button::Button,
    dialog::{DialogAction, DialogClose, DialogFooter},
};
use nxtools_protocol::{
    OpenShellTargetCommand, PlatformCommand, ShellOpenAction, archive_browser::*,
};
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
        if self.entries.values().any(|pending| pending.window_id == id) {
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
        if self.entries.values().any(|p| p.window_id == id) {
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
        self.passwords.remove(&pending.operation);
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
                        .send_command_with_id(
                            PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                                target: path,
                                action: ShellOpenAction::Open,
                            }),
                            shell_id,
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
