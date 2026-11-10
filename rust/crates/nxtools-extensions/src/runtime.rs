use crate::{
    BrokerClient, BrokerError, BrokerSecretStore, BrokerStart, Causality, DeclarativeAction,
    InstalledPackage, InvocationIdentity, InvocationNotice, PackageRegistry, ScriptProgram,
    SecretStore,
};
use serde_json::Value;
use std::{
    collections::HashMap,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, watch};
use uuid::Uuid;

#[path = "runtime_storage.rs"]
mod storage_binding;
use storage_binding::RunPackage;

pub type HostFuture<'a> = Pin<Box<dyn Future<Output = HostReply> + Send + 'a>>;
/// Existing feature owners implement this boundary; runtime never owns their DB/window state.
pub trait FeatureAdapter: Send + Sync {
    fn describe(&self) -> Value;
    fn describe_for(&self, _context: &HostContext) -> Value {
        self.describe()
    }
    fn call<'a>(
        &'a self,
        context: &'a HostContext,
        method: &'a str,
        params: Value,
    ) -> HostFuture<'a>;
}
#[derive(Clone)]
pub struct HostContext {
    pub identity: InvocationIdentity,
    pub feature: String,
    pub entry: String,
    pub causality: Causality,
    pub cancellation: watch::Receiver<bool>,
    pub blobs: Option<crate::blobs::BlobRun>,
}
pub struct HostReply {
    pub success: bool,
    pub value: Value,
}
impl HostReply {
    pub fn error(code: &'static str) -> Self {
        Self {
            success: false,
            value: serde_json::json!({"code":code,"retryable":matches!(code,"busy"|"ui_queue_full"|"download_unavailable"|"clipboard_busy"|"disconnected"),"message":match code {"invalid_params"=>"Parameters do not match the method contract", "invalid_cursor"|"cursor_stale"=>"Restart the bounded query without a cursor", "resolution_required"=>"Explicit resolution is required before submitting", "ui_unavailable"=>"This operation needs an available desktop feature owner", "native_failed"=>"The Windows operation did not complete successfully", "unsupported"=>"This host does not implement the requested method", _=>"The host operation did not complete; inspect the stable error code"}}),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RuntimeError {
    #[error("extension package is not enabled and trusted")]
    Untrusted,
    #[error("extension handler unavailable")]
    HandlerUnavailable,
    #[error("extension invocation queue full")]
    QueueFull,
    #[error("extension invocation cancelled")]
    Cancelled,
    #[error("extension invocation timed out")]
    Timeout,
    #[error("extension native runtime unavailable")]
    NativeUnavailable,
    #[error("extension protocol failed")]
    Protocol,
    #[error("extension result is stale")]
    Stale,
    #[error("extension handler reported failure")]
    HandlerFailed,
    #[error("extension transport is not available in this runtime")]
    UnsupportedTransport,
    #[error("extension settings are not ready")]
    InvalidSettings,
}
pub struct InvokeRequest {
    pub package_id: String,
    pub handler_id: String,
    pub correlation_id: Uuid,
    pub generation: u64,
    pub context: Value,
    pub causality: Causality,
    pub cancellation: watch::Receiver<bool>,
    pub batches: Option<mpsc::Sender<(InvocationIdentity, InvocationNotice)>>,
}
pub struct InvokeOutcome {
    pub identity: InvocationIdentity,
    pub status: String,
    pub value: Value,
}

struct ActiveRun {
    package: String,
    cancel: watch::Sender<bool>,
    snapshot: Option<RunPackage>,
}
struct Inner {
    registry: RwLock<PackageRegistry>,
    broker_path: PathBuf,
    broker: Mutex<Option<BrokerClient>>,
    adapter: Arc<dyn FeatureAdapter>,
    pending: Arc<Semaphore>,
    stopped: AtomicBool,
    #[cfg(any(test, feature = "test-support"))]
    fixture_broker: Mutex<Option<PathBuf>>,
    verification: Arc<Semaphore>,
    global: Arc<Semaphore>,
    plugins: Mutex<HashMap<String, Arc<Semaphore>>>,
    active: Mutex<HashMap<Uuid, ActiveRun>>,
    blobs: crate::blobs::BlobStore,
    progress: crate::progress::ProgressHub,
    storage: Option<Arc<dyn crate::storage::StorageHost>>,
    storage_binding_slots: Arc<Semaphore>,
}
#[derive(Clone)]
pub struct ExtensionRuntime(Arc<Inner>);

impl ExtensionRuntime {
    pub fn new(
        registry: PackageRegistry,
        broker_path: PathBuf,
        adapter: Arc<dyn FeatureAdapter>,
    ) -> Self {
        Self::new_with_storage(registry, broker_path, adapter, None)
    }
    pub fn new_with_storage(
        registry: PackageRegistry,
        broker_path: PathBuf,
        adapter: Arc<dyn FeatureAdapter>,
        storage: Option<Arc<dyn crate::storage::StorageHost>>,
    ) -> Self {
        Self(Arc::new(Inner {
            registry: RwLock::new(registry),
            broker_path,
            broker: Mutex::new(None),
            adapter,
            pending: Arc::new(Semaphore::new(32)),
            stopped: AtomicBool::new(false),
            #[cfg(any(test, feature = "test-support"))]
            fixture_broker: Mutex::new(None),
            global: Arc::new(Semaphore::new(4)),
            verification: Arc::new(Semaphore::new(4)),
            plugins: Mutex::new(HashMap::new()),
            active: Mutex::new(HashMap::new()),
            blobs: Default::default(),
            progress: Default::default(),
            storage,
            storage_binding_slots: Arc::new(Semaphore::new(4)),
        }))
    }
    #[cfg(any(test, feature = "test-support"))]
    pub fn bind_owned_fixture_broker(&self, fixture: &std::path::Path) -> Result<(), RuntimeError> {
        let parent = std::env::current_exe().map_err(|_| RuntimeError::NativeUnavailable)?;
        let broker = BrokerClient::launch_inner(
            fixture,
            &[
                "--extension-fixture-broker".into(),
                std::process::id().to_string(),
                parent.to_string_lossy().into_owned(),
            ],
        )
        .map_err(|_| RuntimeError::NativeUnavailable)?;
        *self
            .0
            .fixture_broker
            .lock()
            .map_err(|_| RuntimeError::NativeUnavailable)? = Some(fixture.to_owned());
        *self
            .0
            .broker
            .lock()
            .map_err(|_| RuntimeError::NativeUnavailable)? = Some(broker);
        Ok(())
    }
    pub fn subscribe_progress(&self) -> watch::Receiver<u64> {
        self.0.progress.subscribe()
    }
    pub fn progress(&self, correlation_id: Uuid) -> Option<crate::InvocationProgress> {
        let progress = self.0.progress.get(correlation_id)?;
        let active = self
            .0
            .active
            .lock()
            .ok()?
            .get(&progress.identity.run_id)
            .is_some_and(|run| !*run.cancel.borrow());
        if !active {
            return None;
        }
        let current = self.snapshot().ok()?.packages.into_iter().any(|package| {
            package.enabled
                && package.trust.is_some()
                && package.manifest.id == progress.identity.package_id
                && package.content_hash == progress.identity.content_hash
        });
        current.then_some(progress)
    }
    pub fn snapshot(&self) -> Result<PackageRegistry, RuntimeError> {
        self.0
            .registry
            .read()
            .map(|registry| registry.clone())
            .map_err(|_| RuntimeError::NativeUnavailable)
    }
    pub fn publish_registry(&self, registry: PackageRegistry) -> Result<(), RuntimeError> {
        registry.validate().map_err(|_| RuntimeError::Untrusted)?;
        let mut current = self
            .0
            .registry
            .write()
            .map_err(|_| RuntimeError::NativeUnavailable)?;
        let active = self
            .0
            .active
            .lock()
            .map_err(|_| RuntimeError::NativeUnavailable)?;
        for run in active.values() {
            let old = current
                .packages
                .iter()
                .find(|package| package.manifest.id == run.package);
            let new = registry
                .packages
                .iter()
                .find(|package| package.manifest.id == run.package);
            let old_runtime = current.runtime_executable(&run.package);
            let new_runtime = registry.runtime_executable(&run.package);
            let selection_changed = match (&old_runtime, &new_runtime) {
                (Ok(old), Ok(new)) => {
                    !crate::registry::same_runtime_selection(old.as_deref(), new.as_deref())
                }
                _ => old_runtime != new_runtime,
            };
            if old != new || selection_changed {
                let _ = run.cancel.send(true);
            }
        }
        *current = registry;
        Ok(())
    }
    pub fn cancel_package(&self, id: &str) {
        if let Ok(active) = self.0.active.lock() {
            for run in active.values().filter(|run| run.package == id) {
                let _ = run.cancel.send(true);
            }
        }
    }
    pub fn shutdown(&self) {
        self.0.stopped.store(true, Ordering::Release);
        self.0.blobs.close();
        if let Ok(active) = self.0.active.lock() {
            for run in active.values() {
                let _ = run.cancel.send(true);
            }
        }
        if let Ok(broker) = self.0.broker.lock()
            && let Some(broker) = broker.as_ref()
        {
            broker.shutdown();
        }
    }
    pub async fn shutdown_and_wait(&self) -> Result<(), crate::blobs::BlobError> {
        self.shutdown();
        self.0.blobs.shutdown().await
    }
    pub fn secret_store(&self) -> Result<BrokerSecretStore, RuntimeError> {
        Ok(BrokerSecretStore(self.broker()?))
    }
    fn broker(&self) -> Result<BrokerClient, RuntimeError> {
        if self.0.stopped.load(Ordering::Acquire) {
            return Err(RuntimeError::NativeUnavailable);
        }
        let mut broker = self
            .0
            .broker
            .lock()
            .map_err(|_| RuntimeError::NativeUnavailable)?;
        if let Some(client) = broker.as_ref() {
            if client.is_available() {
                return Ok(client.clone());
            }
            if !client.can_replace() {
                return Err(RuntimeError::NativeUnavailable);
            }
        }
        #[cfg(any(test, feature = "test-support"))]
        let fixture = self
            .0
            .fixture_broker
            .lock()
            .map_err(|_| RuntimeError::NativeUnavailable)?
            .clone();
        #[cfg(not(any(test, feature = "test-support")))]
        let fixture: Option<PathBuf> = None;
        let client = if let Some(fixture) = fixture {
            let parent = std::env::current_exe().map_err(|_| RuntimeError::NativeUnavailable)?;
            BrokerClient::launch_inner(
                &fixture,
                &[
                    "--extension-fixture-broker".into(),
                    std::process::id().to_string(),
                    parent.to_string_lossy().into_owned(),
                ],
            )
        } else {
            BrokerClient::launch(&self.0.broker_path)
        }
        .map_err(|_| RuntimeError::NativeUnavailable)?;
        *broker = Some(client.clone());
        Ok(client)
    }
    fn package(&self, request: &InvokeRequest) -> Result<InstalledPackage, RuntimeError> {
        self.snapshot()?
            .packages
            .into_iter()
            .find(|package| {
                package.manifest.id == request.package_id
                    && package.enabled
                    && package.trust.is_some()
            })
            .ok_or(RuntimeError::Untrusted)
    }
    fn registered_current(&self, package: &InstalledPackage) -> Result<(), RuntimeError> {
        let snapshot = self.snapshot()?;
        if snapshot
            .packages
            .iter()
            .find(|current| current.manifest.id == package.manifest.id)
            != Some(package)
            || !package.enabled
        {
            return Err(RuntimeError::Stale);
        }
        let selected = snapshot
            .runtime_executable(&package.manifest.id)
            .map_err(|_| RuntimeError::Untrusted)?;
        if let Some(selected) = selected
            && package.trust.as_ref().is_none_or(|trust| {
                !crate::registry::same_runtime_selection(
                    Some(&selected),
                    Some(&trust.runtime.executable),
                )
            })
        {
            return Err(RuntimeError::Stale);
        }
        Ok(())
    }
    #[cfg(test)]
    async fn still_current(&self, package: &InstalledPackage) -> Result<(), RuntimeError> {
        self.still_run(&RunPackage::new(package.clone())).await
    }
    async fn execution_slots(
        &self,
        plugin: Arc<Semaphore>,
    ) -> Result<[OwnedSemaphorePermit; 2], RuntimeError> {
        // Running capacity and pending capacity are distinct. A blocked package
        // cannot consume the three otherwise unused global slots as queue space.
        if let Ok(plugin_slot) = plugin.clone().try_acquire_owned()
            && let Ok(global_slot) = self.0.global.clone().try_acquire_owned()
        {
            return Ok([plugin_slot, global_slot]);
        }
        let queued = self
            .0
            .pending
            .clone()
            .try_acquire_owned()
            .map_err(|_| RuntimeError::QueueFull)?;
        let plugin_slot = plugin
            .acquire_owned()
            .await
            .map_err(|_| RuntimeError::NativeUnavailable)?;
        let global_slot = self
            .0
            .global
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| RuntimeError::NativeUnavailable)?;
        drop(queued);
        Ok([plugin_slot, global_slot])
    }

    fn describe_for(&self, context: &HostContext) -> Value {
        let mut value = self.0.adapter.describe_for(context);
        let Some(object) = value.as_object_mut() else {
            return value;
        };
        let methods = object
            .entry("methods")
            .or_insert_with(|| Value::Array(Vec::new()));
        if let Some(methods) = methods.as_array_mut() {
            for method in crate::method_catalog(false, false, false)
                .as_array()
                .into_iter()
                .flatten()
                .filter(|method| {
                    method["method"].as_str().is_some_and(|name| {
                        name.starts_with("blobs.") || name.starts_with("storage.")
                    })
                })
            {
                if !methods
                    .iter()
                    .any(|existing| existing["method"] == method["method"])
                {
                    methods.push(method.clone());
                }
            }
            for method in methods {
                if method["minimum_transport"]
                    .as_u64()
                    .is_some_and(|version| version > 1)
                    && context.blobs.is_none()
                {
                    method["available"] = Value::Bool(false);
                    method["unavailable_code"] = Value::String("unsupported_transport".into());
                } else if method["method"]
                    .as_str()
                    .is_some_and(|name| name.starts_with("storage."))
                {
                    method["available"] = Value::Bool(self.0.storage.is_some());
                    method["unavailable_code"] = if self.0.storage.is_some() {
                        Value::Null
                    } else {
                        Value::String("capability_unavailable".into())
                    };
                }
            }
        }
        value
    }
    async fn dispatch_host(
        &self,
        run: &RunPackage,
        context: &HostContext,
        method: &str,
        params: Value,
    ) -> Result<crate::blob_calls::Completion, RuntimeError> {
        self.still_run(run).await?;
        let package = run.snapshot()?;
        if *context.cancellation.borrow() {
            return Err(RuntimeError::Cancelled);
        }
        let started = Instant::now();
        tracing::info!(event="extension.host_call.accepted",correlation_id=%context.identity.correlation_id,run_id=%context.identity.run_id,"extension host operation admitted");
        if method.starts_with("storage.") {
            let reply = self.storage_call(run, context, method, params).await;
            self.still_run(run).await?;
            tracing::info!(event="extension.host_call.terminal",correlation_id=%context.identity.correlation_id,run_id=%context.identity.run_id,success=reply.success,elapsed_ms=started.elapsed().as_millis(),"extension host operation terminated");
            return Ok(reply.into());
        }
        if method.starts_with("blobs.") {
            let result = crate::blob_calls::call(context, method, params).await;
            self.still_run(run).await?;
            tracing::info!(event="extension.host_call.terminal",correlation_id=%context.identity.correlation_id,run_id=%context.identity.run_id,success=result.reply.success,elapsed_ms=started.elapsed().as_millis(),"extension host operation terminated");
            return Ok(result);
        }
        let reply = if method == "host.describe" {
            let mut value = self.describe_for(context);
            let object = value.as_object_mut().ok_or(RuntimeError::Protocol)?;
            object.insert(
                "transport".into(),
                crate::session::transport_description(
                    package
                        .manifest
                        .runtime
                        .as_ref()
                        .map_or(1, |runtime| runtime.protocol_version),
                ),
            );
            HostReply {
                success: true,
                value,
            }
        } else if method == "credentials.get" {
            let key = params.get("key").and_then(Value::as_str).filter(|key| {
                package.manifest.settings.iter().any(|setting| {
                    setting.key == *key && matches!(setting.field, crate::SettingType::Password)
                })
            });
            if let Some(key) = key {
                match self.secret_store()?.get(&package.manifest.id, key).await {
                    Ok(value) => HostReply {
                        success: true,
                        value: serde_json::json!({"value":value}),
                    },
                    Err(_) => HostReply::error("credential_failed"),
                }
            } else {
                HostReply::error("invalid_setting")
            }
        } else if context.feature == "downloads"
            && context.entry == "resolve"
            && matches!(
                method,
                "downloads.submit" | "downloads.probe" | "downloads.enqueue"
            )
        {
            HostReply::error("resolver_reentry")
        } else {
            self.0.adapter.call(context, method, params).await
        };
        tracing::info!(event="extension.host_call.terminal",correlation_id=%context.identity.correlation_id,run_id=%context.identity.run_id,success=reply.success,elapsed_ms=started.elapsed().as_millis(),"extension host operation terminated");
        self.still_run(run).await?;
        Ok(reply.into())
    }

    pub async fn invoke(&self, mut request: InvokeRequest) -> Result<InvokeOutcome, RuntimeError> {
        let accepted = Instant::now();
        let package = self.package(&request)?;
        self.registered_current(&package)?;
        let handler = package
            .manifest
            .handlers
            .iter()
            .find(|handler| handler.id == request.handler_id)
            .cloned()
            .ok_or(RuntimeError::HandlerUnavailable)?;
        if handler
            .accepts
            .as_ref()
            .is_some_and(|accepts| !accepts.accepts_context(&handler.feature, &request.context))
        {
            return Err(RuntimeError::HandlerUnavailable);
        }
        let DeclarativeAction::Script { entry_file } = &handler.action else {
            return Err(RuntimeError::HandlerUnavailable);
        };
        let runtime = package
            .manifest
            .runtime
            .as_ref()
            .ok_or(RuntimeError::HandlerUnavailable)?;
        if !matches!(runtime.protocol_version, 1 | 2) || runtime.mode != crate::RuntimeMode::Once {
            return Err(RuntimeError::UnsupportedTransport);
        }
        let timeout = Duration::from_millis(u64::from(
            runtime.effective_timeout_ms(&handler.feature, &handler.entry),
        ));
        let deadline = tokio::time::Instant::now() + timeout;
        let run_id = Uuid::new_v4();
        let identity = InvocationIdentity {
            run_id,
            correlation_id: request.correlation_id,
            generation: request.generation,
            package_id: package.manifest.id.clone(),
            handler_id: handler.id.clone(),
            content_hash: package.content_hash.clone(),
        };
        let run_package = RunPackage::new(package.clone());
        let (cancel, mut local_cancel) = watch::channel(false);
        self.register_run(run_id, run_package.clone(), cancel)?;
        let guard = ActiveGuard {
            runtime: self.clone(),
            run_id,
        };
        let semaphore = {
            let mut plugins = self
                .0
                .plugins
                .lock()
                .map_err(|_| RuntimeError::NativeUnavailable)?;
            if !plugins.contains_key(&package.manifest.id) && plugins.len() >= 256 {
                return Err(RuntimeError::QueueFull);
            }
            plugins
                .entry(package.manifest.id.clone())
                .or_insert_with(|| Arc::new(Semaphore::new(1)))
                .clone()
        };
        tracing::info!(event="extension.run.queued", correlation_id=%identity.correlation_id, run_id=%run_id, generation=identity.generation, "extension invocation queued");
        let execution_cancel = local_cancel.clone();
        let caller_cancel = request.cancellation.clone();
        let execution = async {
            if *caller_cancel.borrow() || *execution_cancel.borrow() {
                return Err(RuntimeError::Cancelled);
            }
            if !package.manifest.settings_ready(&package.settings) {
                return Err(RuntimeError::InvalidSettings);
            }
            let slots = self.execution_slots(semaphore).await?;
            // The exact host-issued data binding may have updated this queued run.
            self.still_run(&run_package).await?;
            let causality = request
                .causality
                .enter(&package.manifest.id, &handler.id)
                .map_err(|_| RuntimeError::HandlerUnavailable)?;
            let v2 = runtime.protocol_version == 2;
            let mut invocation =
                crate::session::WireInvocation::new(identity.clone(), runtime.protocol_version);
            let blobs = invocation
                .session_id()
                .map(|session| self.0.blobs.invocation(identity.clone(), session));
            let _blob_guard = BlobGuard(blobs.clone());
            let context = HostContext {
                identity: identity.clone(),
                feature: handler.feature.clone(),
                entry: handler.entry.clone(),
                causality: causality.clone(),
                cancellation: execution_cancel.clone(),
                blobs: blobs.clone(),
            };
            if !package.manifest.requires.is_empty() {
                let description = self.describe_for(&context);
                let methods = description
                    .get("methods")
                    .and_then(Value::as_array)
                    .ok_or(RuntimeError::HandlerUnavailable)?;
                if package.manifest.requires.iter().any(|required| {
                    !methods.iter().any(|method| {
                        method["method"] == required.method
                            && method["version"] == required.version
                            && method["available"] == true
                    })
                }) {
                    return Err(RuntimeError::HandlerUnavailable);
                }
            }
            let settings =
                serde_json::to_value(package.manifest.effective_settings(&package.settings))
                    .map_err(|_| RuntimeError::Protocol)?;
            let causal_value =
                serde_json::to_value(causality).map_err(|_| RuntimeError::Protocol)?;
            let input = invocation
                .initial_frame(
                    &handler.feature,
                    request.context.clone(),
                    settings.clone(),
                    causal_value.clone(),
                )
                .map_err(|_| RuntimeError::Protocol)?;
            let launch = prepare_launch(
                &package,
                entry_file,
                input,
                &identity,
                deadline.saturating_duration_since(tokio::time::Instant::now()),
            )?;
            let mut native = self
                .broker()?
                .start_with_slots(identity.clone(), launch, Some(slots))
                .map_err(map_broker)?;
            let mut complete = None;
            let mut stderr_bytes = 0usize;
            let mut handshake = v2.then(|| tokio::time::Instant::now() + Duration::from_secs(5));
            let mut calls = tokio::task::JoinSet::new();
            let mut data_jobs = tokio::task::JoinSet::new();
            if v2 {
                tracing::info!(event="extension.session.transition",correlation_id=%identity.correlation_id,run_id=%run_id,stage="initializing","extension once session initialized");
            }
            loop {
                self.0.progress.flush(run_id);
                if blobs.as_ref().is_some_and(|run| run.ack_expired()) {
                    return Err(RuntimeError::Timeout);
                }
                let event = tokio::select! {
                    _ = async { if let Some(deadline)=handshake { tokio::time::sleep_until(deadline).await } else { std::future::pending::<()>().await } } => return Err(RuntimeError::Timeout),
                    _ = tokio::time::sleep(Duration::from_millis(100)), if blobs.is_some() => continue,
                    result = data_jobs.join_next(), if !data_jobs.is_empty() => {
                        self.registered_run(&run_package)?;
                        match result.ok_or(RuntimeError::Protocol)?.map_err(|_|RuntimeError::Protocol)?? {
                            BlobIo::Uploaded { transfer, sequence, offset } => {
                                native.write(invocation.data_ack_frame(transfer,sequence,offset).map_err(|_|RuntimeError::Protocol)?).map_err(map_broker)?;
                            },
                            BlobIo::Downloaded(Some(chunk)) => {
                                native.write(invocation.data_frame(&chunk).map_err(|_|RuntimeError::Protocol)?).map_err(map_broker)?;
                            },
                            BlobIo::Downloaded(None) => {},
                        }
                        continue;
                    },
                    reply = calls.join_next(), if !calls.is_empty() => {
                        let (call_id, completion): (String, crate::blob_calls::Completion) = reply.ok_or(RuntimeError::Protocol)?.map_err(|_|RuntimeError::Protocol)??;
                        self.still_run(&run_package).await?;
                        let download = completion.transfer.map(|transfer| invocation.register_transfer(transfer)).transpose().map_err(|_|RuntimeError::Protocol)?.flatten();
                        let reply = completion.reply;
                        native.write(invocation.host_result(&call_id,reply.success,reply.value,&identity).map_err(|_|RuntimeError::Protocol)?).map_err(map_broker)?;
                        if let Some(transfer) = download {
                            download_chunk(&mut data_jobs, blobs.clone().ok_or(RuntimeError::Protocol)?, transfer)?;
                        }
                        continue;
                    },
                    event = native.next() => event.map_err(map_broker)?,
                };
                match event.op.as_str() {
                    "started" => {}
                    "stderr" => {
                        stderr_bytes += event.data.as_ref().map_or(0, String::len);
                        if stderr_bytes > 64 * 1024 {
                            return Err(RuntimeError::Protocol);
                        }
                    }
                    "stdout" => {
                        self.registered_run(&run_package)?;
                        let notice = invocation
                            .accept(
                                event
                                    .data
                                    .as_deref()
                                    .ok_or(RuntimeError::Protocol)?
                                    .as_bytes(),
                                &identity,
                            )
                            .map_err(|_| RuntimeError::Protocol)?;
                        match notice {
                            crate::SessionNotice::Ready => {
                                self.still_run(&run_package).await?;
                                handshake = None;
                                tracing::info!(event="extension.session.transition",correlation_id=%identity.correlation_id,run_id=%run_id,stage="ready",elapsed_ms=accepted.elapsed().as_millis(),"extension once session ready");
                                let input = invocation
                                    .invoke_after_ready(
                                        &handler.feature,
                                        request.context.clone(),
                                        settings.clone(),
                                        causal_value.clone(),
                                    )
                                    .map_err(|_| RuntimeError::Protocol)?;
                                native.write(input).map_err(map_broker)?;
                            }
                            crate::SessionNotice::Progress(frame) => {
                                self.0
                                    .progress
                                    .report(&identity, frame)
                                    .map_err(|_| RuntimeError::Protocol)?;
                            }
                            crate::SessionNotice::RequestFinished => {
                                tracing::info!(event="extension.session.transition",correlation_id=%identity.correlation_id,run_id=%run_id,stage="request_finished",elapsed_ms=accepted.elapsed().as_millis(),"extension once request quiesced; native cleanup still required");
                            }
                            crate::SessionNotice::Data(frame) => {
                                if data_jobs.len() >= 4 {
                                    return Err(RuntimeError::QueueFull);
                                }
                                let run = blobs.clone().ok_or(RuntimeError::Protocol)?;
                                data_jobs.spawn(async move {
                                    let offset = run
                                        .upload(
                                            frame.transfer_id,
                                            frame.sequence,
                                            frame.offset,
                                            frame.data,
                                        )
                                        .await
                                        .map_err(|_| RuntimeError::Protocol)?;
                                    Ok(BlobIo::Uploaded {
                                        transfer: frame.transfer_id,
                                        sequence: frame.sequence,
                                        offset,
                                    })
                                });
                            }
                            crate::SessionNotice::DataAck(frame) => {
                                let run = blobs.clone().ok_or(RuntimeError::Protocol)?;
                                if run
                                    .acknowledge(frame.transfer_id, frame.sequence, frame.offset)
                                    .map_err(|_| RuntimeError::Protocol)?
                                {
                                    download_chunk(&mut data_jobs, run, frame.transfer_id)?;
                                }
                            }
                            crate::SessionNotice::Invocation(notice) => match notice {
                                InvocationNotice::HostCall {
                                    call_id,
                                    method,
                                    params,
                                } => {
                                    if v2 {
                                        if calls.len() >= 8 {
                                            return Err(RuntimeError::QueueFull);
                                        }
                                        let runtime = self.clone();
                                        let package = run_package.clone();
                                        let context = context.clone();
                                        calls.spawn(async move {
                                            runtime
                                                .dispatch_host(&package, &context, &method, params)
                                                .await
                                                .map(|reply| (call_id, reply))
                                        });
                                        continue;
                                    }
                                    let reply = self
                                        .dispatch_host(&run_package, &context, &method, params)
                                        .await?
                                        .reply;
                                    self.still_run(&run_package).await?;
                                    native
                                        .write(
                                            invocation
                                                .host_result(
                                                    &call_id,
                                                    reply.success,
                                                    reply.value,
                                                    &identity,
                                                )
                                                .map_err(|_| RuntimeError::Protocol)?,
                                        )
                                        .map_err(map_broker)?;
                                }
                                InvocationNotice::ResultBatch { .. } => {
                                    self.still_run(&run_package).await?;
                                    if let Some(batches) = request.batches.as_ref() {
                                        batches
                                            .try_send((identity.clone(), notice))
                                            .map_err(|_| RuntimeError::QueueFull)?;
                                    }
                                }
                                InvocationNotice::Completed { status, value } => {
                                    if !data_jobs.is_empty()
                                        || blobs.as_ref().is_some_and(|run| !run.quiescent())
                                    {
                                        return Err(RuntimeError::Protocol);
                                    }
                                    self.still_run(&run_package).await?;
                                    complete = Some((status, value));
                                }
                            },
                        }
                    }
                    "exited" => {
                        invocation
                            .process_exit(event.success, event.cleanup_complete)
                            .map_err(|_| RuntimeError::Protocol)?;
                        self.still_run(&run_package).await?;
                        let (status, value) = complete.ok_or(RuntimeError::Protocol)?;
                        return Ok(InvokeOutcome {
                            identity: identity.clone(),
                            status,
                            value,
                        });
                    }
                    _ => return Err(RuntimeError::NativeUnavailable),
                }
            }
        };
        let outcome = tokio::select! {
            _ = request.cancellation.changed() => Err(RuntimeError::Cancelled),
            _ = local_cancel.changed() => Err(RuntimeError::Cancelled),
            _ = tokio::time::sleep_until(deadline) => Err(RuntimeError::Timeout),
            outcome = execution => outcome,
        };
        drop(guard);
        let success = outcome
            .as_ref()
            .is_ok_and(|result| result.status != "error");
        tracing::info!(event="extension.run.terminal", correlation_id=%identity.correlation_id, run_id=%run_id, success, elapsed_ms=accepted.elapsed().as_millis(), "extension invocation terminated");
        outcome
    }
}
struct BlobGuard(Option<crate::blobs::BlobRun>);
impl Drop for BlobGuard {
    fn drop(&mut self) {
        if let Some(run) = &self.0 {
            run.retire();
        }
    }
}
enum BlobIo {
    Uploaded {
        transfer: Uuid,
        sequence: u64,
        offset: usize,
    },
    Downloaded(Option<crate::blobs::BlobChunk>),
}
fn download_chunk(
    jobs: &mut tokio::task::JoinSet<Result<BlobIo, RuntimeError>>,
    run: crate::blobs::BlobRun,
    transfer: Uuid,
) -> Result<(), RuntimeError> {
    if jobs.len() >= 4 {
        return Err(RuntimeError::QueueFull);
    }
    jobs.spawn(async move {
        run.next_chunk(transfer)
            .await
            .map(BlobIo::Downloaded)
            .map_err(|_| RuntimeError::Protocol)
    });
    Ok(())
}
struct ActiveGuard {
    runtime: ExtensionRuntime,
    run_id: Uuid,
}
impl Drop for ActiveGuard {
    fn drop(&mut self) {
        self.runtime.0.progress.retire(self.run_id);
        if let Ok(mut active) = self.runtime.0.active.lock()
            && let Some(run) = active.remove(&self.run_id)
        {
            let _ = run.cancel.send(true);
        }
    }
}
fn map_broker(error: BrokerError) -> RuntimeError {
    match error {
        BrokerError::QueueFull => RuntimeError::QueueFull,
        _ => RuntimeError::NativeUnavailable,
    }
}
fn prepare_launch(
    package: &InstalledPackage,
    entry_file: &str,
    input: String,
    identity: &InvocationIdentity,
    remaining: Duration,
) -> Result<BrokerStart, RuntimeError> {
    let runtime = package
        .manifest
        .runtime
        .as_ref()
        .ok_or(RuntimeError::HandlerUnavailable)?;
    let approval = package.trust.as_ref().ok_or(RuntimeError::Untrusted)?;
    let directory = package
        .code_directory
        .as_ref()
        .ok_or(RuntimeError::Untrusted)?;
    let entry = directory.join(entry_file.replace('\\', "/"));
    let mut arguments = match runtime.program {
        ScriptProgram::Python => vec!["-u".into()],
        ScriptProgram::Powershell | ScriptProgram::Pwsh => vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-File".into(),
        ],
        ScriptProgram::Node | ScriptProgram::Exe => Vec::new(),
    };
    if runtime.program != ScriptProgram::Exe {
        arguments.push(local_windows_path(&entry)?);
    }
    arguments.extend(runtime.arguments.clone());
    Ok(BrokerStart {
        version: 1,
        op: "start".into(),
        run_id: identity.run_id,
        plugin_id: identity.package_id.clone(),
        executable: local_windows_path(&approval.runtime.executable)?,
        arguments,
        working_directory: local_windows_path(directory)?,
        timeout_ms: remaining.as_millis().clamp(1, 120_000) as u32,
        memory_bytes: 512 * 1024 * 1024,
        max_processes: 8,
        stdin: input,
        data_plane: (runtime.protocol_version == 2).then_some(2),
    })
}
fn local_windows_path(path: &std::path::Path) -> Result<String, RuntimeError> {
    let text = path.to_str().ok_or(RuntimeError::Untrusted)?;
    let text = text.strip_prefix("\\\\?\\").unwrap_or(text);
    if text.starts_with("UNC\\") || !path.is_absolute() {
        return Err(RuntimeError::Untrusted);
    }
    Ok(text.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Adapter;
    impl FeatureAdapter for Adapter {
        fn describe(&self) -> Value {
            serde_json::json!({"version":1})
        }
        fn call<'a>(
            &'a self,
            _context: &'a HostContext,
            _method: &'a str,
            _params: Value,
        ) -> HostFuture<'a> {
            Box::pin(async { HostReply::error("unsupported") })
        }
    }

