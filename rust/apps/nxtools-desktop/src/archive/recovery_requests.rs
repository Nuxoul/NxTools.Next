//! Object-scoped read-only discovery on the original application owner. No execution authority.
use super::*;
use nxtools_protocol::archive_recovery::{
    ArchiveRecoveryInspectCommand, ArchiveRecoveryInspection,
};
use nxtools_protocol::{PlatformCommand, PlatformCommandResult};

#[derive(Clone, Copy)]
enum Target {
    Form { epoch: u64 },
    Browser,
}
pub(super) struct Pending {
    command: ArchiveRecoveryInspectCommand,
    target: Target,
    connection: Option<u64>,
    started: Instant,
    _deadline: Task<()>,
}
impl ArchiveApplication {
    /// Retires only UI bookkeeping; native inspection retains its slot until actual return.
    pub(super) fn retire_recovery(&mut self, session: Uuid) {
        self.jobs.recoveries.retain(|id, pending| {
            if pending.command.session_id != session { return true; }
            tracing::info!(event="archive.recovery.discovery.finished", correlation_id=%id, outcome="retired", elapsed_ms=pending.started.elapsed().as_millis() as u64, "archive recovery discovery retired");
            false
        });
    }
    pub(in crate::archive) fn request_recovery(
        &mut self,
        form: Uuid,
        generation: u64,
        epoch: u64,
        source: String,
        destination: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let command = ArchiveRecoveryInspectCommand {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: form,
            generation,
            source,
            destination,
        };
        let target = Target::Form { epoch };
        if !self.recovery_target_current(&command, target, cx) {
            return;
        }
        self.retire_recovery(form);
        self.dispatch_recovery(command, target, cx);
    }
    pub(super) fn inspect_browser_recovery(
        &mut self,
        id: Uuid,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        let Some(slot) = self.windows.get(&id).filter(|slot| {
            slot.generation == generation && slot.depth == 0 && slot.leases.is_empty()
        }) else {
            return;
        };
        let command = ArchiveRecoveryInspectCommand {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: id,
            generation,
            source: slot.path.to_string_lossy().into_owned(),
            destination: None,
        };
        self.dispatch_recovery(command, Target::Browser, cx);
    }
    fn dispatch_recovery(
        &mut self,
        command: ArchiveRecoveryInspectCommand,
        target: Target,
        cx: &mut Context<Self>,
    ) {
        if command.validate().is_err() || self.jobs.recoveries.len() >= 8 {
            self.deliver_recovery(
                &command,
                target,
                Err("恢复记录暂时无法检查，请稍后重新检查。".into()),
                cx,
            );
            return;
        }
        let operation = command.operation_id;
        let deadline = cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(7))
                .await;
            let _ = owner.update(cx, |app, cx| {
                app.recovery_result(
                    operation,
                    &PlatformCommandResult {
                        success: false,
                        error: Some("recovery_timeout".into()),
                        native_error_code: None,
                        data: None,
                    },
                    cx,
                );
            });
        });
        self.jobs.recoveries.insert(
            operation,
            Pending {
                command: command.clone(),
                target,
                connection: self.platform.connection_generation(),
                started: Instant::now(),
                _deadline: deadline,
            },
        );
        tracing::info!(event="archive.recovery.discovery.accepted",correlation_id=%operation, "archive recovery discovery accepted");
        if self
            .platform
            .send_command_with_id_correlated(
                PlatformCommand::ArchiveRecoveryInspect(command),
                operation,
                operation,
            )
            .is_err()
        {
            self.recovery_result(
                operation,
                &PlatformCommandResult {
                    success: false,
                    error: Some("disconnected".into()),
                    native_error_code: None,
                    data: None,
                },
                cx,
            );
        }
    }
    fn recovery_target_current(
        &self,
        command: &ArchiveRecoveryInspectCommand,
        target: Target,
        cx: &App,
    ) -> bool {
        match target {
            Target::Form { epoch } => {
                self.jobs
                    .forms
                    .get(&command.session_id)
                    .is_some_and(|slot| {
                        slot.generation == command.generation
                            && slot
                                .view
                                .read_with(cx, |form, cx| {
                                    form.recovery_request_current(
                                        epoch,
                                        &command.source,
                                        command.destination.as_deref(),
                                        cx,
                                    )
                                })
                                .unwrap_or(false)
                    })
            }
            Target::Browser => self.windows.get(&command.session_id).is_some_and(|slot| {
                slot.generation == command.generation
                    && slot.depth == 0
                    && slot.leases.is_empty()
                    && slot.path == std::path::Path::new(&command.source)
            }),
        }
    }
    pub(super) fn recovery_result(
        &mut self,
        id: Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(pending) = self.jobs.recoveries.remove(&id) else {
            return false;
        };
        let connection_current = pending.connection.is_some()
            && pending.connection == self.platform.connection_generation();
        let outcome = if result.success && connection_current {
            result
                .data
                .as_deref()
                .filter(|data| data.len() <= 32 * 1024)
                .and_then(|data| serde_json::from_str::<ArchiveRecoveryInspection>(data).ok())
                .filter(|value| value.validate(&pending.command).is_ok())
                .ok_or_else(|| "恢复记录身份无效或已变化，请重新检查。".into())
        } else {
            let reason = if connection_current {
                result.error.as_deref()
            } else {
                Some("disconnected")
            };
            let message = match reason {
                Some("recovery_not_found") => {
                    "此压缩包没有可用的空间回收记录。请确认选择的是原压缩包，并保留对应恢复日志。"
                }
                Some("recovery_missing_companion") => {
                    "找不到对应恢复日志。请将原恢复日志放回原压缩包所在文件夹后重新检查。"
                }
                Some("recovery_mismatch" | "recovery_corrupt") => {
                    "压缩包与恢复记录不匹配，或记录已损坏。请检查原压缩包和对应恢复日志。"
                }
                Some(
                    "recovery_busy"
                    | "source_busy"
                    | "recovery_busy_or_timeout"
                    | "recovery_timeout",
                ) => "恢复记录检查繁忙或超时，请稍后重新检查。",
                Some("recovery_denied") => "权限不足，无法读取原压缩包或恢复日志。",
                Some("recovery_source_unavailable") => {
                    "找不到原压缩包，请确认仍使用原文件和原文件夹。"
                }
                Some("recovery_path_invalid") => {
                    "来源或目标路径不符合恢复要求，请选择原来的本地文件夹。"
                }
                Some("disconnected") => "原生连接已断开，请连接后重新检查。",
                _ => "无法验证恢复记录，请检查原压缩包、原目标文件夹及恢复日志后重试。",
            };
            Err(match result.native_error_code {
                Some(code) => format!("{message}（Windows {code}）"),
                None => message.into(),
            })
        };
        tracing::info!(event="archive.recovery.discovery.finished",correlation_id=%id,success=outcome.is_ok(),elapsed_ms=pending.started.elapsed().as_millis() as u64,native_error_code=?result.native_error_code,"archive recovery discovery finished");
        if self.recovery_target_current(&pending.command, pending.target, cx) {
            self.deliver_recovery(&pending.command, pending.target, outcome, cx);
        }
        true
    }
    fn deliver_recovery(
        &self,
        command: &ArchiveRecoveryInspectCommand,
        target: Target,
        result: Result<ArchiveRecoveryInspection, String>,
        cx: &mut Context<Self>,
    ) {
        let generation = command.generation;
        match target {
            Target::Form { epoch } => {
                let Some(slot) = self
                    .jobs
                    .forms
                    .get(&command.session_id)
                    .filter(|slot| slot.generation == generation)
                else {
                    return;
                };
                let view = slot.view.clone();
                let window = slot.window;
                cx.defer(move |cx| {
                    let _ = window.update(cx, |_, window, cx| {
                        let _ = view.update(cx, |form, cx| {
                            if form.generation == generation {
                                form.apply_recovery(epoch, result, window, cx);
                            }
                        });
                    });
                });
            }
            Target::Browser => {
                let Ok(inspection) = result else {
                    return;
                };
                let Some(slot) = self
                    .windows
                    .get(&command.session_id)
                    .filter(|slot| slot.generation == generation)
                else {
                    return;
                };
                let view = slot.view.clone();
                let source = slot.path.clone();
                cx.defer(move |cx| {
                    let _ = view.update(cx, |view, cx| {
                        if view.generation == generation
                            && view.path == source
                            && matches!(
                                view.status,
                                BrowserStatus::Failed(_) | BrowserStatus::Stale(_)
                            )
                        {
                            view.recovery = Some(inspection);
                            cx.notify();
                        }
                    });
                });
            }
        }
    }
    pub(super) fn recovery_disconnected(&mut self, cx: &mut Context<Self>) {
        let ids = self.jobs.recoveries.keys().copied().collect::<Vec<_>>();
        for id in ids {
            self.recovery_result(
                id,
                &PlatformCommandResult {
                    success: false,
                    error: Some("disconnected".into()),
                    native_error_code: None,
                    data: None,
                },
                cx,
            );
        }
    }
    pub(in crate::archive) fn continue_reclamation_job(
        &mut self,
        id: Uuid,
        attempt: Uuid,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        let Some(record) = self.jobs.records.get(&id).filter(|record| {
            record.active_operation == attempt
                && record.command.generation == generation
                && record.can_continue_reclamation()
        }) else {
            return;
        };
        let Some(request) = record.command.request.as_ref() else {
            return;
        };
        let source = PathBuf::from(&request.paths[0]);
        let destination = request.destination.as_ref().map(PathBuf::from);
        self.open_reclamation_continue_form(source, destination, cx);
    }
    pub(in crate::archive) fn continue_reclamation_browser(
        &mut self,
        id: Uuid,
        generation: u64,
        recovery_id: Uuid,
        cx: &mut Context<Self>,
    ) {
        let context = self
            .windows
            .get(&id)
            .filter(|slot| {
                slot.generation == generation && slot.depth == 0 && slot.leases.is_empty()
            })
            .and_then(|slot| {
                slot.view
                    .read_with(cx, |view, _| {
                        let record = view.recovery.as_ref().filter(|record| {
                            record.recovery_id == recovery_id
                                && !record.completed
                                && view.generation == generation
                                && matches!(
                                    view.status,
                                    BrowserStatus::Failed(_) | BrowserStatus::Stale(_)
                                )
                        })?;
                        Some((
                            slot.path.clone(),
                            record.destination.as_ref().map(PathBuf::from),
                        ))
                    })
                    .ok()
                    .flatten()
            });
        self.finish_open(id, cx);
        if let Some((source, destination)) = context {
            self.open_reclamation_continue_form(source, destination, cx);
        }
    }
}
