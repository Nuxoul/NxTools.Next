use nxtools_extensions::{
    InvocationIdentity,
    storage::{StorageError, StorageOwner},
};
use serde_json::json;
use tokio::sync::watch;
use uuid::Uuid;

fn identity() -> InvocationIdentity {
    InvocationIdentity {
        run_id: Uuid::new_v4(),
        correlation_id: Uuid::new_v4(),
        generation: 1,
        package_id: "fixture.storage".into(),
        handler_id: "run".into(),
        content_hash: "a".repeat(64),
    }
}

#[tokio::test]
async fn durable_namespace_reopens_null_and_preserves_revision_on_cas_conflict() {
    let root = tempfile::tempdir().expect("owned storage root");
    let namespace = Uuid::new_v4();
    let owner = StorageOwner::new(root.path().join("data")).expect("bounded storage owner");
    owner
        .reserve_namespace(namespace, "fixture.storage".into())
        .await
        .expect("durable namespace reservation");
    let (_cancel, cancellation) = watch::channel(false);
    let lease = owner.namespace(namespace, identity(), cancellation.clone());
    let missing = lease.get("value".into()).await.expect("missing");
    assert!(!missing.found);
    assert_eq!(missing.revision, 0);
    let written = lease
        .set("value".into(), json!(null), Some(0))
        .await
        .expect("null is a value");
    assert_eq!(written.revision, 1);
    assert!(!written.cleanup_pending);
    drop(lease);
    drop(owner);

    let reopened = StorageOwner::new(root.path().join("data")).expect("fresh owner");
    let lease = reopened.namespace(namespace, identity(), cancellation);
    let value = lease.get("value".into()).await.expect("durable reopen");
    assert!(value.found);
    assert_eq!(value.value, json!(null));
    assert_eq!(value.revision, 1);
    assert_eq!(
        lease
            .set("value".into(), json!("must not replace"), Some(0))
            .await,
        Err(StorageError::Conflict)
    );
    assert_eq!(
        lease
            .get("value".into())
            .await
            .expect("unchanged after conflict"),
        value
    );
    assert_eq!(
        lease
            .delete("value".into(), Some(1))
            .await
            .expect("delete")
            .revision,
        2
    );
    let missing = lease
        .get("value".into())
        .await
        .expect("missing after delete");
    assert!(!missing.found);
    assert_eq!(missing.revision, 2);
}

#[tokio::test]
async fn missing_initialization_marker_never_reinitializes_existing_data() {
    let root = tempfile::tempdir().expect("owned root");
    let directory = root.path().join("data");
    let owner = StorageOwner::new(directory.clone()).expect("owner");
    let namespace = Uuid::new_v4();
    owner
        .reserve_namespace(namespace, "fixture.storage".into())
        .await
        .expect("reserve");
    let (_cancel, cancellation) = watch::channel(false);
    let lease = owner.namespace(namespace, identity(), cancellation);
    lease
        .set("kept".into(), json!("durable"), None)
        .await
        .expect("write");
    let before = std::fs::read(directory.join("catalog.json")).expect("durable catalog");
    std::fs::remove_file(directory.join("initialized")).expect("incomplete initialization fixture");
    assert_eq!(lease.get("kept".into()).await, Err(StorageError::Corrupt));
    assert_eq!(
        std::fs::read(directory.join("catalog.json")).expect("catalog retained"),
        before
    );
}

#[tokio::test]
async fn independent_owners_serialize_cas_and_corrupt_catalog_is_not_overwritten() {
    let root = tempfile::tempdir().expect("owned root");
    let directory = root.path().join("data");
    let first = StorageOwner::new(directory.clone()).expect("first owner");
    let second = StorageOwner::new(directory.clone()).expect("second owner");
    let namespace = Uuid::new_v4();
    first
        .reserve_namespace(namespace, "fixture.storage".into())
        .await
        .expect("reserve");
    let (_cancel, cancellation) = watch::channel(false);
    let left = first.namespace(namespace, identity(), cancellation.clone());
    let right = second.namespace(namespace, identity(), cancellation);
    let (a, b) = tokio::join!(
        left.set("winner".into(), json!("left"), Some(0)),
        right.set("winner".into(), json!("right"), Some(0))
    );
    assert!(matches!(
        (&a, &b),
        (Ok(_), Err(StorageError::Conflict)) | (Err(StorageError::Conflict), Ok(_))
    ));
    let value = left.get("winner".into()).await.expect("winner");
    assert_eq!(value.revision, 1);
    assert!(value.value == "left" || value.value == "right");
    let corrupt = b"{\"version\":1,\"version\":1,\"namespaces\":{},\"pending\":null}";
    std::fs::write(directory.join("catalog.json"), corrupt).expect("duplicate-key corruption");
    assert_eq!(
        right.set("winner".into(), json!("lost"), None).await,
        Err(StorageError::Corrupt)
    );
    assert_eq!(
        std::fs::read(directory.join("catalog.json")).expect("corrupt retained"),
        corrupt
    );
}

