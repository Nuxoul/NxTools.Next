use crate::{BrokerClient, SecretFuture, SecretStore, SecretStoreError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub(crate) struct CredentialRequest {
    pub version: u16,
    pub op: &'static str,
    pub request_id: Uuid,
    pub package_id: String,
    pub setting_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CredentialResponse {
    pub version: u16,
    pub op: String,
    pub request_id: Uuid,
    pub success: bool,
    #[serde(default)]
    pub found: Option<bool>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub native_error_code: Option<i32>,
    pub elapsed_ms: u64,
}

impl CredentialResponse {
    pub(crate) fn validate(&self) -> bool {
        self.version == 1
            && self.op == "credential_result"
            && !self.request_id.is_nil()
            && self
                .reason
                .as_ref()
                .is_none_or(|reason| reason.len() <= 128)
            && self
                .value
                .as_ref()
                .is_none_or(|value| value.len() <= 32 * 1024)
    }
    pub(crate) fn observe_metadata(&self) {
        tracing::info!(event="extension.credential.terminal", correlation_id=%self.request_id, success=self.success, native_error_code=?self.native_error_code, elapsed_ms=self.elapsed_ms, "extension credential operation terminated");
    }
}

/// Protocol adapter. Authenticated owning composition controls trust and Save serialization.
#[derive(Clone)]
pub struct BrokerSecretStore(pub BrokerClient);
impl SecretStore for BrokerSecretStore {
    fn get<'a>(&'a self, package: &'a str, key: &'a str) -> SecretFuture<'a, Option<String>> {
        Box::pin(async move {
            let request = request("credential_get", package, key, None)?;
            let response = self
                .0
                .credential(request)
                .await
                .map_err(|_| SecretStoreError)?;
            if !response.success {
                return Err(SecretStoreError);
            }
            match (response.found, response.value) {
                (Some(false), None) => Ok(None),
                (Some(true), Some(value)) if value.len() <= 32 * 1024 => Ok(Some(value)),
                _ => Err(SecretStoreError),
            }
        })
    }
    fn set<'a>(
        &'a self,
        package: &'a str,
        key: &'a str,
        value: Option<&'a str>,
    ) -> SecretFuture<'a, ()> {
        Box::pin(async move {
            let request = request(
                if value.is_some() {
                    "credential_set"
                } else {
                    "credential_delete"
                },
                package,
                key,
                value,
            )?;
            let response = self
                .0
                .credential(request)
                .await
                .map_err(|_| SecretStoreError)?;
            if response.success && response.value.is_none() && response.found.is_none() {
                Ok(())
            } else {
                Err(SecretStoreError)
            }
        })
    }
}
fn request(
    op: &'static str,
    package: &str,
    key: &str,
    value: Option<&str>,
) -> Result<CredentialRequest, SecretStoreError> {
    if !crate::valid_id(package)
        || !crate::valid_id(key)
        || value.is_some_and(|value| value.len() > 32 * 1024)
    {
        return Err(SecretStoreError);
    }
    Ok(CredentialRequest {
        version: 1,
        op,
        request_id: Uuid::new_v4(),
        package_id: package.into(),
        setting_key: key.into(),
        value: value.map(str::to_owned),
    })
}
