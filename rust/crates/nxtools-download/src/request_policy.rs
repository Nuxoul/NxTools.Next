use std::sync::{Arc, RwLock};

use nxtools_core::DownloadSettings;
use reqwest::header::{HeaderName, HeaderValue};
use thiserror::Error;

use crate::{DownloadCertificatePolicy, DownloadRequest, DownloadRequestHeader};

pub const MAX_DEFAULT_HEADER_TEXT_BYTES: usize = 32 * 1024;
pub const MAX_DEFAULT_COOKIE_BYTES: usize = 16 * 1024;
pub const MAX_DEFAULT_HEADER_COUNT: usize = 64;
const MAX_HEADER_NAME_BYTES: usize = 256;
const MAX_HEADER_VALUE_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone)]
pub struct DownloadHttpPolicy {
    current: Arc<RwLock<DownloadHttpPolicySnapshot>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DownloadHttpPolicySnapshot {
    default_headers: Vec<DownloadRequestHeader>,
    default_cookie: Option<String>,
    certificate_policy: DownloadCertificatePolicy,
}

impl DownloadHttpPolicy {
    pub fn from_download_settings(
        settings: &DownloadSettings,
    ) -> Result<Self, DownloadHttpPolicyError> {
        Ok(Self {
            current: Arc::new(RwLock::new(parse_snapshot(settings)?)),
        })
    }

    pub fn update_from_download_settings(
        &self,
        settings: &DownloadSettings,
    ) -> Result<(), DownloadHttpPolicyError> {
        let next = parse_snapshot(settings)?;
        *self
            .current
            .write()
            .map_err(|_| DownloadHttpPolicyError::LockPoisoned)? = next;
        Ok(())
    }

    pub(crate) fn apply_to_request(
        &self,
        request: &mut DownloadRequest,
    ) -> Result<(), DownloadHttpPolicyError> {
        let snapshot = self
            .current
            .read()
            .map_err(|_| DownloadHttpPolicyError::LockPoisoned)?
            .clone();
        for header in snapshot.default_headers {
            if !request
                .headers
                .iter()
                .any(|existing| existing.name.eq_ignore_ascii_case(&header.name))
            {
                request.headers.push(header);
            }
        }
        if request.referrer.is_some() {
            request
                .headers
                .retain(|header| !header.name.eq_ignore_ascii_case("referer"));
        }
        if request.cookie.is_some() {
            request
                .headers
                .retain(|header| !header.name.eq_ignore_ascii_case("cookie"));
        } else if !request
            .headers
            .iter()
            .any(|header| header.name.eq_ignore_ascii_case("cookie"))
        {
            request.cookie = snapshot.default_cookie;
        }
        request.certificate_policy = snapshot.certificate_policy;
        Ok(())
    }
}

fn parse_snapshot(
    settings: &DownloadSettings,
) -> Result<DownloadHttpPolicySnapshot, DownloadHttpPolicyError> {
    let raw_headers = settings.default_request_headers.trim();
    if raw_headers.len() > MAX_DEFAULT_HEADER_TEXT_BYTES {
        return Err(DownloadHttpPolicyError::HeaderTextTooLarge);
    }
    let cookie = normalize_cookie(&settings.default_cookie)?;
    let mut headers = Vec::new();
    let mut embedded_cookie = None;
    for raw_line in raw_headers.lines() {
        let mut line = raw_line.trim().trim_end_matches('\\').trim();
        if line.is_empty() {
            continue;
        }
        let curl_header = line
            .strip_prefix("-H ")
            .or_else(|| line.strip_prefix("--header "));
        if let Some(value) = curl_header {
            line = strip_matching_quotes(value.trim());
        }
        let (name, value) = line
            .split_once(':')
            .ok_or(DownloadHttpPolicyError::InvalidHeaderLine)?;
        let name = name.trim();
        let value = value.trim();
        validate_header(name, value)?;
        if name.eq_ignore_ascii_case("cookie") {
            embedded_cookie = Some(value.to_owned());
            continue;
        }
        if let Some(existing) = headers
            .iter_mut()
            .find(|header: &&mut DownloadRequestHeader| header.name.eq_ignore_ascii_case(name))
        {
            existing.value = value.to_owned();
        } else {
            if headers.len() >= MAX_DEFAULT_HEADER_COUNT {
                return Err(DownloadHttpPolicyError::TooManyHeaders);
            }
            headers.push(DownloadRequestHeader {
                name: name.to_owned(),
                value: value.to_owned(),
            });
        }
    }
    Ok(DownloadHttpPolicySnapshot {
        default_headers: headers,
        default_cookie: cookie.or(embedded_cookie),
        certificate_policy: if settings.accept_invalid_certificates {
            DownloadCertificatePolicy::AcceptInvalid
        } else {
            DownloadCertificatePolicy::Strict
        },
    })
}

fn normalize_cookie(value: &str) -> Result<Option<String>, DownloadHttpPolicyError> {
    let value = value.trim();
    let value = if value
        .get(.."Cookie:".len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("Cookie:"))
    {
        value["Cookie:".len()..].trim()
    } else {
        value
    };
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > MAX_DEFAULT_COOKIE_BYTES
        || value.contains(['\r', '\n'])
        || HeaderValue::from_str(value).is_err()
    {
        return Err(DownloadHttpPolicyError::InvalidCookie);
    }
    Ok(Some(value.to_owned()))
}

pub(crate) fn validate_header(name: &str, value: &str) -> Result<(), DownloadHttpPolicyError> {
    if name.is_empty()
        || name.len() > MAX_HEADER_NAME_BYTES
        || value.is_empty()
        || value.len() > MAX_HEADER_VALUE_BYTES
        || name.contains([':', '\r', '\n'])
        || value.contains(['\r', '\n'])
        || HeaderName::from_bytes(name.as_bytes()).is_err()
        || HeaderValue::from_str(value).is_err()
    {
        return Err(DownloadHttpPolicyError::InvalidHeaderLine);
    }
    if is_download_transport_owned_header(name) {
        return Err(DownloadHttpPolicyError::TransportHeader);
    }
    Ok(())
}

pub(crate) fn is_valid_header_value(value: &str) -> bool {
    HeaderValue::from_str(value).is_ok()
}

fn strip_matching_quotes(value: &str) -> &str {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if matches!(
            (bytes[0], bytes[value.len() - 1]),
            (b'\'', b'\'') | (b'"', b'"')
        ) {
            return &value[1..value.len() - 1];
        }
    }
    value
}

