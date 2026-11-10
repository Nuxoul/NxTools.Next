//! Rust desktop-to-desktop only. Never forwarded to PlatformHost or a codec worker.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const VERSION: u32 = 1;
pub const MAX_PENDING: usize = 32;
pub const MAX_PATHS: usize = 256;
pub const MAX_PATH_BYTES: usize = 32 * 1024;
pub const MAX_REQUEST_PATH_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveIntentAction {
    Open,
    Extract,
    Compress,
    CompressZip,
    #[serde(rename = "compress_7z")]
    CompressSevenZ,
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveExtractMode {
    Smart,
    Here,
    Name,
    Dialog,
    SmartDialog,
    NameDialog,
}

// Debug is explicitly redacted: paths are not diagnostic data.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopArchiveIntent {
    pub version: u32,
    pub request_id: Uuid,
    pub action: ArchiveIntentAction,
    pub paths: Vec<String>,
    pub mode: Option<ArchiveExtractMode>,
}
impl std::fmt::Debug for DesktopArchiveIntent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DesktopArchiveIntent")
            .field("version", &self.version)
            .field("request_id", &self.request_id)
            .field("action", &self.action)
            .field("path_count", &self.paths.len())
            .field("mode", &self.mode)
            .finish()
    }
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
    ReadinessTimeout,
    OwnerUnavailable,
    OpenFailed,
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
pub enum LaunchReceiptState {
    Queued,
    Handled,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopArchiveReceipt {
    pub version: u32,
    pub request_id: Uuid,
    pub state: LaunchReceiptState,
    pub failure: Option<LaunchFailure>,
}
impl DesktopArchiveReceipt {
    pub fn new(request_id: Uuid, result: Result<bool, LaunchFailure>) -> Self {
        let (state, failure) = match result {
            Ok(false) => (LaunchReceiptState::Queued, None),
            Ok(true) => (LaunchReceiptState::Handled, None),
            Err(failure) => (LaunchReceiptState::Rejected, Some(failure)),
        };
        Self {
            version: VERSION,
            request_id,
            state,
            failure,
        }
    }
    pub fn validate(&self, request_id: Uuid) -> Result<(), LaunchFailure> {
        if self.version != VERSION {
            return Err(LaunchFailure::UnsupportedVersion);
        }
        if self.request_id != request_id
            || (self.state == LaunchReceiptState::Rejected) != self.failure.is_some()
        {
            return Err(LaunchFailure::InvalidResponse);
        }
        Ok(())
    }
}
impl DesktopArchiveIntent {
    pub fn validate(&self) -> Result<(), LaunchFailure> {
        if self.version != VERSION {
            return Err(LaunchFailure::UnsupportedVersion);
        }
        if self.request_id.is_nil()
            || self.paths.is_empty()
            || self.paths.len() > MAX_PATHS
            || matches!(
                self.action,
                ArchiveIntentAction::Open | ArchiveIntentAction::Test
            ) && self.paths.len() != 1
            || (self.action == ArchiveIntentAction::Extract) != self.mode.is_some()
        {
            return Err(LaunchFailure::InvalidRequest);
        }
        let mut bytes = 0usize;
        for path in &self.paths {
            bytes = bytes.saturating_add(path.len());
            // Windows absolute paths only; device namespaces and drive-relative paths are not launch inputs.
            let absolute = path.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
                && path.as_bytes().get(1) == Some(&b':')
                && path
                    .as_bytes()
                    .get(2)
                    .is_some_and(|c| matches!(c, b'\\' | b'/'))
                || path.starts_with(r"\\")
                    && !path.starts_with(r"\\.\")
                    && !path.starts_with(r"\\?\")
                    && path[2..]
                        .split(['\\', '/'])
                        .take(2)
                        .filter(|part| !part.is_empty())
                        .count()
                        == 2;
            if path.is_empty()
                || path.chars().any(char::is_control)
                || path.len() > MAX_PATH_BYTES
                || !absolute
            {
                return Err(LaunchFailure::InvalidRequest);
            }
        }
        if bytes > MAX_REQUEST_PATH_BYTES {
            return Err(LaunchFailure::InvalidRequest);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> DesktopArchiveIntent {
        DesktopArchiveIntent {
            version: VERSION,
            request_id: Uuid::new_v4(),
            action: ArchiveIntentAction::Open,
            paths: vec![r"C:\synthetic\a.zip".into()],
            mode: None,
        }
    }
    #[test]
    fn strict_roundtrip_and_bounds() {
        let value = request();
        let json = serde_json::to_string(&value).expect("encode");
        let decoded: DesktopArchiveIntent = serde_json::from_str(&json).expect("decode");
        assert!(decoded == value);
        assert_eq!(decoded.validate(), Ok(()));
        assert!(
            serde_json::from_str::<DesktopArchiveIntent>(
                &json.replace("\"version\":1", "\"version\":1,\"password\":\"sentinel\"")
            )
            .is_err()
        );
        let mut value = value;
        value.version += 1;
        assert_eq!(value.validate(), Err(LaunchFailure::UnsupportedVersion));
        value.version = VERSION;
        for path in [
            "relative.zip",
            "C:relative.zip",
            "C:\\nul\0.zip",
            r"\\.\pipe\device",
        ] {
            value.paths = vec![path.into()];
            assert_eq!(value.validate(), Err(LaunchFailure::InvalidRequest));
        }
        value.action = ArchiveIntentAction::Compress;
        value.paths = vec![r"C:\source".into(); MAX_PATHS];
        assert_eq!(value.validate(), Ok(()));
        value.paths.push(r"C:\overflow".into());
        assert_eq!(value.validate(), Err(LaunchFailure::InvalidRequest));
    }
}