#[tokio::test]
async fn metadata_pages_bind_invocation_and_revision_without_returning_values() {
    let root = tempfile::tempdir().expect("owned root");
    let owner = StorageOwner::new(root.path().join("data")).expect("owner");
    let namespace = Uuid::new_v4();
    owner
        .reserve_namespace(namespace, "fixture.storage".into())
        .await
        .expect("reserve");
    let (_cancel, cancellation) = watch::channel(false);
    let lease = owner.namespace(namespace, identity(), cancellation.clone());
    for index in 0..70 {
        lease
            .set(format!("key-{index:02}"), json!("not list metadata"), None)
            .await
            .expect("bounded write");
    }
    let first = lease.list("key-".into(), 32, None).await.expect("page one");
    assert_eq!(first.items.len(), 32);
    assert!(first.has_more);
    assert_eq!(first.items[0].key, "key-00");
    assert_eq!(first.items[0].revision, 1);
    assert!(
        !serde_json::to_value(&first).expect("metadata")["items"][0]
            .as_object()
            .expect("item")
            .contains_key("value")
    );
    let foreign = owner.namespace(namespace, identity(), cancellation);
    assert_eq!(
        foreign.list("key-".into(), 32, first.next_cursor).await,
        Err(StorageError::InvalidCursor)
    );
    let second = lease
        .list("key-".into(), 32, first.next_cursor)
        .await
        .expect("page two");
    assert_eq!(second.items.len(), 32);
    assert_eq!(second.items[0].key, "key-32");
    let last = lease
        .list("key-".into(), 32, second.next_cursor)
        .await
        .expect("page three");
    assert_eq!(last.items.len(), 6);
    assert!(!last.has_more);
    assert_eq!(last.next_cursor, None);
    lease
        .set("key-70".into(), json!(true), Some(70))
        .await
        .expect("mutation");
    assert_eq!(
        lease.list("key-".into(), 32, first.next_cursor).await,
        Err(StorageError::CursorStale)
    );
}

#[tokio::test]
async fn bounded_keys_values_depth_and_namespace_isolation_preserve_prior_revision() {
    let root = tempfile::tempdir().expect("owned root");
    let owner = StorageOwner::new(root.path().join("data")).expect("owner");
    let namespace = Uuid::new_v4();
    let other = Uuid::new_v4();
    owner
        .reserve_namespace(namespace, "fixture.storage".into())
        .await
        .expect("reserve");
    owner
        .reserve_namespace(other, "fixture.other".into())
        .await
        .expect("other reserve");
    let (cancel, cancellation) = watch::channel(false);
    let lease = owner.namespace(namespace, identity(), cancellation.clone());
    let foreign = owner.namespace(other, identity(), cancellation);
    assert_eq!(foreign.get("value".into()).await, Err(StorageError::Stale));
    lease
        .set("界".repeat(42), json!(true), Some(0))
        .await
        .expect("126 UTF8 bytes");
    assert_eq!(
        lease.set("界".repeat(43), json!(true), None).await,
        Err(StorageError::InvalidParams)
    );
    assert_eq!(
        lease.set("bad\nkey".into(), json!(true), None).await,
        Err(StorageError::InvalidParams)
    );
    assert_eq!(
        lease
            .set("oversized".into(), json!("x".repeat(65535)), None)
            .await,
        Err(StorageError::InvalidParams)
    );
    let mut deep = json!(null);
    for _ in 0..17 {
        deep = json!([deep]);
    }
    assert_eq!(
        lease.set("deep".into(), deep, None).await,
        Err(StorageError::InvalidParams)
    );
    for index in 0..255 {
        lease
            .set(format!("key-{index}"), json!(index), None)
            .await
            .expect("bounded key");
    }
    assert_eq!(
        lease.set("key-overflow".into(), json!(true), None).await,
        Err(StorageError::ResourceLimit)
    );
    let value = lease
        .get("key-254".into())
        .await
        .expect("prior data remains");
    assert_eq!(value.revision, 256);
    cancel.send(true).expect("cancel before admission");
    assert_eq!(
        lease.delete("key-254".into(), None).await,
        Err(StorageError::Cancelled)
    );
}

