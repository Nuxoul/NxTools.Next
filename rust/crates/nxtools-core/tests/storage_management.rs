//! Actual locked Core configuration + durable owner; no product/native process is launched.
use nxtools_core::{AppConfig, ConfigExtensionStorage};
use nxtools_extensions::{
    ImportPreview, InstallationDataId as Uuid, InvocationIdentity, RuntimeIdentity,
    storage::{StorageError, StorageHost, StorageNamespace, StorageOwner},
};
use serde_json::json;
use std::{fs, path::PathBuf};
use tokio::sync::watch;

struct Fixture {
    root: tempfile::TempDir,
    path: PathBuf,
    owner: StorageOwner,
    cancellation: watch::Receiver<bool>,
    _cancel: watch::Sender<bool>,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().expect("owned management fixture");
        let path = root.path().join("config.json");
        let source = root.path().join("source");
        fs::create_dir(&source).expect("source");
        let manifest = json!({"schema_version":1,"id":"fixture.manage","name":"Manage","version":"1","runtime":{"program":"python","protocol_version":2},"files":["entry.py"],"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":"entry.py"}}]});
        fs::write(
            source.join("extension.json"),
            serde_json::to_vec(&manifest).expect("json"),
        )
        .expect("manifest");
        fs::write(source.join("entry.py"), b"never executed").expect("owned code");
        fs::write(
            root.path().join("fixture.exe"),
            b"synthetic interpreter; never executed",
        )
        .expect("interpreter");
        let mut config = AppConfig::default();
        config.download.default_connection_limit = 23;
        config.save_to(&path).expect("seed");
        let owner = StorageOwner::new(root.path().join("data")).expect("owner");
        let (_cancel, cancellation) = watch::channel(false);
        let fixture = Self {
            root,
            path,
            owner,
            cancellation,
            _cancel,
        };
        fixture.import();
        fixture
    }
    fn config(&self) -> AppConfig {
        AppConfig::load_from(&self.path).expect("fresh config")
    }
    fn import(&self) {
        AppConfig::import_extension_preview_checked(
            &self.path,
            ImportPreview::from_directory(&self.root.path().join("source")).expect("preview"),
            &self.root.path().join("packages"),
            "owned".into(),
            None,
        )
        .expect("checked import");
    }
    fn approve(&self) {
        let package = self.config().extensions.packages.remove(0);
        AppConfig::approve_extension(
            &self.path,
            "fixture.manage",
            &package.content_hash,
            RuntimeIdentity::inspect(&self.root.path().join("fixture.exe")).expect("identity"),
        )
        .expect("approval");
        AppConfig::set_extension_enabled(&self.path, "fixture.manage", true)
            .expect("separate enable");
    }
    fn bind(&self) -> Uuid {
        ConfigExtensionStorage::new(self.path.clone(), self.owner.clone())
            .bind(&self.config().extensions.packages[0], &|| false)
            .expect("binding")
            .data_uuid
            .expect("UUID")
    }
    fn namespace(&self, namespace: Uuid) -> StorageNamespace {
        self.owner.namespace(
            namespace,
            InvocationIdentity {
                run_id: Uuid::new_v4(),
                correlation_id: Uuid::new_v4(),
                generation: 1,
                package_id: "fixture.manage".into(),
                handler_id: "run".into(),
                content_hash: "a".repeat(64),
            },
            self.cancellation.clone(),
        )
    }
}

