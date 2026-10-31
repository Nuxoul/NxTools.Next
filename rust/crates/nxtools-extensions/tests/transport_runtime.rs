#![cfg(feature = "test-support")]
use nxtools_extensions::{
    Causality, ExtensionRuntime, FeatureAdapter, HostContext, HostFuture, HostReply, ImportPreview,
    InvokeRequest, PackageRegistry, RuntimeError, RuntimeIdentity,
};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::{Barrier, Semaphore, watch};
use uuid::Uuid;

struct OwnedAdapter(Arc<AtomicUsize>);
impl FeatureAdapter for OwnedAdapter {
    fn describe(&self) -> Value {
        json!({"version":1,"fixture":"original-adapter"})
    }
    fn call<'a>(&'a self, _: &'a HostContext, method: &'a str, _: Value) -> HostFuture<'a> {
        Box::pin(async move {
            assert_eq!(method, "fixture.echo");
            self.0.fetch_add(1, Ordering::SeqCst);
            HostReply {
                success: true,
                value: json!({"echo":"owned"}),
            }
        })
    }
}
fn python() -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|path| path.join("python.exe"))
        .find(|path| path.is_file())
        .expect("installed fixture Python")
}
struct Fixture {
    runtime: ExtensionRuntime,
    _root: tempfile::TempDir,
}
impl Fixture {
    fn new(script: &str, adapter: Arc<dyn FeatureAdapter>) -> Self {
        Self::with_program(script, adapter, "python", "entry.py")
    }
    fn with_program(
        script: &str,
        adapter: Arc<dyn FeatureAdapter>,
        program: &str,
        entry: &str,
    ) -> Self {
        Self::with_settings(script, adapter, program, entry, json!([]))
    }
    fn with_settings(
        script: &str,
        adapter: Arc<dyn FeatureAdapter>,
        program: &str,
        entry: &str,
        settings: Value,
    ) -> Self {
        Self::with_timeout(script, adapter, program, entry, settings, None)
    }
    fn with_timeout(
        script: &str,
        adapter: Arc<dyn FeatureAdapter>,
        program: &str,
        entry: &str,
        settings: Value,
        timeout_ms: Option<u32>,
    ) -> Self {
        let root = tempfile::tempdir().expect("owned v2 process fixture");
        let source = root.path().join("source");
        std::fs::create_dir(&source).expect("source");
        let manifest = json!({"schema_version":1,"id":"fixture.v2","name":"V2","version":"1","runtime":{"program":program,"protocol_version":2,"timeout_ms":timeout_ms},"settings":settings,"files":[entry],"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":entry}}]});
        std::fs::write(
            source.join("extension.json"),
            serde_json::to_vec(&manifest).expect("manifest"),
        )
        .expect("write");
        std::fs::write(source.join(entry), script).expect("owned script");
        let preview = ImportPreview::from_directory(&source)
            .expect("preview")
            .stage_into(&root.path().join("packages"))
            .expect("stage");
        let hash = preview.content_hash.clone();
        let mut registry = PackageRegistry::default();
        registry
            .install(preview, "owned".into(), false)
            .expect("install");
        registry
            .approve(
                "fixture.v2",
                &hash,
                RuntimeIdentity::inspect(&if program == "python" {
                    python()
                } else {
                    std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
                        .map(|path| path.join("node.exe"))
                        .find(|path| path.is_file())
                        .expect("installed fixture Node")
                })
                .expect("identity"),
            )
            .expect("approve");
        registry
            .set_enabled("fixture.v2", true, &[])
            .expect("enable");
        let runtime = ExtensionRuntime::new(registry, PathBuf::new(), adapter);
        let broker = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).expect("workspace")
            .join("csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe");
        runtime
            .bind_owned_fixture_broker(&broker)
            .expect("original audited broker");
        Self {
            runtime,
            _root: root,
        }
    }
    fn request(context: Value, cancellation: watch::Receiver<bool>) -> InvokeRequest {
        InvokeRequest {
            package_id: "fixture.v2".into(),
            handler_id: "run".into(),
            correlation_id: Uuid::new_v4(),
            generation: 1,
            context,
            causality: Causality::root(Uuid::new_v4()),
            cancellation,
            batches: None,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.runtime.shutdown();
    }
}
const BOOT: &str = r#"import json,sys
init=json.loads(sys.stdin.readline())
assert init['protocol_version']==2 and init['op']=='initialize'
session=init['session_id']
print(json.dumps({'protocol_version':2,'op':'ready','session_id':session}),flush=True)
f=json.loads(sys.stdin.readline())
assert f['op']=='invoke' and f['session_id']==session
run=f['run_id']
def emit(op,**fields):
    print(json.dumps(dict(protocol_version=2,op=op,session_id=session,run_id=run,**fields)),flush=True)
