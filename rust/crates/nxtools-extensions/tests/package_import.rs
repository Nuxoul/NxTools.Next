use nxtools_extensions::ImportPreview;

const WEBSITE: &str = r#"{
    "schema_version": 1,
    "id": "example.web",
    "name": "网站搜索",
    "version": "1.0.0",
    "handlers": [{
        "id": "search",
        "title": "搜索示例网站",
        "feature": "quick_launch",
        "entry": "command",
        "keywords": ["web"],
        "action": {"type":"website","url_template":"https://example.test/search?q={query}"}
    }]
}"#;

#[test]
fn declarative_import_preview_preserves_identity_and_never_needs_a_runtime() {
    let preview = ImportPreview::from_json(WEBSITE.as_bytes()).expect("valid declarative package");
    assert_eq!(preview.manifest.id, "example.web");
    assert_eq!(preview.manifest.handlers[0].keywords, ["web"]);
    assert_eq!(preview.content_hash.len(), 64);
    assert_eq!(
        ImportPreview::from_json(WEBSITE.as_bytes()).expect("repeat preview"),
        preview
    );
}

#[test]
fn invalid_declarations_never_become_an_import_preview() {
    for (label, json) in [
        (
            "traversal identity",
            WEBSITE.replace("example.web", "../example"),
        ),
        ("empty title", WEBSITE.replace("网站搜索", "")),
        (
            "keyword whitespace",
            WEBSITE.replace("\"web\"", "\"two words\""),
        ),
        (
            "unsupported URL",
            WEBSITE.replace("https://example.test", "file:///example"),
        ),
        (
            "credentials URL",
            WEBSITE.replace("example.test", "user:password@example.test"),
        ),
        (
            "unsupported feature",
            WEBSITE.replace("quick_launch", "unknown_feature"),
        ),
        (
            "privilege request",
            WEBSITE.replace(
                "\"type\":\"website\"",
                "\"type\":\"website\",\"run_as_admin\":true",
            ),
        ),
        (
            "duplicate JSON key",
            WEBSITE.replace(
                "\"schema_version\": 1",
                "\"schema_version\": 1,\"schema_version\": 1",
            ),
        ),
    ] {
        assert!(
            ImportPreview::from_json(json.as_bytes()).is_err(),
            "{label} must be rejected"
        );
    }
    let mut duplicate: serde_json::Value = serde_json::from_str(WEBSITE).expect("fixture");
    let handler = duplicate["handlers"][0].clone();
    duplicate["handlers"]
        .as_array_mut()
        .expect("handlers")
        .push(handler);
    assert!(
        ImportPreview::from_json(&serde_json::to_vec(&duplicate).expect("fixture JSON")).is_err()
    );
}

#[test]
fn excessive_or_newer_manifests_have_fixed_error_categories() {
    use nxtools_extensions::{MAX_MANIFEST_BYTES, PackageError};
    assert_eq!(
        ImportPreview::from_json(&vec![b' '; MAX_MANIFEST_BYTES + 1]),
        Err(PackageError::ManifestTooLarge)
    );
    assert_eq!(
        ImportPreview::from_json(
            WEBSITE
                .replace("schema_version\": 1", "schema_version\": 2")
                .as_bytes()
        ),
        Err(PackageError::UnsupportedVersion)
    );
}

#[test]
fn registry_requires_explicit_enable_and_preserves_state_on_conflict_or_replace_failure() {
    use nxtools_extensions::{PackageError, PackageRegistry};
    let mut registry = PackageRegistry::default();
    registry
        .install(
            ImportPreview::from_json(WEBSITE.as_bytes()).expect("preview"),
            "fixture.json".into(),
            false,
        )
        .expect("import");
    assert!(
        !registry.packages[0].enabled,
        "import is not authorization to execute"
    );
    let before = registry.clone();
    assert!(matches!(
        registry.set_enabled("example.web", true, &["WEB".into()]),
        Err(PackageError::KeywordConflict { .. })
    ));
    assert_eq!(registry, before, "conflicting enable changes nothing");
    registry
        .set_enabled("example.web", true, &[])
        .expect("explicit enable");
    let enabled = registry.clone();
    assert_eq!(
        registry.install(
            ImportPreview::from_json(WEBSITE.as_bytes()).expect("preview"),
            "other.json".into(),
            false
        ),
        Err(PackageError::AlreadyInstalled)
    );
    assert_eq!(registry, enabled);
    registry
        .install(
            ImportPreview::from_json(WEBSITE.replace("1.0.0", "1.1.0").as_bytes()).expect("update"),
            "update.json".into(),
            true,
        )
        .expect("explicit replace");
    assert!(
        !registry.packages[0].enabled,
        "new content must not inherit execution authorization"
    );
    let persisted = serde_json::to_vec(&registry).expect("registry JSON");
    let reopened: PackageRegistry = serde_json::from_slice(&persisted).expect("reopen");
    assert_eq!(reopened, registry);
}

