//! Session-only timeline; execution and cancellation stay with the original run owners.
use super::*;
use crate::extensions::{
    event_history::EventTerminal,
    manual_runs::{ManualRun, ManualState},
};
use gpui_kit::component::{
    ActiveTheme as _,
    checkbox::Checkbox,
    empty::{Empty, EmptyDescription, EmptyHeader, EmptyTitle},
    list::ListItem,
    tag::Tag,
};

#[derive(Clone)]
enum Activity {
    Manual(Box<ManualRun>),
    Automatic(EventTerminal),
}

impl Activity {
    fn key(&self) -> (bool, i64) {
        match self {
            Self::Manual(run) => (run.state.pending(), run.at_unix_ms),
            Self::Automatic(event) => (false, event.at_unix_ms),
        }
    }

    fn failed(&self) -> bool {
        match self {
            Self::Manual(run) => matches!(run.state, ManualState::Failed(_)),
            Self::Automatic(event) => {
                !event.outcome.success()
                    && event.outcome != crate::extensions::event_history::EventOutcome::Cancelled
            }
        }
    }
}

fn ordered_activity(
    manual: Vec<ManualRun>,
    automatic: Vec<EventTerminal>,
    failed_only: bool,
) -> Vec<Activity> {
    let mut activity = manual
        .into_iter()
        .map(|run| Activity::Manual(Box::new(run)))
        .chain(automatic.into_iter().map(Activity::Automatic))
        .filter(|row| !failed_only || row.failed())
        .collect::<Vec<_>>();
    activity.sort_by_key(|row| std::cmp::Reverse(row.key()));
    activity
}

impl ExtensionManagerView {
    pub(super) fn render_activity(&self, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let owner = self.application.read(cx);
        let activity = ordered_activity(
            owner.recent_manual_runs(),
            owner.recent_events(),
            self.activity_failed_only,
        );
        let view = cx.entity().downgrade();
        let mut body = div()
            .id("extension-session-activity")
            .debug_selector(|| "extension-session-activity".into())
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div().flex().flex_col().child("本次会话").child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child("记录关闭程序后清空，不显示原始输入。"),
                        ),
                    )
                    .child(
                        Checkbox::new("extension-activity-failed-only")
                            .label("只看失败")
                            .checked(self.activity_failed_only)
                            .on_change(move |checked, _, cx| {
                                let requested = *checked;
                                let view = view.clone();
                                cx.defer(move |cx| {
                                    let _ = view.update(cx, |view, cx| {
                                        view.activity_failed_only = requested;
                                        cx.notify();
                                    });
                                });
                            }),
                    ),
            );
        if activity.is_empty() {
            body = body.child(
                div()
                    .debug_selector(|| "extension-activity-empty".into())
                    .child(
                        Empty::new().header(
                            EmptyHeader::new()
                                .title(EmptyTitle::new().child(if self.activity_failed_only {
                                    "没有失败记录"
                                } else {
                                    "还没有调用记录"
                                }))
                                .description(
                                    EmptyDescription::new()
                                        .child("手动和自动调用会在这里显示结果。"),
                                ),
                        ),
                    ),
            );
        }
        for row in activity {
            let time = chrono::DateTime::from_timestamp_millis(row.key().1).map_or_else(
                || "—".to_owned(),
                |time| {
                    time.with_timezone(&chrono::Local)
                        .format("%H:%M:%S")
                        .to_string()
                },
            );
            let failed = row.failed();
            match row {
                Activity::Manual(run) => {
                    let id = run.id;
                    let app = self.application.clone();
                    let mut actions = div().flex().flex_wrap().gap_2();
                    if owner.can_open_settings_for_run(&run) {
                        let package = run.package_id.clone();
                        actions = actions.child(extensions_settings::view_button(
                            format!("extension-activity-settings-{id}"),
                            "完成设置",
                            owner.busy,
                            cx.entity().downgrade(),
                            move |view, window, cx| view.open_settings(package.clone(), window, cx),
                        ));
                    } else {
                        actions = actions.child(
                            Button::new(format!("extension-activity-{id}"))
                                .label("查看调用")
                                .outline()
                                .on_click(move |_, window, cx| {
                                    crate::extensions::manual_actions::show_run(
                                        app.clone(),
                                        id,
                                        None,
                                        window,
                                        cx,
                                    )
                                }),
                        );
                    }
                    if run.state.pending() {
                        let app = self.application.downgrade();
                        actions = actions.child(extensions_settings::app_button(
                            format!("extension-activity-cancel-{id}"),
                            if matches!(run.state, ManualState::Cancelling) {
                                "正在取消…"
                            } else {
                                "取消调用"
                            },
                            matches!(run.state, ManualState::Cancelling),
                            app,
                            move |app, cx| app.cancel_manual_run(id, cx),
                        ));
                    }
                    let status = if failed {
                        Tag::danger()
                    } else if run.state.pending() {
                        Tag::info()
                    } else {
                        Tag::secondary()
                    };
                    let content = div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .text_sm()
                        .child(format!("手动 · {} · {}", run.package_name, run.action_name))
                        .child(status.small().child(run.state.label()))
                        .children(run.progress.as_ref().map(|progress| {
                            crate::extensions::manual_runs::progress_view(
                                progress,
                                format!("extension-activity-progress-{id}"),
                            )
                        }));
                    body = body.child(activity_row(
                        format!("extension-activity-row-{id}"),
                        time,
                        content,
                        actions,
                    ));
                }
                Activity::Automatic(event) => {
                    let package = owner
                        .packages
                        .packages
                        .iter()
                        .find(|p| p.manifest.id == event.package_id);
                    let handler = package.and_then(|p| {
                        p.manifest
                            .handlers
                            .iter()
                            .find(|h| h.id == event.handler_id && usage::is_event(h))
                    });
                    let paused = owner.event_paused(&event.package_id, &event.handler_id);
                    let id = format!(
                        "extension-event-activity-{}-{}-{}",
                        event.event_id, event.package_id, event.handler_id
                    );
                    let status = if failed {
                        Tag::danger()
                    } else {
                        Tag::secondary()
                    };
                    let content = div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .text_sm()
                        .child(format!(
                            "自动 · {} · {}",
                            package.map_or("已移除扩展", |p| p.manifest.name.as_str()),
                            handler.map_or("事件", |h| h.title.as_str())
                        ))
                        .child(status.small().child(event.outcome.label()));
                    let mut actions = div();
                    let needs_settings = event.outcome
                        == crate::extensions::event_history::EventOutcome::InvalidSettings
                        && package.is_some_and(|package| !package.manifest.settings.is_empty());
                    if failed && handler.is_some() {
                        let package = event.package_id.clone();
                        actions = actions.child(extensions_settings::view_button(
                            format!("{id}-details"),
                            if needs_settings {
                                "完成设置"
                            } else if paused {
                                "查看暂停原因"
                            } else {
                                "查看自动触发"
                            },
                            owner.busy,
                            cx.entity().downgrade(),
                            move |view, window, cx| {
                                if needs_settings {
                                    view.open_settings(package.clone(), window, cx);
                                } else {
                                    view.open_package(package.clone());
                                    view.detail_initial_page = 3;
                                }
                            },
                        ));
                    }
                    body = body.child(activity_row(id, time, content, actions));
                }
            }
        }
        body.into_any_element()
    }
}

