use std::sync::Arc;
use std::time::Duration;

use nxtools_download::{
    DownloadEngine, DownloadEvent, DownloadFinishingStage, DownloadRequest, DownloadStatus,
    RequestOrigin,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn hls_finishing_stop_preserves_only_the_artifacts_owned_by_its_outcome() {
    let bytes = Arc::new(
        (0..8 * 1024 * 1024)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>(),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind HLS fixture");
    let address = listener.local_addr().expect("HLS fixture address");
    let served_bytes = Arc::clone(&bytes);
    let server = tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let bytes = Arc::clone(&served_bytes);
            tokio::spawn(async move {
                let mut request = Vec::new();
                let mut buffer = [0_u8; 1024];
                while !request.windows(4).any(|value| value == b"\r\n\r\n") {
                    let count = stream
                        .read(&mut buffer)
                        .await
                        .expect("read fixture request");
                    if count == 0 {
                        return;
                    }
                    request.extend_from_slice(&buffer[..count]);
                }
                let playlist = b"#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXTINF:10.0,\nsegment.ts\n#EXT-X-ENDLIST\n";
                let is_playlist = request.starts_with(b"GET /index.m3u8 ")
                    || request.starts_with(b"HEAD /index.m3u8 ");
                let body: &[u8] = if is_playlist { playlist } else { &bytes };
                let mime = if is_playlist {
                    "application/vnd.apple.mpegurl"
                } else {
                    "video/mp2t"
                };
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: {mime}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                if stream.write_all(header.as_bytes()).await.is_ok()
                    && !request.starts_with(b"HEAD ")
                {
                    let _ = stream.write_all(body).await;
                }
            });
        }
    });

    for cancel in [false, true] {
        let output = tempfile::tempdir().expect("HLS output directory");
        let engine = DownloadEngine::open_in_memory().expect("open HLS engine");
        let events = engine
            .take_event_receiver()
            .expect("receive production events");
        let url = format!("http://{address}/index.m3u8");
        let request: DownloadRequest = serde_json::from_value(serde_json::json!({
            "source_url": url,
            "save_directory": output.path(),
            "file_name": "joined.ts",
            "connection_limit": 1,
            "media": { "kind": "hls", "data": { "playlist_url": url } }
        }))
        .expect("decode HLS request");
        let task_id = engine
            .submit(request, RequestOrigin::Ui)
            .expect("submit HLS task");
        let phase = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let event = events.recv().await.expect("production event");
                if let DownloadEvent::TaskProgress { progress } = event.event
                    && progress.task_id == task_id
                    && let Some(phase) = progress.finishing
                    && phase.stage == DownloadFinishingStage::Concatenating
                {
                    break phase;
                }
            }
        })
        .await
        .expect("HLS must publish its concatenation boundary");
        assert_eq!(phase.completed, 0);
        assert_eq!(phase.total, Some(bytes.len() as u64));
        let temporary = engine
            .task(task_id)
            .expect("read merging task")
            .expect("merging task exists")
            .temporary_path
            .expect("merge temporary path");
        if cancel {
            engine.cancel(task_id).expect("cancel concatenation");
        } else {
            engine.pause(task_id).expect("pause concatenation");
        }
        let expected = if cancel {
            DownloadStatus::Cancelled
        } else {
            DownloadStatus::Paused
        };
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let task = engine
                    .task(task_id)
                    .expect("read stopped task")
                    .expect("task exists");
                if task.task.status() == expected {
                    break;
                }
                assert_ne!(
                    task.task.status(),
                    DownloadStatus::Completed,
                    "stop must precede publication"
                );
                assert_ne!(
                    task.task.status(),
                    DownloadStatus::Failed,
                    "stop failed: {:?}",
                    task.error
                );
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("stop must settle after writer cleanup");
        assert!(!temporary.exists(), "partial merge must not survive stop");
        assert!(!output.path().join("joined.ts").exists());
        let staging = output
            .path()
            .join(format!(".nxtools-hls-{}", task_id.as_uuid()));
        if cancel {
            assert!(!staging.exists(), "cancel must remove owned fragments");
        } else {
            assert!(staging.is_dir(), "pause must retain durable fragments");
            engine.resume(task_id).expect("resume concatenation");
            let completed = engine
                .wait_for_terminal(task_id, Duration::from_secs(10))
                .await
                .expect("resumed HLS terminal");
            assert_eq!(
                completed.task.status(),
                DownloadStatus::Completed,
                "{:?}",
                completed.error
            );
            assert_eq!(
                std::fs::read(output.path().join("joined.ts")).expect("read joined file"),
                *bytes
            );
        }
    }
    server.abort();
    let _ = server.await;
}
