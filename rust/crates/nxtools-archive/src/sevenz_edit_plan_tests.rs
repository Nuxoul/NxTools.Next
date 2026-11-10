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

fn opt05_large_sevenz_selection() -> (Vec<sevenz_rust2::ArchiveEntry>, ArchiveJobModify) {
    let mut files = Vec::new();
    let mut ordinals = Vec::new();
    for root in (0..128).rev() {
        let mut path = format!("r{root:03}");
        for depth in 0..32 {
            if depth != 0 {
                path.push_str("/d");
            }
            ordinals.push(files.len() as u64);
            files.push(sevenz_rust2::ArchiveEntry::new_directory(&format!(
                "{path}/"
            )));
        }
        for file in 0..480 {
            files.push(sevenz_rust2::ArchiveEntry::new_file(&format!(
                "{path}/f{file:03}"
            )));
        }
    }
    assert_eq!(files.len(), 65_536);
    assert_eq!(ordinals.len(), 4_096);
    (files, intent(Action::Delete, &ordinals, None, None))
}

#[test]
fn opt05_before_sevenz_selection_three_trials() {
    let (files, intent) = opt05_large_sevenz_selection();
    for trial in 1..=3 {
        let mut work = SelectionWork::default();
        let started = std::time::Instant::now();
        let plan = build_observed(
            &files,
            &intent,
            &ResourceLimits::default(),
            &OperationControl::default(),
            &mut work,
        )
        .expect("original 7z plan boundary");
        let elapsed_us = started.elapsed().as_micros();
        assert!(
            plan.is_empty(),
            "delete includes all descendants without an empty-result cap"
        );
        eprintln!(
            "OPT05_SELECTION {}",
            serde_json::json!({
                "boundary": "sevenz_edit_plan", "trial": trial,
                "build_mode": if cfg!(debug_assertions) { "debug" } else { "release" },
                "catalog": files.len(), "selected_ids": intent.ordinals.len(),
                "remaining": plan.len(), "elapsed_us": elapsed_us, "work": work,
            })
        );
    }
}

#[test]
fn opt05_red_sevenz_selection_single_pass() {
    let (files, intent) = opt05_large_sevenz_selection();
    let mut work = SelectionWork::default();
    let plan = build_observed(
        &files,
        &intent,
        &ResourceLimits::default(),
        &OperationControl::default(),
        &mut work,
    )
    .expect("delete plan");
    assert!(plan.is_empty());
    assert!(
        work.catalog_visits <= 65_536
            && work.path_normalizations <= 65_536
            && work.path_normalizations + work.root_normalizations <= 65_536
            && work.normalization_allocations == 0
            && work.prefix_probes <= 65_536 * 16 + 4_096 * 64
            && work.directory_roots <= 128
            && work.ordinal_probes <= 65_536 + 4_096,
        "7z subtree expansion must use merged component-ended prefixes: {work:?}"
    );
}

#[test]
fn opt05_sevenz_selection_keeps_empty_implicit_and_original_record_order() {
    let input = files(&[
        ("a/sub/z", false),
        ("ab/peer", false),
        ("a/", true),
        ("a/empty/", true),
        ("a/sub/x", false),
        ("loose", false),
    ]);
    let mut work = SelectionWork::default();
    let plan = build_observed(
        &input,
        &intent(Action::Delete, &[2, 3, 0], None, None),
        &ResourceLimits::default(),
        &OperationControl::default(),
        &mut work,
    )
    .expect("overlap");
    assert_eq!(names(&plan), ["ab/peer", "loose"]);
    assert_eq!(
        plan.iter().map(|item| item.origin).collect::<Vec<_>>(),
        [Some(1), Some(5)]
    );
    assert_eq!(input[0].name, "a/sub/z");
    let plan =
        run(&input, &intent(Action::Delete, &[3], None, None)).expect("empty directory only");
    assert_eq!(
        plan.iter().map(|item| item.origin).collect::<Vec<_>>(),
        [Some(0), Some(1), Some(2), Some(4), Some(5)]
    );
    let plan = run(
        &input,
        &intent(Action::Rename, &[0], Some("moved/"), Some("a/sub/")),
    )
    .expect("implicit subtree rename remains format-owned");
    assert_eq!(
        names(&plan),
        ["moved/z", "ab/peer", "a/", "a/empty/", "moved/x", "loose"]
    );
    assert_eq!(
        plan.iter().map(|item| item.origin).collect::<Vec<_>>(),
        (0..6).map(Some).collect::<Vec<_>>()
    );
}

#[test]
fn opt05_sevenz_selection_budget_rejects_scratch_before_rewrite() {
    let input = files(&[("a/", true), ("a/leaf", false), ("keep", false)]);
    let limits = ResourceLimits {
        max_metadata_bytes: 1,
        ..Default::default()
    };
    let mut work = SelectionWork::default();
    assert_eq!(
        build_observed(
            &input,
            &intent(Action::Delete, &[0], None, None),
            &limits,
            &OperationControl::default(),
            &mut work
        )
        .err(),
        Some(ArchiveError::ResourceLimit)
    );
    assert_eq!(work.plan_visits, 0);
    assert_eq!(input[0].name, "a/");
}

#[test]
fn opt05_sevenz_production_build_records_selection_on_success_and_failure() {
    for keep in ["keep", "../bad"] {
        let input = files(&[("a/", true), ("a/leaf", false), (keep, false)]);
        let control = OperationControl::default();
        let result = build(
            &input,
            &intent(Action::Delete, &[0], None, None),
            &ResourceLimits::default(),
            &control,
        );
        if keep == "keep" {
            assert_eq!(names(&result.expect("valid retained member")), ["keep"]);
        } else {
            assert_eq!(result.err(), Some(ArchiveError::UnsafePath));
        }
        let observed = control.work_observation().selection;
        assert_eq!(observed.catalog_visits, 3);
        assert_eq!(observed.plan_visits, 3);
        assert_eq!(observed.directory_roots, 1);
    }
}

#[test]
fn opt05_selection_working_limit_bounds_sevenz_scratch() {
    let input = files(&[("a/", true), ("a/leaf", false), ("keep", false)]);
    let limits = ResourceLimits {
        max_working_bytes: 1,
        ..Default::default()
    };
    assert_eq!(
        build(
            &input,
            &intent(Action::Delete, &[0], None, None),
            &limits,
            &OperationControl::default()
        )
        .err(),
        Some(ArchiveError::ResourceLimit)
    );
}
