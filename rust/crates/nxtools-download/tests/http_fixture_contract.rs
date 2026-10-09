mod support;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use support::{DeterministicHttpServer, RawHttpResponse};

#[test]
fn fixture_serves_valid_ranges_and_head_fallback() {
    let server = DeterministicHttpServer::start();

    let ranged = server.request("GET", "/range", &[("Range", "bytes=4-7")]);
    assert_eq!(ranged.status, 206);
    assert_eq!(ranged.header("content-range"), Some("bytes 4-7/16"));
    assert_eq!(ranged.body, b"4567");

    let rejected_head = server.request("HEAD", "/head-rejected", &[]);
    assert_eq!(rejected_head.status, 405);
    let fallback = server.request("GET", "/head-rejected", &[("Range", "bytes=0-0")]);
    assert_eq!(fallback.status, 206);
    assert_eq!(fallback.body, b"0");
}

#[test]
fn fixture_exposes_no_range_redirect_and_error_responses() {
    let server = DeterministicHttpServer::start();

    let ignored_range = server.request("GET", "/no-range", &[("Range", "bytes=4-7")]);
    assert_eq!(ignored_range.status, 200);
    assert_eq!(ignored_range.body, b"0123456789abcdef");
    assert_eq!(ignored_range.header("accept-ranges"), None);

    let redirect = server.request("GET", "/redirect", &[]);
    assert_eq!(redirect.status, 302);
    assert_eq!(redirect.header("location"), Some("/range"));

    assert_eq!(server.request("GET", "/status/429", &[]).status, 429);
    assert_eq!(server.request("GET", "/status/500", &[]).status, 500);
    assert_eq!(server.request("GET", "/missing", &[]).status, 404);
}

#[test]
fn fixture_distinguishes_range_advertising_from_range_compliance() {
    let server = DeterministicHttpServer::start();

    let ignored = server.request(
        "GET",
        "/advertised-range-ignored",
        &[("Range", "bytes=4-7")],
    );
    assert_eq!(ignored.status, 200);
    assert_eq!(ignored.header("accept-ranges"), Some("bytes"));
    assert_eq!(ignored.body, b"0123456789abcdef");

    let mismatched = server.request(
        "GET",
        "/mismatched-content-range",
        &[("Range", "bytes=4-7")],
    );
    assert_eq!(mismatched.status, 206);
    assert_eq!(mismatched.header("content-range"), Some("bytes 5-8/16"));
    assert_eq!(mismatched.body, b"4567");
}

#[test]
fn fixture_can_serve_an_unknown_length_chunked_response() {
    let server = DeterministicHttpServer::start();

    let chunked = server.request("GET", "/chunked", &[]);

    assert_eq!(chunked.status, 200);
    assert_eq!(chunked.header("content-length"), None);
    assert_eq!(chunked.header("transfer-encoding"), Some("chunked"));
    assert_eq!(chunked.body, b"4\r\n0123\r\nc\r\n456789abcdef\r\n0\r\n\r\n");
}

#[test]
fn fixture_can_drop_and_stall_a_response_deterministically() {
    let server = DeterministicHttpServer::start();

    let dropped = server.request("GET", "/drop", &[]);
    assert_eq!(dropped.status, 200);
    assert_eq!(dropped.header("content-length"), Some("16"));
    assert_eq!(dropped.body, b"01234");

    let mut stream = TcpStream::connect(server.address()).expect("connect to fixture");
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("set read timeout");
    stream
        .write_all(b"GET /stall HTTP/1.1\r\nHost: fixture\r\nConnection: close\r\n\r\n")
        .expect("write request");

    let mut received = Vec::new();
    let mut buffer = [0_u8; 256];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => panic!("stall route closed before stalling"),
            Ok(read) => {
                received.extend_from_slice(&buffer[..read]);
                let Some(header_end) = received.windows(4).position(|window| window == b"\r\n\r\n")
                else {
                    continue;
                };
                if received.len() > header_end + 4 {
                    break;
                }
            }
            Err(error) => panic!("unexpected fixture read error: {error}"),
        }
    }

    stream
        .set_read_timeout(Some(Duration::from_millis(50)))
        .expect("set stall timeout");
    let timed_out = match stream.read(&mut buffer) {
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            true
        }
        Ok(_) => false,
        Err(error) => panic!("unexpected fixture read error after headers: {error}"),
    };
    assert!(timed_out, "stall route must exceed the client timeout");
}

#[test]
fn fixture_can_change_resource_validators_between_requests() {
    let server = DeterministicHttpServer::start();

    let first: RawHttpResponse = server.request("HEAD", "/mutable", &[]);
    assert_eq!(first.header("etag"), Some("\"fixture-v1\""));
    assert_eq!(
        first.header("last-modified"),
        Some("Mon, 27 Jul 2026 00:00:01 GMT")
    );

    server.advance_resource_revision();

    let second = server.request("HEAD", "/mutable", &[]);
    assert_eq!(second.header("etag"), Some("\"fixture-v2\""));
    assert_eq!(
        second.header("last-modified"),
        Some("Mon, 27 Jul 2026 00:00:02 GMT")
    );
}

#[test]
fn fixture_preserves_accepted_connections_until_delayed_request_bytes_arrive() {
    for concurrent in [false, true] {
        let server = if concurrent {
            DeterministicHttpServer::start_concurrent()
        } else {
            DeterministicHttpServer::start()
        };
        let mut stream =
            TcpStream::connect(server.address()).expect("connect delayed fixture client");
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("bound delayed fixture response");
        // Give the accept/read boundary a turn before any HTTP bytes exist.
        std::thread::sleep(Duration::from_millis(50));
        stream
            .write_all(b"GET /range HTTP/1.1\r\nHost: fixture\r\nRange: bytes=4-7\r\nConnection: close\r\n\r\n")
            .expect("accepted fixture connection must survive delayed request bytes");
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .expect("read delayed fixture response");
        assert!(
            response.starts_with(b"HTTP/1.1 206 "),
            "accepted fixture connection lost before headers; concurrent={concurrent}, response_bytes={}",
            response.len()
        );
        assert!(response.ends_with(b"4567"));
        assert_eq!(server.total_requests(), 1);
    }
}
