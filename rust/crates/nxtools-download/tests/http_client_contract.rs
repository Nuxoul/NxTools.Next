mod support;

use std::path::PathBuf;
use std::time::Duration;

use nxtools_download::{
    DownloadMediaProbe, DownloadProbe, DownloadRequest, DownloadSegment, DownloadSegmentState,
    DownloadSegmentTransferControl, DownloadSegmentTransferOutcome, DownloadSegmentTransferRequest,
    DownloadTaskId, DownloadTransferContext, DownloadTransferControl, DownloadTransferOutcome,
    HttpDownloadClient, HttpDownloadError, ResourceValidator, SegmentedArtifact,
};
use support::{DeterministicHttpServer, concurrent_fixture_bytes};

static HTTP_CONTRACT_GATE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn request(server: &DeterministicHttpServer, path: &str) -> DownloadRequest {
    DownloadRequest {
        source_url: format!("http://{}{path}", server.address()),
        save_directory: PathBuf::from(r"D:\Downloads"),
        file_name: None,
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        connection_limit: Some(1),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
    }
}

fn single_stream_probe(request: &DownloadRequest) -> DownloadProbe {
    DownloadProbe {
        final_url: request.source_url.clone(),
        file_name: request.file_name.clone(),
        total_bytes: Some(16),
        supports_ranges: false,
        content_type: None,
        validator: ResourceValidator::default(),
        media: None,
    }
}

#[tokio::test]
async fn probe_resolves_and_orders_hls_master_variants() {
    let _guard = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let client = HttpDownloadClient::new().expect("create HTTP client");

    let probe = client
        .probe(&request(&server, "/hls/master.m3u8"))
        .await
        .expect("probe HLS master playlist");
    let DownloadMediaProbe::Hls(hls) = probe.media.expect("HLS probe metadata");

    assert!(hls.is_master);
    assert_eq!(hls.variants.len(), 2);
    assert_eq!(hls.variants[0].resolution, Some((1920, 1080)));
    assert_eq!(hls.variants[0].bandwidth, 4_200_000);
    assert_eq!(hls.variants[0].average_bandwidth, Some(3_900_000));
    assert_eq!(hls.variants[0].frame_rate_milli, Some(60_000));
    assert_eq!(hls.variants[0].codecs, "avc1.640028,mp4a.40.2");
    assert_eq!(
        hls.variants[0].playlist_url,
        format!("http://{}/hls/high/index.m3u8", server.address())
    );
    assert_eq!(hls.variants[1].resolution, Some((640, 360)));
    assert_eq!(hls.variants[1].frame_rate_milli, Some(29_970));
}

#[tokio::test]
async fn probe_rejects_an_hls_manifest_above_the_bounded_body_limit() {
    let _guard = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let client = HttpDownloadClient::new().expect("create HTTP client");

    let error = client
        .probe(&request(&server, "/hls/too-large.m3u8"))
        .await
        .expect_err("reject oversized HLS manifest");

    assert!(matches!(error, HttpDownloadError::HlsManifestTooLarge));
}

#[tokio::test]
async fn probe_rejects_an_hls_playlist_without_variants_or_media_segments() {
    let _guard = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let client = HttpDownloadClient::new().expect("create HTTP client");

    let error = client
        .probe(&request(&server, "/hls/empty.m3u8"))
        .await
        .expect_err("reject empty HLS playlist");

    assert!(matches!(
        error,
        HttpDownloadError::InvalidHlsManifest("playlist contains no playable entries")
    ));
}

