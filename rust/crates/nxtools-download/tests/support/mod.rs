#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const BODY: &[u8; 16] = b"0123456789abcdef";
const CONCURRENT_BODY_LEN: usize = 256 * 1024;
const IMBALANCED_BODY_LEN: usize = 8 * 1024 * 1024;

pub struct DeterministicHttpServer {
    address: SocketAddr,
    revision: Arc<AtomicU64>,
    resume_transfer_started: Arc<AtomicBool>,
    active_range_transfers: Arc<AtomicU64>,
    max_active_range_transfers: Arc<AtomicU64>,
    transient_range_failures_remaining: Arc<AtomicU64>,
    transient_range_attempts: Arc<AtomicU64>,
    fallback_stream_failures_remaining: Arc<AtomicU64>,
    fallback_stream_attempts: Arc<AtomicU64>,
    validator_range_failures_remaining: Arc<AtomicU64>,
    validator_range_attempts: Arc<AtomicU64>,
    varying_validator_full_stream_attempts: Arc<AtomicU64>,
    dynamic_range_attempts: Arc<AtomicU64>,
    network_available: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    listener_thread: Option<JoinHandle<()>>,
}

impl DeterministicHttpServer {
    pub fn start() -> Self {
        Self::start_with_connection_workers(false)
    }

    pub fn start_concurrent() -> Self {
        Self::start_with_connection_workers(true)
    }

    fn start_with_connection_workers(spawn_connection_workers: bool) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind HTTP fixture");
        listener
            .set_nonblocking(true)
            .expect("make fixture listener non-blocking");
        let address = listener.local_addr().expect("read fixture address");
        let revision = Arc::new(AtomicU64::new(1));
        let resume_transfer_started = Arc::new(AtomicBool::new(false));
        let active_range_transfers = Arc::new(AtomicU64::new(0));
        let max_active_range_transfers = Arc::new(AtomicU64::new(0));
        let transient_range_failures_remaining = Arc::new(AtomicU64::new(1));
        let transient_range_attempts = Arc::new(AtomicU64::new(0));
        let fallback_stream_failures_remaining = Arc::new(AtomicU64::new(1));
        let fallback_stream_attempts = Arc::new(AtomicU64::new(0));
        let validator_range_failures_remaining = Arc::new(AtomicU64::new(1));
        let validator_range_attempts = Arc::new(AtomicU64::new(0));
        let varying_validator_full_stream_attempts = Arc::new(AtomicU64::new(0));
        let dynamic_range_attempts = Arc::new(AtomicU64::new(0));
        let network_available = Arc::new(AtomicBool::new(true));
        let shutdown = Arc::new(AtomicBool::new(false));
        let listener_revision = Arc::clone(&revision);
        let listener_resume_transfer_started = Arc::clone(&resume_transfer_started);
        let listener_active_range_transfers = Arc::clone(&active_range_transfers);
        let listener_max_active_range_transfers = Arc::clone(&max_active_range_transfers);
        let listener_transient_range_failures_remaining =
            Arc::clone(&transient_range_failures_remaining);
        let listener_transient_range_attempts = Arc::clone(&transient_range_attempts);
        let listener_fallback_stream_failures_remaining =
            Arc::clone(&fallback_stream_failures_remaining);
        let listener_fallback_stream_attempts = Arc::clone(&fallback_stream_attempts);
        let listener_validator_range_failures_remaining =
            Arc::clone(&validator_range_failures_remaining);
        let listener_validator_range_attempts = Arc::clone(&validator_range_attempts);
        let listener_varying_validator_full_stream_attempts =
            Arc::clone(&varying_validator_full_stream_attempts);
        let listener_dynamic_range_attempts = Arc::clone(&dynamic_range_attempts);
        let listener_network_available = Arc::clone(&network_available);
        let listener_shutdown = Arc::clone(&shutdown);

