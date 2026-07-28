use std::rc::Rc;
use std::time::Duration;

use gpui::{App, Context, SharedString, Window};

const MAX_VISIBLE_NOTIFICATIONS: usize = 3;
const COMPLETION_DURATION: Duration = Duration::from_secs(6);
const ERROR_DURATION: Duration = Duration::from_secs(10);
pub(crate) const NOTIFICATION_EXIT_DURATION: Duration = Duration::from_millis(120);

type NotificationActionHandler = dyn Fn(&mut Window, &mut App);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NotificationLevel {
    Progress,
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NotificationDetail {
    pub label: SharedString,
    pub value: SharedString,
}

impl NotificationDetail {
    pub(crate) fn new(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct NotificationAction {
    pub label: SharedString,
    handler: Rc<NotificationActionHandler>,
}

impl NotificationAction {
    pub(crate) fn new(
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            handler: Rc::new(handler),
        }
    }

    pub(crate) fn invoke(&self, window: &mut Window, cx: &mut App) {
        (self.handler)(window, cx);
    }
}

#[derive(Clone)]
pub(crate) struct AppNotification {
    pub id: SharedString,
    pub level: NotificationLevel,
    pub title: SharedString,
    pub message: SharedString,
    pub details: Vec<NotificationDetail>,
    pub action: Option<NotificationAction>,
    pub exiting: bool,
    generation: u64,
}

impl AppNotification {
    fn new(
        id: impl Into<SharedString>,
        level: NotificationLevel,
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            level,
            title: title.into(),
            message: message.into(),
            details: Vec::new(),
            action: None,
            exiting: false,
            generation: 0,
        }
    }

    fn with_details(mut self, details: Vec<NotificationDetail>) -> Self {
        self.details = details;
        self
    }

    fn with_action(mut self, action: Option<NotificationAction>) -> Self {
        self.action = action;
        self
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

#[derive(Default)]
pub(crate) struct NotificationCenter {
    items: Vec<AppNotification>,
    next_generation: u64,
}

impl NotificationCenter {
    pub(crate) fn items(&self) -> &[AppNotification] {
        &self.items
    }

    pub(crate) fn show_progress(
        &mut self,
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.show(
            AppNotification::new(id, NotificationLevel::Progress, title, message),
            None,
            cx,
        );
    }

    pub(crate) fn show_info(
        &mut self,
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.show(
            AppNotification::new(id, NotificationLevel::Info, title, message),
            Some(COMPLETION_DURATION),
            cx,
        );
    }

    pub(crate) fn show_success(
        &mut self,
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.show(
            AppNotification::new(id, NotificationLevel::Success, title, message),
            Some(COMPLETION_DURATION),
            cx,
        );
    }

    pub(crate) fn show_success_with_details(
        &mut self,
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        details: Vec<NotificationDetail>,
        action: Option<NotificationAction>,
        cx: &mut Context<Self>,
    ) {
        self.show(
            AppNotification::new(id, NotificationLevel::Success, title, message)
                .with_details(details)
                .with_action(action),
            Some(COMPLETION_DURATION),
            cx,
        );
    }

    pub(crate) fn show_error(
        &mut self,
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.show(
            AppNotification::new(id, NotificationLevel::Error, title, message),
            Some(ERROR_DURATION),
            cx,
        );
    }

    pub(crate) fn dismiss(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some((id, generation)) = self
            .items
            .iter()
            .find(|item| item.id.as_ref() == id)
            .map(|item| (item.id.clone(), item.generation))
        else {
            return;
        };
        self.start_dismiss(id, generation, cx);
    }

    fn show(
        &mut self,
        notification: AppNotification,
        auto_dismiss: Option<Duration>,
        cx: &mut Context<Self>,
    ) {
        let id = notification.id.clone();
        let generation = self.upsert(notification);
        cx.notify();
        let Some(duration) = auto_dismiss else {
            return;
        };
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(duration).await;
            let _ = this.update(cx, |this, cx| {
                this.start_dismiss(id, generation, cx);
            });
        })
        .detach();
    }

    fn upsert(&mut self, mut notification: AppNotification) -> u64 {
        self.next_generation = self.next_generation.wrapping_add(1).max(1);
        notification.generation = self.next_generation;
        let generation = notification.generation;
        self.items.retain(|existing| existing.id != notification.id);
        self.items.insert(0, notification);
        self.items.truncate(MAX_VISIBLE_NOTIFICATIONS);
        generation
    }

    fn dismiss_generation(&mut self, id: &str, generation: u64) -> bool {
        let previous_len = self.items.len();
        self.items
            .retain(|item| item.id.as_ref() != id || item.generation != generation);
        self.items.len() != previous_len
    }

    fn begin_dismiss_generation(&mut self, id: &str, generation: u64) -> bool {
        let Some(notification) = self
            .items
            .iter_mut()
            .find(|item| item.id.as_ref() == id && item.generation == generation)
        else {
            return false;
        };
        if notification.exiting {
            return false;
        }
        notification.exiting = true;
        true
    }

    fn start_dismiss(&mut self, id: SharedString, generation: u64, cx: &mut Context<Self>) {
        if !self.begin_dismiss_generation(id.as_ref(), generation) {
            return;
        }
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(NOTIFICATION_EXIT_DURATION)
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.dismiss_generation(id.as_ref(), generation) {
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::{AppNotification, NotificationCenter, NotificationLevel};

    #[test]
    fn notification_queue_replaces_by_id_bounds_visible_items_and_rejects_stale_dismissal() {
        let mut center = NotificationCenter::default();
        let first_generation = center.upsert(AppNotification::new(
            "clipboard.database",
            NotificationLevel::Progress,
            "数据库维护",
            "正在备份…",
        ));
        let replacement_generation = center.upsert(AppNotification::new(
            "clipboard.database",
            NotificationLevel::Success,
            "数据库维护",
            "备份完成",
        ));
        assert_ne!(first_generation, replacement_generation);
        assert!(!center.dismiss_generation("clipboard.database", first_generation));
        assert_eq!(center.items().len(), 1);
        assert_eq!(center.items()[0].level, NotificationLevel::Success);
        assert_eq!(center.items()[0].message, "备份完成");

        center.upsert(AppNotification::new(
            "second",
            NotificationLevel::Info,
            "第二条",
            "message",
        ));
        center.upsert(AppNotification::new(
            "third",
            NotificationLevel::Info,
            "第三条",
            "message",
        ));
        center.upsert(AppNotification::new(
            "fourth",
            NotificationLevel::Error,
            "第四条",
            "message",
        ));

        assert_eq!(center.items().len(), 3);
        assert_eq!(center.items()[0].id, "fourth");
        assert!(
            center
                .items()
                .iter()
                .all(|item| item.id != "clipboard.database")
        );
    }

    #[test]
    fn completed_notification_preserves_structured_key_details() {
        let notification = AppNotification::new(
            "clipboard.database",
            NotificationLevel::Success,
            "数据库备份",
            "已创建可恢复备份 · 200 KB",
        )
        .with_details(vec![super::NotificationDetail::new(
            "保存位置",
            r"C:\Users\Administrator\AppData\Roaming\NxTools\NxTools.Next\data\backups",
        )]);

        assert_eq!(notification.details.len(), 1);
        assert_eq!(notification.details[0].label, "保存位置");
        assert!(notification.details[0].value.ends_with(r"data\backups"));
    }

    #[test]
    fn dismiss_marks_the_matching_generation_as_exiting_before_removal() {
        let mut center = NotificationCenter::default();
        let generation = center.upsert(AppNotification::new(
            "clipboard.database",
            NotificationLevel::Success,
            "数据库备份",
            "备份完成",
        ));

        assert!(center.begin_dismiss_generation("clipboard.database", generation));
        assert!(center.items()[0].exiting);
        assert!(!center.begin_dismiss_generation("clipboard.database", generation));
        assert!(center.dismiss_generation("clipboard.database", generation));
        assert!(center.items().is_empty());
    }
}