    #[tokio::test]
    async fn blob_shutdown_runtime_drains_live_backing_without_broker_or_script() {
        let parent = tempfile::tempdir().expect("isolated runtime shutdown");
        let directory = parent.path().join("backing");
        let mut runtime = ExtensionRuntime::new(
            PackageRegistry::default(),
            PathBuf::new(),
            Arc::new(Adapter),
        );
        Arc::get_mut(&mut runtime.0)
            .expect("unshared runtime")
            .blobs = crate::blobs::BlobStore::new(directory.clone());
        let run = runtime.0.blobs.invocation(
            InvocationIdentity {
                run_id: Uuid::new_v4(),
                correlation_id: Uuid::new_v4(),
                generation: 1,
                package_id: "fixture.shutdown".into(),
                handler_id: "run".into(),
                content_hash: "a".repeat(64),
            },
            Uuid::new_v4(),
        );
        run.begin(3, "text/plain".into(), "a".repeat(64))
            .await
            .expect("real temporary backing");
        runtime.shutdown_and_wait().await.expect("runtime drain");
        assert!(
            !directory.exists(),
            "runtime shutdown must join physical blob cleanup"
        );
        assert!(runtime.0.broker.lock().expect("broker state").is_none());
        assert_eq!(
            runtime.0.blobs.available_bytes(),
            crate::blobs::MAX_GLOBAL_BLOB_BYTES
        );
    }

