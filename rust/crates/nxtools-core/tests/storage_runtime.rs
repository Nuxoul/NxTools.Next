//! Actual owned staged Python + original audited broker + original locked Core config.
use nxtools_core::{AppConfig, ConfigExtensionStorage};
use nxtools_extensions::{
    Causality, ExtensionRuntime, FeatureAdapter, HostContext, HostFuture, HostReply, ImportPreview,
    InstallationDataId as Uuid, InvokeRequest, RuntimeIdentity, storage::StorageOwner,
};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::watch;
struct NoFeatures;
impl FeatureAdapter for NoFeatures {
    fn describe(&self) -> Value {
        json!({})
    }
    fn call<'a>(&'a self, _: &'a HostContext, _: &'a str, _: Value) -> HostFuture<'a> {
        Box::pin(async { HostReply::error("unsupported") })
    }
}
fn python() -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|path| path.join("python.exe"))
        .find(|path| path.is_file())
        .expect("installed fixture Python")
}
const SCRIPT: &str = r#"import json,sys
init=json.loads(sys.stdin.readline())
session=init['session_id']
print(json.dumps({'protocol_version':2,'op':'ready','session_id':session}),flush=True)
f=json.loads(sys.stdin.readline())
run=f['run_id']
def emit(op,**fields):
    print(json.dumps(dict(protocol_version=2,op=op,session_id=session,run_id=run,**fields)),flush=True)
def call(call_id,method,params):
    emit('host_call',call_id=call_id,method=method,params=params)
    return json.loads(sys.stdin.readline())
phase=f['context']['phase']
if phase=='inspect':
    reply=call('describe','host.describe',{})
    methods={m['method']:m for m in reply['value']['methods']}
    assert methods['storage.set']['available'] and methods['storage.set']['minimum_transport']==2
    for method,params,code in [('storage.set',{'key':'bad'},'invalid_params'),('storage.get',{'key':'x','namespace':'forged'},'invalid_params'),('storage.list',{'limit':33},'invalid_params'),('storage.future',{},'unsupported')]:
        reply=call('invalid-'+method,method,params)
        assert not reply['success'] and reply['value']['code']==code
elif phase=='write':
    reply=call('set','storage.set',{'key':'fixture-value','value':None,'expected_revision':0})
    assert reply['success'] and reply['value']['revision']==1
elif phase=='parallel':
    for i in range(4):
        emit('host_call',call_id='parallel-'+str(i),method='storage.set',params={'key':'parallel-'+str(i),'value':i})
    replies=[json.loads(sys.stdin.readline()) for _ in range(4)]
    assert all(reply['success'] for reply in replies)
    assert sorted(reply['value']['revision'] for reply in replies)==[3,4,5,6]
else:
    reply=call('get','storage.get',{'key':'fixture-value'})
    assert reply['success'] and reply['value']['found'] and reply['value']['value'] is None and reply['value']['revision']==1
    reply=call('conflict','storage.set',{'key':'fixture-value','value':'must not replace','expected_revision':0})
    assert not reply['success'] and reply['value']['code']=='storage_conflict'
    reply=call('list','storage.list',{'limit':1})
    assert reply['success'] and reply['value']['items']==[{'key':'fixture-value','revision':1}] and not reply['value']['has_more']
    reply=call('delete','storage.delete',{'key':'fixture-value','expected_revision':1})
    assert reply['success'] and reply['value']['revision']==2
    reply=call('missing','storage.get',{'key':'fixture-value'})
    assert reply['success'] and not reply['value']['found']
emit('complete',status='success',value={'text':'storage-owned'})
emit('request_finished')
"#;
fn runtime(path: &Path, root: &Path) -> ExtensionRuntime {
    let registry = AppConfig::load_from(path)
        .expect("fresh Core config")
        .extensions;
    let owner = StorageOwner::new(root.join("data")).expect("host-owned data root");
    let runtime = ExtensionRuntime::new_with_storage(
        registry,
        PathBuf::new(),
        Arc::new(NoFeatures),
        Some(Arc::new(ConfigExtensionStorage::new(
            path.to_owned(),
            owner,
        ))),
    );
    let broker = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).expect("workspace").join("csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe");
    runtime
        .bind_owned_fixture_broker(&broker)
        .expect("original audited broker");
    runtime
}
async fn invoke(runtime: &ExtensionRuntime, phase: &str) {
    let (_cancel, cancellation) = watch::channel(false);
    let outcome = runtime
        .invoke(InvokeRequest {
            package_id: "fixture.storage".into(),
            handler_id: "run".into(),
            correlation_id: Uuid::new_v4(),
            generation: 1,
            context: json!({"phase":phase}),
            causality: Causality::root(Uuid::new_v4()),
            cancellation,
            batches: None,
        })
        .await
        .expect("actual script and native cleanup");
    assert_eq!(outcome.status, "success");
    assert_eq!(outcome.value["text"], "storage-owned");
}
#[tokio::test]
async fn expansion_storage_real_two_invocations_persist_null_cas_list_and_delete() {
    let root = tempfile::tempdir().expect("owned config/process fixture");
    let source = root.path().join("source");
    std::fs::create_dir(&source).expect("source");
    let manifest = json!({"schema_version":1,"id":"fixture.storage","name":"Storage","version":"1","runtime":{"program":"python","protocol_version":2,"timeout_ms":30000},"files":["entry.py"],"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":"entry.py"}}]});
    std::fs::write(
        source.join("extension.json"),
        serde_json::to_vec(&manifest).expect("json"),
    )
    .expect("manifest");
    std::fs::write(source.join("entry.py"), SCRIPT).expect("owned script");
    let path = root.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let preview = ImportPreview::from_directory(&source).expect("preview");
    let hash = preview.content_hash.clone();
    AppConfig::import_extension_preview_checked(
        &path,
        preview,
        &root.path().join("packages"),
        "owned".into(),
        None,
    )
    .expect("original checked import");
    AppConfig::approve_extension(
        &path,
        "fixture.storage",
        &hash,
        RuntimeIdentity::inspect(&python()).expect("actual identity"),
    )
    .expect("original approval");
    AppConfig::set_extension_enabled(&path, "fixture.storage", true).expect("separate enable");
    let before = std::fs::read(&path).expect("config before storage");
    let first = runtime(&path, root.path());
    invoke(&first, "inspect").await;
    assert_eq!(
        std::fs::read(&path).expect("discovery and invalid calls have no allocation"),
        before
    );
    invoke(&first, "write").await;
    let namespace = AppConfig::load_from(&path)
        .expect("committed Core pointer")
        .extensions
        .packages[0]
        .data_uuid
        .expect("lazy UUID");
    assert_eq!(
        first.snapshot().expect("runtime metadata").packages[0].data_uuid,
        Some(namespace)
    );
    first.shutdown();
    drop(first);
    let second = runtime(&path, root.path());
    invoke(&second, "read").await;
    invoke(&second, "parallel").await;
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("stable pointer")
            .extensions
            .packages[0]
            .data_uuid,
        Some(namespace)
    );
    second.shutdown();
}
