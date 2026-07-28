use nxtools_core::CustomCommandSettings;
use nxtools_finder::{
    ApplicationSource, ExternalSearchEntry, FinderCatalog, SearchResultAction,
    query_requests_file_index_with_commands,
};

fn command() -> CustomCommandSettings {
    CustomCommandSettings {
        keyword: "git".into(),
        title: "Git status".into(),
        program: "git.exe".into(),
        argument_template: "status %s".into(),
        ..CustomCommandSettings::default()
    }
}

#[test]
fn matching_command_is_a_typed_result_without_hiding_other_providers() {
    let directory = tempfile::tempdir().expect("temp dir");
    let catalog =
        FinderCatalog::open(directory.path().join("history.db"), &[]).expect("open catalog");
    let indexed = [ExternalSearchEntry {
        id: "indexed:c:/docs/git-short-guide.txt".into(),
        name: "Git short guide".into(),
        target: r"C:\Docs\git-short-guide.txt".into(),
        source: ApplicationSource::IndexedFile,
    }];
    let results = catalog
        .search_with_sources("g", &indexed, &[command()], 10)
        .expect("search");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].entry.source, ApplicationSource::CustomCommand);
    assert_eq!(results[0].entry.name, "Git status");
    assert_eq!(results[0].entry.target, "git.exe");
    let SearchResultAction::RunCommand { command } = &results[0].action else {
        panic!("expected typed command action");
    };
    assert_eq!(command.executable, "git.exe");
    assert_eq!(command.arguments, ["status", ""]);
    assert!(
        results
            .iter()
            .any(|result| result.entry.source == ApplicationSource::IndexedFile)
    );
}

#[test]
fn command_prefixes_still_request_the_file_index() {
    let commands = [command()];
    assert!(query_requests_file_index_with_commands("g", &commands));
    assert!(query_requests_file_index_with_commands(
        "git one", &commands
    ));
    assert!(query_requests_file_index_with_commands("github", &commands));
    assert!(query_requests_file_index_with_commands("report", &commands));
}

#[test]
fn disabled_and_invalid_commands_do_not_match() {
    let directory = tempfile::tempdir().expect("temp dir");
    let catalog =
        FinderCatalog::open(directory.path().join("history.db"), &[]).expect("open catalog");
    let mut disabled = command();
    disabled.enabled = false;
    let mut invalid = command();
    invalid.argument_template = "%s0".into();
    assert!(
        catalog
            .search_with_sources("git value", &[], &[disabled, invalid], 10)
            .expect("search")
            .is_empty()
    );
}

#[test]
fn custom_command_prefix_uses_activation_history_order() {
    let directory = tempfile::tempdir().expect("temp dir");
    let mut catalog =
        FinderCatalog::open(directory.path().join("history.db"), &[]).expect("open catalog");
    let mut go = command();
    go.keyword = "go".into();
    go.title = "Go".into();
    catalog
        .record_activation("custom-command:go", "g", 1_700_000_000_000)
        .expect("record command activation");

    let results = catalog
        .search_with_sources("g", &[], &[command(), go], 10)
        .expect("search prefix");
    assert_eq!(results[0].entry.id, "custom-command:go");
}