#[cfg(windows)]
#[tokio::test]
async fn restore_checks_current_trust_empty_target_and_failed_core_commit_preserves_values() {
    use std::os::windows::fs::OpenOptionsExt as _;
    let fixture = Fixture::new();
    fixture.approve();
    let source = fixture.bind();
    let old_source = fixture.namespace(source);
    old_source
        .set("kept".into(), json!({"source":null}), Some(0))
        .await
        .expect("actual value");
    let original = fixture.config();
    AppConfig::remove_extension_data_checked(
        &fixture.path,
        "fixture.manage",
        &original.extensions,
        &fixture.owner,
        false,
    )
    .expect("default retain");
    let retained = fixture
        .owner
        .retained_metadata_blocking("fixture.manage")
        .expect("review")
        .remove(0);
    fixture.import();
    let untrusted = fixture.config().extensions.packages.remove(0);
    let before = fs::read(&fixture.path).expect("untrusted bytes");
    assert!(
        AppConfig::restore_extension_data_checked(
            &fixture.path,
            &untrusted,
            &retained,
            &fixture.owner,
            &|| false
        )
        .is_err()
    );
    assert_eq!(
        fs::read(&fixture.path).expect("untrusted did not write"),
        before
    );
    fixture.approve();
    let destination = fixture.bind();
    let old_destination = fixture.namespace(destination);
    old_destination
        .set("target".into(), json!("no overwrite"), Some(0))
        .await
        .expect("nonempty target");
    let reviewed = fixture.config().extensions.packages.remove(0);
    assert!(
        AppConfig::restore_extension_data_checked(
            &fixture.path,
            &reviewed,
            &retained,
            &fixture.owner,
            &|| false
        )
        .is_err()
    );
    assert_eq!(
        old_destination
            .get("target".into())
            .await
            .expect("unchanged target")
            .value,
        "no overwrite"
    );
    old_destination
        .delete("target".into(), Some(1))
        .await
        .expect("explicitly empty target");
    let before = fs::read(&fixture.path).expect("before failed commit");
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&fixture.path)
        .expect("actual config rename denial");
    assert!(
        AppConfig::restore_extension_data_checked(
            &fixture.path,
            &reviewed,
            &retained,
            &fixture.owner,
            &|| false
        )
        .is_err()
    );
    assert_eq!(fs::read(&fixture.path).expect("old config"), before);
    assert!(
        !old_destination
            .get("kept".into())
            .await
            .expect("failed commit reactivates old empty pointer")
            .found
    );
    assert_eq!(
        fixture
            .owner
            .retained_metadata_blocking("fixture.manage")
            .expect("source unchanged"),
        vec![retained.clone()]
    );
    drop(held);
    assert!(
        AppConfig::restore_extension_data_checked(
            &fixture.path,
            &reviewed,
            &retained,
            &fixture.owner,
            &|| true
        )
        .is_err()
    );
    let (restored, pending) = AppConfig::restore_extension_data_checked(
        &fixture.path,
        &reviewed,
        &retained,
        &fixture.owner,
        &|| false,
    )
    .expect("explicit restore");
    assert!(!pending);
    assert_eq!(restored.download.default_connection_limit, 23);
    let current = restored.extensions.packages[0]
        .data_uuid
        .expect("new pointer");
    assert_ne!(current, source);
    assert_ne!(current, destination);
    assert_eq!(
        fixture
            .namespace(current)
            .get("kept".into())
            .await
            .expect("restored actual value")
            .value,
        json!({"source":null})
    );
    assert_eq!(
        old_source.get("kept".into()).await,
        Err(StorageError::Stale)
    );
    assert_eq!(
        old_destination.get("kept".into()).await,
        Err(StorageError::Stale)
    );
    assert!(
        AppConfig::restore_extension_data_checked(
            &fixture.path,
            &reviewed,
            &retained,
            &fixture.owner,
            &|| false
        )
        .is_err(),
        "old package review is stale"
    );
}

