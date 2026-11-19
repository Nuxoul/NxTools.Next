use super::history::Store;

#[test]
fn markdown_reader_history_deduplicates_names_and_keeps_before_restore() {
    let folder = tempfile::tempdir().unwrap();
    let database = folder.path().join("history.sqlite3");
    let source = folder.path().join("document.md");
    let first = b"\xef\xbb\xbf# First\r\n";
    let second = b"# Second\n";
    let third = b"# Latest\n";
    std::fs::write(&source, third).unwrap();
    let mut store = Store::open(&database).unwrap();
    let first_id = store.record(&source, first, "opened").unwrap();
    assert_eq!(store.record(&source, first, "saved").unwrap(), first_id);
    store.rename(&source, first_id, "Initial").unwrap();
    store.record(&source, second, "saved").unwrap();
    drop(store);
    let mut store = Store::open(&database).unwrap();
    let rows = store.list(&source).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].name.as_deref(), Some("Initial"));
    store.restore(&source, first_id, third).unwrap();
    assert_eq!(std::fs::read(&source).unwrap(), first);
    let latest = store.list(&source).unwrap();
    assert_eq!(latest.len(), 4);
    assert_eq!(store.bytes(&source, latest[1].id).unwrap(), third);
}