    #[tokio::test]
    async fn changed_runtime_publication_cancels_a_real_queued_invocation() {
        let (_root, registry, second) = configured_runtime_fixture();
        let runtime = ExtensionRuntime::new(registry.clone(), PathBuf::new(), Arc::new(Adapter));
        runtime
            .0
            .plugins
            .lock()
            .expect("plugins")
            .insert("fixture.inherited".into(), Arc::new(Semaphore::new(0)));
        let (_caller, cancellation) = watch::channel(false);
        let invocation_runtime = runtime.clone();
        let invocation = tokio::spawn(async move {
            invocation_runtime
                .invoke(InvokeRequest {
                    package_id: "fixture.inherited".into(),
                    handler_id: "run".into(),
                    correlation_id: Uuid::new_v4(),
                    generation: 1,
                    context: Value::Null,
                    causality: Causality::root(Uuid::new_v4()),
                    cancellation,
                    batches: None,
                })
                .await
        });
        tokio::time::timeout(Duration::from_secs(2), async {
            while runtime.0.pending.available_permits() == 32 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("actual invocation queued");
        let mut changed = registry;
        changed
            .set_runtime_defaults(std::collections::BTreeMap::from([(
                "python".into(),
                second,
            )]))
            .expect("changed selection");
        runtime.publish_registry(changed).expect("publish");
        let result = tokio::time::timeout(Duration::from_secs(2), invocation)
            .await
            .expect("cancel completes")
            .expect("invocation task");
        assert!(matches!(result, Err(RuntimeError::Cancelled)));
        assert!(runtime.0.active.lock().expect("active cleanup").is_empty());
        assert_eq!(runtime.0.pending.available_permits(), 32);
    }

    fn configured_runtime_fixture() -> (tempfile::TempDir, PackageRegistry, PathBuf) {
        let root = tempfile::tempdir().expect("runtime configuration fixture");
        let source = root.path().join("source");
        std::fs::create_dir(&source).expect("source");
        let first = root.path().join("first.exe");
        let second = root.path().join("second.exe");
        std::fs::write(&first, b"first synthetic interpreter; never launched").expect("first");
        std::fs::write(&second, b"second synthetic interpreter; never launched").expect("second");
        let identity = crate::RuntimeIdentity::inspect(&first).expect("identity");
        let mut registry = PackageRegistry::default();
        registry
            .set_runtime_defaults(std::collections::BTreeMap::from([(
                "python".into(),
                identity.executable.clone(),
            )]))
            .expect("default");
        for id in ["fixture.inherited", "fixture.pinned"] {
            let manifest = serde_json::json!({"schema_version":1,"id":id,"name":"Fixture","version":"1","runtime":{"program":"python"},"files":["entry.py"],"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":"entry.py"}}]});
            std::fs::write(
                source.join("extension.json"),
                serde_json::to_vec(&manifest).expect("manifest JSON"),
            )
            .expect("manifest");
            std::fs::write(source.join("entry.py"), b"never executed").expect("code");
            let preview = crate::ImportPreview::from_directory(&source)
                .expect("preview")
                .stage_into(&root.path().join("packages"))
                .expect("stage");
            let hash = preview.content_hash.clone();
            registry
                .install(preview, "fixture".into(), false)
                .expect("install");
            if id == "fixture.pinned" {
                registry
                    .set_runtime_override(id, Some(identity.executable.clone()))
                    .expect("pin");
            }
            registry
                .approve(id, &hash, identity.clone())
                .expect("approve");
            registry.set_enabled(id, true, &[]).expect("enable");
        }
        (root, registry, second)
    }

    #[test]
    fn runtime_default_publication_cancels_inheriting_runs_but_not_pinned_runs() {
        let (_root, registry, second) = configured_runtime_fixture();
        let runtime = ExtensionRuntime::new(registry.clone(), PathBuf::new(), Arc::new(Adapter));
        let (inherited, inherited_cancel) = watch::channel(false);
        let (pinned, pinned_cancel) = watch::channel(false);
        runtime.0.active.lock().expect("active").insert(
            Uuid::new_v4(),
            ActiveRun {
                package: "fixture.inherited".into(),
                cancel: inherited,
                snapshot: None,
            },
        );
        runtime.0.active.lock().expect("active").insert(
            Uuid::new_v4(),
            ActiveRun {
                package: "fixture.pinned".into(),
                cancel: pinned,
                snapshot: None,
            },
        );
        runtime
            .publish_registry(registry.clone())
            .expect("unchanged publish");
        assert!(!*inherited_cancel.borrow());
        assert!(!*pinned_cancel.borrow());
        let mut changed = registry;
        changed
            .set_runtime_defaults(std::collections::BTreeMap::from([(
                "python".into(),
                second,
            )]))
            .expect("change default");
        runtime
            .publish_registry(changed)
            .expect("publish revocation through original boundary");
        assert!(*inherited_cancel.borrow());
        assert!(!*pinned_cancel.borrow());
        assert!(
            runtime.snapshot().expect("snapshot").packages[0]
                .trust
                .is_none()
        );
    }

    #[test]
    fn expansion_storage_owned_binding_updates_exact_run_snapshots_but_publication_still_cancels() {
        let (_root, registry, _) = configured_runtime_fixture();
        let reviewed = registry.packages[0].clone();
        let runtime = ExtensionRuntime::new(registry, PathBuf::new(), Arc::new(Adapter));
        let run_package = RunPackage::new(reviewed.clone());
        let (cancel, cancellation) = watch::channel(false);
        runtime.0.active.lock().expect("active").insert(
            Uuid::new_v4(),
            ActiveRun {
                package: reviewed.manifest.id.clone(),
                cancel,
                snapshot: Some(run_package.clone()),
            },
        );
        let mut bound = reviewed.clone();
        bound.data_uuid = Some(Uuid::new_v4());
        runtime
            .apply_storage_binding(&reviewed, &bound)
            .expect("exact host metadata commit");
        assert_eq!(run_package.snapshot().expect("run snapshot"), bound);
        assert!(!*cancellation.borrow());
        runtime
            .registered_run(&run_package)
            .expect("current run remains current");
        let mut forged = bound.clone();
        forged.enabled = false;
        assert_eq!(
            runtime.apply_storage_binding(&bound, &forged),
            Err(RuntimeError::Stale)
        );
        let mut ordinary = runtime.snapshot().expect("registry");
        ordinary.packages[0].data_uuid = Some(Uuid::new_v4());
        runtime
            .publish_registry(ordinary)
            .expect("ordinary publication");
        assert!(
            *cancellation.borrow(),
            "ordinary UUID changes are not silently ignored"
        );
        assert_eq!(
            runtime.registered_run(&run_package),
            Err(RuntimeError::Stale)
        );
    }

    #[test]
    fn expansion_storage_binding_rejects_stale_snapshot_before_active_registration() {
        let (_root, registry, _) = configured_runtime_fixture();
        let reviewed = registry.packages[0].clone();
        let runtime = ExtensionRuntime::new(registry, PathBuf::new(), Arc::new(Adapter));
        let old_run = RunPackage::new(reviewed.clone());
        let mut bound = reviewed.clone();
        bound.data_uuid = Some(Uuid::new_v4());
        runtime
            .apply_storage_binding(&reviewed, &bound)
            .expect("binding won registration race");
        let (cancel, _cancellation) = watch::channel(false);
        assert_eq!(
            runtime.register_run(Uuid::new_v4(), old_run, cancel),
            Err(RuntimeError::Stale)
        );
        assert!(runtime.0.active.lock().expect("active").is_empty());
        let (cancel, _cancellation) = watch::channel(false);
        let current_run = RunPackage::new(bound);
        runtime
            .register_run(Uuid::new_v4(), current_run.clone(), cancel)
            .expect("fresh exact snapshot");
        runtime
            .registered_run(&current_run)
            .expect("current admitted run");
        assert_eq!(runtime.0.active.lock().expect("active").len(), 1);
    }

    #[tokio::test]
    async fn expansion_storage_v1_dispatch_and_discovery_never_allocate_or_delegate() {
        let (_root, registry, _) = configured_runtime_fixture();
        let package = registry.packages[0].clone();
        let runtime = ExtensionRuntime::new(registry, PathBuf::new(), Arc::new(Adapter));
        let (_cancel, cancellation) = watch::channel(false);
        let context = HostContext {
            identity: InvocationIdentity {
                run_id: Uuid::new_v4(),
                correlation_id: Uuid::new_v4(),
                generation: 1,
                package_id: package.manifest.id.clone(),
                handler_id: "run".into(),
                content_hash: package.content_hash.clone(),
            },
            feature: "tools".into(),
            entry: "action".into(),
            causality: Causality::root(Uuid::new_v4()),
            cancellation,
            blobs: None,
        };
        let reply = runtime
            .dispatch_host(
                &RunPackage::new(package),
                &context,
                "storage.get",
                serde_json::json!({"key":"fixture"}),
            )
            .await
            .expect("normal host error")
            .reply;
        assert!(!reply.success);
        assert_eq!(reply.value["code"], "unsupported_transport");
        let methods = runtime.describe_for(&context);
        for method in methods["methods"]
            .as_array()
            .expect("methods")
            .iter()
            .filter(|method| {
                method["method"]
                    .as_str()
                    .is_some_and(|name| name.starts_with("storage."))
            })
        {
            assert_eq!(method["available"], false);
            assert_eq!(method["unavailable_code"], "unsupported_transport");
        }
        assert!(
            runtime.snapshot().expect("registry").packages[0]
                .data_uuid
                .is_none()
        );
    }

    #[tokio::test]
    async fn runtime_rejects_direct_config_bypass_even_when_package_trust_is_unchanged() {
        let (_root, registry, second) = configured_runtime_fixture();
        let package = registry.packages[0].clone();
        let runtime = ExtensionRuntime::new(registry.clone(), PathBuf::new(), Arc::new(Adapter));
        runtime
            .still_current(&package)
            .await
            .expect("current trusted selection");
        let mut bypassed = registry;
        bypassed.runtime_defaults.insert("python".into(), second);
        runtime
            .publish_registry(bypassed)
            .expect("structurally valid config");
        assert_eq!(
            runtime.still_current(&package).await,
            Err(RuntimeError::Untrusted)
        );
        let mut changed = runtime.snapshot().expect("snapshot");
        changed.runtime_defaults.clear();
        runtime.publish_registry(changed).expect("legacy fallback");
        runtime
            .still_current(&package)
            .await
            .expect("legacy approval fallback remains valid");
    }
    #[tokio::test(flavor = "current_thread")]
    async fn remediation_completion_status_and_error_text_survive_real_process_cleanup() {
        let root = tempfile::tempdir().expect("owned completion fixture");
        let source = root.path().join("source");
        std::fs::create_dir(&source).expect("source");
        let manifest = serde_json::json!({"schema_version":1,"id":"fixture.completion","name":"Completion","version":"1","runtime":{"program":"python"},"files":["entry.py"],"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":"entry.py"}}]});
        std::fs::write(
            source.join("extension.json"),
            serde_json::to_vec(&manifest).expect("manifest"),
        )
        .expect("write manifest");
        std::fs::write(source.join("entry.py"), "import json,sys\nf=json.loads(sys.stdin.readline())\nprint(json.dumps({'protocol_version':1,'op':'complete','run_id':f['run_id'],'status':f['context']['status'],'value':{'text':'Check input settings','code':'invalid_input'}}),flush=True)\n").expect("owned script");
        let preview = crate::ImportPreview::from_directory(&source)
            .expect("preview")
            .stage_into(&root.path().join("packages"))
            .expect("stage");
        let hash = preview.content_hash.clone();
        let mut registry = PackageRegistry::default();
        registry
            .install(preview, "fixture".into(), false)
            .expect("install");
        registry
            .approve(
                "fixture.completion",
                &hash,
                crate::RuntimeIdentity::inspect(&installed("python.exe")).expect("interpreter"),
            )
            .expect("approval");
        registry
            .set_enabled("fixture.completion", true, &[])
            .expect("enable");
        let fixture = fixture_path();
        let runtime = ExtensionRuntime::new(registry, fixture.clone(), Arc::new(Adapter));
        runtime
            .bind_owned_fixture_broker(&fixture)
            .expect("owned broker");
        for status in ["error", "handled", "not_handled", "success"] {
            let (_cancel, cancellation) = watch::channel(false);
            let outcome = runtime
                .invoke(InvokeRequest {
                    package_id: "fixture.completion".into(),
                    handler_id: "run".into(),
                    correlation_id: Uuid::new_v4(),
                    generation: 1,
                    context: serde_json::json!({"status":status}),
                    causality: Causality::root(Uuid::new_v4()),
                    cancellation,
                    batches: None,
                })
                .await
                .expect("valid completion remains distinct from native/protocol failure");
            assert_eq!(outcome.status, status);
            assert_eq!(outcome.value["text"], "Check input settings");
            assert_eq!(outcome.value["code"], "invalid_input");
        }
        runtime.shutdown();
    }
    fn installed(name: &str) -> PathBuf {
        std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
            .map(|directory| directory.join(name))
            .find(|path| path.is_file())
            .expect("required fixture interpreter already installed")
    }
    #[tokio::test(flavor = "current_thread")]
    async fn owned_python_node_powershell_and_pwsh_use_real_runtime_and_duplex_protocol() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).expect("workspace").join("csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe");
        let parent = std::env::current_exe().expect("test process");
        let broker = BrokerClient::launch_inner(
            &fixture,
            &[
                "--extension-fixture-broker".into(),
                std::process::id().to_string(),
                parent.to_string_lossy().into_owned(),
            ],
        )
        .expect("owned broker");
        let python = include_str!("../../../../docs/extensions/examples/protocol.py");
        let node = include_str!("../../../../docs/extensions/examples/protocol.js");
        let powershell = include_str!("../../../../docs/extensions/examples/protocol.ps1");
        for (program, executable, filename, script) in [
            ("python", installed("python.exe"), "entry.py", python),
            ("node", installed("node.exe"), "entry.js", node),
            (
                "powershell",
                PathBuf::from(std::env::var_os("WINDIR").expect("Windows"))
                    .join("System32/WindowsPowerShell/v1.0/powershell.exe"),
                "entry.ps1",
                powershell,
            ),
            ("pwsh", installed("pwsh.exe"), "entry.ps1", powershell),
        ] {
            let root = tempfile::tempdir().expect("owned fixture");
            let source = root.path().join("source");
            std::fs::create_dir(&source).expect("source");
            let manifest = serde_json::json!({"schema_version":1,"id":format!("fixture.{program}"),"name":"Fixture","version":"1","runtime":{"program":program,"timeout_ms":30000},"files":[filename],"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":filename}}]});
            std::fs::write(
                source.join("extension.json"),
                serde_json::to_vec(&manifest).expect("manifest"),
            )
            .expect("write manifest");
            std::fs::write(source.join(filename), script).expect("write own script");
            let preview = crate::ImportPreview::from_directory(&source)
                .expect("preview")
                .stage_into(&root.path().join("packages"))
                .expect("stage");
            let hash = preview.content_hash.clone();
            let mut registry = PackageRegistry::default();
            registry
                .install(preview, "owned fixture".into(), false)
                .expect("install");
            registry
                .approve(
                    &format!("fixture.{program}"),
                    &hash,
                    crate::RuntimeIdentity::inspect(&executable).expect("interpreter identity"),
                )
                .expect("fixture approval");
            // Test-only state bypass: production registry deliberately rejects script enablement
            // until all cross-feature acceptance gates pass. No user config is read or changed.
            registry.packages[0].enabled = true;
            let runtime = ExtensionRuntime::new(registry, fixture.clone(), Arc::new(Adapter));
            *runtime.0.broker.lock().expect("fixture broker slot") = Some(broker.clone());
            let (cancel, cancellation) = watch::channel(false);
            let result = runtime
                .invoke(InvokeRequest {
                    package_id: format!("fixture.{program}"),
                    handler_id: "run".into(),
                    correlation_id: Uuid::new_v4(),
                    generation: 1,
                    context: serde_json::json!({}),
                    causality: Causality::root(Uuid::new_v4()),
                    cancellation,
                    batches: None,
                })
                .await
                .unwrap_or_else(|error| panic!("{program} invocation failed: {error}"));
            assert_eq!(result.value["text"], "fixture", "{program}");
            let staged = runtime.snapshot().expect("snapshot").packages[0]
                .code_directory
                .clone()
                .expect("staged")
                .join(filename);
            std::fs::write(staged, b"changed owned fixture").expect("change code");
            let request = InvokeRequest {
                package_id: format!("fixture.{program}"),
                handler_id: "run".into(),
                correlation_id: Uuid::new_v4(),
                generation: 2,
                context: serde_json::json!({}),
                causality: Causality::root(Uuid::new_v4()),
                cancellation: cancel.subscribe(),
                batches: None,
            };
            assert!(
                matches!(runtime.invoke(request).await, Err(RuntimeError::Untrusted)),
                "changed code rejected before launch"
            );
        }
        let root = tempfile::tempdir().expect("owned EXE package");
        let source = root.path().join("source");
        std::fs::create_dir(&source).expect("EXE source");
        let mut files = Vec::new();
        for file in
            std::fs::read_dir(fixture.parent().expect("fixture directory")).expect("fixture files")
        {
            let file = file.expect("fixture file");
            if !file.file_type().expect("fixture type").is_file()
                || file
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "pdb")
            {
                continue;
            }
            let name = file.file_name().to_string_lossy().into_owned();
            std::fs::copy(file.path(), source.join(&name))
                .expect("copy only owned test runtime files");
            files.push(name);
        }
        let manifest = serde_json::json!({"schema_version":1,"id":"fixture.exe","name":"EXE","version":"1","runtime":{"program":"exe","timeout_ms":30000,"arguments":["--extension-fixture-child","protocol-host-call"]},"files":files,"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":"NxTools.PlatformHost.Tests.exe"}}]});
        std::fs::write(
            source.join("extension.json"),
            serde_json::to_vec(&manifest).expect("EXE declaration"),
        )
        .expect("EXE manifest");
        let preview = crate::ImportPreview::from_directory(&source)
            .expect("EXE preview")
            .stage_into(&root.path().join("packages"))
            .expect("EXE stage");
        let hash = preview.content_hash.clone();
        let staged_exe = preview
            .code_directory
            .as_ref()
            .expect("EXE directory")
            .join("NxTools.PlatformHost.Tests.exe");
        let mut registry = PackageRegistry::default();
        registry
            .install(preview, "owned fixture".into(), false)
            .expect("EXE install");
        registry
            .approve(
                "fixture.exe",
                &hash,
                crate::RuntimeIdentity::inspect(&staged_exe).expect("EXE identity"),
            )
            .expect("EXE approval");
        registry
            .set_enabled("fixture.exe", true, &[])
            .expect("explicit fixture enable");
        let runtime = ExtensionRuntime::new(registry, fixture.clone(), Arc::new(Adapter));
        *runtime.0.broker.lock().expect("EXE broker") = Some(broker.clone());
        let (_cancel, cancellation) = watch::channel(false);
        let outcome = runtime
            .invoke(InvokeRequest {
                package_id: "fixture.exe".into(),
                handler_id: "run".into(),
                correlation_id: Uuid::new_v4(),
                generation: 1,
                context: serde_json::json!({}),
                causality: Causality::root(Uuid::new_v4()),
                cancellation,
                batches: None,
            })
            .await
            .expect("staged EXE actual invocation");
        assert_eq!(outcome.value["text"], "fixture");
        broker.shutdown();
    }
    fn fixture_path() -> PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors().nth(3).expect("workspace")
            .join("csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe")
    }

    fn tree_package(
        root: &std::path::Path,
        fixture: &std::path::Path,
        id: &str,
        timeout_ms: u32,
    ) -> InstalledPackage {
        let source = root.join(id);
        std::fs::create_dir(&source).expect("owned source");
        let mut files = Vec::new();
        for file in
            std::fs::read_dir(fixture.parent().expect("fixture directory")).expect("fixture files")
        {
            let file = file.expect("fixture file");
            if !file.file_type().expect("file type").is_file()
                || file
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "pdb")
            {
                continue;
            }
            let name = file.file_name().to_string_lossy().into_owned();
            std::fs::copy(file.path(), source.join(&name)).expect("copy owned fixture");
            files.push(name);
        }
        let marker = root.join(format!("{id}.marker"));
        let manifest = serde_json::json!({"schema_version":1,"id":id,"name":"Fixture","version":"1",
            "runtime":{"program":"exe","timeout_ms":timeout_ms,"arguments":["--extension-fixture-child","tree",marker.to_string_lossy()]},
            "files":files,"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action",
            "action":{"type":"script","entry_file":"NxTools.PlatformHost.Tests.exe"}}]});
        std::fs::write(
            source.join("extension.json"),
            serde_json::to_vec(&manifest).expect("manifest"),
        )
        .expect("write manifest");
        let preview = crate::ImportPreview::from_directory(&source)
            .expect("preview")
            .stage_into(&root.join("packages"))
            .expect("stage");
        let executable = preview
            .code_directory
            .as_ref()
            .expect("code")
            .join("NxTools.PlatformHost.Tests.exe");
        let hash = preview.content_hash.clone();
        let mut registry = PackageRegistry::default();
        registry
            .install(preview, "owned fixture".into(), false)
            .expect("install");
        registry
            .approve(
                id,
                &hash,
                crate::RuntimeIdentity::inspect(&executable).expect("identity"),
            )
            .expect("approve");
        let mut package = registry.packages.remove(0);
        package.enabled = true; // isolated admission fixture; no persistent config or user package.
        package
    }

    fn request(id: &str, cancellation: watch::Receiver<bool>) -> InvokeRequest {
        InvokeRequest {
            package_id: id.into(),
            handler_id: "run".into(),
            correlation_id: Uuid::new_v4(),
            generation: 1,
            context: serde_json::json!({}),
            causality: Causality::root(Uuid::new_v4()),
            cancellation,
            batches: None,
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn blocked_package_has_exactly_32_pending_without_reducing_four_global_slots() {
        let root = tempfile::tempdir().expect("owned packages");
        let fixture = fixture_path();
        let ids = [
            "fixture.queue",
            "fixture.two",
            "fixture.three",
            "fixture.four",
        ];
        let mut registry = PackageRegistry::default();
        for id in ids {
            registry
                .packages
                .push(tree_package(root.path(), &fixture, id, 30_000));
        }
        let runtime = ExtensionRuntime::new(registry, fixture.clone(), Arc::new(Adapter));
        runtime
            .bind_owned_fixture_broker(&fixture)
            .expect("owned fake-store broker");
        let (cancel, cancellation) = watch::channel(false);
        tokio::time::timeout(Duration::from_secs(20), async {
            let mut tasks = Vec::new();
            let active_runtime = runtime.clone();
            let active_request = request(ids[0], cancellation.clone());
            tasks.push(tokio::spawn(async move {
                active_runtime.invoke(active_request).await
            }));
            while !root.path().join(format!("{}.marker", ids[0])).exists() {
                assert!(
                    !tasks[0].is_finished(),
                    "first invocation must reach native child"
                );
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            for _ in 0..32 {
                let queued_runtime = runtime.clone();
                let queued_request = request(ids[0], cancellation.clone());
                tasks.push(tokio::spawn(async move {
                    queued_runtime.invoke(queued_request).await
                }));
            }
            while runtime.0.pending.available_permits() != 0 {
                tokio::task::yield_now().await;
            }
            assert_eq!(runtime.0.active.lock().expect("active").len(), 33);
            assert!(matches!(
                runtime.invoke(request(ids[0], cancellation.clone())).await,
                Err(RuntimeError::QueueFull)
            ));
            // Three unrelated packages still start even with the pending queue full.
            for id in &ids[1..] {
                let other_runtime = runtime.clone();
                let other_request = request(id, cancellation.clone());
                tasks.push(tokio::spawn(async move {
                    other_runtime.invoke(other_request).await
                }));
                while !root.path().join(format!("{id}.marker")).exists() {
                    assert!(
                        !tasks.last().expect("other task").is_finished(),
                        "unrelated package must start"
                    );
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }
            assert_eq!(runtime.0.global.available_permits(), 0);
            assert_eq!(runtime.0.active.lock().expect("active").len(), 36);
            cancel
                .send(true)
                .expect("cancel owned runs including queued");
            for task in tasks {
                assert!(matches!(
                    task.await.expect("invocation task"),
                    Err(RuntimeError::Cancelled)
                ));
            }
            // Runtime futures return promptly, but slots remain owned until C# tree cleanup.
            while runtime.0.global.available_permits() != 4 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            assert_eq!(runtime.0.pending.available_permits(), 32);
            assert!(runtime.0.active.lock().expect("active cleared").is_empty());
            assert!(
                runtime
                    .broker()
                    .expect("same healthy broker")
                    .is_available()
            );
        })
        .await
        .expect("bounded queue and native cleanup regression");
        runtime.shutdown();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn lost_transport_restarts_only_after_owned_broker_reaped_and_not_after_shutdown() {
        let fixture = fixture_path();
        let runtime = ExtensionRuntime::new(
            PackageRegistry::default(),
            fixture.clone(),
            Arc::new(Adapter),
        );
        runtime
            .bind_owned_fixture_broker(&fixture)
            .expect("owned fake-store broker");
        let old = runtime.broker().expect("original");
        old.shutdown(); // The same stop/reap path used by transport EOF or write failure.
        tokio::time::timeout(Duration::from_secs(10), async {
            while !old.can_replace() {
                assert!(matches!(
                    runtime.broker(),
                    Err(RuntimeError::NativeUnavailable)
                ));
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("exact owned process reaped");
        let replacement = runtime.secret_store().expect("demand replacement");
        assert_eq!(
            replacement
                .get("fixture.recovery", "token")
                .await
                .expect("fake store only"),
            None
        );
        assert!(
            runtime
                .broker()
                .expect("replacement available")
                .is_available()
        );
        assert!(!old.is_available());
        runtime.shutdown();
        assert!(matches!(
            runtime.broker(),
            Err(RuntimeError::NativeUnavailable)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn pending_wait_counts_against_invocation_deadline() {
        let root = tempfile::tempdir().expect("owned package");
        let fixture = fixture_path();
        let package = tree_package(root.path(), &fixture, "fixture.deadline", 100);
        let mut registry = PackageRegistry::default();
        registry.packages.push(package);
        let runtime = ExtensionRuntime::new(registry, fixture, Arc::new(Adapter));
        // No broker or interpreter starts: the caller deadline expires behind an occupied slot.
        let plugin = Arc::new(Semaphore::new(0));
        runtime
            .0
            .plugins
            .lock()
            .expect("plugins")
            .insert("fixture.deadline".into(), plugin);
        let (_cancel, cancellation) = watch::channel(false);
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            runtime.invoke(request("fixture.deadline", cancellation)),
        )
        .await
        .expect("bounded queue deadline");
        assert!(matches!(result, Err(RuntimeError::Timeout)));
        assert_eq!(runtime.0.pending.available_permits(), 32);
        assert_eq!(runtime.0.global.available_permits(), 4);
        assert!(runtime.0.active.lock().expect("active").is_empty());
    }
}
