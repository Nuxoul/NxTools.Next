use super::{ArchiveAction, ArchiveCommand, ArchiveReport};
use crate::{EXIT_HOST_UNAVAILABLE, EXIT_OPERATION_FAILED};
use nxtools_archive::product::ArchiveReadClient;
use nxtools_ipc::{PlatformBridge, PlatformNotification, platform_bridge, run_platform_server};
use nxtools_protocol::{PlatformEvent, archive_job::*, archive_read::*};
use serde_json::json;
use std::{
    path::Path,
    process::{Child, Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};
use uuid::Uuid;

const EXECUTION_DEADLINE: Duration = Duration::from_secs(45);
const CANCEL_DRAIN: Duration = Duration::from_secs(10);

pub(super) fn execute(command: ArchiveCommand, executable: &Path) -> ArchiveReport {
    let id = Uuid::new_v4();
    let action = command.action;
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(_) => {
            return ArchiveReport::failed(id, action, "runtime_unavailable", EXIT_HOST_UNAVAILABLE);
        }
    };
    let mut child = None;
    let mut result = runtime.block_on(run(command, executable, id, &mut child));
    // Drop ALL private pipe tasks before waiting for the headless child's original session drain.
    drop(runtime);
    if let Some(mut child) = child {
        let until = Instant::now() + CANCEL_DRAIN;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() && result.success {
                        result.success = false;
                        result.error = Some(super::ArchiveFailure {
                            code: "host_cleanup_failed".into(),
                            native_error_code: status.code(),
                        });
                        result.exit_code = EXIT_OPERATION_FAILED;
                    }
                    break;
                }
                Ok(None) if Instant::now() < until => std::thread::sleep(Duration::from_millis(20)),
                _ => {
                    // Only this invocation's child; never a name-based kill or unrelated product.
                    let _ = child.kill();
                    let _ = child.wait();
                    result.success = false;
                    result.error = Some(super::ArchiveFailure {
                        code: "host_cleanup_incomplete".into(),
                        native_error_code: None,
                    });
                    result.exit_code = EXIT_OPERATION_FAILED;
                    break;
                }
            }
        }
    }
    result
}

