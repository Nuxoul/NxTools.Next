//! Host-neutral request resolution. No extension runtime, process or UI ownership lives here.
//! Implementations must return nonblocking cancellable futures; dropping a future retires its
//! invocation. Native/process cleanup and bounded stuck slots remain the adapter's responsibility.
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{DownloadRequest, DownloadRequestHeader, DownloadRequestMedia};

pub const MAX_RESOLUTION_RECEIPTS: usize = 256;
pub const RESOLUTION_RECEIPT_TTL: Duration = Duration::from_secs(600);
pub const RESOLUTION_OPERATION_DEADLINE: Duration = Duration::from_secs(60);
pub const MAX_CONCURRENT_RESOLUTIONS: usize = 4;
pub const MAX_RESOLUTION_URL_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadResolverIdentity {
    pub content_version: String,
    pub settings_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadResolverStage {
    pub package_id: String,
    pub handler_id: String,
    pub content_version: String,
}

/// An explicitly supplied replacement context, never merged with inherited credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadResolvedContext {
    pub headers: Vec<DownloadRequestHeader>,
    pub cookie: Option<String>,
    pub referrer: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DownloadResolvedRequest {
    pub source_url: String,
    pub file_name: Option<String>,
    pub context: Option<DownloadResolvedContext>,
    pub media: Option<DownloadRequestMedia>,
    pub provenance: Vec<DownloadResolverStage>,
    pub valid_for: Option<Duration>,
}

impl DownloadResolvedRequest {
    /// Apply one validated stage through the same domain boundary used at final acceptance.
    pub fn apply_to(
        self,
        request: DownloadRequest,
    ) -> Result<(DownloadRequest, Vec<DownloadResolverStage>, Duration), DownloadResolutionError>
    {
        apply_resolution(request, self)
    }
}

#[derive(Debug, Clone)]
pub enum DownloadResolutionResult {
    NotHandled,
    Resolved(Box<DownloadResolvedRequest>),
}

pub type DownloadResolutionFuture<'a> = Pin<
    Box<dyn Future<Output = Result<DownloadResolutionResult, DownloadResolutionError>> + Send + 'a>,
>;

pub trait DownloadRequestResolver: Send + Sync {
    /// Immutable content/settings approval identity, rechecked before publication/acceptance.
    /// Composition must replace/revoke through DownloadEngine::set_request_resolver before
    /// publishing changed approvals/settings. Identity reads are synchronous and nonblocking;
    /// final persistence serializes with that host generation, never with adapter callbacks.
    fn identity(&self) -> DownloadResolverIdentity;
    fn resolve<'a>(
        &'a self,
        request: &'a DownloadRequest,
        operation: &'a DownloadResolutionOperation,
    ) -> DownloadResolutionFuture<'a>;
    /// Only the built-in no-op should opt into the zero-runtime fast path.
    fn is_noop(&self) -> bool {
        false
    }
}

