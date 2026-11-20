use nxtools_finder::{ApplicationSource, FinderCatalog, SearchResultAction};

#[test]
fn windows_builtin_calculator_and_paint_are_searchable_without_shortcuts() {
    let temporary = tempfile::tempdir().expect("system application fixture");
    let catalog = FinderCatalog::open(temporary.path().join("history.db"), &[])
        .expect("open system application catalog");
    for (query, target) in [
        ("计算器", "calc.exe"),
        ("calculator", "calc.exe"),
        ("画图", "mspaint.exe"),
        ("paint", "mspaint.exe"),
    ] {
        let results = catalog
            .search(query, 20)
            .expect("search system application");
        assert!(
            results.iter().any(|result| {
                result.entry.target == target
                    && result.entry.source == ApplicationSource::System
                    && result.action == SearchResultAction::OpenTarget
            }),
            "missing Windows application for {query}"
        );
    }
}

fn installed(name: &str, id: &str) -> nxtools_finder::InstalledApplication {
    nxtools_finder::InstalledApplication {
        name: name.into(),
        target: format!("shell:AppsFolder\\{id}"),
    }
}

#[test]
fn windows_installed_snapshot_discovers_unknown_apps_and_bilingual_builtin_aliases() {
    let temporary = tempfile::tempdir().expect("installed apps fixture");
    let mut catalog =
        FinderCatalog::open(temporary.path().join("history.db"), &[]).expect("catalog");
    let applications = vec![
        installed(
            "Calculator",
            "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
        ),
        installed("Paint", "Microsoft.Paint_8wekyb3d8bbwe!App"),
        installed("Snipping Tool", "Microsoft.ScreenSketch_8wekyb3d8bbwe!App"),
        installed("Camera", "Microsoft.WindowsCamera_8wekyb3d8bbwe!App"),
        installed("未来系统工具", "Future.VendorApp_publisher!Application"),
    ];
    catalog
        .replace_installed_applications(applications.clone())
        .expect("installed snapshot");
    for (query, index) in [
        ("计算器", 0),
        ("calcu", 0),
        ("画图", 1),
        ("paint", 1),
        ("截图工具", 2),
        ("snipping", 2),
        ("相机", 3),
        ("camera", 3),
        ("未来系统工具", 4),
    ] {
        let results = catalog.search(query, 20).expect("installed app search");
        assert!(
            results
                .iter()
                .any(|result| result.entry.target == applications[index].target),
            "{query}"
        );
    }
    assert!(
        catalog
            .entries()
            .iter()
            .all(|entry| entry.target != "calc.exe" && entry.target != "mspaint.exe")
    );
    assert!(!catalog.entries().iter().any(|entry| entry.name == "照片"));
    let first = catalog
        .search("未来系统工具", 1)
        .expect("future app")
        .remove(0);
    catalog
        .record_activation(&first.entry.id, "未来系统工具", 100)
        .expect("record installed usage");
    catalog.refresh(&[]);
    let retained = catalog
        .search("未来系统工具", 1)
        .expect("retained native snapshot")
        .remove(0);
    assert_eq!(retained.activation_count, 1);
    assert_eq!(catalog.status().indexed_items, catalog.entries().len());
}

#[test]
fn windows_installed_snapshot_replacement_rejects_partial_data_and_removes_uninstalled_apps() {
    let temporary = tempfile::tempdir().expect("snapshot retirement fixture");
    let mut catalog =
        FinderCatalog::open(temporary.path().join("history.db"), &[]).expect("catalog");
    let camera = installed("Camera", "Microsoft.WindowsCamera_8wekyb3d8bbwe!App");
    catalog
        .replace_installed_applications(vec![camera.clone(), camera])
        .expect("deduplicated snapshot");
    assert_eq!(catalog.search("camera", 20).expect("one camera").len(), 1);
    let previous = catalog.entries().to_vec();
    assert!(
        catalog
            .replace_installed_applications(vec![
                installed("Paint", "Microsoft.Paint_8wekyb3d8bbwe!App"),
                installed("", "Broken.App_publisher!App")
            ])
            .is_err()
    );
    assert_eq!(catalog.entries(), previous);
    catalog
        .replace_installed_applications(vec![])
        .expect("empty successful snapshot");
    assert!(
        catalog
            .search("camera", 20)
            .expect("removed camera")
            .is_empty()
    );
    assert!(
        catalog
            .entries()
            .iter()
            .any(|entry| entry.target == "mspaint.exe")
    );
}

#[test]
fn windows_app_targets_match_the_native_aumid_contract() {
    use nxtools_finder::is_packaged_application_target;
    assert!(is_packaged_application_target(
        r"shell:AppsFolder\Future.Tool_publisher!App.Main"
    ));
    for target in [
        r"shell:OtherFolder\Future.Tool!App",
        r"shell:AppsFolder\!App",
        r"shell:AppsFolder\Family!",
        r"shell:AppsFolder\Family!App!Extra",
        r"shell:AppsFolder\Family!App --argument",
        r"shell:AppsFolder\Family!App\child",
    ] {
        assert!(!is_packaged_application_target(target), "{target}");
    }
}