        let listener_thread = thread::spawn(move || {
            while !listener_shutdown.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if listener_shutdown.load(Ordering::Acquire) {
                            break;
                        }
                        let revision = Arc::clone(&listener_revision);
                        let resume_transfer_started = Arc::clone(&listener_resume_transfer_started);
                        let active_range_transfers = Arc::clone(&listener_active_range_transfers);
                        let max_active_range_transfers =
                            Arc::clone(&listener_max_active_range_transfers);
                        let transient_range_failures_remaining =
                            Arc::clone(&listener_transient_range_failures_remaining);
                        let transient_range_attempts =
                            Arc::clone(&listener_transient_range_attempts);
                        let fallback_stream_failures_remaining =
                            Arc::clone(&listener_fallback_stream_failures_remaining);
                        let fallback_stream_attempts =
                            Arc::clone(&listener_fallback_stream_attempts);
                        let validator_range_failures_remaining =
                            Arc::clone(&listener_validator_range_failures_remaining);
                        let validator_range_attempts =
                            Arc::clone(&listener_validator_range_attempts);
                        let varying_validator_full_stream_attempts =
                            Arc::clone(&listener_varying_validator_full_stream_attempts);
                        let dynamic_range_attempts = Arc::clone(&listener_dynamic_range_attempts);
                        let network_available = Arc::clone(&listener_network_available);
                        let serve = move || {
                            serve_connection(
                                stream,
                                &revision,
                                &resume_transfer_started,
                                &active_range_transfers,
                                &max_active_range_transfers,
                                &transient_range_failures_remaining,
                                &transient_range_attempts,
                                &fallback_stream_failures_remaining,
                                &fallback_stream_attempts,
                                &validator_range_failures_remaining,
                                &validator_range_attempts,
                                &varying_validator_full_stream_attempts,
                                &dynamic_range_attempts,
                                &network_available,
                            );
                        };
                        if spawn_connection_workers {
                            thread::spawn(serve);
                        } else {
                            serve();
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            address,
            revision,
            resume_transfer_started,
            active_range_transfers,
            max_active_range_transfers,
            transient_range_failures_remaining,
            transient_range_attempts,
            fallback_stream_failures_remaining,
            fallback_stream_attempts,
            validator_range_failures_remaining,
            validator_range_attempts,
            varying_validator_full_stream_attempts,
            dynamic_range_attempts,
            network_available,
            shutdown,
            listener_thread: Some(listener_thread),
        }
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn advance_resource_revision(&self) {
        self.revision.fetch_add(1, Ordering::AcqRel);
    }

    pub fn max_concurrent_range_transfers(&self) -> u64 {
        self.max_active_range_transfers.load(Ordering::Acquire)
    }

    pub fn transient_range_attempts(&self) -> u64 {
        self.transient_range_attempts.load(Ordering::Acquire)
    }

    pub fn set_fallback_stream_failures(&self, failures: u64) {
        self.fallback_stream_failures_remaining
            .store(failures, Ordering::Release);
    }

    pub fn fallback_stream_attempts(&self) -> u64 {
        self.fallback_stream_attempts.load(Ordering::Acquire)
    }

    pub fn validator_range_attempts(&self) -> u64 {
        self.validator_range_attempts.load(Ordering::Acquire)
    }

    pub fn varying_validator_full_stream_attempts(&self) -> u64 {
        self.varying_validator_full_stream_attempts
            .load(Ordering::Acquire)
    }

    pub fn dynamic_range_attempts(&self) -> u64 {
        self.dynamic_range_attempts.load(Ordering::Acquire)
    }

    pub fn set_network_available(&self, available: bool) {
        self.network_available.store(available, Ordering::Release);
    }

    pub fn request(&self, method: &str, path: &str, headers: &[(&str, &str)]) -> RawHttpResponse {
        let mut stream = TcpStream::connect(self.address).expect("connect to HTTP fixture");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set fixture request timeout");

        let mut request =
            format!("{method} {path} HTTP/1.1\r\nHost: fixture\r\nConnection: close\r\n");
        for (name, value) in headers {
            request.push_str(name);
            request.push_str(": ");
            request.push_str(value);
            request.push_str("\r\n");
        }
        request.push_str("\r\n");
        stream
            .write_all(request.as_bytes())
            .expect("write fixture request");

        let mut bytes = Vec::new();
        stream
            .read_to_end(&mut bytes)
            .expect("read fixture response");
        RawHttpResponse::parse(&bytes)
    }
}

impl Drop for DeterministicHttpServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.address);
        if let Some(listener_thread) = self.listener_thread.take() {
            listener_thread.join().expect("join fixture listener");
        }
    }
}

#[derive(Debug)]
pub struct RawHttpResponse {
    pub status: u16,
    headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

impl RawHttpResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    fn parse(bytes: &[u8]) -> Self {
        let header_end = bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("fixture response contains a header terminator");
        let head = std::str::from_utf8(&bytes[..header_end]).expect("fixture headers are UTF-8");
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|value| value.parse::<u16>().ok())
            .expect("fixture response contains a status");
        let headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
            .collect();

