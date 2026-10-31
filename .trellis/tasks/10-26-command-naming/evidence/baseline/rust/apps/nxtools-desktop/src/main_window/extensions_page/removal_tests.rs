use super::*;
use crate::extensions::{service::ExtensionService, ui_host::ExtensionServiceGlobal};
use gpui_kit::{AppContext, Modifiers, TestAppContext, VisualTestContext, WeakEntity, size};
use nxtools_core::AppConfig;
use nxtools_extensions::{InvocationIdentity, storage::StorageOwner};

struct Fixture {
    root: tempfile::TempDir,
    path: PathBuf,
    config: AppConfig,
    owner: StorageOwner,
    namespace: uuid::Uuid,
    _engine: nxtools_download::DownloadEngine,
    service: ExtensionService,
}
impl Fixture {
    fn new() -> Self {
        Self::new_inner(false)
    }
    fn new_script() -> Self {
        Self::new_inner(true)
    }
    fn new_inner(script: bool) -> Self {
        let root = tempfile::tempdir().expect("owned removal storage");
        let path = root.path().join("config.json");
        let mut config = AppConfig::default();
        config.extensions.install(nxtools_extensions::ImportPreview::from_json(br#"{"schema_version":1,"id":"fixture.remove","name":"Removal fixture","version":"1","handlers":[{"id":"open","title":"Open","feature":"tools","entry":"action","action":{"type":"website","url_template":"https://example.invalid/"}}]}"#).expect("manifest"), "owned".into(), false).expect("install");
        if script {
            let source = root.path().join("source");
            std::fs::create_dir(&source).expect("source");
            std::fs::write(source.join("entry.py"), b"never executed").expect("code");
            std::fs::write(
                root.path().join("fixture.exe"),
                b"synthetic interpreter; never executed",
            )
            .expect("runtime");
            std::fs::write(source.join("extension.json"),br#"{"schema_version":1,"id":"fixture.remove","name":"Removal fixture","version":"1","runtime":{"program":"python","protocol_version":2},"files":["entry.py"],"handlers":[{"id":"open","title":"Open","feature":"tools","entry":"action","action":{"type":"script","entry_file":"entry.py"}}]}"#).expect("manifest");
            config.save_to(&path).expect("base");
            AppConfig::remove_extension(&path, "fixture.remove")
                .expect("remove declarative fixture");
            config = AppConfig::import_extension_preview_checked(
                &path,
                nxtools_extensions::ImportPreview::from_directory(&source).expect("preview"),
                &root.path().join("packages"),
                "owned".into(),
                None,
            )
            .expect("original import");
            let hash = config.extensions.packages[0].content_hash.clone();
            config = AppConfig::approve_extension(
                &path,
                "fixture.remove",
                &hash,
                nxtools_extensions::RuntimeIdentity::inspect(&root.path().join("fixture.exe"))
                    .expect("identity"),
            )
            .expect("approval");
        }
        let namespace = uuid::Uuid::new_v4();
        config.extensions.packages[0].data_uuid = Some(namespace);
        config.save_to(&path).expect("seed");
        let engine = nxtools_download::DownloadEngine::open(root.path().join("downloads.db"))
            .expect("engine");
        let (service, _) = ExtensionService::new_with_storage(
            &engine,
            config.extensions.clone(),
            None,
            false,
            path.clone(),
            root.path().join("data"),
        )
        .expect("formal storage host");
        let owner = service.storage_owner().expect("same bounded owner");
        owner
            .reserve_namespace_blocking(namespace, "fixture.remove")
            .expect("reserve");
        let io = tokio::runtime::Runtime::new().expect("fixture IO");
        let (_cancel, cancellation) = tokio::sync::watch::channel(false);
        let identity = InvocationIdentity {
            run_id: uuid::Uuid::new_v4(),
            correlation_id: uuid::Uuid::new_v4(),
            generation: 1,
            package_id: "fixture.remove".into(),
            handler_id: "open".into(),
            content_hash: config.extensions.packages[0].content_hash.clone(),
        };
        io.block_on(owner.namespace(namespace, identity, cancellation).set(
            "kept".into(),
            serde_json::json!("actual data"),
            Some(0),
        ))
        .expect("actual persisted value");
        config.save_to(&path).expect("seed");
        Self {
            root,
            path,
            config,
            owner,
            namespace,
            _engine: engine,
            service,
        }
    }
    fn mount<'a>(
        &self,
        cx: &'a mut TestAppContext,
    ) -> (
        Entity<ExtensionApplication>,
        Entity<ExtensionManagerView>,
        &'a mut VisualTestContext,
        ExtensionService,
    ) {
        cx.update(gpui_kit::init);
        cx.executor().allow_parking();
        let service = self.service.clone();
        cx.update(|cx| cx.set_global(ExtensionServiceGlobal(service.clone())));
        let center = cx.new(|_| crate::notifications::NotificationCenter::default());
        cx.update(|cx| crate::notifications::register(&center, cx));
        let application = cx.new(|_| {
            ExtensionApplication::new(
                self.config.clone(),
                self.path.clone(),
                None,
                WeakEntity::new_invalid(),
            )
        });
        application.update(cx, |app, cx| {
            app.bind_ui(WeakEntity::new_invalid(), center, cx)
        });
        let owner = application.clone();
        let slot = Rc::new(RefCell::new(None));
        let retained = slot.clone();
        let (_, window) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| ExtensionManagerView::new(owner, window, cx));
            *retained.borrow_mut() = Some(view.clone());
            gpui_kit::base::Root::new(view, window, cx)
        });
        window.simulate_resize(size(px(744.0), px(430.0)));
        let view = slot.borrow().clone().expect("view");
        (application, view, window, service)
    }
    fn retained(&self) -> Vec<nxtools_extensions::storage::RetainedStorage> {
        self.owner
            .retained_metadata_blocking("fixture.remove")
            .expect("metadata")
    }
    fn backing(&self) -> PathBuf {
        let catalog: serde_json::Value = serde_json::from_slice(
            &std::fs::read(self.root.path().join("data/catalog.json")).expect("catalog"),
        )
        .expect("json");
        self.root.path().join("data").join(format!(
            "{}.json",
            catalog["namespaces"][self.namespace.to_string()]["file"]
                .as_str()
                .expect("file")
        ))
    }
}
fn click(window: &mut VisualTestContext, selector: &'static str) {
    window.update(|window, cx| window.draw(cx).clear(cx));
    let bounds = window
        .debug_bounds(selector)
        .expect("current production control");
    assert!(bounds.top() >= px(0.0) && bounds.bottom() <= px(430.0));
    window.simulate_click(bounds.center(), Modifiers::default());
    window.run_until_parked();
}
fn checkbox(window: &mut VisualTestContext) -> bool {
    window.update(|window, cx| {
        window.draw(cx).clear(cx);
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| {
                node.role() == Some(gpui_kit::Role::CheckBox)
                    && node
                        .path()
                        .iter()
                        .any(|id| id == &gpui_kit::ElementId::from("extension-remove-clear-data"))
            })
            .expect("official clear checkbox")
            .checked()
            .expect("checked value")
    })
}
fn idle(owner: &Entity<ExtensionApplication>, window: &mut VisualTestContext) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        window.run_until_parked();
        if !owner.read_with(window, |app, _| app.busy) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "actual removal terminal"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