def finish():
    emit('complete',status='success',value={'text':'v2-owned'})
    emit('request_finished')
"#;
#[tokio::test]
async fn expansion_v2_once_real_script_host_call_and_native_cleanup_keep_legacy_broker_owner() {
    let calls = Arc::new(AtomicUsize::new(0));
    let fixture = Fixture::new(
        &format!(
            "{BOOT}{}",
            r#"
emit('host_call',call_id='echo',method='fixture.echo',params={})
reply=json.loads(sys.stdin.readline())
assert reply['protocol_version']==2 and reply['session_id']==session and reply['run_id']==run
assert reply['success'] and reply['value']['echo']=='owned'
finish()
"#
        ),
        Arc::new(OwnedAdapter(calls.clone())),
    );
    let (_cancel, cancellation) = watch::channel(false);
    let result = fixture
        .runtime
        .invoke(Fixture::request(json!({}), cancellation))
        .await
        .expect("real v2 once invocation after native cleanup");
    assert_eq!(result.status, "success");
    assert_eq!(result.value["text"], "v2-owned");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn expansion_v2_describe_reports_only_implemented_transport_and_preserves_adapter_metadata() {
    let fixture = Fixture::new(
        &format!(
            "{BOOT}{}",
            r#"
emit('host_call',call_id='describe',method='host.describe',params={})
reply=json.loads(sys.stdin.readline())
assert reply['success'] and reply['value']['fixture']=='original-adapter'
t=reply['value']['transport']
assert t['active_protocol_version']==2 and t['supported_protocol_versions']==[1,2]
assert t['supported_modes']==['once'] and t['mode']=='once'
assert t['limits']['frame_bytes']==262144
assert t['limits']['input_control_bytes']==1048576 and t['limits']['output_control_bytes']==4194304
assert t['limits']['host_calls_active']==8 and t['limits']['host_calls_total']==64
assert t['progress']==True and t['blobs']==True
assert t['data_limits']['blob_bytes']==8388608 and t['data_limits']['chunk_bytes']==65536
assert t['data_limits']['chunks']==512 and t['progress_limits']['frames']==256
blob_methods=[m for m in reply['value']['methods'] if m['method'].startswith('blobs.')]
assert len(blob_methods)==4 and all(m['minimum_transport']==2 and m['available'] for m in blob_methods)
finish()
"#
        ),
        Arc::new(OwnedAdapter(Arc::new(AtomicUsize::new(0)))),
    );
    let (_cancel, cancellation) = watch::channel(false);
    fixture
        .runtime
        .invoke(Fixture::request(json!({}), cancellation))
        .await
        .expect("truthful runtime-owned transport discovery");
}
struct ParallelAdapter {
    barrier: Barrier,
    entered: AtomicUsize,
}
impl FeatureAdapter for ParallelAdapter {
    fn describe(&self) -> Value {
        json!({})
    }
    fn call<'a>(&'a self, _: &'a HostContext, method: &'a str, _: Value) -> HostFuture<'a> {
        Box::pin(async move {
            assert_eq!(method, "fixture.parallel");
            self.entered.fetch_add(1, Ordering::SeqCst);
            // No sleep: a serial/blocked frame reader cannot release this exact eight-party barrier.
            self.barrier.wait().await;
            HostReply {
                success: true,
                value: json!({"released":true}),
            }
        })
    }
}
#[tokio::test]
async fn expansion_v2_original_reader_admits_eight_waiting_calls_without_serial_deadlock() {
    let adapter = Arc::new(ParallelAdapter {
        barrier: Barrier::new(8),
        entered: AtomicUsize::new(0),
    });
    let fixture = Fixture::new(
        &format!(
            "{BOOT}{}",
            r#"
for i in range(8):
    emit('host_call',call_id='call-'+str(i),method='fixture.parallel',params={})
seen=set()
for i in range(8):
    reply=json.loads(sys.stdin.readline())
    assert reply['session_id']==session and reply['run_id']==run and reply['success']
    assert reply['value']['released'] and reply['call_id'] not in seen
    seen.add(reply['call_id'])
finish()
"#
        ),
        adapter.clone(),
    );
    let (_cancel, cancellation) = watch::channel(false);
    fixture
        .runtime
        .invoke(Fixture::request(json!({}), cancellation))
        .await
        .expect("eight live futures through one reader");
    assert_eq!(adapter.entered.load(Ordering::SeqCst), 8);
}
struct BlockingAdapter {
    entered: Semaphore,
    dropped: Semaphore,
    calls: AtomicUsize,
    blob: std::sync::Mutex<Option<nxtools_extensions::blobs::BlobRun>>,
}
struct PendingCall<'a>(&'a Semaphore);
impl Drop for PendingCall<'_> {
    fn drop(&mut self) {
        self.0.add_permits(1);
    }
}
impl FeatureAdapter for BlockingAdapter {
    fn describe(&self) -> Value {
        json!({})
    }
    fn call<'a>(&'a self, context: &'a HostContext, method: &'a str, _: Value) -> HostFuture<'a> {
        Box::pin(async move {
            assert_eq!(method, "fixture.block");
            let _pending = PendingCall(&self.dropped);
            *self.blob.lock().expect("retained current lease") = context.blobs.clone();
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.entered.add_permits(1);
            std::future::pending().await
        })
    }
}
#[tokio::test]
async fn expansion_v2_cancel_and_revoke_drop_pending_calls_before_new_owned_run() {
    for revoke in [false, true] {
        let adapter = Arc::new(BlockingAdapter {
            entered: Semaphore::new(0),
            dropped: Semaphore::new(0),
            calls: AtomicUsize::new(0),
            blob: Default::default(),
        });
        let fixture = Fixture::new(
            &format!(
                "{BOOT}{}",
                r#"
if f['context'].get('block'):
    import hashlib
    emit('host_call',call_id='begin',method='blobs.begin',params={'length':3,'media_type':'text/plain','sha256':hashlib.sha256(b'abc').hexdigest()})
    assert json.loads(sys.stdin.readline())['success']
    emit('progress',sequence=1,message='Pending fixture')
    emit('host_call',call_id='blocked',method='fixture.block',params={})
    sys.stdin.read()
else:
    finish()
"#
            ),
            adapter.clone(),
        );
        let saved = fixture.runtime.snapshot().expect("current registry");
        let (cancel, cancellation) = watch::channel(false);
        let request = Fixture::request(json!({"block":true}), cancellation);
        let correlation = request.correlation_id;
        let runtime = fixture.runtime.clone();
        let pending = tokio::spawn(async move { runtime.invoke(request).await });
        tokio::time::timeout(Duration::from_secs(5), adapter.entered.acquire())
            .await
            .expect("actual host call admitted")
            .expect("entered")
            .forget();
        if revoke {
            let mut retired = saved.clone();
            retired
                .revoke("fixture.v2")
                .expect("revoke current owned package");
            fixture
                .runtime
                .publish_registry(retired)
                .expect("publish revoke");
        } else {
            cancel.send(true).expect("cancel current run");
        }
        let result = tokio::time::timeout(Duration::from_secs(3), pending)
            .await
            .expect("bounded logical cancellation")
            .expect("invocation task");
        assert!(matches!(
            result,
            Err(RuntimeError::Cancelled | RuntimeError::Stale | RuntimeError::Untrusted)
        ));
        tokio::time::timeout(Duration::from_secs(3), adapter.dropped.acquire())
            .await
            .expect("pending host future actually dropped")
            .expect("drop")
            .forget();
        assert!(fixture.runtime.progress(correlation).is_none());
        let retained = adapter
            .blob
            .lock()
            .expect("lease")
            .clone()
            .expect("blob context");
        assert!(
            matches!(
                retained.begin(1, "text/plain".into(), "a".repeat(64)).await,
                Err(nxtools_extensions::blobs::BlobError::Stale)
            ),
            "cancel/revoke retires all retained blob access"
        );
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1, "no effect replay");
        if revoke {
            fixture
                .runtime
                .publish_registry(saved)
                .expect("restore explicit fixture trust for a new request");
        }
        let (_cancel, cancellation) = watch::channel(false);
        fixture
            .runtime
            .invoke(Fixture::request(json!({}), cancellation))
            .await
            .expect("new invocation waits original native cleanup slots");
        assert_eq!(
            adapter.calls.load(Ordering::SeqCst),
            1,
            "new request never replays old host call"
        );
    }
}
#[tokio::test]
async fn expansion_v2_real_wrong_session_duplicate_ready_missing_finish_and_handshake_timeout_fail()
{
    let cases = [
        ("import json,sys\nf=json.loads(sys.stdin.readline())\nprint(json.dumps(dict(protocol_version=2,op='ready',session_id='11111111-1111-4111-8111-111111111111')),flush=True)\nsys.stdin.read()\n".to_string(), RuntimeError::Protocol),
        (format!("{BOOT}\nprint(json.dumps(dict(protocol_version=2,op='ready',session_id=session)),flush=True)\nsys.stdin.read()\n"), RuntimeError::Protocol),
        (format!("{BOOT}\nemit('complete',status='success',value={{}})\n"), RuntimeError::Protocol),
        ("import sys\nsys.stdin.readline()\nsys.stdin.read()\n".to_string(), RuntimeError::Timeout),
    ];
    for (script, expected) in cases {
        let fixture = Fixture::new(
            &script,
            Arc::new(OwnedAdapter(Arc::new(AtomicUsize::new(0)))),
        );
        let (_cancel, cancellation) = watch::channel(false);
        assert!(
            matches!(fixture.runtime.invoke(Fixture::request(json!({}), cancellation)).await, Err(actual) if actual == expected)
        );
    }
}