#[cfg(windows)]
#[tokio::test]
async fn physical_copy_delete_failure_retains_reservation_across_owner_reopen() {
    use std::os::windows::fs::OpenOptionsExt as _;
    let root = tempfile::tempdir().expect("owned root");
    let directory = root.path().join("data");
    let owner = StorageOwner::new(directory.clone()).expect("owner");
    let namespace = Uuid::new_v4();
    owner
        .reserve_namespace(namespace, "fixture.storage".into())
        .await
        .expect("reserve");
    let (_cancel, cancellation) = watch::channel(false);
    let lease = owner.namespace(namespace, identity(), cancellation.clone());
    lease
        .set("value".into(), json!("old"), None)
        .await
        .expect("first commit");
    let catalog: serde_json::Value = serde_json::from_slice(
        &std::fs::read(directory.join("catalog.json")).expect("owned catalog"),
    )
    .expect("catalog");
    let backing = catalog["namespaces"][namespace.to_string()]["file"]
        .as_str()
        .expect("opaque file");
    let old_path = directory.join(format!("{backing}.json"));
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&old_path)
        .expect("real deny-delete handle");
    let committed = lease
        .set("value".into(), json!("new"), Some(1))
        .await
        .expect("commit is not rolled back by cleanup failure");
    assert_eq!(committed.revision, 2);
    assert!(committed.cleanup_pending);
    assert!(old_path.is_file());
    drop(lease);
    drop(owner);
    let reopened = StorageOwner::new(directory.clone()).expect("new IO owner");
    let lease = reopened.namespace(namespace, identity(), cancellation);
    assert_eq!(
        lease
            .get("value".into())
            .await
            .expect("committed fact")
            .value,
        "new"
    );
    assert_eq!(
        lease
            .set("value".into(), json!("must not allocate"), Some(2))
            .await,
        Err(StorageError::CleanupPending)
    );
    drop(held);
    let committed = lease
        .set("value".into(), json!("after physical cleanup"), Some(2))
        .await
        .expect("explicit next write drains reservation");
    assert_eq!(committed.revision, 3);
    assert!(!committed.cleanup_pending);
    assert!(!old_path.exists());
}

#[tokio::test]
async fn prepared_retirement_keeps_active_data_until_fresh_registry_confirms_removal() {
    use nxtools_extensions::{ImportPreview, PackageRegistry};
    let root = tempfile::tempdir().expect("owned root");
    let owner = StorageOwner::new(root.path().join("data")).expect("owner");
    let namespace = Uuid::new_v4();
    owner
        .reserve_namespace(namespace, "fixture.storage".into())
        .await
        .expect("reserve");
    let (_cancel, cancellation) = watch::channel(false);
    let lease = owner.namespace(namespace, identity(), cancellation.clone());
    lease
        .set("retained".into(), json!("original"), None)
        .await
        .expect("actual data");
    let mut registry = PackageRegistry::default();
    registry.install(ImportPreview::from_json(br#"{"schema_version":1,"id":"fixture.storage","name":"Storage","version":"1","handlers":[{"id":"web","title":"Web","feature":"tools","entry":"action","action":{"type":"website","url_template":"https://fixture.test"}}]}"#).expect("package"), "owned".into(), false).expect("install");
    registry.packages[0].data_uuid = Some(namespace);
    owner
        .prepare_retirement_blocking(namespace, "fixture.storage", true)
        .expect("durable preparation only");
    assert_eq!(
        lease
            .get("retained".into())
            .await
            .expect("preparation is not removal")
            .value,
        "original"
    );
    owner
        .reconcile_current_registry_blocking(&registry)
        .expect("failed or cancelled Core removal retains current pointer");
    assert_eq!(
        lease
            .get("retained".into())
            .await
            .expect("still active")
            .value,
        "original"
    );
    owner
        .prepare_retirement_blocking(namespace, "fixture.storage", false)
        .expect("default retain preparation");
    let mut removed = registry.clone();
    removed
        .remove("fixture.storage")
        .expect("registry removal fixture");
    owner
        .reconcile_current_registry_blocking(&removed)
        .expect("retire after confirmed commit");
    assert_eq!(lease.get("retained".into()).await, Err(StorageError::Stale));
    let retained = owner
        .retained_metadata_blocking("fixture.storage")
        .expect("retained metadata");
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0].namespace, namespace);
    assert_eq!(retained[0].revision, 1);
    assert!(retained[0].bytes > 0);
    let fresh = Uuid::new_v4();
    owner
        .reserve_namespace(fresh, "fixture.storage".into())
        .await
        .expect("same-ID fresh namespace");
    assert!(
        !owner
            .namespace(fresh, identity(), cancellation)
            .get("retained".into())
            .await
            .expect("no automatic inheritance")
            .found
    );
}

