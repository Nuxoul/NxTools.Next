//! Real bounded loader/writer regressions. Synthetic owned JSON, never product configuration.
use super::*;

fn legacy_document(details: Option<bool>) -> serde_json::Value {
    let state = UiState {
        version: 1,
        geometry: UiGeometry {
            x: -123,
            y: 456,
            width: 1180,
            height: 780,
            maximized: true,
        },
        widths: [355., 96., 144., 128., 224., 136., 88., 120., 152., 416.],
        visible: [
            true, false, false, false, false, true, false, true, false, true,
        ],
        show_tree: false,
        show_comment: true,
        sort: UiSortKey::Packed,
        descending: true,
        details_view: details.unwrap_or(false),
        recent: vec![std::env::temp_dir().join("synthetic-recent.zip")],
    };
    let mut document = serde_json::to_value(state).expect("legacy document");
    if details.is_none() {
        document
            .as_object_mut()
            .expect("object")
            .remove("details_view");
    }
    document
}

fn wait_for_load(store: &UiStateStore) -> UiStateLoad {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let status = store.load_status();
        if !matches!(status, UiStateLoad::Loading) {
            return status;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "bounded UI state load"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn version_two_defaults_to_details_and_all_five_primary_columns() {
    let state = UiState::default();
    assert_eq!(state.version, 2);
    assert!(state.details_view);
    assert_eq!(&state.visible[..5], &[true; 5]);
    let mut document = serde_json::to_value(state).expect("default document");
    document
        .as_object_mut()
        .expect("object")
        .remove("details_view");
    let decoded: UiState = serde_json::from_value(document).expect("additive optional mode");
    assert!(
        decoded.details_view,
        "v2 absent mode uses the current default"
    );
}

#[test]
fn version_one_missing_or_false_mode_migrates_only_requested_fields() {
    let directory = tempfile::tempdir().expect("owned state");
    let path = directory.path().join("archive-ui.json");
    for details in [None, Some(false)] {
        let document = legacy_document(details);
        let mut expected: UiState = serde_json::from_value(document.clone()).expect("legacy");
        expected.version = 2;
        expected.details_view = true;
        expected.visible[..5].fill(true);
        let original = serde_json::to_vec(&document).expect("legacy bytes");
        fs::write(&path, &original).expect("write owned legacy");
        assert_eq!(
            load(&path),
            Ok(expected.clone()),
            "migrate without resetting preferences"
        );
        assert_eq!(
            fs::read(&path).expect("read legacy"),
            original,
            "load is not an unowned write"
        );
        let store = UiStateStore::initialize(path.clone()).expect("original writer");
        assert_eq!(wait_for_load(&store), UiStateLoad::Ready(expected));
        // A new v2 user's explicit name mode must survive this migration and every reopen.
        let mut selected = store.current_snapshot();
        selected.details_view = false;
        selected.visible[2] = false;
        store
            .enqueue_save(selected.clone())
            .expect("save v2 choice");
        store.finish_for_test();
        assert_eq!(load(&path), Ok(selected.clone()));
        let restarted = UiStateStore::initialize(path.clone()).expect("restart writer");
        assert_eq!(wait_for_load(&restarted), UiStateLoad::Ready(selected));
        restarted.finish_for_test();
    }
}

#[test]
fn version_one_explicit_details_retains_column_preferences() {
    let directory = tempfile::tempdir().expect("owned state");
    let path = directory.path().join("archive-ui.json");
    let document = legacy_document(Some(true));
    let mut expected: UiState = serde_json::from_value(document.clone()).expect("legacy details");
    expected.version = 2;
    fs::write(&path, serde_json::to_vec(&document).expect("bytes")).expect("legacy write");
    assert_eq!(
        load(&path),
        Ok(expected),
        "already-detailed legacy custom columns are not reset"
    );
}

#[test]
fn migration_never_repairs_or_overwrites_invalid_existing_configuration() {
    let directory = tempfile::tempdir().expect("owned state");
    let path = directory.path().join("archive-ui.json");
    let mut invalid_width = legacy_document(Some(false));
    invalid_width["widths"][3] = serde_json::json!(1);
    let mut hidden_name = legacy_document(None);
    hidden_name["visible"][0] = serde_json::json!(false);
    let mut future = legacy_document(Some(false));
    future["version"] = serde_json::json!(99);
    let mut null_mode = legacy_document(None);
    null_mode["details_view"] = serde_json::Value::Null;
    for original in [
        serde_json::to_vec(&invalid_width).expect("invalid width"),
        serde_json::to_vec(&hidden_name).expect("hidden name"),
        serde_json::to_vec(&future).expect("future version"),
        serde_json::to_vec(&null_mode).expect("invalid null mode"),
        b"{broken-json".to_vec(),
    ] {
        fs::write(&path, &original).expect("invalid owned bytes");
        let store = UiStateStore::initialize(path.clone()).expect("original store");
        assert!(matches!(wait_for_load(&store), UiStateLoad::Preserved(_)));
        assert_eq!(
            store.enqueue_save(UiState::default()),
            Err(UiStateStoreError::Preserved)
        );
        store.finish_for_test();
        assert_eq!(fs::read(&path).expect("preserved evidence"), original);
    }
}