#[derive(Default)]
pub struct NoopDownloadRequestResolver;
impl DownloadRequestResolver for NoopDownloadRequestResolver {
    fn identity(&self) -> DownloadResolverIdentity {
        DownloadResolverIdentity {
            content_version: String::new(),
            settings_version: String::new(),
        }
    }
    fn resolve<'a>(
        &'a self,
        _: &'a DownloadRequest,
        _: &'a DownloadResolutionOperation,
    ) -> DownloadResolutionFuture<'a> {
        Box::pin(async { Ok(DownloadResolutionResult::NotHandled) })
    }
    fn is_noop(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub struct DownloadResolutionControl {
    cancelled: Arc<AtomicBool>,
    timeout: Duration,
    extension_causality: Option<DownloadExtensionCausality>,
    host_cancellation: Option<tokio::sync::watch::Receiver<bool>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadExtensionCausality {
    pub root_id: Uuid,
    pub origin_plugin: String,
    pub handlers: Vec<String>,
}
impl DownloadExtensionCausality {
    pub fn validate(&self) -> Result<(), DownloadResolutionError> {
        if self.root_id.is_nil()
            || self.origin_plugin.is_empty()
            || self.origin_plugin.len() > 128
            || self.handlers.is_empty()
            || self.handlers.len() > 4
            || self.handlers.iter().enumerate().any(|(index, handler)| {
                handler.is_empty()
                    || handler.len() > 256
                    || self.handlers[..index].contains(handler)
            })
        {
            return Err(DownloadResolutionError::InvalidResult);
        }
        Ok(())
    }
}

impl Default for DownloadResolutionControl {
    fn default() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            timeout: RESOLUTION_OPERATION_DEADLINE,
            extension_causality: None,
            host_cancellation: None,
        }
    }
}
impl DownloadResolutionControl {
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            timeout: timeout.min(RESOLUTION_OPERATION_DEADLINE),
            ..Self::default()
        }
    }
    pub fn with_extension_causality(
        causality: DownloadExtensionCausality,
    ) -> Result<Self, DownloadResolutionError> {
        causality.validate()?;
        Ok(Self {
            extension_causality: Some(causality),
            ..Self::default()
        })
    }
    pub fn extension_causality(&self) -> Option<&DownloadExtensionCausality> {
        self.extension_causality.as_ref()
    }
    /// Observe the host's cancellation at every real engine boundary, even when the host
    /// executor has not yet polled its forwarding future. A closed host lease also cancels.
    pub fn with_host_cancellation(
        mut self,
        cancellation: tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        self.host_cancellation = Some(cancellation);
        self
    }
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
            || self.host_cancellation.as_ref().is_some_and(|cancellation| {
                *cancellation.borrow() || cancellation.has_changed().is_err()
            })
    }
}

#[derive(Debug, Clone)]
pub struct DownloadResolutionOperation {
    pub operation_id: Uuid,
    pub correlation_id: Uuid,
    pub session_generation: u64,
    pub deadline: Instant,
    pub control: DownloadResolutionControl,
}
impl DownloadResolutionOperation {
    pub fn check(&self) -> Result<(), DownloadResolutionError> {
        if self.control.is_cancelled() {
            return Err(DownloadResolutionError::Cancelled);
        }
        if Instant::now() >= self.deadline {
            return Err(DownloadResolutionError::Deadline);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "receipt", rename_all = "snake_case")]
pub enum DownloadResolutionMode {
    #[default]
    Resolve,
    Bypass,
    Receipt(Uuid),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadResolutionOptions {
    pub operation_id: Uuid,
    #[serde(default)]
    pub mode: DownloadResolutionMode,
}
impl Default for DownloadResolutionOptions {
    fn default() -> Self {
        Self {
            operation_id: Uuid::new_v4(),
            mode: DownloadResolutionMode::Resolve,
        }
    }
}

/// Durable original and resolver identity; DownloadTask::request is the accepted effective
/// request. Old tasks omit this field, and resume/segments never invoke the resolver again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadResolutionProvenance {
    pub original_request: DownloadRequest,
    pub resolver: DownloadResolverIdentity,
    pub stages: Vec<DownloadResolverStage>,
    pub operation_id: Uuid,
    pub correlation_id: Uuid,
    pub session_generation: u64,
    pub bypassed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extension_causality: Option<DownloadExtensionCausality>,
}

/// Informational effective context plus host-issued opaque handle. The server never accepts
/// client-supplied effective/provenance data as authorization for a resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadProbeResolution {
    pub receipt: Uuid,
    pub expires_at_unix_ms: i64,
    pub effective_request: DownloadRequest,
    pub provenance: DownloadResolutionProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DownloadResolutionError {
    #[error("download resolution was cancelled")]
    Cancelled,
    #[error("download resolution operation deadline exceeded; retry or explicitly bypass")]
    Deadline,
    #[error("download resolver rejected the request; retry or explicitly bypass")]
    Rejected,
    #[error("download resolver returned invalid or oversized context")]
    InvalidResult,
    #[error(
        "download resolution receipt is unavailable, expired or invalid; explicitly retry resolution"
    )]
    ResolutionRequired,
    #[error("download resolution approval changed; explicitly retry resolution")]
    IdentityChanged,
    #[error("download resolution capacity is occupied; retry later")]
    Busy,
}

