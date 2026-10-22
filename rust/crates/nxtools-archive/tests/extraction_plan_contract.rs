use nxtools_archive::{
    ArchiveEngine, ArchiveError, OpenRequest, OperationControl,
    planning::{ExtractionPlan, ExtractionSelection, TargetStrategy},
};
use std::path::PathBuf;
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn smart_plan_counts_implicit_roots_and_selected_structure_without_io() {
    let snapshot = ArchiveEngine::default()
        .open(
            OpenRequest::new(fixture("stored.zip")),
            &OperationControl::default(),
        )
        .expect("independent ZIP");
    let plan = ExtractionPlan::build(
        &snapshot,
        ExtractionSelection::All,
        TargetStrategy::Smart {
            archive_stem: "资料包".into(),
            single_file_folder: false,
        },
    )
    .expect("smart output plan");
    assert_eq!(plan.subfolder(), Some("资料包"));
    assert_eq!(plan.files().len(), 3);
    assert_eq!(plan.total_bytes(), Some(275));
    assert!(plan.full_archive());
    let id = snapshot.entries()[1].id();
    let selected = ExtractionPlan::build(
        &snapshot,
        ExtractionSelection::Entries(vec![id]),
        TargetStrategy::Here,
    )
    .expect("selected subtree stays relative");
    assert!(!selected.full_archive());
    assert_eq!(selected.files()[0].relative_path(), "资料/说明.txt");
    assert_eq!(selected.total_bytes(), Some(19));
}
#[test]
fn selected_explicit_directory_includes_descendants_and_rejects_stale_ids() {
    let snapshot = ArchiveEngine::default()
        .open(
            OpenRequest::new(fixture("stored.zip")),
            &OperationControl::default(),
        )
        .expect("metadata");
    let plan = ExtractionPlan::build(
        &snapshot,
        ExtractionSelection::Entries(vec![snapshot.entries()[0].id()]),
        TargetStrategy::Smart {
            archive_stem: "unused".into(),
            single_file_folder: false,
        },
    )
    .expect("selected directory");
    assert_eq!(plan.files().len(), 1);
    assert_eq!(plan.subfolder(), None);
    let mut stale = snapshot.entries()[1].id();
    stale.session = uuid::Uuid::new_v4();
    assert!(matches!(
        ExtractionPlan::build(
            &snapshot,
            ExtractionSelection::Entries(vec![stale]),
            TargetStrategy::Here
        ),
        Err(ArchiveError::SourceChanged)
    ));
}
#[test]
fn unsafe_and_colliding_archives_never_get_a_writable_plan() {
    for (name, error) in [
        ("unsafe.zip", ArchiveError::UnsafePath),
        ("collisions.zip", ArchiveError::OutputCollision),
    ] {
        let snapshot = ArchiveEngine::default()
            .open(
                OpenRequest::new(fixture(name)),
                &OperationControl::default(),
            )
            .expect("unsafe remains browsable");
        assert!(
            matches!(ExtractionPlan::build(&snapshot,ExtractionSelection::All,TargetStrategy::Here),Err(actual) if actual==error)
        );
    }
}
#[test]
fn single_file_smart_policy_and_invalid_stem_are_explicit() {
    let mut request = OpenRequest::new(fixture("gbk.zip"));
    request.encoding = nxtools_archive::NameEncoding::Gbk;
    let snapshot = ArchiveEngine::default()
        .open(request, &OperationControl::default())
        .expect("one file");
    let direct = ExtractionPlan::build(
        &snapshot,
        ExtractionSelection::All,
        TargetStrategy::Smart {
            archive_stem: "one".into(),
            single_file_folder: false,
        },
    )
    .expect("single file direct");
    assert_eq!(direct.subfolder(), None);
    let folder = ExtractionPlan::build(
        &snapshot,
        ExtractionSelection::All,
        TargetStrategy::Smart {
            archive_stem: "one".into(),
            single_file_folder: true,
        },
    )
    .expect("single file folder");
    assert_eq!(folder.subfolder(), Some("one"));
    assert!(matches!(
        ExtractionPlan::build(
            &snapshot,
            ExtractionSelection::All,
            TargetStrategy::Name("../unsafe".into())
        ),
        Err(ArchiveError::UnsafePath)
    ));
}

#[test]
fn empty_selection_and_empty_archive_never_create_a_destination_or_authorize_source_delete() {
    let engine = ArchiveEngine::default();
    for name in ["empty.zip", "stored.zip"] {
        let snapshot = engine
            .open(
                OpenRequest::new(fixture(name)),
                &OperationControl::default(),
            )
            .expect("valid metadata");
        let plan = ExtractionPlan::build(
            &snapshot,
            ExtractionSelection::Entries(Vec::new()),
            TargetStrategy::Name("unused".into()),
        )
        .expect("empty selection");
        assert!(plan.files().is_empty() && plan.directories().is_empty());
        assert_eq!(plan.subfolder(), None);
        assert!(!plan.permits_source_delete_request());
        if name == "empty.zip" {
            let all = ExtractionPlan::build(
                &snapshot,
                ExtractionSelection::All,
                TargetStrategy::Name("unused".into()),
            )
            .expect("empty full archive");
            assert!(all.files().is_empty() && all.directories().is_empty());
            assert_eq!(all.subfolder(), None);
            assert!(!all.permits_source_delete_request());
        } else {
            let ids = snapshot.entries().iter().map(|entry| entry.id()).collect();
            let selected_all = ExtractionPlan::build(
                &snapshot,
                ExtractionSelection::Entries(ids),
                TargetStrategy::Here,
            )
            .expect("all IDs selected");
            assert!(!selected_all.full_archive());
            assert!(!selected_all.permits_source_delete_request());
        }
    }
}
