//! Browser prompt projection over the existing private worker rendezvous.
use super::*;
use gpui_kit::ParentElement;
use gpui_kit::component::{
    WindowExt,
    button::Button,
    dialog::{DialogAction, DialogClose, DialogFooter},
    input::{Input, InputState},
};
use nxtools_protocol::{
    PlatformCommand,
    archive_password::{ArchivePasswordPrompt, ArchivePasswordReply, PrivatePassword},
};

impl ArchiveApplication {
    fn password_current(&self, prompt: &ArchivePasswordPrompt) -> bool {
        let Some(slot) = self
            .windows
            .get(&prompt.session_id)
            .filter(|slot| slot.generation == prompt.generation)
        else {
            return false;
        };
        self.reads.get(&prompt.operation_id).is_some_and(|read| {
            read.window_id == prompt.session_id && read.generation == prompt.generation
        }) || self.entries.values().any(|entry| {
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
        if !self.password_current(&prompt)
            || self.passwords.get(&prompt.operation_id) == Some(&prompt)
        {
            return;
        }
        let source = self.windows[&prompt.session_id].window;
        self.passwords.insert(prompt.operation_id, prompt.clone());
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            if !owner
                .read_with(cx, |app, _| {
                    app.passwords.get(&prompt.operation_id) == Some(&prompt)
                        && app.password_current(&prompt)
                })
                .unwrap_or(false)
            {
                return;
            }
            let _ = source.update(cx, |_, window, cx| {
                let input = cx.new(|cx| InputState::new(window, cx).masked(true));
                let owner = owner.clone();
                let prompt = prompt.clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    let confirm_owner = owner.clone();
                    let cancel_owner = owner.clone();
                    let confirm_prompt = prompt.clone();
                    let cancel_prompt = prompt.clone();
                    let confirm_input = input.clone();
                    let cancel_input = input.clone();
                    dialog
                        .title(if prompt.error_code == "WrongPassword" {
                            "密码错误，请重试"
                        } else {
                            "归档需要密码"
                        })
                        .child(Input::new(&input).aria_label("归档密码"))
                        .child("密码仅用于当前操作，不会保存。取消不会启动文件。")
                        .footer(
                            DialogFooter::new()
                                .child(DialogClose::new().trigger(|button| button.label("取消")))
                                .child(
                                    DialogAction::new()
                                        .child(Button::new("archive-password-ok").label("确定")),
                                ),
                        )
                        .on_ok(move |_, window, cx| {
                            let value = confirm_input.read(cx).value();
                            let secret = PrivatePassword::new(value.as_ref());
                            confirm_input.update(cx, |input, cx| input.set_value("", window, cx));
                            let owner = confirm_owner.clone();
                            let prompt = confirm_prompt.clone();
                            cx.defer(move |cx| {
                                let _ = owner
                                    .update(cx, |app, _| app.password_reply(prompt, Some(secret)));
                            });
                            true
                        })
                        .on_cancel(move |_, window, cx| {
                            cancel_input.update(cx, |input, cx| input.set_value("", window, cx));
                            let owner = cancel_owner.clone();
                            let prompt = cancel_prompt.clone();
                            cx.defer(move |cx| {
                                let _ = owner.update(cx, |app, _| app.password_reply(prompt, None));
                            });
                            true
                        })
                });
            });
        });
    }
    pub(super) fn password_reply(
        &mut self,
        prompt: ArchivePasswordPrompt,
        password: Option<PrivatePassword>,
    ) {
        if self.passwords.get(&prompt.operation_id) != Some(&prompt)
            || !self.password_current(&prompt)
        {
            return;
        }
        self.passwords.remove(&prompt.operation_id);
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