#[test]
fn enabled_packages_cannot_hide_another_packages_keyword() {
    use nxtools_extensions::PackageRegistry;
    let mut registry = PackageRegistry::default();
    for id in ["example.web", "example.other"] {
        registry
            .install(
                ImportPreview::from_json(WEBSITE.replace("example.web", id).as_bytes())
                    .expect("preview"),
                "fixture".into(),
                false,
            )
            .expect("import");
    }
    registry
        .set_enabled("example.web", true, &[])
        .expect("first enabled");
    let before = registry.clone();
    assert!(registry.set_enabled("example.other", true, &[]).is_err());
    assert_eq!(registry, before);
    registry
        .set_enabled("example.web", false, &[])
        .expect("disable first");
    registry
        .set_enabled("example.other", true, &[])
        .expect("resolved conflict");
}

#[test]
fn nested_duplicate_fields_and_nested_lists_are_not_silently_interpreted() {
    let duplicate = WEBSITE.replace(
        "\"url_template\":",
        "\"url_template\":\"https://first.test/\",\"url_template\":",
    );
    assert!(
        ImportPreview::from_json(duplicate.as_bytes()).is_err(),
        "nested tagged action duplicate key"
    );
    let mut nested: serde_json::Value = serde_json::from_str(WEBSITE).expect("fixture");
    nested["handlers"][0]["action"] = serde_json::json!({
        "type":"items", "items":[{"id":"outer","title":"Outer","action":{
            "type":"items","items":[{"id":"inner","title":"Inner","action":{"type":"folder","path":"C:\\Fixture"}}]
        }}]
    });
    assert!(ImportPreview::from_json(&serde_json::to_vec(&nested).expect("JSON")).is_err());
}

#[test]
fn user_alias_resolves_conflict_without_changing_package_content_identity() {
    use nxtools_extensions::PackageRegistry;
    let mut registry = PackageRegistry::default();
    registry
        .install(
            ImportPreview::from_json(WEBSITE.as_bytes()).expect("preview"),
            "fixture".into(),
            false,
        )
        .expect("install");
    let hash = registry.packages[0].content_hash.clone();
    registry
        .set_aliases(
            "example.web",
            "search",
            vec!["lookup".into()],
            &["web".into()],
        )
        .expect("alias change");
    registry
        .set_enabled("example.web", true, &["web".into()])
        .expect("resolved enable");
    assert_eq!(registry.packages[0].content_hash, hash);
    let before = registry.clone();
    assert!(
        registry
            .set_aliases("example.web", "search", vec!["WEB".into()], &["web".into()])
            .is_err()
    );
    assert_eq!(registry, before);
    registry.validate().expect("valid alias state");
}

#[test]
fn import_rejects_invalid_program_template_before_any_search_or_execution() {
    let mut json: serde_json::Value = serde_json::from_str(WEBSITE).expect("fixture");
    json["handlers"][0]["action"] =
        serde_json::json!({"type":"program","executable":"fixture.exe","argument_template":"%s0"});
    assert!(
        ImportPreview::from_json(&serde_json::to_vec(&json).expect("json")).is_err(),
        "invalid positional argument must fail import"
    );
    json["handlers"][0]["action"]["argument_template"] = serde_json::json!("\"unterminated");
    assert!(ImportPreview::from_json(&serde_json::to_vec(&json).expect("json")).is_err());
}
