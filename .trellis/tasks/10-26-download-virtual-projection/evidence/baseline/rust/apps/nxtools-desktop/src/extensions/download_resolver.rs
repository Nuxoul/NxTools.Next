//! Shared GUI/headless Adapter. DownloadEngine retains request/persistence/HTTP ownership.
use nxtools_download::{
    DownloadRequest, DownloadRequestResolver, DownloadResolutionError, DownloadResolutionFuture,
    DownloadResolutionOperation, DownloadResolutionResult, DownloadResolvedContext,
    DownloadResolvedRequest, DownloadResolverIdentity, DownloadResolverStage,
};
use nxtools_extensions::{
    Causality, ExtensionRuntime, HandlerReference, InvokeOutcome, InvokeRequest, PackageRegistry,
    ResolverCycleGuard, RuntimeError,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio::sync::watch;

pub(crate) type HandlerFuture<'a> =
    Pin<Box<dyn Future<Output = Result<InvokeOutcome, RuntimeError>> + Send + 'a>>;
pub(crate) trait ResolverInvoker: Send + Sync {
    fn invoke<'a>(&'a self, request: InvokeRequest) -> HandlerFuture<'a>;
}
impl ResolverInvoker for ExtensionRuntime {
    fn invoke<'a>(&'a self, request: InvokeRequest) -> HandlerFuture<'a> {
        Box::pin(ExtensionRuntime::invoke(self, request))
    }
}
#[derive(Clone)]
pub(crate) struct ExtensionDownloadResolver {
    registry: PackageRegistry,
    identity: DownloadResolverIdentity,
    invoker: Arc<dyn ResolverInvoker>,
}
impl ExtensionDownloadResolver {
    pub(crate) fn new(registry: PackageRegistry, invoker: Arc<dyn ResolverInvoker>) -> Self {
        let content = registry
            .packages
            .iter()
            .map(|package| {
                (
                    &package.manifest.id,
                    &package.content_hash,
                    package.enabled,
                    &package.trust,
                )
            })
            .collect::<Vec<_>>();
        let settings = registry
            .packages
            .iter()
            .map(|package| (&package.manifest.id, &package.settings))
            .collect::<Vec<_>>();
        let content_version = digest(serde_json::to_vec(&content).unwrap_or_default());
        let settings_version = digest(
            serde_json::to_vec(&(
                settings,
                &registry.resolver_preferences,
                &registry.resolver_chains,
            ))
            .unwrap_or_default(),
        );
        Self {
            registry,
            identity: DownloadResolverIdentity {
                content_version,
                settings_version,
            },
            invoker,
        }
    }
    async fn resolve_chain(
        &self,
        original: &DownloadRequest,
        operation: &DownloadResolutionOperation,
    ) -> Result<DownloadResolutionResult, DownloadResolutionError> {
        operation.check()?;
        let original_url = url::Url::parse(&original.source_url)
            .map_err(|_| DownloadResolutionError::InvalidResult)?;
        let route = self
            .registry
            .resolver_route(&original_url)
            .map_err(|_| DownloadResolutionError::Rejected)?;
        if route.is_empty() {
            return Ok(DownloadResolutionResult::NotHandled);
        }
        let mut effective = original.clone();
        let mut cycle = ResolverCycleGuard::new(&original_url);
        let mut stages = Vec::new();
        let mut validity = Duration::from_secs(600);
        let mut handled = false;
        for (stage_index, reference) in route.into_iter().enumerate() {
            operation.check()?;
            if operation
                .control
                .extension_causality()
                .is_some_and(|origin| {
                    origin.origin_plugin == reference.package_id
                        || origin.handlers.contains(&format!(
                            "{}/{}",
                            reference.package_id, reference.handler_id
                        ))
                })
            {
                return Err(DownloadResolutionError::Rejected);
            }
            let package = self
                .registry
                .packages
                .iter()
                .find(|package| package.manifest.id == reference.package_id)
                .ok_or(DownloadResolutionError::IdentityChanged)?;
            let request = self.stage_request(
                reference.clone(),
                original,
                &effective,
                operation,
                stage_index,
            )?;
            let (_cancel, cancellation) = watch::channel(false);
            let request = InvokeRequest {
                cancellation,
                ..request
            };
            let remaining = operation
                .deadline
                .saturating_duration_since(std::time::Instant::now())
                .min(Duration::from_secs(30));
            let outcome = tokio::time::timeout(remaining, self.invoker.invoke(request))
                .await
                .map_err(|_| DownloadResolutionError::Deadline)?
                .map_err(|error| match error {
                    RuntimeError::Cancelled => DownloadResolutionError::Cancelled,
                    RuntimeError::Timeout => DownloadResolutionError::Deadline,
                    RuntimeError::Untrusted | RuntimeError::Stale => {
                        DownloadResolutionError::IdentityChanged
                    }
                    _ => DownloadResolutionError::Rejected,
                })?;
            operation.check()?;
            if outcome.identity.content_hash != package.content_hash
                || outcome.identity.package_id != reference.package_id
                || outcome.identity.handler_id != reference.handler_id
            {
                return Err(DownloadResolutionError::IdentityChanged);
            }
            if outcome.status == "not_handled" {
                continue;
            }
            if !matches!(outcome.status.as_str(), "handled" | "success") {
                return Err(DownloadResolutionError::Rejected);
            }
            let output: ResolverOutput = serde_json::from_value(outcome.value)
                .map_err(|_| DownloadResolutionError::InvalidResult)?;
            let url =
                url::Url::parse(&output.url).map_err(|_| DownloadResolutionError::InvalidResult)?;
            cycle
                .accept(&url)
                .map_err(|_| DownloadResolutionError::Rejected)?;
            let provenance = DownloadResolverStage {
                package_id: reference.package_id,
                handler_id: reference.handler_id,
                content_version: package.content_hash.clone(),
            };
            let result = DownloadResolvedRequest {
                source_url: output.url,
                file_name: output.file_name,
                context: output.context,
                media: None,
                provenance: vec![provenance.clone()],
                valid_for: output.valid_for_ms.map(Duration::from_millis),
            };
            let (next, _, stage_validity) = result.apply_to(effective)?;
            effective = next;
            stages.push(provenance);
            validity = validity.min(stage_validity);
            handled = true;
            tracing::info!(event="extension.resolve.stage.completed", correlation_id=%operation.correlation_id, operation_id=%operation.operation_id, stage=stage_index, "extension resolver stage completed");
        }
        if !handled {
            return Ok(DownloadResolutionResult::NotHandled);
        }
        Ok(DownloadResolutionResult::Resolved(Box::new(
            DownloadResolvedRequest {
                source_url: effective.source_url,
                file_name: effective.file_name,
                context: Some(DownloadResolvedContext {
                    headers: effective.headers,
                    cookie: effective.cookie,
                    referrer: effective.referrer,
                }),
                media: effective.media,
                provenance: stages,
                valid_for: Some(validity),
            },
        )))
    }
    fn stage_request(
        &self,
        reference: HandlerReference,
        original: &DownloadRequest,
        effective: &DownloadRequest,
        operation: &DownloadResolutionOperation,
        stage: usize,
    ) -> Result<InvokeRequest, DownloadResolutionError> {
        let context = serde_json::json!({"original_url":original.source_url,"request":effective,"stage":stage});
        if serde_json::to_vec(&context).map_or(true, |bytes| bytes.len() > 128 * 1024) {
            return Err(DownloadResolutionError::InvalidResult);
        }
        let (_cancel, cancellation) = watch::channel(false);
        let causality = operation.control.extension_causality().map_or_else(
            || Causality::root(operation.operation_id),
            |origin| Causality {
                root_id: origin.root_id,
                origin_plugin: Some(origin.origin_plugin.clone()),
                handlers: origin.handlers.clone(),
            },
        );
        Ok(InvokeRequest {
            package_id: reference.package_id,
            handler_id: reference.handler_id,
            correlation_id: operation.correlation_id,
            generation: operation.session_generation,
            context,
            causality,
            cancellation,
            batches: None,
        })
    }
}
impl DownloadRequestResolver for ExtensionDownloadResolver {
    fn identity(&self) -> DownloadResolverIdentity {
        self.identity.clone()
    }
    fn resolve<'a>(
        &'a self,
        request: &'a DownloadRequest,
        operation: &'a DownloadResolutionOperation,
    ) -> DownloadResolutionFuture<'a> {
        Box::pin(self.resolve_chain(request, operation))
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolverOutput {
    url: String,
    #[serde(default)]
    file_name: Option<String>,
    #[serde(default)]
    context: Option<DownloadResolvedContext>,
    #[serde(default)]
    valid_for_ms: Option<u64>,
}
fn digest(bytes: Vec<u8>) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nxtools_download::{
        DownloadEngine, DownloadResolutionControl, DownloadResolutionOptions, DownloadSubmitMode,
        RequestOrigin,
    };
    use nxtools_extensions::{ImportPreview, InvocationIdentity, RuntimeIdentity};
    use std::sync::Mutex;
    use uuid::Uuid;
    struct Invoker {
        registry: PackageRegistry,
        outputs: Mutex<std::collections::VecDeque<Result<serde_json::Value, RuntimeError>>>,
        inputs: Mutex<Vec<serde_json::Value>>,
    }
    impl ResolverInvoker for Invoker {
        fn invoke<'a>(&'a self, request: InvokeRequest) -> HandlerFuture<'a> {
            Box::pin(async move {
                self.inputs.lock().expect("inputs").push(request.context);
                let value = self
                    .outputs
                    .lock()
                    .expect("outputs")
                    .pop_front()
                    .expect("stage output")?;
                let package = self
                    .registry
                    .packages
                    .iter()
                    .find(|package| package.manifest.id == request.package_id)
                    .expect("package");
                Ok(InvokeOutcome {
                    identity: InvocationIdentity {
                        run_id: Uuid::new_v4(),
                        correlation_id: request.correlation_id,
                        generation: request.generation,
                        package_id: request.package_id,
                        handler_id: request.handler_id,
                        content_hash: package.content_hash.clone(),
                    },
                    status: "handled".into(),
                    value,
                })
            })
        }
    }
    fn registry(root: &std::path::Path) -> PackageRegistry {
        let mut registry = PackageRegistry::default();
        let runtime_file = root.join("runtime.exe");
        std::fs::write(&runtime_file, b"identity fixture; never executed")
            .expect("runtime identity");
        for name in ["first", "second"] {
            let source = root.join(name);
            std::fs::create_dir(&source).expect("source");
            let manifest = serde_json::json!({"schema_version":1,"id":format!("fixture.{name}"),"name":name,"version":"1","runtime":{"program":"python"},"files":["entry.py"],"handlers":[{"id":"resolve","title":"Resolve","feature":"downloads","entry":"resolve","url_matches":[{"host":"original.test","path":"*"}],"action":{"type":"script","entry_file":"entry.py"}}]});
            std::fs::write(
                source.join("extension.json"),
                serde_json::to_vec(&manifest).expect("manifest"),
            )
            .expect("write declaration");
            std::fs::write(source.join("entry.py"), b"owned fixture").expect("code");
            let preview = ImportPreview::from_directory(&source)
                .expect("preview")
                .stage_into(&root.join("packages"))
                .expect("stage");
            let hash = preview.content_hash.clone();
            registry
                .install(preview, "fixture".into(), false)
                .expect("install");
            registry
                .approve(
                    &format!("fixture.{name}"),
                    &hash,
                    RuntimeIdentity::inspect(&runtime_file).expect("runtime"),
                )
                .expect("approve");
        }
        for package in &mut registry.packages {
            package.enabled = true;
        } // Fixture-only gate bypass, never saved.
        registry
    }
    fn request(root: &std::path::Path) -> DownloadRequest {
        DownloadRequest {
            source_url: "https://original.test/page".into(),
            save_directory: root.to_owned(),
            save_directory_explicit: true,
            file_name: Some("chosen.bin".into()),
            headers: vec![nxtools_download::DownloadRequestHeader {
                name: "Authorization".into(),
                value: "private fixture".into(),
            }],
            cookie: Some("private fixture".into()),
            referrer: Some("https://original.test/private".into()),
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
        }
    }
    fn route() -> Vec<HandlerReference> {
        ["first", "second"]
            .into_iter()
            .map(|name| HandlerReference {
                package_id: format!("fixture.{name}"),
                handler_id: "resolve".into(),
            })
            .collect()
    }
    #[test]
    fn real_engine_chain_cleans_context_between_stages_and_preserves_original_provenance() {
        let root = tempfile::tempdir().expect("fixture");
        let mut registry = registry(root.path());
        assert!(
            registry
                .resolver_route(&url::Url::parse("https://original.test/page").expect("URL"))
                .is_err(),
            "tie is not arbitrary handler choice"
        );
        registry
            .resolver_chains
            .insert("original.test".into(), route());
        let invoker = Arc::new(Invoker {
            registry: registry.clone(),
            outputs: Mutex::new(std::collections::VecDeque::from([
                Ok(serde_json::json!({"url":"https://next.test/middle"})),
                Ok(
                    serde_json::json!({"url":"https://final.test/file","file_name":"suggested.bin"}),
                ),
            ])),
            inputs: Mutex::new(Vec::new()),
        });
        let resolver = ExtensionDownloadResolver::new(registry, invoker.clone());
        let engine = DownloadEngine::open(root.path().join("downloads.db")).expect("engine");
        engine
            .set_request_resolver(Arc::new(resolver))
            .expect("inject");
        let input = request(root.path());
        let (id, _) = engine
            .submit_with_resolution(
                input.clone(),
                RequestOrigin::Ui,
                DownloadSubmitMode::QueueOnly,
                None,
                DownloadResolutionOptions::default(),
                DownloadResolutionControl::default(),
            )
            .expect("common acceptance");
        let tasks = engine.tasks().expect("tasks");
        let task = tasks
            .iter()
            .find(|task| task.task.id() == id)
            .expect("accepted task");
        assert_eq!(task.task.request().source_url, "https://final.test/file");
        assert_eq!(task.task.request().file_name.as_deref(), Some("chosen.bin"));
        assert!(task.task.request().headers.is_empty());
        assert!(task.task.request().cookie.is_none());
        assert_eq!(
            task.task
                .resolution()
                .expect("durable provenance")
                .original_request,
            input
        );
        assert_eq!(task.task.resolution().expect("provenance").stages.len(), 2);
        let inputs = invoker.inputs.lock().expect("inputs");
        assert_eq!(
            inputs[1]["request"]["source_url"],
            "https://next.test/middle"
        );
        assert_eq!(inputs[1]["request"]["headers"], serde_json::json!([]));
        assert!(inputs[1]["request"]["cookie"].is_null());
    }
    #[test]
    fn real_engine_chain_stops_on_error_or_cycle_without_creating_task() {
        for fail in [true, false] {
            let root = tempfile::tempdir().expect("fixture");
            let mut registry = registry(root.path());
            registry
                .resolver_chains
                .insert("original.test".into(), route());
            let outputs = if fail {
                vec![Err(RuntimeError::HandlerFailed)]
            } else {
                vec![
                    Ok(serde_json::json!({"url":"https://next.test/middle"})),
                    Ok(serde_json::json!({"url":"https://original.test/page"})),
                ]
            };
            let invoker = Arc::new(Invoker {
                registry: registry.clone(),
                outputs: Mutex::new(outputs.into()),
                inputs: Mutex::new(Vec::new()),
            });
            let engine = DownloadEngine::open(root.path().join("downloads.db")).expect("engine");
            engine
                .set_request_resolver(Arc::new(ExtensionDownloadResolver::new(
                    registry,
                    invoker.clone(),
                )))
                .expect("inject");
            assert!(
                engine
                    .submit_with_resolution(
                        request(root.path()),
                        RequestOrigin::Ui,
                        DownloadSubmitMode::QueueOnly,
                        None,
                        DownloadResolutionOptions::default(),
                        DownloadResolutionControl::default()
                    )
                    .is_err()
            );
            assert!(engine.tasks().expect("tasks").is_empty());
            assert_eq!(
                invoker.inputs.lock().expect("inputs").len(),
                if fail { 1 } else { 2 }
            );
        }
    }
}