#[cfg(windows)]
#[tokio::test]
async fn checked_clear_denial_keeps_retired_metadata_until_actual_delete_and_recovers_after_delete_crash()
 {
    use std::os::windows::fs::OpenOptionsExt as _;
    let root = tempfile::tempdir().expect("owned root");
    let directory = root.path().join("data");
    let owner = StorageOwner::new(directory.clone()).expect("owner");
    let namespace = Uuid::new_v4();
    owner
        .reserve_namespace(namespace, "fixture.storage".into())
        .await
        .expect("reserve");
    let (_cancel, cancellation) = watch::channel(false);
    let lease = owner.namespace(namespace, identity(), cancellation);
    lease
        .set("kept".into(), json!("actual data"), None)
        .await
        .expect("write");
    let catalog: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("catalog.json")).expect("catalog"))
            .expect("json");
    let backing = catalog["namespaces"][namespace.to_string()]["file"]
        .as_str()
        .expect("opaque backing");
    let path = directory.join(format!("{backing}.json"));
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&path)
        .expect("deny delete");
    owner
        .prepare_retirement_blocking(namespace, "fixture.storage", true)
        .expect("prepared explicit clear");
    assert!(
        owner
            .reconcile_current_registry_blocking(&Default::default())
            .expect("removed but cleanup pending")
    );
    assert_eq!(lease.get("kept".into()).await, Err(StorageError::Stale));
    let metadata = owner
        .retained_metadata_blocking("fixture.storage")
        .expect("pending metadata");
    assert_eq!(metadata.len(), 1);
    assert!(metadata[0].cleanup_pending);
    assert!(path.exists());
    drop(held);
    drop(lease);
    drop(owner);
    let reopened = StorageOwner::new(directory.clone()).expect("recovery owner");
    // Simulate the real crash barrier after physical delete but before catalog commit.
    std::fs::remove_file(&path).expect("delete owned pending backing");
    assert!(
        !reopened
            .reconcile_current_registry_blocking(&Default::default())
            .expect("recover completed physical deletion")
    );
    assert!(
        reopened
            .retained_metadata_blocking("fixture.storage")
            .expect("actual cleanup acknowledged")
            .is_empty()
    );
    assert!(
        !directory.join(format!("{namespace}.lock")).exists(),
        "released metadata slot must not leak an unbounded lock file"
    );
}

