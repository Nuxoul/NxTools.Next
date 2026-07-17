use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use nxtools_protocol::{Envelope, HelloAck, MAX_FRAME_BYTES, Message, Pong};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(windows)]
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tracing::{info, warn};

pub fn dispatch(message: &Envelope) -> Option<Envelope> {
    let response = match &message.message {
        Message::Hello(hello) => {
            const SUPPORTED_CAPABILITIES: [&str; 1] = ["foreground_context"];
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
        Message::HelloAck(_) | Message::Pong(_) | Message::Error(_) => return None,
    };
    Some(Envelope::reply_to(message, response))
}

#[cfg(windows)]
pub async fn run_platform_server(pipe_name: &str) -> Result<()> {
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

        if let Err(error) = handle_connection(server).await {
            warn!(%error, "platform host connection ended with an error");
        }
    }
}

#[cfg(not(windows))]
pub async fn run_platform_server(_pipe_name: &str) -> Result<()> {
    anyhow::bail!("the platform host named pipe is only available on Windows")
}

#[cfg(windows)]
async fn handle_connection(server: NamedPipeServer) -> Result<()> {
    info!("platform host connected");
    let (reader, mut writer) = tokio::io::split(server);
    let mut reader = BufReader::new(reader);

    loop {
        let Some(frame) = read_bounded_frame(&mut reader).await? else {
            info!("platform host disconnected");
            return Ok(());
        };

        let envelope = Envelope::decode_line(&frame).context("invalid platform frame")?;
        if let Some(response) = dispatch(&envelope) {
            writer
                .write_all(response.encode_line()?.as_bytes())
                .await
                .context("failed to write platform response")?;
            writer
                .flush()
                .await
                .context("failed to flush platform response")?;
        }
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

#[cfg(test)]
mod tests {
    use nxtools_protocol::{Hello, Message, Ping};
    use tokio::io::BufReader;

    use super::*;

    #[test]
    fn hello_negotiates_requested_capabilities() {
        let request = Envelope::new(Message::Hello(Hello {
            client_name: "platform_host".into(),
            client_version: "0.1.0".into(),
            capabilities: vec!["clipboard".into(), "foreground_context".into()],
        }));
        let response = dispatch(&request).expect("hello response");

        let Message::HelloAck(ack) = response.message else {
            panic!("expected hello acknowledgement")
        };
        assert_eq!(ack.accepted_capabilities, vec!["foreground_context"]);
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