fn review(view: &Entity<ExtensionManagerView>, window: &mut VisualTestContext) {
    view.update_in(window, |view, window, cx| {
        view.confirm_removal("fixture.remove".into(), window, cx)
    });
    window.run_until_parked();
}
#[gpui_kit::test]
fn expansion_storage_removal_default_retains_and_checked_cancel_never_erases(
    cx: &mut TestAppContext,
) {
    let fixture = Fixture::new();
    let before = std::fs::read(&fixture.path).expect("before");
    let backing = fixture.backing();
    let (owner, view, window, service) = fixture.mount(cx);
    review(&view, window);
    assert!(!checkbox(window), "destructive choice defaults off");
    click(window, "extension-remove-clear-data");
    assert!(checkbox(window));
    click(window, "extension-remove-cancel");
    assert_eq!(std::fs::read(&fixture.path).expect("unchanged"), before);
    assert!(backing.exists());
    assert!(fixture.retained().is_empty());
    review(&view, window);
    assert!(
        !checkbox(window),
        "reopening does not reuse destructive intent"
    );
    click(window, "extension-remove-confirm");
    idle(&owner, window);
    assert!(
        AppConfig::load_from(&fixture.path)
            .expect("removed")
            .extensions
            .packages
            .is_empty()
    );
    let retained = fixture.retained();
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0].revision, 1);
    assert!(!retained[0].cleanup_pending);
    assert!(backing.exists());
    assert_eq!(
        owner.read_with(window, |app, _| app.last_commit),
        Some(("remove", true, true))
    );
    service.runtime.shutdown();
}
#[cfg(windows)]
#[gpui_kit::test]
fn expansion_storage_removal_clear_failure_is_committed_with_cleanup_pending(
    cx: &mut TestAppContext,
) {
    use std::os::windows::fs::OpenOptionsExt as _;
    let fixture = Fixture::new();
    let backing = fixture.backing();
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&backing)
        .expect("real delete denial");
    let (owner, view, window, service) = fixture.mount(cx);
    review(&view, window);
    click(window, "extension-remove-clear-data");
    assert!(checkbox(window));
    click(window, "extension-remove-confirm");
    idle(&owner, window);
    assert!(
        AppConfig::load_from(&fixture.path)
            .expect("committed")
            .extensions
            .packages
            .is_empty()
    );
    assert!(owner.read_with(window, |app, _| app.packages.packages.is_empty()));
    assert!(backing.exists());
    assert!(fixture.retained()[0].cleanup_pending);
    let notice = window.update(|_, cx| {
        crate::notifications::current(cx)
            .expect("center")
            .read(cx)
            .items()
            .last()
            .expect("terminal notice")
            .clone()
    });
    assert!(
        notice.message.contains("已移除") && notice.message.contains("待清理"),
        "truthful committed/pending feedback: {}",
        notice.message
    );
    assert_ne!(
        notice.level,
        crate::notifications::NotificationLevel::Success
    );
    drop(held);
    service.runtime.shutdown();
}

