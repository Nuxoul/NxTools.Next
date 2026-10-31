use base64::{Engine as _, engine::general_purpose::STANDARD};
use nxtools_extensions::{
    InvocationIdentity,
    blobs::{BlobError, BlobStore, MAX_BLOB_BYTES, MAX_GLOBAL_BLOB_BYTES},
};
use sha2::{Digest, Sha256};
use uuid::Uuid;
fn identity() -> InvocationIdentity {
    InvocationIdentity {
        run_id: Uuid::new_v4(),
        correlation_id: Uuid::new_v4(),
        generation: 1,
        package_id: "fixture.blobs".into(),
        handler_id: "run".into(),
        content_hash: "a".repeat(64),
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
#[tokio::test]
async fn expansion_blob_store_actual_file_exact_chunks_hash_ack_and_cleanup() {
    let root = tempfile::tempdir().expect("owned blob root");
    let directory = root.path().join("backing");
    let store = BlobStore::new(directory.clone());
    let id = identity();
    let sid = Uuid::new_v4();
    let run = store.invocation(id.clone(), sid);
    let bytes = (0..1048576 + 19)
        .map(|i| (i % 251) as u8)
        .collect::<Vec<_>>();
    let upload = run
        .begin(
            bytes.len(),
            "application/octet-stream".into(),
            digest(&bytes),
        )
        .await
        .expect("begin");
    assert!(run.check(&id, sid).is_ok());
    assert_eq!(run.check(&identity(), sid), Err(BlobError::Stale));
    for (index, chunk) in bytes.chunks(65536).enumerate() {
        assert_eq!(
            run.upload(
                upload.transfer_id,
                index as u64 + 1,
                index * 65536,
                STANDARD.encode(chunk)
            )
            .await
            .expect("sequential write"),
            (index * 65536) + chunk.len()
        );
    }
    let sealed = run
        .seal(upload.blob.blob_id)
        .await
        .expect("complete digest");
    assert_eq!(sealed.sha256, digest(&bytes));
    let read = run.read(sealed.blob_id, 0, None).expect("read span");
    let mut received = Vec::new();
    while let Some(chunk) = run.next_chunk(read.transfer_id).await.expect("chunk") {
        assert_eq!(chunk.offset, received.len());
        let decoded = STANDARD.decode(chunk.data).expect("base64");
        assert!(decoded.len() <= 65536);
        received.extend(decoded);
        assert!(
            run.next_chunk(read.transfer_id).await.is_err(),
            "one outstanding chunk until ACK"
        );
        run.acknowledge(read.transfer_id, chunk.sequence, received.len())
            .expect("ACK");
    }
    assert_eq!(received, bytes);
    assert!(run.quiescent());
    run.release(sealed.blob_id)
        .await
        .expect("actual release cleanup");
    assert_eq!(store.available_bytes(), MAX_GLOBAL_BLOB_BYTES);
    assert!(!directory.exists());
}
#[tokio::test]
async fn expansion_blob_store_gap_overlap_digest_foreign_and_retired_handles_reject() {
    let root = tempfile::tempdir().expect("owned root");
    let store = BlobStore::new(root.path().join("backing"));
    let run = store.invocation(identity(), Uuid::new_v4());
    let upload = run
        .begin(3, "text/plain".into(), digest(b"abc"))
        .await
        .expect("begin");
    assert_eq!(
        run.upload(upload.transfer_id, 1, 1, STANDARD.encode(b"abc"))
            .await,
        Err(BlobError::InvalidTransfer)
    );
    assert_eq!(
        run.upload(upload.transfer_id, 2, 0, STANDARD.encode(b"abc"))
            .await,
        Err(BlobError::InvalidTransfer)
    );
    assert_eq!(
        run.upload(upload.transfer_id, 1, 0, "bad%%".into()).await,
        Err(BlobError::InvalidTransfer)
    );
    assert!(run.seal(upload.blob.blob_id).await.is_err());
    run.upload(upload.transfer_id, 1, 0, STANDARD.encode(b"abd"))
        .await
        .expect("write within shape");
    assert!(
        run.seal(upload.blob.blob_id).await.is_err(),
        "digest mismatch"
    );
    let other = store.invocation(identity(), Uuid::new_v4());
    assert!(other.read(upload.blob.blob_id, 0, None).is_err());
    run.retire();
    assert_eq!(
        run.upload(upload.transfer_id, 1, 0, STANDARD.encode(b"abc"))
            .await,
        Err(BlobError::Stale)
    );
    store.cleanup().await.expect("retired physical cleanup");
    assert_eq!(store.available_bytes(), MAX_GLOBAL_BLOB_BYTES);
}
#[tokio::test]
async fn expansion_blob_store_two_max_leases_fit_run_and_global_quotas_hold_until_cleanup() {
    let root = tempfile::tempdir().expect("owned quota root");
    let store = BlobStore::new(root.path().join("backing"));
    let mut runs = Vec::new();
    for _ in 0..4 {
        let run = store.invocation(identity(), Uuid::new_v4());
        for _ in 0..2 {
            run.begin(
                MAX_BLOB_BYTES,
                "application/octet-stream".into(),
                "a".repeat(64),
            )
            .await
            .expect("two8MiB reservations");
        }
        assert!(
            run.begin(1, "text/plain".into(), "a".repeat(64))
                .await
                .is_err()
        );
        runs.push(run);
    }
    assert_eq!(store.available_bytes(), 0);
    let extra = store.invocation(identity(), Uuid::new_v4());
    assert!(
        extra
            .begin(1, "text/plain".into(), "a".repeat(64))
            .await
            .is_err()
    );
    for run in &runs {
        run.retire();
    }
    store.cleanup().await.expect("actual owned cleanup");
    assert_eq!(store.available_bytes(), MAX_GLOBAL_BLOB_BYTES);
}
#[cfg(windows)]
#[tokio::test]
async fn expansion_blob_store_failed_delete_retains_exact_permit_until_file_released() {
    use std::os::windows::fs::OpenOptionsExt as _;
    let root = tempfile::tempdir().expect("owned locked root");
    let directory = root.path().join("backing");
    let store = BlobStore::new(directory.clone());
    let run = store.invocation(identity(), Uuid::new_v4());
    let upload = run
        .begin(3, "text/plain".into(), digest(b"abc"))
        .await
        .expect("begin");
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(directory.join(upload.blob.blob_id.to_string()));
    // The existing open write handle allows ordinary reads but Windows refuses an exclusive new handle.
    // Use a normal read handle that denies deletion while sharing read+write with that owner.
    assert!(held.is_err());
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(directory.join(upload.blob.blob_id.to_string()))
        .expect("held no-delete read");
    run.retire();
    assert_eq!(store.cleanup().await, Err(BlobError::Io));
    assert_eq!(store.available_bytes(), MAX_GLOBAL_BLOB_BYTES - 3);
    drop(held);
    store
        .cleanup()
        .await
        .expect("retry after actual file release");
    assert_eq!(store.available_bytes(), MAX_GLOBAL_BLOB_BYTES);
}

#[tokio::test]
async fn expansion_blob_store_last_owner_drop_still_drains_actual_backing() {
    let root = tempfile::tempdir().expect("owned detached cleanup root");
    let directory = root.path().join("backing");
    let store = BlobStore::new(directory.clone());
    let run = store.invocation(identity(), Uuid::new_v4());
    run.begin(3, "text/plain".into(), digest(b"abc"))
        .await
        .expect("created owned backing");
    drop(run);
    drop(store);
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while directory.exists() {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("last store drop must not abandon backing or cleanup reservations");
}

#[cfg(windows)]
#[tokio::test]
async fn expansion_blob_store_retry_owner_survives_store_drop_and_denied_delete() {
    use std::os::windows::fs::OpenOptionsExt as _;
    let root = tempfile::tempdir().expect("owned delayed cleanup root");
    let directory = root.path().join("backing");
    let store = BlobStore::new(directory.clone());
    let run = store.invocation(identity(), Uuid::new_v4());
    let upload = run
        .begin(3, "text/plain".into(), digest(b"abc"))
        .await
        .expect("owned backing");
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(directory.join(upload.blob.blob_id.to_string()))
        .expect("deny deletion");
    run.retire();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    assert_eq!(store.available_bytes(), MAX_GLOBAL_BLOB_BYTES - 3);
    drop(run);
    drop(store);
    assert!(
        directory.exists(),
        "failed deletion remains real retained backing"
    );
    drop(held);
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while directory.exists() {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("bounded retry owner outlives public store");
}
