//! Extension-only HTTP consumer of the original immutable product NetworkPolicy.
//! Payloads never enter diagnostics; there is no cookie jar, inherited download state or retry.
use flate2::{Decompress, FlushDecompress, Status};
use nxtools_extensions::{
    HostReply, InvocationIdentity,
    blobs::{BlobRun, HostBlobWriter, MAX_CHUNK_BYTES},
    http::{
        HttpHeader, HttpRequest, HttpRun, MAX_HEADER_BYTES, MAX_HEADERS, MAX_REQUEST_BYTES,
        MAX_RUN_IO_BYTES, parse_request_url,
    },
};
use nxtools_network::{NetworkOperationSnapshot, NetworkPolicy};
use reqwest::{
    Client, Response,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde_json::{Value, json};
use std::{
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};
use tokio::sync::{Semaphore, watch};

static GLOBAL_REQUESTS: OnceLock<Arc<Semaphore>> = OnceLock::new();
#[derive(Clone)]
pub(crate) struct ExtensionHttpClient {
    policy: NetworkPolicy,
}
impl ExtensionHttpClient {
    pub(crate) fn new(policy: NetworkPolicy) -> Self {
        Self { policy }
    }
    /// Caller retains the original runtime current-trust admission/reply checks. HttpRun and BlobRun
    /// belong to that same immutable invocation; this consumer owns no registry/config controller.
    pub(crate) async fn request(
        &self,
        identity: &InvocationIdentity,
        run: &HttpRun,
        blobs: &BlobRun,
        mut cancellation: watch::Receiver<bool>,
        params: Value,
    ) -> HostReply {
        let request = match HttpRequest::parse(params) {
            Ok(request) => request,
            Err(failure) => return HostReply::error(failure.code()),
        };
        if *cancellation.borrow() {
            return HostReply::error("cancelled");
        }
        let admission = match run.admit(identity, &request) {
            Ok(admission) => admission,
            Err(failure) => return HostReply::error(failure.code()),
        };
        let permit = match GLOBAL_REQUESTS
            .get_or_init(|| Arc::new(Semaphore::new(4)))
            .clone()
            .try_acquire_owned()
        {
            Ok(permit) => permit,
            Err(_) => return HostReply::error("resource_limit"),
        };
        let snapshot = match self.policy.snapshot() {
            Ok(snapshot) => snapshot,
            Err(_) => return HostReply::error("network_unavailable"),
        };
        let started = Instant::now();
        tracing::info!(event="extension.http.accepted",correlation_id=%identity.correlation_id,run_id=%identity.run_id,mode=nxtools_network::proxy_mode_name(snapshot.mode()),"bounded extension HTTP admitted");
        let operation_cancel = cancellation.clone();
        let _permit = permit;
        let outcome = tokio::select! {
            biased;
            _ = cancellation.changed() => Err("cancelled"),
            _ = tokio::time::sleep_until(admission.deadline()) => Err("timeout"),
            result = execute(identity,run,blobs,&request,snapshot,admission.deadline(),&operation_cancel) => result,
        };
        match outcome {
            Ok(value) => {
                tracing::info!(event="extension.http.terminal",correlation_id=%identity.correlation_id,run_id=%identity.run_id,reason="success",status=value["status"].as_u64().unwrap_or(0),byte_count=value["body_bytes"].as_u64().unwrap_or(0),redirect_count=value["redirect_count"].as_u64().unwrap_or(0),elapsed_ms=started.elapsed().as_millis() as u64,"bounded extension HTTP complete");
                HostReply {
                    success: true,
                    value,
                }
            }
            Err(reason) => {
                tracing::info!(event="extension.http.terminal",correlation_id=%identity.correlation_id,run_id=%identity.run_id,reason,elapsed_ms=started.elapsed().as_millis() as u64,"bounded extension HTTP retired; server effects may already exist");
                HostReply::error(reason)
            }
        }
    }
}
fn client(snapshot: NetworkOperationSnapshot, remaining: Duration) -> Result<Client, &'static str> {
    snapshot
        .build_client(
            Client::builder()
                .connect_timeout(remaining.min(Duration::from_secs(3)))
                .timeout(remaining)
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .referer(false)
                .no_gzip()
                .no_brotli()
                .no_deflate()
                .no_zstd()
                .http1_only(),
        )
        .map_err(|_| "network_unavailable")
}
fn network_error(failure: reqwest::Error) -> &'static str {
    if failure.is_timeout() {
        "timeout"
    } else {
        "network_failed"
    }
}
fn headers(input: &[HttpHeader]) -> Result<HeaderMap, &'static str> {
    let mut headers = HeaderMap::new();
    for header in input {
        let name = HeaderName::from_bytes(header.name.as_bytes()).map_err(|_| "invalid_params")?;
        let value = HeaderValue::from_str(&header.value).map_err(|_| "invalid_params")?;
        headers.append(name, value);
    }
    Ok(headers)
}
fn response_headers(input: &HeaderMap) -> Result<Vec<Value>, &'static str> {
    let mut result = Vec::new();
    let mut bytes = 0usize;
    for (index, (name, value)) in input.iter().enumerate() {
        bytes = bytes
            .checked_add(name.as_str().len())
            .and_then(|total| total.checked_add(value.as_bytes().len()))
            .ok_or("response_headers_too_large")?;
        if index >= MAX_HEADERS || bytes > MAX_HEADER_BYTES {
            return Err("response_headers_too_large");
        }
        if name == reqwest::header::SET_COOKIE {
            continue;
        }
        let value = value.to_str().map_err(|_| "invalid_response_headers")?;
        result.push(json!({"name":name.as_str(),"value":value}));
    }
    Ok(result)
}
fn current(
    cancellation: &watch::Receiver<bool>,
    deadline: tokio::time::Instant,
) -> Result<(), &'static str> {
    if *cancellation.borrow() {
        Err("cancelled")
    } else if tokio::time::Instant::now() >= deadline {
        Err("timeout")
    } else {
        Ok(())
    }
}
async fn execute(
    identity: &InvocationIdentity,
    run: &HttpRun,
    blobs: &BlobRun,
    request: &HttpRequest,
    snapshot: NetworkOperationSnapshot,
    deadline: tokio::time::Instant,
    cancellation: &watch::Receiver<bool>,
) -> Result<Value, &'static str> {
    // Validate caller headers and request backing before allocating a response or contacting a peer.
    let mut caller_headers = headers(&request.headers)?;
    let body = if let Some(text) = &request.body_text {
        Some(text.as_bytes().to_vec())
    } else if let Some(id) = request.body_blob_id {
        Some(
            blobs
                .host_read_sealed(id, MAX_REQUEST_BYTES)
                .await
                .map_err(|failure| failure.code())?,
        )
    } else {
        None
    };
    let mut writer = blobs
        .host_writer(
            request.response_max_bytes,
            "application/octet-stream".into(),
        )
        .await
        .map_err(|failure| failure.code())?;
    current(cancellation, deadline)?;
    let client = client(
        snapshot,
        deadline.saturating_duration_since(tokio::time::Instant::now()),
    )?;
    let mut url = request.url.clone();
    let mut redirects = 0u8;
    loop {
        current(cancellation, deadline)?;
        if !run.allows_url(&url) {
            return Err("origin_denied");
        }
        let mut outgoing = client
            .request(
                reqwest::Method::from_bytes(request.method.as_bytes())
                    .map_err(|_| "invalid_params")?,
                url.clone(),
            )
            .headers(caller_headers.clone());
        if let Some(body) = &body {
            run.charge(body.len()).map_err(|failure| failure.code())?;
            outgoing = outgoing.body(body.clone());
        }
        tracing::info!(event="extension.http.transition",correlation_id=%identity.correlation_id,run_id=%identity.run_id,stage="request_started",redirect_count=redirects,"bounded extension HTTP contact");
        let mut response = outgoing.send().await.map_err(network_error)?;
        let exposed_headers = response_headers(response.headers())?;
        let status = response.status().as_u16();
        let following = request.max_redirects > 0
            && matches!(request.method.as_str(), "GET" | "HEAD")
            && matches!(status, 301 | 302 | 303 | 307 | 308);
        if following && let Some(location) = response.headers().get(reqwest::header::LOCATION) {
            if redirects >= request.max_redirects {
                return Err("redirect_limit");
            }
            let location = location.to_str().map_err(|_| "invalid_redirect")?;
            if location.len() > 8192
                || location
                    .chars()
                    .any(|ch| ch.is_control() || ch.is_whitespace())
                || location.contains('\\')
            {
                return Err("invalid_redirect");
            }
            // Validate absolute authority spelling before Url::join canonicalizes ambiguous IPv4.
            let absolute = if location.starts_with("//") {
                Some(format!("{}:{location}", url.scheme()))
            } else if location.contains("://") {
                Some(location.to_owned())
            } else {
                None
            };
            if let Some(absolute) = absolute {
                parse_request_url(&absolute).map_err(|_| "invalid_redirect")?;
            }
            let next = url.join(location).map_err(|_| "invalid_redirect")?;
            parse_request_url(next.as_str()).map_err(|_| "invalid_redirect")?;
            if url.scheme() == "https" && next.scheme() != "https" {
                return Err("redirect_downgrade");
            }
            if !run.allows_url(&next) {
                return Err("origin_denied");
            }
            if url.origin() != next.origin() {
                caller_headers.clear();
            }
            redirects += 1;
            url = next;
            continue;
        }
        if request.method != "HEAD" {
            consume(
                &mut response,
                &mut writer,
                run,
                request.response_max_bytes,
                deadline,
                cancellation,
            )
            .await?;
        }
        current(cancellation, deadline)?;
        let blob = writer.seal().await.map_err(|failure| failure.code())?;
        return Ok(
            json!({"status":status,"headers":exposed_headers,"body_blob_id":blob.blob_id,"body_bytes":blob.length,"final_url":url.as_str(),"redirect_count":redirects}),
        );
    }
}
enum Decoder {
    Identity,
    Compressed { decoder: Decompress, ended: bool },
}
impl Decoder {
    fn new(headers: &HeaderMap) -> Result<Self, &'static str> {
        let encodings = headers
            .get_all(reqwest::header::CONTENT_ENCODING)
            .iter()
            .collect::<Vec<_>>();
        if encodings.is_empty() {
            return Ok(Self::Identity);
        }
        if encodings.len() != 1 {
            return Err("unsupported_content_encoding");
        }
        match encodings[0]
            .to_str()
            .map_err(|_| "unsupported_content_encoding")?
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "identity" => Ok(Self::Identity),
            "gzip" => Ok(Self::Compressed {
                decoder: Decompress::new_gzip(15),
                ended: false,
            }),
            "deflate" => Ok(Self::Compressed {
                decoder: Decompress::new(true),
                ended: false,
            }),
            _ => Err("unsupported_content_encoding"),
        }
    }
}
async fn consume(
    response: &mut Response,
    writer: &mut HostBlobWriter,
    run: &HttpRun,
    maximum: usize,
    deadline: tokio::time::Instant,
    cancellation: &watch::Receiver<bool>,
) -> Result<(), &'static str> {
    let mut decoder = Decoder::new(response.headers())?;
    let mut decoded = 0usize;
    let mut encoded = 0usize;
    while let Some(chunk) = response.chunk().await.map_err(network_error)? {
        current(cancellation, deadline)?;
        if chunk.is_empty() {
            continue;
        }
        encoded = encoded
            .checked_add(chunk.len())
            .ok_or("content_too_large")?;
        if encoded > MAX_RUN_IO_BYTES {
            return Err("content_too_large");
        }
        match &mut decoder {
            Decoder::Identity => {
                for bytes in chunk.chunks(MAX_CHUNK_BYTES) {
                    write_decoded(
                        writer,
                        run,
                        bytes,
                        &mut decoded,
                        maximum,
                        deadline,
                        cancellation,
                    )
                    .await?;
                }
            }
            Decoder::Compressed { decoder, ended } => {
                if *ended && !chunk.is_empty() {
                    return Err("body_decode_failed");
                }
                let mut input = chunk.as_ref();
                loop {
                    current(cancellation, deadline)?;
                    let mut output = vec![0u8; MAX_CHUNK_BYTES];
                    let before_in = decoder.total_in();
                    let before_out = decoder.total_out();
                    let status = decoder
                        .decompress(input, &mut output, FlushDecompress::None)
                        .map_err(|_| "body_decode_failed")?;
                    let consumed = (decoder.total_in() - before_in) as usize;
                    let produced = (decoder.total_out() - before_out) as usize;
                    input = &input[consumed..];
                    output.truncate(produced);
                    if produced > 0 {
                        write_decoded(
                            writer,
                            run,
                            &output,
                            &mut decoded,
                            maximum,
                            deadline,
                            cancellation,
                        )
                        .await?;
                    }
                    if status == Status::StreamEnd {
                        *ended = true;
                        if !input.is_empty() {
                            return Err("body_decode_failed");
                        }
                        break;
                    }
                    if input.is_empty() && produced < MAX_CHUNK_BYTES {
                        break;
                    }
                    if consumed == 0 && produced == 0 {
                        return Err("body_decode_failed");
                    }
                    tokio::task::yield_now().await;
                }
            }
        }
    }
    if let Decoder::Compressed { decoder, ended } = &mut decoder
        && !*ended
    {
        loop {
            current(cancellation, deadline)?;
            let mut output = vec![0u8; MAX_CHUNK_BYTES];
            let before = decoder.total_out();
            let status = decoder
                .decompress(&[], &mut output, FlushDecompress::Finish)
                .map_err(|_| "body_decode_failed")?;
            let produced = (decoder.total_out() - before) as usize;
            output.truncate(produced);
            if produced > 0 {
                write_decoded(
                    writer,
                    run,
                    &output,
                    &mut decoded,
                    maximum,
                    deadline,
                    cancellation,
                )
                .await?;
            }
            if status == Status::StreamEnd {
                break;
            }
            if produced == 0 {
                return Err("body_decode_failed");
            }
            tokio::task::yield_now().await;
        }
    }
    Ok(())
}
async fn write_decoded(
    writer: &mut HostBlobWriter,
    run: &HttpRun,
    bytes: &[u8],
    decoded: &mut usize,
    maximum: usize,
    deadline: tokio::time::Instant,
    cancellation: &watch::Receiver<bool>,
) -> Result<(), &'static str> {
    current(cancellation, deadline)?;
    *decoded = decoded
        .checked_add(bytes.len())
        .ok_or("content_too_large")?;
    run.charge(bytes.len()).map_err(|failure| failure.code())?;
    if *decoded > maximum {
        return Err("content_too_large");
    }
    writer
        .write(bytes.to_vec())
        .await
        .map_err(|failure| failure.code())
}
