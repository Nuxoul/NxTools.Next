use nxtools_finder::{ApplicationRoot, ApplicationSource, ExternalSearchEntry, FinderCatalog};

fn item(name: &str, path: &str, directory: bool) -> ExternalSearchEntry {
    ExternalSearchEntry {
        id: format!("indexed:{}", path.replace('/', "\\").to_lowercase()),
        name: name.into(),
        target: path.into(),
        source: if directory {
            ApplicationSource::IndexedDirectory
        } else {
            ApplicationSource::IndexedFile
        },
    }
}

#[test]
fn weak_executable_does_not_displace_exact_directory() {
    let temp = tempfile::tempdir().expect("temp");
    let catalog = FinderCatalog::open(temp.path().join("history.db"), &[]).expect("catalog");
    let rows = catalog
        .search_with_external(
            "hum",
            &[
                item("Helium.exe", r"D:\Tools\Helium.exe", false),
                item("hum", r"D:\Work\hum", true),
            ],
            20,
        )
        .expect("search");
    assert_eq!(rows[0].entry.name, "hum");
}

#[test]
fn latest_exact_keyword_choice_beats_frequency_and_survives_reopen() {
    let temp = tempfile::tempdir().expect("temp");
    let db = temp.path().join("history.db");
    let external = [
        item("Alpha", r"D:\Alpha", true),
        item("Alpha Notes", r"D:\Alpha Notes", true),
    ];
    let mut catalog = FinderCatalog::open(&db, &[]).expect("catalog");
    for timestamp in 1..=10 {
        catalog
            .record_activation(&external[0].id, "alpha", timestamp)
            .expect("old history");
    }
    catalog
        .record_activation(&external[1].id, "alpha", 20)
        .expect("new choice");
    drop(catalog);
    let catalog = FinderCatalog::open(&db, &[]).expect("reopen");
    let rows = catalog
        .search_with_external("alpha", &external, 20)
        .expect("search");
    assert_eq!(rows[0].entry.id, external[1].id);
    assert_eq!(rows[1].activation_count, 10);
}

#[test]
fn prefix_keyword_learning_is_distinct_from_exact_and_unrelated_history() {
    let temp = tempfile::tempdir().expect("temp");
    let db = temp.path().join("history.db");
    let external = [
        item("vis", r"D:\vis", true),
        item("Visual Studio", r"D:\Visual Studio", true),
        item("Visual Notes", r"D:\Visual Notes", true),
    ];
    let mut catalog = FinderCatalog::open(&db, &[]).expect("catalog");
    catalog
        .record_activation(&external[1].id, "visual studio", 20)
        .expect("prefix");
    catalog
        .record_activation(&external[2].id, "notes", 50)
        .expect("unrelated");
    drop(catalog);
    let mut catalog = FinderCatalog::open(&db, &[]).expect("reopen");
    let rows = catalog
        .search_with_external("vis", &external, 20)
        .expect("prefix search");
    assert_eq!(rows[0].entry.id, external[1].id);
    catalog
        .record_activation(&external[0].id, "vis", 1)
        .expect("exact association");
    let rows = catalog
        .search_with_external("vis", &external, 20)
        .expect("exact search");
    assert_eq!(
        rows[0].entry.id, external[0].id,
        "exact keyword association precedes newer prefix association"
    );
}

#[test]
fn app_and_indexed_history_share_identity_without_reset_or_double_count() {
    let temp = tempfile::tempdir().expect("temp");
    let root = temp.path().join("apps");
    std::fs::create_dir(&root).expect("apps");
    std::fs::write(root.join("Visual Editor.lnk"), b"fixture").expect("shortcut");
    let roots = [ApplicationRoot {
        path: root,
        source: ApplicationSource::StartMenu,
    }];
    let db = temp.path().join("history.db");
    let mut catalog = FinderCatalog::open(&db, &roots).expect("catalog");
    let app = catalog
        .entries()
        .iter()
        .find(|entry| entry.name == "Visual Editor")
        .expect("application")
        .clone();
    let indexed = item(
        "Visual Editor.lnk",
        &app.target.replace('\\', "/").to_uppercase(),
        false,
    );
    catalog
        .record_activation(&indexed.id, "visual", 10)
        .expect("indexed history");
    catalog
        .record_activation(&app.id, "visual", 20)
        .expect("application history");
    for _ in 0..2 {
        drop(catalog);
        catalog = FinderCatalog::open(&db, &roots).expect("reopen");
        let rows = catalog
            .search_with_external("visual", std::slice::from_ref(&indexed), 20)
            .expect("search");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].entry.source, ApplicationSource::StartMenu);
        assert_eq!(rows[0].activation_count, 2);
        assert_eq!(rows[0].query_activation_count, 2);
        assert_eq!(rows[0].last_query_activation_at_unix_ms, Some(20));
    }
    catalog.clear_history().expect("clear existing stores");
    let rows = catalog.search("visual", 20).expect("after clear");
    assert_eq!(rows[0].activation_count, 0);
}

