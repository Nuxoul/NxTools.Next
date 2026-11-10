use super::*;

fn identity() -> InvocationIdentity {
    InvocationIdentity {
        run_id: Uuid::new_v4(),
        correlation_id: Uuid::new_v4(),
        generation: 1,
        package_id: "fixture.blob-shutdown".into(),
        handler_id: "run".into(),
        content_hash: "a".repeat(64),
    }
}

#[tokio::test]
async fn blob_shutdown_retires_live_runs_physically_deletes_and_joins_owner() {
    let parent = tempfile::tempdir().expect("isolated blob shutdown");
    let directory = parent.path().join("backing");
    let store = BlobStore::new(directory.clone());
    let root = Arc::downgrade(&store.0.root);
    let run = store.invocation(identity(), Uuid::new_v4());
    let upload = run
        .begin(3, "text/plain".into(), "a".repeat(64))
        .await
        .expect("physical backing");
    let path = directory.join(upload.blob.blob_id.to_string());
    assert!(path.is_file());
    store.shutdown().await.expect("explicit shutdown");
    assert!(
        !path.exists(),
        "shutdown must retire logical access and finish physical deletion"
    );
    assert!(!directory.exists());
    assert_eq!(store.available_bytes(), MAX_GLOBAL_BLOB_BYTES);
    assert_eq!(
        root.strong_count(),
        1,
        "joined cleanup thread no longer owns Root"
    );
    assert_eq!(
        run.check(&run.0.identity, run.0.session_id),
        Err(BlobError::Stale)
    );
    let later = store.invocation(identity(), Uuid::new_v4());
    assert_eq!(
        later
            .begin(1, "text/plain".into(), "a".repeat(64))
            .await
            .err(),
        Some(BlobError::Stale)
    );
    store
        .shutdown()
        .await
        .expect("repeated shutdown is complete");
}

#[cfg(windows)]
#[tokio::test]
async fn blob_shutdown_waits_for_original_failed_delete_retry_without_releasing_permits() {
    use std::os::windows::fs::OpenOptionsExt as _;
    let parent = tempfile::tempdir().expect("isolated physical delete failure");
    let directory = parent.path().join("backing");
    let store = BlobStore::new(directory.clone());
    let root = Arc::downgrade(&store.0.root);
    let run = store.invocation(identity(), Uuid::new_v4());
    let upload = run
        .begin(3, "text/plain".into(), "a".repeat(64))
        .await
        .expect("backing");
    let path = directory.join(upload.blob.blob_id.to_string());
    let held = OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&path)
        .expect("self-owned no-delete handle");
    run.retire();
    assert_eq!(store.cleanup().await, Err(BlobError::Io));
    let first = store.clone();
    let second = store.clone();
    let first = tokio::spawn(async move { first.shutdown().await });
    let second = tokio::spawn(async move { second.shutdown().await });
    tokio::time::sleep(Duration::from_millis(80)).await;
    assert!(
        !first.is_finished() && !second.is_finished(),
        "every shutdown caller must wait for the same real cleanup owner"
    );
    assert!(path.exists());
    assert_eq!(store.available_bytes(), MAX_GLOBAL_BLOB_BYTES - 3);
    drop(held);
    tokio::time::timeout(Duration::from_secs(3), async {
        first
            .await
            .expect("first shutdown task")
            .expect("first drain");
        second
            .await
            .expect("second shutdown task")
            .expect("same drain");
    })
    .await
    .expect("physical unlock permits cleanup and join");
    assert!(!directory.exists());
    assert_eq!(store.available_bytes(), MAX_GLOBAL_BLOB_BYTES);
    assert_eq!(root.strong_count(), 1);
}

