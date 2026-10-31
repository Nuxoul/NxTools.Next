//! Original ArchiveApplication routing for advisory form observations, never an execution queue.
use super::*;
use crate::archive::space::{self, Metadata, Plan};
use nxtools_protocol::PlatformCommand;
use nxtools_protocol::archive_job::ArchiveJobRequest;
use nxtools_protocol::archive_space::{ArchiveSpaceCommand, ArchiveSpaceObservation};

pub(super) struct Pending {
    command: ArchiveSpaceCommand,
    request: ArchiveJobRequest,
    metadata: Option<Metadata>,
    epoch: u64,
    started: Instant,
    _deadline: Task<()>,
}
impl ArchiveApplication {
    /// Retire UI bookkeeping only. Native observation slots stay owned until actual cleanup.
    pub(super) fn retire_form_space(&mut self, form: Uuid) {
        self.jobs.spaces.retain(|operation, pending| {
            if pending.command.session_id != form { return true; }
            tracing::info!(event="archive.space.finished",correlation_id=%operation,session_id=%form,outcome="form_retired",elapsed_ms=pending.started.elapsed().as_millis() as u64,"archive advisory observation retired without native cancellation");
            false
        });
    }
    pub(in crate::archive) fn request_space(
        &mut self,
        form: Uuid,
        generation: u64,
        epoch: u64,
        request: ArchiveJobRequest,
        expected_target: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if !self.jobs.forms.get(&form).is_some_and(|slot| {
            slot.generation == generation
                && slot
                    .view
                    .read_with(cx, |form, cx| {
                        form.space_request_current(epoch, &request, cx)
                    })
                    .unwrap_or(false)
        }) {
            return;
        }
        if self
            .jobs
            .spaces
            .values()
            .any(|pending| pending.command.session_id == form)
        {
            if let Some(slot) = self.jobs.forms.get(&form) {
                let view = slot.view.clone();
                cx.defer(move |cx| {
                    let _ = view.update(cx, |form, _| form.wait_for_space_request(epoch));
                });
            }
            return;
        }
        if self.jobs.spaces.len() >= 8 {
            self.deliver_space(
                form,
                generation,
                epoch,
                Err("空间检查仍在完成，请稍后重新检查。".into()),
                cx,
            );
            return;
        }
        let metadata = self
            .windows
            .values()
            .find(|slot| {
                request
                    .paths
                    .first()
                    .is_some_and(|path| slot.path == std::path::Path::new(path))
            })
            .and_then(|slot| {
                let browser = slot.browser.clone()?;
                slot.view
                    .read_with(cx, |view, _| {
                        if view.generation != slot.generation || view.codepage != request.codepage {
                            return None;
                        }
                        let model = view.model.as_ref()?;
                        if model.snapshot != browser.snapshot_id || model.entries.len() > 65_536 {
                            return None;
                        }
                        let bytes = model.entries.iter().try_fold(0usize, |sum, entry| {
                            sum.checked_add(entry.name.len() + 256)
                        })?;
                        if bytes > 16 << 20 {
                            return None;
                        }
                        Some(Metadata {
                            browser,
                            entries: model.entries.clone(),
                        })
                    })
                    .ok()
                    .flatten()
            });
        let operation = Uuid::new_v4();
        let command = ArchiveSpaceCommand {
            version: 1,
            operation_id: operation,
            session_id: form,
            generation,
            source: request.paths.first().cloned().unwrap_or_default(),
            destination: request.destination.clone().unwrap_or_default(),
            expected_source: metadata
                .as_ref()
                .map(|data| data.browser.source.native.clone()),
            expected_target,
        };
        if command.validate().is_err() {
            self.deliver_space(
                form,
                generation,
                epoch,
                Err("目标或来源路径无效，请重新选择本地文件夹。".into()),
                cx,
            );
            return;
        }
        let deadline = cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(7))
                .await;
            let _ = owner.update(cx, |app, cx| {
                app.space_result(
                    operation,
                    &nxtools_protocol::PlatformCommandResult {
                        success: false,
                        error: Some("space_timeout".into()),
                        native_error_code: None,
                        data: None,
                    },
                    cx,
                )
            });
        });
        self.jobs.spaces.insert(
            operation,
            Pending {
                command: command.clone(),
                request,
                metadata,
                epoch,
                started: Instant::now(),
                _deadline: deadline,
            },
        );
        tracing::info!(event="archive.space.accepted",correlation_id=%operation,session_id=%form,"archive space observation accepted");
        if self
            .platform
            .send_command_with_id_correlated(
                PlatformCommand::ArchiveSpace(command),
                operation,
                operation,
            )
            .is_err()
        {
            self.space_result(
                operation,
                &nxtools_protocol::PlatformCommandResult {
                    success: false,
                    error: Some("disconnected".into()),
                    native_error_code: None,
                    data: None,
                },
                cx,
            );
        }
    }
    pub(super) fn space_result(
        &mut self,
        id: Uuid,
        result: &nxtools_protocol::PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(pending) = self.jobs.spaces.remove(&id) else {
            return false;
        };
        let metadata_current = pending.metadata.as_ref().is_none_or(|metadata| {
            self.windows.values().any(|slot| {
                slot.browser.as_ref() == Some(&metadata.browser)
                    && slot
                        .view
                        .read_with(cx, |view, _| {
                            view.generation == slot.generation
                                && view.codepage == pending.request.codepage
                                && view.model.as_ref().is_some_and(|model| {
                                    model.snapshot == metadata.browser.snapshot_id
                                })
                        })
                        .unwrap_or(false)
            })
        });
        let outcome = if result.success {
            result
                .data
                .as_deref()
                .filter(|data| data.len() <= 4096)
                .and_then(|data| serde_json::from_str::<ArchiveSpaceObservation>(data).ok())
                .filter(|observed| {
                    metadata_current
                        && observed.validate(&pending.command).is_ok()
                        && observed.source_matches
                })
                .map(|observation| {
                    let estimate = space::estimate_observed(
                        &pending.request,
                        pending.metadata.as_ref(),
                        &observation,
                    );
                    Plan {
                        observation,
                        estimate,
                        concurrent_jobs: self
                            .jobs
                            .records
                            .values()
                            .filter(|record| record.terminal.is_none())
                            .count(),
                    }
                })
                .ok_or_else(|| "容量结果无效或源身份已变化，请重新检查。".into())
        } else {
            let message = match result.error.as_deref() {
                Some("source_changed") => "来源已变化，请重新打开压缩包后检查。",
                Some("target_changed") => "目标目录或卷已变化；旧计划失效，请重新检查。",
                Some("source_unavailable") => "来源不存在或卷成员不完整，请重新选择压缩包。",
                Some("source_denied") => "权限不足，无法验证源压缩包。",
                Some("source_busy" | "target_busy") => {
                    "来源或目标正在被其他操作占用，请稍后重新检查。"
                }
                Some("source_wrong_kind" | "target_wrong_kind") => {
                    "来源或目标类型已变化，请重新选择普通文件/目录。"
                }
                Some("resource_limit") => {
                    "预检超出原有界源/元数据观察范围，需求仍未知；未提升执行预算。"
                }
                Some("target_denied" | "open_component") if result.native_error_code == Some(5) => {
                    "权限不足，无法观察目标文件夹容量。"
                }
                Some("reparse_component") => {
                    "目标包含重解析点；未放宽原本地路径限制，请选择普通本地目录。"
                }
                Some("space_timeout" | "space_busy_or_timeout") => {
                    "空间检查繁忙或超时，请稍后重新检查；未预留磁盘空间。"
                }
                Some("disconnected") => "原生连接不可用，请连接后重新检查。",
                _ => "无法读取容量或验证来源/目标，请检查路径、权限后重新检查。",
            };
            Err(match result.native_error_code {
                Some(code) => format!("{message}（Windows {code}）"),
                None => message.into(),
            })
        };
        tracing::info!(event="archive.space.finished",correlation_id=%id,success=outcome.is_ok(),elapsed_ms=pending.started.elapsed().as_millis() as u64,native_error_code=?result.native_error_code,"archive space observation finished");
        self.deliver_space(
            pending.command.session_id,
            pending.command.generation,
            pending.epoch,
            outcome,
            cx,
        );
        true
    }
    fn deliver_space(
        &self,
        id: Uuid,
        generation: u64,
        epoch: u64,
        result: Result<Plan, String>,
        cx: &mut Context<Self>,
    ) {
        let Some(slot) = self
            .jobs
            .forms
            .get(&id)
            .filter(|slot| slot.generation == generation)
        else {
            return;
        };
        let view = slot.view.clone();
        cx.defer(move |cx| {
            let _ = view.update(cx, |form, cx| form.apply_space(epoch, result, cx));
        });
    }
    pub(super) fn invalidate_space_source(&self, id: Uuid, cx: &mut Context<Self>) {
        let Some(path) = self.windows.get(&id).map(|slot| slot.path.clone()) else {
            return;
        };
        let forms = self
            .jobs
            .forms
            .values()
            .map(|slot| slot.view.clone())
            .collect::<Vec<_>>();
        for view in forms {
            let path = path.clone();
            cx.defer(move |cx| {
                let _ = view.update(cx, |form, cx| form.invalidate_space_source(&path, cx));
            });
        }
    }
    pub(super) fn space_disconnected(&mut self, cx: &mut Context<Self>) {
        let ids = self.jobs.spaces.keys().copied().collect::<Vec<_>>();
        for id in ids {
            self.space_result(
                id,
                &nxtools_protocol::PlatformCommandResult {
                    success: false,
                    error: Some("disconnected".into()),
                    native_error_code: None,
                    data: None,
                },
                cx,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{TestAppContext, VisualTestContext};
    use nxtools_protocol::{Message, PlatformCommandResult};
    fn draw(surface: &mut VisualTestContext) {
        surface.run_until_parked();
        surface.update(|window, cx| {
            window.activate_window();
            window.draw(cx).clear(cx);
        });
        surface.run_until_parked();
    }
    #[gpui_kit::test]
    fn hz1_original_application_receipt_matches_form_and_unknown_remains_honest(
        cx: &mut TestAppContext,
    ) {
        let (owner, endpoint) = super::super::tests::setup(cx);
        owner.update(cx, |app, cx| {
            app.open_extract_form(r"C:\owned\source.zip".into(), None, cx)
        });
        cx.run_until_parked();
        let (window, id) = owner.read_with(cx, |app, _| {
            let (id, slot) = app.jobs.forms.iter().next().expect("original form");
            (slot.window, *id)
        });
        let wire = endpoint
            .try_receive_command_for_contract_test()
            .expect("original capacity dispatch");
        let Message::PlatformCommand(PlatformCommand::ArchiveSpace(command)) = wire.message else {
            panic!("capacity command");
        };
        assert_eq!(command.session_id, id);
        assert_eq!(wire.correlation_id, Some(command.operation_id));
        let observation = ArchiveSpaceObservation {
            version: 1,
            operation_id: command.operation_id,
            session_id: id,
            generation: command.generation,
            available_bytes: 8192,
            allocation_bytes: 4096,
            target_identity: "v1:target".into(),
            volume_identity: "v1:volume".into(),
            destination_missing: true,
            missing_destination_directories: Some(1),
            source_matches: true,
            source_set: None,
        };
        let result = PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: Some(serde_json::to_string(&observation).expect("observation")),
        };
        owner.update(cx, |app, cx| {
            assert!(app.job_notification(
                &PlatformNotification::CommandResult {
                    correlation_id: wire.id,
                    result: result.clone()
                },
                cx
            ))
        });
        let mut surface = VisualTestContext::from_window(window, cx);
        draw(&mut surface);
        assert!(surface.debug_bounds("archive-space-preflight").is_some());
        let view = owner.read_with(&surface, |app, _| {
            app.jobs.forms[&id].view.upgrade().expect("form")
        });
        assert_eq!(
            view.read_with(&surface, |form, _| form.space_state_for_test()),
            (false, Some((8192, None))),
            "observed free space retained, missing metadata not zero"
        );
        // Same original receipt is consumed once; duplicate cannot write a replacement form.
        owner.update(&mut surface, |app, cx| {
            assert!(!app.space_result(wire.id, &result, cx))
        });
        assert!(
            owner.read_with(&surface, |app, _| app.jobs.spaces.is_empty()
                && app.jobs.records.is_empty())
        );
        surface.update(|window, cx| view.update(cx, |form, cx| form.close(window, cx)));
        surface.run_until_parked();
    }
    #[gpui_kit::test]
    fn hz1_original_request_timeout_and_disconnect_clear_pending(cx: &mut TestAppContext) {
        let (owner, endpoint) = super::super::tests::setup(cx);
        owner.update(cx, |app, cx| {
            app.open_extract_form(r"C:\owned\source.zip".into(), None, cx)
        });
        cx.run_until_parked();
        let wire = endpoint
            .try_receive_command_for_contract_test()
            .expect("capacity dispatch");
        let Message::PlatformCommand(PlatformCommand::ArchiveSpace(command)) = wire.message else {
            panic!("capacity");
        };
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(8));
        cx.run_until_parked();
        assert!(
            owner.read_with(cx, |app, _| app.jobs.spaces.is_empty()),
            "deadline retires pending request"
        );
        owner.update(cx, |app, cx| app.space_disconnected(cx));
        let (window, view) = owner.read_with(cx, |app, _| {
            let slot = &app.jobs.forms[&command.session_id];
            (slot.window, slot.view.clone())
        });
        window
            .update(cx, |_, window, cx| {
                view.update(cx, |form, cx| form.close(window, cx))
            })
            .expect("form window")
            .expect("form");
        cx.run_until_parked();
    }
}

