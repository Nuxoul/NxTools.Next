use nxtools_extensions::{ImportPreview, PackageRegistry};
use nxtools_finder::{ApplicationSource, ExternalSearchEntry, FinderCatalog, SearchResultAction};

fn packages() -> PackageRegistry {
    let json = br#"{"schema_version":1,"id":"example.commands","name":"Commands","version":"1.0.0","handlers":[
        {"id":"web","title":"Web search","feature":"quick_launch","entry":"command","keywords":["web"],"action":{"type":"website","url_template":"https://example.test/?q={query}"}},
        {"id":"run","title":"Run command","feature":"quick_launch","entry":"command","keywords":["run"],"action":{"type":"program","executable":"C:\\Fixture\\tool.exe","argument_template":"--name %s","working_directory":"D:\\Work"}},
        {"id":"places","title":"Places","feature":"quick_launch","entry":"command","keywords":["places"],"action":{"type":"items","items":[
            {"id":"docs","title":"Documentation","action":{"type":"folder","path":"C:\\Fixture\\Docs"}},
            {"id":"source","title":"Source files","action":{"type":"folder","path":"C:\\Fixture\\Source"}}
        ]}}
    ]}"#;
    let mut registry = PackageRegistry::default();
    registry
        .install(
            ImportPreview::from_json(json).expect("preview"),
            "fixture".into(),
            false,
        )
        .expect("install");
    registry
        .set_enabled("example.commands", true, &[])
        .expect("enable");
    registry
}

#[test]
fn enabled_package_uses_real_finder_url_and_existing_command_preparation() {
    let temp = tempfile::tempdir().expect("temp");
    let finder = FinderCatalog::open(temp.path().join("history.db"), &[]).expect("finder");
    let registry = packages();
    let external = [ExternalSearchEntry {
        id: "indexed:guide".into(),
        name: "web guide".into(),
        target: "C:\\Fixture\\web guide.txt".into(),
        source: ApplicationSource::IndexedFile,
    }];
    let results = finder
        .search_with_packages("web 中文 & name", &external, &[], &registry, 20)
        .expect("search");
    let web = results
        .iter()
        .find(|row| row.entry.id == "extension:example.commands/web/action")
        .expect("package row");
    assert_eq!(
        web.entry.target,
        "https://example.test/?q=%E4%B8%AD%E6%96%87%20%26%20name"
    );
    assert_eq!(web.action, SearchResultAction::OpenTarget);
    let run = finder
        .search_with_packages("run \"two words\"", &[], &[], &registry, 20)
        .expect("program");
    let command = run
        .iter()
        .find_map(|row| row.prepared_command())
        .expect("prepared");
    assert_eq!(command.executable, r"C:\Fixture\tool.exe");
    assert_eq!(command.arguments, ["--name", "\"two words\""]);
    assert_eq!(command.working_directory.as_deref(), Some(r"D:\Work"));
    assert!(!command.run_as_admin);
    let merged = finder
        .search_with_packages_normal("w", &external, &[], &registry, 20)
        .expect("merge");
    assert!(merged.iter().any(|row| row.entry.id == "indexed:guide"));
}

#[test]
fn fixed_list_ids_and_disabled_package_state_survive_query_changes() {
    let temp = tempfile::tempdir().expect("temp");
    let finder = FinderCatalog::open(temp.path().join("history.db"), &[]).expect("finder");
    let mut registry = packages();
    let rows = finder
        .search_with_packages("places doc", &[], &[], &registry, 20)
        .expect("items");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].entry.id, "extension:example.commands/places/docs");
    assert_eq!(rows[0].entry.target, r"C:\Fixture\Docs");
    registry
        .set_enabled("example.commands", false, &[])
        .expect("disable");
    let rows = finder
        .search_with_packages("places doc", &[], &[], &registry, 20)
        .expect("disabled");
    assert!(
        rows.iter()
            .all(|row| !row.entry.id.starts_with("extension:"))
    );
}

#[test]
fn user_alias_changes_query_entry_without_losing_stable_result_id() {
    let temp = tempfile::tempdir().expect("temp");
    let finder = FinderCatalog::open(temp.path().join("history.db"), &[]).expect("finder");
    let mut registry = packages();
    registry
        .set_aliases("example.commands", "web", vec!["lookup".into()], &[])
        .expect("alias");
    let new = finder
        .search_with_packages("lookup hello", &[], &[], &registry, 20)
        .expect("new keyword");
    assert!(
        new.iter()
            .any(|row| row.entry.id == "extension:example.commands/web/action")
    );
    let old = finder
        .search_with_packages("web hello", &[], &[], &registry, 20)
        .expect("old keyword");
    assert!(
        old.iter()
            .all(|row| row.entry.id != "extension:example.commands/web/action")
    );
}

#[test]
fn website_template_can_use_encoded_words_in_a_path_without_turning_spaces_into_plus() {
    let (target, _, _) = nxtools_finder::prepare_package_action(
        &nxtools_extensions::DeclarativeAction::Website {
            url_template: "https://example.test/find/{query}".into(),
        },
        "two words/中文",
    )
    .expect("path substitution");
    assert_eq!(
        target,
        "https://example.test/find/two%20words%2F%E4%B8%AD%E6%96%87"
    );
}

#[test]
fn program_template_preview_uses_existing_validation_before_execution() {
    let mut registry = packages();
    if let nxtools_extensions::DeclarativeAction::Program {
        argument_template, ..
    } = &mut registry.packages[0].manifest.handlers[1].action
    {
        *argument_template = "%s0".into();
    }
    assert!(nxtools_finder::validate_package_commands(&registry.packages[0].manifest).is_err());
}