#[test]
fn existing_split_history_is_merged_on_load_without_rewriting_user_rows() {
    let temp = tempfile::tempdir().expect("temp");
    let db = temp.path().join("history.db");
    drop(FinderCatalog::open(&db, &[]).expect("schema"));
    let connection = rusqlite::Connection::open(&db).expect("fixture history");
    for (id, count, timestamp) in [
        (r"app:D:\Tools\编辑器.lnk", 3, 10),
        (r"indexed:d:/tools/编辑器.lnk", 2, 20),
    ] {
        connection
            .execute(
                "INSERT INTO quick_launch_usage VALUES (?1,?2,?3)",
                rusqlite::params![id, count, timestamp],
            )
            .expect("usage");
        connection
            .execute(
                "INSERT INTO quick_launch_query_usage VALUES (?1,'edit',?2,?3)",
                rusqlite::params![id, count, timestamp],
            )
            .expect("query usage");
    }
    drop(connection);
    let mut catalog = FinderCatalog::open(&db, &[]).expect("existing history");
    let external = [item("Editor.lnk", r"D:\Tools\编辑器.lnk", false)];
    let rows = catalog
        .search_with_external("edit", &external, 20)
        .expect("search");
    assert_eq!(rows[0].activation_count, 5);
    assert_eq!(rows[0].last_query_activation_at_unix_ms, Some(20));
    catalog
        .record_activation(&external[0].id, "edit", 30)
        .expect("new choice");
    drop(catalog);
    let catalog = FinderCatalog::open(&db, &[]).expect("reopen after write");
    let rows = catalog
        .search_with_external("edit", &external, 20)
        .expect("search");
    assert_eq!(
        rows[0].activation_count, 6,
        "legacy aliases must not be re-counted on write"
    );
    assert_eq!(rows[0].query_activation_count, 6);
}

#[test]
fn duplicate_application_shortcuts_merge_but_same_named_files_remain() {
    let temp = tempfile::tempdir().expect("temp");
    let roots = ["user", "common"].map(|folder| {
        let root = temp.path().join(folder);
        std::fs::create_dir(&root).expect("root");
        std::fs::write(root.join("Editor.lnk"), b"fixture").expect("shortcut");
        ApplicationRoot {
            path: root,
            source: ApplicationSource::StartMenu,
        }
    });
    let catalog = FinderCatalog::open(temp.path().join("history.db"), &roots).expect("catalog");
    let external = [
        item("Editor.txt", r"D:\First\Editor.txt", false),
        item("Editor.txt", r"D:\Second\Editor.txt", false),
    ];
    let rows = catalog
        .search_with_external("editor", &external, 20)
        .expect("search");
    assert_eq!(
        rows.iter()
            .filter(|row| row.entry.source == ApplicationSource::StartMenu)
            .count(),
        1
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.entry.source == ApplicationSource::IndexedFile)
            .count(),
        2
    );
    assert_eq!(rows.len(), 3);
}

#[test]
fn nonmatching_duplicate_cannot_suppress_a_matching_path_alias() {
    let temp = tempfile::tempdir().expect("temp");
    let catalog = FinderCatalog::open(temp.path().join("history.db"), &[]).expect("catalog");
    let external = [
        item("unrelated", r"D:\Tools\Editor.exe", false),
        item("Editor", r"D:\Tools\Editor.exe", false),
    ];
    assert_eq!(
        catalog
            .search_with_external("editor", &external, 20)
            .expect("search")
            .len(),
        1
    );
}