#[tokio::test]
async fn actual_full_global_and_namespace_files_include_retained_generations_without_eviction() {
    use nxtools_extensions::storage::{MAX_GLOBAL_BYTES, MAX_NAMESPACE_BYTES};
    let root = tempfile::tempdir().expect("owned quota fixture");
    let directory = root.path().join("data");
    let owner = StorageOwner::new(directory.clone()).expect("owner");
    let active = Uuid::new_v4();
    owner
        .reserve_namespace(active, "fixture.storage".into())
        .await
        .expect("initialize real store");
    // Real valid versioned documents at the exact configured boundary; do not lower
    // production limits or replace the actual owner/admission path with a predicate.
    let mut values = serde_json::Map::new();
    for index in 0..65 {
        values.insert(
            format!("key-{index:03}"),
            json!({"value":"x".repeat(64_500),"revision":1}),
        );
    }
    let mut document = json!({"version":1,"revision":1,"values":values});
    let excess = serde_json::to_vec(&document)
        .expect("document")
        .len()
        .checked_sub(MAX_NAMESPACE_BYTES)
        .expect("slightly oversized fixture");
    document["values"]["key-064"]["value"] = json!("x".repeat(64_500 - excess));
    let bytes = serde_json::to_vec(&document).expect("exact document");
    assert_eq!(bytes.len(), MAX_NAMESPACE_BYTES);
    let mut records = serde_json::Map::new();
    for index in 0..MAX_GLOBAL_BYTES / MAX_NAMESPACE_BYTES {
        let namespace = if index == 0 { active } else { Uuid::new_v4() };
        let file = Uuid::new_v4();
        std::fs::write(directory.join(format!("{file}.json")), &bytes)
            .expect("actual quota backing");
        records.insert(namespace.to_string(), json!({"package":"fixture.storage","state":if index==0 {"active"} else {"retained"},"file":file,"bytes":bytes.len(),"revision":1}));
    }
    let fresh = Uuid::new_v4();
    records.insert(
        fresh.to_string(),
        json!({"package":"fixture.storage","state":"active","file":null,"bytes":0,"revision":0}),
    );
    let catalog_path = directory.join("catalog.json");
    let catalog_bytes =
        serde_json::to_vec(&json!({"version":1,"namespaces":records,"pending":null}))
            .expect("catalog");
    std::fs::write(&catalog_path, &catalog_bytes).expect("owned full quota state");
    let (_cancel, cancellation) = watch::channel(false);
    let full = owner.namespace(active, identity(), cancellation.clone());
    assert_eq!(
        full.set("overflow".into(), json!("x"), Some(1)).await,
        Err(StorageError::ResourceLimit)
    );
    let empty = owner.namespace(fresh, identity(), cancellation.clone());
    assert_eq!(
        empty.set("new".into(), json!("x"), Some(0)).await,
        Err(StorageError::ResourceLimit)
    );
    assert_eq!(
        std::fs::read(&catalog_path).expect("no quota eviction"),
        catalog_bytes
    );
    assert_eq!(
        owner
            .retained_metadata_blocking("fixture.storage")
            .expect("retained quota")
            .len(),
        31
    );
    assert_eq!(
        full.get("key-000".into())
            .await
            .expect("last good searchable")
            .revision,
        1
    );
    drop(full);
    drop(empty);
    drop(owner);
    let reopened = StorageOwner::new(directory).expect("new owner");
    assert_eq!(
        reopened
            .namespace(fresh, identity(), cancellation)
            .set("new".into(), json!("x"), None)
            .await,
        Err(StorageError::ResourceLimit)
    );
    assert_eq!(
        std::fs::read(&catalog_path).expect("durable quota retained"),
        catalog_bytes
    );
}

#[tokio::test]
async fn actual_metadata_slot_limit_preserves_every_prereserved_generation_on_reopen() {
    use nxtools_extensions::storage::MAX_NAMESPACES;
    let root = tempfile::tempdir().expect("owned metadata fixture");
    let directory = root.path().join("data");
    let owner = StorageOwner::new(directory.clone()).expect("owner");
    owner
        .reserve_namespace(Uuid::new_v4(), "fixture.storage".into())
        .await
        .expect("initialize");
    let mut records = serde_json::Map::new();
    for _ in 0..MAX_NAMESPACES {
        records.insert(Uuid::new_v4().to_string(), json!({"package":"fixture.storage","state":"retained","file":null,"bytes":0,"revision":0}));
    }
    let path = directory.join("catalog.json");
    let bytes = serde_json::to_vec(&json!({"version":1,"namespaces":records,"pending":null}))
        .expect("bounded metadata");
    std::fs::write(&path, &bytes).expect("actual full metadata catalog");
    assert_eq!(
        owner
            .reserve_namespace(Uuid::new_v4(), "fixture.storage".into())
            .await,
        Err(StorageError::ResourceLimit)
    );
    assert_eq!(std::fs::read(&path).expect("not evicted"), bytes);
    drop(owner);
    let reopened = StorageOwner::new(directory).expect("reopened");
    assert_eq!(
        reopened
            .retained_metadata_blocking("fixture.storage")
            .expect("all retained")
            .len(),
        MAX_NAMESPACES
    );
    assert_eq!(
        reopened.reserve_namespace_blocking(Uuid::new_v4(), "fixture.storage"),
        Err(StorageError::ResourceLimit)
    );
    assert_eq!(std::fs::read(path).expect("stable full catalog"), bytes);
}