#[cfg(windows)]
#[tokio::test]
async fn reviewed_retained_clear_and_retry_keep_data_until_actual_delete_without_config_rewrite() {
    use std::os::windows::fs::OpenOptionsExt as _;
    let fixture = Fixture::new();
    fixture.approve();
    let source = fixture.bind();
    let lease = fixture.namespace(source);
    lease
        .set("kept".into(), json!("actual last-good value"), Some(0))
        .await
        .expect("actual data");
    let original = fixture.config();
    let before = fs::read(&fixture.path).expect("original bytes");
    let held_config = fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&fixture.path)
        .expect("deny config rename");
    assert!(
        AppConfig::remove_extension_data_checked(
            &fixture.path,
            "fixture.manage",
            &original.extensions,
            &fixture.owner,
            true
        )
        .is_err()
    );
    assert_eq!(fs::read(&fixture.path).expect("config unchanged"), before);
    assert_eq!(
        lease
            .get("kept".into())
            .await
            .expect("failed remove cannot erase")
            .value,
        "actual last-good value"
    );
    drop(held_config);
    AppConfig::remove_extension_data_checked(
        &fixture.path,
        "fixture.manage",
        &original.extensions,
        &fixture.owner,
        false,
    )
    .expect("retain after successful removal");
    let reviewed_registry = fixture.config().extensions;
    let retained = fixture
        .owner
        .retained_metadata_blocking("fixture.manage")
        .expect("review")
        .remove(0);
    let mut stale = retained.clone();
    stale.revision += 1;
    let before = fs::read(&fixture.path).expect("removed bytes");
    assert!(
        AppConfig::clear_retained_extension_data_checked(
            &fixture.path,
            "fixture.manage",
            &reviewed_registry,
            &stale,
            &fixture.owner
        )
        .is_err()
    );
    let directory = fixture.root.path().join("data");
    let catalog: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("catalog.json")).expect("catalog"))
            .expect("json");
    let backing = directory.join(format!(
        "{}.json",
        catalog["namespaces"][source.to_string()]["file"]
            .as_str()
            .expect("backing")
    ));
    let held_data = fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&backing)
        .expect("actual deny-delete backing");
    assert!(
        AppConfig::clear_retained_extension_data_checked(
            &fixture.path,
            "fixture.manage",
            &reviewed_registry,
            &retained,
            &fixture.owner
        )
        .expect("truthful cleanup pending")
    );
    assert!(backing.is_file());
    assert_eq!(lease.get("kept".into()).await, Err(StorageError::Stale));
    let pending = fixture
        .owner
        .retained_metadata_blocking("fixture.manage")
        .expect("pending review")
        .remove(0);
    assert!(pending.cleanup_pending);
    assert_eq!(pending.bytes, retained.bytes);
    assert!(
        AppConfig::clear_retained_extension_data_checked(
            &fixture.path,
            "fixture.manage",
            &reviewed_registry,
            &retained,
            &fixture.owner
        )
        .is_err(),
        "old retained state is not a retry confirmation"
    );
    drop(held_data);
    assert!(
        !AppConfig::clear_retained_extension_data_checked(
            &fixture.path,
            "fixture.manage",
            &reviewed_registry,
            &pending,
            &fixture.owner
        )
        .expect("explicit fresh retry")
    );
    assert!(!backing.exists());
    assert!(!directory.join(format!("{source}.lock")).exists());
    assert!(
        fixture
            .owner
            .retained_metadata_blocking("fixture.manage")
            .expect("removed metadata")
            .is_empty()
    );
    assert_eq!(
        fs::read(&fixture.path).expect("clear never rewrote config"),
        before
    );
}

#[cfg(windows)]
#[tokio::test]
async fn checked_remove_with_clear_reports_removed_but_physical_cleanup_pending() {
    use std::os::windows::fs::OpenOptionsExt as _;
    let fixture = Fixture::new();
    fixture.approve();
    let namespace = fixture.bind();
    let lease = fixture.namespace(namespace);
    lease
        .set("kept".into(), json!("actual data"), None)
        .await
        .expect("data");
    let directory = fixture.root.path().join("data");
    let catalog: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("catalog.json")).expect("catalog"))
            .expect("json");
    let backing = directory.join(format!(
        "{}.json",
        catalog["namespaces"][namespace.to_string()]["file"]
            .as_str()
            .expect("backing")
    ));
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&backing)
        .expect("actual delete denial");
    let (removed, pending) = AppConfig::remove_extension_data_checked(
        &fixture.path,
        "fixture.manage",
        &fixture.config().extensions,
        &fixture.owner,
        true,
    )
    .expect("registry removal committed");
    assert!(pending);
    assert!(removed.extensions.packages.is_empty());
    assert!(fixture.config().extensions.packages.is_empty());
    assert!(backing.is_file());
    assert_eq!(lease.get("kept".into()).await, Err(StorageError::Stale));
    let retained = fixture
        .owner
        .retained_metadata_blocking("fixture.manage")
        .expect("cleanup review")
        .remove(0);
    assert!(retained.cleanup_pending);
    drop(held);
    assert!(
        !AppConfig::clear_retained_extension_data_checked(
            &fixture.path,
            "fixture.manage",
            &removed.extensions,
            &retained,
            &fixture.owner
        )
        .expect("explicit retry")
    );
    assert!(!backing.exists());
    assert!(
        fixture.config().extensions.packages.is_empty(),
        "cleanup must never resurrect registry"
    );
}

