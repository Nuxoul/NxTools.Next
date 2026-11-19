//! Real loopback probe boundary; no external network or user download artifacts.
use nxtools_download::{DownloadRequest, HttpDownloadClient};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

#[tokio::test]
async fn naming_plain_header_utf8_once_plus_literal_and_safe_fallback() {
    let client = HttpDownloadClient::new().expect("HTTP client");
    for (header, expected) in [
        (
            "attachment; filename=\"%E6%8A%A5%E5%91%8A%20final.zip\"",
            "报告 final.zip",
        ),
        ("attachment; filename=\"a+b.zip\"", "a+b.zip"),
        ("attachment; filename=\"a%2520b.zip\"", "a%20b.zip"),
        ("attachment; filename=\"a;b.zip\"", "a;b.zip"),
        ("attachment; filename=\"bad%FF.zip\"", "bad%FF.zip"),
        ("attachment; filename=\"bad%ZZ.zip\"", "bad%ZZ.zip"),
        ("attachment; filename=\"..%2Fbad.exe\"", "url.bin"),
        (
            "attachment; filename=\"plain.zip\"; filename*=UTF-8''%E6%96%B0.zip",
            "新.zip",
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").expect("loopback");
        listener.set_nonblocking(true).expect("bounded accept");
        let address = listener.local_addr().expect("address");
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            for _ in 0..2 {
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error)
                            if error.kind() == std::io::ErrorKind::WouldBlock
                                && Instant::now() < deadline =>
                        {
                            std::thread::sleep(Duration::from_millis(2))
                        }
                        other => panic!("bounded fixture accept: {other:?}"),
                    }
                };
                stream.set_nonblocking(false).expect("accepted blocking IO");
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .expect("read bound");
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .expect("write bound");
                let mut request = Vec::new();
                let mut byte = [0u8; 1];
                while request.len() < 8192 && !request.ends_with(b"\r\n\r\n") {
                    if stream.read(&mut byte).expect("request") == 0 {
                        break;
                    }
                    request.push(byte[0]);
                }
                let head = request.starts_with(b"HEAD ");
                let status = if head {
                    "200 OK"
                } else {
                    "206 Partial Content"
                };
                let length = if head { 16 } else { 1 };
                let range = if head {
                    ""
                } else {
                    "Content-Range: bytes 0-0/16\r\n"
                };
                write!(stream, "HTTP/1.1 {status}\r\nContent-Length: {length}\r\nContent-Disposition: {header}\r\n{range}Connection: close\r\n\r\n").expect("headers");
                if !head {
                    stream.write_all(b"x").expect("range byte");
                }
            }
        });
        let request = DownloadRequest {
            source_url: format!("http://{address}/url.bin"),
            save_directory: std::env::temp_dir(),
            save_directory_explicit: false,
            file_name: None,
            headers: vec![],
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
        let result = client.probe(&request).await;
        server.join().expect("fixture joined");
        assert_eq!(
            result.expect("probe").file_name.as_deref(),
            Some(expected),
            "header fixture {header}"
        );
    }
}