        Self {
            status,
            headers,
            body: bytes[header_end + 4..].to_vec(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn serve_connection(
    mut stream: TcpStream,
    revision: &AtomicU64,
    resume_transfer_started: &AtomicBool,
    active_range_transfers: &AtomicU64,
    max_active_range_transfers: &AtomicU64,
    transient_range_failures_remaining: &AtomicU64,
    transient_range_attempts: &AtomicU64,
    fallback_stream_failures_remaining: &AtomicU64,
    fallback_stream_attempts: &AtomicU64,
    validator_range_failures_remaining: &AtomicU64,
    validator_range_attempts: &AtomicU64,
    varying_validator_full_stream_attempts: &AtomicU64,
    dynamic_range_attempts: &AtomicU64,
    network_available: &AtomicBool,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut request_bytes = Vec::new();
    let mut buffer = [0_u8; 1024];

    while request_bytes.len() < 16 * 1024 {
        match stream.read(&mut buffer) {
            Ok(0) => return,
            Ok(read) => {
                request_bytes.extend_from_slice(&buffer[..read]);
                if request_bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => return,
        }
    }

    let request = String::from_utf8_lossy(&request_bytes);
    let mut lines = request.split("\r\n");
    let Some(request_line) = lines.next() else {
        return;
    };
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or("");
    let path = request_parts.next().unwrap_or("");
    let headers: BTreeMap<String, String> = lines
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    match path {
        "/range" => serve_range(&mut stream, method, headers.get("range")),
        "/range-if-range-incompatible" => serve_if_range_incompatible(
            &mut stream,
            method,
            headers.get("range"),
            headers.get("if-range"),
        ),
        "/range-resume-required" => serve_resume_required(
            &mut stream,
            method,
            headers.get("range"),
            headers.get("if-range"),
            resume_transfer_started,
        ),
        "/range-probe-only" => serve_probe_only_range(
            &mut stream,
            method,
            headers.get("range"),
            fallback_stream_failures_remaining,
            fallback_stream_attempts,
            false,
        ),
        "/range-probe-only-unavailable" => serve_probe_only_range(
            &mut stream,
            method,
            headers.get("range"),
            fallback_stream_failures_remaining,
            fallback_stream_attempts,
            true,
        ),
        "/range-mutable" => {
            serve_mutable_range(&mut stream, method, headers.get("range"), revision)
        }
        "/range-concurrent" => serve_concurrent_range(
            &mut stream,
            method,
            headers.get("range"),
            active_range_transfers,
            max_active_range_transfers,
        ),
        "/range-concurrent-varying-validator" => serve_concurrent_varying_validator(
            &mut stream,
            method,
            headers.get("range"),
            active_range_transfers,
            max_active_range_transfers,
            varying_validator_full_stream_attempts,
        ),
        "/range-progress-then-ignored" => {
            serve_range_progress_then_ignored(&mut stream, method, headers.get("range"))
        }
        "/range-concurrent-if-range-incompatible" => {
            if headers.contains_key("if-range") {
                let body = concurrent_fixture_bytes();
                write_response(
                    &mut stream,
                    200,
                    "OK",
                    &[
                        ("Accept-Ranges", "bytes"),
                        ("ETag", "\"conditional-representation\""),
                    ],
                    if method == "HEAD" { b"" } else { &body },
                    Some(body.len()),
                );
            } else {
                serve_concurrent_range(
                    &mut stream,
                    method,
                    headers.get("range"),
                    active_range_transfers,
                    max_active_range_transfers,
                );
            }
        }
        "/range-disconnect" => {
            serve_disconnectable_range(&mut stream, method, headers.get("range"), network_available)
        }
        "/range-imbalanced" => serve_imbalanced_range(
            &mut stream,
            method,
            headers.get("range"),
            dynamic_range_attempts,
        ),
        "/range-transient" => serve_transient_range(
            &mut stream,
            method,
            headers.get("range"),
            transient_range_failures_remaining,
            transient_range_attempts,
        ),
        "/range-validator-replica" => serve_validator_replica_range(
            &mut stream,
            method,
            headers.get("range"),
            validator_range_failures_remaining,
            validator_range_attempts,
        ),
        "/range-always-transient" => serve_always_range_error(
            &mut stream,
            method,
            headers.get("range"),
            transient_range_attempts,
            503,
            "Service Unavailable",
        ),
        "/range-permanent" => serve_always_range_error(
            &mut stream,
            method,
            headers.get("range"),
            transient_range_attempts,
            404,
            "Not Found",
        ),
        "/head-rejected" if method == "HEAD" => {
            write_response(&mut stream, 405, "Method Not Allowed", &[], b"", None);
        }
        "/head-rejected" => serve_range(&mut stream, method, headers.get("range")),
        "/head-only" if method == "HEAD" => write_response(
            &mut stream,
            200,
            "OK",
            &[("ETag", "\"head-only\"")],
            b"",
            Some(BODY.len()),
        ),
        "/head-only" if headers.contains_key("range") => {
            write_response(&mut stream, 416, "Range Not Satisfiable", &[], b"", None);
        }
        "/head-only" => write_response(
            &mut stream,
            200,
            "OK",
            &[("ETag", "\"head-only\"")],
            BODY,
            Some(BODY.len()),
        ),
        "/get-only" if method == "HEAD" => {
            write_response(&mut stream, 405, "Method Not Allowed", &[], b"", None);
        }
        "/get-only" if headers.contains_key("range") => {
            write_response(&mut stream, 416, "Range Not Satisfiable", &[], b"", None);
        }
        "/get-only" => write_response(
            &mut stream,
            200,
            "OK",
            &[("ETag", "\"get-only\"")],
            BODY,
            Some(BODY.len()),
        ),
        "/no-range" => write_response(
            &mut stream,
            200,
            "OK",
            &[("ETag", "\"no-range\"")],
            if method == "HEAD" { b"" } else { BODY },
            Some(BODY.len()),
        ),
        "/advertised-range-ignored" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Accept-Ranges", "bytes"), ("ETag", "\"ignored-range\"")],
            if method == "HEAD" { b"" } else { BODY },
            Some(BODY.len()),
        ),
        "/mismatched-content-range" => write_response(
            &mut stream,
            206,
            "Partial Content",
            &[
                ("Accept-Ranges", "bytes"),
                ("Content-Range", "bytes 5-8/16"),
            ],
            if method == "HEAD" { b"" } else { &BODY[4..=7] },
            Some(4),
        ),
        "/missing-content-range" => write_response(
            &mut stream,
            206,
            "Partial Content",
            &[("Accept-Ranges", "bytes")],
            if method == "HEAD" { b"" } else { &BODY[4..=7] },
            Some(4),
        ),
        "/mismatched-content-length" => write_response(
            &mut stream,
            206,
            "Partial Content",
            &[
                ("Accept-Ranges", "bytes"),
                ("Content-Range", "bytes 4-7/16"),
            ],
            if method == "HEAD" { b"" } else { &BODY[4..=7] },
            Some(5),
        ),
        "/range-validator-changed" if method == "HEAD" => write_response(
            &mut stream,
            200,
            "OK",
            &[
                ("Accept-Ranges", "bytes"),
                ("ETag", "\"fixture-v1\""),
                ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
            ],
            b"",
            Some(BODY.len()),
        ),
        "/range-validator-changed" => {
            serve_range_with_validator(&mut stream, method, headers.get("range"), "\"fixture-v2\"")
        }
        "/redirect" => write_response(
            &mut stream,
            302,
            "Found",
            &[("Location", "/range")],
            b"",
            None,
        ),
        "/drop" => {
            write_response(&mut stream, 200, "OK", &[], &BODY[..5], Some(BODY.len()));
        }
        "/stall" => serve_stall(&mut stream),
        "/chunked" => serve_chunked(&mut stream, method),
        "/status/429" => write_response(
            &mut stream,
            429,
            "Too Many Requests",
            &[("Retry-After", "1")],
            b"retry",
            None,
        ),
        "/status/500" => {
            write_response(
                &mut stream,
                500,
                "Internal Server Error",
                &[],
                b"error",
                None,
            );
        }
        "/mutable" => {
            let revision = revision.load(Ordering::Acquire);
            let etag = format!("\"fixture-v{revision}\"");
            let last_modified = format!("Mon, 27 Jul 2026 00:00:{revision:02} GMT");
            write_response(
                &mut stream,
                200,
                "OK",
                &[
                    ("Accept-Ranges", "bytes"),
                    ("ETag", &etag),
                    ("Last-Modified", &last_modified),
                ],
                if method == "HEAD" { b"" } else { BODY },
                Some(BODY.len()),
            );
        }
        "/named/from-url.bin" => write_response(
            &mut stream,
            200,
            "OK",
            &[
                (
                    "Content-Disposition",
                    "attachment; filename=\"server-name.bin\"",
                ),
                ("Content-Type", "application/octet-stream"),
            ],
            if method == "HEAD" { b"" } else { BODY },
            Some(BODY.len()),
        ),
        _ => write_response(&mut stream, 404, "Not Found", &[], b"missing", None),
    }
    let _ = stream.shutdown(Shutdown::Write);
}

fn serve_range(stream: &mut TcpStream, method: &str, range: Option<&String>) {
    serve_range_with_validator(stream, method, range, "\"fixture-static\"");
}

fn serve_if_range_incompatible(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    if_range: Option<&String>,
) {
    if if_range.is_some() {
        write_response(
            stream,
            200,
            "OK",
            &[
                ("Accept-Ranges", "bytes"),
                ("ETag", "\"conditional-representation\""),
            ],
            if method == "HEAD" { b"" } else { BODY },
            Some(BODY.len()),
        );
        return;
    }
    serve_range_with_validator(stream, method, range, "\"fixture-static\"");
}

fn serve_resume_required(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    if_range: Option<&String>,
    transfer_started: &AtomicBool,
) {
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", "\"resume-static\""),
        ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(BODY.len()));
        return;
    }

    let parsed_range = range.and_then(|value| parse_range(value));
    if parsed_range == Some((0, 0)) {
        let mut headers = common.to_vec();
        headers.push(("Content-Range", "bytes 0-0/16"));
        write_response(
            stream,
            206,
            "Partial Content",
            &headers,
            &BODY[..1],
            Some(1),
        );
        return;
    }

    if let Some((start, end)) = parsed_range
        && start <= end
        && end < BODY.len()
    {
        if start > 0
            && transfer_started.load(Ordering::Acquire)
            && if_range.map(String::as_str) != Some("\"resume-static\"")
        {
            write_response(
                stream,
                428,
                "Precondition Required",
                &common,
                b"resume validator required",
                None,
            );
            return;
        }
        if start == 0 && transfer_started.swap(true, Ordering::AcqRel) {
            write_response(stream, 409, "Conflict", &common, b"restart rejected", None);
            return;
        }
        let content_range = format!("bytes {start}-{end}/{}", BODY.len());
        let mut headers = common.to_vec();
        headers.push(("Content-Range", &content_range));
        write_slow_response(stream, 206, "Partial Content", &headers, &BODY[start..=end]);
        return;
    }

    if transfer_started.swap(true, Ordering::AcqRel) {
        write_response(stream, 409, "Conflict", &common, b"restart rejected", None);
    } else {
        write_slow_response(stream, 200, "OK", &common, BODY);
    }
}

