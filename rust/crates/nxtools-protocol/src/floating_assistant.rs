//! Bounded passive floating-window native boundary. Layout/state remain Desktop-owned.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const FLOATING_ASSISTANT_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FloatingAssistantOperation {
    Prepare,
    Update,
    Show,
    Hide,
    Retire,
    ReleaseDrop,
    PickImage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloatingAssistantCommand {
    pub version: u16,
    pub session_id: Uuid,
    pub generation: u64,
    pub window_handle: i64,
    pub source_window_handle: i64,
    pub source_process_id: u32,
    pub operation: FloatingAssistantOperation,
    /// Device-pixel screen origin; size is separately expressed in logical pixels.
    pub x: i32,
    pub y: i32,
    pub size: u32,
    pub opacity: u8,
    pub click_through: bool,
    pub position_locked: bool,
    /// Only intentional edge-hide placement may retain eight visible device pixels.
    #[serde(default)]
    pub edge_hidden: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FloatingAssistantEventKind {
    LeftClick,
    RightClick,
    MoveCompleted,
    HoverEnter,
    HoverLeave,
    Wheel,
    Drop,
    Environment,
    Retired,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloatingAssistantEvent {
    pub version: u16,
    pub session_id: Uuid,
    pub generation: u64,
    pub window_handle: i64,
    pub correlation_id: Uuid,
    pub sequence: u64,
    pub kind: FloatingAssistantEventKind,
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub cursor_monitor: Option<crate::CursorMonitorSnapshot>,
    pub delta: i32,
    pub fullscreen: bool,
    pub text: Option<String>,
    pub url: Option<String>,
    pub html: Option<String>,
    pub image_path: Option<String>,
    pub image_format: Option<String>,
    pub files: Vec<String>,
    pub drop_id: Option<Uuid>,
}

impl FloatingAssistantEvent {
    pub fn is_bounded(&self) -> bool {
        self.version == FLOATING_ASSISTANT_VERSION
            && !self.session_id.is_nil()
            && !self.correlation_id.is_nil()
            && self.generation != 0
            && self.window_handle != 0
            && self
                .text
                .as_ref()
                .is_none_or(|v| v.len() <= 196_608 && !v.contains('\0'))
            && self
                .url
                .as_ref()
                .is_none_or(|v| v.len() <= 8192 && !v.contains('\0'))
            && self
                .html
                .as_ref()
                .is_none_or(|v| v.len() <= 262_144 && !v.contains('\0'))
            && self
                .image_path
                .as_ref()
                .is_none_or(|v| v.len() <= 32768 && !v.contains('\0'))
            && self.files.len() <= 32
            && self
                .files
                .iter()
                .all(|v| v.len() <= 32768 && !v.contains('\0'))
            && self.files.iter().map(String::len).sum::<usize>() <= 196_608
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloatingAssistantBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    pub work_left: i32,
    pub work_top: i32,
    pub work_right: i32,
    pub work_bottom: i32,
    pub monitor_left: i32,
    pub monitor_top: i32,
    pub monitor_right: i32,
    pub monitor_bottom: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Envelope, Message, PlatformCommand};

    #[test]
    fn native_floating_command_preserves_typed_operation_and_units() {
        let command = FloatingAssistantCommand {
            version: 1,
            session_id: Uuid::new_v4(),
            generation: 7,
            window_handle: 123,
            source_window_handle: 456,
            source_process_id: 789,
            operation: FloatingAssistantOperation::Prepare,
            x: -1920,
            y: 40,
            size: 72,
            opacity: 48,
            click_through: false,
            edge_hidden: false,
            position_locked: true,
            drop_id: None,
        };
        let envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::FloatingAssistant(command.clone()),
        ));
        let json = serde_json::to_string(&envelope).expect("encode command");
        assert!(json.contains("\"command\":\"floating_assistant\""));
        assert!(json.contains("\"operation\":\"prepare\""));
        let decoded: Envelope = serde_json::from_str(&json).expect("decode command");
        assert_eq!(
            decoded.message,
            Message::PlatformCommand(PlatformCommand::FloatingAssistant(command))
        );
    }
}
