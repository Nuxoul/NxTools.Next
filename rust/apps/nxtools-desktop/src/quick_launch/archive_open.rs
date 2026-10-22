//! Internal ordinary archive Open, retaining Quick Find's pending/result/usage owner.
use super::*;
impl QuickLaunchApplication {
    pub(super) fn try_archive_open(
        &mut self,
        result: &SearchResult,
        action: &QuickLaunchAction,
        query: &str,
        trigger: QuickLaunchActionTrigger,
        cx: &mut Context<Self>,
    ) -> bool {
        if *action != QuickLaunchAction::Open
            || result.entry.source != ApplicationSource::IndexedFile
            || !result.opens_target()
            || !crate::archive_launch::recognized_archive(std::path::Path::new(
                &result.entry.target,
            ))
        {
            return false;
        }
        let id = uuid::Uuid::new_v4();
        let owner = cx.entity().downgrade();
        let initial_session = self.popup_session_id;
        self.pending_actions.insert(
            id,
            PendingQuickLaunchAction {
                correlation_id: id,
                popup_session_id: initial_session,
                entry_id: result.entry.id.clone(),
                display_name: result.entry.name.clone(),
                target: result.entry.target.clone(),
                query: query.to_owned(),
                action: action.clone(),
                directory_visit: None,
                trigger,
                source: result.entry.source,
                started_at: Instant::now(),
            },
        );
        let accepted = crate::archive::open_internal(
            result.entry.target.clone().into(),
            move |outcome, cx| {
                let owner = owner.clone();
                cx.defer(move |cx| {
                    let _ = owner.update(cx, |app, cx| {
                        app.handle_command_result(
                            id,
                            PlatformCommandResult {
                                success: outcome.is_ok(),
                                error: outcome
                                    .err()
                                    .map(|_| "归档窗口未能打开；请检查文件后重试。".into()),
                                native_error_code: None,
                                data: None,
                            },
                            cx,
                        )
                    });
                });
            },
            cx,
        );
        if !accepted {
            self.pending_actions.remove(&id);
            return false;
        }
        info!(event="quick_launch.action.accepted",correlation_id=%id,action="open_archive",trigger=trigger.diagnostic_label(),"quick-launch internal archive open accepted");
        if initial_session == self.popup_session_id {
            self.hide_popup(cx);
        }
        if let Some(pending) = self.pending_actions.get_mut(&id) {
            pending.popup_session_id = self.popup_session_id;
        }
        true
    }
}
