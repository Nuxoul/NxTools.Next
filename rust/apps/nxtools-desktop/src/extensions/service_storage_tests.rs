//! Real Core/config/owned-process composition, without product/native UI launch.
use super::*;
use nxtools_core::AppConfig;
use nxtools_extensions::{Causality, ImportPreview, RuntimeIdentity};
use serde_json::json;
use std::path::Path;

const SCRIPT: &str = r#"import json,sys
init=json.loads(sys.stdin.readline()); s=init['session_id']
print(json.dumps(dict(protocol_version=2,op='ready',session_id=s)),flush=True)
f=json.loads(sys.stdin.readline()); r=f['run_id']
def emit(op,**fields): print(json.dumps(dict(protocol_version=2,op=op,session_id=s,run_id=r,**fields)),flush=True)
def call(method,params):
    emit('host_call',call_id=method,method=method,params=params)
    return json.loads(sys.stdin.readline())
if f['context']['write']:
    reply=call('storage.set',{'key':'owned','value':None,'expected_revision':0})
    assert reply['success'] and reply['value']['revision']==1
else:
    reply=call('storage.get',{'key':'owned'})
    assert reply['success'] and reply['value']['found'] and reply['value']['value'] is None
emit('complete',status='success',value={})
emit('request_finished')
"#;

fn config(root: &Path) -> (PathBuf, AppConfig) {
    config_source(root, SCRIPT, false)
}
fn config_source(root: &Path, script: &str, resolver: bool) -> (PathBuf, AppConfig) {
    let source = root.join("source");
    std::fs::create_dir(&source).expect("source");
    let handler = if resolver {
        json!({"id":"run","title":"Resolve","feature":"downloads","entry":"resolve","url_matches":[{"host":"original.test","path":"*"}],"action":{"type":"script","entry_file":"entry.py"}})
    } else {
        json!({"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":"entry.py"}})
    };
    let manifest = json!({"schema_version":1,"id":"fixture.storage","name":"Storage","version":"1","runtime":{"program":"python","protocol_version":2,"timeout_ms":30000},"files":["entry.py"],"handlers":[handler]});
    std::fs::write(
        source.join("extension.json"),
        serde_json::to_vec(&manifest).expect("json"),
    )
    .expect("manifest");
    std::fs::write(source.join("entry.py"), script).expect("owned script");
    let path = root.join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let preview = ImportPreview::from_directory(&source).expect("preview");
    let hash = preview.content_hash.clone();
    AppConfig::import_extension_preview_checked(
        &path,
        preview,
        &root.join("packages"),
        "owned".into(),
        None,
    )
    .expect("original import");
    let python = std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|directory| directory.join("python.exe"))
        .find(|path| path.is_file())
        .expect("fixture Python");
    AppConfig::approve_extension(
        &path,
        "fixture.storage",
        &hash,
        RuntimeIdentity::inspect(&python).expect("identity"),
    )
    .expect("approval");
    let config =
        AppConfig::set_extension_enabled(&path, "fixture.storage", true).expect("separate enable");
    (path, config)
}
fn bind_broker(service: &ExtensionService) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).expect("workspace")
        .join("csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe");
    let executor = service.executor();
    let _entered = executor.enter();
    service
        .runtime
        .bind_owned_fixture_broker(&fixture)
        .expect("audited fixture broker");
}
fn invoke(service: &ExtensionService, write: bool) {
    let (_cancel, cancellation) = tokio::sync::watch::channel(false);
    let task = service.invoke(InvokeRequest {
        package_id: "fixture.storage".into(),
        handler_id: "run".into(),
        correlation_id: uuid::Uuid::new_v4(),
        generation: 1,
        context: json!({"write":write}),
        causality: Causality::root(uuid::Uuid::new_v4()),
        cancellation,
        batches: None,
    });
    let outcome = service
        .executor()
        .block_on(task)
        .expect("join")
        .expect("real process and cleanup");
    assert_eq!(outcome.status, "success");
}

