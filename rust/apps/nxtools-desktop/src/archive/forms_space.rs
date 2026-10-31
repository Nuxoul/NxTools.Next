//! Advisory capacity states inside the original OperationForm and native request lifecycle.
use super::*;
use crate::archive::progress::format_bytes;
use crate::archive::space::Plan;
use gpui_kit::component::label::Label;

impl OperationForm {
    pub(in crate::archive) fn space_disconnected(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.draft, Draft::Extract(_)) || self.retired {
            return;
        }
        if self.recovery.active {
            self.retire_recovery();
            self.recovery.error = Some("原生连接已断开，请连接后重新检查。".into());
            self.revalidate(cx);
            return;
        }
        self.space.epoch = self.space.epoch.wrapping_add(1).max(1);
        self.space.pending = false;
        self.space.waiting = false;
        self.space.stale = self.space.plan.is_some();
        self.space.plan = None;
        self.space.error = Some("连接已断开，重新连接后可刷新空间信息。".into());
        cx.notify();
    }
    pub(in crate::archive) fn space_request_current(
        &self,
        epoch: u64,
        request: &ArchiveJobRequest,
        cx: &App,
    ) -> bool {
        !self.retired
            && !self.submitted
            && self.space.pending
            && self.space.epoch == epoch
            && self.space.key.as_ref() == Some(request)
            && request.destination.as_deref() == Some(self.destination.read(cx).value().as_ref())
    }
    pub(in crate::archive) fn wait_for_space_request(&mut self, epoch: u64) {
        if !self.retired && self.space.epoch == epoch && self.space.pending {
            self.space.waiting = true;
        }
    }
    #[cfg(test)]
    pub(in crate::archive) fn space_state_for_test(&self) -> (bool, Option<(u64, Option<u64>)>) {
        (
            self.space.pending,
            self.space
                .plan
                .as_ref()
                .map(|plan| (plan.observation.available_bytes, plan.estimate.allocation)),
        )
    }
    pub(super) fn refresh_space(&mut self, cx: &mut Context<Self>) {
        if self.retired || self.submitted || self.space.pending {
            return;
        }
        self.sync_inputs(cx);
        let Draft::Extract(draft) = &self.draft else {
            return;
        };
        let Ok(request) = draft.request(&self.settings) else {
            self.space.invalidate(None);
            self.space.error = None;
            cx.notify();
            return;
        };
        self.space.invalidate(Some(request.clone()));
        let expected = self
            .space
            .plan
            .as_ref()
            .map(|plan| plan.observation.target_identity.clone());
        self.space.epoch = self.space.epoch.wrapping_add(1).max(1);
        let epoch = self.space.epoch;
        self.space.pending = true;
        self.space.waiting = false;
        self.space.plan = None;
        self.space.error = None;
        let owner = self.owner.clone();
        let (id, generation) = (self.id, self.generation);
        let local = cx.entity().downgrade();
        cx.defer(move |cx| {
            if owner
                .update(cx, |app, cx| {
                    app.request_space(id, generation, epoch, request, expected, cx)
                })
                .is_err()
            {
                let _ = local.update(cx, |form, cx| {
                    form.apply_space(epoch, Err("连接不可用，稍后可刷新空间信息。".into()), cx)
                });
            }
        });
        cx.notify();
    }
    pub(in crate::archive) fn apply_space(
        &mut self,
        epoch: u64,
        result: Result<Plan, String>,
        cx: &mut Context<Self>,
    ) {
        if self.retired || self.submitted {
            return;
        }
        if self.space.epoch != epoch {
            if self.space.waiting {
                self.space.waiting = false;
                self.space.pending = false;
                self.refresh_space(cx);
            }
            return;
        }
        if !self.space.pending {
            return;
        }
        self.sync_inputs(cx);
        let current = match &self.draft {
            Draft::Extract(draft) => draft.request(&self.settings).ok(),
            _ => None,
        };
        if current != self.space.key {
            self.space.invalidate(current);
            cx.notify();
            return;
        }
        self.space.pending = false;
        self.space.waiting = false;
        self.space.stale = false;
        match result {
            Ok(plan) => {
                self.space.plan = Some(plan);
                self.space.error = None;
            }
            Err(message) => {
                self.space.plan = None;
                self.space.error = Some(message);
            }
        }
        cx.notify();
    }
    pub(in crate::archive) fn invalidate_space_source(
        &mut self,
        path: &std::path::Path,
        cx: &mut Context<Self>,
    ) {
        if self
            .space
            .key
            .as_ref()
            .and_then(|key| key.paths.first())
            .is_some_and(|source| std::path::Path::new(source) == path)
        {
            self.space.epoch = self.space.epoch.wrapping_add(1).max(1);
            self.space.pending = false;
            self.space.waiting = false;
            self.space.stale = true;
            self.space.plan = None;
            self.space.error = Some("压缩包已变化或正在刷新，读取完成后可重新检查。".into());
            cx.notify();
        }
    }
    pub(super) fn space_section(&self, cx: &Context<Self>) -> impl IntoElement {
        let mut section = div()
            .id("archive-space-preflight")
            .debug_selector(|| "archive-space-preflight".into())
            .flex()
            .flex_col()
            .gap_2()
            .min_w_0()
            .text_sm();
        if self.space.key.is_none() {
            return section.child(
                Label::new("选定压缩包和目标后自动检查").text_color(cx.theme().muted_foreground),
            );
        }
        if self.space.pending {
            return section.child(
                div()
                    .debug_selector(|| "archive-space-pending".into())
                    .child(Label::new("正在检查空间").text_color(cx.theme().muted_foreground)),
            );
        }
        if let Some(message) = &self.space.error {
            section = if self.space.stale {
                section
                    .child(
                        div()
                            .debug_selector(|| "archive-space-expired".into())
                            .child(Label::new("空间信息已过期")),
                    )
                    .child(
                        div()
                            .text_color(cx.theme().muted_foreground)
                            .child(message.clone()),
                    )
            } else {
                section.child(
                    div()
                        .debug_selector(|| "archive-space-failed".into())
                        .child(
                            Alert::warning("archive-space-failure", message.clone())
                                .with_size(Size::Small),
                        ),
                )
            };
        } else if let Some(plan) = &self.space.plan {
            let values = DescriptionList::vertical()
                .columns(2)
                .bordered(false)
                .with_size(Size::Small)
                .item(
                    "当前可用",
                    format_bytes(plan.observation.available_bytes),
                    1,
                )
                .when_some(plan.estimate.allocation, |list, bytes| {
                    list.item("预计需要", format_bytes(bytes), 1)
                });
            section = section.child(
                div()
                    .debug_selector(|| "archive-space-values".into())
                    .child(values),
            );
            match plan.shortfall() {
                Some(0) => {
                    section = section.child(
                        div()
                            .debug_selector(|| "archive-space-available".into())
                            .child(Label::new(format!(
                                "预计足够，完成后剩余 {}",
                                format_bytes(plan.remaining().unwrap_or_default())
                            ))),
                    )
                }
                Some(shortfall) => {
                    section = section.child(
                        div()
                            .debug_selector(|| "archive-space-shortage".into())
                            .child(
                                Alert::warning(
                                    "archive-space-shortfall",
                                    format!(
                                        "还差 {}。可更换目标文件夹或先清理磁盘空间。",
                                        format_bytes(shortfall)
                                    ),
                                )
                                .with_size(Size::Small),
                            ),
                    )
                }
                None => {
                    section = section.child(
                        div()
                            .debug_selector(|| "archive-space-unknown".into())
                            .child(Label::new("所需空间暂未知，可继续普通解压")),
                    );
                    if let Some(reason) = plan.estimate.limitation {
                        section = section
                            .child(div().text_color(cx.theme().muted_foreground).child(reason));
                    }
                    if plan.estimate.lower_bound > 0 {
                        section =
                            section.child(div().text_color(cx.theme().muted_foreground).child(
                                format!("已知文件至少 {}", format_bytes(plan.estimate.lower_bound)),
                            ));
                    }
                }
            }
            section = section.child(div().text_color(cx.theme().muted_foreground).child(
                plan.estimate.payload.map_or_else(
                    || "仅供估算，未预留空间；实际需求和可用空间可能变化。".to_owned(),
                    |bytes| {
                        format!(
                            "文件共 {}。所需空间为估算，未预留空间。",
                            format_bytes(bytes)
                        )
                    },
                ),
            ));
            if plan.concurrent_jobs > 0 {
                section = section.child(div().text_color(cx.theme().muted_foreground).child(
                    format!("另有 {} 个任务可能使用此空间。", plan.concurrent_jobs),
                ));
            }
        } else {
            section = section.child(
                Label::new(if self.space.stale {
                    "空间信息已过期"
                } else {
                    "尚未检查空间"
                })
                .text_color(cx.theme().muted_foreground),
            );
        }
        section.child(
            Button::new("archive-space-refresh")
                .debug_selector(|| "archive-space-refresh".into())
                .label("刷新空间")
                .outline()
                .with_size(Size::Small)
                .disabled(self.busy())
                .on_click(cx.listener(|this, _, _, cx| this.refresh_space(cx))),
        )
    }
}
