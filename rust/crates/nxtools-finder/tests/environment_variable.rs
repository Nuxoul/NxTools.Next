use nxtools_finder::{
    ApplicationSource, ExternalSearchEntry, FinderCatalog, SearchResultAction,
    query_requests_file_index,
};
use tempfile::tempdir;

#[test]
fn complete_environment_variable_is_resolved_case_insensitively() {
    let (name, expected) = ["TEMP", "TMP", "USERPROFILE", "SYSTEMROOT"]
        .into_iter()
        .find_map(|name| {
            let value = std::env::var_os(name)?.to_string_lossy().into_owned();
            std::path::Path::new(&value)
                .is_dir()
                .then_some((name, value))
        })
        .expect("the Windows test process must expose an existing directory variable");

    let directory = tempdir().expect("temporary catalog");
    let catalog = FinderCatalog::open(directory.path().join("history.sqlite3"), &[])
        .expect("open empty catalog");
    let results = catalog
        .search(&format!("  %{}%  ", name.to_ascii_lowercase()), 10)
        .expect("resolve directory variable");
    let variable = &results[0];

    assert_eq!(variable.entry.name, expected);
    assert_eq!(variable.entry.target, variable.entry.name);
    assert_eq!(variable.action, SearchResultAction::OpenTarget);
}

#[test]
fn existing_directory_environment_variable_is_an_open_target() {
    let directory = tempdir().expect("temporary catalog");
    let catalog = FinderCatalog::open(directory.path().join("history.sqlite3"), &[])
        .expect("open empty catalog");
    let (name, expected) = ["TEMP", "TMP", "USERPROFILE", "SYSTEMROOT"]
        .into_iter()
        .find_map(|name| {
            let value = std::env::var_os(name)?.to_string_lossy().into_owned();
            std::path::Path::new(&value)
                .is_dir()
                .then_some((name, value))
        })
        .expect("the Windows test process must expose an existing directory variable");

    let results = catalog
        .search(&format!("%{name}%"), 10)
        .expect("search environment variable");

    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(result.entry.source, ApplicationSource::EnvironmentVariable);
    assert_eq!(result.entry.name, expected);
    assert_eq!(result.entry.target, result.entry.name);
    assert!(result.subtitle.as_deref().is_some_and(|subtitle| {
        subtitle.contains(&format!("%{name}%")) && subtitle.contains("打开文件夹")
    }));
    assert_eq!(result.action, SearchResultAction::OpenTarget);
}

#[test]
fn incomplete_unknown_and_path_like_inputs_are_not_environment_results() {
    let directory = tempdir().expect("temporary catalog");
    let catalog = FinderCatalog::open(directory.path().join("history.sqlite3"), &[])
        .expect("open empty catalog");
    for query in [
        "%PATH",
        "%NXTOOLS_VARIABLE_THAT_DOES_NOT_EXIST_7D3E2A1C%",
        r"C:\%PATH%\file.txt",
    ] {
        assert!(
            catalog
                .search(query, 10)
                .expect("search invalid environment query")
                .iter()
                .all(|result| result.entry.source != ApplicationSource::EnvironmentVariable),
            "unexpected environment result for {query}"
        );
    }
}

#[test]
fn complete_variable_syntax_never_requests_the_file_index() {
    assert!(!query_requests_file_index("%PATH%"));
    assert!(!query_requests_file_index(
        "%NXTOOLS_VARIABLE_THAT_DOES_NOT_EXIST_7D3E2A1C%"
    ));
    assert!(query_requests_file_index("%PATH"));
    assert!(query_requests_file_index("%PATH-NAME%"));
}

#[test]
fn resolved_variable_owns_the_query_instead_of_merging_file_candidates() {
    let directory = tempdir().expect("temporary catalog");
    let catalog = FinderCatalog::open(directory.path().join("history.sqlite3"), &[])
        .expect("open empty catalog");
    let external = [ExternalSearchEntry {
        id: "indexed:path".into(),
        name: "%PATH%".into(),
        target: r"C:\Archive\%PATH%.txt".into(),
        source: ApplicationSource::IndexedFile,
    }];

    let results = catalog
        .search_with_external("%PATH%", &external, 10)
        .expect("search exact environment variable");

    assert!(!results.is_empty());
    assert!(
        results
            .iter()
            .all(|result| result.entry.source == ApplicationSource::EnvironmentVariable)
    );
    assert!(
        results
            .iter()
            .all(|result| result.entry.id != external[0].id)
    );
}
