//! Source-bound continuation state and presentation on the original operation form.
use super::*;
use nxtools_protocol::archive_recovery::ArchiveRecoveryInspection;

#[derive(Default)]
pub(super) struct RecoveryState {
    pub(super) active: bool,
    pub(super) epoch: u64,
    pub(super) pending: bool,
    pub(super) candidate_destination: Option<String>,
    pub(super) inspection: Option<ArchiveRecoveryInspection>,
    pub(super) error: Option<String>,
}

impl OperationForm {
    pub(super) fn begin_recovery(
        &mut self,
        destination: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.retired || self.submitted || !self.is_continuation() {
            return;
        }
        self.recovery.active = true;
        self.recovery.epoch = self.recovery.epoch.wrapping_add(1).max(1);
        self.recovery.pending = true;
        self.recovery.candidate_destination = destination.clone();
        self.recovery.inspection = None;
        self.recovery.error = None;
        self.set_recovery_candidate(destination.as_deref(), window, cx);
        self.revalidate(cx);

        let Some(source) = self.continuation_source() else {
            self.recovery.pending = false;
            self.recovery.error = Some("原压缩包路径无效，无法检查恢复记录。".into());
            self.revalidate(cx);
            return;
        };
        let epoch = self.recovery.epoch;
        let owner = self.owner.clone();
        let local = cx.entity().downgrade();
        let form_id = self.id;
        let generation = self.generation;
        let source_window = window.window_handle();
        cx.defer(move |cx| {
            if !local
                .read_with(cx, |form, cx| {
                    form.recovery_request_current(epoch, &source, destination.as_deref(), cx)
                })
                .unwrap_or(false)
            {
                return;
            }
            if owner
                .update(cx, |app, cx| {
                    app.request_recovery(form_id, generation, epoch, source, destination, cx)
                })
                .is_err()
            {
                let _ = source_window.update(cx, |_, window, cx| {
                    let _ = local.update(cx, |form, cx| {
                        form.apply_recovery(
                            epoch,
                            Err("压缩包操作所有者已关闭，请重新打开表单。".into()),
                            window,
                            cx,
                        )
                    });
                });
            }
        });
        cx.notify();
    }

    pub(super) fn is_continuation(&self) -> bool {
        matches!(&self.draft, Draft::Extract(draft) if draft.continue_reclamation)
    }

    fn continuation_source(&self) -> Option<String> {
        match &self.draft {
            Draft::Extract(draft) if draft.continue_reclamation => {
                draft.source.to_str().map(str::to_owned)
            }
            _ => None,
        }
    }

    fn recovery_other_busy(&self) -> bool {
        self.retired
            || self.submitted
            || self.enumerating
            || self.pending_picker.is_some()
            || self.pending_format.is_some()
            || self.reclamation_confirmation_pending
    }

    fn set_recovery_candidate(
        &mut self,
        destination: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.destination.update(cx, |input, cx| {
            input.set_value(destination.unwrap_or_default(), window, cx)
        });
        self.recovery_id
            .update(cx, |input, cx| input.set_value("", window, cx));
        if let Draft::Extract(draft) = &mut self.draft
            && draft.continue_reclamation
        {
            draft.destination = destination.unwrap_or_default().to_owned();
            draft.strategy = ArchiveDestination::Here;
            draft.conflict = ArchiveConflictPolicy::Ask;
            draft.selection = None;
            draft.selected_only = false;
            draft.single_file_folder = false;
            draft.remember = false;
            draft.remember_destination = false;
            draft.reclamation = true;
            draft.recovery_id.clear();
            draft.copy_motw = false;
        }
    }

    pub(in crate::archive) fn recovery_request_current(
        &self,
        epoch: u64,
        source: &str,
        destination: Option<&str>,
        _cx: &App,
    ) -> bool {
        self.recovery.active
            && self.recovery.pending
            && self.recovery.epoch == epoch
            && !self.recovery_other_busy()
            && self.continuation_source().as_deref() == Some(source)
            && self.recovery.candidate_destination.as_deref() == destination
    }

