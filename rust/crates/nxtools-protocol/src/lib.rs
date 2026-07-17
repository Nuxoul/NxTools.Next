use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
pub const DEFAULT_PLATFORM_PIPE_NAME: &str = "nxtools-next-platform-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub protocol_version: u16,
    pub id: Uuid,
    pub correlation_id: Option<Uuid>,
    #[serde(flatten)]
    pub message: Message,
}

impl Envelope {
    pub fn new(message: Message) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            id: Uuid::new_v4(),
            correlation_id: None,
            message,
        }
    }

    pub fn reply_to(request: &Self, message: Message) -> Self {
        Self {
            correlation_id: Some(request.id),
            ..Self::new(message)
        }
    }

    pub fn encode_line(&self) -> Result<String, ProtocolError> {
        let mut encoded = serde_json::to_string(self)?;
        if encoded.len() > MAX_FRAME_BYTES {
            return Err(ProtocolError::FrameTooLarge(encoded.len()));
        }
        encoded.push('\n');
        Ok(encoded)
    }

    pub fn decode_line(line: &str) -> Result<Self, ProtocolError> {
        if line.len() > MAX_FRAME_BYTES {
            return Err(ProtocolError::FrameTooLarge(line.len()));
        }
        let json = line.trim_end_matches(['\r', '\n']);
        let header: VersionHeader = serde_json::from_str(json)?;
        if header.protocol_version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion {
                expected: PROTOCOL_VERSION,
                actual: header.protocol_version,
            });
        }
        Ok(serde_json::from_str(json)?)
    }
}

#[derive(Deserialize)]
struct VersionHeader {
    protocol_version: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum Message {
    Hello(Hello),
    HelloAck(HelloAck),
    Ping(Ping),
    Pong(Pong),
    PlatformEvent(PlatformEvent),
    Error(ProtocolFault),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub client_name: String,
    pub client_version: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloAck {
    pub server_name: String,
    pub server_version: String,
    pub accepted_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ping {
    pub sent_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pong {
    pub sent_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum PlatformEvent {
    ForegroundChanged(ForegroundContext),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForegroundContext {
    pub process_id: u32,
    pub process_name: String,
    pub window_title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolFault {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("frame is {0} bytes, exceeding the protocol limit")]
    FrameTooLarge(usize),
    #[error("unsupported protocol version {actual}; expected {expected}")]
    UnsupportedVersion { expected: u16, actual: u16 },
    #[error("invalid JSON frame: {0}")]
    InvalidJson(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_round_trips_with_correlation() {
        let request = Envelope::new(Message::Hello(Hello {
            client_name: "platform_host".into(),
            client_version: "0.1.0".into(),
            capabilities: vec!["foreground_context".into()],
        }));
        let reply = Envelope::reply_to(
            &request,
            Message::HelloAck(HelloAck {
                server_name: "desktop".into(),
                server_version: "0.1.0".into(),
                accepted_capabilities: vec!["foreground_context".into()],
            }),
        );

        let decoded = Envelope::decode_line(&reply.encode_line().expect("encode")).expect("decode");
        assert_eq!(decoded, reply);
        assert_eq!(decoded.correlation_id, Some(request.id));
    }

    #[test]
    fn incompatible_versions_are_rejected_before_dispatch() {
        let line = format!(
            r#"{{"protocol_version":99,"id":"{}","correlation_id":null,"type":"ping","payload":{{"sent_at_unix_ms":0}}}}"#,
            Uuid::new_v4()
        );

        assert!(matches!(
            Envelope::decode_line(&line),
            Err(ProtocolError::UnsupportedVersion {
                expected: PROTOCOL_VERSION,
                actual: 99
            })
        ));
    }

    #[test]
    fn incompatible_version_wins_over_unknown_message_payload() {
        let line = format!(
            r#"{{"protocol_version":99,"id":"{}","correlation_id":null,"type":"future_message","payload":{{"shape":"unknown"}}}}"#,
            Uuid::new_v4()
        );

        assert!(matches!(
            Envelope::decode_line(&line),
            Err(ProtocolError::UnsupportedVersion { actual: 99, .. })
        ));
    }

    #[test]
    fn oversized_frames_are_rejected() {
        let line = "x".repeat(MAX_FRAME_BYTES + 1);
        assert!(matches!(
            Envelope::decode_line(&line),
            Err(ProtocolError::FrameTooLarge(_))
        ));
    }
}