#[tokio::test]
async fn blob_shutdown_waits_for_cancelled_host_io_actual_return() {
    let parent = tempfile::tempdir().expect("isolated active host IO");
    let directory = parent.path().join("backing");
    let store = BlobStore::new(directory.clone());
    let run = store.invocation(identity(), Uuid::new_v4());
    let writer = run
        .host_writer(1024, "text/plain".into())
        .await
        .expect("host reservation");
    let backing = run
        .0
        .state
        .lock()
        .expect("run state")
        .blobs
        .values()
        .next()
        .expect("host backing")
        .clone();
    let (entered, started) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let retained = run.clone();
    let operation = tokio::spawn(async move {
        let _writer = writer;
        retained
            .io(move || {
                let mut state = backing.file.lock().map_err(|_| BlobError::Io)?;
                entered.send(()).expect("blocking write entered");
                wait.recv_timeout(Duration::from_secs(3))
                    .map_err(|_| BlobError::Io)?;
                state
                    .file
                    .as_mut()
                    .ok_or(BlobError::Io)?
                    .write_all(b"abc")
                    .map_err(|_| BlobError::Io)
            })
            .await
    });
    started.await.expect("actual IO owns backing");
    operation.abort();
    let _ = operation.await;
    run.retire();
    let retained = store.clone();
    let shutdown = tokio::spawn(async move { retained.shutdown().await });
    tokio::time::sleep(Duration::from_millis(80)).await;
    assert!(
        !shutdown.is_finished(),
        "logical cancellation must not complete shutdown ahead of actual IO"
    );
    assert_eq!(store.available_bytes(), MAX_GLOBAL_BLOB_BYTES - 1024);
    release.send(()).expect("finish actual write");
    tokio::time::timeout(Duration::from_secs(3), shutdown)
        .await
        .expect("cleanup finishes after actual IO")
        .expect("shutdown task")
        .expect("shutdown result");
    assert!(run.quiescent());
    assert!(!directory.exists());
    assert_eq!(store.available_bytes(), MAX_GLOBAL_BLOB_BYTES);
}

fn abandoned(parent: &std::path::Path) -> (PathBuf, PathBuf) {
    let directory = parent.join(format!("nxtools-extension-blobs-{}", Uuid::new_v4()));
    std::fs::create_dir(&directory).expect("owned abandoned run");
    std::fs::write(directory.join(".owner"), b"nxtools-extension-blobs\n").expect("owner marker");
    let blob = directory.join(Uuid::new_v4().to_string());
    std::fs::write(&blob, b"abc").expect("abandoned physical blob");
    (directory, blob)
}

async fn startup(parent: &std::path::Path) {
    let store = BlobStore::new(parent.join(format!("nxtools-extension-blobs-{}", Uuid::new_v4())));
    store
        .shutdown()
        .await
        .expect("startup recovery owner joined");
}

#[tokio::test]
async fn blob_recovery_reclaims_only_valid_unlocked_runs() {
    let parent = tempfile::tempdir().expect("isolated recovery parent");
    let (dead, blob) = abandoned(parent.path());
    let (live, live_blob) = abandoned(parent.path());
    let lease = OpenOptions::new()
        .read(true)
        .write(true)
        .open(live.join(".owner"))
        .expect("live marker");
    fs2::FileExt::lock_exclusive(&lease).expect("independent live instance");
    let (unknown, unknown_blob) = abandoned(parent.path());
    std::fs::remove_file(unknown.join(".owner")).expect("legacy unmarked run");
    let (invalid, invalid_blob) = abandoned(parent.path());
    std::fs::write(
        invalid.join(".owner"),
        vec![b'x'; b"nxtools-extension-blobs\n".len()],
    )
    .expect("foreign marker with matching length");
    let (nested, nested_blob) = abandoned(parent.path());
    std::fs::create_dir(nested.join("nested")).expect("unexpected directory");
    let foreign = parent.path().join("nxtools-extension-blobs-not-a-uuid");
    std::fs::create_dir(&foreign).expect("foreign prefix");
    std::fs::write(foreign.join(".owner"), b"nxtools-extension-blobs\n").expect("foreign marker");
    let active_directory = parent
        .path()
        .join(format!("nxtools-extension-blobs-{}", Uuid::new_v4()));
    let active = BlobStore::new(active_directory.clone());
    let active_run = active.invocation(identity(), Uuid::new_v4());
    let active_blob = active_run
        .begin(3, "text/plain".into(), "a".repeat(64))
        .await
        .expect("independent store owns real lease");
    startup(parent.path()).await;
    assert!(
        !blob.exists() && !dead.exists(),
        "startup must physically recover a proven inactive run"
    );
    for path in [&live_blob, &unknown_blob, &invalid_blob, &nested_blob] {
        assert!(path.exists(), "non-reclaimable run retained");
    }
    assert!(foreign.exists());
    drop(lease);
    startup(parent.path()).await;
    assert!(!live.exists(), "released instance becomes recoverable");
    assert!(
        active_directory
            .join(active_blob.blob.blob_id.to_string())
            .is_file(),
        "production live lease survives both recoveries"
    );
    active
        .shutdown()
        .await
        .expect("independent owner drains itself");
}