#[test]
fn expansion_storage_gui_and_headless_service_bind_persist_and_reject_delayed_registry() {
    for ui in [false, true] {
        let root = tempfile::tempdir().expect("owned service fixture");
        let (path, initial) = config(root.path());
        let engine = DownloadEngine::open(root.path().join("downloads.db")).expect("engine");
        let data = root.path().join("data/extensions-data");
        let before = std::fs::read(&path).expect("before");
        let (service, receiver) = ExtensionService::new_with_storage(
            &engine,
            initial.extensions.clone(),
            None,
            ui,
            path.clone(),
            data.clone(),
        )
        .expect("configured production composition");
        assert_eq!(receiver.is_some(), ui);
        service
            .storage
            .as_ref()
            .expect("configured owner")
            .ensure_reconciled()
            .expect("startup recovery");
        assert_eq!(std::fs::read(&path).expect("no rewrite"), before);
        assert!(
            service.runtime.snapshot().expect("snapshot").packages[0]
                .data_uuid
                .is_none()
        );
        let mut changed = service
            .subscribe_storage_metadata()
            .expect("bounded metadata signal");
        bind_broker(&service);
        invoke(&service, true);
        assert!(changed.has_changed().expect("signal alive"));
        changed.borrow_and_update();
        let bound = AppConfig::load_from(&path).expect("committed metadata");
        assert!(bound.extensions.packages[0].data_uuid.is_some());
        assert_eq!(service.runtime.snapshot().expect("live"), bound.extensions);
        assert!(
            service.publish_registry(initial.extensions).is_err(),
            "a delayed ordinary snapshot must not revoke a durable binding"
        );
        assert_eq!(
            service.runtime.snapshot().expect("retained live"),
            bound.extensions
        );
        let (saved, applied) = service
            .commit_config(&path, |path| {
                AppConfig::save_extension_settings(path, "fixture.storage", Default::default())
            })
            .expect("original locked settings commit + serialized publication");
        assert!(applied);
        assert_eq!(
            saved.extensions.packages[0].data_uuid,
            bound.extensions.packages[0].data_uuid
        );
        invoke(&service, false);
        service.runtime.shutdown();
        drop(service);
        let (reopened, _) =
            ExtensionService::new_with_storage(&engine, saved.extensions, None, ui, path, data)
                .expect("fresh configured service");
        bind_broker(&reopened);
        invoke(&reopened, false);
        reopened.runtime.shutdown();
    }
}

#[test]
fn expansion_storage_metadata_refresh_is_exact_and_does_not_import_other_changes() {
    let root = tempfile::tempdir().expect("metadata fixture");
    let (_, config) = config(root.path());
    let mut cached = config.extensions.clone();
    let mut bound = cached.clone();
    bound.packages[0].data_uuid = Some(uuid::Uuid::new_v4());
    assert!(refresh_storage_metadata(&mut cached, &bound));
    assert_eq!(cached, bound);
    assert!(!refresh_storage_metadata(&mut cached, &config.extensions));
    let mut changed = bound.clone();
    changed.packages[0].enabled = false;
    changed.packages[0].data_uuid = Some(uuid::Uuid::new_v4());
    assert!(!refresh_storage_metadata(&mut cached, &changed));
    assert_eq!(
        cached, bound,
        "storage metadata observation cannot overwrite settings/trust/enable or rebind a UUID"
    );
}

#[test]
fn expansion_storage_binding_and_original_commit_share_the_full_publication_boundary() {
    use nxtools_extensions::storage::StorageHost;
    let root = tempfile::tempdir().expect("publication fixture");
    let (path, config) = config(root.path());
    let engine = DownloadEngine::open(root.path().join("downloads.db")).expect("engine");
    let (service, _) = ExtensionService::new_with_storage(
        &engine,
        config.extensions.clone(),
        None,
        false,
        path.clone(),
        root.path().join("data"),
    )
    .expect("service");
    let host = service.storage.clone().expect("storage");
    let reviewed = config.extensions.packages[0].clone();
    let (committed, committed_rx) = std::sync::mpsc::channel();
    let (release, release_rx) = std::sync::mpsc::channel();
    let binding = std::thread::spawn(move || {
        let release_rx = Mutex::new(release_rx);
        host.bind_and_apply(&reviewed, &|| false, &|bound| {
            committed.send(bound.clone()).expect("durable barrier");
            release_rx
                .lock()
                .expect("release lock")
                .recv()
                .expect("release");
            Ok(())
        })
    });
    let bound = committed_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("Core pointer committed before apply");
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("disk")
            .extensions
            .packages[0],
        bound
    );
    let committing = service.clone();
    let (entered, entered_rx) = std::sync::mpsc::channel();
    let (attempted, attempted_rx) = std::sync::mpsc::channel();
    let commit = std::thread::spawn(move || {
        attempted.send(()).expect("management worker started");
        committing.commit_config(&path, |path| {
            entered.send(()).expect("entered");
            AppConfig::save_extension_settings(path, "fixture.storage", Default::default())
        })
    });
    attempted_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("management attempted original boundary");
    assert!(
        entered_rx
            .recv_timeout(std::time::Duration::from_millis(100))
            .is_err(),
        "management cannot publish between the durable bind and exact runtime apply"
    );
    release.send(()).expect("release actual binding");
    assert_eq!(binding.join().expect("worker").expect("binding"), bound);
    let (saved, applied) = commit.join().expect("commit worker").expect("commit");
    assert!(applied);
    assert_eq!(saved.extensions.packages[0], bound);
    assert_eq!(
        service.runtime.snapshot().expect("runtime"),
        saved.extensions
    );
    service.runtime.shutdown();
}