fn serve_probe_only_range(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    fallback_failures_remaining: &AtomicU64,
    fallback_attempts: &AtomicU64,
    respond_unavailable: bool,
) {
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", "\"probe-only\""),
        ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(BODY.len()));
        return;
    }
    if range.and_then(|value| parse_range(value)) == Some((0, 0)) {
        let mut headers = common.to_vec();
        headers.push(("Content-Range", "bytes 0-0/16"));
        write_response(
            stream,
            206,
            "Partial Content",
            &headers,
            &BODY[..1],
            Some(1),
        );
        return;
    }
    if range.is_none() {
        fallback_attempts.fetch_add(1, Ordering::AcqRel);
        if fallback_failures_remaining
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
                remaining.checked_sub(1)
            })
            .is_ok()
        {
            if respond_unavailable {
                write_response(stream, 503, "Service Unavailable", &common, b"", Some(0));
            } else {
                let _ = stream.shutdown(Shutdown::Both);
            }
            return;
        }
    }
    write_response(stream, 200, "OK", &common, BODY, Some(BODY.len()));
}

fn serve_mutable_range(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    revision: &AtomicU64,
) {
    let revision = revision.load(Ordering::Acquire);
    let etag = format!("\"mutable-v{revision}\"");
    let last_modified = format!("Mon, 27 Jul 2026 00:00:{revision:02} GMT");
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", etag.as_str()),
        ("Last-Modified", last_modified.as_str()),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(BODY.len()));
        return;
    }
    if let Some((start, end)) = range.and_then(|value| parse_range(value))
        && start <= end
        && end < BODY.len()
    {
        let content_range = format!("bytes {start}-{end}/{}", BODY.len());
        let mut headers = common.to_vec();
        headers.push(("Content-Range", &content_range));
        if (start, end) == (0, 0) {
            write_response(
                stream,
                206,
                "Partial Content",
                &headers,
                &BODY[..1],
                Some(1),
            );
        } else {
            write_slow_response(stream, 206, "Partial Content", &headers, &BODY[start..=end]);
        }
        return;
    }
    write_response(stream, 200, "OK", &common, BODY, Some(BODY.len()));
}

