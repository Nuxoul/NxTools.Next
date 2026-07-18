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
    PlatformCommand(PlatformCommand),
    PlatformCommandResult(PlatformCommandResult),
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
    ClipboardCaptured(ClipboardCapture),
    ClipboardPanelRequested(ClipboardPanelRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForegroundContext {
    pub window_handle: i64,
    pub focus_window_handle: i64,
    pub process_id: u32,
    pub process_name: String,
    pub window_title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardCapture {
    pub text: Option<String>,
    pub image: Option<ClipboardImageData>,
    pub paths: Vec<String>,
    #[serde(default)]
    pub native_formats: Vec<ClipboardNativeFormat>,
    pub source_icon_staging_path: Option<String>,
    pub source: ForegroundContext,
    pub captured_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardNativeFormat {
    pub format_id: u32,
    pub format_name: String,
    pub data_path: String,
    pub size_bytes: u64,
    pub sort_order: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardImageData {
    pub staging_path: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardPanelRequest {
    pub target: ForegroundContext,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub cursor_display_index: u32,
    pub cursor_monitor_dpi: u32,
    pub requested_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", content = "data", rename_all = "snake_case")]
pub enum PlatformCommand {
    PasteText(PasteTextCommand),
    PasteFiles(PasteFilesCommand),
    PasteImage(PasteImageCommand),
    PasteNativeFormats(PasteNativeFormatsCommand),
    SetWindowTopmost(SetWindowTopmostCommand),
    SetWindowVisibility(SetWindowVisibilityCommand),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasteNativeFormatsCommand {
    pub formats: Vec<ClipboardNativeFormat>,
    pub paste_after_write: bool,
    pub target_window_handle: i64,
    pub target_focus_window_handle: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasteTextCommand {
    pub text: String,
    #[serde(default = "default_true")]
    pub paste_after_write: bool,
    pub target_window_handle: i64,
    pub target_focus_window_handle: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasteFilesCommand {
    pub paths: Vec<String>,
    #[serde(default = "default_true")]
    pub paste_after_write: bool,
    pub target_window_handle: i64,
    pub target_focus_window_handle: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasteImageCommand {
    pub image_path: String,
    #[serde(default = "default_true")]
    pub paste_after_write: bool,
    pub target_window_handle: i64,
    pub target_focus_window_handle: i64,
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetWindowTopmostCommand {
    pub window_handle: i64,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetWindowVisibilityCommand {
    pub window_handle: i64,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformCommandResult {
    pub success: bool,
    pub error: Option<String>,
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

    fn fixture_context() -> ForegroundContext {
        ForegroundContext {
            window_handle: 42,
            focus_window_handle: 43,
            process_id: 7,
            process_name: "fixture".into(),
            window_title: "fixture window".into(),
        }
    }

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

    #[test]
    fn clipboard_text_capture_round_trips_as_platform_event() {
        let envelope = Envelope::new(Message::PlatformEvent(PlatformEvent::ClipboardCaptured(
            ClipboardCapture {
                text: Some("真实剪贴板内容".into()),
                image: None,
                paths: Vec::new(),
                native_formats: Vec::new(),
                source_icon_staging_path: None,
                source: ForegroundContext {
                    window_handle: 42,
                    focus_window_handle: 43,
                    process_id: 7,
                    process_name: "notepad".into(),
                    window_title: "notes.txt".into(),
                },
                captured_at_unix_ms: 1_700_000_000_000,
            },
        )));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("clipboard event must decode");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn paste_text_command_round_trips_with_target_window() {
        let envelope = Envelope::new(Message::PlatformCommand(PlatformCommand::PasteText(
            PasteTextCommand {
                text: "paste me".into(),
                paste_after_write: false,
                target_window_handle: 99,
                target_focus_window_handle: 100,
            },
        )));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("paste command must decode");
        assert_eq!(decoded, envelope);
        let Message::PlatformCommand(PlatformCommand::PasteText(command)) = decoded.message else {
            panic!("expected text clipboard command");
        };
        assert!(!command.paste_after_write);
    }

    #[test]
    fn image_and_files_round_trip_as_capture_and_paste_messages() {
        for message in [
            Message::PlatformEvent(PlatformEvent::ClipboardCaptured(ClipboardCapture {
                text: Some("alternate representation".into()),
                image: Some(ClipboardImageData {
                    staging_path: "C:/temp/image.png".into(),
                    width: 640,
                    height: 480,
                }),
                paths: vec!["C:/one.txt".into(), "C:/two.txt".into()],
                native_formats: Vec::new(),
                source_icon_staging_path: Some("C:/temp/source.png".into()),
                source: fixture_context(),
                captured_at_unix_ms: 10,
            })),
            Message::PlatformCommand(PlatformCommand::PasteFiles(PasteFilesCommand {
                paths: vec!["C:/one.txt".into()],
                paste_after_write: true,
                target_window_handle: 5,
                target_focus_window_handle: 6,
            })),
            Message::PlatformCommand(PlatformCommand::PasteImage(PasteImageCommand {
                image_path: "C:/image.png".into(),
                paste_after_write: true,
                target_window_handle: 5,
                target_focus_window_handle: 6,
            })),
        ] {
            let envelope = Envelope::new(message);
            let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
                .expect("multi-format message must decode");
            assert_eq!(decoded, envelope);
        }
    }

    #[test]
    fn native_clipboard_formats_round_trip_by_staged_file() {
        let format = ClipboardNativeFormat {
            format_id: 49_222,
            format_name: "HTML Format".into(),
            data_path: "C:/temp/html-format.bin".into(),
            size_bytes: 128,
            sort_order: 2,
        };
        let capture = Envelope::new(Message::PlatformEvent(PlatformEvent::ClipboardCaptured(
            ClipboardCapture {
                text: Some("formatted".into()),
                image: None,
                paths: Vec::new(),
                native_formats: vec![format.clone()],
                source_icon_staging_path: None,
                source: fixture_context(),
                captured_at_unix_ms: 12,
            },
        )));
        let command = Envelope::new(Message::PlatformCommand(
            PlatformCommand::PasteNativeFormats(PasteNativeFormatsCommand {
                formats: vec![format],
                paste_after_write: false,
                target_window_handle: 42,
                target_focus_window_handle: 43,
            }),
        ));

        for envelope in [capture, command] {
            let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
                .expect("native clipboard payload must decode");
            assert_eq!(decoded, envelope);
        }
    }

    #[test]
    fn topmost_command_round_trips_with_native_window() {
        let envelope = Envelope::new(Message::PlatformCommand(PlatformCommand::SetWindowTopmost(
            SetWindowTopmostCommand {
                window_handle: 777,
                enabled: true,
            },
        )));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("topmost command must decode");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn visibility_command_round_trips_with_native_window() {
        let envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                window_handle: 778,
                visible: false,
            }),
        ));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("visibility command must decode");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn clipboard_panel_request_preserves_target_and_cursor_position() {
        let envelope = Envelope::new(Message::PlatformEvent(
            PlatformEvent::ClipboardPanelRequested(ClipboardPanelRequest {
                target: ForegroundContext {
                    window_handle: 88,
                    focus_window_handle: 89,
                    process_id: 9,
                    process_name: "editor".into(),
                    window_title: "draft.md".into(),
                },
                cursor_x: 1200,
                cursor_y: 700,
                cursor_display_index: 1,
                cursor_monitor_dpi: 144,
                requested_at_unix_ms: 1_700_000_000_500,
            }),
        ));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("panel request must decode");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn command_result_is_correlated_to_the_original_command() {
        let command = Envelope::new(Message::PlatformCommand(PlatformCommand::PasteText(
            PasteTextCommand {
                text: "paste".into(),
                paste_after_write: true,
                target_window_handle: 9,
                target_focus_window_handle: 10,
            },
        )));
        let result = Envelope::reply_to(
            &command,
            Message::PlatformCommandResult(PlatformCommandResult {
                success: false,
                error: Some("target closed".into()),
            }),
        );

        let decoded = Envelope::decode_line(&result.encode_line().expect("encode"))
            .expect("command result must decode");
        assert_eq!(decoded.correlation_id, Some(command.id));
        assert_eq!(decoded, result);
    }
}
