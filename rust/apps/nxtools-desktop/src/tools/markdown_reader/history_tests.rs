use super::history::Store;

#[test]
fn markdown_reader_history_deduplicates_names_and_keeps_before_restore() {
    let folder = tempfile::tempdir().expect("create history fixture directory");
    let database = folder.path().join("history.sqlite3");
    let source = folder.path().join("document.md");
    let first = b"\xef\xbb\xbf# First\r\n";
    let second = b"# Second\n";
    let third = b"# Latest\n";
    std::fs::write(&source, third).expect("write latest history source fixture");
    let mut store = Store::open(&database).expect("open history fixture database");
    let first_id = store
        .record(&source, first, "opened")
        .expect("record initial history version");
    assert_eq!(
        store
            .record(&source, first, "saved")
            .expect("record duplicate history version"),
        first_id
    );
    store
        .rename(&source, first_id, "Initial")
        .expect("name initial history version");
    store
        .record(&source, second, "saved")
        .expect("record second history version");
    drop(store);
    let mut store = Store::open(&database).expect("reopen persisted history database");
    let rows = store
        .list(&source)
        .expect("list persisted history versions");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].name.as_deref(), Some("Initial"));
    store
        .restore(&source, first_id, third)
        .expect("restore initial history version");
    assert_eq!(
        std::fs::read(&source).expect("read restored source bytes"),
        first
    );
    let latest = store.list(&source).expect("list history after restore");
    assert_eq!(latest.len(), 4);
    assert_eq!(
        store
            .bytes(&source, latest[1].id)
            .expect("read recoverable before-restore version"),
        third
    );
}