pub fn concurrent_fixture_bytes() -> Vec<u8> {
    (0..CONCURRENT_BODY_LEN)
        .map(|index| (index % 251) as u8)
        .collect()
}

pub fn imbalanced_fixture_bytes() -> Vec<u8> {
    (0..IMBALANCED_BODY_LEN)
        .map(|index| (index % 251) as u8)
        .collect()
}

fn serve_concurrent_range(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    active: &AtomicU64,
    max_active: &AtomicU64,
) {
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", "\"concurrent-static\""),
        ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(CONCURRENT_BODY_LEN));
        return;
    }
    let body = concurrent_fixture_bytes();
    if let Some((start, end)) = range.and_then(|value| parse_range(value))
        && start <= end
        && end < body.len()
    {
        let content_range = format!("bytes {start}-{end}/{}", body.len());
        let mut headers = common.to_vec();
        headers.push(("Content-Range", &content_range));
        if (start, end) == (0, 0) {
            write_response(
                stream,
                206,
                "Partial Content",
                &headers,
                &body[..1],
                Some(1),
            );
            return;
        }
        let current = active.fetch_add(1, Ordering::AcqRel) + 1;
        max_active.fetch_max(current, Ordering::AcqRel);
        write_throttled_response(stream, 206, "Partial Content", &headers, &body[start..=end]);
        active.fetch_sub(1, Ordering::AcqRel);
        return;
    }
    write_response(stream, 200, "OK", &common, &body, Some(CONCURRENT_BODY_LEN));
}

