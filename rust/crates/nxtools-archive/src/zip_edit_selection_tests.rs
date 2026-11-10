use super::*;

#[test]
fn opt05_zip_edit_delete_expands_once() {
    let (snapshot, ids) = crate::opt05_edit_selection_fixture::large();
    let control = OperationControl::default();
    let started = std::time::Instant::now();
    let result = intent(
        &snapshot,
        &[ZipEdit::Delete { entries: ids }],
        &ResourceLimits::default(),
        &control,
    )
    .expect("original ZIP intent boundary");
    assert_eq!(result.deleted.len(), 65_536);
    assert!(
        snapshot
            .entries
            .iter()
            .all(|entry| result.deleted.contains(&entry.id.ordinal))
    );
    let work = control.work_observation().selection;
    eprintln!(
        "OPT05_EDIT_SELECTION {}",
        serde_json::json!({
            "boundary":"zip_delete_intent", "elapsed_us":started.elapsed().as_micros(), "work":work,
        })
    );
    assert!(
        work.catalog_visits <= 65_536
            && work.path_normalizations + work.root_normalizations <= 65_536
            && work.prefix_probes <= 65_536 * 16 + 4_096 * 64,
        "ZIP delete expansion is one original catalog pass: {work:?}"
    );
}
