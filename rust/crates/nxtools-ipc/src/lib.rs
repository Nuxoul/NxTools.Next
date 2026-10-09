use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use async_channel::{Receiver, Sender};
use nxtools_protocol::{
    Envelope, HelloAck, MAX_FRAME_BYTES, Message, PlatformCommand, PlatformCommandResult,
    PlatformEvent, Pong,
};
use thiserror::Error;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(windows)]
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::Mutex as AsyncMutex;
use tracing::{info, warn};

#[cfg(windows)]
mod local_pipe;
#[cfg(windows)]
pub use local_pipe::{
    INDEXER_PIPE_NAME, LocalPipeConnection, accept_current_user_pipe, accept_local_indexer_pipe,
    connect_local_indexer_pipe, connect_local_pipe,
};

const PENDING_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const PENDING_COMMAND_SWEEP: Duration = Duration::from_secs(1);

struct PendingCommand {
    generation: u64,
    accepted_at: Instant,
    summon_reply: Option<Sender<PlatformCommandResult>>,
}

#[derive(Debug)]
struct PendingTerminal {
    id: uuid::Uuid,
    summon_reply: Option<Sender<PlatformCommandResult>>,
}

struct ActiveConnection {
    generation: u64,
    commands: Sender<Envelope>,
}

struct CommandRouter {
    capacity: usize,
    active: Option<ActiveConnection>,
    pending: HashMap<uuid::Uuid, PendingCommand>,
}

type ReplacedConnection = Option<(u64, Vec<PendingTerminal>)>;
type OpenConnection = (u64, Receiver<Envelope>, ReplacedConnection);

fn failed_platform_result(error: impl Into<String>) -> PlatformCommandResult {
    PlatformCommandResult {
        success: false,
        error: Some(error.into()),
        native_error_code: None,
        data: None,
    }
}
pub struct PlatformBridge {
    notifications: Receiver<PlatformNotification>,
    router: Arc<Mutex<CommandRouter>>,
    connected: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct PlatformServerEndpoint {
    notifications: Sender<PlatformNotification>,
    router: Arc<Mutex<CommandRouter>>,
    connected: Arc<AtomicBool>,
    connection_generation: Arc<AtomicU64>,
    contract_receiver: Arc<Mutex<Option<Receiver<Envelope>>>>,
    primary_connection_lock: Arc<AsyncMutex<()>>,
}

pub fn platform_bridge(capacity: usize) -> (PlatformBridge, PlatformServerEndpoint) {
    let (notification_sender, notification_receiver) = async_channel::bounded(capacity);
    let connected = Arc::new(AtomicBool::new(false));
    let connection_generation = Arc::new(AtomicU64::new(0));
    let router = Arc::new(Mutex::new(CommandRouter {
        capacity,
        active: None,
        pending: HashMap::new(),
    }));
    let primary_connection_lock = Arc::new(AsyncMutex::new(()));
    let contract_receiver = Arc::new(Mutex::new(None));
    (
        PlatformBridge {
            notifications: notification_receiver,
            router: Arc::clone(&router),
            connected: Arc::clone(&connected),
        },
        PlatformServerEndpoint {
            notifications: notification_sender,
            router,
            connected,
            connection_generation,
            contract_receiver,
            primary_connection_lock,
        },
    )
}

impl PlatformServerEndpoint {
    /// In-memory lifecycle seam used by GPUI/native-boundary contract tests.
    #[doc(hidden)]
    pub fn set_connected_for_contract_test(&self, connected: bool) {
        if connected {
            let (_, _, replaced) = self.open_connection();
            self.try_send_disconnected(replaced);
            return;
        }
        let generation = self
            .router
            .lock()
            .ok()
            .and_then(|router| router.active.as_ref().map(|active| active.generation));
        if let Some(generation) = generation {
            if let Some(pending_command_ids) = self.disconnect_connection(generation) {
                self.try_send_disconnected(Some((generation, pending_command_ids)));
            }
        } else {
            self.connected.store(false, Ordering::Release);
        }
    }

    /// Opens an isolated command queue for a contract test or a live pipe connection.
    #[doc(hidden)]
    pub fn open_connection_for_contract_test(&self) -> Receiver<Envelope> {
        let (_, receiver, replaced) = self.open_connection();
        self.try_send_disconnected(replaced);
        receiver
    }

    /// Receives one command from the active contract-test connection queue.
    #[doc(hidden)]
    pub fn try_receive_command_for_contract_test(&self) -> Result<Envelope, String> {
        let receiver = self
            .contract_receiver
            .lock()
            .map_err(|error| error.to_string())?
            .clone()
            .ok_or_else(|| "no contract-test connection is open".to_owned())?;
        receiver.try_recv().map_err(|error| error.to_string())
    }