async fn sdk_process(program_filter: &str) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace");
    for (program, entry, sdk, tail) in [
        (
            "python",
            "entry.py",
            "nxtools.py",
            "\nsession=Session(protocol_version=2)\nassert session.call('fixture.echo',{})['echo']=='owned'\nsession.progress(1,2,'SDK fixture')\npayload=b'x'*(1048576+19)\nblob=session.blob_upload(payload)\nassert session.blob_read(blob)==payload\nsession.blob_release(blob)\nsession.complete('success',{'text':'sdk-owned'})\n",
        ),
        (
            "node",
            "entry.mjs",
            "nxtools.mjs",
            "\nconst session=await Session.start(process.stdin,process.stdout,{protocolVersion:2});\nif((await session.call('fixture.echo')).echo!=='owned') throw new Error('fixture');\nawait session.progress({completed:1,total:2,message:'SDK fixture'});\nconst payload=Buffer.alloc(1048576+19,120);\nconst blob=await session.blobUpload(payload);\nif(!(await session.blobRead(blob)).equals(payload)) throw new Error('bytes');\nawait session.blobRelease(blob);\nawait session.complete('success',{text:'sdk-owned'});\n",
        ),
    ] {
        if program != program_filter {
            continue;
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let code = std::fs::read_to_string(workspace.join("docs/extensions/sdk").join(sdk))
            .expect("actual SDK source")
            + tail;
        let fixture =
            Fixture::with_program(&code, Arc::new(OwnedAdapter(calls.clone())), program, entry);
        let (_cancel, cancellation) = watch::channel(false);
        let outcome = fixture
            .runtime
            .invoke(Fixture::request(json!({}), cancellation))
            .await
            .unwrap_or_else(|failure| {
                panic!("actual {program} SDK completion and native cleanup: {failure:?}; admitted_calls={}", calls.load(Ordering::SeqCst))
            });
        assert_eq!(outcome.value["text"], "sdk-owned");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn expansion_v2_python_sdk_runs_through_original_native_owner() {
    sdk_process("python").await;
}
#[tokio::test]
async fn expansion_v2_node_sdk_runs_through_original_native_owner() {
    sdk_process("node").await;
}

#[tokio::test]
async fn expansion_settings_real_script_receives_valid_missing_default_without_config_write() {
    let fixture = Fixture::with_settings(
        &format!("{BOOT}\nassert f['settings']['name']=='default-ready'\nfinish()\n"),
        Arc::new(OwnedAdapter(Arc::new(AtomicUsize::new(0)))),
        "python",
        "entry.py",
        json!([{"key":"name","title":"Name","type":"text","default":"default-ready","validation":{"required":true,"min_length":2}}]),
    );
    let mut saved = fixture.runtime.snapshot().expect("registry");
    saved.packages[0].settings.clear();
    fixture
        .runtime
        .publish_registry(saved.clone())
        .expect("valid absent default snapshot");
    let (_cancel, cancellation) = watch::channel(false);
    fixture
        .runtime
        .invoke(Fixture::request(json!({}), cancellation))
        .await
        .expect("original real process receives effective defaults");
    assert_eq!(
        fixture.runtime.snapshot().expect("unchanged registry"),
        saved
    );
}

#[tokio::test]
async fn expansion_v2_blob_real_roundtrip_above_legacy_input_limit_preserves_exact_bytes() {
    struct NoEffects;
    impl FeatureAdapter for NoEffects {
        fn describe(&self) -> Value {
            json!({})
        }
        fn call<'a>(&'a self, _: &'a HostContext, _: &'a str, _: Value) -> HostFuture<'a> {
            Box::pin(async { HostReply::error("unsupported") })
        }
    }
    let fixture = Fixture::new(
        &format!(
            "{BOOT}{}",
            r#"
import base64,hashlib
payload=bytes((i%251 for i in range(1048576+19)))
digest=hashlib.sha256(payload).hexdigest()
emit('host_call',call_id='begin',method='blobs.begin',params={'length':len(payload),'media_type':'application/octet-stream','sha256':digest})
reply=json.loads(sys.stdin.readline())
assert reply['success']
blob=reply['value']['blob_id']
transfer=reply['value']['transfer_id']
for sequence,offset in enumerate(range(0,len(payload),65536),1):
    chunk=payload[offset:offset+65536]
    emit('data',transfer_id=transfer,sequence=sequence,offset=offset,data=base64.b64encode(chunk).decode('ascii'))
    ack=json.loads(sys.stdin.readline())
    assert ack['op']=='data_ack' and ack['transfer_id']==transfer and ack['sequence']==sequence and ack['offset']==offset+len(chunk)
emit('host_call',call_id='seal',method='blobs.seal',params={'blob_id':blob})
reply=json.loads(sys.stdin.readline())
assert reply['success'] and reply['value']['sha256']==digest
emit('host_call',call_id='read',method='blobs.read',params={'blob_id':blob})
reply=json.loads(sys.stdin.readline())
assert reply['success'] and reply['value']['length']==len(payload)
transfer=reply['value']['transfer_id']
received=bytearray()
for sequence in range(1,18):
    part=json.loads(sys.stdin.readline())
    assert part['op']=='data' and part['transfer_id']==transfer and part['sequence']==sequence and part['offset']==len(received)
    chunk=base64.b64decode(part['data'],validate=True)
    assert len(chunk)<=65536
    received.extend(chunk)
    emit('data_ack',transfer_id=transfer,sequence=sequence,offset=len(received))
assert bytes(received)==payload and hashlib.sha256(received).hexdigest()==digest
emit('host_call',call_id='release',method='blobs.release',params={'blob_id':blob})
assert json.loads(sys.stdin.readline())['success']
finish()
"#
        ),
        Arc::new(NoEffects),
    );
    let (_cancel, cancellation) = watch::channel(false);
    fixture
        .runtime
        .invoke(Fixture::request(json!({}), cancellation))
        .await
        .expect("exact >1MiB upload/download under original broker cleanup");
}

