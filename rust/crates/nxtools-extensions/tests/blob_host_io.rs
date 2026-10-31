use base64::{Engine as _, engine::general_purpose::STANDARD};
use nxtools_extensions::{
    InvocationIdentity,
    blobs::{BlobError, BlobStore, MAX_BLOB_BYTES, MAX_CHUNK_BYTES, MAX_GLOBAL_BLOB_BYTES},
};
use sha2::{Digest, Sha256};
use uuid::Uuid;
fn identity() -> InvocationIdentity {
    InvocationIdentity {
        run_id: Uuid::new_v4(),
        correlation_id: Uuid::new_v4(),
        generation: 1,
        package_id: "fixture.host-blob".into(),
        handler_id: "run".into(),
        content_hash: "a".repeat(64),
    }
}
async fn cleanup(store: &BlobStore) {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let _ = store.cleanup().await;
            if store.available_bytes() == MAX_GLOBAL_BLOB_BYTES {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("physical backing cleanup");
}
#[tokio::test]
async fn expansion_http_host_blob_reserves_before_write_and_seals_actual_bytes_for_original_ack_reader()
 {
    let root = tempfile::tempdir().expect("owned root");
    let store = BlobStore::new(root.path().join("backing"));
    let run = store.invocation(identity(), Uuid::new_v4());
    let mut writer = run
        .host_writer(2 * MAX_CHUNK_BYTES, "application/octet-stream".into())
        .await
        .expect("reserve before HTTP");
    assert_eq!(
        store.available_bytes(),
        MAX_GLOBAL_BLOB_BYTES - 2 * MAX_CHUNK_BYTES
    );
    assert!(
        !run.quiescent(),
        "unsealed host response is not protocol completion"
    );
    let bytes = vec![42_u8; MAX_CHUNK_BYTES + 17];
    for chunk in bytes.chunks(MAX_CHUNK_BYTES) {
        writer
            .write(chunk.to_vec())
            .await
            .expect("bounded host chunk");
    }
    let blob = writer.seal().await.expect("actual response complete");
    assert_eq!(blob.length, bytes.len());
    assert_eq!(blob.sha256, format!("{:x}", Sha256::digest(&bytes)));
    assert!(run.quiescent());
    assert_eq!(
        run.host_read_sealed(blob.blob_id, bytes.len())
            .await
            .expect("bounded sealed request body"),
        bytes
    );
    assert_eq!(
        run.host_read_sealed(blob.blob_id, bytes.len() - 1).await,
        Err(BlobError::ResourceLimit)
    );
    let read = run
        .read(blob.blob_id, 0, None)
        .expect("original data reader actual length");
    assert_eq!(read.length, bytes.len());
    let mut received = Vec::new();
    while let Some(chunk) = run
        .next_chunk(read.transfer_id)
        .await
        .expect("original chunk")
    {
        received.extend(STANDARD.decode(chunk.data).expect("original wire bytes"));
        run.acknowledge(read.transfer_id, chunk.sequence, received.len())
            .expect("original ACK");
    }
    assert_eq!(received, bytes);
    run.release(blob.blob_id)
        .await
        .expect("release physical backing");
    cleanup(&store).await;
}
#[tokio::test]
async fn expansion_http_host_blob_drop_overflow_zero_and_retirement_never_publish_partial_success()
{
    let root = tempfile::tempdir().expect("owned root");
    let store = BlobStore::new(root.path().join("backing"));
    let run = store.invocation(identity(), Uuid::new_v4());
    assert!(
        run.host_writer(MAX_BLOB_BYTES + 1, "text/plain".into())
            .await
            .is_err()
    );
    {
        let mut writer = run
            .host_writer(3, "text/plain".into())
            .await
            .expect("reservation");
        assert_eq!(
            writer.write(vec![1; MAX_CHUNK_BYTES + 1]).await,
            Err(BlobError::InvalidParams)
        );
        writer
            .write(b"abc".to_vec())
            .await
            .expect("at exact capacity");
        assert_eq!(
            writer.write(b"d".to_vec()).await,
            Err(BlobError::ResourceLimit)
        );
    }
    assert!(
        run.quiescent(),
        "drop retires incomplete host response without a result token"
    );
    cleanup(&store).await;
    let empty = run
        .host_writer(0, "application/octet-stream".into())
        .await
        .expect("empty reservation")
        .seal()
        .await
        .expect("zero-byte response");
    assert_eq!(empty.length, 0);
    assert_eq!(empty.sha256, format!("{:x}", Sha256::digest([])));
    assert!(
        run.host_read_sealed(empty.blob_id, 0)
            .await
            .expect("empty read")
            .is_empty()
    );
    run.release(empty.blob_id).await.expect("empty release");
    let mut writer = run
        .host_writer(3, "text/plain".into())
        .await
        .expect("new response");
    writer.write(b"a".to_vec()).await.expect("partial response");
    run.retire();
    assert_eq!(writer.write(b"b".to_vec()).await, Err(BlobError::Stale));
    assert!(
        writer.seal().await.is_err(),
        "retired invocation cannot publish a blob"
    );
    cleanup(&store).await;
}
#[tokio::test]
async fn expansion_http_host_blob_request_requires_same_run_and_sealed_upload() {
    let root = tempfile::tempdir().expect("owned root");
    let store = BlobStore::new(root.path().join("backing"));
    let run = store.invocation(identity(), Uuid::new_v4());
    let upload = run
        .begin(
            3,
            "text/plain".into(),
            format!("{:x}", Sha256::digest(b"abc")),
        )
        .await
        .expect("original upload");
    assert_eq!(
        run.host_read_sealed(upload.blob.blob_id, 3).await,
        Err(BlobError::InvalidTransfer)
    );
    run.upload(upload.transfer_id, 1, 0, STANDARD.encode(b"abc"))
        .await
        .expect("original wire upload");
    let blob = run.seal(upload.blob.blob_id).await.expect("original seal");
    let foreign = store.invocation(identity(), Uuid::new_v4());
    assert_eq!(
        foreign.host_read_sealed(blob.blob_id, 3).await,
        Err(BlobError::Stale)
    );
    assert_eq!(
        run.host_read_sealed(blob.blob_id, 3)
            .await
            .expect("same run sealed bytes"),
        b"abc"
    );
    run.retire();
    assert_eq!(
        run.host_read_sealed(blob.blob_id, 3).await,
        Err(BlobError::Stale)
    );
    cleanup(&store).await;
}
