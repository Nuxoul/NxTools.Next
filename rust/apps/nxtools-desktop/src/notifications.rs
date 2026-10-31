//! Bounded business ingress and action ownership; the official NotificationList owns UI lifetime.
use gpui_kit::{AnyWindowHandle, App, Context, Entity, Global, SharedString, WeakEntity, Window};
pub(crate) use nxtools_core::NotificationPosition;
use std::{rc::Rc, time::Duration};
pub(crate) mod copy;
pub(crate) mod presentation;
pub(crate) mod screen;

#[derive(Clone)]
pub(crate) struct NotificationCenterGlobal(pub WeakEntity<NotificationCenter>);
impl Global for NotificationCenterGlobal {}
pub(crate) fn register(center: &Entity<NotificationCenter>, cx: &mut App) {
    cx.set_global(NotificationCenterGlobal(center.downgrade()));
}
pub(crate) fn current(cx: &App) -> Option<Entity<NotificationCenter>> {
    cx.try_global::<NotificationCenterGlobal>()
        .and_then(|global| global.0.upgrade())
}
const MAX_VISIBLE_NOTIFICATIONS: usize = 3;
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
    pub additional_actions: Vec<NotificationAction>,
    pub target: Option<AnyWindowHandle>,
    action_claimed: bool,
    pub exiting: bool,
    generation: u64,
    persistent: bool,
    dwell: Option<Duration>,
}
impl AppNotification {
    pub(crate) fn new(
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
            additional_actions: Vec::new(),
            target: None,
            action_claimed: false,
            exiting: false,
            generation: 0,
            persistent: level == NotificationLevel::Progress,
            dwell: None,
        }
    }
    pub(crate) fn with_details(mut self, details: Vec<NotificationDetail>) -> Self {
        self.details = details;
        self
    }
    fn with_action(mut self, action: Option<NotificationAction>) -> Self {
        self.action = action;
        self
    }
    pub(crate) fn actions(mut self, actions: Vec<NotificationAction>) -> Self {
        let mut actions = actions.into_iter().take(2);
        self.action = actions.next();
        self.additional_actions = actions.collect();
        self
    }
    pub(crate) fn in_window(mut self, window: AnyWindowHandle) -> Self {
        self.target = Some(window);
        self
    }
    /// Debug fixtures can request persistence or an explicit official timeout.
    #[cfg(any(test, debug_assertions))]
    pub(crate) fn duration(mut self, duration: Option<Duration>) -> Self {
        self.persistent = duration.is_none();
        self.dwell = duration;
        self
    }
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}
pub(crate) struct NotificationCenter {
    items: Vec<AppNotification>,
    next_generation: u64,
    position: NotificationPosition,
    duration_seconds: u32,
    copy: copy::CopyRuntime,
}
impl Default for NotificationCenter {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            next_generation: 0,
            position: NotificationPosition::default(),
            duration_seconds: nxtools_core::default_notification_duration(),
            copy: copy::CopyRuntime::default(),
        }
    }
}
impl NotificationCenter {
    pub(crate) fn set_duration_seconds(&mut self, seconds: u32, cx: &mut Context<Self>) {
        let seconds = seconds.clamp(1, 60);
        if self.duration_seconds() != seconds {
            self.duration_seconds = seconds;
            cx.notify();
        }
    }
    pub(crate) fn duration_seconds(&self) -> u32 {
        self.duration_seconds
    }
    pub(crate) fn position(&self) -> NotificationPosition {
        self.position
    }
    pub(crate) fn set_position(&mut self, position: NotificationPosition, cx: &mut Context<Self>) {
        if self.position != position {
            self.position = position;
            cx.notify();
        }
    }
    pub(crate) fn push(&mut self, notification: AppNotification, cx: &mut Context<Self>) {
        let generation = self.upsert(notification);
        tracing::info!(
            event = "ui.notification.accepted",
            generation,
            visible_count = self.items.len(),
            "notification admitted to official presentation"
        );
        cx.notify();
    }
    pub(crate) fn items(&self) -> &[AppNotification] {
        &self.items
    }
    pub(crate) fn invoke_action(
        &mut self,
        id: &str,
        generation: u64,
        index: usize,
        cx: &mut Context<Self>,
    ) -> Option<NotificationAction> {
        let item = self.items.iter_mut().find(|item| {
            item.id.as_ref() == id
                && item.generation == generation
                && !item.exiting
                && !item.action_claimed
        })?;
        let action = if index == 0 {
            item.action.clone()
        } else {
            item.additional_actions.get(index - 1).cloned()
        }?;
        item.action_claimed = true;
        tracing::info!(
            event = "ui.notification.action",
            generation,
            action_index = index,
            "notification action claimed"
        );
        self.start_dismiss(id.into(), generation, cx);
        Some(action)
    }
    pub(crate) fn dismiss_window(&mut self, window: AnyWindowHandle, cx: &mut Context<Self>) {
        let before = self.items.len();
        self.items.retain(|item| item.target != Some(window));
        self.copy.retain(&self.items);
        if before != self.items.len() {
            cx.notify();
        }
    }
    pub(crate) fn show_progress(
        &mut self,
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.push(
            AppNotification::new(id, NotificationLevel::Progress, title, message),
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
        self.push(
            AppNotification::new(id, NotificationLevel::Info, title, message),
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
        self.push(
            AppNotification::new(id, NotificationLevel::Success, title, message),
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
        self.push(
            AppNotification::new(id, NotificationLevel::Error, title, message),
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
        self.push(
            AppNotification::new(id, NotificationLevel::Success, title, message)
                .with_details(details)
                .with_action(action),
            cx,
        );
    }
    pub(crate) fn dismiss_generation(
        &mut self,
        id: impl Into<SharedString>,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        self.start_dismiss(id.into(), generation, cx);
    }
    fn upsert(&mut self, mut notification: AppNotification) -> u64 {
        if !notification.persistent {
            notification
                .dwell
                .get_or_insert(Duration::from_secs(self.duration_seconds() as u64));
        }
        self.next_generation = self.next_generation.wrapping_add(1).max(1);
        notification.generation = self.next_generation;
        self.items.retain(|item| item.id != notification.id);
        self.items.insert(0, notification);
        self.items.truncate(MAX_VISIBLE_NOTIFICATIONS);
        self.copy.retain(&self.items);
        self.next_generation
    }
    fn remove_generation(&mut self, id: &str, generation: u64) -> bool {
        let before = self.items.len();
        self.items
            .retain(|item| item.id.as_ref() != id || item.generation != generation);
        self.copy.retain(&self.items);
        before != self.items.len()
    }
    fn begin_dismiss_generation(&mut self, id: &str, generation: u64) -> bool {
        let Some(item) = self
            .items
            .iter_mut()
            .find(|item| item.id.as_ref() == id && item.generation == generation && !item.exiting)
        else {
            return false;
        };
        item.exiting = true;
        true
    }
    fn start_dismiss(&mut self, id: SharedString, generation: u64, cx: &mut Context<Self>) {
        if self.begin_dismiss_generation(&id, generation) {
            cx.notify();
        }
    }
    fn completed_close(&mut self, id: &str, generation: u64, cx: &mut Context<Self>) {
        if self.remove_generation(id, generation) {
            tracing::info!(
                event = "ui.notification.closed",
                generation,
                "official notification lifecycle finished"
            );
            cx.notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notification_queue_replaces_by_id_bounds_visible_items_and_rejects_stale_dismissal() {
        let mut center = NotificationCenter::default();
        let old = center.upsert(AppNotification::new(
            "one",
            NotificationLevel::Progress,
            "one",
            "progress",
        ));
        let current = center.upsert(AppNotification::new(
            "one",
            NotificationLevel::Success,
            "one",
            "done",
        ));
        assert_ne!(old, current);
        assert!(!center.remove_generation("one", old));
        for id in ["two", "three", "four"] {
            center.upsert(AppNotification::new(
                id,
                NotificationLevel::Info,
                id,
                "message",
            ));
        }
        assert_eq!(center.items.len(), 3);
        assert_eq!(center.items[0].id, "four");
    }
    #[test]
    fn completed_notification_preserves_structured_key_details() {
        let note = AppNotification::new("backup", NotificationLevel::Success, "backup", "done")
            .with_details(vec![NotificationDetail::new("位置", "owned")]);
        assert_eq!(note.details[0].value, "owned");
    }
    #[gpui_kit::test]
    fn stale_and_duplicate_action_claims_never_execute_replacement(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use gpui_kit::AppContext;
        let center = cx.new(|_| NotificationCenter::default());
        center.update(cx, |center, cx| {
            center.push(
                AppNotification::new("run", NotificationLevel::Info, "run", "old")
                    .actions(vec![NotificationAction::new("open", |_, _| {})]),
                cx,
            )
        });
        let old = center.read_with(cx, |center, _| center.items[0].generation());
        center.update(cx, |center, cx| {
            center.push(
                AppNotification::new("run", NotificationLevel::Info, "run", "new")
                    .actions(vec![NotificationAction::new("open", |_, _| {})]),
                cx,
            );
            assert!(center.invoke_action("run", old, 0, cx).is_none());
            let generation = center.items[0].generation();
            assert!(center.invoke_action("run", generation, 0, cx).is_some());
            assert!(center.invoke_action("run", generation, 0, cx).is_none());
        });
    }
}
#[cfg(test)]
mod copy_tests;
#[cfg(test)]
mod expiry_tests;
#[cfg(test)]
pub(crate) mod presentation_tests;