#[tokio::test]
async fn blob_recovery_bounds_run_batch_and_validates_all_direct_files_before_delete() {
    let parent = tempfile::tempdir().expect("bounded recovery parent");
    let mut runs = Vec::new();
    for _ in 0..35 {
        runs.push(abandoned(parent.path()).0);
    }
    startup(parent.path()).await;
    let remaining = runs.iter().filter(|path| path.exists()).count();
    assert_eq!(
        remaining, 3,
        "one startup reclaims at most 32 candidate runs"
    );
    startup(parent.path()).await;
    assert!(runs.iter().all(|path| !path.exists()));
    let (oversized, blob) = abandoned(parent.path());
    for _ in 0..256 {
        std::fs::write(oversized.join(Uuid::new_v4().to_string()), b"")
            .expect("bounded direct file fixture");
    }
    startup(parent.path()).await;
    assert!(
        blob.exists(),
        "oversized direct entry set is not partially deleted"
    );
    assert_eq!(
        std::fs::read_dir(&oversized)
            .expect("retained root")
            .count(),
        258
    );
    std::fs::remove_file(blob).expect("reduce direct files to legal maximum");
    startup(parent.path()).await;
    assert!(
        !oversized.exists(),
        "all 256 legal direct files are recoverable"
    );
}

#[cfg(windows)]
#[tokio::test]
async fn blob_recovery_failed_delete_keeps_owner_for_next_startup_retry() {
    use std::os::windows::fs::OpenOptionsExt as _;
    let parent = tempfile::tempdir().expect("recovery delete retry");
    let (directory, blob) = abandoned(parent.path());
    let held = OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&blob)
        .expect("deny deletion");
    startup(parent.path()).await;
    assert!(blob.exists() && directory.join(".owner").is_file());
    drop(held);
    startup(parent.path()).await;
    assert!(
        !directory.exists(),
        "later startup retries physical deletion"
    );
}

#[cfg(windows)]
#[test]
fn blob_shutdown_concurrent_waiters_do_not_occupy_all_blocking_io_workers() {
    use std::os::windows::fs::OpenOptionsExt as _;
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(4)
        .build()
        .expect("bounded executor");
    executor.block_on(async {
        let parent = tempfile::tempdir().expect("bounded join waiters");
        let directory = parent.path().join("backing");
        let store = BlobStore::new(directory.clone());
        let run = store.invocation(identity(), Uuid::new_v4());
        let upload = run.begin(3, "text/plain".into(), "a".repeat(64)).await.expect("backing");
        let held = OpenOptions::new().read(true).share_mode(3).open(directory.join(upload.blob.blob_id.to_string())).expect("held delete");
        let mut waiters = Vec::new();
        for _ in 0..4 {
            let retained = store.clone();
            waiters.push(tokio::spawn(async move { retained.shutdown().await }));
        }
        tokio::time::sleep(Duration::from_millis(80)).await;
        let mut io = tokio::task::spawn_blocking(|| ());
        let available = tokio::time::timeout(Duration::from_millis(150), &mut io).await.is_ok();
        drop(held);
        for waiter in waiters { waiter.await.expect("shutdown task").expect("physical drain"); }
        if !available { io.await.expect("probe after drain"); }
        assert!(available, "concurrent shutdown callers must share one blocking join and leave IO workers available");
    });
}

