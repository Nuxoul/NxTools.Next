//! Manual extension outcomes use the same bounded in-window notification system.
use super::{ExtensionApplication, manual_runs::ManualState};
use crate::notifications::{
    AppNotification, NotificationAction, NotificationDetail, NotificationLevel,
};
use gpui_kit::Context;

impl ExtensionApplication {
    pub(crate) fn notify_manual_run(&self, id: uuid::Uuid, cx: &mut Context<Self>) {
        let Some(center) = self.notifications.clone() else {
            return;
        };
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let Some(owner) = owner.upgrade() else { return };
            let Some(run) = owner.read(cx).manual_run(id) else {
                return;
            };
            if let Some(origin) = &run.origin
                && !origin
                    .panel
                    .upgrade()
                    .is_some_and(|panel| panel.read(cx).extension_origin_current(origin.session_id))
            {
                return;
            }
            let level = match run.state {
                ManualState::Running | ManualState::Cancelling => NotificationLevel::Progress,
                ManualState::Succeeded => NotificationLevel::Success,
                ManualState::Failed(_) => NotificationLevel::Error,
                _ => NotificationLevel::Info,
            };
            let message = if run.items.is_empty() {
                run.state.label().to_owned()
            } else if run.state.pending() {
                format!("{} · 已保存 {} 个结果", run.state.label(), run.items.len())
            } else {
                format!(
                    "{} · 已保存 {} 个结果，原文保留",
                    run.state.label(),
                    run.items.len()
                )
            };
            let mut actions = Vec::new();
            if run.state.pending() {
                let weak = owner.downgrade();
                actions.push(NotificationAction::new("取消", move |_, cx| {
                    let weak = weak.clone();
                    cx.defer(move |cx| {
                        let _ = weak.update(cx, |app, cx| app.cancel_manual_run(id, cx));
                    });
                }));
            } else if let (Some(origin), Some(item)) =
                (run.origin.clone(), run.items.first().copied())
            {
                let weak = owner.downgrade();
                actions.push(NotificationAction::new("查看结果", move |_, cx| {
                    let origin = origin.clone();
                    let weak = weak.clone();
                    cx.defer(move |cx| {
                        let current = weak
                            .upgrade()
                            .and_then(|owner| owner.read(cx).manual_run(id))
                            .is_some_and(|run| run.items.contains(&item));
                        if !current {
                            return;
                        }
                        let _ = origin.window.update(cx, |_, window, cx| {
                            let _ = origin.panel.update(cx, |panel, cx| {
                                panel.locate_extension_result(
                                    origin.session_id,
                                    id,
                                    item,
                                    window,
                                    cx,
                                )
                            });
                        });
                    });
                }));
            }
            let weak = owner.downgrade();
            let details_origin = run.origin.clone();
            actions.push(NotificationAction::new("详情", move |window, cx| {
                let Some(owner) = weak.upgrade() else { return };
                if owner.read(cx).manual_run(id).is_none() {
                    return;
                }
                if let Some(origin) = &details_origin
                    && !origin.panel.upgrade().is_some_and(|panel| {
                        panel.read(cx).extension_origin_current(origin.session_id)
                    })
                {
                    return;
                }
                super::manual_actions::show_run(owner, id, details_origin.clone(), window, cx);
            }));
            let mut notification = AppNotification::new(
                format!("extension-manual-{id}"),
                level,
                run.action_name.clone(),
                message,
            )
            .with_details(vec![NotificationDetail::new(
                "结果数",
                run.items.len().to_string(),
            )])
            .actions(actions);
            if let Some(origin) = run.origin {
                notification = notification.in_window(origin.window);
                if !run.state.pending() {
                    let _ = origin.panel.update(cx, |panel, cx| {
                        panel.dismiss_extension_feedback(origin.session_id, origin.item_id, id, cx)
                    });
                }
            }
            center.update(cx, |center, cx| center.push(notification, cx));
        });
    }
    pub(crate) fn refresh_manual_result_notifications(&self, cx: &mut Context<Self>) {
        let Some(center) = &self.notifications else {
            return;
        };
        let ids = self
            .recent_manual_runs()
            .into_iter()
            .filter(|run| !run.state.pending())
            .filter_map(|run| {
                let key = format!("extension-manual-{}", run.id);
                center
                    .read(cx)
                    .items()
                    .iter()
                    .find(|item| item.id.as_ref() == key && !item.exiting)
                    .filter(|item| {
                        item.details.first().is_none_or(|detail| {
                            detail.value.as_ref() != run.items.len().to_string()
                        })
                    })
                    .map(|_| run.id)
            })
            .collect::<Vec<_>>();
        for id in ids {
            self.notify_manual_run(id, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{AppContext, TestAppContext, WeakEntity};
    #[gpui_kit::test]
    fn manual_terminal_uses_shared_notification_once_and_never_trusts_plugin_item_ids(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let center = cx.new(|_| crate::notifications::NotificationCenter::default());
        let owner = cx.new(|_| {
            ExtensionApplication::new(
                Default::default(),
                Default::default(),
                None,
                WeakEntity::new_invalid(),
            )
        });
        let id = uuid::Uuid::new_v4();
        let (cancel, _) = tokio::sync::watch::channel(false);
        owner.update(cx, |app, cx| {
            app.notifications = Some(center.clone());
            app.begin_manual_run(id, "fixture".into(), "工具".into(), "处理测试".into());
            app.pending_manual.insert(id, ("fixture".into(), cancel));
            app.finish_manual_run(
                id,
                Ok(nxtools_extensions::InvokeOutcome {
                    identity: nxtools_extensions::InvocationIdentity {
                        run_id: id,
                        correlation_id: id,
                        generation: 1,
                        package_id: "fixture".into(),
                        handler_id: "action".into(),
                        content_hash: "a".repeat(64),
                    },
                    status: "success".into(),
                    value: serde_json::json!({"item_id":999,"text":"author explanation"}),
                }),
                cx,
            );
        });
        cx.run_until_parked();
        center.read_with(cx, |center, _| {
            assert_eq!(center.items().len(), 1);
            let notice = &center.items()[0];
            assert_eq!(notice.level, NotificationLevel::Success);
            assert_eq!(notice.details[0].value, "0");
            assert_eq!(
                notice.action.as_ref().expect("explicit Details only").label,
                "详情"
            );
            assert!(notice.additional_actions.is_empty());
        });
        let generation = center.read_with(cx, |center, _| center.items()[0].generation());
        owner.update(cx, |app, cx| {
            app.finish_manual_run(id, Err(nxtools_extensions::RuntimeError::Cancelled), cx)
        });
        cx.run_until_parked();
        assert_eq!(
            center.read_with(cx, |center, _| center.items()[0].generation()),
            generation,
            "duplicate terminal cannot replace notification"
        );
    }
}