    /// Publishes one observed notification through the same bounded transport as PlatformHost.
    #[doc(hidden)]
    pub fn send_notification_for_contract_test(
        &self,
        notification: PlatformNotification,
    ) -> Result<(), String> {
        self.notifications
            .try_send(notification)
            .map_err(|error| error.to_string())
    }

    fn open_connection(&self) -> OpenConnection {
        let Ok(mut router) = self.router.lock() else {
            self.connected.store(false, Ordering::Release);
            let generation = self
                .connection_generation
                .fetch_add(1, Ordering::AcqRel)
                .wrapping_add(1);
            let (_, receiver) = async_channel::bounded(0);
            return (generation, receiver, None);
        };
        let generation = self
            .connection_generation
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1);
        let (sender, receiver) = async_channel::bounded(router.capacity);
        let replaced = router.active.take().map(|active| {
            let pending_command_ids = router
                .pending
                .extract_if(|_, pending| pending.generation == active.generation)
                .map(|(id, pending)| PendingTerminal {
                    id,
                    summon_reply: pending.summon_reply,
                })
                .collect::<Vec<_>>();
            (active.generation, pending_command_ids)
        });
        router.active = Some(ActiveConnection {
            generation,
            commands: sender,
        });
        drop(router);
        self.connected.store(true, Ordering::Release);
        if let Ok(mut contract_receiver) = self.contract_receiver.lock() {
            *contract_receiver = Some(receiver.clone());
        }
        (generation, receiver, replaced)
    }

    fn try_send_disconnected(&self, disconnected: ReplacedConnection) {
        if let Some((generation, pending)) = disconnected {
            let pending_command_ids = pending.iter().map(|pending| pending.id).collect::<Vec<_>>();
            for pending_command in pending {
                if let Some(reply) = pending_command.summon_reply {
                    let _ = reply.try_send(failed_platform_result("platform host disconnected"));
                }
            }
            let _ = self
                .notifications
                .try_send(PlatformNotification::Disconnected {
                    generation,
                    pending_command_ids,
                });
        }
    }

    async fn send_notification(&self, notification: PlatformNotification) -> Result<()> {
        self.notifications
            .send(notification)
            .await
            .context("desktop platform notification receiver closed")
    }

    async fn send_summon_command(
        &self,
        envelope: Envelope,
        reply: Sender<PlatformCommandResult>,
    ) -> Result<()> {
        let command_id = envelope.id;
        let mut router = self
            .router
            .lock()
            .map_err(|_| anyhow::anyhow!("platform command router is poisoned"))?;
        let active = router
            .active
            .as_ref()
            .map(|active| (active.generation, active.commands.clone()))
            .context("platform host is disconnected")?;
        router.pending.insert(
            command_id,
            PendingCommand {
                generation: active.0,
                accepted_at: Instant::now(),
                summon_reply: Some(reply),
            },
        );
        match active.1.try_send(envelope) {
            Ok(()) => Ok(()),
            Err(async_channel::TrySendError::Full(_)) => {
                router.pending.remove(&command_id);
                Err(anyhow::anyhow!("platform host command queue is full"))
            }
            Err(async_channel::TrySendError::Closed(_)) => {
                router.pending.remove(&command_id);
                Err(anyhow::anyhow!("platform host command queue is closed"))
            }
        }
    }

    fn complete_command(&self, generation: u64, command_id: uuid::Uuid) -> Option<PendingCommand> {
        self.router.lock().ok().and_then(|mut router| {
            router
                .pending
                .get(&command_id)
                .is_some_and(|pending| pending.generation == generation)
                .then(|| {
                    router
                        .pending
                        .remove(&command_id)
                        .expect("pending command exists")
                })
        })
    }

    fn disconnect_connection(&self, generation: u64) -> Option<Vec<PendingTerminal>> {
        let pending = self.router.lock().ok().and_then(|mut router| {
            if router
                .active
                .as_ref()
                .is_none_or(|active| active.generation != generation)
            {
                return None;
            }
            router.active = None;
            Some(
                router
                    .pending
                    .extract_if(|_, pending| pending.generation == generation)
                    .map(|(id, pending)| PendingTerminal {
                        id,
                        summon_reply: pending.summon_reply,
                    })
                    .collect::<Vec<_>>(),
            )
        });
        if pending.is_some() {
            self.connected.store(false, Ordering::Release);
        }
        pending
    }

    fn is_active_generation(&self, generation: u64) -> bool {
        self.router.lock().ok().is_some_and(|router| {
            router
                .active
                .as_ref()
                .is_some_and(|active| active.generation == generation)
        })
    }