#[cfg(windows)]
#[tokio::test]
async fn blob_recovery_reparse_candidate_marker_and_child_never_touch_targets() {
    fn junction(link: &std::path::Path, target: &std::path::Path) {
        let result = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .expect("owned junction fixture command");
        assert!(result.status.success(), "owned temporary junction creation");
        assert!(is_reparse(
            &std::fs::symlink_metadata(link).expect("junction metadata")
        ));
    }
    let parent = tempfile::tempdir().expect("isolated reparse recovery");
    let target = parent.path().join("target");
    std::fs::create_dir(&target).expect("self-owned target");
    std::fs::write(target.join(".owner"), b"nxtools-extension-blobs\n").expect("target marker");
    let target_blob = target.join(Uuid::new_v4().to_string());
    std::fs::write(&target_blob, b"must survive recovery").expect("target bytes");
    let linked_run = parent
        .path()
        .join(format!("nxtools-extension-blobs-{}", Uuid::new_v4()));
    junction(&linked_run, &target);
    let (marker_run, marker_blob) = abandoned(parent.path());
    let marker_link = marker_run.join(".owner");
    std::fs::remove_file(&marker_link).expect("replace only fixture marker");
    junction(&marker_link, &target);
    let (child_run, child_blob) = abandoned(parent.path());
    let child_link = child_run.join(Uuid::new_v4().to_string());
    junction(&child_link, &target);
    startup(parent.path()).await;
    assert_eq!(
        std::fs::read(&target_blob).expect("target survives"),
        b"must survive recovery"
    );
    assert!(target.join(".owner").is_file());
    assert!(linked_run.exists() && marker_blob.is_file() && child_blob.is_file());
    for link in [&linked_run, &marker_link, &child_link] {
        assert!(is_reparse(
            &std::fs::symlink_metadata(link).expect("link retained")
        ));
        std::fs::remove_dir(link).expect("remove only owned junction");
    }
}

#[tokio::test]
async fn blob_shutdown_failed_owner_initialization_is_physically_owned_before_error() {
    let parent = tempfile::tempdir().expect("isolated marker failure");
    let directory = parent.path().join("backing");
    let store = BlobStore::new(directory.clone());
    {
        let mut lease = store
            .0
            .root
            .cleanup_lock
            .lock()
            .expect("original Root serialization");
        std::fs::create_dir(&directory).expect("owned run root");
        assert_eq!(
            recovery::create_lease_with(&directory, &mut lease, |file| {
                file.write_all(b"partial").map_err(|_| BlobError::Io)?;
                Err(BlobError::Io)
            }),
            Err(BlobError::Io)
        );
        assert_eq!(
            recovery::create_lease(&directory, &mut lease),
            Err(BlobError::Io),
            "unfinished lease cannot admit another blob"
        );
        assert!(
            directory.join(".owner").is_file(),
            "real file exists before initialization failure"
        );
    }
    assert_eq!(
        store.cleanup().await,
        Ok(()),
        "original owner must remove its failed marker initialization"
    );
    assert!(!directory.exists());
    let run = store.invocation(identity(), Uuid::new_v4());
    run.begin(3, "text/plain".into(), "a".repeat(64))
        .await
        .expect("idle cleanup permits a fresh initialized lease");
    store
        .shutdown()
        .await
        .expect("failure does not strand shutdown");
    assert!(!directory.exists());
}