#[tokio::test]
async fn expansion_v2_two_eight_mib_blobs_roundtrip_exact_512_chunks_with_native_cleanup() {
    let fixture = Fixture::new(
        &format!(
            "{BOOT}{}",
            r#"
import base64,hashlib
payload=b'B'*(8*1024*1024)
digest=hashlib.sha256(payload).hexdigest()
blobs=[]
for n in range(2):
    emit('host_call',call_id='begin'+str(n),method='blobs.begin',params={'length':len(payload),'media_type':'application/octet-stream','sha256':digest})
    reply=json.loads(sys.stdin.readline())
    assert reply['success']
    blob=reply['value']['blob_id']; transfer=reply['value']['transfer_id']
    blobs.append(blob)
    for sequence,offset in enumerate(range(0,len(payload),65536),1):
        emit('data',transfer_id=transfer,sequence=sequence,offset=offset,data=base64.b64encode(payload[offset:offset+65536]).decode('ascii'))
        ack=json.loads(sys.stdin.readline())
        assert ack['op']=='data_ack' and ack['offset']==offset+65536
    emit('host_call',call_id='seal'+str(n),method='blobs.seal',params={'blob_id':blob})
    assert json.loads(sys.stdin.readline())['success']
emit('host_call',call_id='overquota',method='blobs.begin',params={'length':1,'media_type':'text/plain','sha256':hashlib.sha256(b'x').hexdigest()})
reply=json.loads(sys.stdin.readline())
assert not reply['success'] and reply['value']['code']=='resource_limit'
for n,blob in enumerate(blobs):
    emit('host_call',call_id='read'+str(n),method='blobs.read',params={'blob_id':blob})
    reply=json.loads(sys.stdin.readline()); assert reply['success']
    transfer=reply['value']['transfer_id']; received=bytearray()
    for sequence in range(1,129):
        part=json.loads(sys.stdin.readline())
        assert part['op']=='data' and part['sequence']==sequence and part['offset']==len(received)
        received.extend(base64.b64decode(part['data'],validate=True))
        emit('data_ack',transfer_id=transfer,sequence=sequence,offset=len(received))
    assert bytes(received)==payload and hashlib.sha256(received).hexdigest()==digest
    emit('host_call',call_id='release'+str(n),method='blobs.release',params={'blob_id':blob})
    assert json.loads(sys.stdin.readline())['success']
finish()
"#
        ),
        Arc::new(OwnedAdapter(Arc::new(AtomicUsize::new(0)))),
    );
    let (_cancel, cancellation) = watch::channel(false);
    fixture
        .runtime
        .invoke(Fixture::request(json!({}), cancellation))
        .await
        .expect("16MiB each direction/512chunks original native once cleanup");
}

