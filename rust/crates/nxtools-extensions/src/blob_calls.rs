//! Strict runtime-owned blob methods; registration is applied before a reply is observable.
use crate::{HostContext, HostReply, blobs::BlobError};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

pub(crate) enum Transfer {
    Upload {
        id: Uuid,
        length: usize,
    },
    Download {
        id: Uuid,
        offset: usize,
        length: usize,
    },
    Release {
        upload: Uuid,
    },
}
pub(crate) struct Completion {
    pub reply: HostReply,
    pub transfer: Option<Transfer>,
}
impl From<HostReply> for Completion {
    fn from(reply: HostReply) -> Self {
        Self {
            reply,
            transfer: None,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Begin {
    length: usize,
    media_type: String,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Blob {
    blob_id: Uuid,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Read {
    blob_id: Uuid,
    #[serde(default)]
    offset: usize,
    length: Option<usize>,
}
fn parse<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, BlobError> {
    serde_json::from_value(params).map_err(|_| BlobError::InvalidParams)
}
pub(crate) async fn call(context: &HostContext, method: &str, params: Value) -> Completion {
    let Some(run) = &context.blobs else {
        return HostReply::error("unsupported_transport").into();
    };
    let result: Result<Completion, BlobError> = async {
        let (value, transfer) = match method {
            "blobs.begin" => {
                let p: Begin = parse(params)?;
                let upload = run.begin(p.length, p.media_type, p.sha256).await?;
                let transfer = Transfer::Upload {
                    id: upload.transfer_id,
                    length: upload.blob.length,
                };
                (
                    serde_json::to_value(upload).map_err(|_| BlobError::Io)?,
                    Some(transfer),
                )
            }
            "blobs.seal" => {
                let p: Blob = parse(params)?;
                (
                    serde_json::to_value(run.seal(p.blob_id).await?).map_err(|_| BlobError::Io)?,
                    None,
                )
            }
            "blobs.read" => {
                let p: Read = parse(params)?;
                let read = run.read(p.blob_id, p.offset, p.length)?;
                let transfer = Transfer::Download {
                    id: read.transfer_id,
                    offset: p.offset,
                    length: read.length,
                };
                (
                    serde_json::to_value(read).map_err(|_| BlobError::Io)?,
                    Some(transfer),
                )
            }
            "blobs.release" => {
                let p: Blob = parse(params)?;
                let upload = run.upload_id(p.blob_id)?;
                let result = run.release(p.blob_id).await;
                return Ok(Completion {
                    reply: match result {
                        Ok(()) => HostReply {
                            success: true,
                            value: serde_json::json!({}),
                        },
                        Err(failure) => HostReply::error(failure.code()),
                    },
                    transfer: upload.map(|upload| Transfer::Release { upload }),
                });
            }
            _ => return Ok(HostReply::error("unsupported").into()),
        };
        Ok(Completion {
            reply: HostReply {
                success: true,
                value,
            },
            transfer,
        })
    }
    .await;
    result.unwrap_or_else(|failure| HostReply::error(failure.code()).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest as _;
    #[tokio::test]
    async fn expansion_http_host_blob_release_does_not_unregister_a_nonexistent_wire_upload() {
        let root = tempfile::tempdir().expect("owned blob release");
        let store = crate::blobs::BlobStore::new(root.path().join("backing"));
        let identity = crate::InvocationIdentity {
            run_id: Uuid::new_v4(),
            correlation_id: Uuid::new_v4(),
            generation: 1,
            package_id: "fixture.http".into(),
            handler_id: "run".into(),
            content_hash: "a".repeat(64),
        };
        let run = store.invocation(identity.clone(), Uuid::new_v4());
        let (_cancel, cancellation) = tokio::sync::watch::channel(false);
        let context = HostContext {
            identity,
            feature: "tools".into(),
            entry: "action".into(),
            causality: crate::Causality::root(Uuid::new_v4()),
            cancellation,
            blobs: Some(run.clone()),
        };
        let blob = run
            .host_writer(3, "text/plain".into())
            .await
            .expect("host response")
            .seal()
            .await
            .expect("complete empty response");
        let completion = call(
            &context,
            "blobs.release",
            serde_json::json!({"blob_id":blob.blob_id}),
        )
        .await;
        assert!(completion.reply.success);
        assert!(
            completion.transfer.is_none(),
            "host response has no script upload to unregister"
        );
        let upload = run
            .begin(
                0,
                "text/plain".into(),
                format!("{:x}", sha2::Sha256::digest([])),
            )
            .await
            .expect("original upload");
        let completion = call(
            &context,
            "blobs.release",
            serde_json::json!({"blob_id":upload.blob.blob_id}),
        )
        .await;
        assert!(completion.reply.success);
        assert!(
            matches!(completion.transfer, Some(Transfer::Release { upload: id }) if id == upload.transfer_id)
        );
    }
}
