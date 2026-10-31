use super::*;
use std::collections::HashSet;

#[tokio::test(flavor = "current_thread")]
async fn remediation_download_pages_traverse_seventy_tasks_and_reject_foreign_cursor() {
    let root = tempfile::tempdir().expect("owned paging");
    let engine = DownloadEngine::open(root.path().join("downloads.db")).expect("engine");
    for index in 0..70 {
        let request=serde_json::from_value(serde_json::json!({"source_url":format!("https://example.test/{index}"),"save_directory":root.path(),"file_name":format!("{index}.bin"),"headers":[],"scheduled_at_unix_ms":nxtools_ipc::unix_time_ms()+3_600_000})).expect("request");
        engine
            .enqueue(request, RequestOrigin::Internal)
            .expect("queue without network");
    }
    let adapter = FeatureAdapters::new(engine.clone(), None, None);
    let (context, _cancel) = super::tests::download_context();
    let mut cursor = Value::Null;
    let mut seen = HashSet::new();
    for page_number in 0..3 {
        let page = adapter
            .call(
                &context,
                "downloads.list_page",
                serde_json::json!({"limit":32,"cursor":cursor}),
            )
            .await;
        assert!(page.success, "{:?}", page.value);
        let items = page.value["items"].as_array().expect("page");
        assert_eq!(items.len(), if page_number == 2 { 6 } else { 32 });
        for item in items {
            assert!(seen.insert(item["task_id"].as_str().expect("id").to_owned()));
        }
        if page_number == 0 {
            let (foreign, _foreign_cancel) = super::tests::download_context();
            let bad = adapter
                .call(
                    &foreign,
                    "downloads.list_page",
                    serde_json::json!({"limit":32,"cursor":page.value["next_cursor"]}),
                )
                .await;
            assert_eq!(bad.value["code"], "invalid_cursor");
        }
        cursor = page.value["next_cursor"].clone();
        assert_eq!(page.value["has_more"], page_number != 2);
    }
    assert_eq!(seen.len(), 70);
    assert!(cursor.is_null());
    let legacy = adapter
        .call(&context, "downloads.list", serde_json::json!({"limit":32}))
        .await;
    assert_eq!(
        legacy
            .value
            .as_array()
            .expect("legacy array unchanged")
            .len(),
        32
    );
}

#[tokio::test(flavor = "current_thread")]
async fn remediation_clipboard_pages_traverse_and_history_mutation_retires_cursor() {
    let root = tempfile::tempdir().expect("owned history paging");
    let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        root.path().join("clipboard.db"),
        root.path().join("assets"),
        nxtools_clipboard::ClipboardRetentionPolicy {
            max_items: 1000,
            retention_days: 30,
        },
        nxtools_ipc::unix_time_ms(),
    )
    .expect("worker");
    let capture = |index| nxtools_protocol::ClipboardCapture {
        text: Some(format!("page fixture {index}")),
        paths: vec![],
        image: None,
        native_formats: vec![],
        source_icon_staging_path: None,
        source: nxtools_protocol::ForegroundContext {
            window_handle: 0,
            focus_window_handle: 0,
            process_id: 0,
            process_name: String::new(),
            window_title: String::new(),
        },
        captured_at_unix_ms: nxtools_ipc::unix_time_ms() + index,
    };
    for index in 0..70 {
        worker
            .capture_committed_async(capture(index), ClipboardCommitCausality::capture())
            .await
            .expect("capture")
            .expect("commit");
    }
    let engine = DownloadEngine::open(root.path().join("downloads.db")).expect("engine");
    let adapter = FeatureAdapters::new(
        engine.clone(),
        Some(ClipboardExtensionAdapter::new(worker.clone())),
        None,
    );
    let (context, _cancel) = super::tests::download_context();
    let mut cursor = Value::Null;
    let mut first = Value::Null;
    let mut seen = HashSet::new();
    for page_number in 0..3 {
        let page = adapter
            .call(
                &context,
                "clipboard.query_page",
                serde_json::json!({"query":"page fixture","limit":32,"cursor":cursor}),
            )
            .await;
        assert!(page.success, "{:?}", page.value);
        for item in page.value["items"].as_array().expect("items") {
            assert!(seen.insert(item["item_id"].as_i64().expect("id")));
        }
        cursor = page.value["next_cursor"].clone();
        if page_number == 0 {
            first = cursor.clone();
        }
    }
    assert_eq!(seen.len(), 70);
    assert!(cursor.is_null());
    worker
        .capture_committed_async(capture(99), ClipboardCommitCausality::capture())
        .await
        .expect("new capture");
    let stale = adapter
        .call(
            &context,
            "clipboard.query_page",
            serde_json::json!({"query":"page fixture","limit":32,"cursor":first}),
        )
        .await;
    assert_eq!(stale.value["code"], "cursor_stale");
    worker.shutdown().expect("owned worker shutdown");
}