#[tokio::test]
async fn expansion_progress_real_script_coalesces_latest_and_terminal_retires_without_success_inference()
 {
    let fixture = Fixture::new(
        &format!(
            "{BOOT}{}",
            r#"
import time
for sequence in range(1,257):
    emit('progress',sequence=sequence,completed=sequence,total=256,message='Plugin reported')
time.sleep(1)
emit('complete',status='error',value={'text':'still failed'})
emit('request_finished')
"#
        ),
        Arc::new(OwnedAdapter(Arc::new(AtomicUsize::new(0)))),
    );
    let mut changed = fixture.runtime.subscribe_progress();
    let (_cancel, cancellation) = watch::channel(false);
    let request = Fixture::request(json!({}), cancellation);
    let correlation = request.correlation_id;
    let runtime = fixture.runtime.clone();
    let mut task = tokio::spawn(async move { runtime.invoke(request).await });
    let mut updates = 0;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            tokio::select! {
                changed = changed.changed() => changed.expect("progress subscription"),
                outcome = &mut task => panic!("run terminated before latest progress: {}", match outcome.expect("task") { Ok(result)=>result.status, Err(failure)=>failure.to_string() }),
            }
            if let Some(progress) = fixture.runtime.progress(correlation) {
                updates += 1;
                assert_eq!(progress.identity.correlation_id, correlation);
                if progress.sequence == 256 {
                    assert_eq!(progress.percentage(), Some(100.0));
                    break;
                }
            }
        }
    })
    .await
    .expect("last flood value not lost");
    assert!(
        updates <= 4,
        "bounded latest-value publication, not256notifications"
    );
    assert!(!task.is_finished(), "100percent never completes invocation");
    let outcome = task
        .await
        .expect("task")
        .expect("valid error completion after cleanup");
    assert_eq!(outcome.status, "error");
    assert!(
        fixture.runtime.progress(correlation).is_none(),
        "terminal removes UI-only text"
    );
}