fn serve_concurrent_varying_validator(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    active: &AtomicU64,
    max_active: &AtomicU64,
    full_stream_attempts: &AtomicU64,
) {
    let base_headers = [
        ("Accept-Ranges", "bytes"),
        ("Last-Modified", "Sat, 01 Aug 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        let mut headers = base_headers.to_vec();
        headers.push(("ETag", "\"probe-replica\""));
        write_response(stream, 200, "OK", &headers, b"", Some(CONCURRENT_BODY_LEN));
        return;
    }

    let body = concurrent_fixture_bytes();
    if let Some((start, end)) = range.and_then(|value| parse_range(value))
        && start <= end
        && end < body.len()
    {
        let validator = if (start / 4096) % 2 == 0 {
            "\"replica-a\""
        } else {
            "\"replica-b\""
        };
        let content_range = format!("bytes {start}-{end}/{}", body.len());
        let mut headers = base_headers.to_vec();
        headers.push(("ETag", validator));
        headers.push(("Content-Range", &content_range));
        if (start, end) == (0, 0) {
            headers[2] = ("ETag", "\"probe-replica\"");
            write_response(
                stream,
                206,
                "Partial Content",
                &headers,
                &body[..1],
                Some(1),
            );
            return;
        }
        let current = active.fetch_add(1, Ordering::AcqRel) + 1;
        max_active.fetch_max(current, Ordering::AcqRel);
        write_throttled_response(stream, 206, "Partial Content", &headers, &body[start..=end]);
        active.fetch_sub(1, Ordering::AcqRel);
        return;
    }

    full_stream_attempts.fetch_add(1, Ordering::AcqRel);
    let mut headers = base_headers.to_vec();
    headers.push(("ETag", "\"full-replica\""));
    write_response(
        stream,
        200,
        "OK",
        &headers,
        &body,
        Some(CONCURRENT_BODY_LEN),
    );
}

fn serve_range_progress_then_ignored(stream: &mut TcpStream, method: &str, range: Option<&String>) {
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", "\"mixed-range-replica\""),
        ("Last-Modified", "Sat, 01 Aug 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(CONCURRENT_BODY_LEN));
        return;
    }

    let body = concurrent_fixture_bytes();
    if let Some((start, end)) = range.and_then(|value| parse_range(value))
        && start <= end
        && end < body.len()
    {
        let content_range = format!("bytes {start}-{end}/{}", body.len());
        let mut headers = common.to_vec();
        headers.push(("Content-Range", &content_range));
        if (start, end) == (0, 0) || start == 0 {
            write_response(
                stream,
                206,
                "Partial Content",
                &headers,
                &body[start..=end],
                Some(end - start + 1),
            );
            return;
        }

        // Let the leading segment become observable before another CDN replica ignores Range.
        // This reproduces the production failure where fallback was incorrectly gated on zero
        // in-flight progress.
        thread::sleep(Duration::from_millis(250));
        write_response(stream, 200, "OK", &common, &body, Some(CONCURRENT_BODY_LEN));
        return;
    }

    write_response(stream, 200, "OK", &common, &body, Some(CONCURRENT_BODY_LEN));
}

