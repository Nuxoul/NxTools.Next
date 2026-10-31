//! One optional passive desktop assistant; existing feature owners retain all actions.
pub(crate) mod actions;
pub(crate) mod application;
mod lifecycle;
mod media;
mod playback;
mod view;

pub(crate) use application::FloatingAssistantApplication;
use gpui_kit::{App, Entity, Global};
use nxtools_ipc::PlatformNotification;

struct FloatingGlobal(Entity<FloatingAssistantApplication>);
impl Global for FloatingGlobal {}
pub(crate) fn current(cx: &App) -> Option<Entity<FloatingAssistantApplication>> {
    cx.try_global::<FloatingGlobal>()
        .map(|global| global.0.clone())
}
pub(crate) fn register(owner: Entity<FloatingAssistantApplication>, cx: &mut App) {
    cx.set_global(FloatingGlobal(owner));
}
pub(crate) fn handle(notification: PlatformNotification, cx: &mut App) {
    let Some(owner) = current(cx).map(|owner| owner.downgrade()) else {
        return;
    };
    cx.defer(move |cx| {
        let _ = owner.update(cx, |owner, cx| owner.handle(notification, cx));
    });
}
