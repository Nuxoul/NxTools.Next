//! Explorer Markdown handoff between desktop processes, not a PlatformHost command.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const VERSION: u32 = 1;
const MAX_PATH_BYTES: usize = 128 * 1024;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopMarkdownIntent {
    pub version: u32,
    pub request_id: Uuid,
    pub path: String,
}

impl std::fmt::Debug for DesktopMarkdownIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopMarkdownIntent")
            .field("version", &self.version)
            .field("request_id", &self.request_id)
            .finish_non_exhaustive()
    }
}

impl DesktopMarkdownIntent {
    pub fn new(path: String, request_id: Uuid) -> Self {
        Self {
            version: VERSION,
            request_id,
            path,
        }
    }

    pub fn validate(&self) -> Result<(), LaunchFailure> {
        if self.version != VERSION {
            return Err(LaunchFailure::UnsupportedVersion);
        }
        if self.request_id.is_nil()
            || self.path.len() > MAX_PATH_BYTES
            || !document_path(&self.path)
        {
            return Err(LaunchFailure::InvalidRequest);
        }
        Ok(())
    }
}

fn document_path(path: &str) -> bool {
    fn drive_tail(path: &str) -> Option<&str> {
        match path.as_bytes() {
            [letter, b':', b'\\' | b'/', ..] if letter.is_ascii_alphabetic() => Some(&path[3..]),
            _ => None,
        }
    }

    let tail = if let Some(extended) = path.strip_prefix(r"\\?\") {
        if extended
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("UNC\\"))
        {
            share_tail(&extended[4..])
        } else {
            drive_tail(extended)
        }
    } else if let Some(share) = path.strip_prefix(r"\\").or_else(|| path.strip_prefix("//")) {
        share_tail(share)
    } else {
        drive_tail(path)
    };
    let Some(tail) = tail else { return false };
    tail.rsplit(['\\', '/'])
        .next()
        .and_then(|name| name.rsplit_once('.'))
        .is_some_and(|(stem, ext)| {
            !stem.is_empty()
                && ["md", "markdown"]
                    .iter()
                    .any(|known| ext.eq_ignore_ascii_case(known))
        })
        && usable_path_text(tail)
}

fn share_tail(path: &str) -> Option<&str> {
    let mut parts = path.splitn(3, ['\\', '/']);
    let server = parts.next()?;
    let share = parts.next()?;
    if [server, share]
        .iter()
        .any(|part| matches!(*part, "" | "." | "..") || !usable_path_text(part))
        || ["pipe", "mailslot"]
            .iter()
            .any(|name| share.eq_ignore_ascii_case(name))
    {
        return None;
    }
    parts.next()
}

fn usable_path_text(path: &str) -> bool {
    !path
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, ':' | '<' | '>' | '"' | '|' | '?' | '*'))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchFailure {
    UnsupportedVersion,
    InvalidRequest,
    Unauthorized,
    QueueFull,
    DuplicateId,
    Disconnected,
    OwnerUnavailable,
    NotFile,
    FileUnavailable,
    NotAdmitted,
    AdmissionTimeout,
    TransportTimeout,
    InvalidResponse,
}

impl std::fmt::Display for LaunchFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for LaunchFailure {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownOutcome {
    Admitted,
    Rejected(LaunchFailure),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopMarkdownReceipt {
    pub version: u32,
    pub request_id: Uuid,
    pub outcome: MarkdownOutcome,
}

impl DesktopMarkdownReceipt {
    pub fn new(request_id: Uuid, result: Result<(), LaunchFailure>) -> Self {
        let outcome = match result {
            Ok(()) => MarkdownOutcome::Admitted,
            Err(failure) => MarkdownOutcome::Rejected(failure),
        };
        Self {
            version: VERSION,
            request_id,
            outcome,
        }
    }

    pub fn validate(&self, request_id: Uuid) -> Result<(), LaunchFailure> {
        if self.version != VERSION {
            return Err(LaunchFailure::UnsupportedVersion);
        }
        if self.request_id != request_id || request_id.is_nil() {
            return Err(LaunchFailure::InvalidResponse);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Envelope, Message};

    #[test]
    fn markdown_launch_is_a_distinct_redacted_correlated_wire_message() {
        let intent = DesktopMarkdownIntent::new(
            r"\\server\shared\内部 notes.MARKDOWN".into(),
            Uuid::new_v4(),
        );
        intent.validate().expect("valid shared Markdown path");
        let mut request = Envelope::new(Message::DesktopMarkdownIntent(intent.clone()));
        request.id = intent.request_id;
        let encoded = request.encode_line().expect("encode Markdown launch");
        assert!(encoded.contains("\"type\":\"desktop_markdown_intent\""));
        assert_eq!(
            Envelope::decode_line(&encoded).expect("decode Markdown launch"),
            request
        );
        assert!(!format!("{request:?}").contains("内部 notes"));

        let receipt =
            DesktopMarkdownReceipt::new(intent.request_id, Err(LaunchFailure::NotAdmitted));
        let reply = Envelope::reply_to(&request, Message::DesktopMarkdownReceipt(receipt.clone()));
        let encoded = reply.encode_line().expect("encode Markdown receipt");
        assert!(encoded.contains("\"type\":\"desktop_markdown_receipt\""));
        assert_eq!(
            Envelope::decode_line(&encoded).expect("decode Markdown receipt"),
            reply
        );
        assert_eq!(reply.correlation_id, Some(intent.request_id));
        assert_eq!(receipt.validate(intent.request_id), Ok(()));
        assert_eq!(
            receipt.validate(Uuid::new_v4()),
            Err(LaunchFailure::InvalidResponse)
        );
        assert!(
            serde_json::from_str::<DesktopMarkdownReceipt>(&format!(
                r#"{{"version":1,"request_id":"{}"}}"#,
                intent.request_id
            ))
            .is_err()
        );
    }

    #[test]
    fn syntax_rejects_devices_relative_streams_and_non_markdown_without_disk_io() {
        for path in [
            r"filename.md",
            r"C:filename.md",
            r"\filename.md",
            r"\\server\filename.md",
            r"\\.\pipe\filename.md",
            r"\\?\GLOBALROOT\filename.md",
            r"C:\file.txt:stream.md",
            r"C:\dir\file.txt",
            r"C:\dir\.md",
            r"C:\dir\file.md\",
            "C:\\bad\0.md",
        ] {
            let intent = DesktopMarkdownIntent::new(path.into(), Uuid::new_v4());
            assert_eq!(
                intent.validate(),
                Err(LaunchFailure::InvalidRequest),
                "{path:?}"
            );
        }
        for path in [
            r"C:\file.md",
            r"C:/dir/note.Md",
            r"\\?\C:\long name.md",
            r"\\?\UNC\host\share\file.md",
        ] {
            assert_eq!(
                DesktopMarkdownIntent::new(path.into(), Uuid::new_v4()).validate(),
                Ok(())
            );
        }
        let mut intent = DesktopMarkdownIntent::new(
            format!("C:\\{}.md", "x".repeat(MAX_PATH_BYTES)),
            Uuid::new_v4(),
        );
        assert_eq!(intent.validate(), Err(LaunchFailure::InvalidRequest));
        intent.path = r"C:\note.md".into();
        intent.request_id = Uuid::nil();
        assert_eq!(intent.validate(), Err(LaunchFailure::InvalidRequest));
        intent.version += 1;
        assert_eq!(intent.validate(), Err(LaunchFailure::UnsupportedVersion));
    }
}