#[gpui_kit::test]
fn expansion_storage_retained_data_remains_reachable_without_installed_package_and_clear_needs_review(
    cx: &mut TestAppContext,
) {
    let fixture = Fixture::new();
    let backing = fixture.backing();
    let (owner, view, window, service) = fixture.mount(cx);
    review(&view, window);
    click(window, "extension-remove-confirm");
    idle(&owner, window);
    let before = std::fs::read(&fixture.path).expect("removed config");
    click(window, "extension-storage-open");
    idle(&owner, window);
    window.update(|window, cx| window.draw(cx).clear(cx));
    assert!(window.debug_bounds("extension-storage-record-0").is_some());
    click(window, "extension-storage-clear-0");
    assert!(window.update(|window, cx| window.has_active_dialog(cx)));
    click(window, "extension-storage-cancel");
    assert!(backing.exists());
    assert_eq!(
        std::fs::read(&fixture.path).expect("cancel no write"),
        before
    );
    click(window, "extension-storage-clear-0");
    click(window, "extension-storage-confirm");
    idle(&owner, window);
    assert!(!backing.exists());
    assert!(fixture.retained().is_empty());
    assert_eq!(
        std::fs::read(&fixture.path).expect("clear never rewrites config"),
        before
    );
    service.runtime.shutdown();
}

#[gpui_kit::test]
fn expansion_storage_retained_restore_requires_fresh_confirmation_and_preserves_values(
    cx: &mut TestAppContext,
) {
    let mut fixture = Fixture::new_script();
    let old = fixture.namespace;
    let removed = AppConfig::remove_extension_data_checked(
        &fixture.path,
        "fixture.remove",
        &fixture.config.extensions,
        &fixture.owner,
        false,
    )
    .expect("retire")
    .0;
    fixture
        .service
        .commit_config(&fixture.path, |_| Ok(removed))
        .expect("publish removal");
    let preview =
        nxtools_extensions::ImportPreview::from_directory(&fixture.root.path().join("source"))
            .expect("preview");
    fixture.config = AppConfig::import_extension_preview_checked(
        &fixture.path,
        preview,
        &fixture.root.path().join("packages"),
        "owned".into(),
        None,
    )
    .expect("fresh install");
    let hash = fixture.config.extensions.packages[0].content_hash.clone();
    fixture.config = AppConfig::approve_extension(
        &fixture.path,
        "fixture.remove",
        &hash,
        nxtools_extensions::RuntimeIdentity::inspect(&fixture.root.path().join("fixture.exe"))
            .expect("identity"),
    )
    .expect("approval");
    fixture
        .service
        .commit_config(&fixture.path, |_| Ok(fixture.config.clone()))
        .expect("publish fresh target");
    let before = std::fs::read(&fixture.path).expect("before");
    let (owner, _, window, service) = fixture.mount(cx);
    click(window, "extension-storage-open");
    idle(&owner, window);
    click(window, "extension-storage-restore-0");
    click(window, "extension-storage-cancel");
    assert_eq!(std::fs::read(&fixture.path).expect("cancel"), before);
    click(window, "extension-storage-restore-0");
    click(window, "extension-storage-confirm");
    idle(&owner, window);
    let saved = AppConfig::load_from(&fixture.path).expect("restored");
    let fresh = saved.extensions.packages[0].data_uuid.expect("fresh UUID");
    assert_ne!(fresh, old);
    assert_eq!(
        owner.read_with(window, |app, _| app.packages.clone()),
        saved.extensions
    );
    let (_cancel, cancellation) = tokio::sync::watch::channel(false);
    let identity = InvocationIdentity {
        run_id: uuid::Uuid::new_v4(),
        correlation_id: uuid::Uuid::new_v4(),
        generation: 1,
        package_id: "fixture.remove".into(),
        handler_id: "open".into(),
        content_hash: hash,
    };
    let io = tokio::runtime::Runtime::new().expect("IO");
    let value = io
        .block_on(
            fixture
                .owner
                .namespace(fresh, identity, cancellation)
                .get("kept".into()),
        )
        .expect("preserved");
    assert!(value.found);
    assert_eq!(value.value, serde_json::json!("actual data"));
    assert_eq!(value.revision, 1);
    service.runtime.shutdown();
}

