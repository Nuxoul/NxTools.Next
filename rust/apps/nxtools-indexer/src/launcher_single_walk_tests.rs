use super::*;

#[test]
fn launcher_search_matches_both_kinds_with_one_candidate_walk() {
    let snapshots = [test_snapshot(
        r"C:\",
        Some(1),
        Some(20),
        (0..64)
            .map(|row| {
                test_entry(
                    format!(r"C:\fixture\needle-{row:03}"),
                    format!("needle-{row:03}"),
                    row % 2 == 0,
                )
            })
            .collect(),
    )];
    let request = IndexSearchRequest {
        generation: 73,
        query: "needle".into(),
        limit: 40,
        context: IndexSearchContext {
            include_files: true,
            include_directories: true,
            retain_kinds_apart: true,
            ..Default::default()
        },
    };
    let visits = AtomicUsize::new(0);
    let mut batches = Vec::new();
    assert!(search_launcher_snapshots_while(
        &snapshots,
        &IndexSettings::default(),
        &request,
        &|| {
            visits.fetch_add(1, AtomicOrdering::Relaxed);
            true
        },
        |items, complete| {
            batches.push((items, complete));
            true
        },
    ));
    assert_eq!(batches.iter().filter(|(_, complete)| *complete).count(), 1);
    assert!(batches.last().is_some_and(|(_, complete)| *complete));
    let items = batches
        .into_iter()
        .flat_map(|(items, _)| items)
        .collect::<Vec<_>>();
    assert_eq!(items.len(), 64);
    assert_eq!(items.iter().filter(|item| item.is_directory).count(), 32);
    assert_eq!(items.iter().filter(|item| !item.is_directory).count(), 32);
    for row in 0..64 {
        assert!(
            items
                .iter()
                .any(|item| item.path == format!(r"C:\fixture\needle-{row:03}")
                    && item.is_directory == (row % 2 == 0))
        );
    }
    // The existing current-query boundary is visited for each hot name, row fanout
    // and materialized candidate. Allow bounded terminal/send checks, not a second scan.
    let visits = visits.load(AtomicOrdering::Relaxed);
    println!("LAUNCHER_SINGLE_WALK rows=64 current_checks={visits}");
    assert!(
        visits <= 64 * 3 + 16,
        "both kinds must share the matching walk, observed {visits} current checks"
    );
}

