use std::io::{BufRead, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};

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
use tracing::{info, warn};

#[cfg(windows)]
mod local_pipe;
#[cfg(windows)]
pub use local_pipe::{
    INDEXER_PIPE_NAME, LocalPipeConnection, accept_current_user_pipe, accept_local_indexer_pipe,
    connect_local_indexer_pipe, connect_local_pipe,
};

pub struct PlatformBridge {
    notifications: Receiver<PlatformNotification>,
    commands: Sender<Envelope>,
    connected: Arc<AtomicBool>,
}

pub struct PlatformServerEndpoint {
    notifications: Sender<PlatformNotification>,
    commands: Receiver<Envelope>,
    connected: Arc<AtomicBool>,
}

pub fn platform_bridge(capacity: usize) -> (PlatformBridge, PlatformServerEndpoint) {
    let (notification_sender, notification_receiver) = async_channel::bounded(capacity);
    let (command_sender, command_receiver) = async_channel::bounded(capacity);
    let connected = Arc::new(AtomicBool::new(false));
    (
        PlatformBridge {
            notifications: notification_receiver,
            commands: command_sender,
            connected: Arc::clone(&connected),
        },
        PlatformServerEndpoint {
            notifications: notification_sender,
            commands: command_receiver,
            connected,
        },
    )
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
        if !self.is_connected() {
            return Err(PlatformBridgeError::Disconnected);
        }
        let command_id = envelope.id;
        self.commands
            .try_send(envelope)
            .map_err(|error| match error {
                async_channel::TrySendError::Full(_) => PlatformBridgeError::Busy,
                async_channel::TrySendError::Closed(_) => PlatformBridgeError::Disconnected,
            })?;
        Ok(command_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformNotification {
    Connected,
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
}

pub fn dispatch(message: &Envelope) -> Option<Envelope> {
    let response = match &message.message {
        Message::Hello(hello) => {
            const SUPPORTED_CAPABILITIES: [&str; 28] = [
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

        if let Err(error) = handle_connection(server, &endpoint).await {
            warn!(%error, "platform host connection ended with an error");
        }
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
    if !matches!(handshake.message, Message::Hello(_)) {
        anyhow::bail!("platform host did not begin with a hello message");
    }
    let acknowledgement =
        dispatch(&handshake).context("hello did not produce an acknowledgement")?;
    write_envelope(&mut writer, &acknowledgement).await?;

    endpoint.connected.store(true, Ordering::Release);
    let _connection = ConnectionState::new(endpoint);
    endpoint
        .notifications
        .send(PlatformNotification::Connected)
        .await
        .context("desktop platform notification receiver closed")?;

    loop {
        tokio::select! {
            frame = read_bounded_frame(&mut reader) => {
                let Some(frame) = frame? else {
                    info!("platform host disconnected");
                    return Ok(());
                };
                let envelope = Envelope::decode_line(&frame).context("invalid platform frame")?;
                match &envelope.message {
                    Message::PlatformEvent(event) => endpoint
                        .notifications
                        .send(PlatformNotification::Event(event.clone()))
                        .await
                        .context("desktop platform event receiver closed")?,
                    Message::PlatformCommandResult(result) => {
                        let correlation_id = envelope
                            .correlation_id
                            .context("platform command result had no correlation ID")?;
                        endpoint
                            .notifications
                            .send(PlatformNotification::CommandResult {
                                correlation_id,
                                result: result.clone(),
                            })
                            .await
                            .context("desktop platform notification receiver closed")?;
                    }
                    _ => {
                        if let Some(response) = dispatch(&envelope) {
                            write_envelope(&mut writer, &response).await?;
                        }
                    }
                }
            }
            command = endpoint.commands.recv() => {
                let envelope = command.context("desktop platform command sender closed")?;
                write_envelope(&mut writer, &envelope).await?;
            }
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

struct ConnectionState<'a> {
    endpoint: &'a PlatformServerEndpoint,
}

impl<'a> ConnectionState<'a> {
    fn new(endpoint: &'a PlatformServerEndpoint) -> Self {
        Self { endpoint }
    }
}

impl Drop for ConnectionState<'_> {
    fn drop(&mut self) {
        self.endpoint.connected.store(false, Ordering::Release);
        while self.endpoint.commands.try_recv().is_ok() {}
    }
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
    fn bridge_preserves_a_caller_owned_lifecycle_correlation() {
        let (bridge, server) = platform_bridge(4);
        server.connected.store(true, Ordering::Release);
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

        let envelope = server.commands.try_recv().expect("queued command");
        assert_eq!(envelope.correlation_id, Some(lifecycle_id));
        assert_ne!(envelope.id, lifecycle_id);
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
