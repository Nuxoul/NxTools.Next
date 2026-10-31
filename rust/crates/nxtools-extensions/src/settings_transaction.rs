//! Secret writes are compensated if any write or the nonsecret config commit fails.
//! The feature owner serializes Save and publishes only the returned committed snapshot.
use crate::{InstalledPackage, PackageError, SettingType};
use serde_json::Value;
use std::{collections::BTreeMap, future::Future, pin::Pin};

pub type SecretFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, SecretStoreError>> + Send + 'a>>;
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("extension credential operation failed")]
pub struct SecretStoreError;

pub trait SecretStore: Send + Sync {
    fn get<'a>(&'a self, package: &'a str, key: &'a str) -> SecretFuture<'a, Option<String>>;
    fn set<'a>(
        &'a self,
        package: &'a str,
        key: &'a str,
        value: Option<&'a str>,
    ) -> SecretFuture<'a, ()>;
}

/// No Debug/Serialize implementation: secret draft must not reach logs/config by accident.
pub struct SettingsDraft {
    pub values: BTreeMap<String, Value>,
    /// Absent = unchanged; None = explicit deletion.
    pub secrets: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsSaveError {
    #[error("extension settings are invalid")]
    Invalid,
    #[error("extension credential read failed")]
    ReadFailed,
    #[error("extension settings save failed and was compensated")]
    SaveFailed,
    #[error("extension settings save failed and compensation was incomplete")]
    CompensationFailed,
}

pub async fn save_settings<T, F, Fut>(
    package: &InstalledPackage,
    draft: SettingsDraft,
    store: &impl SecretStore,
    commit: F,
) -> Result<T, SettingsSaveError>
where
    F: FnOnce(BTreeMap<String, Value>) -> Fut,
    Fut: Future<Output = Result<T, PackageError>>,
{
    if !package.manifest.settings_ready(&draft.values)
        || draft.secrets.len() > package.manifest.settings.len()
        || draft.secrets.iter().any(|(key, value)| {
            !package.manifest.settings.iter().any(|setting| {
                setting.key == *key && matches!(setting.field, SettingType::Password)
            }) || value
                .as_ref()
                .is_some_and(|value| value.len() > 32 * 1024 || value.contains('\0'))
        })
    {
        return Err(SettingsSaveError::Invalid);
    }
    let mut previous = BTreeMap::new();
    for key in draft.secrets.keys() {
        previous.insert(
            key.clone(),
            store
                .get(&package.manifest.id, key)
                .await
                .map_err(|_| SettingsSaveError::ReadFailed)?,
        );
    }
    let mut attempted = Vec::new();
    for (key, value) in &draft.secrets {
        // Mark attempted BEFORE mutation: native writes may mutate and then report failure.
        attempted.push(key.clone());
        if store
            .set(&package.manifest.id, key, value.as_deref())
            .await
            .is_err()
        {
            return Err(compensate(package, &attempted, &previous, store).await);
        }
    }
    match commit(draft.values).await {
        Ok(snapshot) => Ok(snapshot),
        Err(_) => Err(compensate(package, &attempted, &previous, store).await),
    }
}

async fn compensate(
    package: &InstalledPackage,
    attempted: &[String],
    previous: &BTreeMap<String, Option<String>>,
    store: &impl SecretStore,
) -> SettingsSaveError {
    let mut failed = false;
    for key in attempted.iter().rev() {
        if store
            .set(
                &package.manifest.id,
                key,
                previous.get(key).and_then(Option::as_deref),
            )
            .await
            .is_err()
        {
            failed = true;
        }
    }
    if failed {
        SettingsSaveError::CompensationFailed
    } else {
        SettingsSaveError::SaveFailed
    }
}