#[tokio::test]
async fn expansion_blob_real_invalid_chunks_and_stalled_ack_fail_without_reusing_transfer() {
    let prefix = r#"
import hashlib,base64,time
emit('host_call',call_id='begin',method='blobs.begin',params={'length':3,'media_type':'text/plain','sha256':hashlib.sha256(b'abc').hexdigest()})
reply=json.loads(sys.stdin.readline()); assert reply['success']
blob=reply['value']['blob_id']; transfer=reply['value']['transfer_id']
"#;
    for malformed in [
        "emit('data',transfer_id=transfer,sequence=1,offset=1,data='YWJj')",
        "emit('data',transfer_id=transfer,sequence=2,offset=0,data='YWJj')",
        "emit('data',transfer_id=transfer,sequence=1,offset=0,data='bad%%')",
        "emit('data',transfer_id='11111111-1111-4111-8111-111111111111',sequence=1,offset=0,data='YWJj')",
        "emit('data',transfer_id=transfer,sequence=1,offset=0,data=base64.b64encode(b'x'*65537).decode())",
    ] {
        let fixture = Fixture::new(
            &format!("{BOOT}{prefix}\n{malformed}\n"),
            Arc::new(OwnedAdapter(Arc::new(AtomicUsize::new(0)))),
        );
        let (_cancel, cancellation) = watch::channel(false);
        assert!(
            fixture
                .runtime
                .invoke(Fixture::request(json!({}), cancellation))
                .await
                .is_err(),
            "malformed data must fail current run"
        );
    }
    let fixture = Fixture::new(
        &format!(
            "{BOOT}{prefix}{}",
            r#"
emit('data',transfer_id=transfer,sequence=1,offset=0,data='YWJj')
assert json.loads(sys.stdin.readline())['op']=='data_ack'
emit('host_call',call_id='seal',method='blobs.seal',params={'blob_id':blob}); assert json.loads(sys.stdin.readline())['success']
emit('host_call',call_id='read',method='blobs.read',params={'blob_id':blob}); assert json.loads(sys.stdin.readline())['success']
assert json.loads(sys.stdin.readline())['op']=='data'
time.sleep(10)
finish()
"#
        ),
        Arc::new(OwnedAdapter(Arc::new(AtomicUsize::new(0)))),
    );
    let (_cancel, cancellation) = watch::channel(false);
    let started = std::time::Instant::now();
    assert!(matches!(
        fixture
            .runtime
            .invoke(Fixture::request(json!({}), cancellation))
            .await,
        Err(RuntimeError::Timeout)
    ));
    assert!(
        started.elapsed() < Duration::from_secs(9),
        "ACK deadline, not30second invocation timeout"
    );
}