async fn run(
    mut command: ArchiveCommand,
    executable: &Path,
    id: Uuid,
    child: &mut Option<Child>,
) -> ArchiveReport {
    let action = command.action;
    for path in std::iter::once(&mut command.source).chain(command.destination.iter_mut()) {
        let Ok(absolute) = std::path::absolute(&*path) else {
            return ArchiveReport::failed(
                id,
                action,
                "invalid_request",
                crate::EXIT_REQUEST_REJECTED,
            );
        };
        let Some(text) = absolute.to_str() else {
            return ArchiveReport::failed(
                id,
                action,
                "invalid_request",
                crate::EXIT_REQUEST_REJECTED,
            );
        };
        let normalized = text.replace('/', "\\");
        if normalized.starts_with("\\\\") || normalized.chars().any(char::is_control) {
            return ArchiveReport::failed(
                id,
                action,
                "invalid_request",
                crate::EXIT_REQUEST_REJECTED,
            );
        }
        *path = normalized.into();
    }
    let Some(directory) = executable.parent() else {
        return ArchiveReport::failed(id, action, "host_unavailable", EXIT_HOST_UNAVAILABLE);
    };
    let host_path = directory.join("NxTools.PlatformHost.exe");
    if !host_path.is_file() {
        return ArchiveReport::failed(id, action, "host_unavailable", EXIT_HOST_UNAVAILABLE);
    }
    let pipe = format!("nxtools-archive-cli-{}", Uuid::new_v4());
    let (bridge, endpoint) = platform_bridge(32);
    let bridge = Arc::new(bridge);
    let read_client = match ArchiveReadClient::new(bridge.archive_client_bridge()) {
        Ok(client) => client,
        Err(_) => {
            return ArchiveReport::failed(id, action, "runtime_unavailable", EXIT_HOST_UNAVAILABLE);
        }
    };
    let notices = bridge.notification_receiver();
    let pipe_for_server = pipe.clone();
    let server = tokio::spawn(async move { run_platform_server(&pipe_for_server, endpoint).await });
    tokio::task::yield_now().await;
    let mut host_command = Command::new(host_path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        host_command.creation_flags(CREATE_NO_WINDOW);
    }
    *child = match host_command
        .args([
            "--archive-cli",
            "--parent-pid",
            &std::process::id().to_string(),
            "--pipe-name",
            &pipe,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(process) => Some(process),
        Err(_) => {
            return ArchiveReport::failed(id, action, "host_unavailable", EXIT_HOST_UNAVAILABLE);
        }
    };
    let connected = tokio::time::timeout(Duration::from_secs(5), async {
        while !bridge.is_connected() {
            if server.is_finished()
                || child
                    .as_mut()
                    .and_then(|p| p.try_wait().ok().flatten())
                    .is_some()
            {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        true
    })
    .await
    .unwrap_or(false);
    if !connected {
        return ArchiveReport::failed(id, action, "host_unavailable", EXIT_HOST_UNAVAILABLE);
    }
    if action == ArchiveAction::Extract {
        run_job(&command, id, &bridge, &notices).await
    } else {
        run_read(&command, id, &read_client, &notices).await
    }
}

async fn run_read(
    command: &ArchiveCommand,
    id: Uuid,
    client: &ArchiveReadClient,
    notices: &async_channel::Receiver<PlatformNotification>,
) -> ArchiveReport {
    let action = command.action;
    let request = ArchiveReadCommand {
        version: 1,
        operation_id: id,
        session_id: Uuid::new_v4(),
        generation: 1,
        action: if action == ArchiveAction::List {
            ArchiveReadAction::Open
        } else {
            ArchiveReadAction::Test
        },
        path: Some(command.source.to_string_lossy().into_owned()),
        codepage: command.codepage,
        limits: None,
        sequence: None,
    };
    let operation = match client.start(request) {
        Ok(operation) => operation,
        Err(code) => return read_error(id, action, code),
    };
    let mut entries = Vec::new();
    let mut snapshot = None;
    let mut format = None;
    let started = Instant::now();
    let mut cancel_at = None;
    let signal = tokio::signal::ctrl_c();
    tokio::pin!(signal);
    let mut signal_done = false;
    loop {
        if let Some(batch) = operation.take_batch() {
            let sequence = batch.sequence;
            snapshot = Some(batch.snapshot_id);
            format = Some(batch.format);
            entries.extend(batch.entries);
            if let Err(code) = operation.acknowledge(sequence) {
                return read_error(id, action, code);
            }
        }
        if let Some(terminal) = operation.terminal() {
            let success = terminal.status == ArchiveReadStatus::Succeeded
                && terminal.code.is_none()
                && terminal.cleanup_complete;
            let code = if !terminal.cleanup_complete {
                Some("cleanup_incomplete".into())
            } else {
                terminal.code.map(read_code)
            };
            let native = terminal.native_error_code;
            return ArchiveReport::with_terminal(
                id,
                action,
                json!({"snapshot_id":snapshot,"format":format,"entries":entries,"terminal":terminal}),
                success,
                code,
                native,
            );
        }
        if cancel_at.is_some_and(|at: Instant| at.elapsed() >= CANCEL_DRAIN) {
            return ArchiveReport::with_terminal(
                id,
                action,
                json!({"cleanup_complete":false}),
                false,
                Some("cleanup_incomplete".into()),
                None,
            );
        }
        if cancel_at.is_none() && started.elapsed() >= EXECUTION_DEADLINE {
            let _ = operation.cancel();
            cancel_at = Some(Instant::now());
        }
        tokio::select! {
            result = client.poll() => if let Err(code) = result { return read_error(id, action, code); },
            notice = notices.recv() => if notice.is_err() { return read_error(id, action, ArchiveReadError::Disconnected); },
            signal_result = &mut signal, if !signal_done => {
                signal_done = true;
                if signal_result.is_ok() && cancel_at.is_none() { let _ = operation.cancel(); cancel_at = Some(Instant::now()); }
            },
            _ = tokio::time::sleep(Duration::from_millis(10)) => {},
        }
    }
}
fn read_code(code: ArchiveReadError) -> String {
    serde_json::to_value(code)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| "io_failure".into())
}
fn read_error(id: Uuid, action: ArchiveAction, code: ArchiveReadError) -> ArchiveReport {
    ArchiveReport::with_terminal(
        id,
        action,
        json!({"cleanup_complete":false}),
        false,
        Some(read_code(code)),
        None,
    )
}

async fn run_job(
    command: &ArchiveCommand,
    id: Uuid,
    bridge: &PlatformBridge,
    notices: &async_channel::Receiver<PlatformNotification>,
) -> ArchiveReport {
    let session_id = Uuid::new_v4();
    let request = ArchiveJobCommand {
        version: 1,
        operation_id: id,
        session_id,
        generation: 1,
        action: ArchiveJobAction::Start,
        reply: None,
        batch_id: None,
        request: Some(ArchiveJobRequest {
            kind: ArchiveJobKind::Extract,
            paths: vec![command.source.to_string_lossy().into_owned()],
            destination: command
                .destination
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
            codepage: command.codepage,
            limits: ArchiveJobLimits::default(),
            extract: Some(ArchiveJobExtract {
                strategy: ArchiveJobStrategy::Here,
                single_file_folder: false,
                conflict: command.conflict,
                copy_motw: true,
                selection: None,
                reclamation: None,
            }),
            create: None,
            modify: None,
            readmission: None,
        }),
    };
    let Some(connection) = bridge.connection_generation() else {
        return ArchiveReport::failed(id, command.action, "disconnected", EXIT_HOST_UNAVAILABLE);
    };
    if request.validate().is_err() {
        return ArchiveReport::failed(
            id,
            command.action,
            "invalid_request",
            crate::EXIT_REQUEST_REJECTED,
        );
    }
    let start_receipt = Uuid::new_v4();
    if bridge
        .send_archive_job(request.clone(), start_receipt, connection)
        .is_err()
    {
        return ArchiveReport::failed(
            id,
            command.action,
            "host_unavailable",
            EXIT_HOST_UNAVAILABLE,
        );
    }
    let mut state = JobConsumer::new(request, command.create_destination);
    let started = Instant::now();
    let mut cancel_at = None;
    let signal = tokio::signal::ctrl_c();
    tokio::pin!(signal);
    let mut signal_done = false;
    loop {
        if cancel_at.is_some_and(|at: Instant| at.elapsed() >= CANCEL_DRAIN) {
            return ArchiveReport::with_terminal(
                id,
                command.action,
                json!({"cleanup_complete":false}),
                false,
                Some("cleanup_incomplete".into()),
                None,
            );
        }
        if started.elapsed() >= EXECUTION_DEADLINE && cancel_at.is_none() {
            let _ = bridge.send_archive_job(
                state.control(ArchiveJobAction::Cancel, None),
                Uuid::new_v4(),
                connection,
            );
            cancel_at = Some(Instant::now());
        }
        tokio::select! {
            notice = notices.recv() => match notice {
                Ok(PlatformNotification::Event(PlatformEvent::ArchiveJob(event))) => {
                    match state.consume(*event) {
                        Ok(JobStep::Continue) => {},
                        Ok(JobStep::Control(control)) => {
                            if control.action == ArchiveJobAction::Cancel { cancel_at.get_or_insert_with(Instant::now); }
                            if bridge.send_archive_job(*control, Uuid::new_v4(), connection).is_err() {
                                return ArchiveReport::with_terminal(id, command.action, json!({"cleanup_complete":false}), false, Some("disconnected".into()), None);
                            }
                        },
                        Ok(JobStep::Complete(report)) => return report,
                        Err(()) => {
                            let _ = bridge.send_archive_job(state.control(ArchiveJobAction::Cancel, None), Uuid::new_v4(), connection);
                            return ArchiveReport::with_terminal(id, command.action, json!({"cleanup_complete":false}), false, Some("protocol_violation".into()), None);
                        },
                    }
                },
                Ok(PlatformNotification::CommandResult { correlation_id, result }) if correlation_id == start_receipt && !result.success => {
                    // A receipt is not cleanup: the original session normally follows with a rejection terminal.
                    state.rejected_receipt = true;
                },
                Ok(PlatformNotification::Disconnected { .. }) | Err(_) => return ArchiveReport::with_terminal(id, command.action, json!({"cleanup_complete":false}), false, Some("disconnected".into()), None),
                _ => {},
            },
            signal_result = &mut signal, if !signal_done => {
                signal_done = true;
                if signal_result.is_ok() && cancel_at.is_none() {
                    let _ = bridge.send_archive_job(state.control(ArchiveJobAction::Cancel, None), Uuid::new_v4(), connection);
                    cancel_at = Some(Instant::now());
                }
            },
            _ = tokio::time::sleep(Duration::from_millis(10)) => {},
        }
    }
}

struct JobConsumer {
    request: ArchiveJobCommand,
    create_destination: bool,
    refusal: Option<String>,
    rejected_receipt: bool,
}
enum JobStep {
    Continue,
    Control(Box<ArchiveJobCommand>),
    Complete(ArchiveReport),
}
impl JobConsumer {
    fn new(request: ArchiveJobCommand, create_destination: bool) -> Self {
        Self {
            request,
            create_destination,
            refusal: None,
            rejected_receipt: false,
        }
    }
    fn control(
        &self,
        action: ArchiveJobAction,
        reply: Option<ArchiveJobReply>,
    ) -> ArchiveJobCommand {
        ArchiveJobCommand {
            action,
            request: None,
            reply,
            ..self.request.clone()
        }
    }
    fn consume(&mut self, event: ArchiveJobEvent) -> Result<JobStep, ()> {
        if event.operation_id != self.request.operation_id
            || event.session_id != self.request.session_id
            || event.generation != self.request.generation
        {
            return Ok(JobStep::Continue);
        }
        event.validate().map_err(|_| ())?;
        if let Some(prompt) = event.prompt {
            if self.refusal.is_some() {
                return Ok(JobStep::Continue);
            }
            if prompt.kind == ArchiveJobPromptKind::Destination
                && prompt.error_code.as_deref() == Some("create_destination")
                && self.create_destination
            {
                return Ok(JobStep::Control(Box::new(self.control(
                    ArchiveJobAction::Reply,
                    Some(ArchiveJobReply {
                        ticket: prompt.ticket,
                        kind: prompt.kind,
                        password: None,
                        decision: Some(ArchiveJobConflictDecision::Overwrite),
                        apply_to_all: false,
                        approved_limit: None,
                        remember_batch_password: false,
                    }),
                ))));
            }
            self.refusal = Some(
                match prompt.kind {
                    ArchiveJobPromptKind::Password => "password_required",
                    ArchiveJobPromptKind::Conflict => "output_collision",
                    ArchiveJobPromptKind::Destination
                        if prompt.error_code.as_deref() == Some("create_destination") =>
                    {
                        "destination_required"
                    }
                    _ => "interaction_required",
                }
                .into(),
            );
            return Ok(JobStep::Control(Box::new(
                self.control(ArchiveJobAction::Cancel, None),
            )));
        }
        if let Some(terminal) = event.terminal {
            let success = matches!(
                terminal.outcome,
                ArchiveJobOutcome::Succeeded | ArchiveJobOutcome::SucceededWithWarnings
            ) && terminal.code.is_none()
                && terminal.cleanup_complete
                && !self.rejected_receipt
                && self.refusal.is_none();
            let code = if !terminal.cleanup_complete {
                Some("cleanup_incomplete".into())
            } else if matches!(terminal.code.as_deref(), Some("cancelled") | None) {
                self.refusal.clone().or_else(|| terminal.code.clone())
            } else {
                terminal.code.clone()
            };
            let native = terminal.native_error_code;
            return Ok(JobStep::Complete(ArchiveReport::with_terminal(
                self.request.operation_id,
                ArchiveAction::Extract,
                json!({"terminal":terminal,"interaction_required":self.refusal}),
                success,
                code,
                native,
            )));
        }
        Ok(JobStep::Continue)
    }
}

#[cfg(test)]
mod tests;
