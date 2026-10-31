use nxtools_extensions::{
    ImportPreview, InvocationIdentity, PackageRegistry,
    storage::{StorageError, StorageOwner},
};
use serde_json::json;
use std::{fs, time::Duration};
use tokio::sync::watch;
use uuid::Uuid;

fn registry(namespace: Option<Uuid>) -> PackageRegistry {
    let mut registry = PackageRegistry::default();
    registry.install(ImportPreview::from_json(br#"{"schema_version":1,"id":"fixture.restore","name":"Restore","version":"1","handlers":[{"id":"web","title":"Web","feature":"tools","entry":"action","action":{"type":"website","url_template":"https://fixture.test"}}]}"#).expect("package"), "owned".into(), false).expect("install");
    registry.packages[0].data_uuid = namespace;
    registry
}
fn identity() -> InvocationIdentity {
    InvocationIdentity {
        run_id: Uuid::new_v4(),
        correlation_id: Uuid::new_v4(),
        generation: 1,
        package_id: "fixture.restore".into(),
        handler_id: "web".into(),
        content_hash: "a".repeat(64),
    }
}

#[tokio::test]
async fn explicit_restore_rekeys_data_without_resurrecting_old_handles_and_recovers_pointer_barriers()
 {
    let root = tempfile::tempdir().expect("owned restore fixture");
    let directory = root.path().join("data");
    let owner = StorageOwner::new(directory.clone()).expect("owner");
    let source = Uuid::new_v4();
    let destination = Uuid::new_v4();
    let (_cancel, cancellation) = watch::channel(false);
    owner
        .reserve_namespace(source, "fixture.restore".into())
        .await
        .expect("source");
    let old_source = owner.namespace(source, identity(), cancellation.clone());
    old_source
        .set("kept".into(), json!({"nested":null}), Some(0))
        .await
        .expect("actual retained data");
    owner
        .reconcile_current_registry_blocking(&Default::default())
        .expect("removed source");
    let reviewed = owner
        .retained_metadata_blocking("fixture.restore")
        .expect("review")
        .remove(0);
    owner
        .reserve_namespace(destination, "fixture.restore".into())
        .await
        .expect("fresh target");
    let old_destination = owner.namespace(destination, identity(), cancellation.clone());
    let before = registry(Some(destination));
    let aborted = Uuid::new_v4();
    owner
        .prepare_restore_blocking("fixture.restore", &reviewed, Some(destination), aborted)
        .expect("durable restore intent");
    assert_eq!(
        old_destination.get("kept".into()).await,
        Err(StorageError::Stale),
        "prepared restore must quiesce old destination"
    );
    assert_eq!(
        old_source.get("kept".into()).await,
        Err(StorageError::Stale)
    );
    // Real persisted before-pointer barrier: fresh owner resolves the actual old pointer.
    let reopened = StorageOwner::new(directory.clone()).expect("crash recovery owner");
    reopened
        .reconcile_current_registry_blocking(&before)
        .expect("uncommitted pointer retains source and destination");
    assert!(
        !old_destination
            .get("kept".into())
            .await
            .expect("old empty destination remains active")
            .found
    );
    assert_eq!(
        reopened
            .retained_metadata_blocking("fixture.restore")
            .expect("source retained"),
        vec![reviewed.clone()]
    );
    let restored = Uuid::new_v4();
    reopened
        .prepare_restore_blocking("fixture.restore", &reviewed, Some(destination), restored)
        .expect("explicit new restore");
    let committed = registry(Some(restored));
    // Real persisted after-pointer barrier, without performing final catalog publication first.
    let recovered = StorageOwner::new(directory.clone()).expect("postcommit recovery owner");
    recovered
        .reconcile_current_registry_blocking(&committed)
        .expect("finish committed pointer");
    let current = recovered.namespace(restored, identity(), cancellation);
    let value = current.get("kept".into()).await.expect("restored value");
    assert!(value.found);
    assert_eq!(value.value, json!({"nested":null}));
    assert_eq!(value.revision, 1);
    assert_eq!(
        old_source.get("kept".into()).await,
        Err(StorageError::Stale)
    );
    assert_eq!(
        old_destination.get("kept".into()).await,
        Err(StorageError::Stale)
    );
    assert!(
        recovered
            .retained_metadata_blocking("fixture.restore")
            .expect("no retained duplicate")
            .is_empty()
    );
    assert!(!directory.join(format!("{source}.lock")).exists());
    assert!(!directory.join(format!("{destination}.lock")).exists());
    assert_eq!(
        current
            .set("new".into(), json!(true), Some(1))
            .await
            .expect("next CAS")
            .revision,
        2
    );
}

#[tokio::test]
async fn restore_waits_for_actual_io_and_rejects_foreign_stale_ambiguous_and_full_metadata() {
    use fs2::FileExt as _;
    let root = tempfile::tempdir().expect("owned restore limits");
    let directory = root.path().join("data");
    let owner = StorageOwner::new(directory.clone()).expect("owner");
    let source = Uuid::new_v4();
    owner
        .reserve_namespace(source, "fixture.restore".into())
        .await
        .expect("source");
    let (_cancel, cancellation) = watch::channel(false);
    owner
        .namespace(source, identity(), cancellation)
        .set("kept".into(), json!("durable"), None)
        .await
        .expect("data");
    owner
        .reconcile_current_registry_blocking(&Default::default())
        .expect("retained");
    let reviewed = owner
        .retained_metadata_blocking("fixture.restore")
        .expect("review")
        .remove(0);
    let before = fs::read(directory.join("catalog.json")).expect("before invalid review");
    let mut stale = reviewed.clone();
    stale.bytes += 1;
    assert_eq!(
        owner.prepare_restore_blocking("fixture.restore", &stale, None, Uuid::new_v4()),
        Err(StorageError::Stale)
    );
    assert_eq!(
        owner.prepare_restore_blocking("fixture.foreign", &reviewed, None, Uuid::new_v4()),
        Err(StorageError::Stale)
    );
    assert_eq!(
        fs::read(directory.join("catalog.json")).expect("no metadata mutation"),
        before
    );
    let lock = fs::OpenOptions::new()
        .write(true)
        .open(directory.join(format!("{source}.lock")))
        .expect("actual namespace lock");
    lock.lock_exclusive().expect("hold physical IO boundary");
    let task_owner = owner.clone();
    let task_review = reviewed.clone();
    let restored = Uuid::new_v4();
    let (send, receive) = std::sync::mpsc::channel();
    let task = std::thread::spawn(move || {
        send.send(task_owner.prepare_restore_blocking(
            "fixture.restore",
            &task_review,
            None,
            restored,
        ))
        .expect("receiver");
    });
    assert!(
        receive.recv_timeout(Duration::from_millis(50)).is_err(),
        "logical state cannot skip an actual leased file"
    );
    assert_eq!(
        fs::read(directory.join("catalog.json")).expect("blocked prepare no commit"),
        before
    );
    fs2::FileExt::unlock(&lock).expect("physical work finishes");
    receive
        .recv_timeout(Duration::from_secs(5))
        .expect("actual drain completes")
        .expect("prepared");
    task.join().expect("worker");
    let intent = fs::read(directory.join("catalog.json")).expect("durable intent");
    assert_eq!(
        owner.reconcile_current_registry_blocking(&registry(Some(Uuid::new_v4()))),
        Err(StorageError::Corrupt)
    );
    assert_eq!(
        fs::read(directory.join("catalog.json")).expect("ambiguous pointer retains intent"),
        intent
    );
    owner
        .reconcile_current_registry_blocking(&registry(None))
        .expect("old unallocated pointer aborts intent");
    let mut catalog: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("catalog.json")).expect("catalog"))
            .expect("json");
    let records = catalog["namespaces"].as_object_mut().expect("records");
    while records.len() < nxtools_extensions::storage::MAX_NAMESPACES {
        records.insert(Uuid::new_v4().to_string(), json!({"package":"fixture.restore","state":"retained","retirement":null,"file":null,"bytes":0,"revision":0}));
    }
    fs::write(
        directory.join("catalog.json"),
        serde_json::to_vec(&catalog).expect("bounded full catalog"),
    )
    .expect("actual metadata cap");
    let before = fs::read(directory.join("catalog.json")).expect("full metadata");
    assert_eq!(
        owner.prepare_restore_blocking("fixture.restore", &reviewed, None, Uuid::new_v4()),
        Err(StorageError::ResourceLimit)
    );
    assert_eq!(
        fs::read(directory.join("catalog.json")).expect("no eviction"),
        before
    );
}