#[tokio::test]
async fn expansion_blob_real_release_incomplete_and_zero_length_can_complete() {
    for length in [0, 3] {
        let fixture = Fixture::new(
            &format!(
                "{BOOT}{}",
                r#"
import hashlib
length=f['context']['length']
emit('host_call',call_id='bad',method='blobs.begin',params={'length':length,'media_type':'text/plain','sha256':hashlib.sha256(b'').hexdigest(),'unknown':True})
reply=json.loads(sys.stdin.readline()); assert not reply['success'] and reply['value']['code']=='invalid_params'
emit('host_call',call_id='begin',method='blobs.begin',params={'length':length,'media_type':'text/plain','sha256':hashlib.sha256(b'').hexdigest()})
reply=json.loads(sys.stdin.readline()); assert reply['success']; blob=reply['value']['blob_id']
if length==0:
    emit('host_call',call_id='seal',method='blobs.seal',params={'blob_id':blob}); assert json.loads(sys.stdin.readline())['success']
    emit('host_call',call_id='read',method='blobs.read',params={'blob_id':blob}); reply=json.loads(sys.stdin.readline()); assert reply['success'] and reply['value']['length']==0
emit('host_call',call_id='release',method='blobs.release',params={'blob_id':blob}); assert json.loads(sys.stdin.readline())['success']
emit('host_call',call_id='late',method='blobs.seal',params={'blob_id':blob}); assert not json.loads(sys.stdin.readline())['success']
finish()
"#
            ),
            Arc::new(OwnedAdapter(Arc::new(AtomicUsize::new(0)))),
        );
        let (_cancel, cancellation) = watch::channel(false);
        fixture
            .runtime
            .invoke(Fixture::request(json!({"length":length}), cancellation))
            .await
            .expect("explicit released upload permits quiescence");
    }
}

#[tokio::test]
async fn expansion_progress_real_reports_never_extend_original_deadline() {
    let fixture = Fixture::with_timeout(
        &format!(
            "{BOOT}{}",
            r#"
import time
for sequence in range(1,100):
    emit('progress',sequence=sequence,message='Still running')
    time.sleep(0.1)
finish()
"#
        ),
        Arc::new(OwnedAdapter(Arc::new(AtomicUsize::new(0)))),
        "python",
        "entry.py",
        json!([]),
        Some(1000),
    );
    let (_cancel, cancellation) = watch::channel(false);
    let request = Fixture::request(json!({}), cancellation);
    let correlation = request.correlation_id;
    let started = std::time::Instant::now();
    assert!(matches!(
        fixture.runtime.invoke(request).await,
        Err(RuntimeError::Timeout)
    ));
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "progress never refreshes original deadline"
    );
    assert!(fixture.runtime.progress(correlation).is_none());
}

