#[path = "pages.rs"]
mod pages;
#[cfg(test)]
#[path = "pages_tests.rs"]
mod pages_tests;
use crate::clipboard::extension_adapter::{
    ClipboardExtensionAdapter, ClipboardExtensionContent, ClipboardExtensionSnapshot,
    ClipboardExtensionSnapshotContent,
};
use crate::clipboard_worker::ClipboardCommitCausality;
use nxtools_download::{
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadCommand, DownloadCommandEnvelope, DownloadEngine,
    DownloadTaskAction, DownloadTaskId, RequestOrigin,
};
use nxtools_extensions::{FeatureAdapter, HostContext, HostFuture, HostReply};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub(crate) struct UiHostCall {
    pub identity: nxtools_extensions::InvocationIdentity,
    pub method: String,
    pub params: Value,
    pub causality: nxtools_extensions::Causality,
    pub cancellation: tokio::sync::watch::Receiver<bool>,
    pub reply: async_channel::Sender<HostReply>,
}
#[derive(Clone)]
pub(crate) struct FeatureAdapters {
    engine: nxtools_download::WeakDownloadEngine,
    clipboard: Option<ClipboardExtensionAdapter>,
    ui: Option<async_channel::Sender<UiHostCall>>,
    blocking: Arc<Semaphore>,
    pub(super) manual_effects: super::manual_runs::ManualEffects,
    clipboard_writes: Arc<Semaphore>,
    cursors: Arc<std::sync::Mutex<nxtools_extensions::PageCursors>>,
}
impl FeatureAdapters {
    pub(crate) fn new(
        engine: DownloadEngine,
        clipboard: Option<ClipboardExtensionAdapter>,
        ui: Option<async_channel::Sender<UiHostCall>>,
    ) -> Self {
        Self {
            engine: engine.downgrade(),
            clipboard,
            ui,
            blocking: Arc::new(Semaphore::new(4)),
            manual_effects: Default::default(),
            clipboard_writes: Arc::new(Semaphore::new(4)),
            cursors: Default::default(),
        }
    }
    async fn dispatch(&self, context: &HostContext, method: &str, params: Value) -> HostReply {
        if *context.cancellation.borrow() {
            return HostReply::error("cancelled");
        }
        if method == "clipboard.query_page" {
            return self.clipboard_page(context, params).await;
        }
        if method.starts_with("downloads.") {
            return self.download(context, method, params).await;
        }
        if method.starts_with("clipboard.") && method != "clipboard.copy" {
            let Some(clipboard) = self.clipboard.as_ref() else {
                return HostReply::error("ui_unavailable");
            };
            return match method {
                "clipboard.get" => {
                    let Ok(input) = serde_json::from_value::<ItemId>(params) else {
                        return HostReply::error("invalid_params");
                    };
                    match clipboard.get(input.item_id).await {
                        Ok(item) => success(snapshot(item)),
                        Err(_) => HostReply::error("clipboard_unavailable"),
                    }
                }
                "clipboard.query" => {
                    let Ok(input) = serde_json::from_value::<ClipboardQuery>(params) else {
                        return HostReply::error("invalid_params");
                    };
                    match clipboard.query(&input.query, input.limit).await {
                        Ok(items) => {
                            success(Value::Array(items.into_iter().map(snapshot).collect()))
                        }
                        Err(_) => HostReply::error("clipboard_unavailable"),
                    }
                }
                "clipboard.create_derived" => {
                    let Ok(input) = serde_json::from_value::<Derived>(params) else {
                        return HostReply::error("invalid_params");
                    };
                    let content = match (input.text, input.paths) {
                        (Some(text), None) => ClipboardExtensionContent::Text(text),
                        (None, Some(paths)) => ClipboardExtensionContent::Files(paths),
                        _ => return HostReply::error("invalid_params"),
                    };
                    let causality = ClipboardCommitCausality {
                        root_operation_id: context.causality.root_id,
                        origin_plugin: context.causality.origin_plugin.clone(),
                        invoked_handlers: context.causality.handlers.clone(),
                        depth: context.causality.handlers.len() as u8,
                        derived_from: Some(input.source_item_id),
                    };
                    let Ok(permit) = self.clipboard_writes.clone().try_acquire_owned() else {
                        return HostReply::error("clipboard_busy");
                    };
                    let cancellation = context.cancellation.clone();
                    let clipboard = clipboard.clone();
                    let effects = self.manual_effects.clone();
                    let identity = context.identity.clone();
                    // An admitted serialized write can outlive cancellation of its caller.
                    // This bounded task observes its actual commit without replaying the write.
                    let task = tokio::spawn(async move {
                        let _permit = permit;
                        if *cancellation.borrow() {
                            return HostReply::error("cancelled");
                        }
                        match clipboard
                            .create_derived(input.source_item_id, content, causality)
                            .await
                        {
                            Ok(committed) => {
                                effects.committed(&identity, committed.item_id);
                                success(serde_json::json!({"item_id":committed.item_id}))
                            }
                            Err(_) => HostReply::error("clipboard_write_failed"),
                        }
                    });
                    task.await
                        .unwrap_or_else(|_| HostReply::error("clipboard_write_failed"))
                }
                _ => HostReply::error("unsupported"),
            };
        }
        if !matches!(
            method,
            "common.open"
                | "common.program"
                | "common.copy"
                | "common.notify"
                | "clipboard.copy"
                | "quick_launch.query"
                | "quick_launch.search"
                | "quick_launch.search_catalog"
        ) {
            return HostReply::error("unsupported");
        }
        let Some(ui) = self.ui.as_ref() else {
            return HostReply::error("ui_unavailable");
        };
        let (reply, receiver) = async_channel::bounded(1);
        let request = UiHostCall {
            identity: context.identity.clone(),
            method: method.into(),
            params,
            causality: context.causality.clone(),
            cancellation: context.cancellation.clone(),
            reply,
        };
        if ui.try_send(request).is_err() {
            return HostReply::error("ui_queue_full");
        }
        match receiver.recv().await {
            Ok(reply) => reply,
            Err(_) => HostReply::error("ui_unavailable"),
        }
    }
    async fn download(&self, context: &HostContext, method: &str, params: Value) -> HostReply {
        let Some(engine) = self.engine.upgrade() else {
            return HostReply::error("download_unavailable");
        };
        let permit = match self.blocking.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => return HostReply::error("busy"),
        };
        let method = method.to_owned();
        let correlation = context.identity.correlation_id;
        let cancellation = context.cancellation.clone();
        let causality = nxtools_download::DownloadExtensionCausality {
            root_id: context.causality.root_id,
            origin_plugin: context.identity.package_id.clone(),
            handlers: context.causality.handlers.clone(),
        };
        let control = if matches!(
            method.as_str(),
            "downloads.probe" | "downloads.submit" | "downloads.enqueue"
        ) {
            match nxtools_download::DownloadResolutionControl::with_extension_causality(causality) {
                Ok(control) => control,
                Err(_) => return HostReply::error("invalid_causality"),
            }
        } else {
            nxtools_download::DownloadResolutionControl::default()
        };
        let control = control.with_host_cancellation(cancellation.clone());
        // Dropping the awaiting host future must retire the real engine operation, not only
        // detach its blocking JoinHandle. The blocking closure owns its permit until it exits.
        let cancellation_guard = CancelDownloadOnDrop(control.clone());
        let cursors = self.cursors.clone();
        let identity = context.identity.clone();
        let mut worker = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            if control.is_cancelled() {
                return HostReply::error("cancelled");
            }
            match method.as_str() {
                "downloads.list_page" => pages::download_page(&engine, &cursors, &identity, params),
                "downloads.list" => {
                    let Ok(input) = serde_json::from_value::<List>(params) else {
                        return HostReply::error("invalid_params");
                    };
                    if !(1..=32).contains(&input.limit) {
                        return HostReply::error("invalid_params");
                    };
                    match engine.tasks() {
                        Ok(tasks) => success(Value::Array(
                            tasks
                                .into_iter()
                                .take(input.limit)
                                .map(|task| task_snapshot(&task))
                                .collect(),
                        )),
                        Err(_) => HostReply::error("download_failed"),
                    }
                }
                "downloads.get" => {
                    let Ok(input) = serde_json::from_value::<TaskId>(params) else {
                        return HostReply::error("invalid_params");
                    };
                    match engine.task(input.task_id) {
                        Ok(Some(task)) => success(task_snapshot(&task)),
                        _ => HostReply::error("task_unavailable"),
                    }
                }
                "downloads.pause" | "downloads.resume" | "downloads.cancel" => {
                    let Ok(input) = serde_json::from_value::<TaskId>(params) else {
                        return HostReply::error("invalid_params");
                    };
                    let action = match method.as_str() {
                        "downloads.pause" => DownloadTaskAction::Pause,
                        "downloads.resume" => DownloadTaskAction::Resume,
                        _ => DownloadTaskAction::Cancel,
                    };
                    command_reply(engine.handle_command(DownloadCommandEnvelope {
                        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                        request_id: correlation,
                        origin: RequestOrigin::Internal,
                        command: DownloadCommand::Task {
                            task_id: input.task_id,
                            action,
                        },
                    }))
                }
                "downloads.probe" | "downloads.submit" | "downloads.enqueue" => {
                    let Ok(input) = serde_json::from_value::<Submit>(params) else {
                        return HostReply::error("invalid_params");
                    };
                    let options = match input.resolution_options(correlation) {
                        Ok(options) => options,
                        Err(()) => return HostReply::error("invalid_params"),
                    };
                    if method == "downloads.probe" {
                        match engine.probe_with_resolution(input.request, options, control) {
                            Ok(probe) => {
                                success(serde_json::to_value(probe).unwrap_or(Value::Null))
                            }
                            Err(error) => download_error_reply(error),
                        }
                    } else {
                        match engine.submit_with_resolution(
                            input.request,
                            RequestOrigin::Internal,
                            if method == "downloads.enqueue" {
                                nxtools_download::DownloadSubmitMode::QueueOnly
                            } else {
                                nxtools_download::DownloadSubmitMode::StartNow
                            },
                            input.idempotency_key,
                            options,
                            control,
                        ) {
                            Ok((task_id, duplicate)) => success(
                                serde_json::json!({"task_id":task_id,"duplicate":duplicate}),
                            ),
                            Err(error) => download_error_reply(error),
                        }
                    }
                }
                _ => HostReply::error("unsupported"),
            }
        });
        let result = tokio::select! {
            biased;
            () = wait_for_download_cancellation(cancellation) => {
                cancellation_guard.0.cancel();
                // Cancellation is not proof that uncooperative blocking IO has finished.
                // Await its terminal result; the permit is never freed by dropping this future.
                let _ = worker.await;
                return HostReply::error("cancelled");
            }
            result = &mut worker => result,
        };
        result.unwrap_or_else(|_| HostReply::error("download_failed"))
    }
}
impl FeatureAdapter for FeatureAdapters {
    fn describe(&self) -> Value {
        let mut description = serde_json::json!({"version":1,"headless":self.ui.is_none(),"downloads":["list","get","probe","submit","enqueue","pause","resume","cancel"],"clipboard":if self.clipboard.is_some(){vec!["get","query","create_derived","copy"]}else{Vec::<&str>::new()},"ui":if self.ui.is_some(){vec!["common.open","common.program","common.copy","common.notify","quick_launch.query","quick_launch.search"]}else{Vec::<&str>::new()},"limits":{"frame_bytes":256*1024,"active_host_calls":8,"total_host_calls":64,"credential_native_bytes":2560,"page_items":32,"page_bytes":128*1024,"cursor_ttl_ms":60000,"cursor_slots":128,"clipboard_content_bytes":128*1024,"clipboard_query_bytes":4096}});
        description["methods"] =
            nxtools_extensions::method_catalog(self.ui.is_some(), self.clipboard.is_some(), false);
        description["search_scope"] = serde_json::json!({"quick_launch.search":"local_catalog","indexed_files":false,"script_providers":false});
        description
    }
    fn describe_for(&self, context: &HostContext) -> Value {
        let mut description = self.describe();
        description["methods"] = nxtools_extensions::method_catalog(
            self.ui.is_some(),
            self.clipboard.is_some(),
            context.feature == "downloads" && context.entry == "resolve",
        );
        description
    }
    fn call<'a>(
        &'a self,
        context: &'a HostContext,
        method: &'a str,
        params: Value,
    ) -> HostFuture<'a> {
        Box::pin(self.dispatch(context, method, params))
    }
}
struct CancelDownloadOnDrop(nxtools_download::DownloadResolutionControl);
impl Drop for CancelDownloadOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
async fn wait_for_download_cancellation(mut cancellation: tokio::sync::watch::Receiver<bool>) {
    loop {
        let cancelled = *cancellation.borrow_and_update();
        if cancelled || cancellation.changed().await.is_err() {
            return;
        }
    }
}
fn download_error_reply(error: nxtools_download::DownloadEngineError) -> HostReply {
    use nxtools_download::{DownloadEngineError, DownloadResolutionError};
    HostReply::error(match error {
        DownloadEngineError::Resolution(DownloadResolutionError::Cancelled) => "cancelled",
        DownloadEngineError::Resolution(
            DownloadResolutionError::ResolutionRequired | DownloadResolutionError::IdentityChanged,
        ) => "resolution_required",
        DownloadEngineError::InvalidRequest(_) => "invalid_request",
        DownloadEngineError::TaskNotFound(_) => "task_unavailable",
        DownloadEngineError::InvalidAction { .. } | DownloadEngineError::RetryNotAllowed(_) => {
            "invalid_state"
        }
        DownloadEngineError::Storage(_) => "storage_failed",
        DownloadEngineError::NetworkPolicy(_) | DownloadEngineError::DownloadHttpPolicy(_) => {
            "network_policy"
        }
        DownloadEngineError::Http(_) => "network_failed",
        _ => "download_failed",
    })
}
fn success(value: Value) -> HostReply {
    HostReply {
        success: true,
        value,
    }
}
fn snapshot(item: ClipboardExtensionSnapshot) -> Value {
    match item.content {
        ClipboardExtensionSnapshotContent::Text(text) => {
            serde_json::json!({"item_id":item.item_id,"kind":"text","text":text})
        }
        ClipboardExtensionSnapshotContent::Files(paths) => {
            serde_json::json!({"item_id":item.item_id,"kind":"files","paths":paths})
        }
        ClipboardExtensionSnapshotContent::Image(image) => {
            serde_json::json!({"item_id":item.item_id,"kind":"image","width":image.width,"height":image.height,"image_read_available":false})
        }
    }
}
fn task_snapshot(task: &nxtools_download::DownloadEngineTask) -> Value {
    serde_json::json!({"task_id":task.task.id(),"status":task.task.status(),"request":task.task.request(),"resolution":task.task.resolution()})
}
fn command_reply(result: nxtools_download::DownloadCommandResult) -> HostReply {
    if let nxtools_download::DownloadCommandOutcome::Rejected { fault } = &result.outcome {
        let code = match fault.code {
            nxtools_download::DownloadFaultCode::InvalidRequest => "invalid_request",
            nxtools_download::DownloadFaultCode::NotFound => "task_unavailable",
            nxtools_download::DownloadFaultCode::InvalidState => "invalid_state",
            nxtools_download::DownloadFaultCode::ResolutionRequired => "resolution_required",
            nxtools_download::DownloadFaultCode::Cancelled => "cancelled",
            _ => "download_failed",
        };
        return HostReply::error(code);
    }
    match serde_json::to_value(result) {
        Ok(value) => success(value),
        Err(_) => HostReply::error("download_failed"),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ItemId {
    item_id: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClipboardQuery {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Derived {
    source_item_id: i64,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    paths: Option<Vec<String>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskId {
    task_id: DownloadTaskId,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct List {
    #[serde(default = "default_limit")]
    limit: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Submit {
    request: nxtools_download::DownloadRequest,
    #[serde(default)]
    idempotency_key: Option<uuid::Uuid>,
    #[serde(default)]
    resolution: Option<HostDownloadResolutionOptions>,
}
/// Host API subset of DownloadResolutionOptions: only resolve or a host-issued receipt.
/// Reuse the probe's provenance.operation_id, not the subsequent host-call correlation ID.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HostDownloadResolutionOptions {
    #[serde(default)]
    operation_id: Option<uuid::Uuid>,
    #[serde(default)]
    mode: HostDownloadResolutionMode,
}
#[derive(Clone, Copy, Default, Deserialize)]
#[serde(
    tag = "kind",
    content = "receipt",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum HostDownloadResolutionMode {
    #[default]
    Resolve,
    Receipt(uuid::Uuid),
}
impl Submit {
    fn resolution_options(
        &self,
        correlation: uuid::Uuid,
    ) -> Result<nxtools_download::DownloadResolutionOptions, ()> {
        let Some(resolution) = self.resolution.as_ref() else {
            return Ok(nxtools_download::DownloadResolutionOptions {
                operation_id: correlation,
                mode: nxtools_download::DownloadResolutionMode::Resolve,
            });
        };
        let operation_id = resolution.operation_id.unwrap_or(correlation);
        if operation_id.is_nil() {
            return Err(());
        }
        let mode = match resolution.mode {
            HostDownloadResolutionMode::Resolve => {
                nxtools_download::DownloadResolutionMode::Resolve
            }
            HostDownloadResolutionMode::Receipt(receipt) => {
                if receipt.is_nil() || resolution.operation_id.is_none() {
                    return Err(());
                }
                nxtools_download::DownloadResolutionMode::Receipt(receipt)
            }
        };
        Ok(nxtools_download::DownloadResolutionOptions { operation_id, mode })
    }
}
fn default_limit() -> usize {
    32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;

    struct LocalDownloadServer {
        url: String,
        requests: Arc<AtomicUsize>,
        stopped: Arc<AtomicBool>,
        worker: Option<std::thread::JoinHandle<()>>,
    }
    impl LocalDownloadServer {
        fn start() -> Self {
            use std::io::{Read as _, Write as _};
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("local fixture");
            let url = format!(
                "http://{}/file.bin",
                listener.local_addr().expect("address")
            );
            listener.set_nonblocking(true).expect("bounded listener");
            let stopped = Arc::new(AtomicBool::new(false));
            let requests = Arc::new(AtomicUsize::new(0));
            let worker_stop = stopped.clone();
            let worker_requests = requests.clone();
            let worker = std::thread::spawn(move || {
                while !worker_stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream
                                .set_nonblocking(false)
                                .expect("blocking accepted socket");
                            stream
                                .set_read_timeout(Some(Duration::from_secs(2)))
                                .expect("read timeout");
                            stream
                                .set_write_timeout(Some(Duration::from_secs(2)))
                                .expect("write timeout");
                            let mut input = Vec::new();
                            let mut buffer = [0_u8; 1024];
                            while input.len() < 8192
                                && !input.windows(4).any(|part| part == b"\r\n\r\n")
                            {
                                let Ok(count) = stream.read(&mut buffer) else {
                                    break;
                                };
                                if count == 0 {
                                    break;
                                }
                                input.extend_from_slice(&buffer[..count]);
                            }
                            worker_requests.fetch_add(1, Ordering::SeqCst);
                            let input = String::from_utf8_lossy(&input);
                            let head = input.starts_with("HEAD ");
                            let range = input.to_ascii_lowercase().contains("range: bytes=0-0");
                            let (status, length, range_header) = if range {
                                ("206 Partial Content", 1, "Content-Range: bytes 0-0/8\r\n")
                            } else {
                                ("200 OK", 8, "")
                            };
                            let response = format!(
                                "HTTP/1.1 {status}\r\nContent-Length: {length}\r\n{range_header}Accept-Ranges: bytes\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n"
                            );
                            let _ = stream.write_all(response.as_bytes());
                            if !head {
                                let _ = stream.write_all(if range { b"f" } else { b"fixture!" });
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(2));
                        }
                        Err(_) => break,
                    }
                }
            });
            Self {
                url,
                requests,
                stopped,
                worker: Some(worker),
            }
        }
    }
    impl Drop for LocalDownloadServer {
        fn drop(&mut self) {
            self.stopped.store(true, Ordering::SeqCst);
            if let Some(worker) = self.worker.take() {
                worker.join().expect("owned HTTP fixture shutdown");
            }
        }
    }
    struct AdapterResolver {
        calls: AtomicUsize,
        retired: Arc<AtomicUsize>,
        target: String,
        validity: Duration,
        pause: bool,
        entered: tokio::sync::Notify,
        release: tokio::sync::Notify,
        control: std::sync::Mutex<Option<nxtools_download::DownloadResolutionControl>>,
    }
    struct RetiredResolver(Arc<AtomicUsize>);
    impl Drop for RetiredResolver {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    impl nxtools_download::DownloadRequestResolver for AdapterResolver {
        fn identity(&self) -> nxtools_download::DownloadResolverIdentity {
            nxtools_download::DownloadResolverIdentity {
                content_version: "adapter-fixture-content".into(),
                settings_version: "adapter-fixture-settings".into(),
            }
        }
        fn resolve<'a>(
            &'a self,
            _: &'a nxtools_download::DownloadRequest,
            operation: &'a nxtools_download::DownloadResolutionOperation,
        ) -> nxtools_download::DownloadResolutionFuture<'a> {
            Box::pin(async move {
                let _retired = RetiredResolver(self.retired.clone());
                self.calls.fetch_add(1, Ordering::SeqCst);
                *self.control.lock().expect("fixture control") = Some(operation.control.clone());
                self.entered.notify_one();
                if self.pause {
                    self.release.notified().await;
                }
                // Deliberately return a result even if cancelled: the actual engine, not the
                // injected resolver, must refuse probe publication and task persistence.
                Ok(nxtools_download::DownloadResolutionResult::Resolved(
                    Box::new(nxtools_download::DownloadResolvedRequest {
                        source_url: self.target.clone(),
                        file_name: None,
                        context: None,
                        media: None,
                        provenance: vec![nxtools_download::DownloadResolverStage {
                            package_id: "fixture.resolver".into(),
                            handler_id: "resolve".into(),
                            content_version: "adapter-fixture-content".into(),
                        }],
                        valid_for: Some(self.validity),
                    }),
                ))
            })
        }
    }
    fn adapter_resolver(
        server: &LocalDownloadServer,
        pause: bool,
        validity: Duration,
    ) -> Arc<AdapterResolver> {
        Arc::new(AdapterResolver {
            calls: AtomicUsize::new(0),
            retired: Arc::new(AtomicUsize::new(0)),
            target: server.url.clone(),
            validity,
            pause,
            entered: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            control: std::sync::Mutex::new(None),
        })
    }
    pub(super) fn download_context() -> (HostContext, tokio::sync::watch::Sender<bool>) {
        let (cancel, cancellation) = tokio::sync::watch::channel(false);
        let root_id = uuid::Uuid::new_v4();
        (
            HostContext {
                identity: nxtools_extensions::InvocationIdentity {
                    run_id: uuid::Uuid::new_v4(),
                    correlation_id: uuid::Uuid::new_v4(),
                    generation: 1,
                    package_id: "fixture.host".into(),
                    handler_id: "action".into(),
                    content_hash: "a".repeat(64),
                },
                feature: "tools".into(),
                entry: "action".into(),
                causality: nxtools_extensions::Causality::root(root_id)
                    .enter("fixture.host", "action")
                    .expect("fixture causality"),
                cancellation,
                blobs: None,
            },
            cancel,
        )
    }
    fn local_request(
        server: &LocalDownloadServer,
        directory: &std::path::Path,
    ) -> nxtools_download::DownloadRequest {
        serde_json::from_value(serde_json::json!({
            "source_url":server.url.replace("/file.bin", "/original"),
            "save_directory":directory,
            "save_directory_explicit":true,
            "file_name":"chosen.bin",
            "headers":[],
            "connection_limit":1,
            "scheduled_at_unix_ms":nxtools_ipc::unix_time_ms() + 60_000
        }))
        .expect("synthetic request with compatible defaults")
    }
    async fn wait_until(mut condition: impl FnMut() -> bool) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !condition() {
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
        })
        .await
        .expect("bounded fixture condition");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn download_host_cancellation_and_future_drop_retire_paused_resolution_without_effects() {
        for method in ["downloads.probe", "downloads.enqueue", "downloads.submit"] {
            for drop_future in [false, true] {
                let root = tempfile::tempdir().expect("owned DB");
                let server = LocalDownloadServer::start();
                let engine =
                    DownloadEngine::open(root.path().join("downloads.db")).expect("engine");
                let resolver = adapter_resolver(&server, true, Duration::from_secs(600));
                engine
                    .set_request_resolver(resolver.clone())
                    .expect("inject paused resolver");
                let adapter = FeatureAdapters::new(engine.clone(), None, None);
                let (context, cancel) = download_context();
                let mut call = adapter.call(
                    &context,
                    method,
                    serde_json::json!({
                        "request":local_request(&server, root.path())
                    }),
                );
                tokio::select! {
                    () = resolver.entered.notified() => {}
                    reply = &mut call => panic!("paused resolver returned early: {}", reply.success),
                    () = tokio::time::sleep(Duration::from_secs(5)) => panic!("resolver did not start"),
                }
                assert_eq!(adapter.blocking.available_permits(), 3);
                if drop_future {
                    drop(call);
                } else {
                    cancel.send(true).expect("cancel active host lease");
                    // The engine observes the host watch synchronously; do not poll the host
                    // future between cancellation and releasing the resolver.
                    assert!(
                        resolver
                            .control
                            .lock()
                            .expect("control")
                            .as_ref()
                            .expect("active")
                            .is_cancelled()
                    );
                    resolver.release.notify_one();
                    let reply = tokio::time::timeout(Duration::from_secs(5), call)
                        .await
                        .expect("terminal cancellation");
                    assert!(!reply.success);
                    assert_eq!(reply.value["code"], "cancelled");
                }
                assert!(
                    resolver
                        .control
                        .lock()
                        .expect("control")
                        .as_ref()
                        .expect("active")
                        .is_cancelled()
                );
                resolver.release.notify_one();
                wait_until(|| {
                    adapter.blocking.available_permits() == 4
                        && resolver.retired.load(Ordering::SeqCst) == 1
                })
                .await;
                assert_eq!(resolver.calls.load(Ordering::SeqCst), 1);
                assert_eq!(
                    server.requests.load(Ordering::SeqCst),
                    0,
                    "no HTTP probe after retirement"
                );
                assert!(engine.tasks().expect("tasks").is_empty(), "no task commit");
                assert!(
                    nxtools_download::SqliteDownloadStore::open(root.path().join("downloads.db"))
                        .expect("reopen store")
                        .load_tasks()
                        .expect("durable tasks")
                        .is_empty()
                );
            }
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn download_host_probe_receipt_enqueue_and_submit_resolve_once_and_preserve_causal_root()
    {
        for method in ["downloads.enqueue", "downloads.submit"] {
            let root = tempfile::tempdir().expect("owned DB");
            let server = LocalDownloadServer::start();
            let engine = DownloadEngine::open(root.path().join("downloads.db")).expect("engine");
            let resolver = adapter_resolver(&server, false, Duration::from_secs(600));
            engine
                .set_request_resolver(resolver.clone())
                .expect("inject counted resolver");
            let adapter = FeatureAdapters::new(engine.clone(), None, None);
            let (mut context, _cancel) = download_context();
            let request = local_request(&server, root.path());
            let probe = adapter
                .call(
                    &context,
                    "downloads.probe",
                    serde_json::json!({"request":request}),
                )
                .await;
            assert!(probe.success, "local probe succeeded");
            let receipt: nxtools_download::DownloadProbeResolution =
                serde_json::from_value(probe.value["resolution"].clone())
                    .expect("host-issued receipt");
            assert_eq!(
                receipt.provenance.operation_id,
                context.identity.correlation_id
            );
            // Each host call has a different correlation ID; receipt operation and causality
            // belong to the original probe and are explicitly retained across confirmation.
            context.identity.correlation_id = uuid::Uuid::new_v4();
            let submitted = adapter.call(&context, method, serde_json::json!({
                "request":request,
                "resolution":{"operation_id":receipt.provenance.operation_id,"mode":{"kind":"receipt","receipt":receipt.receipt}}
            })).await;
            assert!(submitted.success, "host receipt accepted");
            assert_eq!(resolver.calls.load(Ordering::SeqCst), 1);
            let tasks = engine.tasks().expect("accepted task");
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].task.request().source_url, server.url);
            let provenance = tasks[0].task.resolution().expect("durable provenance");
            assert_eq!(provenance.operation_id, receipt.provenance.operation_id);
            let causality = provenance
                .extension_causality
                .as_ref()
                .expect("host causality");
            assert_eq!(causality.root_id, context.causality.root_id);
            assert_eq!(causality.handlers, context.causality.handlers);
            assert_eq!(causality.origin_plugin, context.identity.package_id);
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn download_host_invalid_expired_and_revoked_receipts_fail_without_reresolution() {
        for invalid in ["token", "operation", "request", "expired", "revoked"] {
            let root = tempfile::tempdir().expect("owned DB");
            let server = LocalDownloadServer::start();
            let engine = DownloadEngine::open(root.path().join("downloads.db")).expect("engine");
            let validity = if invalid == "expired" {
                Duration::from_secs(1)
            } else {
                Duration::from_secs(600)
            };
            let resolver = adapter_resolver(&server, false, validity);
            engine
                .set_request_resolver(resolver.clone())
                .expect("inject");
            let adapter = FeatureAdapters::new(engine.clone(), None, None);
            let (context, _cancel) = download_context();
            let mut request = local_request(&server, root.path());
            let probe = adapter
                .call(
                    &context,
                    "downloads.probe",
                    serde_json::json!({"request":request}),
                )
                .await;
            assert!(probe.success, "receipt fixture probe succeeded");
            let receipt: nxtools_download::DownloadProbeResolution =
                serde_json::from_value(probe.value["resolution"].clone()).expect("receipt");
            let mut token = receipt.receipt;
            let mut operation = receipt.provenance.operation_id;
            match invalid {
                "token" => token = uuid::Uuid::new_v4(),
                "operation" => operation = uuid::Uuid::new_v4(),
                "request" => request.source_url.push_str("-changed"),
                "expired" => tokio::time::sleep(Duration::from_millis(1100)).await,
                "revoked" => engine
                    .set_request_resolver(Arc::new(nxtools_download::NoopDownloadRequestResolver))
                    .expect("revoke"),
                _ => unreachable!(),
            }
            let network_requests = server.requests.load(Ordering::SeqCst);
            for method in ["downloads.enqueue", "downloads.submit"] {
                let reply = adapter.call(&context, method, serde_json::json!({
                    "request":request,
                    "resolution":{"operation_id":operation,"mode":{"kind":"receipt","receipt":token}}
                })).await;
                assert!(!reply.success, "invalid {invalid}");
                assert_eq!(
                    reply.value["code"], "resolution_required",
                    "invalid {invalid}"
                );
            }
            assert_eq!(resolver.calls.load(Ordering::SeqCst), 1);
            assert_eq!(server.requests.load(Ordering::SeqCst), network_requests);
            assert!(engine.tasks().expect("tasks").is_empty());
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn download_host_resolution_input_is_strict_and_legacy_defaults_still_resolve() {
        let root = tempfile::tempdir().expect("owned DB");
        let server = LocalDownloadServer::start();
        let engine = DownloadEngine::open(root.path().join("downloads.db")).expect("engine");
        let resolver = adapter_resolver(&server, false, Duration::from_secs(600));
        engine
            .set_request_resolver(resolver.clone())
            .expect("inject");
        let adapter = FeatureAdapters::new(engine.clone(), None, None);
        let (context, _cancel) = download_context();
        let request = local_request(&server, root.path());
        for resolution in [
            serde_json::json!({"mode":{"kind":"bypass"}}),
            serde_json::json!({"operation_id":uuid::Uuid::new_v4(),"effective_request":request}),
            serde_json::json!({"mode":{"kind":"receipt","receipt":uuid::Uuid::new_v4()}}),
            serde_json::json!({"operation_id":uuid::Uuid::nil()}),
            serde_json::json!({"operation_id":uuid::Uuid::new_v4(),"mode":{"kind":"receipt","receipt":uuid::Uuid::nil()}}),
            serde_json::json!({"mode":{"kind":"resolve","unexpected":true}}),
        ] {
            let reply = adapter
                .call(
                    &context,
                    "downloads.enqueue",
                    serde_json::json!({"request":request,"resolution":resolution}),
                )
                .await;
            assert!(!reply.success);
            assert_eq!(reply.value["code"], "invalid_params");
        }
        assert_eq!(resolver.calls.load(Ordering::SeqCst), 0);
        assert!(engine.tasks().expect("tasks").is_empty());
        let accepted = adapter
            .call(
                &context,
                "downloads.enqueue",
                serde_json::json!({"request":request}),
            )
            .await;
        assert!(accepted.success);
        assert_eq!(resolver.calls.load(Ordering::SeqCst), 1);
        let empty = serde_json::from_value::<Submit>(
            serde_json::json!({"request":request,"resolution":{}}),
        )
        .expect("optional options defaults");
        assert_eq!(
            empty
                .resolution_options(context.identity.correlation_id)
                .expect("defaults"),
            nxtools_download::DownloadResolutionOptions {
                operation_id: context.identity.correlation_id,
                mode: nxtools_download::DownloadResolutionMode::Resolve,
            }
        );
    }
    #[tokio::test(flavor = "current_thread")]
    async fn actual_feature_host_calls_share_engine_and_clipboard_worker_and_headless_is_explicit()
    {
        let root = tempfile::tempdir().expect("owned feature fixture");
        let engine = DownloadEngine::open(root.path().join("downloads.db")).expect("engine");
        let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
            root.path().join("clipboard.db"),
            root.path().join("assets"),
            nxtools_clipboard::ClipboardRetentionPolicy {
                max_items: 1000,
                retention_days: 30,
            },
            nxtools_ipc::unix_time_ms(),
        )
        .expect("worker");
        let adapter = FeatureAdapters::new(
            engine.clone(),
            Some(ClipboardExtensionAdapter::new(worker.clone())),
            None,
        );
        let (_cancel, cancellation) = tokio::sync::watch::channel(false);
        let root_id = uuid::Uuid::new_v4();
        let identity = nxtools_extensions::InvocationIdentity {
            run_id: uuid::Uuid::new_v4(),
            correlation_id: root_id,
            generation: 1,
            package_id: "fixture.host".into(),
            handler_id: "action".into(),
            content_hash: "a".repeat(64),
        };
        let context = HostContext {
            identity,
            feature: "tools".into(),
            entry: "action".into(),
            causality: nxtools_extensions::Causality::root(root_id)
                .enter("fixture.host", "action")
                .expect("causality"),
            cancellation,
            blobs: None,
        };
        let input = nxtools_download::DownloadRequest {
            source_url: "https://fixture.test/file".into(),
            save_directory: root.path().to_owned(),
            save_directory_explicit: true,
            file_name: Some("file.bin".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            certificate_policy: Default::default(),
            connection_limit: Some(1),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
        };
        let submitted = adapter
            .call(
                &context,
                "downloads.enqueue",
                serde_json::json!({"request":input}),
            )
            .await;
        assert!(submitted.success);
        assert_eq!(engine.tasks().expect("accepted owner tasks").len(), 1);
        let listed = adapter
            .call(&context, "downloads.list", serde_json::json!({"limit":32}))
            .await;
        assert!(listed.success);
        assert_eq!(listed.value.as_array().expect("tasks").len(), 1);
        assert_eq!(
            adapter
                .call(
                    &context,
                    "common.open",
                    serde_json::json!({"target":"fixture"})
                )
                .await
                .value["code"],
            "ui_unavailable"
        );
        let source = worker
            .capture_committed_async(
                nxtools_protocol::ClipboardCapture {
                    text: Some("original fixture".into()),
                    paths: Vec::new(),
                    image: None,
                    native_formats: Vec::new(),
                    source_icon_staging_path: None,
                    source: nxtools_protocol::ForegroundContext {
                        window_handle: 0,
                        focus_window_handle: 0,
                        process_id: 0,
                        process_name: String::new(),
                        window_title: String::new(),
                    },
                    captured_at_unix_ms: nxtools_ipc::unix_time_ms(),
                },
                ClipboardCommitCausality::capture(),
            )
            .await
            .expect("persist original")
            .expect("commit");
        adapter.manual_effects.begin(
            context.identity.correlation_id,
            &context.identity.package_id,
            &context.identity.handler_id,
            &context.identity.content_hash,
        );
        let derived = adapter
            .call(
                &context,
                "clipboard.create_derived",
                serde_json::json!({"source_item_id":source.item_id,"text":"derived fixture"}),
            )
            .await;
        assert!(derived.success);
        assert_eq!(
            adapter
                .manual_effects
                .items(context.identity.correlation_id),
            vec![
                derived.value["item_id"]
                    .as_i64()
                    .expect("real committed ID")
            ]
        );
        let original = adapter
            .call(
                &context,
                "clipboard.get",
                serde_json::json!({"item_id":source.item_id}),
            )
            .await;
        assert_eq!(original.value["text"], "original fixture");
        let queried = adapter
            .call(
                &context,
                "clipboard.query",
                serde_json::json!({"query":"fixture","limit":32}),
            )
            .await;
        assert_eq!(queried.value.as_array().expect("history rows").len(), 2);
        assert_eq!(
            adapter
                .call(&context, "arbitrary.reflect", serde_json::json!({}))
                .await
                .value["code"],
            "unsupported"
        );
        worker.shutdown().expect("owned shutdown");
    }
}
