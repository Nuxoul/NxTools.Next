//! Bounded presentation over original logical jobs. No scheduler, native authority or clock.
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::{
    Disableable, Sizable, Size,
    button::Button,
    label::Label,
    table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow},
};
use gpui_kit::{
    InteractiveElement as _, IntoElement, ParentElement, StatefulInteractiveElement as _, Styled,
    div,
};
use uuid::Uuid;
fn readable(id: &'static str, value: String) -> gpui_kit::AnyElement {
    div()
        .id(id)
        .role(gpui_kit::Role::Label)
        .aria_label(value.clone())
        .test_support()
        .min_w_0()
        .child(Label::new(value).whitespace_normal())
        .into_any_element()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MemberState {
    Queued,
    Admitting,
    Running,
    Waiting,
    Paused,
    Succeeded,
    Warnings,
    Failed,
    Cancelled,
}
impl MemberState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "排队",
            Self::Admitting => "正在准入",
            Self::Running => "运行",
            Self::Waiting => "等待交互",
            Self::Paused => "暂停",
            Self::Succeeded => "成功",
            Self::Warnings => "警告",
            Self::Failed => "失败",
            Self::Cancelled => "取消",
        }
    }
}
#[derive(Clone)]
pub(super) struct BatchMember {
    pub id: Uuid,
    pub label: String,
    pub state: MemberState,
    pub never_started: bool,
    pub cleanup_unknown: bool,
    pub published: bool,
    pub retry: bool,
    pub retry_pending: bool,
    pub detail: String,
}
#[derive(Clone)]
pub(super) struct BatchSnapshot {
    pub id: Uuid,
    pub members: Vec<BatchMember>,
    pub not_admitted: usize,
    pub pending_forms: usize,
    pub cancelled_unstarted: usize,
    pub refusal: Option<String>,
}
impl BatchSnapshot {
    pub fn heading(&self) -> String {
        let count = |state| self.members.iter().filter(|m| m.state == state).count();
        format!(
            "归档任务 {}：排队 {}，运行 {}，等待交互 {}，暂停 {}，成功 {}，警告 {}，失败 {}，取消 {}；正在准入 {}",
            self.members.len() + self.not_admitted + self.pending_forms + self.cancelled_unstarted,
            count(MemberState::Queued),
            count(MemberState::Running),
            count(MemberState::Waiting),
            count(MemberState::Paused),
            count(MemberState::Succeeded),
            count(MemberState::Warnings),
            count(MemberState::Failed),
            count(MemberState::Cancelled) + self.cancelled_unstarted,
            count(MemberState::Admitting)
        )
    }
    pub fn facts(&self) -> String {
        format!(
            "未开始 {}（含排队/排队取消及未接收）；待确认 {}，提交前取消 {}；清理未确认 {}；有实际发布 {}。任务数不是文件数；不汇总未知字节百分比。",
            self.not_admitted
                + self.pending_forms
                + self.cancelled_unstarted
                + self.members.iter().filter(|m| m.never_started).count(),
            self.pending_forms,
            self.cancelled_unstarted,
            self.members.iter().filter(|m| m.cleanup_unknown).count(),
            self.members.iter().filter(|m| m.published).count()
        )
    }
}
pub(super) struct BatchRecord {
    pub members: Vec<Uuid>,
    pub requested: usize,
    pub refusal: Option<String>,
    pub terminal_logged: bool,
    pub pending_forms: Vec<Uuid>,
    pub cancelled_unstarted: usize,
}

/// Official stateless table, no second selection/controller; actions return original logical IDs.
pub(super) fn render(
    snapshot: &BatchSnapshot,
    owner: gpui_kit::WeakEntity<super::ArchiveApplication>,
) -> gpui_kit::AnyElement {
    let mut body = TableBody::new();
    for member in &snapshot.members {
        let id = member.id;
        let detail_owner = owner.clone();
        let retry_owner = owner.clone();
        let stop_owner = owner.clone();
        let active = matches!(
            member.state,
            MemberState::Queued
                | MemberState::Admitting
                | MemberState::Running
                | MemberState::Waiting
                | MemberState::Paused
        );
        let actions = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .child(
                Button::new(gpui_kit::SharedString::from(format!(
                    "archive-batch-detail-{id}"
                )))
                .label("详情")
                .outline()
                .with_size(Size::Medium)
                .on_click(move |_, _, cx| {
                    let owner = detail_owner.clone();
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |app, cx| app.show_job(id, cx));
                    });
                }),
            )
            .child(
                Button::new(gpui_kit::SharedString::from(format!(
                    "archive-batch-retry-{id}"
                )))
                .label(if member.retry_pending {
                    "正在重验"
                } else {
                    "重试"
                })
                .outline()
                .with_size(Size::Medium)
                .disabled(!member.retry || member.retry_pending)
                .on_click(move |_, _, cx| {
                    let owner = retry_owner.clone();
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |app, cx| app.retry_job(id, cx));
                    });
                }),
            )
            .child(
                Button::new(gpui_kit::SharedString::from(format!(
                    "archive-batch-stop-{id}"
                )))
                .label("停止")
                .outline()
                .with_size(Size::Medium)
                .disabled(!active)
                .on_click(move |_, _, cx| {
                    let owner = stop_owner.clone();
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |app, cx| {
                            app.control_job(
                                id,
                                nxtools_protocol::archive_job::ArchiveJobAction::Cancel,
                                cx,
                            )
                        });
                    });
                }),
            );
        body = body.child(
            TableRow::new()
                .child(
                    TableCell::new()
                        .min_w_0()
                        .flex_1()
                        .child(Label::new(member.label.clone()).whitespace_normal()),
                )
                .child(
                    TableCell::new()
                        .min_w_0()
                        .flex_1()
                        .child(
                            Label::new(format!(
                                "{}{}{}",
                                member.state.label(),
                                if member.cleanup_unknown {
                                    " · 清理未确认"
                                } else {
                                    ""
                                },
                                if member.published {
                                    " · 已有发布"
                                } else {
                                    ""
                                }
                            ))
                            .whitespace_normal(),
                        )
                        .child(Label::new(member.detail.clone()).whitespace_normal()),
                )
                .child(TableCell::new().min_w_0().flex_1().child(actions)),
        );
    }
    let mut content = div()
        .flex()
        .flex_col()
        .gap_2()
        .min_w_0()
        .id(gpui_kit::SharedString::from(format!(
            "archive-batch-{}",
            snapshot.id
        )))
        .child(readable("archive-batch-counts", snapshot.heading()))
        .child(readable("archive-batch-facts", snapshot.facts()));
    if let Some(refusal) = &snapshot.refusal {
        content = content.child(gpui_kit::component::alert::Alert::warning(
            "archive-batch-refusal",
            format!("未接收 {} 个任务：{refusal}", snapshot.not_admitted),
        ));
    }
    content
        .child(
            Table::new()
                .accessibility_label("归档批次任务状态")
                .with_size(Size::Small)
                .child(
                    TableHeader::new().child(
                        TableRow::new()
                            .child(TableHead::new().child("归档任务"))
                            .child(TableHead::new().child("实际状态 / 失败详情"))
                            .child(TableHead::new().child("操作")),
                    ),
                )
                .child(body),
        )
        .into_any_element()
}