    pub(in crate::archive) fn apply_recovery(
        &mut self,
        epoch: u64,
        result: Result<ArchiveRecoveryInspection, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.recovery.active
            || !self.recovery.pending
            || self.recovery.epoch != epoch
            || self.recovery_other_busy()
        {
            return;
        }
        match result {
            Ok(inspection)
                if inspection.session_id == self.id && inspection.generation == self.generation =>
            {
                self.recovery.pending = false;
                self.recovery.error = None;
                self.recovery.inspection = Some(inspection.clone());
                if let Some(destination) = inspection.destination.as_deref() {
                    self.recovery.candidate_destination = Some(destination.to_owned());
                    self.destination.update(cx, |input, cx| {
                        input.set_value(destination.to_owned(), window, cx)
                    });
                }
                if !inspection.completed
                    && let Some(destination) = inspection.destination.as_deref()
                {
                    let recovery_id = inspection.recovery_id.to_string();
                    self.recovery_id.update(cx, |input, cx| {
                        input.set_value(recovery_id.clone(), window, cx)
                    });
                    if let Draft::Extract(draft) = &mut self.draft {
                        draft.destination = destination.to_owned();
                        draft.recovery_id = recovery_id;
                        draft.copy_motw = inspection.copy_motw;
                    }
                } else {
                    let candidate = if inspection.completed {
                        inspection.destination.clone()
                    } else {
                        None
                    };
                    self.recovery.candidate_destination = candidate.clone();
                    self.set_recovery_candidate(candidate.as_deref(), window, cx);
                    self.recovery.inspection = Some(inspection);
                }
            }
            Ok(_) => return,
            Err(message) => {
                self.recovery.pending = false;
                self.recovery.inspection = None;
                self.recovery.error = Some(message);
                let candidate = self.recovery.candidate_destination.clone();
                self.set_recovery_candidate(candidate.as_deref(), window, cx);
            }
        }
        self.revalidate(cx);
    }

    pub(super) fn retire_recovery(&mut self) {
        self.recovery.epoch = self.recovery.epoch.wrapping_add(1).max(1);
        self.recovery.pending = false;
        self.recovery.inspection = None;
        self.recovery.error = None;
    }

    pub(super) fn recovery_facts(&self) -> Option<(String, String, bool)> {
        let inspection = self.recovery.inspection.as_ref()?;
        if self.recovery.pending || inspection.completed {
            return None;
        }
        Some((
            inspection.destination.clone()?,
            inspection.recovery_id.to_string(),
            inspection.copy_motw,
        ))
    }

    pub(super) fn require_recovery_ready(&self) -> Result<(), String> {
        if !self.is_continuation() {
            return Ok(());
        }
        if self.recovery.pending {
            return Err("正在检查原压缩包的恢复记录，请稍候。".into());
        }
        if let Some(message) = &self.recovery.error {
            return Err(format!("恢复记录检查失败：{message}"));
        }
        let Some(inspection) = &self.recovery.inspection else {
            return Err("尚未识别可继续的恢复记录。".into());
        };
        if inspection.completed {
            return Err("恢复记录已完成，没有可继续的剩余解压。".into());
        }
        if inspection.destination.is_none() {
            return Err("未识别原目标文件夹，请选择原文件夹后重新检查。".into());
        }
        Ok(())
    }

    pub(super) fn recovery_submit_ready(&self) -> bool {
        self.require_recovery_ready().is_ok()
    }

    fn retry_recovery(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.recovery_other_busy() || self.recovery.pending {
            return;
        }
        self.begin_recovery(self.recovery.candidate_destination.clone(), window, cx);
    }

    fn recovery_missing_target(&self) -> bool {
        !self.recovery.pending
            && self
                .recovery
                .inspection
                .as_ref()
                .is_none_or(|inspection| !inspection.completed && inspection.destination.is_none())
    }