pub fn is_download_transport_owned_header(name: &str) -> bool {
    [
        "accept-encoding",
        "connection",
        "content-length",
        "host",
        "proxy-connection",
        "range",
        "transfer-encoding",
    ]
    .iter()
    .any(|blocked| name.eq_ignore_ascii_case(blocked))
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DownloadHttpPolicyError {
    #[error("default Header text exceeds the bounded limit")]
    HeaderTextTooLarge,
    #[error("default Header contains an invalid line")]
    InvalidHeaderLine,
    #[error("default Header contains too many fields")]
    TooManyHeaders,
    #[error("transport-owned Header cannot be configured")]
    TransportHeader,
    #[error("default Cookie is invalid or exceeds the bounded limit")]
    InvalidCookie,
    #[error("download HTTP policy lock is unavailable")]
    LockPoisoned,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_transport_owned_headers_from_download_settings() {
        let settings = DownloadSettings {
            default_request_headers: "Host: untrusted.example".into(),
            ..DownloadSettings::default()
        };

        assert_eq!(
            DownloadHttpPolicy::from_download_settings(&settings)
                .expect_err("transport-owned Header must be rejected"),
            DownloadHttpPolicyError::TransportHeader
        );
    }

    #[test]
    fn rejects_values_that_cannot_cross_the_real_http_header_boundary() {
        for (headers, cookie) in [
            ("Invalid Name: value", ""),
            ("X-NxTools-Test: value\u{7f}", ""),
            ("", "session=value\u{7f}"),
        ] {
            let settings = DownloadSettings {
                default_request_headers: headers.into(),
                default_cookie: cookie.into(),
                ..DownloadSettings::default()
            };
            assert!(DownloadHttpPolicy::from_download_settings(&settings).is_err());
        }
    }

    #[test]
    fn dedicated_cookie_overrides_embedded_header_cookie() {
        let settings = DownloadSettings {
            default_request_headers: "Cookie: embedded=1".into(),
            default_cookie: "Cookie: dedicated=1".into(),
            ..DownloadSettings::default()
        };
        let policy =
            DownloadHttpPolicy::from_download_settings(&settings).expect("parse HTTP policy");
        let mut request = DownloadRequest {
            source_url: "https://example.test/file.bin".into(),
            save_directory: std::path::PathBuf::from(r"C:\Downloads"),
            file_name: Some("file.bin".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            certificate_policy: DownloadCertificatePolicy::Strict,
            connection_limit: Some(1),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
        };

        policy
            .apply_to_request(&mut request)
            .expect("apply HTTP policy");
        assert_eq!(request.cookie.as_deref(), Some("dedicated=1"));
        assert!(
            request
                .headers
                .iter()
                .all(|header| !header.name.eq_ignore_ascii_case("cookie"))
        );
    }

    #[test]
    fn task_cookie_overrides_task_cookie_header_and_configured_default() {
        let settings = DownloadSettings {
            default_cookie: "configured=1".into(),
            ..DownloadSettings::default()
        };
        let policy =
            DownloadHttpPolicy::from_download_settings(&settings).expect("parse HTTP policy");
        let mut request = DownloadRequest {
            source_url: "https://example.test/file.bin".into(),
            save_directory: std::path::PathBuf::from(r"C:\Downloads"),
            file_name: Some("file.bin".into()),
            headers: vec![DownloadRequestHeader {
                name: "Cookie".into(),
                value: "header=1".into(),
            }],
            cookie: Some("task=1".into()),
            referrer: None,
            certificate_policy: DownloadCertificatePolicy::Strict,
            connection_limit: Some(1),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
        };

        policy
            .apply_to_request(&mut request)
            .expect("apply HTTP policy");
        assert_eq!(request.cookie.as_deref(), Some("task=1"));
        assert!(
            request
                .headers
                .iter()
                .all(|header| !header.name.eq_ignore_ascii_case("cookie"))
        );
    }

    #[test]
    fn dedicated_referrer_overrides_referer_header() {
        let settings = DownloadSettings {
            default_request_headers: "Referer: https://default.example/".into(),
            ..DownloadSettings::default()
        };
        let policy =
            DownloadHttpPolicy::from_download_settings(&settings).expect("parse HTTP policy");
        let mut request = DownloadRequest {
            source_url: "https://example.test/file.bin".into(),
            save_directory: std::path::PathBuf::from(r"C:\Downloads"),
            file_name: Some("file.bin".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: Some("https://task.example/".into()),
            certificate_policy: DownloadCertificatePolicy::Strict,
            connection_limit: Some(1),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
        };

        policy
            .apply_to_request(&mut request)
            .expect("apply HTTP policy");
        assert_eq!(request.referrer.as_deref(), Some("https://task.example/"));
        assert!(
            request
                .headers
                .iter()
                .all(|header| !header.name.eq_ignore_ascii_case("referer"))
        );
    }

    #[test]
    fn preserves_quotes_inside_ordinary_header_values() {
        let settings = DownloadSettings {
            default_request_headers: "If-None-Match: \"etag\"".into(),
            ..DownloadSettings::default()
        };
        let policy =
            DownloadHttpPolicy::from_download_settings(&settings).expect("parse HTTP policy");
        let snapshot = policy.current.read().expect("read HTTP policy");
        assert_eq!(snapshot.default_headers[0].value, "\"etag\"");
    }

    #[test]
    fn rejects_cookie_with_line_breaks_or_excessive_length() {
        for value in [
            "session=one\r\nInjected: value".to_owned(),
            "x".repeat(MAX_DEFAULT_COOKIE_BYTES + 1),
        ] {
            let settings = DownloadSettings {
                default_cookie: value,
                ..DownloadSettings::default()
            };
            assert_eq!(
                DownloadHttpPolicy::from_download_settings(&settings)
                    .expect_err("invalid Cookie must be rejected"),
                DownloadHttpPolicyError::InvalidCookie
            );
        }
    }
}