#[tokio::test]
async fn expansion_blob_ack_drains_original_reader_while_eight_host_effects_wait() {
    struct Adapter(Barrier);
    impl FeatureAdapter for Adapter {
        fn describe(&self) -> Value {
            json!({})
        }
        fn call<'a>(
            &'a self,
            context: &'a HostContext,
            method: &'a str,
            _: Value,
        ) -> HostFuture<'a> {
            Box::pin(async move {
                assert_eq!(method, "fixture.await_blob");
                self.0.wait().await;
                let run = context
                    .blobs
                    .as_ref()
                    .expect("shared invocation blob owner");
                tokio::time::timeout(Duration::from_secs(3), async {
                    while !run.quiescent() {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .expect("ACK must drain while all8host effects are pending");
                HostReply {
                    success: true,
                    value: json!({}),
                }
            })
        }
    }
    let adapter = Arc::new(Adapter(Barrier::new(8)));
    let fixture = Fixture::new(
        &format!(
            "{BOOT}{}",
            r#"
import hashlib
emit('host_call',call_id='begin',method='blobs.begin',params={'length':3,'media_type':'text/plain','sha256':hashlib.sha256(b'abc').hexdigest()})
reply=json.loads(sys.stdin.readline()); assert reply['success']; transfer=reply['value']['transfer_id']; blob=reply['value']['blob_id']
emit('data',transfer_id=transfer,sequence=1,offset=0,data='YWJj')
assert json.loads(sys.stdin.readline())['op']=='data_ack'
emit('host_call',call_id='seal',method='blobs.seal',params={'blob_id':blob}); assert json.loads(sys.stdin.readline())['success']
emit('host_call',call_id='read',method='blobs.read',params={'blob_id':blob}); reply=json.loads(sys.stdin.readline()); assert reply['success']; download=reply['value']['transfer_id']
part=json.loads(sys.stdin.readline()); assert part['op']=='data'
for n in range(8): emit('host_call',call_id='wait'+str(n),method='fixture.await_blob',params={})
emit('data_ack',transfer_id=download,sequence=1,offset=3)
for n in range(8): assert json.loads(sys.stdin.readline())['success']
finish()
"#
        ),
        adapter.clone(),
    );
    let (_cancel, cancellation) = watch::channel(false);
    let runtime = fixture.runtime.clone();
    let task = tokio::spawn(async move {
        runtime
            .invoke(Fixture::request(json!({}), cancellation))
            .await
    });
    tokio::time::timeout(Duration::from_secs(8), task)
        .await
        .expect("one reader drains ACK independently")
        .expect("task")
        .expect("native cleanup");
}

#[tokio::test]
async fn expansion_storage_without_host_is_unavailable_and_never_delegated_to_adapter() {
    struct NoStorage;
    impl FeatureAdapter for NoStorage {
        fn describe(&self) -> Value {
            json!({})
        }
        fn call<'a>(&'a self, _: &'a HostContext, _: &'a str, _: Value) -> HostFuture<'a> {
            Box::pin(async {
                panic!("runtime storage methods must not escape to a feature adapter")
            })
        }
    }
    let fixture = Fixture::new(
        &format!(
            "{BOOT}{}",
            r#"
def call(call_id,method,params):
    emit('host_call',call_id=call_id,method=method,params=params)
    return json.loads(sys.stdin.readline())
reply=call('describe','host.describe',{})
methods={m['method']:m for m in reply['value']['methods']}
assert not methods['storage.set']['available'] and methods['storage.set']['unavailable_code']=='capability_unavailable'
for method,params in [('storage.set',{'key':'fixture-value','value':None}),('storage.get',{'key':'fixture-value'}),('storage.delete',{'key':'fixture-value'}),('storage.list',{})]:
    reply=call(method,method,params)
    assert not reply['success'] and reply['value']['code']=='capability_unavailable'
finish()
"#
        ),
        Arc::new(NoStorage),
    );
    let (_cancel, cancellation) = watch::channel(false);
    fixture
        .runtime
        .invoke(Fixture::request(json!({}), cancellation))
        .await
        .expect("unavailable capability remains a normal host error after native cleanup");
    assert!(
        fixture.runtime.snapshot().expect("snapshot").packages[0]
            .data_uuid
            .is_none()
    );
}