#[cfg(windows)]
#[gpui_kit::test]
fn expansion_storage_retained_clear_failure_requires_fresh_explicit_retry(cx: &mut TestAppContext) {
    use std::os::windows::fs::OpenOptionsExt as _;
    let fixture = Fixture::new();
    let backing = fixture.backing();
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&backing)
        .expect("delete denial");
    let (owner, view, window, service) = fixture.mount(cx);
    review(&view, window);
    click(window, "extension-remove-confirm");
    idle(&owner, window);
    let before = std::fs::read(&fixture.path).expect("removed config");
    click(window, "extension-storage-open");
    idle(&owner, window);
    click(window, "extension-storage-clear-0");
    click(window, "extension-storage-confirm");
    idle(&owner, window);
    assert!(backing.exists());
    assert!(fixture.retained()[0].cleanup_pending);
    assert!(owner.read_with(window, |app, _| app.storage_management.reviewed.is_none()));
    assert_eq!(std::fs::read(&fixture.path).expect("unchanged"), before);
    drop(held);
    window.run_until_parked();
    assert!(backing.exists(), "no automatic cleanup retry");
    click(window, "extension-storage-refresh");
    idle(&owner, window);
    click(window, "extension-storage-clear-0");
    click(window, "extension-storage-confirm");
    idle(&owner, window);
    assert!(!backing.exists());
    assert!(fixture.retained().is_empty());
    assert_eq!(
        std::fs::read(&fixture.path).expect("no config rewrite"),
        before
    );
    service.runtime.shutdown();
}

#[gpui_kit::test]
fn expansion_storage_retained_stale_confirmation_and_busy_cancel_keep_data(
    cx: &mut TestAppContext,
) {
    let fixture = Fixture::new();
    let backing = fixture.backing();
    let (owner, view, window, service) = fixture.mount(cx);
    review(&view, window);
    click(window, "extension-remove-confirm");
    idle(&owner, window);
    click(window, "extension-storage-open");
    idle(&owner, window);
    click(window, "extension-storage-clear-0");
    owner.update_in(window, |app, _, cx| {
        app.packages
            .runtime_defaults
            .insert("python".into(), fixture.root.path().join("changed.exe"));
        cx.notify();
    });
    click(window, "extension-storage-confirm");
    assert!(!owner.read_with(window, |app, _| app.busy));
    assert!(backing.exists());
    click(window, "extension-storage-cancel");
    owner.update_in(window, |app, _, cx| {
        app.packages = AppConfig::load_from(&fixture.path)
            .expect("fresh")
            .extensions;
        cx.notify();
    });
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(fixture.root.path().join("data/quota.lock"))
        .expect("actual quota lock");
    lock.lock().expect("held lock");
    click(window, "extension-storage-refresh");
    let config_lock = std::fs::OpenOptions::new()
        .write(true)
        .open(fixture.path.with_extension("json.lock"))
        .expect("Core lock");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        match config_lock.try_lock() {
            Ok(()) => {
                config_lock.unlock().expect("probe release");
            }
            Err(std::fs::TryLockError::WouldBlock) => break,
            Err(error) => panic!("Core lock probe: {error}"),
        }
        window.run_until_parked();
        assert!(
            std::time::Instant::now() < deadline,
            "actual reader must own Core lock before waiting on quota"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(owner.read_with(window, |app, _| app.busy));
    click(window, "extension-storage-cancel-operation");
    assert!(owner.read_with(window, |app, _| app.busy
        && app.storage_management.cancelling()));
    lock.unlock().expect("release");
    idle(&owner, window);
    assert!(backing.exists());
    assert!(owner.read_with(window, |app, _| app.storage_management.reviewed.is_none()));
    service.runtime.shutdown();
}