#[cfg(windows)]
#[tokio::test]
async fn committed_restore_keeps_cleanup_slot_until_real_old_lock_deletion() {
    use std::os::windows::fs::OpenOptionsExt as _;
    let root = tempfile::tempdir().expect("owned cleanup fixture");
    let directory = root.path().join("data");
    let owner = StorageOwner::new(directory.clone()).expect("owner");
    let source = Uuid::new_v4();
    owner
        .reserve_namespace(source, "fixture.restore".into())
        .await
        .expect("source");
    let (_cancel, cancellation) = watch::channel(false);
    owner
        .namespace(source, identity(), cancellation.clone())
        .set("kept".into(), json!(true), None)
        .await
        .expect("data");
    owner
        .reconcile_current_registry_blocking(&Default::default())
        .expect("retained");
    let reviewed = owner
        .retained_metadata_blocking("fixture.restore")
        .expect("review")
        .remove(0);
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(directory.join(format!("{source}.lock")))
        .expect("deny actual lock deletion");
    let restored = Uuid::new_v4();
    owner
        .prepare_restore_blocking("fixture.restore", &reviewed, None, restored)
        .expect("prepare");
    let committed = registry(Some(restored));
    assert!(
        owner
            .reconcile_current_registry_blocking(&committed)
            .expect("pointer committed but lock cleanup pending")
    );
    assert_eq!(
        owner
            .namespace(restored, identity(), cancellation.clone())
            .get("kept".into())
            .await
            .expect("new pointer usable")
            .value,
        true
    );
    assert!(
        owner
            .retained_metadata_blocking("fixture.restore")
            .expect("cleanup is not a duplicate restorable generation")
            .is_empty()
    );
    let catalog: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("catalog.json")).expect("catalog"))
            .expect("json");
    assert_eq!(
        catalog["namespaces"].as_object().expect("records").len(),
        2,
        "actual lock still owns a finite metadata reservation"
    );
    drop(held);
    let reopened = StorageOwner::new(directory.clone()).expect("fresh cleanup owner");
    assert!(
        !reopened
            .reconcile_current_registry_blocking(&committed)
            .expect("explicit later reconciliation")
    );
    assert!(!directory.join(format!("{source}.lock")).exists());
    assert_eq!(
        owner
            .namespace(source, identity(), cancellation)
            .get("kept".into())
            .await,
        Err(StorageError::Stale)
    );
}

#[test]
fn retained_management_pages_are_bounded_and_do_not_expose_live_records() {
    let root = tempfile::tempdir().expect("owned paged metadata");
    let owner = StorageOwner::new(root.path().join("data")).expect("owner");
    for _ in 0..18 {
        owner
            .reserve_namespace_blocking(Uuid::new_v4(), "fixture.restore")
            .expect("reserve");
    }
    owner
        .reconcile_current_registry_blocking(&Default::default())
        .expect("retire");
    let live = Uuid::new_v4();
    owner
        .reserve_namespace_blocking(live, "fixture.restore")
        .expect("live");
    let first = owner.retained_page_blocking(None, None).expect("first");
    assert_eq!(first.items.len(), 16);
    let second = owner
        .retained_page_blocking(None, first.next_after)
        .expect("second");
    assert_eq!(second.items.len(), 2);
    assert!(second.next_after.is_none());
    assert!(
        first
            .items
            .iter()
            .chain(&second.items)
            .all(|item| item.package == "fixture.restore" && item.storage.namespace != live)
    );
    assert!(
        owner
            .retained_page_blocking(Some("fixture.other".into()), None)
            .expect("filter")
            .items
            .is_empty()
    );
}
