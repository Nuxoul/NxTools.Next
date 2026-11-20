use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProcessWindowPickCommand {
    Start {
        session_id: Uuid,
        source_window_handle: i64,
        overlay_window_handle: i64,
    },
    ShowOverlay {
        session_id: Uuid,
    },
    Cancel {
        session_id: Uuid,
    },
}

/// Native device-pixel coordinates, including negative virtual-screen origins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowPickRect {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}

impl WindowPickRect {
    pub fn is_valid(self) -> bool {
        self.width > 0 && self.height > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowPickCandidate {
    pub window_handle: i64,
    pub process_id: u32,
    pub process_name: String,
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowPickState {
    Hover,
    Selected,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessWindowPickChanged {
    pub session_id: Uuid,
    pub state: WindowPickState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport: Option<WindowPickRect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate: Option<WindowPickCandidate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_error_code: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Envelope, Message, PlatformCommand, PlatformEvent};

    #[test]
    fn window_picker_wire_preserves_action_shapes_and_device_pixels() {
        let id = Uuid::new_v4();
        for command in [
            ProcessWindowPickCommand::Start {
                session_id: id,
                source_window_handle: 101,
                overlay_window_handle: 102,
            },
            ProcessWindowPickCommand::ShowOverlay { session_id: id },
            ProcessWindowPickCommand::Cancel { session_id: id },
        ] {
            let envelope = Envelope::new(Message::PlatformCommand(
                PlatformCommand::ProcessWindowPick(command),
            ));
            let json = serde_json::to_string(&envelope).expect("command JSON");
            assert!(json.contains("process_window_pick"));
            assert_eq!(
                serde_json::from_str::<Envelope>(&json).expect("command roundtrip"),
                envelope
            );
        }
        let event = ProcessWindowPickChanged {
            session_id: id,
            state: WindowPickState::Hover,
            viewport: Some(WindowPickRect {
                left: -1920,
                top: -120,
                width: 3840,
                height: 1200,
            }),
            candidate: Some(WindowPickCandidate {
                window_handle: 201,
                process_id: 42,
                process_name: "Editor.exe".into(),
                left: -1800,
                top: -80,
                width: 1000,
                height: 800,
            }),
            native_error_code: None,
        };
        let envelope = Envelope::new(Message::PlatformEvent(
            PlatformEvent::ProcessWindowPickChanged(event),
        ));
        let json = serde_json::to_string(&envelope).expect("event JSON");
        assert!(json.contains("process_window_pick_changed"));
        assert!(json.contains("\"left\":-1920"));
        assert_eq!(
            serde_json::from_str::<Envelope>(&json).expect("event roundtrip"),
            envelope
        );
        assert!(
            serde_json::from_str::<ProcessWindowPickCommand>(&format!(
                r#"{{"action":"cancel","session_id":"{id}","overlay_window_handle":102}}"#
            ))
            .is_err()
        );
        assert!(
            serde_json::from_str::<ProcessWindowPickCommand>(&format!(
                r#"{{"action":"start","session_id":"{id}"}}"#
            ))
            .is_err()
        );
        assert!(serde_json::from_str::<WindowPickState>("\"unknown\"").is_err());
    }
}
