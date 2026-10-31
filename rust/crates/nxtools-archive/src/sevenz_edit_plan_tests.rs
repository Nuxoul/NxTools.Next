use super::*;
use nxtools_protocol::archive_browser::{ArchiveBrowserContext, ArchiveSourceBinding};
use uuid::Uuid;

fn intent(
    action: Action,
    ordinals: &[u64],
    value: Option<&str>,
    prefix: Option<&str>,
) -> ArchiveJobModify {
    ArchiveJobModify {
        browser: ArchiveBrowserContext {
            snapshot_id: Uuid::new_v4(),
            format: "SevenZ".into(),
            comment: String::new(),
            source: ArchiveSourceBinding {
                native: "v1:test".into(),
                digest: [0; 32],
                size: 0,
                modified_unix_ns: 0,
            },
            sevenz_edit: None,
            tar_edit: None,
        },
        action,
        ordinals: ordinals.to_vec(),
        value: value.map(str::to_owned),
        source_prefix: prefix.map(str::to_owned),
    }
}
fn files(names: &[(&str, bool)]) -> Vec<sevenz_rust2::ArchiveEntry> {
    names
        .iter()
        .map(|(name, dir)| {
            if *dir {
                sevenz_rust2::ArchiveEntry::new_directory(name)
            } else {
                sevenz_rust2::ArchiveEntry::new_file(name)
            }
        })
        .collect()
}
fn planned(names: &[(&str, bool)]) -> Vec<Planned> {
    files(names)
        .into_iter()
        .enumerate()
        .map(|(i, entry)| Planned {
            entry,
            origin: Some(i),
            added: None,
        })
        .collect()
}
fn run(files: &[sevenz_rust2::ArchiveEntry], intent: &ArchiveJobModify) -> Result<Vec<Planned>> {
    build(
        files,
        intent,
        &ResourceLimits::default(),
        &OperationControl::default(),
    )
}
fn names(result: &[Planned]) -> Vec<&str> {
    result.iter().map(|item| item.entry.name.as_str()).collect()
}

#[test]
fn sevenz_plan_delete_overlapping_directories_deduplicates_without_ab_prefix() {
    let input = files(&[
        ("a/", true),
        ("a/b/", true),
        ("a/b/leaf", false),
        ("ab/leaf", false),
        ("a2", false),
    ]);
    let result =
        run(&input, &intent(Action::Delete, &[0, 1, 2], None, None)).expect("overlapping deletion");
    assert_eq!(names(&result), ["ab/leaf", "a2"]);
    assert_eq!(
        result.iter().map(|item| item.origin).collect::<Vec<_>>(),
        [Some(3), Some(4)]
    );
}

#[test]
fn sevenz_plan_rename_unicode_explicit_and_implicit_subtrees_preserves_ordinals() {
    let input = files(&[("猫/", true), ("猫/子/leaf", false), ("猫咪/leaf", false)]);
    let result =
        run(&input, &intent(Action::Rename, &[0], Some("犬/"), None)).expect("explicit directory");
    assert_eq!(names(&result)[1..], ["犬/子/leaf", "猫咪/leaf"]);
    assert_eq!(result[0].entry.name.trim_end_matches('/'), "犬");
    assert_eq!(
        result.iter().map(|item| item.origin).collect::<Vec<_>>(),
        [Some(0), Some(1), Some(2)]
    );
    let implicit = files(&[
        ("猫/子/leaf", false),
        ("猫咪/leaf", false),
        ("猫/root", false),
    ]);
    let result = run(
        &implicit,
        &intent(Action::Rename, &[0], Some("犬/"), Some("猫/")),
    )
    .expect("implicit directory");
    assert_eq!(names(&result), ["犬/子/leaf", "猫咪/leaf", "犬/root"]);
}

#[test]
fn sevenz_plan_rename_file_changes_only_selected_ordinal() {
    let input = files(&[("a", false), ("ab", false), ("dir/leaf", false)]);
    let result =
        run(&input, &intent(Action::Rename, &[1], Some("renamed"), None)).expect("file rename");
    assert_eq!(names(&result), ["a", "renamed", "dir/leaf"]);
    assert_eq!(result[1].origin, Some(1));
}

