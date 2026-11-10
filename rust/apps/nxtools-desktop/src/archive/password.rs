//! Browser prompt projection over the existing private worker rendezvous.
use super::*;
use gpui_kit::component::{
    WindowExt,
    button::Button,
    dialog::{DialogAction, DialogClose, DialogFooter},
    input::{Input, InputState},
};
use gpui_kit::{InteractiveElement, ParentElement};
use nxtools_protocol::{
    PlatformCommand,
    archive_password::{ArchivePasswordPrompt, ArchivePasswordReply, PrivatePassword},
};

pub(super) struct PasswordDialog {
    session: Uuid,
    window: AnyWindowHandle,
    alive: Rc<std::cell::Cell<bool>>,
    scope: Rc<std::cell::RefCell<Option<gpui_kit::FocusHandle>>>,
    input: Entity<InputState>,
}

/// Per-host lifecycle state, not a second password queue. Tickets stay in `passwords`.
#[derive(Default)]
pub(super) struct PasswordRecovery {
    retired: Vec<PasswordDialog>,
    focus: Option<Uuid>,
    queued: bool,
    subscription: Option<gpui_kit::Subscription>,
}

impl ArchiveApplication {
    pub(super) fn tab_is_active(&self, id: Uuid, cx: &App) -> bool {
        self.host.as_ref().is_some_and(|host| {
            host.view
                .read_with(cx, |host, _| host.active == Some(id))
                .unwrap_or(false)
        })
    }
    pub(super) fn tab_activated(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if !self.tab_is_active(id, cx) {
            return;
        }
        let hidden = self
            .password_dialogs
            .iter()
            .filter(|(_, dialog)| dialog.session != id)
            .map(|(operation, _)| *operation)
            .collect::<Vec<_>>();
        for operation in hidden {
            self.hide_password_dialog(operation, cx);
        }
        let preview_hidden = self
            .passwords
            .values()
            .filter(|prompt| prompt.session_id != id)
            .map(|prompt| prompt.operation_id)
            .collect::<Vec<_>>();
        for operation in preview_hidden {
            self.suspend_preview_password(operation, cx);
        }
        if let Some(host) = self.host.as_mut() {
            host.password_recovery.focus = Some(id);
            let host_id = host.id;
            self.queue_password_recovery(host_id, cx);
        }
    }