pub(crate) fn validate_identity(
    identity: &DownloadResolverIdentity,
) -> Result<(), DownloadResolutionError> {
    if [&identity.content_version, &identity.settings_version]
        .iter()
        .any(|value| value.is_empty() || value.len() > 256)
    {
        return Err(DownloadResolutionError::InvalidResult);
    }
    Ok(())
}

pub(crate) fn apply_resolution(
    mut effective: DownloadRequest,
    result: DownloadResolvedRequest,
) -> Result<(DownloadRequest, Vec<DownloadResolverStage>, Duration), DownloadResolutionError> {
    if result.source_url.len() > MAX_RESOLUTION_URL_BYTES
        || result.provenance.is_empty()
        || result.provenance.len() > 8
        || result.provenance.iter().any(|stage| {
            [&stage.package_id, &stage.handler_id, &stage.content_version]
                .iter()
                .any(|value| value.is_empty() || value.len() > 256)
        })
    {
        return Err(DownloadResolutionError::InvalidResult);
    }
    if result.context.as_ref().is_some_and(|context| {
        context.headers.len() > crate::MAX_DEFAULT_HEADER_COUNT
            || context
                .headers
                .iter()
                .any(|header| header.name.len() > 256 || header.value.len() > 8 * 1024)
            || context
                .cookie
                .as_ref()
                .is_some_and(|value| value.len() > crate::MAX_DEFAULT_COOKIE_BYTES)
            || context
                .referrer
                .as_ref()
                .is_some_and(|value| value.len() > MAX_RESOLUTION_URL_BYTES)
    }) {
        return Err(DownloadResolutionError::InvalidResult);
    }
    let old_url = url::Url::parse(&effective.source_url)
        .map_err(|_| DownloadResolutionError::InvalidResult)?;
    let new_url =
        url::Url::parse(&result.source_url).map_err(|_| DownloadResolutionError::InvalidResult)?;
    if !matches!(new_url.scheme(), "http" | "https")
        || new_url.host_str().is_none()
        || !new_url.username().is_empty()
        || new_url.password().is_some()
    {
        return Err(DownloadResolutionError::InvalidResult);
    }
    if old_url.origin() != new_url.origin() {
        effective.headers.clear();
        effective.cookie = None;
        effective.referrer = None;
        effective.media = None;
    }
    if effective.source_url != result.source_url {
        // A length derived from the old resource cannot constrain the transformed resource.
        effective.expected_total_bytes = None;
        effective.media = None;
    }
    effective.source_url = result.source_url;
    if effective.file_name.is_none() {
        effective.file_name = result.file_name;
    }
    if let Some(context) = result.context {
        effective.headers = context.headers;
        effective.cookie = context.cookie;
        effective.referrer = context.referrer;
    }
    if let Some(media) = result.media {
        effective.media = Some(media);
    }
    if effective
        .referrer
        .as_ref()
        .is_some_and(|value| value.len() > MAX_RESOLUTION_URL_BYTES)
    {
        return Err(DownloadResolutionError::InvalidResult);
    }
    effective
        .validate()
        .map_err(|_| DownloadResolutionError::InvalidResult)?;
    let validity = result
        .valid_for
        .unwrap_or(RESOLUTION_RECEIPT_TTL)
        .min(RESOLUTION_RECEIPT_TTL);
    if validity.is_zero() {
        return Err(DownloadResolutionError::ResolutionRequired);
    }
    Ok((effective, result.provenance, validity))
}
