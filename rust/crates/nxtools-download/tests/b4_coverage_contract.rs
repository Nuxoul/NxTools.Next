mod support;

use nxtools_download::{
    DownloadDuplicateStrategy, DownloadEngine, DownloadEvent, DownloadRequest, DownloadStatus,
    RequestOrigin,
};
use std::time::Duration;
use support::{DeterministicHttpServer, imbalanced_fixture_bytes};

#[tokio::test]
async fn b4_ref10_real_parallel_rebalance_progress_carries_true_range_offsets() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("owned writer output");
    let engine = DownloadEngine::open_in_memory().expect("original engine");
    let events = engine.take_event_receiver().expect("original event stream");
    let task_id = engine
        .submit(
            DownloadRequest {
                source_url: format!("http://{}/range-imbalanced", server.address()),
                save_directory: directory.path().to_owned(),
                save_directory_explicit: false,
                file_name: Some("coverage.bin".into()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                source_page: None,
                certificate_policy: Default::default(),
                connection_limit: Some(2),
                speed_limit_bytes_per_second: None,
                expected_total_bytes: None,
                scheduled_at_unix_ms: None,
                adaptive_segment_policy: None,
                media: None,
                category_id: None,
                category_name: None,
                duplicate_strategy: DownloadDuplicateStrategy::UseDefault,
            },
            RequestOrigin::Ui,
        )
        .expect("parallel writer admission");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(8))
        .await
        .expect("writer completed");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "{:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(completed.final_path.expect("final output")).expect("writer bytes"),
        imbalanced_fixture_bytes()
    );
    assert!(
        server.dynamic_range_attempts() >= 4,
        "real tail rebalance prerequisite"
    );
    let mut snapshots = Vec::new();
    while let Ok(envelope) = events.try_recv() {
        if let DownloadEvent::TaskProgress { progress } = envelope.event
            && progress.task_id == task_id
            && progress.segments.len() >= 2
        {
            snapshots.push(progress);
        }
    }
    assert!(!snapshots.is_empty(), "original parallel progress stream");
    assert!(
        snapshots
            .iter()
            .any(|progress| progress.downloaded_bytes > 0
                && Some(progress.downloaded_bytes) != progress.total_bytes),
        "written partial progress prerequisite"
    );
    for progress in snapshots {
        let total = progress.total_bytes.expect("real known total");
        let mut ranges = Vec::new();
        for segment in progress.segments {
            let value = serde_json::to_value(&segment).expect("actual wire projection");
            let start = value["start_byte"].as_u64().expect("live written projection must carry true plan start, never infer from segment index");
            let length = segment.total_bytes.expect("actual segment length");
            assert!(length > 0 && start < total && length <= total - start);
            assert!(
                segment.downloaded_bytes <= length,
                "allocation is not extra written progress"
            );
            ranges.push((start, start + length));
        }
        ranges.sort_unstable();
        assert!(
            ranges.windows(2).all(|pair| pair[0].1 <= pair[1].0),
            "rebalance projects nonoverlapping current plan ranges"
        );
    }
}