#[gpui_kit::test]
fn expansion_storage_real_manual_call_refreshes_manager_after_lease_and_keeps_terminal(
    cx: &mut gpui_kit::TestAppContext,
) {
    use crate::extensions::ExtensionApplication;
    use gpui_kit::{AppContext, WeakEntity};
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let root = tempfile::tempdir().expect("owned manual storage");
    let (path, config) = config(root.path());
    let engine = DownloadEngine::open(root.path().join("downloads.db")).expect("engine");
    let (service, _) = ExtensionService::new_with_storage(
        &engine,
        config.extensions.clone(),
        None,
        false,
        path.clone(),
        root.path().join("data"),
    )
    .expect("configured service");
    bind_broker(&service);
    let owner = cx.new(|_| {
        let mut app =
            ExtensionApplication::new(config, path.clone(), None, WeakEntity::new_invalid());
        app.service = Some(service.clone());
        app
    });
    let id = owner.update(cx, |app, cx| {
        app.observe_storage_metadata(cx);
        let id = app
            .start_manual(
                "fixture.storage".into(),
                "run".into(),
                "tools",
                json!({"write":true}),
                cx,
            )
            .expect("actual manual request");
        assert!(
            app.packages.packages[0].data_uuid.is_none(),
            "observer cannot reenter the current leased entity"
        );
        id
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while owner.read_with(cx, |app, _| {
        app.has_pending_manual() || app.packages.packages[0].data_uuid.is_none()
    }) {
        cx.run_until_parked();
        assert!(
            std::time::Instant::now() < deadline,
            "actual binding, deferred metadata and native terminal"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    owner.read_with(cx, |app, _| {
        assert_eq!(
            app.packages,
            AppConfig::load_from(&path)
                .expect("committed config")
                .extensions
        );
        assert_eq!(
            app.manual_run(id).expect("manual terminal").state,
            crate::extensions::manual_runs::ManualState::Succeeded
        );
    });
    service.runtime.shutdown();
    let released = service.storage_release_probe_for_test();
    owner.update(cx, |app, _| app.unbind_service_for_test());
    drop(service);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !released() {
        cx.run_until_parked();
        assert!(
            std::time::Instant::now() < deadline,
            "manual fixture actual service cleanup"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn expansion_storage_real_resolver_binding_keeps_accepted_tasks_and_next_route_usable() {
    let root = tempfile::tempdir().expect("owned resolver storage");
    let script = r#"import json,sys
i=json.loads(sys.stdin.readline()); s=i['session_id']
print(json.dumps(dict(protocol_version=2,op='ready',session_id=s)),flush=True)
f=json.loads(sys.stdin.readline()); r=f['run_id']
def emit(op,**fields): print(json.dumps(dict(protocol_version=2,op=op,session_id=s,run_id=r,**fields)),flush=True)
emit('host_call',call_id='get',method='storage.get',params={'key':'owned'})
a=json.loads(sys.stdin.readline()); assert a['success']
if not a['value']['found']:
    emit('host_call',call_id='set',method='storage.set',params={'key':'owned','value':None,'expected_revision':0})
    a=json.loads(sys.stdin.readline()); assert a['success']
emit('complete',status='handled',value={'url':'https://effective.test/owned'})
emit('request_finished')
"#;
    let (path, config) = config_source(root.path(), script, true);
    let engine = DownloadEngine::open(root.path().join("downloads.db")).expect("engine");
    let (service, _) = ExtensionService::new_with_storage(
        &engine,
        config.extensions.clone(),
        None,
        false,
        path.clone(),
        root.path().join("data"),
    )
    .expect("configured resolver");
    bind_broker(&service);
    let identity_before =
        ExtensionDownloadResolver::new(config.extensions, Arc::new(service.clone()));
    let request: nxtools_download::DownloadRequest = serde_json::from_value(json!({
        "source_url":"https://original.test/page", "save_directory":root.path(),
        "file_name":"first.bin", "save_directory_explicit":true,
    }))
    .expect("synthetic queued request");
    let submit = |request| {
        engine.submit_with_resolution(
            request,
            nxtools_download::RequestOrigin::Ui,
            nxtools_download::DownloadSubmitMode::QueueOnly,
            None,
            nxtools_download::DownloadResolutionOptions::default(),
            nxtools_download::DownloadResolutionControl::default(),
        )
    };
    let (first, _) = submit(request.clone()).expect("first real resolver uses storage");
    let bound = AppConfig::load_from(&path).expect("durable binding");
    assert!(bound.extensions.packages[0].data_uuid.is_some());
    let identity_after =
        ExtensionDownloadResolver::new(bound.extensions, Arc::new(service.clone()));
    use nxtools_download::DownloadRequestResolver;
    assert_eq!(
        identity_before.identity(),
        identity_after.identity(),
        "metadata must not invalidate ordinary resolver receipts"
    );
    let mut next = request;
    next.file_name = Some("second.bin".into());
    let (second, _) = submit(next).expect("same resolver remains usable after lazy binding");
    assert_ne!(first, second);
    for id in [first, second] {
        assert_eq!(
            engine
                .task(id)
                .expect("task")
                .expect("retained")
                .task
                .request()
                .source_url,
            "https://effective.test/owned"
        );
    }
    assert_eq!(engine.tasks().expect("queued only").len(), 2);
    service.runtime.shutdown();
}

#[test]
fn expansion_storage_startup_reconciles_absent_data_but_corrupt_config_retains_it() {
    use nxtools_extensions::storage::StorageOwner;
    let root = tempfile::tempdir().expect("startup recovery fixture");
    let path = root.path().join("config.json");
    let data = root.path().join("data");
    let owner = StorageOwner::new(data.clone()).expect("seed owner");
    let namespace = uuid::Uuid::new_v4();
    owner
        .reserve_namespace_blocking(namespace, "fixture.removed")
        .expect("durable orphan reservation");
    std::fs::write(&path, b"invalid config").expect("corrupt configuration");
    let before = std::fs::read(data.join("catalog.json")).expect("last good catalog");
    let engine = DownloadEngine::open(root.path().join("downloads.db")).expect("engine");
    let (service, _) = ExtensionService::new_with_storage(
        &engine,
        PackageRegistry::default(),
        None,
        false,
        path.clone(),
        data.clone(),
    )
    .expect("composition has no caller-thread config IO");
    assert!(
        service
            .storage
            .as_ref()
            .expect("storage")
            .ensure_reconciled()
            .is_err()
    );
    assert_eq!(
        std::fs::read(data.join("catalog.json")).expect("retained catalog"),
        before
    );
    assert!(
        owner
            .retained_metadata_blocking("fixture.removed")
            .expect("not speculatively retired")
            .is_empty()
    );
    AppConfig::default()
        .save_to(&path)
        .expect("explicit fixture repair");
    let config_bytes = std::fs::read(&path).expect("config");
    service
        .storage
        .as_ref()
        .expect("storage")
        .ensure_reconciled()
        .expect("fresh locked reconciliation");
    let retained = owner
        .retained_metadata_blocking("fixture.removed")
        .expect("retained generation");
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0].namespace, namespace);
    assert!(!retained[0].cleanup_pending);
    assert_eq!(
        std::fs::read(path).expect("startup never rewrites config"),
        config_bytes
    );
    service.runtime.shutdown();
}

#[test]
fn expansion_storage_service_drop_releases_real_owner_after_executor_grace() {
    for _ in 0..6 {
        let root = tempfile::tempdir().expect("owned lifecycle");
        let path = root.path().join("config.json");
        AppConfig::default().save_to(&path).expect("config");
        let engine = DownloadEngine::open(root.path().join("downloads.db")).expect("engine");
        let (service, _) = ExtensionService::new_with_storage(
            &engine,
            PackageRegistry::default(),
            None,
            false,
            path,
            root.path().join("data"),
        )
        .expect("bounded service");
        let storage = Arc::downgrade(service.storage.as_ref().expect("real storage host"));
        drop(service);
        drop(engine);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while storage.upgrade().is_some() {
            assert!(
                std::time::Instant::now() < deadline,
                "retired executor must release its actual storage owner"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}