#[cfg(test)]
mod known_snapshot_tests {
    use super::*;
    use gpui_kit::TestAppContext;
    use nxtools_protocol::{Message, PlatformCommandResult, archive_browser::*};
    #[gpui_kit::test]
    fn hz1_original_browser_all_and_selected_snapshot_reach_real_form_plan(
        cx: &mut TestAppContext,
    ) {
        let (owner, endpoint) = super::super::tests::setup(cx);
        let path = PathBuf::from(r"C:\owned\known.zip");
        owner.update(cx, |app, cx| {
            app.open_canonical(path.clone(), 0, vec![], cx)
        });
        cx.run_until_parked();
        let (session, generation, snapshot) = owner.update(cx, |app, cx| {
            let (&id, slot) = app.windows.iter_mut().next().expect("original browser");
            let snapshot = Uuid::new_v4();
            slot.browser = Some(ArchiveBrowserContext {
                snapshot_id: snapshot,
                format: "Zip".into(),
                comment: String::new(),
                source: ArchiveSourceBinding {
                    native: "v1:known-source".into(),
                    digest: [0; 32],
                    size: 999_999,
                    modified_unix_ns: 0,
                },
                sevenz_edit: None,
                tar_edit: None,
            });
            let view = slot.view.clone();
            let entries = vec![
                serde_json::json!({"ordinal":0,"name":"a/","kind":"Directory","size":null}),
                serde_json::json!({"ordinal":1,"name":"a/one","kind":"File","size":1}),
                serde_json::json!({"ordinal":2,"name":"b","kind":"File","size":4097}),
            ]
            .into_iter()
            .map(|mut value| {
                let object = value.as_object_mut().expect("entry");
                object.insert("packed_size".into(), serde_json::Value::Null);
                object.insert("encrypted".into(), false.into());
                object.insert("warning_codes".into(), serde_json::json!([]));
                object.insert("provenance".into(), "Utf8Flag".into());
                object.insert("codepage".into(), 65001.into());
                serde_json::from_value(value).expect("native metadata")
            })
            .collect();
            slot.window
                .update(cx, |_, window, cx| {
                    view.update(cx, |view, cx| {
                        view.install(snapshot, entries, "Zip".into(), window, cx)
                    })
                })
                .expect("host")
                .expect("browser");
            (id, slot.generation, snapshot)
        });
        while endpoint.try_receive_command_for_contract_test().is_ok() {}
        for (selection, expected) in [(None, 16384), (Some(vec![2]), 8192)] {
            owner.update(cx, |app, cx| {
                app.extract_browser(session, generation, snapshot, selection, cx)
            });
            cx.run_until_parked();
            let wire = endpoint
                .try_receive_command_for_contract_test()
                .expect("capacity command");
            let Message::PlatformCommand(PlatformCommand::ArchiveSpace(command)) = wire.message
            else {
                panic!("space");
            };
            assert_eq!(command.expected_source.as_deref(), Some("v1:known-source"));
            let observed = ArchiveSpaceObservation {
                version: 1,
                operation_id: command.operation_id,
                session_id: command.session_id,
                generation: command.generation,
                available_bytes: 32768,
                allocation_bytes: 4096,
                target_identity: "v1:target".into(),
                volume_identity: "v1:volume".into(),
                destination_missing: false,
                missing_destination_directories: Some(0),
                source_matches: true,
                source_set: None,
            };
            owner.update(cx, |app, cx| {
                app.space_result(
                    wire.id,
                    &PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some(serde_json::to_string(&observed).expect("wire")),
                    },
                    cx,
                );
            });
            cx.run_until_parked();
            let (window, view) = owner.read_with(cx, |app, _| {
                let slot = &app.jobs.forms[&command.session_id];
                (slot.window, slot.view.clone())
            });
            assert_eq!(
                view.read_with(cx, |form, _| form.space_state_for_test())
                    .expect("form"),
                (false, Some((32768, Some(expected))))
            );
            window
                .update(cx, |_, window, cx| {
                    view.update(cx, |form, cx| form.close(window, cx))
                })
                .expect("window")
                .expect("form");
            cx.run_until_parked();
        }
        let host = owner.read_with(cx, |app, _| app.host.as_ref().expect("host").id);
        owner.update(cx, |app, cx| app.close_host(host, cx));
        cx.run_until_parked();
    }
}
