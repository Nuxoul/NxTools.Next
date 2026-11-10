use nxtools_core::CustomCommandSettings;
use nxtools_extensions::{ImportPreview, PackageRegistry};
use nxtools_finder::{ApplicationRoot, ApplicationSource, ExternalSearchEntry, FinderCatalog};

#[test]
fn duplicate_shortcut_prefix_history_survives_enabled_package_merge() {
    let temp = tempfile::tempdir().expect("temp");
    let roots = ["a", "z"].map(|name| {
        let root = temp.path().join(name);
        std::fs::create_dir(&root).expect("root");
        std::fs::write(root.join("Visual Editor.lnk"), b"fixture").expect("shortcut");
        ApplicationRoot {
            path: root,
            source: ApplicationSource::StartMenu,
        }
    });
    std::fs::write(roots[0].path.join("Vis.lnk"), b"fixture").expect("exact name");
    let mut finder = FinderCatalog::open(temp.path().join("history.db"), &roots).expect("catalog");
    let learned = finder
        .entries()
        .iter()
        .find(|entry| entry.target.contains("z\\Visual"))
        .expect("second shortcut")
        .id
        .clone();
    finder
        .record_activation(&learned, "visual editor", 100)
        .expect("prefix choice");
    let json = br#"{"schema_version":1,"id":"fixture.merge","name":"Merge","version":"1.0.0","handlers":[{"id":"open","title":"Open","feature":"quick_launch","entry":"command","keywords":["open"],"action":{"type":"folder","path":"D:\\Fixture"}}]}"#;
    let mut registry = PackageRegistry::default();
    registry
        .install(
            ImportPreview::from_json(json).expect("preview"),
            "fixture".into(),
            false,
        )
        .expect("install");
    registry
        .set_enabled("fixture.merge", true, &[])
        .expect("enable");
    let ordinary = finder.search("vis", 20).expect("ordinary");
    let merged = finder
        .search_with_packages_normal("vis", &[], &[], &registry, 20)
        .expect("package merge");
    assert_eq!(ordinary[0].entry.name, "Visual Editor");
    assert_eq!(merged, ordinary);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].activation_count, 1);
}

#[test]
fn same_target_commands_and_file_are_distinct_actions() {
    let temp = tempfile::tempdir().expect("temp");
    let finder = FinderCatalog::open(temp.path().join("history.db"), &[]).expect("finder");
    let commands = ["one", "two"].map(|profile| CustomCommandSettings {
        keyword: format!("edit-{profile}"),
        title: format!("Editor {profile}"),
        program: r"D:\Tools\Editor.exe".into(),
        argument_template: format!("--profile {profile}"),
        ..Default::default()
    });
    let indexed = [ExternalSearchEntry {
        id: r"indexed:d:\tools\editor.exe".into(),
        name: "Editor.exe".into(),
        target: r"D:\Tools\Editor.exe".into(),
        source: ApplicationSource::IndexedFile,
    }];
    let results = finder
        .search_with_sources("edit", &indexed, &commands, 20)
        .expect("merge");
    assert_eq!(results.len(), 3);
    assert_eq!(
        results
            .iter()
            .filter(|row| row.prepared_command().is_some())
            .count(),
        2
    );
    assert_eq!(results.iter().filter(|row| row.opens_target()).count(), 1);
}

#[test]
fn general_headroom_does_not_bury_a_weak_application_beyond_the_first_ten() {
    let temp = tempfile::tempdir().expect("temp");
    std::fs::write(temp.path().join("Helium.lnk"), b"fixture").expect("application");
    let roots = [ApplicationRoot {
        path: temp.path().to_owned(),
        source: ApplicationSource::StartMenu,
    }];
    let finder = FinderCatalog::open(temp.path().join("history.db"), &roots).expect("finder");
    let external = (0..14)
        .map(|index| ExternalSearchEntry {
            id: format!(r"indexed:d:\hum{index}"),
            name: format!("hum{index}"),
            target: format!(r"D:\hum{index}"),
            source: ApplicationSource::IndexedDirectory,
        })
        .collect::<Vec<_>>();
    let results = finder
        .search_with_external("hum", &external, 30)
        .expect("rank");
    assert_eq!(
        results.iter().position(|row| row.entry.name == "Helium"),
        Some(10)
    );
    assert_eq!(
        finder
            .search_with_external("hum", &external, 5)
            .expect("bounded"),
        results[..5]
    );
}