    pub(super) fn continuation_fields(&self, cx: &Context<Self>) -> gpui_kit::AnyElement {
        let completed = self
            .recovery
            .inspection
            .as_ref()
            .is_some_and(|inspection| inspection.completed);
        let source = Field::new().label("原压缩包").required(true).child(
            div()
                .debug_selector(|| "archive-recovery-source-group".into())
                .child(
                    InputGroup::new("archive-recovery-source-group")
                        .w_full()
                        .input(Input::new(&self.source).aria_label("原压缩包"))
                        .readonly(true),
                ),
        );
        let destination = Field::new().label("原目标文件夹").required(true).child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .min_w_0()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .debug_selector(|| "archive-recovery-destination-group".into())
                        .child(
                            InputGroup::new("archive-recovery-destination-group")
                                .w_full()
                                .input(Input::new(&self.destination).aria_label("原目标文件夹"))
                                .readonly(true),
                        ),
                )
                .when(self.recovery_missing_target(), |row| {
                    row.child(
                        self.picker_button(
                            "archive-recovery-destination-picker",
                            "选择原文件夹…",
                            ArchivePickerPurpose::Destination,
                            cx,
                        )
                        .with_size(Size::Medium),
                    )
                }),
        );
        let mut status = div()
            .debug_selector(|| "archive-recovery-status".into())
            .flex()
            .flex_col()
            .gap_2();
        if self.recovery.pending {
            status = status.child(
                Alert::info(
                    "archive-recovery-pending",
                    "正在只读检查恢复记录和原目标文件夹。",
                )
                .with_size(Size::Small),
            );
        } else if let Some(message) = &self.recovery.error {
            status = status
                .child(
                    Alert::error("archive-recovery-error", message.clone()).with_size(Size::Small),
                )
                .child(
                    Button::new("archive-recovery-retry")
                        .debug_selector(|| "archive-recovery-retry".into())
                        .label("重试检查")
                        .outline()
                        .with_size(Size::Small)
                        .disabled(self.recovery_other_busy())
                        .on_click(
                            cx.listener(|this, _, window, cx| this.retry_recovery(window, cx)),
                        ),
                );
        } else if let Some(inspection) = &self.recovery.inspection {
            if inspection.completed {
                status = status.child(
                    Alert::success(
                        "archive-recovery-completed",
                        "恢复记录已完成，不会再次提交解压。输出文件的当前状态尚未检查。",
                    )
                    .with_size(Size::Small),
                );
            } else if inspection.destination.is_none() {
                status = status.child(
                    Alert::warning(
                        "archive-recovery-missing-target",
                        "未识别原目标文件夹。请选择当时使用的原文件夹，随后会重新检查；不会改用新位置。",
                    )
                    .with_size(Size::Small),
                );
            } else {
                status = status.child(
                    DescriptionList::vertical()
                        .columns(1)
                        .bordered(false)
                        .with_size(Size::Small)
                        .item("恢复记录", "已识别，开始时仍会校验输出与完整计划", 1)
                        .item("目标核对", "原目标文件夹已确认", 1),
                );
            }
        }
        let preferences = Field::new()
            .label("完成后")
            .child(self.check(
                "archive-open-after",
                "打开目标文件夹",
                self.open_after,
                Flag::Open,
                false,
                cx,
            ))
            .child(self.check(
                "archive-keep-open",
                "保留进度窗口",
                self.keep_open,
                Flag::Keep,
                false,
                cx,
            ));
        Form::vertical()
            .gap_y_3()
            .child(source)
            .child(destination)
            .child(Field::new().label("恢复状态").child(status))
            .when(!completed, |form| form.child(
                Field::new().child(
                    div()
                        .debug_selector(|| "archive-recovery-warning".into())
                        .child(
                            Alert::warning(
                                "archive-recovery-warning-alert",
                                "继续后仍会永久释放已核验的原压缩包数据，不能撤销，也不会恢复完整原包。请保留原压缩包、恢复记录和已解压文件。",
                            )
                            .with_size(Size::Small),
                        ),
                ),
            )
            .child(preferences))
            .into_any_element()
    }

    #[cfg(debug_assertions)]
    pub(in crate::archive) fn preview_recovery_state(
        &mut self,
        mode: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.recovery.active = true;
        self.recovery.epoch = self.recovery.epoch.wrapping_add(1).max(1);
        self.recovery.pending = true;
        self.recovery.inspection = None;
        self.recovery.error = None;
        let destination = match mode {
            "continue-missing" | "continue-pending" => None,
            _ => Some(r"C:\项目\发布资料\经过多层目录的原始输出文件夹\最终交付目标".to_owned()),
        };
        self.recovery.candidate_destination = destination.clone();
        self.set_recovery_candidate(destination.as_deref(), window, cx);
        let epoch = self.recovery.epoch;
        match mode {
            "continue-pending" => self.revalidate(cx),
            "continue-error" => self.apply_recovery(
                epoch,
                Err("恢复记录暂时无法读取，请检查原压缩包访问权限后重试。".into()),
                window,
                cx,
            ),
            _ => self.apply_recovery(
                epoch,
                Ok(ArchiveRecoveryInspection {
                    version: 1,
                    operation_id: Uuid::new_v4(),
                    session_id: self.id,
                    generation: self.generation,
                    recovery_id: Uuid::new_v4(),
                    source_identity: "v1:1:00112233445566778899AABBCCDDEEFF:4096".into(),
                    destination: (mode != "continue-missing").then_some(
                        r"C:\项目\发布资料\经过多层目录的原始输出文件夹\最终交付目标".to_owned(),
                    ),
                    copy_motw: true,
                    completed: mode == "continue-completed",
                }),
                window,
                cx,
            ),
        }
    }
}
