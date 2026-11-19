use super::*;
use crate::{DownloadSegment, DownloadSegmentState};

#[test]
fn retry_after_accepts_seconds_http_dates_and_expired_dates() {
    assert_eq!(parse_retry_after(" 17 "), Some(Duration::from_secs(17)));
    assert_eq!(parse_retry_after("not a date"), None);
    assert_eq!(
        parse_retry_after("Wed, 21 Oct 2015 07:28:00 GMT"),
        Some(Duration::ZERO)
    );
    let future = (chrono::Utc::now() + chrono::Duration::seconds(60))
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string();
    let wait = parse_retry_after(&future).expect("HTTP date");
    assert!(wait > Duration::from_secs(58) && wait <= Duration::from_secs(60));
}

#[test]
fn slow_replacement_requires_grace_relative_speed_remaining_time_and_no_limit() {
    let size = 8 * 1024 * 1024;
    let segment =
        DownloadSegment::new(0, 0, size - 1, 0, DownloadSegmentState::Waiting).expect("range");
    let mut planner = SegmentPlanner::new(&[segment], Duration::ZERO);
    planner.assign(0, None).expect("assignment");
    let state = SegmentReceiveState::new(planner);
    state.note_speed(256 * 1024, Duration::from_millis(250));
    let global = DownloadRateLimiter::new(None).expect("global limit");
    let task = DownloadRateLimiter::new(None).expect("task limit");
    let control = DownloadSegmentTransferControl::new(DownloadTransferControl::with_rate_limiters(
        global.clone(),
        task.clone(),
    ));
    let slow = |at, received, elapsed| {
        state
            .should_replace(0, at, received, elapsed, &control)
            .expect("policy")
    };
    assert!(slow(128 * 1024, 128 * 1024, Duration::from_secs(2)));
    assert!(!slow(128 * 1024, 128 * 1024, Duration::from_millis(1999)));
    assert!(!slow(512 * 1024, 512 * 1024, Duration::from_secs(2)));
    assert!(!slow(size - 128 * 1024, 128 * 1024, Duration::from_secs(2)));
    global
        .set_bytes_per_second(Some(1024 * 1024))
        .expect("global/runtime cap");
    assert!(!slow(128 * 1024, 128 * 1024, Duration::from_secs(2)));
    global
        .set_bytes_per_second(None)
        .expect("remove global/runtime cap");
    task.set_bytes_per_second(Some(1024 * 1024))
        .expect("task cap");
    assert!(!slow(128 * 1024, 128 * 1024, Duration::from_secs(2)));
    task.set_bytes_per_second(None).expect("remove task cap");
    assert!(slow(128 * 1024, 128 * 1024, Duration::from_secs(2)));
    state.note_disk_wait(SIGNIFICANT_DISK_WAIT);
    assert!(!slow(128 * 1024, 128 * 1024, Duration::from_secs(2)));
    *state
        .last_disk_pressure
        .lock()
        .expect("disk-pressure clock") = Some(Instant::now() - DISK_PRESSURE_GRACE);
    assert!(slow(128 * 1024, 128 * 1024, Duration::from_secs(2)));
    let empty_sample = SegmentReceiveState::new(SegmentPlanner::new(&[], Duration::ZERO));
    empty_sample.note_speed(256 * 1024 - 1, Duration::from_millis(1));
    assert_eq!(empty_sample.best_speed.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn partial_write_failure_never_replays_the_batch_into_an_adjacent_owner() {
    for prefix in [2, 4] {
        let root = tempfile::tempdir().expect("owned writer root");
        let artifact = SegmentedArtifact::create(root.path().join("partial.bin"), 8)
            .await
            .expect("artifact");
        let left =
            DownloadSegment::new(0, 0, 3, 0, DownloadSegmentState::Waiting).expect("left range");
        let right = DownloadSegment::new(1, 4, 7, 0, DownloadSegmentState::Waiting)
            .expect("adjacent range");
        let mut right_writer = artifact
            .open_segment(&right)
            .await
            .expect("right owner writer");
        right_writer
            .write_all(&[0xcc; 4])
            .await
            .expect("right owner bytes");
        right_writer.sync().await.expect("right owner sync");
        drop(right_writer);
        let mut planner = SegmentPlanner::new(&[left.clone(), right], Duration::ZERO);
        planner.assign(0, None).expect("left assignment");
        assert_eq!(planner.claim(0, 0, 4), Claim::Granted(4));
        let state = SegmentReceiveState::new(planner);
        let control = DownloadSegmentTransferControl::new(DownloadTransferControl::new());
        let mut writer = artifact
            .open_segment(&left)
            .await
            .expect("left owner writer");
        writer.fail_next_write_after(prefix);
        let mut sink = ClaimedWriter {
            writer,
            state: &state,
            control: &control,
            worker: 0,
            index: 0,
            start: 0,
            position: 0,
            buffer: b"abcd".to_vec(),
            disk_wait: Duration::ZERO,
        };
        let mut progress = |_| {};
        assert!(matches!(
            sink.flush(&mut progress).await,
            Err(HttpDownloadError::Artifact(SegmentedArtifactError::Io(_)))
        ));
        sink.flush(&mut progress)
            .await
            .expect("exit cleanup does not replay failed IO");
        sink.sync().await.expect("sync retained prefix");
        let bytes = tokio::fs::read(artifact.path())
            .await
            .expect("actual artifact bytes");
        assert_eq!(
            &bytes[4..],
            &[0xcc; 4],
            "another owner's range must remain intact"
        );
        assert_eq!(&bytes[..prefix], &b"abcd"[..prefix]);
        assert!(bytes[prefix..4].iter().all(|byte| *byte == 0));
        let planner = state.lock().expect("planner");
        assert_eq!(planner.segments()[0].cursor, 0);
        assert_eq!(
            planner.checkpoints().expect("checkpoint")[0].downloaded_bytes(),
            0
        );
    }
}
