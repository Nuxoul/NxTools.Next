use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use nxtools_download::{
    DownloadProbe, DownloadRequest, DownloadSegment, DownloadSegmentState,
    DownloadSegmentTransferControl, DownloadSegmentTransferRequest, DownloadTaskId,
    DownloadTransferContext, DownloadTransferControl, HttpDownloadClient, HttpDownloadError,
    ResourceValidator, SegmentedArtifact,
};

#[tokio::test]
async fn rejected_segment_status_is_not_an_ignored_range_and_never_writes() {
    for status in [500, 429, 503] {
        let listener = TcpListener::bind("127.0.0.1:0").expect("owned HTTP listener");
        let address = listener.local_addr().expect("local address");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("segment connection");
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .expect("bounded read");
            let mut request = Vec::new();
            let mut byte = [0_u8; 1];
            while !request.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).expect("request headers");
                request.push(byte[0]);
            }
            write!(stream, "HTTP/1.1 {status} Busy\r\nContent-Length: 0\r\nRetry-After: 1\r\nConnection: close\r\n\r\n").expect("status response");
        });
        let root = tempfile::tempdir().expect("owned output");
        let request = DownloadRequest {
            source_url: format!("http://{address}/file"),
            save_directory: root.path().to_owned(),
            save_directory_explicit: true,
            file_name: Some("status.bin".into()),
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
        let probe = DownloadProbe {
            final_url: request.source_url.clone(),
            file_name: request.file_name.clone(),
            total_bytes: Some(16),
            supports_ranges: true,
            content_type: None,
            validator: ResourceValidator::default(),
            media: None,
            resolution: None,
            server_observation: None,
        };
        let path = root.path().join("partial.bin");
        let artifact = SegmentedArtifact::create(&path, 16)
            .await
            .expect("artifact");
        let segment =
            DownloadSegment::new(0, 0, 15, 0, DownloadSegmentState::Active).expect("range");
        let control = DownloadSegmentTransferControl::new(DownloadTransferControl::new());
        let failure = HttpDownloadClient::new()
            .expect("HTTP client")
            .download_segment(
                DownloadSegmentTransferRequest::new(
                    DownloadTransferContext::new(DownloadTaskId::new(), uuid::Uuid::new_v4()),
                    &request,
                    &probe,
                    &segment,
                    &artifact,
                    &control,
                ),
                |_| {},
            )
            .await
            .expect_err("status rejects transfer");
        server.join().expect("actual server exit");
        assert_eq!(
            tokio::fs::read(&path).await.expect("artifact bytes"),
            vec![0; 16]
        );
        assert!(
            !matches!(failure, HttpDownloadError::RangeNotHonored { .. }),
            "HTTP {status} is not an ignored range: {failure:?}"
        );
        if status == 500 {
            assert!(matches!(failure, HttpDownloadError::Status(500)));
        } else {
            assert!(
                failure.to_string().contains("服务器繁忙"),
                "busy response: {failure}"
            );
        }
    }
}
