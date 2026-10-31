//! Host-neutral HTTP admission/validation only. No network client, proxy settings or IO here.
use crate::{InvocationIdentity, NetworkDeclaration, NetworkOrigin};
use serde::{Deserialize, Deserializer, de};
use serde_json::{Map, Value};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use url::Url;
use uuid::Uuid;

pub const MAX_REQUEST_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_RUN_IO_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_HEADERS: usize = 64;
pub const MAX_HEADER_BYTES: usize = 16 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HttpError {
    #[error("invalid HTTP parameters")]
    InvalidParams,
    #[error("HTTP origin is not declared")]
    OriginDenied,
    #[error("HTTP invocation is stale")]
    Stale,
    #[error("HTTP resource limit")]
    ResourceLimit,
    #[error("HTTP deadline expired")]
    Timeout,
}
impl HttpError {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidParams => "invalid_params",
            Self::OriginDenied => "origin_denied",
            Self::Stale => "stale",
            Self::ResourceLimit => "resource_limit",
            Self::Timeout => "timeout",
        }
    }
}
/// Payload-bearing types intentionally have no Debug implementation; diagnostics use fixed codes.
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}
pub struct HttpRequest {
    pub url: Url,
    pub method: String,
    pub headers: Vec<HttpHeader>,
    pub body_text: Option<String>,
    pub body_blob_id: Option<Uuid>,
    pub timeout: Duration,
    pub response_max_bytes: usize,
    pub max_redirects: u8,
}
fn take<T: serde::de::DeserializeOwned>(
    fields: &mut Map<String, Value>,
    key: &str,
) -> Result<Option<T>, HttpError> {
    fields
        .remove(key)
        .map(serde_json::from_value)
        .transpose()
        .map_err(|_| HttpError::InvalidParams)
}
/// Strict original spelling before WHATWG parsing: no credentials, whitespace, backslash,
/// fragments or ambiguous IPv4. No DNS resolution or local/private reachability probe.
pub fn parse_request_url(value: &str) -> Result<Url, HttpError> {
    if value.is_empty()
        || value.len() > 8192
        || value
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace())
        || value.contains(['\\', '#'])
    {
        return Err(HttpError::InvalidParams);
    }
    let (scheme, remainder) = value.split_once("://").ok_or(HttpError::InvalidParams)?;
    let authority = remainder
        .split(['/', '?'])
        .next()
        .ok_or(HttpError::InvalidParams)?;
    NetworkOrigin::parse(&format!("{scheme}://{authority}"))
        .map_err(|_| HttpError::InvalidParams)?;
    let url = Url::parse(value).map_err(|_| HttpError::InvalidParams)?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(HttpError::InvalidParams);
    }
    Ok(url)
}
fn header_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
}
impl HttpRequest {
    pub fn parse(value: Value) -> Result<Self, HttpError> {
        let Value::Object(mut fields) = value else {
            return Err(HttpError::InvalidParams);
        };
        let raw_url: String = take(&mut fields, "url")?.ok_or(HttpError::InvalidParams)?;
        let url = parse_request_url(&raw_url)?;
        let method: String = take(&mut fields, "method")?.ok_or(HttpError::InvalidParams)?;
        if !matches!(
            method.as_str(),
            "GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE"
        ) {
            return Err(HttpError::InvalidParams);
        }
        let mut headers = Vec::new();
        if let Some(value) = fields.remove("headers") {
            let Value::Array(values) = value else {
                return Err(HttpError::InvalidParams);
            };
            if values.len() > MAX_HEADERS {
                return Err(HttpError::InvalidParams);
            }
            let mut bytes = 0usize;
            for value in values {
                let Value::Object(mut header) = value else {
                    return Err(HttpError::InvalidParams);
                };
                let name: String = take(&mut header, "name")?.ok_or(HttpError::InvalidParams)?;
                let value: String = take(&mut header, "value")?.ok_or(HttpError::InvalidParams)?;
                if !header.is_empty()
                    || !header_name(&name)
                    || value.chars().any(|ch| ch.is_control() && ch != '\t')
                    || matches!(
                        name.to_ascii_lowercase().as_str(),
                        "host"
                            | "connection"
                            | "transfer-encoding"
                            | "content-length"
                            | "proxy-authorization"
                            | "proxy-authenticate"
                            | "proxy-connection"
                            | "keep-alive"
                            | "te"
                            | "trailer"
                            | "upgrade"
                    )
                {
                    return Err(HttpError::InvalidParams);
                }
                bytes = bytes
                    .checked_add(name.len())
                    .and_then(|total| total.checked_add(value.len()))
                    .ok_or(HttpError::InvalidParams)?;
                if bytes > MAX_HEADER_BYTES {
                    return Err(HttpError::InvalidParams);
                }
                headers.push(HttpHeader { name, value });
            }
        }
        let body_text: Option<String> = take(&mut fields, "body_text")?;
        let body_blob_id: Option<Uuid> = take(&mut fields, "body_blob_id")?;
        if body_text
            .as_ref()
            .is_some_and(|text| text.len() > MAX_REQUEST_BYTES)
            || (body_text.is_some() && body_blob_id.is_some())
            || body_blob_id.is_some_and(|id| id.is_nil())
        {
            return Err(HttpError::InvalidParams);
        }
        let timeout_ms: u64 = take(&mut fields, "timeout_ms")?.unwrap_or(10000);
        let response_max_bytes: usize =
            take(&mut fields, "response_max_bytes")?.unwrap_or(MAX_REQUEST_BYTES);
        let max_redirects: u8 = take(&mut fields, "max_redirects")?.unwrap_or(0);
        if !fields.is_empty()
            || !(1..=30000).contains(&timeout_ms)
            || !(1..=MAX_RESPONSE_BYTES).contains(&response_max_bytes)
            || max_redirects > 3
        {
            return Err(HttpError::InvalidParams);
        }
        Ok(Self {
            url,
            method,
            headers,
            body_text,
            body_blob_id,
            timeout: Duration::from_millis(timeout_ms),
            response_max_bytes,
            max_redirects,
        })
    }
}
impl<'de> Deserialize<'de> for HttpRequest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let crate::strict_json::StrictValue(value) = Deserialize::deserialize(deserializer)?;
        Self::parse(value).map_err(|_| de::Error::custom("invalid bounded HTTP request"))
    }
}
struct Run {
    identity: InvocationIdentity,
    origins: NetworkDeclaration,
    deadline: tokio::time::Instant,
    active: Arc<Semaphore>,
    bytes: AtomicUsize,
}
/// Owned by one immutable invocation, never a map keyed by caller-supplied IDs.
#[derive(Clone)]
pub struct HttpRun(Arc<Run>);
pub struct HttpAdmission {
    _permit: OwnedSemaphorePermit,
    deadline: tokio::time::Instant,
}
impl HttpAdmission {
    pub fn deadline(&self) -> tokio::time::Instant {
        self.deadline
    }
}
impl HttpRun {
    pub fn new(
        identity: InvocationIdentity,
        origins: NetworkDeclaration,
        deadline: tokio::time::Instant,
    ) -> Self {
        Self(Arc::new(Run {
            identity,
            origins,
            deadline,
            active: Arc::new(Semaphore::new(2)),
            bytes: AtomicUsize::new(0),
        }))
    }
    pub fn allows_url(&self, url: &Url) -> bool {
        self.0.origins.allows_url(url)
    }
    pub fn admit(
        &self,
        identity: &InvocationIdentity,
        request: &HttpRequest,
    ) -> Result<HttpAdmission, HttpError> {
        if identity != &self.0.identity {
            return Err(HttpError::Stale);
        }
        let now = tokio::time::Instant::now();
        if now >= self.0.deadline {
            return Err(HttpError::Timeout);
        }
        if !self.allows_url(&request.url) {
            return Err(HttpError::OriginDenied);
        }
        let permit = self
            .0
            .active
            .clone()
            .try_acquire_owned()
            .map_err(|_| HttpError::ResourceLimit)?;
        Ok(HttpAdmission {
            _permit: permit,
            deadline: self.0.deadline.min(now + request.timeout),
        })
    }
    /// Charge actual decoded request/response bytes, including failed/retired operations; no refund.
    pub fn charge(&self, bytes: usize) -> Result<(), HttpError> {
        self.0
            .bytes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(bytes)
                    .filter(|total| *total <= MAX_RUN_IO_BYTES)
            })
            .map(|_| ())
            .map_err(|_| HttpError::ResourceLimit)
    }
}