fn serve_disconnectable_range(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    network_available: &AtomicBool,
) {
    if !network_available.load(Ordering::Acquire) {
        let _ = stream.shutdown(Shutdown::Both);
        return;
    }
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", "\"disconnect-static\""),
        ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(CONCURRENT_BODY_LEN));
        return;
    }
    let body = concurrent_fixture_bytes();
    if let Some((start, end)) = range.and_then(|value| parse_range(value))
        && start <= end
        && end < body.len()
    {
        let content_range = format!("bytes {start}-{end}/{}", body.len());
        let mut headers = common.to_vec();
        headers.push(("Content-Range", &content_range));
        if (start, end) == (0, 0) {
            write_response(
                stream,
                206,
                "Partial Content",
                &headers,
                &body[..1],
                Some(1),
            );
            return;
        }
        write_disconnectable_response(
            stream,
            206,
            "Partial Content",
            &headers,
            &body[start..=end],
            network_available,
        );
        return;
    }
    write_disconnectable_response(stream, 200, "OK", &common, &body, network_available);
}

fn serve_imbalanced_range(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    attempts: &AtomicU64,
) {
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", "\"imbalanced-static\""),
        ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(IMBALANCED_BODY_LEN));
        return;
    }
    let body = imbalanced_fixture_bytes();
    if let Some((start, end)) = range.and_then(|value| parse_range(value))
        && start <= end
        && end < body.len()
    {
        let content_range = format!("bytes {start}-{end}/{}", body.len());
        let mut headers = common.to_vec();
        headers.push(("Content-Range", &content_range));
        if (start, end) == (0, 0) {
            write_response(
                stream,
                206,
                "Partial Content",
                &headers,
                &body[..1],
                Some(1),
            );
            return;
        }
        attempts.fetch_add(1, Ordering::AcqRel);
        if end < body.len() / 2 {
            write_response(
                stream,
                206,
                "Partial Content",
                &headers,
                &body[start..=end],
                Some(end - start + 1),
            );
        } else {
            write_throttled_response_with_delay(
                stream,
                206,
                "Partial Content",
                &headers,
                &body[start..=end],
                Duration::from_millis(1),
            );
        }
        return;
    }
    write_response(stream, 200, "OK", &common, &body, Some(IMBALANCED_BODY_LEN));
}

fn serve_transient_range(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    failures_remaining: &AtomicU64,
    attempts: &AtomicU64,
) {
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", "\"transient-static\""),
        ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(BODY.len()));
        return;
    }
    let Some((start, end)) = range.and_then(|value| parse_range(value)) else {
        write_response(
            stream,
            503,
            "Service Unavailable",
            &[("Retry-After", "0")],
            b"retry",
            None,
        );
        return;
    };
    if start > end || end >= BODY.len() {
        write_response(stream, 416, "Range Not Satisfiable", &[], b"", None);
        return;
    }
    let content_range = format!("bytes {start}-{end}/{}", BODY.len());
    let mut headers = common.to_vec();
    headers.push(("Content-Range", &content_range));
    if (start, end) == (0, 0) {
        write_response(
            stream,
            206,
            "Partial Content",
            &headers,
            &BODY[..1],
            Some(1),
        );
        return;
    }

    attempts.fetch_add(1, Ordering::AcqRel);
    let should_fail = failures_remaining
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
            remaining.checked_sub(1)
        })
        .is_ok();
    if should_fail {
        write_response(
            stream,
            503,
            "Service Unavailable",
            &[("Retry-After", "0")],
            b"retry",
            None,
        );
    } else {
        write_response(
            stream,
            206,
            "Partial Content",
            &headers,
            &BODY[start..=end],
            Some(end - start + 1),
        );
    }
}