#[test]
fn sevenz_plan_rejects_case_collision_and_file_prefix_collision() {
    let input = files(&[("a/child", false), ("other", false)]);
    for value in ["A/CHILD", "a", "a/child/descendant"] {
        let error = run(&input, &intent(Action::Rename, &[1], Some(value), None))
            .err()
            .expect("collision");
        assert_eq!(error, ArchiveError::OutputCollision, "target {value}");
    }
    assert_eq!(
        validate(
            &planned(&[("A/", true), ("a/", true)]),
            &ResourceLimits::default(),
            &OperationControl::default()
        ),
        Err(ArchiveError::OutputCollision)
    );
    assert_eq!(
        validate(
            &planned(&[("root", false), ("ROOT/child", false)]),
            &ResourceLimits::default(),
            &OperationControl::default()
        ),
        Err(ArchiveError::OutputCollision)
    );
}

#[test]
fn sevenz_plan_rejects_stale_ordinal_and_duplicate_selection() {
    let input = files(&[("a", false), ("b", false)]);
    assert_eq!(
        run(&input, &intent(Action::Delete, &[2], None, None)).err(),
        Some(ArchiveError::SourceChanged)
    );
    assert_eq!(
        run(&input, &intent(Action::Delete, &[0, 0], None, None)).err(),
        Some(ArchiveError::SourceChanged)
    );
    assert_eq!(
        run(&input, &intent(Action::Rename, &[1, 1], Some("c"), None)).err(),
        Some(ArchiveError::SourceChanged)
    );
}

#[test]
fn sevenz_plan_rejects_invalid_paths_and_duplicate_member_names() {
    for bad in ["../escape", "/absolute", "a//b", "CON", "a/./b", "a\\b"] {
        assert_eq!(
            run(
                &files(&[("ok", false)]),
                &intent(Action::Rename, &[0], Some(bad), None)
            )
            .err(),
            Some(ArchiveError::UnsafePath),
            "target {bad}"
        );
    }
    assert_eq!(
        validate(
            &planned(&[("dup", false), ("dup", false)]),
            &ResourceLimits::default(),
            &OperationControl::default()
        ),
        Err(ArchiveError::OutputCollision)
    );
    assert_eq!(
        validate(
            &planned(&[("a/b", false), ("a\\b", false)]),
            &ResourceLimits::default(),
            &OperationControl::default()
        ),
        Err(ArchiveError::UnsafePath)
    );
}

#[test]
fn sevenz_plan_rejects_self_descendant_and_stale_implicit_prefix() {
    let input = files(&[("a/x", false), ("ab/y", false)]);
    assert_eq!(
        run(
            &input,
            &intent(Action::Rename, &[0], Some("a/child/"), Some("a/"))
        )
        .err(),
        Some(ArchiveError::UnsafePath)
    );
    assert_eq!(
        run(
            &input,
            &intent(Action::Rename, &[0], Some("new/"), Some("ab/"))
        )
        .err(),
        Some(ArchiveError::SourceChanged)
    );
    assert_eq!(
        run(
            &input,
            &intent(Action::Rename, &[0], Some("new/"), Some("../a/"))
        )
        .err(),
        Some(ArchiveError::UnsafePath)
    );
}

#[test]
fn sevenz_plan_cancel_stops_before_mutation() {
    let control = OperationControl::default();
    control.cancel();
    assert_eq!(
        build(
            &files(&[("a/", true), ("a/file", false)]),
            &intent(Action::Delete, &[0], None, None),
            &ResourceLimits::default(),
            &control
        )
        .err(),
        Some(ArchiveError::Cancelled)
    );
    assert_eq!(
        validate(
            &planned(&[("a", false)]),
            &ResourceLimits::default(),
            &control
        ),
        Err(ArchiveError::Cancelled)
    );
}
