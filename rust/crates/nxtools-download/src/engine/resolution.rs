//! One common pre-network/pre-lock ingress for probe, direct and browser acceptance.
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tracing::info;
use uuid::Uuid;

use super::{DownloadEngine, DownloadEngineError, now_unix_ms};
use crate::resolution::{apply_resolution, validate_identity};
use crate::{
    DownloadProbe, DownloadProbeResolution, DownloadRequest, DownloadRequestResolver,
    DownloadResolutionControl, DownloadResolutionError, DownloadResolutionMode,
    DownloadResolutionOperation, DownloadResolutionOptions, DownloadResolutionProvenance,
    DownloadResolutionResult, DownloadResolverIdentity, MAX_CONCURRENT_RESOLUTIONS,
    MAX_RESOLUTION_RECEIPTS, NoopDownloadRequestResolver, RESOLUTION_RECEIPT_TTL,
};

pub(super) struct ResolutionState {
    resolver: Arc<dyn DownloadRequestResolver>,
    generation: u64,
    receipts: VecDeque<Receipt>,
    active: HashMap<Uuid, DownloadResolutionControl>,
}
impl Default for ResolutionState {
    fn default() -> Self {
        Self {
            resolver: Arc::new(NoopDownloadRequestResolver),
            generation: 0,
            receipts: VecDeque::new(),
            active: HashMap::new(),
        }
    }
}
#[derive(Clone)]
struct Receipt {
    token: Uuid,
    original_fingerprint: String,
    effective: DownloadRequest,
    provenance: DownloadResolutionProvenance,
    expires: Instant,
    expires_at_unix_ms: i64,
    generation: u64,
}

pub(super) struct PreparedResolution {
    pub request: DownloadRequest,
    pub provenance: Option<DownloadResolutionProvenance>,
    resolver: Arc<dyn DownloadRequestResolver>,
    identity: DownloadResolverIdentity,
    generation: u64,
    expires: Option<Instant>,
    token: Option<Uuid>,
    expires_at_unix_ms: i64,
    original_fingerprint: Option<String>,
    checked_identity: bool,
    pub operation: DownloadResolutionOperation,
}
struct ResolutionSlot(Arc<super::DownloadEngineInner>, Uuid);
impl Drop for ResolutionSlot {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.resolution.lock() {
            state.active.remove(&self.1);
        }
    }
}

impl DownloadEngine {
    /// Inject at GUI/background composition. Replacing/revoking an adapter invalidates every
    /// pre-acceptance receipt and in-flight result; accepted tasks retain their effective context.
    pub fn set_request_resolver(
        &self,
        resolver: Arc<dyn DownloadRequestResolver>,
    ) -> Result<(), DownloadEngineError> {
        if !resolver.is_noop() {
            validate_identity(&resolver.identity())?;
        }
        let mut state = self
            .inner
            .resolution
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        state.generation = state.generation.wrapping_add(1);
        for control in state.active.values() {
            control.cancel();
        }
        let previous = std::mem::replace(&mut state.resolver, resolver);
        state.receipts.clear();
        drop(state);
        drop(previous);
        Ok(())
    }

    pub fn probe_with_resolution(
        &self,
        request: DownloadRequest,
        options: DownloadResolutionOptions,
        control: DownloadResolutionControl,
    ) -> Result<DownloadProbe, DownloadEngineError> {
        let correlation_id = options.operation_id;
        self.probe_resolved_correlated(request, options, control, correlation_id)
    }

    pub(super) fn probe_resolved_correlated(
        &self,
        request: DownloadRequest,
        options: DownloadResolutionOptions,
        control: DownloadResolutionControl,
        correlation_id: Uuid,
    ) -> Result<DownloadProbe, DownloadEngineError> {
        let prepared =
            self.prepare_request_resolution(request, options, control, correlation_id)?;
        let probe = self.probe_effective_blocking(
            prepared.request.clone(),
            correlation_id,
            Some(prepared.operation.clone()),
        )?;
        self.check_request_resolution(&prepared)?;
        let mut probe = probe;
        probe.resolution = self.publish_resolution_receipt(&prepared)?.map(Box::new);
        Ok(probe)
    }