#[tokio::test]
async fn pause_before_single_stream_preparation_creates_no_files() {
    let directory = tempfile::tempdir().expect("create test root");
    let save_directory = directory.path().join("not-created");
    let request = DownloadRequest {
        source_url: "http://127.0.0.1:9/never-requested".into(),
        save_directory: save_directory.clone(),
        file_name: Some("fixture.bin".into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        connection_limit: Some(1),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: Some(16),
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
    };
    let client = HttpDownloadClient::new().expect("create HTTP client");
    let control = DownloadTransferControl::new();
    control.pause();

    let outcome = client
        .download(
            &request,
            &single_stream_probe(&request),
            DownloadTaskId::new(),
            &control,
            |_| {},
        )
        .await
        .expect("honor pause before preparation");

    assert!(matches!(outcome, DownloadTransferOutcome::Paused { .. }));
    assert!(
        !save_directory.exists(),
        "an already-paused transfer must not prepare the destination"
    );
}

#[tokio::test]
async fn cancel_before_single_stream_preparation_creates_no_files() {
    let directory = tempfile::tempdir().expect("create test root");
    let save_directory = directory.path().join("not-created");
    let request = DownloadRequest {
        source_url: "http://127.0.0.1:9/never-requested".into(),
        save_directory: save_directory.clone(),
        file_name: Some("fixture.bin".into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        connection_limit: Some(1),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: Some(16),
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
    };
    let client = HttpDownloadClient::new().expect("create HTTP client");
    let control = DownloadTransferControl::new();
    control.cancel();

    let outcome = client
        .download(
            &request,
            &single_stream_probe(&request),
            DownloadTaskId::new(),
            &control,
            |_| {},
        )
        .await
        .expect("honor cancel before preparation");

    assert!(matches!(outcome, DownloadTransferOutcome::Cancelled { .. }));
    assert!(
        !save_directory.exists(),
        "an already-cancelled transfer must not prepare the destination"
    );
}

#[tokio::test]
async fn download_streams_into_a_task_temporary_file_before_publishing_the_final_path() {
    let _contract = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create destination");
    let mut request = request(&server, "/range");
    request.save_directory = directory.path().to_path_buf();
    request.file_name = Some("fixture.bin".to_owned());
    let client = HttpDownloadClient::new().expect("create HTTP client");
    let probe = client.probe(&request).await.expect("probe fixture");
    let task_id = DownloadTaskId::new();
    let control = DownloadTransferControl::new();

    let outcome = client
        .download(&request, &probe, task_id, &control, |_| {})
        .await
        .expect("download fixture");

    let DownloadTransferOutcome::Completed {
        final_path,
        downloaded_bytes,
    } = outcome
    else {
        panic!("uninterrupted transfer must complete");
    };
    assert_eq!(final_path, directory.path().join("fixture.bin"));
    assert_eq!(downloaded_bytes, 16);
    assert_eq!(
        std::fs::read(&final_path).expect("read completed artifact"),
        b"0123456789abcdef"
    );
    assert_eq!(
        std::fs::read_dir(directory.path())
            .expect("read destination")
            .count(),
        1,
        "temporary artifact must be gone after the final rename"
    );
}

#[tokio::test]
async fn a_valid_partial_response_is_written_only_inside_its_segment_range() {
    let _contract = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create destination");
    let request = request(&server, "/range");
    let client = HttpDownloadClient::new().expect("create HTTP client");
    let probe = client.probe(&request).await.expect("probe fixture");
    let artifact = SegmentedArtifact::create(directory.path().join(".fixture.part"), 16)
        .await
        .expect("create segmented artifact");
    let segment =
        DownloadSegment::new(1, 4, 7, 0, DownloadSegmentState::Active).expect("valid segment");

    let downloaded = client
        .download_segment(
            DownloadSegmentTransferRequest::new(
                DownloadTransferContext::new(DownloadTaskId::new(), uuid::Uuid::new_v4()),
                &request,
                &probe,
                &segment,
                &artifact,
                &DownloadSegmentTransferControl::new(DownloadTransferControl::new()),
            ),
            |_| {},
        )
        .await
        .expect("download valid range");

    assert_eq!(
        downloaded,
        DownloadSegmentTransferOutcome::Completed {
            downloaded_bytes: 4
        }
    );
    assert_eq!(
        tokio::fs::read(artifact.path())
            .await
            .expect("read segmented artifact"),
        [&[0, 0, 0, 0][..], b"4567", &[0, 0, 0, 0, 0, 0, 0, 0][..]].concat()
    );
}

#[tokio::test]
async fn a_fresh_segment_does_not_send_if_range_to_an_incompatible_range_server() {
    let _contract = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create destination");
    let request = request(&server, "/range-if-range-incompatible");
    let client = HttpDownloadClient::new().expect("create HTTP client");
    let probe = client.probe(&request).await.expect("probe fixture");
    let artifact = SegmentedArtifact::create(directory.path().join(".fixture.part"), 16)
        .await
        .expect("create segmented artifact");
    let segment =
        DownloadSegment::new(1, 4, 7, 0, DownloadSegmentState::Active).expect("valid segment");

    let outcome = client
        .download_segment(
            DownloadSegmentTransferRequest::new(
                DownloadTransferContext::new(DownloadTaskId::new(), uuid::Uuid::new_v4()),
                &request,
                &probe,
                &segment,
                &artifact,
                &DownloadSegmentTransferControl::new(DownloadTransferControl::new()),
            ),
            |_| {},
        )
        .await
        .expect("fresh range must not be converted into an incompatible conditional request");

    assert_eq!(
        outcome,
        DownloadSegmentTransferOutcome::Completed {
            downloaded_bytes: 4
        }
    );
    assert_eq!(
        tokio::fs::read(artifact.path())
            .await
            .expect("read segmented artifact"),
        [&[0, 0, 0, 0][..], b"4567", &[0, 0, 0, 0, 0, 0, 0, 0][..]].concat()
    );
}

#[tokio::test]
async fn rebalancing_a_segment_stops_after_a_durable_prefix_without_publishing_completion() {
    let _contract = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create destination");
    let request = request(&server, "/range-concurrent");
    let client = HttpDownloadClient::new().expect("create HTTP client");
    let probe = client.probe(&request).await.expect("probe fixture");
    let expected = concurrent_fixture_bytes();
    let artifact = SegmentedArtifact::create(
        directory.path().join(".rebalance.part"),
        expected.len() as u64,
    )
    .await
    .expect("create segmented artifact");
    let segment = DownloadSegment::new(
        0,
        0,
        expected.len() as u64 - 1,
        0,
        DownloadSegmentState::Active,
    )
    .expect("valid segment");
    let control = DownloadSegmentTransferControl::new(DownloadTransferControl::new());
    let rebalance_control = control.clone();

    let outcome = client
        .download_segment(
            DownloadSegmentTransferRequest::new(
                DownloadTransferContext::new(DownloadTaskId::new(), uuid::Uuid::new_v4()),
                &request,
                &probe,
                &segment,
                &artifact,
                &control,
            ),
            move |progress| {
                if progress.downloaded_bytes > 0 {
                    rebalance_control.request_rebalance();
                }
            },
        )
        .await
        .expect("rebalance segment");

    let DownloadSegmentTransferOutcome::Rebalanced { downloaded_bytes } = outcome else {
        panic!("internal rebalance must not report segment completion");
    };
    assert!(downloaded_bytes > 0);
    assert!(downloaded_bytes < segment.len());
    let artifact_bytes = tokio::fs::read(artifact.path())
        .await
        .expect("read rebalanced artifact");
    assert_eq!(
        &artifact_bytes[..downloaded_bytes as usize],
        &expected[..downloaded_bytes as usize]
    );
    assert!(
        artifact_bytes[downloaded_bytes as usize..]
            .iter()
            .all(|byte| *byte == 0),
        "worker must stop before writing the future split tail"
    );
}

#[tokio::test]
async fn malformed_or_ignored_ranges_are_rejected_before_the_artifact_is_changed() {
    let _contract = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create destination");
    let client = HttpDownloadClient::new().expect("create HTTP client");
    let segment =
        DownloadSegment::new(0, 4, 7, 0, DownloadSegmentState::Active).expect("valid segment");

    for (path, expected_error) in [
        ("/mismatched-content-range", "content_range"),
        ("/advertised-range-ignored", "ignored"),
        ("/missing-content-range", "missing_content_range"),
        ("/mismatched-content-length", "content_length"),
    ] {
        let artifact_path = directory
            .path()
            .join(format!("{}.part", path.trim_start_matches('/')));
        let artifact = SegmentedArtifact::create(&artifact_path, 16)
            .await
            .expect("create segmented artifact");
        let request = request(&server, path);
        let probe = DownloadProbe {
            final_url: request.source_url.clone(),
            file_name: Some("fixture.bin".to_owned()),
            total_bytes: Some(16),
            supports_ranges: true,
            content_type: Some("application/octet-stream".to_owned()),
            validator: if path == "/advertised-range-ignored" {
                ResourceValidator {
                    etag: Some("\"ignored-range\"".to_owned()),
                    last_modified: None,
                }
            } else {
                ResourceValidator::default()
            },
            media: None,
        };

        let error = client
            .download_segment(
                DownloadSegmentTransferRequest::new(
                    DownloadTransferContext::new(DownloadTaskId::new(), uuid::Uuid::new_v4()),
                    &request,
                    &probe,
                    &segment,
                    &artifact,
                    &DownloadSegmentTransferControl::new(DownloadTransferControl::new()),
                ),
                |_| {},
            )
            .await
            .expect_err("untrustworthy range must fail");

        match expected_error {
            "content_range" => {
                assert!(matches!(
                    error,
                    HttpDownloadError::ContentRangeMismatch { .. }
                ))
            }
            "ignored" => assert!(matches!(
                error,
                HttpDownloadError::RangeNotHonored { status: 200 }
            )),
            "missing_content_range" => {
                assert!(matches!(error, HttpDownloadError::MissingContentRange))
            }
            "content_length" => {
                assert!(matches!(
                    error,
                    HttpDownloadError::ContentLengthMismatch {
                        expected_bytes: 4,
                        actual_bytes: 5,
                    }
                ))
            }
            _ => unreachable!(),
        }
        assert_eq!(
            tokio::fs::read(&artifact_path)
                .await
                .expect("read unchanged artifact"),
            vec![0; 16]
        );
    }
}

#[tokio::test]
async fn a_changed_resource_validator_is_rejected_before_segment_writes() {
    let _contract = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create destination");
    let request = request(&server, "/range-validator-changed");
    let client = HttpDownloadClient::new().expect("create HTTP client");
    let mut probe = client
        .probe(&request)
        .await
        .expect("probe first resource version");
    probe.validator = ResourceValidator {
        etag: Some("\"fixture-v1\"".to_owned()),
        last_modified: Some("Mon, 27 Jul 2026 00:00:00 GMT".to_owned()),
    };
    let artifact = SegmentedArtifact::create(directory.path().join(".fixture.part"), 16)
        .await
        .expect("create segmented artifact");
    let segment =
        DownloadSegment::new(0, 4, 7, 0, DownloadSegmentState::Active).expect("valid segment");

    let error = client
        .download_segment(
            DownloadSegmentTransferRequest::new(
                DownloadTransferContext::new(DownloadTaskId::new(), uuid::Uuid::new_v4()),
                &request,
                &probe,
                &segment,
                &artifact,
                &DownloadSegmentTransferControl::new(DownloadTransferControl::new()),
            )
            .with_resume_validator(),
            |_| {},
        )
        .await
        .expect_err("changed validator must fail");

    assert!(matches!(
        error,
        HttpDownloadError::ResourceValidatorMismatch {
            validator: "etag",
            ..
        }
    ));
    assert_eq!(
        tokio::fs::read(artifact.path())
            .await
            .expect("read unchanged artifact"),
        vec![0; 16]
    );
}

#[tokio::test]
async fn pause_stops_the_writer_and_preserves_only_the_temporary_artifact() {
    let _contract = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create destination");
    let mut request = request(&server, "/stall");
    request.save_directory = directory.path().to_path_buf();
    request.file_name = Some("paused.bin".to_owned());
    let client = HttpDownloadClient::with_stall_timeout(Duration::from_millis(300))
        .expect("create HTTP client");
    let probe = client.probe(&request).await.expect("probe fixture");
    let task_id = DownloadTaskId::new();
    let control = DownloadTransferControl::new();
    let progress_control = control.clone();

    let outcome = client
        .download(&request, &probe, task_id, &control, move |progress| {
            if progress.downloaded_bytes > 0 {
                progress_control.pause();
            }
        })
        .await
        .expect("pause fixture");

    let DownloadTransferOutcome::Paused {
        temporary_path,
        downloaded_bytes,
    } = outcome
    else {
        panic!("pause request must not publish the final artifact");
    };
    assert!(downloaded_bytes > 0);
    assert!(temporary_path.exists());
    assert_eq!(
        std::fs::metadata(&temporary_path)
            .expect("inspect paused artifact")
            .len(),
        downloaded_bytes
    );
    assert!(!directory.path().join("paused.bin").exists());
}

#[tokio::test]
async fn cancel_stops_the_writer_and_removes_the_temporary_artifact() {
    let _contract = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create destination");
    let mut request = request(&server, "/stall");
    request.save_directory = directory.path().to_path_buf();
    request.file_name = Some("cancelled.bin".to_owned());
    let client = HttpDownloadClient::new().expect("create HTTP client");
    let probe = client.probe(&request).await.expect("probe fixture");
    let control = DownloadTransferControl::new();
    let progress_control = control.clone();

    let outcome = client
        .download(
            &request,
            &probe,
            DownloadTaskId::new(),
            &control,
            move |progress| {
                if progress.downloaded_bytes > 0 {
                    progress_control.cancel();
                }
            },
        )
        .await
        .expect("cancel fixture");

    assert!(matches!(
        outcome,
        DownloadTransferOutcome::Cancelled {
            downloaded_bytes
        } if downloaded_bytes > 0
    ));
    assert_eq!(
        std::fs::read_dir(directory.path())
            .expect("read destination")
            .count(),
        0,
        "cancel must remove the task-owned temporary artifact"
    );
}

#[tokio::test]
async fn a_stalled_response_times_out_without_publishing_the_final_artifact() {
    let _contract = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create destination");
    let mut request = request(&server, "/stall");
    request.save_directory = directory.path().to_path_buf();
    request.file_name = Some("stalled.bin".to_owned());
    let client = HttpDownloadClient::with_stall_timeout(Duration::from_millis(50))
        .expect("create HTTP client");
    let probe = client.probe(&request).await.expect("probe fixture");

    let error = client
        .download(
            &request,
            &probe,
            DownloadTaskId::new(),
            &DownloadTransferControl::new(),
            |_| {},
        )
        .await
        .expect_err("stalled response must fail");

    assert!(matches!(error, HttpDownloadError::Stalled { .. }));
    assert!(!directory.path().join("stalled.bin").exists());
}

#[tokio::test]
async fn a_truncated_response_never_publishes_the_final_artifact() {
    let _contract = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create destination");
    let mut request = request(&server, "/drop");
    request.save_directory = directory.path().to_path_buf();
    request.file_name = Some("truncated.bin".to_owned());
    let client = HttpDownloadClient::new().expect("create HTTP client");
    let probe = client.probe(&request).await.expect("probe fixture");

    let result = client
        .download(
            &request,
            &probe,
            DownloadTaskId::new(),
            &DownloadTransferControl::new(),
            |_| {},
        )
        .await;

    assert!(result.is_err());
    assert!(!directory.path().join("truncated.bin").exists());
    let artifacts = std::fs::read_dir(directory.path())
        .expect("read destination")
        .collect::<Result<Vec<_>, _>>()
        .expect("read destination entries");
    assert_eq!(artifacts.len(), 1);
    assert!(
        artifacts[0]
            .file_name()
            .to_string_lossy()
            .ends_with(".nxtools-part"),
        "failed transfer must remain recoverable as a temporary artifact"
    );
}

#[tokio::test]
async fn probe_follows_redirects_and_returns_real_response_metadata() {
    let _contract = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let client = HttpDownloadClient::new().expect("create HTTP client");

    let probe = client
        .probe(&request(&server, "/redirect"))
        .await
        .expect("probe redirected fixture");

    assert_eq!(
        probe.final_url,
        format!("http://{}/range", server.address())
    );
    assert_eq!(probe.total_bytes, Some(16));
    assert!(probe.supports_ranges);
    assert_eq!(
        probe.validator,
        ResourceValidator {
            etag: Some("\"fixture-static\"".to_owned()),
            last_modified: Some("Mon, 27 Jul 2026 00:00:00 GMT".to_owned()),
        }
    );
}

#[tokio::test]
async fn probe_prefers_a_safe_content_disposition_name_and_falls_back_when_head_is_rejected() {
    let _contract = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let client = HttpDownloadClient::new().expect("create HTTP client");

    let named = client
        .probe(&request(&server, "/named/from-url.bin"))
        .await
        .expect("probe named fixture");
    assert_eq!(named.file_name.as_deref(), Some("server-name.bin"));
    assert_eq!(
        named.content_type.as_deref(),
        Some("application/octet-stream")
    );

    let fallback = client
        .probe(&request(&server, "/head-rejected"))
        .await
        .expect("probe fixture with rejected HEAD");
    assert_eq!(fallback.total_bytes, Some(16));
    assert!(fallback.supports_ranges);
}

#[tokio::test]
async fn probe_uses_successful_head_when_the_server_rejects_the_range_probe() {
    let _contract = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let client = HttpDownloadClient::new().expect("create HTTP client");

    let probe = client
        .probe(&request(&server, "/head-only"))
        .await
        .expect("HEAD metadata must keep the probe usable");

    assert_eq!(probe.total_bytes, Some(16));
    assert!(!probe.supports_ranges);
    assert_eq!(probe.validator.etag.as_deref(), Some("\"head-only\""));
}

#[tokio::test]
async fn probe_falls_back_to_plain_get_when_head_and_range_are_both_rejected() {
    let _contract = HTTP_CONTRACT_GATE.lock().await;
    let server = DeterministicHttpServer::start();
    let client = HttpDownloadClient::new().expect("create HTTP client");

    let probe = client
        .probe(&request(&server, "/get-only"))
        .await
        .expect("plain GET must be the final metadata fallback");

    assert_eq!(probe.total_bytes, Some(16));
    assert!(!probe.supports_ranges);
    assert_eq!(probe.validator.etag.as_deref(), Some("\"get-only\""));
}