    fn expire_pending_commands(&self, generation: u64, now: Instant) -> Vec<PendingTerminal> {
        self.router
            .lock()
            .map(|mut router| {
                router
                    .pending
                    .extract_if(|_, pending| {
                        pending.generation == generation
                            && now.duration_since(pending.accepted_at) >= PENDING_COMMAND_TIMEOUT
                    })
                    .map(|(id, pending)| PendingTerminal {
                        id,
                        summon_reply: pending.summon_reply,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    async fn send_timeout_results(&self, pending: Vec<PendingTerminal>) -> Result<()> {
        for pending in pending {
            let result = failed_platform_result("platform command timed out");
            if let Some(reply) = pending.summon_reply {
                let _ = reply.send(result).await;
            } else {
                self.send_notification(PlatformNotification::CommandResult {
                    correlation_id: pending.id,
                    result,
                })
                .await?;
            }
        }
        Ok(())
    }

    async fn send_disconnected_terminals(
        &self,
        generation: u64,
        pending: Vec<PendingTerminal>,
    ) -> Result<()> {
        let pending_command_ids = pending.iter().map(|pending| pending.id).collect();
        for pending in pending {
            if let Some(reply) = pending.summon_reply {
                let _ = reply
                    .send(failed_platform_result("platform host disconnected"))
                    .await;
            }
        }
        self.send_notification(PlatformNotification::Disconnected {
            generation,
            pending_command_ids,
        })
        .await
    }
}

impl PlatformBridge {
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    pub fn notification_receiver(&self) -> Receiver<PlatformNotification> {
        self.notifications.clone()
    }

    pub fn send_command(
        &self,
        command: PlatformCommand,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        self.send_command_envelope(Envelope::new(Message::PlatformCommand(command)))
    }

    /// Sends a command using an ID already installed in caller-owned pending state.
    pub fn send_command_with_id(
        &self,
        command: PlatformCommand,
        command_id: uuid::Uuid,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        let mut envelope = Envelope::new(Message::PlatformCommand(command));
        envelope.id = command_id;
        self.send_command_envelope(envelope)
    }

    pub fn send_command_correlated(
        &self,
        command: PlatformCommand,
        correlation_id: uuid::Uuid,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        let mut envelope = Envelope::new(Message::PlatformCommand(command));
        envelope.correlation_id = Some(correlation_id);
        self.send_command_envelope(envelope)
    }

    fn send_command_envelope(&self, envelope: Envelope) -> Result<uuid::Uuid, PlatformBridgeError> {
        let command_id = envelope.id;
        let mut router = self
            .router
            .lock()
            .map_err(|_| PlatformBridgeError::Disconnected)?;
        if router.pending.contains_key(&command_id) {
            return Err(PlatformBridgeError::DuplicateCommandId);
        }
        let active = router
            .active
            .as_ref()
            .map(|active| (active.generation, active.commands.clone()))
            .ok_or(PlatformBridgeError::Disconnected)?;
        router.pending.insert(
            command_id,
            PendingCommand {
                generation: active.0,
                accepted_at: Instant::now(),
                summon_reply: None,
            },
        );
        match active.1.try_send(envelope) {
            Ok(()) => Ok(command_id),
            Err(async_channel::TrySendError::Full(_)) => {
                router.pending.remove(&command_id);
                Err(PlatformBridgeError::Busy)
            }
            Err(async_channel::TrySendError::Closed(_)) => {
                router.pending.remove(&command_id);
                Err(PlatformBridgeError::Disconnected)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformNotification {
    Connected,
    Disconnected {
        generation: u64,
        pending_command_ids: Vec<uuid::Uuid>,
    },
    Event(PlatformEvent),
    CommandResult {
        correlation_id: uuid::Uuid,
        result: PlatformCommandResult,
    },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlatformBridgeError {
    #[error("the Windows platform host is disconnected")]
    Disconnected,
    #[error("the Windows platform command queue is full")]
    Busy,
    #[error("the caller-supplied platform command ID is already pending")]
    DuplicateCommandId,
}

pub fn dispatch(message: &Envelope) -> Option<Envelope> {
    let response = match &message.message {
        Message::Hello(hello) => {
            const SUPPORTED_CAPABILITIES: [&str; 30] = [
                "foreground_context",
                "clipboard_text_capture",
                "clipboard_image_capture",
                "clipboard_files_capture",
                "clipboard_native_formats",
                "paste_text",
                "paste_image",
                "paste_files",
                "clipboard_panel_hotkey",
                "clipboard_configuration",
                "quick_launch_panel_hotkey",
                "quick_launch_configuration",
                "clipboard_archive_picker",
                "clipboard_storage_picker",
                "window_topmost",
                "window_no_activate",
                "window_bounds",
                "taskbar_progress",
                "taskbar_progress_overlay",
                "window_visibility",
                "window_minimized",
                "window_position",
                "shell_icon_extraction",
                "shell_context_actions",
                "directory_navigation",
                "file_dialog_search",
                "window_activation",
                "indexer_service_management",
                "desktop_shell",
                "tray_events",
            ];
            let accepted_capabilities = hello
                .capabilities
                .iter()
                .filter(|capability| SUPPORTED_CAPABILITIES.contains(&capability.as_str()))
                .cloned()
                .collect();
            info!(
                client_name = %hello.client_name,
                client_version = %hello.client_version,
                "platform host handshake accepted"
            );
            Message::HelloAck(HelloAck {
                server_name: "nxtools_desktop".into(),
                server_version: env!("CARGO_PKG_VERSION").into(),
                accepted_capabilities,
            })
        }
        Message::Ping(ping) => Message::Pong(Pong {
            sent_at_unix_ms: ping.sent_at_unix_ms,
        }),
        Message::PlatformEvent(event) => {
            info!(?event, "platform event received");
            return None;
        }
        Message::HelloAck(_)
        | Message::Pong(_)
        | Message::PlatformCommand(_)
        | Message::PlatformCommandResult(_)
        | Message::IndexerCommand(_)
        | Message::IndexerEvent(_)
        | Message::Error(_) => return None,
    };
    Some(Envelope::reply_to(message, response))
}

#[cfg(windows)]
pub async fn run_platform_server(pipe_name: &str, endpoint: PlatformServerEndpoint) -> Result<()> {
    let pipe_path = format!(r"\\.\pipe\{pipe_name}");
    info!(%pipe_path, "platform IPC server started");

    loop {
        let server = ServerOptions::new()
            .create(&pipe_path)
            .with_context(|| format!("failed to create named pipe {pipe_path}"))?;
        server
            .connect()
            .await
            .with_context(|| format!("failed to accept named pipe {pipe_path}"))?;

        let connection_endpoint = endpoint.clone();
        tokio::spawn(async move {
            if let Err(error) = handle_connection(server, &connection_endpoint).await {
                warn!(%error, "platform connection ended with an error");
            }
        });
    }
}

#[cfg(not(windows))]
pub async fn run_platform_server(
    _pipe_name: &str,
    _endpoint: PlatformServerEndpoint,
) -> Result<()> {
    anyhow::bail!("the platform host named pipe is only available on Windows")
}

#[cfg(windows)]
async fn handle_connection(
    server: NamedPipeServer,
    endpoint: &PlatformServerEndpoint,
) -> Result<()> {
    info!("platform host connected");
    let (reader, mut writer) = tokio::io::split(server);
    let mut reader = BufReader::new(reader);

    let Some(handshake_frame) = read_bounded_frame(&mut reader).await? else {
        anyhow::bail!("platform host disconnected before handshake");
    };
    let handshake = Envelope::decode_line(&handshake_frame).context("invalid handshake frame")?;
    let Message::Hello(hello) = &handshake.message else {
        anyhow::bail!("platform host did not begin with a hello message");
    };
    let acknowledgement =
        dispatch(&handshake).context("hello did not produce an acknowledgement")?;
    write_envelope(&mut writer, &acknowledgement).await?;

    if hello.client_name == "nxtools-desktop-summon" {
        return handle_summon_connection(reader, &mut writer, endpoint).await;
    }
    let _primary_connection_guard = endpoint.primary_connection_lock.lock().await;

    let (generation, command_receiver, replaced) = endpoint.open_connection();
    if let Some((old_generation, pending)) = replaced
        && let Err(error) = endpoint
            .send_disconnected_terminals(old_generation, pending)
            .await
    {
        let _ = endpoint.disconnect_connection(generation);
        return Err(error);
    }
    if let Err(error) = endpoint
        .send_notification(PlatformNotification::Connected)
        .await
    {
        let _ = endpoint.disconnect_connection(generation);
        return Err(error);
    }

    let mut sweep = tokio::time::interval(PENDING_COMMAND_SWEEP);
    let connection_result: Result<()> = async {
        loop {
            tokio::select! {
                _ = sweep.tick() => {
                    let expired = endpoint.expire_pending_commands(generation, Instant::now());
                    endpoint.send_timeout_results(expired).await?;
                }
                frame = read_bounded_frame(&mut reader) => {
                    let Some(frame) = frame? else {
                        info!(generation, "platform host disconnected");
                        break Ok(());
                    };
                    let envelope = Envelope::decode_line(&frame).context("invalid platform frame")?;
                    match &envelope.message {
                        Message::PlatformEvent(event) => {
                            if endpoint.is_active_generation(generation) {
                                endpoint
                                    .send_notification(PlatformNotification::Event(event.clone()))
                                    .await?;
                            } else {
                                warn!(generation, "ignored platform event from a stale connection");
                            }
                        }
                        Message::PlatformCommandResult(result) => {
                            let correlation_id = envelope
                                .correlation_id
                                .context("platform command result had no correlation ID")?;
                            if let Some(pending) = endpoint.complete_command(generation, correlation_id) {
                                if let Some(reply) = pending.summon_reply {
                                    let _ = reply.send(result.clone()).await;
                                } else {
                                    endpoint
                                        .send_notification(PlatformNotification::CommandResult {
                                            correlation_id,
                                            result: result.clone(),
                                        })
                                        .await?;
                                }
                            } else {
                                warn!(
                                    generation,
                                    %correlation_id,
                                    "ignored platform command result for a stale or unknown command"
                                );
                            }
                        }
                        _ => {
                            if let Some(response) = dispatch(&envelope) {
                                write_envelope(&mut writer, &response).await?;
                            }
                        }
                    }
                }
                command = command_receiver.recv() => {
                    let envelope = command.context("desktop platform command sender closed")?;
                    write_envelope(&mut writer, &envelope).await?;
                }
            }
        }
    }
    .await;

    if let Some(pending) = endpoint.disconnect_connection(generation) {
        let disconnect_result = endpoint
            .send_disconnected_terminals(generation, pending)
            .await;
        return connection_result.and(disconnect_result);
    }
    connection_result
}

#[cfg(windows)]
async fn handle_summon_connection(
    mut reader: BufReader<tokio::io::ReadHalf<NamedPipeServer>>,
    writer: &mut tokio::io::WriteHalf<NamedPipeServer>,
    endpoint: &PlatformServerEndpoint,
) -> Result<()> {
    loop {
        let Some(frame) = read_bounded_frame(&mut reader).await? else {
            return Ok(());
        };
        let envelope = Envelope::decode_line(&frame).context("invalid summon frame")?;
        if matches!(envelope.message, Message::PlatformCommand(_)) {
            let request = envelope.clone();
            let (reply_sender, reply_receiver) = async_channel::bounded(1);
            let result = match endpoint.send_summon_command(envelope, reply_sender).await {
                Ok(()) => reply_receiver
                    .recv()
                    .await
                    .unwrap_or_else(|_| failed_platform_result("summon result channel closed")),
                Err(error) => failed_platform_result(error.to_string()),
            };
            let response = Envelope::reply_to(&request, Message::PlatformCommandResult(result));
            write_envelope(writer, &response).await?;
        }
    }
}

#[cfg(windows)]
async fn write_envelope<W>(writer: &mut W, envelope: &Envelope) -> Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    writer
        .write_all(envelope.encode_line()?.as_bytes())
        .await
        .context("failed to write platform frame")?;
    writer
        .flush()
        .await
        .context("failed to flush platform frame")
}

async fn read_bounded_frame<R>(reader: &mut R) -> Result<Option<String>>
where
    R: AsyncBufRead + Unpin,
{
    let mut frame = Vec::with_capacity(4096);
    loop {
        let available = reader.fill_buf().await.context("failed to read frame")?;
        if available.is_empty() {
            if frame.is_empty() {
                return Ok(None);
            }
            anyhow::bail!("platform host disconnected in the middle of a frame");
        }

        let newline = available.iter().position(|byte| *byte == b'\n');
        let payload_bytes = newline.unwrap_or(available.len());
        if frame.len() + payload_bytes > MAX_FRAME_BYTES {
            anyhow::bail!("platform host sent a frame larger than {MAX_FRAME_BYTES} bytes");
        }
        frame.extend_from_slice(&available[..payload_bytes]);
        reader.consume(payload_bytes + usize::from(newline.is_some()));

        if newline.is_some() {
            return String::from_utf8(frame)
                .map(Some)
                .context("platform host frame was not valid UTF-8");
        }
    }
}

pub fn unix_time_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

pub fn read_bounded_sync_frame(reader: &mut impl BufRead) -> Result<Option<String>> {
    let mut frame = Vec::with_capacity(4096);
    loop {
        let available = reader.fill_buf().context("failed to read IPC frame")?;
        if available.is_empty() {
            if frame.is_empty() {
                return Ok(None);
            }
            anyhow::bail!("IPC peer disconnected in the middle of a frame");
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let payload_bytes = newline.unwrap_or(available.len());
        if frame.len() + payload_bytes > MAX_FRAME_BYTES {
            anyhow::bail!("IPC peer sent a frame larger than {MAX_FRAME_BYTES} bytes");
        }
        frame.extend_from_slice(&available[..payload_bytes]);
        reader.consume(payload_bytes + usize::from(newline.is_some()));
        if newline.is_some() {
            return String::from_utf8(frame)
                .map(Some)
                .context("IPC frame was not valid UTF-8");
        }
    }
}

pub fn write_envelope_sync(writer: &mut impl Write, envelope: &Envelope) -> Result<()> {
    writer
        .write_all(envelope.encode_line()?.as_bytes())
        .context("failed to write IPC frame")?;
    writer.flush().context("failed to flush IPC frame")
}

#[cfg(test)]
mod tests {
    use nxtools_protocol::{Hello, Message, PasteTextCommand, Ping, PlatformCommand};
    use tokio::io::BufReader;

    use super::*;
    fn summon_request(text: &str) -> Envelope {
        Envelope::new(Message::PlatformCommand(PlatformCommand::PasteText(
            PasteTextCommand {
                text: text.into(),
                paste_after_write: true,
                target_window_handle: 42,
                target_focus_window_handle: 43,
            },
        )))
    }

    #[tokio::test]
    async fn summon_pending_routes_the_actual_result_with_original_request_id() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let mut request = summon_request("summon actual result");
        let original_request_id = request.id;
        request.correlation_id = Some(uuid::Uuid::new_v4());
        let (reply_sender, reply_receiver) = async_channel::bounded(1);

        server
            .send_summon_command(request.clone(), reply_sender)
            .await
            .expect("queue summon command");
        assert_eq!(
            server
                .try_receive_command_for_contract_test()
                .expect("summon queue command")
                .id,
            original_request_id
        );
        let generation = server.connection_generation.load(Ordering::Acquire);
        let pending = server
            .complete_command(generation, original_request_id)
            .expect("summon pending command");
        let actual_result = PlatformCommandResult {
            success: false,
            error: Some("C# native operation failed".into()),
            native_error_code: Some(5),
            data: None,
        };
        pending
            .summon_reply
            .expect("summon reply channel")
            .send(actual_result.clone())
            .await
            .expect("deliver summon result");
        let received = reply_receiver.recv().await.expect("receive summon result");
        let response =
            Envelope::reply_to(&request, Message::PlatformCommandResult(received.clone()));
        assert_eq!(response.correlation_id, Some(original_request_id));
        assert_eq!(received, actual_result);
        assert!(!bridge.is_connected() || server.is_active_generation(generation));
    }

    #[tokio::test]
    async fn summon_queue_rejection_returns_failure_without_pending_state() {
        let (bridge, server) = platform_bridge(1);
        server.set_connected_for_contract_test(true);
        bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "fill queue".into(),
                paste_after_write: true,
                target_window_handle: 1,
                target_focus_window_handle: 2,
            }))
            .expect("fill primary queue");
        let request = summon_request("queue rejection");
        let (reply_sender, _reply_receiver) = async_channel::bounded(1);
        let error = server
            .send_summon_command(request, reply_sender)
            .await
            .expect_err("full summon queue must reject");
        assert!(error.to_string().contains("queue is full"));
    }

    #[tokio::test]
    async fn summon_disconnect_and_timeout_deliver_terminal_failures() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let request = summon_request("disconnect");
        let (reply_sender, reply_receiver) = async_channel::bounded(1);
        server
            .send_summon_command(request.clone(), reply_sender)
            .await
            .expect("queue disconnect summon");
        let generation = server.connection_generation.load(Ordering::Acquire);
        let pending = server
            .disconnect_connection(generation)
            .expect("disconnect active connection");
        server
            .send_disconnected_terminals(generation, pending)
            .await
            .expect("send disconnect terminals");
        let disconnect_failure = reply_receiver.recv().await.expect("disconnect failure");
        assert!(!disconnect_failure.success);
        assert_eq!(
            disconnect_failure.error.as_deref(),
            Some("platform host disconnected")
        );
        assert!(matches!(
            bridge.notification_receiver().recv().await.expect("disconnect notice"),
            PlatformNotification::Disconnected { generation: 1, pending_command_ids }
                if pending_command_ids == vec![request.id]
        ));

        server.set_connected_for_contract_test(true);
        let timeout_request = summon_request("timeout");
        let (timeout_sender, timeout_receiver) = async_channel::bounded(1);
        server
            .send_summon_command(timeout_request, timeout_sender)
            .await
            .expect("queue timeout summon");
        let generation = server.connection_generation.load(Ordering::Acquire);
        let pending = server.expire_pending_commands(
            generation,
            Instant::now() + PENDING_COMMAND_TIMEOUT + Duration::from_millis(1),
        );
        server
            .send_timeout_results(pending)
            .await
            .expect("send timeout terminal");
        let timeout_failure = timeout_receiver.recv().await.expect("timeout failure");
        assert!(!timeout_failure.success);
        assert_eq!(
            timeout_failure.error.as_deref(),
            Some("platform command timed out")
        );
    }

    #[tokio::test]
    async fn stale_generation_cannot_complete_a_new_generation_summon_command() {
        let (_bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let old_request = summon_request("old generation");
        let (reply_sender, _reply_receiver) = async_channel::bounded(1);
        server
            .send_summon_command(old_request.clone(), reply_sender)
            .await
            .expect("queue old summon command");
        let old_generation = server.connection_generation.load(Ordering::Acquire);
        server.open_connection_for_contract_test();
        assert!(
            server
                .complete_command(old_generation, old_request.id)
                .is_none()
        );
    }

    #[test]
    fn hello_negotiates_requested_capabilities() {
        let request = Envelope::new(Message::Hello(Hello {
            client_name: "platform_host".into(),
            client_version: "0.1.0".into(),
            capabilities: vec![
                "clipboard_text_capture".into(),
                "unknown".into(),
                "paste_text".into(),
                "clipboard_panel_hotkey".into(),
                "clipboard_configuration".into(),
                "quick_launch_panel_hotkey".into(),
                "quick_launch_configuration".into(),
                "foreground_context".into(),
                "window_topmost".into(),
                "window_visibility".into(),
                "window_minimized".into(),
                "window_position".into(),
                "shell_context_actions".into(),
                "directory_navigation".into(),
                "file_dialog_search".into(),
            ],
        }));
        let response = dispatch(&request).expect("hello response");

        let Message::HelloAck(ack) = response.message else {
            panic!("expected hello acknowledgement")
        };
        assert_eq!(
            ack.accepted_capabilities,
            vec![
                "clipboard_text_capture",
                "paste_text",
                "clipboard_panel_hotkey",
                "clipboard_configuration",
                "quick_launch_panel_hotkey",
                "quick_launch_configuration",
                "foreground_context",
                "window_topmost",
                "window_visibility",
                "window_minimized",
                "window_position",
                "shell_context_actions",
                "directory_navigation",
                "file_dialog_search"
            ]
        );
        assert_eq!(response.correlation_id, Some(request.id));
    }

    #[test]
    fn ping_preserves_timestamp() {
        let request = Envelope::new(Message::Ping(Ping {
            sent_at_unix_ms: 42,
        }));
        let response = dispatch(&request).expect("pong response");
        assert!(matches!(
            response.message,
            Message::Pong(Pong {
                sent_at_unix_ms: 42
            })
        ));
    }

    #[test]
    fn bridge_rejects_commands_while_platform_host_is_disconnected() {
        let (bridge, _server) = platform_bridge(4);
        let error = bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "must not become stale".into(),
                paste_after_write: true,
                target_window_handle: 42,
                target_focus_window_handle: 43,
            }))
            .expect_err("disconnected command must be rejected");
        assert!(matches!(error, PlatformBridgeError::Disconnected));
    }

    #[test]
    fn caller_supplied_command_id_is_used_for_result_routing() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let command_id = uuid::Uuid::new_v4();
        let result = PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: None,
        };

        assert_eq!(
            bridge
                .send_command_with_id(
                    PlatformCommand::PasteText(PasteTextCommand {
                        text: "routed".into(),
                        paste_after_write: true,
                        target_window_handle: 42,
                        target_focus_window_handle: 43,
                    }),
                    command_id,
                )
                .expect("caller-ID command"),
            command_id
        );
        let envelope = server
            .try_receive_command_for_contract_test()
            .expect("queued command");
        assert_eq!(envelope.id, command_id);

        server
            .send_notification_for_contract_test(PlatformNotification::CommandResult {
                correlation_id: command_id,
                result,
            })
            .expect("publish command result");
        assert!(matches!(
            bridge.notification_receiver().try_recv(),
            Ok(PlatformNotification::CommandResult { correlation_id, .. }) if correlation_id == command_id
        ));
    }

    #[test]
    fn caller_supplied_command_id_cannot_replace_an_inflight_command() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let command_id = uuid::Uuid::new_v4();
        let command = || {
            PlatformCommand::PasteText(PasteTextCommand {
                text: "same id".into(),
                paste_after_write: true,
                target_window_handle: 42,
                target_focus_window_handle: 43,
            })
        };
        bridge
            .send_command_with_id(command(), command_id)
            .expect("first command");

        assert_eq!(
            bridge.send_command_with_id(command(), command_id),
            Err(PlatformBridgeError::DuplicateCommandId)
        );
        assert_eq!(
            server
                .try_receive_command_for_contract_test()
                .expect("original queued command")
                .id,
            command_id
        );
        assert!(server.try_receive_command_for_contract_test().is_err());
    }

