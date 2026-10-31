//! Browser-only private secret rendezvous. Not part of the public read-only ArchiveRead schema.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchivePasswordPrompt {
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub prompt_id: Uuid,
    pub ticket_generation: u64,
    pub error_code: String,
    pub attempt: u32,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PrivatePassword(Vec<u8>);
impl PrivatePassword {
    pub fn new(value: &str) -> Self {
        Self(value.as_bytes().to_vec())
    }
}
impl std::fmt::Debug for PrivatePassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PrivatePassword([REDACTED])")
    }
}
impl Drop for PrivatePassword {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchivePasswordReply {
    pub prompt: ArchivePasswordPrompt,
    pub password: Option<PrivatePassword>,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_password_debug_is_redacted_and_public_read_refuses_password() {
        let secret = PrivatePassword::new("private-fixture-secret");
        assert!(!format!("{secret:?}").contains("private-fixture-secret"));
        let request = serde_json::json!({"version":2,"operation_id":Uuid::new_v4(),"session_id":Uuid::new_v4(),"generation":1,"action":"open","password":"forbidden"});
        assert!(
            serde_json::from_value::<crate::archive_read::ArchiveReadCommand>(request).is_err()
        );
    }
}