    pub(super) fn prepare_request_resolution(
        &self,
        original: DownloadRequest,
        options: DownloadResolutionOptions,
        control: DownloadResolutionControl,
        correlation_id: Uuid,
    ) -> Result<PreparedResolution, DownloadEngineError> {
        let operation = DownloadResolutionOperation {
            operation_id: options.operation_id,
            correlation_id,
            session_generation: self.session_generation(),
            deadline: Instant::now() + control.timeout(),
            control,
        };
        operation.check()?;
        let (resolver, generation) = {
            let state = self
                .inner
                .resolution
                .lock()
                .map_err(|_| DownloadEngineError::LockPoisoned)?;
            (Arc::clone(&state.resolver), state.generation)
        };
        let identity = resolver.identity();
        let mut prepared = PreparedResolution {
            request: original.clone(),
            provenance: None,
            operation,
            resolver: Arc::clone(&resolver),
            identity: identity.clone(),
            generation,
            expires: None,
            token: None,
            expires_at_unix_ms: 0,
            original_fingerprint: None,
            checked_identity: false,
        };
        if let DownloadResolutionMode::Receipt(token) = options.mode {
            original.validate()?;
            let fingerprint = resolution_resource_fingerprint(&original)?;
            let receipt = {
                let mut state = self
                    .inner
                    .resolution
                    .lock()
                    .map_err(|_| DownloadEngineError::LockPoisoned)?;
                state
                    .receipts
                    .retain(|receipt| receipt.expires > Instant::now());
                state
                    .receipts
                    .iter()
                    .find(|receipt| receipt.token == token)
                    .cloned()
            }
            .ok_or(DownloadResolutionError::ResolutionRequired)?;
            if receipt.original_fingerprint != fingerprint
                || receipt.generation != generation
                || receipt.provenance.resolver != identity
                || receipt.provenance.operation_id != prepared.operation.operation_id
                || receipt.provenance.session_generation != self.session_generation()
            {
                return Err(DownloadResolutionError::ResolutionRequired.into());
            }
            prepared.request = receipt.effective;
            prepared.request.save_directory = original.save_directory.clone();
            prepared.request.save_directory_explicit = original.save_directory_explicit;
            if original.file_name.is_some() {
                prepared.request.file_name = original.file_name.clone();
            }
            prepared.request.category_id = original.category_id.clone();
            prepared.request.category_name = original.category_name.clone();
            prepared.request.scheduled_at_unix_ms = original.scheduled_at_unix_ms;
            prepared.request.connection_limit = original.connection_limit;
            prepared.request.speed_limit_bytes_per_second = original.speed_limit_bytes_per_second;
            prepared.request.adaptive_segment_policy = original.adaptive_segment_policy;
            prepared.request.duplicate_strategy = original.duplicate_strategy;
            if original.media.is_some() {
                prepared.request.media = original.media.clone();
            }
            prepared.request.validate()?;
            prepared.provenance = Some(receipt.provenance);
            prepared.expires = Some(receipt.expires);
            prepared.expires_at_unix_ms = receipt.expires_at_unix_ms;
            prepared.original_fingerprint = Some(fingerprint);
            prepared.token = Some(token);
            prepared.checked_identity = true;
            self.check_request_resolution(&prepared)?;
            info!(event = "download.resolve.receipt_reused", %correlation_id, operation_id = %prepared.operation.operation_id, "host resolution receipt reused");
            return Ok(prepared);
        }
        self.inner
            .download_http_policy
            .apply_to_request(&mut prepared.request)?;
        prepared.request.validate()?;
        if !resolver.is_noop()
            && (original.source_url.len() > crate::MAX_RESOLUTION_URL_BYTES
                || serde_json::to_vec(&original).map_or(true, |bytes| bytes.len() > 256 * 1024))
        {
            return Err(DownloadResolutionError::InvalidResult.into());
        }
        if resolver.is_noop() {
            if let Some(causality) = prepared.operation.control.extension_causality() {
                prepared.provenance = Some(DownloadResolutionProvenance {
                    original_request: original,
                    resolver: identity,
                    stages: Vec::new(),
                    operation_id: prepared.operation.operation_id,
                    correlation_id,
                    session_generation: self.session_generation(),
                    bypassed: false,
                    extension_causality: Some(causality.clone()),
                });
            }
            return Ok(prepared);
        }
        validate_identity(&identity)?;
        prepared.checked_identity = true;
        let started = Instant::now();
        info!(event = "download.resolve.accepted", %correlation_id, operation_id = %prepared.operation.operation_id, "download request resolution accepted");
        let result = (|| {
            let (stages, validity, bypassed) = if options.mode == DownloadResolutionMode::Bypass {
                (Vec::new(), RESOLUTION_RECEIPT_TTL, true)
            } else {
                {
                    let mut state = self
                        .inner
                        .resolution
                        .lock()
                        .map_err(|_| DownloadEngineError::LockPoisoned)?;
                    if state.active.len() >= MAX_CONCURRENT_RESOLUTIONS
                        || state.active.contains_key(&prepared.operation.operation_id)
                    {
                        return Err(DownloadResolutionError::Busy.into());
                    }
                    state.active.insert(
                        prepared.operation.operation_id,
                        prepared.operation.control.clone(),
                    );
                }
                let slot = ResolutionSlot(Arc::clone(&self.inner), prepared.operation.operation_id);
                let input = prepared.request.clone();
                let invocation = prepared.operation.clone();
                let worker_resolver = Arc::clone(&resolver);
                let (sender, receiver) = std::sync::mpsc::sync_channel(1);
                let inner = Arc::clone(&self.inner);
                self.inner.spawn_owned_worker(async move {
                    let _slot = slot;
                    let result = {
                        let future = worker_resolver.resolve(&input, &invocation);
                        tokio::pin!(future);
                        loop {
                            tokio::select! {
                                result = &mut future => break result,
                                () = tokio::time::sleep(Duration::from_millis(10)) => {
                                    if inner.draining.load(std::sync::atomic::Ordering::SeqCst) { invocation.control.cancel(); }
                                    if let Err(error) = invocation.check() { break Err(error); }
                                }
                            }
                        }
                    };
                    // Drop the invocation future before exposing terminal ownership.
                    let _ = sender.send(result);
                })?;
                // The adapter future is cooperatively nonblocking. Keep admission occupied until
                // its drop/retirement has completed, rather than spawning unbounded replacements.
                let result = loop {
                    match receiver.recv_timeout(Duration::from_millis(10)) {
                        Ok(result) => break result?,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            prepared.operation.check()?
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                            return Err(DownloadResolutionError::Rejected.into());
                        }
                    }
                };
                match result {
                    DownloadResolutionResult::NotHandled => {
                        (Vec::new(), RESOLUTION_RECEIPT_TTL, false)
                    }
                    DownloadResolutionResult::Resolved(result) => {
                        let (effective, stages, validity) =
                            apply_resolution(prepared.request.clone(), *result)?;
                        prepared.request = effective;
                        (stages, validity, false)
                    }
                }
            };
            prepared.expires = Some(Instant::now() + validity);
            prepared.expires_at_unix_ms = now_unix_ms()
                .saturating_add(i64::try_from(validity.as_millis()).unwrap_or(i64::MAX));
            prepared.provenance = Some(DownloadResolutionProvenance {
                original_request: original.clone(),
                resolver: identity,
                stages,
                operation_id: prepared.operation.operation_id,
                correlation_id,
                session_generation: self.session_generation(),
                bypassed,
                extension_causality: prepared.operation.control.extension_causality().cloned(),
            });
            prepared.original_fingerprint = Some(resolution_resource_fingerprint(&original)?);
            prepared.token = Some(Uuid::new_v4());
            self.check_request_resolution(&prepared)?;
            Ok(prepared)
        })();
        let outcome = match &result {
            Ok(_) => "completed",
            Err(DownloadEngineError::Resolution(DownloadResolutionError::Cancelled)) => "cancelled",
            Err(_) => "failed",
        };
        info!(event = "download.resolve.terminal", %correlation_id, operation_id = %options.operation_id, outcome, elapsed_ms = started.elapsed().as_millis(), "download request resolution retired");
        result
    }

    pub(super) fn check_request_resolution(
        &self,
        prepared: &PreparedResolution,
    ) -> Result<(), DownloadEngineError> {
        self.check_request_resolution_local(prepared)?;
        if prepared.checked_identity && prepared.resolver.identity() != prepared.identity {
            return Err(DownloadResolutionError::IdentityChanged.into());
        }
        Ok(())
    }

    // Under scheduler/task locks only inspect host-owned cancellation, time and generation.
    // Never invoke injected adapter code from a locked acceptance boundary.
    pub(super) fn check_request_resolution_local(
        &self,
        prepared: &PreparedResolution,
    ) -> Result<(), DownloadEngineError> {
        prepared.operation.check()?;
        if prepared
            .expires
            .is_some_and(|expires| expires <= Instant::now())
        {
            return Err(DownloadResolutionError::ResolutionRequired.into());
        }
        if self
            .inner
            .resolution
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?
            .generation
            != prepared.generation
        {
            return Err(DownloadResolutionError::IdentityChanged.into());
        }
        Ok(())
    }

    pub(super) fn lock_resolution_acceptance<'a>(
        &'a self,
        prepared: &PreparedResolution,
    ) -> Result<Option<std::sync::MutexGuard<'a, ResolutionState>>, DownloadEngineError> {
        prepared.operation.check()?;
        if prepared
            .expires
            .is_some_and(|expires| expires <= Instant::now())
        {
            return Err(DownloadResolutionError::ResolutionRequired.into());
        }
        let state = self
            .inner
            .resolution
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        if state.generation != prepared.generation {
            return Err(DownloadResolutionError::IdentityChanged.into());
        }
        Ok(Some(state))
    }

    fn publish_resolution_receipt(
        &self,
        prepared: &PreparedResolution,
    ) -> Result<Option<DownloadProbeResolution>, DownloadEngineError> {
        let (Some(token), Some(provenance), Some(expires), Some(original_fingerprint)) = (
            prepared.token,
            prepared.provenance.as_ref(),
            prepared.expires,
            prepared.original_fingerprint.as_ref(),
        ) else {
            return Ok(None);
        };
        let mut state = self
            .inner
            .resolution
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        if state.generation != prepared.generation {
            return Err(DownloadResolutionError::IdentityChanged.into());
        }
        state
            .receipts
            .retain(|receipt| receipt.expires > Instant::now() && receipt.token != token);
        while state.receipts.len() >= MAX_RESOLUTION_RECEIPTS {
            state.receipts.pop_front();
        }
        state.receipts.push_back(Receipt {
            token,
            original_fingerprint: original_fingerprint.clone(),
            effective: prepared.request.clone(),
            provenance: provenance.clone(),
            expires,
            expires_at_unix_ms: prepared.expires_at_unix_ms,
            generation: prepared.generation,
        });
        Ok(Some(DownloadProbeResolution {
            receipt: token,
            expires_at_unix_ms: prepared.expires_at_unix_ms,
            effective_request: prepared.request.clone(),
            provenance: provenance.clone(),
        }))
    }
}

