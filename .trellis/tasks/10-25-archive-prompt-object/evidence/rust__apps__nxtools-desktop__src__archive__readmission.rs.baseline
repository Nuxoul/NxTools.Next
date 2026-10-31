//! Adapter only: original Rust coordinator owns finite extraction output readmission policy.
use super::*;
use nxtools_archive::{
    OperationControl,
    interaction::{InteractionPort, Prompt, PromptResponse, PromptTicket},
};
use nxtools_protocol::archive_job::*;
use std::sync::mpsc::{SyncSender, sync_channel};
pub(super) struct Readmission {
    pub control: OperationControl,
    pub interaction: InteractionPort,
    pub attempt: Uuid,
    pub response: Option<SyncSender<ArchiveJobTerminal>>,
    pub last: Option<ArchiveJobTerminal>,
}
enum Delivery {
    Attempt(Box<ArchiveJobCommand>, SyncSender<ArchiveJobTerminal>),
    Finished(nxtools_archive::Result<ArchiveJobTerminal>),
}
impl ArchiveApplication {
    pub(super) fn start_readmission(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let Some(record) = self.jobs.records.get(&id) else {
            return;
        };
        let command = record.command.clone();
        let control = OperationControl::with_operation_id(id);
        let interaction = InteractionPort::new(id, 1);
        self.jobs.readmissions.insert(
            id,
            Readmission {
                control: control.clone(),
                interaction: interaction.clone(),
                attempt: id,
                response: None,
                last: None,
            },
        );
        let (sender, receiver) = async_channel::bounded(1);
        let coordinator = std::thread::Builder::new()
            .name("archive-readmission".into())
            .spawn(move || {
                let result = nxtools_archive::product_job::execute_with_output_readmission(
                    command,
                    &control,
                    &interaction,
                    |command, _| {
                        let (response, wait) = sync_channel(1);
                        sender
                            .send_blocking(Delivery::Attempt(Box::new(command), response))
                            .map_err(|_| nxtools_archive::ArchiveError::Cancelled)?;
                        wait.recv()
                            .map_err(|_| nxtools_archive::ArchiveError::IoFailure)
                    },
                );
                let _ = sender.send_blocking(Delivery::Finished(result));
            });
        if coordinator.is_err() {
            self.jobs.readmissions.remove(&id);
            self.job_terminal(
                id,
                super::jobs::local_terminal("coordinator_unavailable", true),
                cx,
            );
            return;
        }
        cx.spawn(async move |this, cx| {
            loop {
                let next = receiver.try_recv();
                match next {
                    Ok(Delivery::Attempt(command, response)) => {
                        let _ = this.update(cx, |app, cx| {
                            let Some(runtime) = app.jobs.readmissions.get_mut(&id) else {
                                return;
                            };
                            runtime.attempt = command.operation_id;
                            runtime.response = Some(response);
                            app.jobs.attempts.insert(command.operation_id, id);
                            app.send_job_attempt(id, *command, cx);
                        });
                    }
                    Ok(Delivery::Finished(result)) => {
                        let _ = this.update(cx, |app, cx| {
                            let Some(mut runtime) = app.jobs.readmissions.remove(&id) else {
                                return;
                            };
                            runtime.interaction.close();
                            let terminal = match result {
                                Ok(terminal) => terminal,
                                Err(error) => {
                                    let mut terminal = runtime.last.take().unwrap_or_else(|| {
                                        super::jobs::local_terminal("io_failure", false)
                                    });
                                    terminal.output_shortfall = None;
                                    if error == nxtools_archive::ArchiveError::Cancelled
                                        && terminal.code.as_deref() != Some("disconnected")
                                    {
                                        terminal.outcome = ArchiveJobOutcome::Cancelled;
                                        terminal.code = Some("cancelled".into());
                                    }
                                    terminal
                                }
                            };
                            app.job_terminal(id, terminal, cx);
                            app.pump_jobs(cx);
                        });
                        break;
                    }
                    Err(async_channel::TryRecvError::Closed) => break,
                    Err(async_channel::TryRecvError::Empty) => {}
                }
                let _ = this.update(cx, |app, cx| app.readmission_prompt(id, cx));
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(25))
                    .await;
            }
        })
        .detach();
    }
    fn readmission_prompt(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let Some(runtime) = self.jobs.readmissions.get(&id) else {
            return;
        };
        let Some(pending) = runtime.interaction.pending() else {
            return;
        };
        let Prompt::OutputBudget {
            current: _,
            required,
        } = pending.prompt
        else {
            return;
        };
        let prompt = ArchiveJobPrompt {
            ticket: ArchiveJobTicket {
                operation_id: pending.ticket.operation_id,
                prompt_id: pending.ticket.prompt_id,
                generation: pending.ticket.generation,
            },
            kind: ArchiveJobPromptKind::OutputBudget,
            error_code: Some(format!(
                "{}:{}",
                required.entry_bytes, required.output_bytes
            )),
            attempt: 1,
            existing_bytes: Some(required.entry_bytes),
            incoming_bytes: Some(required.output_bytes),
            approved_limit: Some(required.output_bytes),
        };
        let Some(record) = self.jobs.records.get_mut(&id) else {
            return;
        };
        if record.prompt.as_ref() == Some(&prompt) {
            return;
        }
        record.prompt = Some(prompt);
        record.state = "等待有限输出预算确认（前次已清理且零提交）".into();
        self.refresh_job(id, cx);
    }
    pub(super) fn reply_readmission(
        &mut self,
        id: Uuid,
        prompt: &ArchiveJobPrompt,
        approved: bool,
    ) -> bool {
        let Some(runtime) = self.jobs.readmissions.get(&id) else {
            return false;
        };
        let Some(pending) = runtime.interaction.pending() else {
            return false;
        };
        let ticket = PromptTicket {
            operation_id: prompt.ticket.operation_id,
            prompt_id: prompt.ticket.prompt_id,
            generation: prompt.ticket.generation,
        };
        if pending.ticket != ticket {
            return false;
        }
        let response = if approved {
            let Prompt::OutputBudget { required, .. } = pending.prompt else {
                return false;
            };
            PromptResponse::OutputBudget { approved: required }
        } else {
            PromptResponse::Cancel
        };
        runtime
            .interaction
            .respond(ticket, response)
            .unwrap_or(false)
    }
}

impl Drop for Readmission {
    fn drop(&mut self) {
        self.control.cancel();
        self.interaction.close();
        self.response.take();
    }
}