    fn queue_password_recovery(&mut self, host_id: Uuid, cx: &mut Context<Self>) {
        let Some(host) = self.host.as_mut().filter(|host| host.id == host_id) else {
            return;
        };
        if std::mem::replace(&mut host.password_recovery.queued, true) {
            return;
        }
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| app.recover_passwords(host_id, cx));
        });
    }

    fn recover_passwords(&mut self, host_id: Uuid, cx: &mut Context<Self>) {
        let Some(host) = self.host.as_mut().filter(|host| host.id == host_id) else {
            return;
        };
        host.password_recovery.queued = false;
        host.password_recovery.subscription = None;
        let source = host.window;
        let mut retired = std::mem::take(&mut host.password_recovery.retired);
        let focus = host.password_recovery.focus;
        let active = host
            .view
            .read_with(cx, |host, _| host.active)
            .ok()
            .flatten();
        let view = active.and_then(|id| self.windows.get(&id)?.view.upgrade());
        let owner = cx.entity().downgrade();
        let recovered = source.update(cx, |_, window, cx| {
            // A covered retired scope stays owned here until Root restores it. Never pop
            // the newer top modal. Each pass is bounded by the scopes actually retired.
            while window.has_active_dialog(cx) {
                let Some(index) = retired.iter().position(|dialog| {
                    dialog
                        .scope
                        .borrow()
                        .as_ref()
                        .is_some_and(|scope| scope.contains_focused(window, cx))
                }) else {
                    break;
                };
                retired.remove(index);
                window.close_dialog(cx);
            }
            if !window.has_active_dialog(cx) {
                retired.clear();
            }
            let modal = window.has_active_dialog(cx);
            if !modal
                && focus == active
                && let Some(view) = &view
            {
                use gpui_kit::Focusable as _;
                view.focus_handle(cx).focus(window, cx);
            }
            // Public focus lifecycle: both immediate close and the original animated
            // close eventually leave the current modal focus. No timer/frame retries.
            let subscription = if modal {
                window.focused(cx).map(|handle| {
                    window.on_focus_out(&handle, cx, move |_, _, cx| {
                        let owner = owner.clone();
                        cx.defer(move |cx| {
                            let _ = owner
                                .update(cx, |app, cx| app.queue_password_recovery(host_id, cx));
                        });
                    })
                })
            } else {
                None
            };
            (modal, retired, subscription)
        });
        let Ok((modal, remaining, subscription)) = recovered else {
            return;
        };
        let Some(host) = self.host.as_mut().filter(|host| host.id == host_id) else {
            return;
        };
        host.password_recovery.retired = remaining;
        host.password_recovery.subscription = subscription;
        if modal {
            return;
        }
        host.password_recovery.focus = None;
        let prompts = self
            .passwords
            .values()
            .filter(|prompt| Some(prompt.session_id) == active && self.password_current(prompt, cx))
            .cloned()
            .collect::<Vec<_>>();
        for prompt in prompts {
            if !self.preview_password_prompt(prompt.clone(), cx) {
                self.password_prompt(prompt, cx);
            }
        }
    }

    pub(super) fn hide_password_dialog(&mut self, operation: Uuid, cx: &mut Context<Self>) {
        let Some(dialog) = self.password_dialogs.remove(&operation) else {
            return;
        };
        dialog.alive.set(false);
        let input = dialog.input.clone();
        let source = dialog.window;
        cx.defer(move |cx| {
            let _ = source.update(cx, |_, window, cx| {
                input.update(cx, |input, cx| input.set_value("", window, cx));
            });
        });
        tracing::info!(event="archive.password.dialog_retired", correlation_id=%operation, session_id=%dialog.session, "archive password presentation retired");
        if let Some(host) = self
            .host
            .as_mut()
            .filter(|host| host.window == dialog.window)
        {
            host.password_recovery.retired.push(dialog);
            let host_id = host.id;
            self.queue_password_recovery(host_id, cx);
        }
    }
    fn password_current(&self, prompt: &ArchivePasswordPrompt, cx: &App) -> bool {
        let Some(slot) = self
            .windows
            .get(&prompt.session_id)
            .filter(|slot| slot.generation == prompt.generation)
        else {
            return false;
        };
        self.reads.get(&prompt.operation_id).is_some_and(|read| {
            read.window_id == prompt.session_id && read.generation == prompt.generation
        }) || slot
            .view
            .read_with(cx, |view, _| {
                view.authorization_password_origin(prompt).is_some()
            })
            .unwrap_or(false)
            || self.entries.values().any(|entry| {
                entry.operation == prompt.operation_id
                    && entry.window_id == prompt.session_id
                    && entry.generation == prompt.generation
                    && !entry.cancelled
                    && slot
                        .browser
                        .as_ref()
                        .is_some_and(|browser| browser.snapshot_id == entry.snapshot)
            })
    }
    pub(super) fn password_prompt(
        &mut self,
        prompt: ArchivePasswordPrompt,
        cx: &mut Context<Self>,
    ) {
        if !self.password_current(&prompt, cx) {
            return;
        }
        if self.passwords.get(&prompt.operation_id) == Some(&prompt)
            && self.password_dialogs.contains_key(&prompt.operation_id)
        {
            return;
        }
        self.hide_password_dialog(prompt.operation_id, cx);
        let source = self.windows[&prompt.session_id].window;
        self.passwords.insert(prompt.operation_id, prompt.clone());
        if !self.tab_is_active(prompt.session_id, cx) {
            return;
        }
        let Some(host_id) = self.host.as_ref().map(|host| host.id) else {
            return;
        };
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            if !owner
                .read_with(cx, |app, cx| {
                    app.host.as_ref().is_some_and(|host| host.id == host_id)
                        && app.passwords.get(&prompt.operation_id) == Some(&prompt)
                        && app.password_current(&prompt, cx)
                        && app.tab_is_active(prompt.session_id, cx)
                })
                .unwrap_or(false)
            {
                return;
            }
            let alive = Rc::new(std::cell::Cell::new(true));
            let scope = Rc::new(std::cell::RefCell::new(None));
            let captured = source.update(cx, |_, window, cx| {
                if window.has_active_dialog(cx) { return None; }
                let input = cx.new(|cx| InputState::new(window, cx).masked(true));
                let retirement_input = input.clone();
                let owner = owner.clone();
                let prompt = prompt.clone();
                let dialog_alive = alive.clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    let confirm_owner = owner.clone();
                    let cancel_owner = owner.clone();
                    let confirm_prompt = prompt.clone();
                    let cancel_prompt = prompt.clone();
                    let confirm_input = input.clone();
                    let cancel_input = input.clone();
                    let confirm_alive = dialog_alive.clone();
                    let cancel_alive = dialog_alive.clone();
                    dialog
                        .title(if prompt.error_code == "WrongPassword" {
                            "密码错误，请重试"
                        } else {
                            "压缩包需要密码"
                        })
                        .child(Input::new(&input).aria_label("压缩包密码"))
                        .child("密码不会写入磁盘；解锁拖出的授权仅短期保留在受保护内存。完成后需重新按住拖动。")
                        .footer(
                            DialogFooter::new()
                                .child(DialogClose::new().trigger(|button| button.label("取消")))
                                .child(
                                    DialogAction::new()
                                        .child(Button::new("archive-password-ok").label("确定").debug_selector(||"archive-password-ok".into())),
                                ),
                        )
                        .on_ok(move |_, window, cx| {
                            if !confirm_alive.replace(false) { return true; }
                            let value = confirm_input.read(cx).value();
                            let secret = PrivatePassword::new(value.as_ref());
                            confirm_input.update(cx, |input, cx| input.set_value("", window, cx));
                            let owner = confirm_owner.clone();
                            let prompt = confirm_prompt.clone();
                            cx.defer(move |cx| {
                                let _ = owner.update(cx, |app, cx| {
                                    app.password_reply(prompt, Some(secret), cx)
                                });
                            });
                            true
                        })
                        .on_cancel(move |_, window, cx| {
                            if !cancel_alive.replace(false) { return true; }
                            cancel_input.update(cx, |input, cx| input.set_value("", window, cx));
                            let owner = cancel_owner.clone();
                            let prompt = cancel_prompt.clone();
                            cx.defer(move |cx| {
                                let _ = owner
                                    .update(cx, |app, cx| app.password_reply(prompt, None, cx));
                            });
                            true
                        })
                });
                *scope.borrow_mut() = window.focused(cx);
                Some(retirement_input)
            });
            if let Ok(Some(input)) = captured {
                let _ = owner.update(cx, |app, _| { app.password_dialogs.insert(prompt.operation_id, PasswordDialog {
                    session: prompt.session_id, window: source, alive, scope, input,
                }); });
            } else {
                let _ = owner.update(cx, |app, cx| app.queue_password_recovery(host_id, cx));
            }
        });
    }
    pub(super) fn password_reply(
        &mut self,
        prompt: ArchivePasswordPrompt,
        password: Option<PrivatePassword>,
        cx: &mut Context<Self>,
    ) {
        if self.passwords.get(&prompt.operation_id) != Some(&prompt)
            || !self.password_current(&prompt, cx)
        {
            return;
        }
        self.hide_password_dialog(prompt.operation_id, cx);
        self.passwords.remove(&prompt.operation_id);
        if let Some(origin) = self.windows.get(&prompt.session_id).and_then(|slot| {
            slot.view
                .read_with(cx, |view, _| view.authorization_password_origin(&prompt))
                .ok()
                .flatten()
        }) {
            let session = prompt.session_id;
            let cancelled = password.is_none();
            let failed = self
                .platform
                .send_archive_authorization_password(
                    ArchivePasswordReply { prompt, password },
                    origin,
                )
                .is_err();
            if (failed || cancelled)
                && let Some(slot) = self.windows.get(&session)
            {
                let view = slot.view.clone();
                cx.defer(move |cx| {
                    let _ = view.update(cx, |view, cx| {
                        view.invalidate_drag();
                        cx.notify();
                    });
                });
            }
            return;
        }
        let operation = prompt.operation_id;
        if self
            .platform
            .send_command_correlated(
                PlatformCommand::ArchivePasswordReply(ArchivePasswordReply {
                    prompt: prompt.clone(),
                    password,
                }),
                operation,
            )
            .is_err()
        {
            if let Some(read) = self.reads.get(&operation) {
                let _ = read.operation.cancel();
            }
            self.cancel_entries(prompt.session_id);
            tracing::warn!(event="archive.password.reply_failed",correlation_id=%operation,"archive password reply submission failed");
        }
    }
}

#[cfg(test)]
#[path = "wac_password_tests.rs"]
mod wac_password_tests;