    #[test]
    fn bridge_preserves_a_caller_owned_lifecycle_correlation() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let lifecycle_id = uuid::Uuid::new_v4();

        bridge
            .send_command_correlated(
                PlatformCommand::PasteText(PasteTextCommand {
                    text: "correlated".into(),
                    paste_after_write: false,
                    target_window_handle: 42,
                    target_focus_window_handle: 43,
                }),
                lifecycle_id,
            )
            .expect("connected command");

        let envelope = server
            .try_receive_command_for_contract_test()
            .expect("queued command");
        assert_eq!(envelope.correlation_id, Some(lifecycle_id));
        assert_ne!(envelope.id, lifecycle_id);
    }

    #[test]
    fn old_connection_queue_is_not_consumed_by_a_new_connection() {
        let (bridge, server) = platform_bridge(4);
        let old_receiver = server.open_connection_for_contract_test();
        let old_id = bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "old".into(),
                paste_after_write: true,
                target_window_handle: 1,
                target_focus_window_handle: 2,
            }))
            .expect("old command");

        let new_receiver = server.open_connection_for_contract_test();
        let new_id = bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "new".into(),
                paste_after_write: true,
                target_window_handle: 3,
                target_focus_window_handle: 4,
            }))
            .expect("new command");

        assert_eq!(
            old_receiver.try_recv().expect("old queue command").id,
            old_id
        );
        assert_eq!(
            new_receiver.try_recv().expect("new queue command").id,
            new_id
        );
        assert!(old_receiver.try_recv().is_err());
    }

    #[test]
    fn disconnect_emits_pending_command_ids_for_the_active_generation() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let command_id = bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "disconnect".into(),
                paste_after_write: true,
                target_window_handle: 5,
                target_focus_window_handle: 6,
            }))
            .expect("pending command");

        server.set_connected_for_contract_test(false);
        let notification = bridge
            .notification_receiver()
            .try_recv()
            .expect("disconnect notification");
        assert!(matches!(
            notification,
            PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids
            } if pending_command_ids == vec![command_id]
        ));
        assert!(!bridge.is_connected());
    }

    #[test]
    fn full_command_queue_returns_busy_without_leaking_pending_state() {
        let (bridge, server) = platform_bridge(1);
        server.set_connected_for_contract_test(true);
        bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "fills queue".into(),
                paste_after_write: true,
                target_window_handle: 7,
                target_focus_window_handle: 8,
            }))
            .expect("first command");
        let error = bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "busy".into(),
                paste_after_write: true,
                target_window_handle: 9,
                target_focus_window_handle: 10,
            }))
            .expect_err("full queue must reject the command");
        assert_eq!(error, PlatformBridgeError::Busy);
    }

    #[tokio::test]
    async fn pending_command_timeout_emits_a_correlated_failure() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let command_id = bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "timeout".into(),
                paste_after_write: true,
                target_window_handle: 11,
                target_focus_window_handle: 12,
            }))
            .expect("pending command");

        let generation = server.connection_generation.load(Ordering::Acquire);
        let expired = server.expire_pending_commands(
            generation,
            Instant::now() + PENDING_COMMAND_TIMEOUT + Duration::from_millis(1),
        );
        assert_eq!(
            expired.iter().map(|pending| pending.id).collect::<Vec<_>>(),
            vec![command_id]
        );
        server
            .send_timeout_results(expired)
            .await
            .expect("timeout notification");
        let notification = bridge
            .notification_receiver()
            .recv()
            .await
            .expect("timeout result");
        assert!(matches!(
            notification,
            PlatformNotification::CommandResult {
                correlation_id,
                result
            } if correlation_id == command_id
                && !result.success
                && result.error.as_deref() == Some("platform command timed out")
        ));
    }

    #[test]
    fn timeout_sweep_is_scoped_to_the_connection_generation() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let old_generation = server.connection_generation.load(Ordering::Acquire);
        bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "old generation".into(),
                paste_after_write: true,
                target_window_handle: 13,
                target_focus_window_handle: 14,
            }))
            .expect("old generation command");

        server.open_connection_for_contract_test();
        let new_generation = server.connection_generation.load(Ordering::Acquire);
        let new_command_id = bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "new generation".into(),
                paste_after_write: true,
                target_window_handle: 15,
                target_focus_window_handle: 16,
            }))
            .expect("new generation command");
        let expiration = Instant::now() + PENDING_COMMAND_TIMEOUT + Duration::from_millis(1);
        assert!(
            server
                .expire_pending_commands(old_generation, expiration)
                .is_empty()
        );
        assert_eq!(
            server
                .expire_pending_commands(new_generation, expiration)
                .into_iter()
                .map(|pending| pending.id)
                .collect::<Vec<_>>(),
            vec![new_command_id]
        );
    }

    #[test]
    fn stale_connection_disconnect_does_not_clear_a_newer_connection() {
        let (bridge, server) = platform_bridge(4);
        let _old_receiver = server.open_connection_for_contract_test();
        let old_generation = server.connection_generation.load(Ordering::Acquire);
        let _new_receiver = server.open_connection_for_contract_test();
        assert!(server.disconnect_connection(old_generation).is_none());
        assert!(bridge.is_connected());
    }

    #[tokio::test]
    async fn frame_reader_stops_at_the_limit_before_newline() {
        let oversized = vec![b'x'; MAX_FRAME_BYTES + 1];
        let mut reader = BufReader::with_capacity(4096, oversized.as_slice());
        let error = read_bounded_frame(&mut reader)
            .await
            .expect_err("oversized frame must fail");
        assert!(error.to_string().contains("larger"));
    }

    #[tokio::test]
    async fn frame_reader_preserves_following_frame() {
        let input = b"first\nsecond\n";
        let mut reader = BufReader::with_capacity(4, input.as_slice());
        assert_eq!(
            read_bounded_frame(&mut reader).await.expect("first"),
            Some("first".into())
        );
        assert_eq!(
            read_bounded_frame(&mut reader).await.expect("second"),
            Some("second".into())
        );
    }
}
