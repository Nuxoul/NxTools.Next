//! Strict runtime-owned storage calls. Parse and bound before requesting a data identity.
use crate::{
    HostReply,
    storage::{StorageError, StorageNamespace},
};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Get {
    key: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Set {
    key: String,
    value: Value,
    expected_revision: Option<u64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Delete {
    key: String,
    expected_revision: Option<u64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct List {
    #[serde(default)]
    prefix: String,
    #[serde(default = "default_limit")]
    limit: usize,
    cursor: Option<Uuid>,
}
fn default_limit() -> usize {
    32
}
fn parse<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, StorageError> {
    serde_json::from_value(params).map_err(|_| StorageError::InvalidParams)
}
pub(crate) enum Call {
    Get(String),
    Set(String, Value, Option<u64>),
    Delete(String, Option<u64>),
    List(String, usize, Option<Uuid>),
}
impl Call {
    pub(crate) fn parse(method: &str, params: Value) -> Result<Self, HostReply> {
        let result: Result<Self, StorageError> = (|| {
            Ok(match method {
                "storage.get" => {
                    let p: Get = parse(params)?;
                    crate::storage::validate_key(&p.key)?;
                    Self::Get(p.key)
                }
                "storage.set" => {
                    if params.get("value").is_none() {
                        return Err(StorageError::InvalidParams);
                    }
                    let p: Set = parse(params)?;
                    crate::storage::validate_key(&p.key)?;
                    crate::storage::validate_value(&p.value)?;
                    revision(p.expected_revision)?;
                    Self::Set(p.key, p.value, p.expected_revision)
                }
                "storage.delete" => {
                    let p: Delete = parse(params)?;
                    crate::storage::validate_key(&p.key)?;
                    revision(p.expected_revision)?;
                    Self::Delete(p.key, p.expected_revision)
                }
                "storage.list" => {
                    let p: List = parse(params)?;
                    if p.prefix.len() > 128
                        || p.prefix.chars().any(char::is_control)
                        || !(1..=32).contains(&p.limit)
                    {
                        return Err(StorageError::InvalidParams);
                    }
                    Self::List(p.prefix, p.limit, p.cursor)
                }
                _ => return Err(StorageError::Unavailable),
            })
        })();
        result.map_err(|error| {
            HostReply::error(
                if matches!(
                    method,
                    "storage.get" | "storage.set" | "storage.delete" | "storage.list"
                ) {
                    error.code()
                } else {
                    "unsupported"
                },
            )
        })
    }
    pub(crate) async fn execute(self, namespace: StorageNamespace) -> HostReply {
        let result = match self {
            Self::Get(key) => namespace.get(key).await.and_then(value),
            Self::Set(key, item, expected) => {
                namespace.set(key, item, expected).await.and_then(value)
            }
            Self::Delete(key, expected) => namespace.delete(key, expected).await.and_then(value),
            Self::List(prefix, limit, cursor) => {
                namespace.list(prefix, limit, cursor).await.and_then(value)
            }
        };
        match result {
            Ok(value) => HostReply {
                success: true,
                value,
            },
            Err(error) => HostReply::error(error.code()),
        }
    }
}
fn value(item: impl serde::Serialize) -> Result<Value, StorageError> {
    serde_json::to_value(item).map_err(|_| StorageError::Corrupt)
}
fn revision(revision: Option<u64>) -> Result<(), StorageError> {
    if revision.is_some_and(|revision| revision > (1_u64 << 53) - 1) {
        Err(StorageError::InvalidParams)
    } else {
        Ok(())
    }
}
