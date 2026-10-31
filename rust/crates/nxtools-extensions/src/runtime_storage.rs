//! Exact, host-issued metadata binding; ordinary registry publication remains unchanged.
use super::*;

#[derive(Clone)]
pub(super) struct RunPackage(Arc<Mutex<InstalledPackage>>, Arc<Semaphore>);
impl RunPackage {
    pub(super) fn new(package: InstalledPackage) -> Self {
        Self(Arc::new(Mutex::new(package)), Arc::new(Semaphore::new(1)))
    }
    pub(super) fn snapshot(&self) -> Result<InstalledPackage, RuntimeError> {
        self.0
            .lock()
            .map(|package| package.clone())
            .map_err(|_| RuntimeError::NativeUnavailable)
    }
}
impl ExtensionRuntime {
    pub(super) fn register_run(
        &self,
        run_id: Uuid,
        snapshot: RunPackage,
        cancel: watch::Sender<bool>,
    ) -> Result<(), RuntimeError> {
        let registry = self
            .0
            .registry
            .read()
            .map_err(|_| RuntimeError::NativeUnavailable)?;
        let package = snapshot.snapshot()?;
        if !package.enabled
            || registry
                .packages
                .iter()
                .find(|current| current.manifest.id == package.manifest.id)
                != Some(&package)
        {
            return Err(RuntimeError::Stale);
        }
        self.0
            .active
            .lock()
            .map_err(|_| RuntimeError::NativeUnavailable)?
            .insert(
                run_id,
                ActiveRun {
                    package: package.manifest.id,
                    cancel,
                    snapshot: Some(snapshot),
                },
            );
        Ok(())
    }
    pub(super) fn registered_run(&self, run: &RunPackage) -> Result<(), RuntimeError> {
        // Same ordering as publication/binding: registry -> active -> run snapshot.
        let registry = self
            .0
            .registry
            .read()
            .map_err(|_| RuntimeError::NativeUnavailable)?;
        let package = run.snapshot()?;
        if !package.enabled
            || registry
                .packages
                .iter()
                .find(|current| current.manifest.id == package.manifest.id)
                != Some(&package)
        {
            return Err(RuntimeError::Stale);
        }
        let selected = registry
            .runtime_executable(&package.manifest.id)
            .map_err(|_| RuntimeError::Untrusted)?;
        if package.trust.as_ref().is_none_or(|trust| {
            !crate::registry::same_runtime_selection(
                selected.as_deref(),
                Some(&trust.runtime.executable),
            )
        }) {
            return Err(RuntimeError::Untrusted);
        }
        Ok(())
    }
    pub(super) async fn still_run(&self, run: &RunPackage) -> Result<(), RuntimeError> {
        self.registered_run(run)?;
        let package = run.snapshot()?;
        let registry = self.snapshot()?;
        let selected = registry
            .runtime_executable(&package.manifest.id)
            .map_err(|_| RuntimeError::Untrusted)?;
        let permit = self
            .0
            .verification
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| RuntimeError::NativeUnavailable)?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            package
                .verify_approval()
                .map_err(|_| RuntimeError::Untrusted)?;
            registry
                .verify_runtime_selection(&package.manifest.id)
                .map_err(|_| RuntimeError::Untrusted)
        })
        .await
        .map_err(|_| RuntimeError::NativeUnavailable)??;
        self.registered_run(run)?;
        let current = run.snapshot()?;
        if self
            .snapshot()?
            .runtime_executable(&current.manifest.id)
            .map_err(|_| RuntimeError::Untrusted)?
            != selected
        {
            return Err(RuntimeError::Stale);
        }
        Ok(())
    }
    pub(super) fn apply_storage_binding(
        &self,
        reviewed: &InstalledPackage,
        bound: &InstalledPackage,
    ) -> Result<(), RuntimeError> {
        let Some(namespace) = bound.data_uuid.filter(|id| !id.is_nil()) else {
            return Err(RuntimeError::Stale);
        };
        if reviewed
            .data_uuid
            .is_some_and(|previous| previous != namespace)
        {
            return Err(RuntimeError::Stale);
        }
        let mut expected = reviewed.clone();
        expected.data_uuid = Some(namespace);
        if &expected != bound {
            return Err(RuntimeError::Stale);
        }
        let mut registry = self
            .0
            .registry
            .write()
            .map_err(|_| RuntimeError::NativeUnavailable)?;
        let active = self
            .0
            .active
            .lock()
            .map_err(|_| RuntimeError::NativeUnavailable)?;
        let current = registry
            .packages
            .iter_mut()
            .find(|package| package.manifest.id == reviewed.manifest.id)
            .ok_or(RuntimeError::Stale)?;
        if current != reviewed && current != bound {
            return Err(RuntimeError::Stale);
        }
        // Only this known durable None->UUID commit can update old exact invocation snapshots.
        // General publication does not use this path and still cancels on any package change.
        for run in active
            .values()
            .filter(|run| run.package == reviewed.manifest.id)
        {
            if let Some(snapshot) = &run.snapshot {
                let mut package = snapshot
                    .0
                    .lock()
                    .map_err(|_| RuntimeError::NativeUnavailable)?;
                if &*package == reviewed {
                    *package = bound.clone();
                }
            }
        }
        *current = bound.clone();
        Ok(())
    }
    pub(super) async fn storage_call(
        &self,
        run: &RunPackage,
        context: &HostContext,
        method: &str,
        params: Value,
    ) -> HostReply {
        if context.blobs.is_none() {
            return HostReply::error("unsupported_transport");
        }
        let call = match crate::storage_calls::Call::parse(method, params) {
            Ok(call) => call,
            Err(reply) => return reply,
        };
        let Some(host) = self.0.storage.clone() else {
            return HostReply::error("capability_unavailable");
        };
        let Ok(permit) = self.0.storage_binding_slots.clone().try_acquire_owned() else {
            return HostReply::error("resource_limit");
        };
        let runtime = self.clone();
        let run = run.clone();
        let cancellation = context.cancellation.clone();
        let identity = context.identity.clone();
        let namespace_cancellation = cancellation.clone();
        // A detached bounded task owns the real blocking transaction through completion.
        // Caller cancellation cannot release its permit or strand a successful UUID commit.
        let binding: tokio::task::JoinHandle<
            Result<
                Result<crate::storage::StorageNamespace, crate::storage::StorageError>,
                RuntimeError,
            >,
        > = tokio::spawn(async move {
            let serial = run
                .1
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| RuntimeError::NativeUnavailable)?;
            runtime.still_run(&run).await?;
            let reviewed = run.snapshot()?;
            let owner = host.owner();
            tokio::task::spawn_blocking(move || {
                let _permit = permit;
                let _serial = serial;
                let started = Instant::now();
                let span = tracing::info_span!("extension.storage.binding", correlation_id=%identity.correlation_id, run_id=%identity.run_id);
                let _entered = span.enter();
                tracing::info!(event="extension.storage.binding_started");
                let bound = match host.bind_and_apply(&reviewed, &|| *cancellation.borrow(), &|bound| {
                    runtime.apply_storage_binding(&reviewed, bound)
                        .map_err(|_| crate::storage::StorageError::Stale)
                }) {
                    Ok(bound) => bound,
                    Err(error) => {
                        tracing::info!(event="extension.storage.binding_terminal", reason=error.code(), elapsed_ms=started.elapsed().as_millis() as u64);
                        return Ok(Err(error));
                    },
                };
                tracing::info!(event="extension.storage.binding_terminal", reason="success", elapsed_ms=started.elapsed().as_millis() as u64);
                Ok(Ok(owner.namespace(
                    bound.data_uuid.ok_or(RuntimeError::Stale)?,
                    identity,
                    namespace_cancellation,
                )))
            })
            .await
            .map_err(|_| RuntimeError::NativeUnavailable)?
        });
        match binding.await {
            Ok(Ok(Ok(namespace))) => call.execute(namespace).await,
            Ok(Ok(Err(error))) => HostReply::error(error.code()),
            _ => HostReply::error("storage_stale"),
        }
    }
}