fn serve_validator_replica_range(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    failures_remaining: &AtomicU64,
    attempts: &AtomicU64,
) {
    if method == "HEAD" || range.and_then(|value| parse_range(value)) == Some((0, 0)) {
        serve_range_with_validator(stream, method, range, "\"validator-stable\"");
        return;
    }

    attempts.fetch_add(1, Ordering::AcqRel);
    let mismatching_replica = failures_remaining
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
            remaining.checked_sub(1)
        })
        .is_ok();
    let validator = if mismatching_replica {
        "\"validator-stale-replica\""
    } else {
        "\"validator-stable\""
    };
    serve_range_with_validator(stream, method, range, validator);
}

fn serve_always_range_error(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    attempts: &AtomicU64,
    status: u16,
    reason: &str,
) {
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", "\"always-transient\""),
        ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(BODY.len()));
        return;
    }
    if range.and_then(|value| parse_range(value)) == Some((0, 0)) {
        let headers = [
            ("Accept-Ranges", "bytes"),
            ("ETag", "\"always-transient\""),
            ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
            ("Content-Range", "bytes 0-0/16"),
        ];
        write_response(
            stream,
            206,
            "Partial Content",
            &headers,
            &BODY[..1],
            Some(1),
        );
        return;
    }
    attempts.fetch_add(1, Ordering::AcqRel);
    write_response(
        stream,
        status,
        reason,
        &[("Retry-After", "0")],
        b"retry",
        None,
    );
}

fn serve_range_with_validator(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    etag: &str,
) {
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", etag),
        ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(BODY.len()));
        return;
    }

    if let Some((start, end)) = range.and_then(|value| parse_range(value))
        && start <= end
        && end < BODY.len()
    {
        let content_range = format!("bytes {start}-{end}/{}", BODY.len());
        let mut headers = common.to_vec();
        headers.push(("Content-Range", &content_range));
        write_response(
            stream,
            206,
            "Partial Content",
            &headers,
            &BODY[start..=end],
            None,
        );
        return;
    }

    write_response(stream, 200, "OK", &common, BODY, None);
}

fn parse_range(value: &str) -> Option<(usize, usize)> {
    let range = value.strip_prefix("bytes=")?;
    let (start, end) = range.split_once('-')?;
    Some((start.parse().ok()?, end.parse().ok()?))
}

fn serve_stall(stream: &mut TcpStream) {
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        BODY.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&BODY[..1]);
    let _ = stream.flush();
    thread::sleep(Duration::from_millis(250));
    let _ = stream.write_all(&BODY[1..]);
}

fn write_slow_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) {
    let mut head = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in headers {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    let _ = stream.write_all(head.as_bytes());
    if let Some((first, rest)) = body.split_first() {
        let _ = stream.write_all(std::slice::from_ref(first));
        let _ = stream.flush();
        thread::sleep(Duration::from_millis(250));
        let _ = stream.write_all(rest);
    }
    let _ = stream.flush();
}

fn write_throttled_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) {
    write_throttled_response_with_delay(
        stream,
        status,
        reason,
        headers,
        body,
        Duration::from_millis(20),
    );
}

fn write_throttled_response_with_delay(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
    delay: Duration,
) {
    let mut head = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in headers {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    let _ = stream.write_all(head.as_bytes());
    for chunk in body.chunks(4 * 1024) {
        let _ = stream.write_all(chunk);
        let _ = stream.flush();
        thread::sleep(delay);
    }
}

fn write_disconnectable_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
    network_available: &AtomicBool,
) {
    let mut head = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in headers {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    if stream.write_all(head.as_bytes()).is_err() {
        return;
    }
    for chunk in body.chunks(4 * 1024) {
        if !network_available.load(Ordering::Acquire) {
            let _ = stream.shutdown(Shutdown::Both);
            return;
        }
        if stream.write_all(chunk).is_err() || stream.flush().is_err() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn serve_chunked(stream: &mut TcpStream, method: &str) {
    let head = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
    let _ = stream.write_all(head.as_bytes());
    if method != "HEAD" {
        let _ = stream.write_all(b"4\r\n0123\r\nc\r\n456789abcdef\r\n0\r\n\r\n");
    }
    let _ = stream.flush();
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
    declared_length: Option<usize>,
) {
    let mut response = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in headers {
        response.push_str(name);
        response.push_str(": ");
        response.push_str(value);
        response.push_str("\r\n");
    }
    response.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        declared_length.unwrap_or(body.len())
    ));
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}