#[tokio::test]
async fn cancelled_restore_after_actual_io_wait_keeps_original_pointer_and_retained_values() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    use std::time::Duration;
    let fixture = Fixture::new();
    fixture.approve();
    let source = fixture.bind();
    fixture
        .namespace(source)
        .set("kept".into(), json!("retain on cancel"), None)
        .await
        .expect("data");
    AppConfig::remove_extension_data_checked(
        &fixture.path,
        "fixture.manage",
        &fixture.config().extensions,
        &fixture.owner,
        false,
    )
    .expect("retain");
    let retained = fixture
        .owner
        .retained_metadata_blocking("fixture.manage")
        .expect("review")
        .remove(0);
    fixture.import();
    fixture.approve();
    let reviewed = fixture.config().extensions.packages.remove(0);
    assert!(reviewed.data_uuid.is_none());
    let before = fs::read(&fixture.path).expect("before");
    let lock = fs::OpenOptions::new()
        .write(true)
        .open(
            fixture
                .root
                .path()
                .join("data")
                .join(format!("{source}.lock")),
        )
        .expect("real namespace lock");
    fs2::FileExt::lock_exclusive(&lock).expect("held actual IO");
    let cancelled = Arc::new(AtomicBool::new(false));
    let task_cancelled = cancelled.clone();
    let task_path = fixture.path.clone();
    let task_owner = fixture.owner.clone();
    let task_retained = retained.clone();
    let (send, receive) = mpsc::channel();
    let task = std::thread::spawn(move || {
        send.send(AppConfig::restore_extension_data_checked(
            &task_path,
            &reviewed,
            &task_retained,
            &task_owner,
            &|| task_cancelled.load(Ordering::Acquire),
        ))
        .expect("receiver");
    });
    assert!(
        receive.recv_timeout(Duration::from_millis(50)).is_err(),
        "actual file lease must drain"
    );
    cancelled.store(true, Ordering::Release);
    fs2::FileExt::unlock(&lock).expect("actual IO finishes");
    let result = receive
        .recv_timeout(Duration::from_secs(5))
        .expect("cancel returns");
    assert_eq!(
        result
            .expect_err("cannot commit after cancel")
            .downcast_ref::<StorageError>(),
        Some(&StorageError::Cancelled)
    );
    task.join().expect("worker");
    assert_eq!(fs::read(&fixture.path).expect("no pointer write"), before);
    assert_eq!(
        fixture
            .owner
            .retained_metadata_blocking("fixture.manage")
            .expect("retained"),
        vec![retained]
    );
}