#[tokio::test]
async fn cancellation_and_dropped_consumer_behind_actual_quota_lock_never_publish_or_free_io_early()
{
    let root = tempfile::tempdir().expect("owned lock fixture");
    let directory = root.path().join("data");
    let owner = StorageOwner::new(directory.clone()).expect("owner");
    let namespace = Uuid::new_v4();
    owner
        .reserve_namespace(namespace, "fixture.storage".into())
        .await
        .expect("reserve");
    let (cancel, cancellation) = watch::channel(false);
    let lease = owner.namespace(namespace, identity(), cancellation);
    lease
        .set("value".into(), json!("last-good"), None)
        .await
        .expect("seed");
    let before = std::fs::read(directory.join("catalog.json")).expect("before");
    let lock = std::fs::OpenOptions::new()
        .write(true)
        .open(directory.join("quota.lock"))
        .expect("existing lock");
    fs2::FileExt::lock_exclusive(&lock).expect("block actual IO owner");
    let waiting = lease.clone();
    let task = tokio::spawn(async move {
        waiting
            .set("value".into(), json!("cancelled"), Some(1))
            .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    assert!(!task.is_finished());
    cancel.send(true).expect("cancel");
    fs2::FileExt::unlock(&lock).expect("release");
    assert_eq!(
        task.await.expect("worker result"),
        Err(StorageError::Cancelled)
    );
    let (_next_cancel, next_cancellation) = watch::channel(false);
    let lease = owner.namespace(namespace, identity(), next_cancellation);
    fs2::FileExt::lock_exclusive(&lock).expect("hold again");
    let waiting = lease.clone();
    let task = tokio::spawn(async move {
        waiting
            .set("value".into(), json!("abandoned"), Some(1))
            .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    assert!(!task.is_finished());
    task.abort();
    assert!(task.await.expect_err("consumer aborted").is_cancelled());
    let reading = lease.clone();
    let read = tokio::spawn(async move { reading.get("value".into()).await });
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    assert!(
        !read.is_finished(),
        "actual worker remains occupied until physical lock release"
    );
    fs2::FileExt::unlock(&lock).expect("physical release");
    assert_eq!(
        read.await.expect("reader").expect("last good").value,
        "last-good"
    );
    assert_eq!(
        std::fs::read(directory.join("catalog.json")).expect("no abandoned commit"),
        before
    );
}

#[cfg(windows)]
#[tokio::test]
async fn actual_catalog_rename_denial_preserves_last_good_revision_and_data() {
    use std::os::windows::fs::OpenOptionsExt as _;
    let root = tempfile::tempdir().expect("owned rename fixture");
    let directory = root.path().join("data");
    let owner = StorageOwner::new(directory.clone()).expect("owner");
    let namespace = Uuid::new_v4();
    owner
        .reserve_namespace(namespace, "fixture.storage".into())
        .await
        .expect("reserve");
    let (_cancel, cancellation) = watch::channel(false);
    let lease = owner.namespace(namespace, identity(), cancellation);
    lease
        .set("value".into(), json!("last-good"), None)
        .await
        .expect("seed");
    let path = directory.join("catalog.json");
    let before = std::fs::read(&path).expect("before");
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&path)
        .expect("deny atomic catalog rename");
    assert_eq!(
        lease
            .set("value".into(), json!("not-committed"), Some(1))
            .await,
        Err(StorageError::Io)
    );
    assert_eq!(std::fs::read(&path).expect("catalog unchanged"), before);
    assert_eq!(
        lease.get("value".into()).await.expect("old value").value,
        "last-good"
    );
    drop(held);
    assert_eq!(
        lease
            .set("value".into(), json!("explicit retry"), Some(1))
            .await
            .expect("new explicit operation")
            .revision,
        2
    );
}