fn activity_row(
    id: String,
    time: String,
    content: gpui_kit::Div,
    actions: gpui_kit::Div,
) -> ListItem {
    ListItem::new(id).w_full().min_w_0().py_2().child(
        div()
            .w_full()
            .min_w_0()
            .flex()
            .items_start()
            .gap_3()
            .child(div().text_xs().flex_shrink_0().child(time))
            .child(content.flex_1())
            .child(actions.flex_shrink_0()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::event_history::EventOutcome;
    use gpui_kit::TestAppContext;

    #[gpui_kit::test]
    fn mixed_activity_prioritizes_running_then_orders_terminals_and_filters_failures(
        cx: &mut TestAppContext,
    ) {
        let app = cx.new(|_| {
            ExtensionApplication::new(
                Default::default(),
                Default::default(),
                None,
                gpui_kit::WeakEntity::new_invalid(),
            )
        });
        let mut manual = app.update(cx, |app, _| {
            let id = app.seed_manual_settings_failure("fixture");
            app.manual_run(id).expect("manual report")
        });
        manual.at_unix_ms = 120;
        let mut pending = manual.clone();
        pending.state = ManualState::Running;
        pending.at_unix_ms = 10;
        let event = |at_unix_ms, outcome| EventTerminal {
            event_id: uuid::Uuid::new_v4(),
            correlation_id: uuid::Uuid::new_v4(),
            package_id: "fixture".into(),
            handler_id: "event".into(),
            at_unix_ms,
            elapsed_ms: 1,
            outcome,
        };
        let automatic = vec![
            event(150, EventOutcome::Succeeded),
            event(1500, EventOutcome::Cancelled),
            event(200, EventOutcome::Timeout),
        ];
        let all = ordered_activity(
            vec![manual.clone(), pending.clone()],
            automatic.clone(),
            false,
        );
        assert_eq!(
            all.iter().map(Activity::key).collect::<Vec<_>>(),
            [
                (true, 10),
                (false, 1500),
                (false, 200),
                (false, 150),
                (false, 120)
            ]
        );
        let failed = ordered_activity(vec![manual, pending], automatic, true);
        assert_eq!(
            failed.iter().map(Activity::key).collect::<Vec<_>>(),
            [(false, 200), (false, 120)]
        );
    }
}