#[test]
fn launcher_shared_walk_matches_independent_kind_queries_with_cold_recovery() {
    for dominant_directories in [false, true] {
        let entries = (0..240)
            .map(|row| {
                let name = ["needle.txt", "a-needle.txt", "年度报告.txt"][row % 3];
                let mut entry = test_entry(
                    format!(r"C:\fixture\d{row:03}\{name}"),
                    name,
                    (row < 220) == dominant_directories,
                );
                entry.identity = Some(mft::MftIdentity {
                    file_reference: row as u64 + 10,
                    parent_reference: 5,
                });
                entry
            })
            .collect();
        let mut snapshot = test_snapshot(r"C:\", Some(17), Some(100), entries);
        snapshot.overlay.deleted.insert(10);
        snapshot.overlay.replacements.insert(
            11,
            test_entry(r"C:\fixture\renamed-needle", "renamed-needle", true).into(),
        );
        snapshot.overlay.replacements.insert(
            900,
            test_entry(r"C:\fixture\delta-needle.txt", "delta-needle.txt", false).into(),
        );
        let snapshots = [
            snapshot,
            test_snapshot(
                r"D:\",
                Some(18),
                Some(100),
                vec![test_entry(r"D:\outside\needle.txt", "needle.txt", false)],
            ),
        ];
        let settings = IndexSettings {
            indexed_volumes: vec![r"C:\".into()],
            excluded_paths: vec![r"C:\fixture\d002".into()],
        };
        for raw in [
            "needle",
            "'needle",
            "needle !a-",
            "*needle",
            "ndbg",
            r"C:\fixture\needle",
            "::fixture needle",
        ] {
            for limit in [0, 1, 7, 20] {
                for directory in [None, Some(r"C:\fixture".to_owned())] {
                    let request = IndexSearchRequest {
                        generation: 74,
                        query: raw.into(),
                        limit,
                        context: IndexSearchContext {
                            include_files: true,
                            include_directories: true,
                            retain_kinds_apart: true,
                            directory,
                            preferred_directory: Some(r"C:\fixture".into()),
                            ..Default::default()
                        },
                    };
                    let mut expected = Vec::new();
                    for directories_only in [false, true] {
                        let mut single = request.clone();
                        single.context.retain_kinds_apart = false;
                        single.context.include_files = !directories_only;
                        single.context.include_directories = directories_only;
                        expected.extend(
                            search_snapshots_while(&snapshots, &settings, &single, || true)
                                .expect("independent kind query"),
                        );
                    }
                    expected.sort_by(|left, right| {
                        right
                            .score
                            .cmp(&left.score)
                            .then_with(|| left.path.cmp(&right.path))
                    });
                    let actual = search_snapshots_while(&snapshots, &settings, &request, || true)
                        .expect("shared matching query");
                    assert_eq!(
                        actual, expected,
                        "{raw}, limit={limit}, dominant_directories={dominant_directories}"
                    );
                }
            }
        }
    }
}

#[test]
fn launcher_shared_walk_cancellation_before_publication_sends_nothing() {
    let snapshots = [test_snapshot(
        r"C:\",
        Some(1),
        Some(20),
        (0..100)
            .map(|row| {
                test_entry(
                    format!(r"C:\fixture\needle-{row}"),
                    format!("needle-{row}"),
                    row % 2 == 0,
                )
            })
            .collect(),
    )];
    let request = IndexSearchRequest {
        generation: 75,
        query: "needle".into(),
        limit: 20,
        context: IndexSearchContext {
            include_files: true,
            include_directories: true,
            retain_kinds_apart: true,
            ..Default::default()
        },
    };
    let checks = AtomicUsize::new(0);
    let mut published = false;
    assert!(!search_launcher_snapshots_while(
        &snapshots,
        &IndexSettings::default(),
        &request,
        &|| checks.fetch_add(1, AtomicOrdering::Relaxed) < 8,
        |_, _| {
            published = true;
            true
        }
    ));
    assert!(
        !published,
        "cancelled computation cannot publish even an empty terminal"
    );
}

#[test]
fn launcher_homophone_fallback_keeps_its_original_combined_limit() {
    let snapshots = [test_snapshot(
        r"C:\",
        Some(1),
        Some(20),
        (0..16)
            .map(|row| {
                test_entry(
                    format!(r"C:\fixture\年度报告-{row:02}"),
                    format!("年度报告-{row:02}"),
                    row % 2 == 0,
                )
            })
            .collect(),
    )];
    let mut request = IndexSearchRequest {
        generation: 76,
        query: "年度报高".into(),
        limit: 3,
        context: IndexSearchContext {
            include_files: true,
            include_directories: true,
            retain_kinds_apart: true,
            ..Default::default()
        },
    };
    assert!(
        search_snapshots_while(&snapshots, &IndexSettings::default(), &request, || true)
            .expect("ordinary search")
            .is_empty()
    );
    request.context.allow_homophone_fallback = true;
    let expected = search_snapshots_while(&snapshots, &IndexSettings::default(), &request, || true)
        .expect("permitted fallback");
    assert_eq!(
        expected.len(),
        3,
        "approximation keeps its combined limit rather than a new per-kind quota"
    );
    let mut actual = Vec::new();
    let mut terminals = 0;
    assert!(search_launcher_snapshots_while(
        &snapshots,
        &IndexSettings::default(),
        &request,
        &|| true,
        |items, complete| {
            actual.extend(items);
            terminals += usize::from(complete);
            true
        }
    ));
    assert_eq!(terminals, 1);
    assert_eq!(actual, expected);
}