pub(super) fn resolution_resource_fingerprint(
    request: &DownloadRequest,
) -> Result<String, DownloadEngineError> {
    let mut resource = request.clone();
    resource.save_directory = std::path::PathBuf::new();
    resource.save_directory_explicit = false;
    resource.file_name = None;
    resource.category_id = None;
    resource.category_name = None;
    resource.scheduled_at_unix_ms = None;
    resource.connection_limit = None;
    resource.speed_limit_bytes_per_second = None;
    resource.adaptive_segment_policy = None;
    resource.duplicate_strategy = Default::default();
    resource.media = None;
    resource.source_page = None;
    Ok(crate::storage::request_fingerprint(&resource)?)
}

#[cfg(test)]
mod publication_tests {
    use super::*;
    #[test]
    fn prepared_noop_request_cannot_commit_after_resolver_publication() {
        let output = tempfile::tempdir().expect("owned output");
        let engine = DownloadEngine::open_in_memory().expect("engine");
        let request = DownloadRequest {
            source_url: "https://fixture.test/file".into(),
            save_directory: output.path().to_owned(),
            save_directory_explicit: true,
            file_name: Some("file.bin".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            source_page: None,
            certificate_policy: Default::default(),
            connection_limit: Some(1),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: Default::default(),
        };
        let prepared = engine
            .prepare_request_resolution(
                request,
                DownloadResolutionOptions::default(),
                DownloadResolutionControl::default(),
                Uuid::new_v4(),
            )
            .expect("prepare no-op");
        engine
            .check_request_resolution(&prepared)
            .expect("same generation");
        engine
            .set_request_resolver(Arc::new(NoopDownloadRequestResolver))
            .expect("publication");
        assert!(matches!(
            engine.check_request_resolution(&prepared),
            Err(DownloadEngineError::Resolution(
                DownloadResolutionError::IdentityChanged
            ))
        ));
        assert!(matches!(
            engine.lock_resolution_acceptance(&prepared),
            Err(DownloadEngineError::Resolution(
                DownloadResolutionError::IdentityChanged
            ))
        ));
        assert!(engine.tasks().expect("tasks").is_empty());
    }
}