#[test]
fn retained_metadata_review_is_fresh_read_only_and_never_retries_cleanup() {
    let fixture = Fixture::new();
    let namespace = Uuid::new_v4();
    fixture
        .owner
        .reserve_namespace_blocking(namespace, "fixture.manage")
        .expect("reserve");
    fixture
        .owner
        .reconcile_current_registry_blocking(&Default::default())
        .expect("retain");
    let reviewed = fixture.config().extensions;
    let before = fs::read(&fixture.path).expect("config bytes");
    let catalog = fs::read(fixture.root.path().join("data/catalog.json")).expect("catalog bytes");
    let page = AppConfig::review_retained_extension_data_checked(
        &fixture.path,
        &reviewed,
        &fixture.owner,
        None,
        None,
    )
    .expect("current metadata");
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].storage.namespace, namespace);
    assert_eq!(fs::read(&fixture.path).expect("no config rewrite"), before);
    assert_eq!(
        fs::read(fixture.root.path().join("data/catalog.json")).expect("no lifecycle mutation"),
        catalog
    );
    let mut defaults = reviewed.runtime_defaults.clone();
    defaults.insert("python".into(), fixture.root.path().join("different.exe"));
    AppConfig::save_extension_runtime_defaults(&fixture.path, defaults)
        .expect("concurrent identity change");
    let changed = fs::read(&fixture.path).expect("changed");
    assert!(
        AppConfig::review_retained_extension_data_checked(
            &fixture.path,
            &reviewed,
            &fixture.owner,
            None,
            None
        )
        .is_err()
    );
    assert_eq!(
        fs::read(&fixture.path).expect("stale read no write"),
        changed
    );
    fs::write(&fixture.path, b"corrupt owned configuration").expect("corrupt");
    assert!(
        AppConfig::review_retained_extension_data_checked(
            &fixture.path,
            &reviewed,
            &fixture.owner,
            None,
            None
        )
        .is_err()
    );
    assert_eq!(
        fs::read(fixture.root.path().join("data/catalog.json")).expect("error retains data"),
        catalog
    );
}

#[cfg(windows)]
#[test]
fn checked_cleanup_retry_keeps_restore_obligations_until_actual_lock_delete() {
    use std::os::windows::fs::OpenOptionsExt as _;
    let fixture = Fixture::new();
    fixture.approve();
    let source = fixture.bind();
    fixture
        .owner
        .verify_namespace_blocking(source, "fixture.manage")
        .expect("actual namespace lock file");
    let original = fixture.config();
    AppConfig::remove_extension_data_checked(
        &fixture.path,
        "fixture.manage",
        &original.extensions,
        &fixture.owner,
        false,
    )
    .expect("retire");
    let retained = fixture
        .owner
        .retained_metadata_blocking("fixture.manage")
        .expect("retained")
        .remove(0);
    fixture.import();
    fixture.approve();
    let target = fixture.config().extensions.packages.remove(0);
    let old_lock = fixture
        .root
        .path()
        .join("data")
        .join(format!("{source}.lock"));
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&old_lock)
        .expect("real lock deletion denial");
    let (restored, pending) = AppConfig::restore_extension_data_checked(
        &fixture.path,
        &target,
        &retained,
        &fixture.owner,
        &|| false,
    )
    .expect("restore");
    assert!(pending);
    assert!(
        fixture
            .owner
            .retained_metadata_blocking("fixture.manage")
            .expect("cleanup-only not restorable")
            .is_empty()
    );
    let ordinary = Uuid::new_v4();
    fixture
        .owner
        .reserve_namespace_blocking(ordinary, "fixture.keep")
        .expect("ordinary retained reservation");
    fixture
        .owner
        .reconcile_current_registry_blocking(&restored.extensions)
        .expect("retain ordinary generation");
    let before = fs::read(&fixture.path).expect("committed bytes");
    assert!(
        AppConfig::retry_extension_storage_cleanup_checked(
            &fixture.path,
            &original.extensions,
            &fixture.owner
        )
        .is_err()
    );
    assert!(
        AppConfig::retry_extension_storage_cleanup_checked(
            &fixture.path,
            &restored.extensions,
            &fixture.owner
        )
        .expect("still pending")
    );
    assert!(old_lock.exists());
    drop(held);
    assert!(
        !AppConfig::retry_extension_storage_cleanup_checked(
            &fixture.path,
            &restored.extensions,
            &fixture.owner
        )
        .expect("actual retry")
    );
    assert!(!old_lock.exists());
    assert_eq!(
        fixture
            .owner
            .retained_metadata_blocking("fixture.keep")
            .expect("ordinary retained data untouched")[0]
            .namespace,
        ordinary
    );
    assert_eq!(
        fs::read(&fixture.path).expect("retry never rewrites config"),
        before
    );
}
