use nxtools_extensions::{ImportPreview, PackageError, PackageRegistry, RuntimeIdentity};

const SCRIPT: &str = r#"{
 "schema_version":1,"id":"example.resolve","name":"Resolver","version":"1.0.0",
 "permissions":[{"id":"network","reason":"Resolve original page URL"}],
 "runtime":{"program":"python","timeout_ms":10000},"files":["resolve.py"],
 "settings":[{"key":"mirror","title":"Mirror","type":"text","default":"https://mirror.test"},{"key":"token","title":"Token","type":"password"}],
 "handlers":[{"id":"resolve","title":"Resolve URL","feature":"downloads","entry":"resolve","url_matches":[{"host":"github.com","path":"/*/main"}],"action":{"type":"script","entry_file":"resolve.py"}}]
}"#;

fn source(manifest: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("source");
    std::fs::write(root.path().join("extension.json"), manifest).expect("manifest");
    std::fs::write(root.path().join("resolve.py"), "print('fixture')\n").expect("code");
    root
}

#[test]
fn script_import_requires_real_code_and_approval_never_follows_changed_bytes() {
    let source = source(SCRIPT);
    let staging = tempfile::tempdir().expect("staging");
    let preview = ImportPreview::from_directory(source.path()).expect("verified code preview");
    assert_eq!(preview.code_files.len(), 1);
    let hash = preview.content_hash.clone();
    let mut registry = PackageRegistry::default();
    assert!(
        registry
            .install(preview.clone(), "fixture".into(), false)
            .is_err(),
        "mutable source is not an installed snapshot"
    );
    registry
        .install(
            preview.stage_into(staging.path()).expect("stage"),
            "fixture".into(),
            false,
        )
        .expect("import");
    assert_eq!(
        registry.set_enabled("example.resolve", true, &[]),
        Err(PackageError::Untrusted)
    );
    assert!(
        ImportPreview::from_json(SCRIPT.as_bytes()).is_err(),
        "JSON alone cannot vouch for script"
    );
    std::fs::write(source.path().join("resolve.py"), "print('changed')\n").expect("changed code");
    let updated = ImportPreview::from_directory(source.path()).expect("new preview");
    assert_ne!(
        updated.content_hash, hash,
        "trust identity includes code bytes"
    );
    registry.packages[0]
        .verify_current_code()
        .expect("staged bytes independent of mutable source");
}

#[test]
fn approval_is_bound_to_actual_code_runtime_and_content_and_replacement_is_untrusted() {
    let source = source(SCRIPT);
    let staging = tempfile::tempdir().expect("staging");
    let staged = ImportPreview::from_directory(source.path())
        .expect("preview")
        .stage_into(staging.path())
        .expect("stage");
    let hash = staged.content_hash.clone();
    let code = staging
        .path()
        .join("example.resolve")
        .join(&hash)
        .join("resolve.py");
    let runtime_file = source.path().join("fixture-runtime.exe");
    std::fs::write(&runtime_file, b"synthetic runtime identity; never executed")
        .expect("runtime fixture");
    let runtime = RuntimeIdentity::inspect(&runtime_file).expect("runtime identity");
    let mut registry = PackageRegistry::default();
    registry
        .install(staged.clone(), "fixture".into(), false)
        .expect("install");
    assert!(
        registry
            .approve("example.resolve", &"0".repeat(64), runtime.clone())
            .is_err()
    );
    assert!(registry.packages[0].trust.is_none());
    registry
        .approve("example.resolve", &hash, runtime.clone())
        .expect("explicit approval");
    registry.packages[0]
        .verify_approval()
        .expect("current approval");
    let staged_manifest = code.parent().expect("code root").join("extension.json");
    let original_manifest = std::fs::read(&staged_manifest).expect("staged declaration");
    std::fs::write(
        &staged_manifest,
        SCRIPT.replace("Resolve original page URL", "Changed permission purpose"),
    )
    .expect("change declaration");
    assert_eq!(
        registry.packages[0].verify_approval(),
        Err(PackageError::Untrusted),
        "staged declaration is part of approval identity"
    );
    std::fs::write(&staged_manifest, original_manifest).expect("restore declaration");
    assert!(
        !registry.packages[0].enabled,
        "approval alone never enables"
    );
    registry
        .set_enabled("example.resolve", true, &[])
        .expect("explicit enable after joint acceptance");
    registry.validate().expect("enabled trusted snapshot");
    let encoded = serde_json::to_vec(&registry).expect("registry");
    let reopened: PackageRegistry = serde_json::from_slice(&encoded).expect("reopen");
    reopened.validate().expect("trusted enabled config");
    std::fs::write(&code, b"changed code").expect("change staged fixture");
    assert_eq!(
        registry.packages[0].verify_approval(),
        Err(PackageError::Untrusted)
    );
    registry
        .set_enabled("example.resolve", false, &[])
        .expect("disable changed code");
    assert_eq!(
        registry.set_enabled("example.resolve", true, &[]),
        Err(PackageError::Untrusted)
    );
    std::fs::write(&code, &staged.code_files["resolve.py"]).expect("restore owned code");
    std::fs::write(&runtime_file, b"changed runtime").expect("change runtime");
    assert_eq!(
        registry.packages[0].verify_approval(),
        Err(PackageError::Untrusted)
    );
    std::fs::write(&runtime_file, b"synthetic runtime identity; never executed")
        .expect("restore runtime");
    registry.revoke("example.resolve").expect("revoke");
    assert!(registry.packages[0].trust.is_none());
    assert!(!registry.packages[0].enabled);
    registry
        .approve("example.resolve", &hash, runtime)
        .expect("reapprove");
    registry
        .install(staged, "replacement".into(), true)
        .expect("explicit replacement");
    assert!(
        registry.packages[0].trust.is_none(),
        "even same-content replacement never inherits authorization"
    );
}

#[test]
fn strict_script_settings_reject_unknown_duplicate_and_secret_defaults() {
    for manifest in [
        SCRIPT.replace("\"type\":\"text\"", "\"type\":\"text\",\"unknown\":true"),
        SCRIPT.replace(
            "\"default\":\"https://mirror.test\"",
            "\"default\":\"a\",\"default\":\"b\"",
        ),
        SCRIPT.replace(
            "\"type\":\"password\"",
            "\"type\":\"password\",\"default\":\"secret\"",
        ),
    ] {
        let source = source(&manifest);
        assert!(
            ImportPreview::from_directory(source.path()).is_err(),
            "invalid setting must not become preview"
        );
    }
}

#[test]
fn password_values_cannot_enter_registry_and_invalid_settings_do_not_mutate_state() {
    let source = source(SCRIPT);
    let staging = tempfile::tempdir().expect("staging");
    let staged = ImportPreview::from_directory(source.path())
        .expect("preview")
        .stage_into(staging.path())
        .expect("stage");
    let mut registry = PackageRegistry::default();
    registry
        .install(staged, "fixture".into(), false)
        .expect("install");
    assert!(!registry.packages[0].settings.contains_key("token"));
    let before = registry.clone();
    let mut settings = registry.packages[0].settings.clone();
    settings.insert(
        "token".into(),
        serde_json::Value::String("must-not-persist".into()),
    );
    assert!(registry.set_settings("example.resolve", settings).is_err());
    assert_eq!(registry, before);
    assert!(
        !serde_json::to_string(&registry)
            .expect("config projection")
            .contains("must-not-persist")
    );
}

#[test]
fn windows_code_paths_reject_aliases_reserved_names_and_manifest_collision() {
    for path in [
        "../escape.py",
        "CON",
        "aux.txt",
        "dir./resolve.py",
        "resolve.py ",
        "extension.json",
        "dir\\NUL.py",
        "dir//resolve.py",
    ] {
        let manifest = SCRIPT.replace("resolve.py", path);
        let source = source(&manifest);
        assert!(
            ImportPreview::from_directory(source.path()).is_err(),
            "unsafe or ambiguous code path {path}"
        );
    }
}

#[test]
fn ordinary_provider_can_have_no_keyword_and_matcher_keeps_host_boundary() {
    let manifest = SCRIPT
        .replace(
            "\"feature\":\"downloads\",\"entry\":\"resolve\"",
            "\"feature\":\"quick_launch\",\"entry\":\"provider\"",
        )
        .replace(
            "\"url_matches\":[{\"host\":\"github.com\",\"path\":\"/*/main\"}],",
            "",
        );
    let source = source(&manifest);
    let staging = tempfile::tempdir().expect("staging");
    let staged = ImportPreview::from_directory(source.path())
        .expect("provider preview")
        .stage_into(staging.path())
        .expect("stage");
    let mut registry = PackageRegistry::default();
    registry
        .install(staged, "fixture".into(), false)
        .expect("ordinary provider import");
    registry.validate().expect("keywordless provider registry");
    let source = super_source();
    let rule = ImportPreview::from_directory(source.path())
        .expect("resolver preview")
        .manifest
        .handlers
        .remove(0)
        .url_matches
        .remove(0);
    assert!(rule.matches(&url::Url::parse("https://github.com/project/main").expect("URL")));
    assert!(!rule.matches(&url::Url::parse("https://evilgithub.com/project/main").expect("URL")));
    assert!(!rule.matches(&url::Url::parse("https://github.com/project/dev").expect("URL")));
}

fn super_source() -> tempfile::TempDir {
    source(SCRIPT)
}

#[test]
fn omitted_runtime_deadline_uses_entry_defaults_and_explicit_limits_are_clamped() {
    let defaults: nxtools_extensions::ScriptRuntime =
        serde_json::from_value(serde_json::json!({"program":"python"})).expect("runtime");
    assert_eq!(
        defaults.effective_timeout_ms("quick_launch", "provider"),
        3_000
    );
    assert_eq!(
        defaults.effective_timeout_ms("downloads", "resolve"),
        10_000
    );
    assert_eq!(defaults.effective_timeout_ms("tools", "action"), 30_000);
    assert_eq!(
        defaults.effective_timeout_ms("clipboard", "committed"),
        30_000
    );
    assert!(
        serde_json::to_value(&defaults)
            .expect("encode")
            .get("timeout_ms")
            .is_none()
    );
    let explicit: nxtools_extensions::ScriptRuntime =
        serde_json::from_value(serde_json::json!({"program":"node","timeout_ms":120000}))
            .expect("explicit");
    assert_eq!(
        explicit.effective_timeout_ms("quick_launch", "provider"),
        10_000
    );
    assert_eq!(
        explicit.effective_timeout_ms("downloads", "resolve"),
        30_000
    );
    assert_eq!(explicit.effective_timeout_ms("tools", "action"), 120_000);
}

#[test]
fn runtime_defaults_and_overrides_reopen_and_only_revoke_changed_selections() {
    let source = source(&SCRIPT.replace(
        "\"handlers\":[",
        "\"handlers\":[{\"id\":\"created\",\"title\":\"Created\",\"feature\":\"downloads\",\"entry\":\"created\",\"action\":{\"type\":\"script\",\"entry_file\":\"resolve.py\"}},{\"id\":\"provider\",\"title\":\"Provider\",\"feature\":\"quick_launch\",\"entry\":\"provider\",\"action\":{\"type\":\"script\",\"entry_file\":\"resolve.py\"}},",
    ));
    let staging = tempfile::tempdir().expect("staging");
    let mut registry = PackageRegistry::default();
    let preview = ImportPreview::from_directory(source.path())
        .expect("preview")
        .stage_into(staging.path())
        .expect("stage");
    let hash = preview.content_hash.clone();
    registry
        .install(preview, "fixture".into(), false)
        .expect("install");
    let first = source.path().join("first.exe");
    let second = source.path().join("second.exe");
    std::fs::write(&first, b"first synthetic interpreter").expect("first");
    std::fs::write(&second, b"second synthetic interpreter").expect("second");
    let first = RuntimeIdentity::inspect(&first).expect("first identity");
    let second = RuntimeIdentity::inspect(&second).expect("second identity");
    let defaults = std::collections::BTreeMap::from([("python".into(), first.executable.clone())]);
    registry
        .set_runtime_defaults(defaults.clone())
        .expect("default");
    registry
        .approve("example.resolve", &hash, first.clone())
        .expect("approve inherited");
    assert!(registry.packages[0].runtime_override.is_none());
    registry
        .set_enabled("example.resolve", true, &[])
        .expect("enable");
    registry
        .set_event("example.resolve", "created", true)
        .expect("event opt in");
    registry
        .set_provider("example.resolve", "provider", true)
        .expect("provider opt in");
    let unchanged = registry.clone();
    registry.set_runtime_defaults(defaults).expect("unchanged");
    assert_eq!(registry, unchanged);
    // Install a second real staged package, rather than forging its content identity.
    let manifest = std::fs::read_to_string(source.path().join("extension.json")).expect("manifest");
    std::fs::write(
        source.path().join("extension.json"),
        manifest.replace("example.resolve", "example.override"),
    )
    .expect("second manifest");
    let preview = ImportPreview::from_directory(source.path())
        .expect("second preview")
        .stage_into(staging.path())
        .expect("second stage");
    let override_hash = preview.content_hash.clone();
    registry
        .install(preview, "fixture".into(), false)
        .expect("second install");
    registry
        .set_runtime_override("example.override", Some(first.executable.clone()))
        .expect("override");
    registry
        .approve("example.override", &override_hash, first.clone())
        .expect("approve override");
    registry
        .set_enabled("example.override", true, &[])
        .expect("enable override");
    let pinned = registry.packages[1].clone();
    registry
        .set_runtime_defaults(std::collections::BTreeMap::from([(
            "python".into(),
            second.executable.clone(),
        )]))
        .expect("change inherited runtime");
    assert!(registry.packages[0].trust.is_none());
    assert!(!registry.packages[0].enabled);
    assert!(registry.packages[0].enabled_events.is_empty());
    assert!(registry.packages[0].enabled_providers.is_empty());
    assert_eq!(registry.packages[1], pinned);
    registry
        .set_runtime_override("example.override", None)
        .expect("inherit instead");
    assert!(
        registry.packages[1].trust.is_none(),
        "clearing override changes effective runtime"
    );
    assert!(!registry.packages[1].enabled);
    assert_eq!(
        registry
            .runtime_executable("example.resolve")
            .expect("inherited"),
        Some(second.executable.clone())
    );
    assert_eq!(
        registry
            .runtime_executable("example.override")
            .expect("now inherited"),
        Some(second.executable.clone())
    );
    let before = registry.clone();
    assert!(registry.approve("example.resolve", &hash, first).is_err());
    assert_eq!(
        registry, before,
        "approval cannot bypass configured selection"
    );
    registry
        .approve("example.resolve", &hash, second.clone())
        .expect("reapprove new selection");
    let file = staging.path().join("registry.json");
    std::fs::write(&file, serde_json::to_vec(&registry).expect("encode")).expect("persist");
    let reopened: PackageRegistry =
        serde_json::from_slice(&std::fs::read(file).expect("read")).expect("reopen");
    assert_eq!(registry, reopened);
    reopened.validate().expect("reopened config");
    reopened.packages[0]
        .verify_approval()
        .expect("new approval");
    let mut tampered = reopened.packages[0].clone();
    tampered.runtime_override = Some(source.path().join("different.exe"));
    assert!(
        tampered.verify_approval().is_err(),
        "package override must agree with approval"
    );
}

#[test]
fn legacy_runtime_selection_and_invalid_configuration_are_atomic() {
    let source = source(SCRIPT);
    let staging = tempfile::tempdir().expect("staging");
    let preview = ImportPreview::from_directory(source.path())
        .expect("preview")
        .stage_into(staging.path())
        .expect("stage");
    let hash = preview.content_hash.clone();
    let mut registry = PackageRegistry::default();
    registry
        .install(preview, "fixture".into(), false)
        .expect("install");
    assert_eq!(
        registry
            .runtime_executable("example.resolve")
            .expect("unconfigured"),
        None
    );
    let runtime_file = source.path().join("runtime.exe");
    std::fs::write(&runtime_file, b"synthetic interpreter").expect("runtime");
    let runtime = RuntimeIdentity::inspect(&runtime_file).expect("identity");
    registry
        .approve("example.resolve", &hash, runtime.clone())
        .expect("old UI approval flow");
    assert_eq!(
        registry.packages[0].runtime_override,
        Some(runtime.executable.clone())
    );
    let mut legacy = serde_json::to_value(&registry).expect("legacy projection");
    legacy
        .as_object_mut()
        .expect("registry object")
        .remove("runtime_defaults");
    legacy["packages"][0]
        .as_object_mut()
        .expect("package object")
        .remove("runtime_override");
    let mut registry: PackageRegistry =
        serde_json::from_value(legacy).expect("old persisted config");
    registry
        .set_enabled("example.resolve", true, &[])
        .expect("legacy trust preserved");
    assert_eq!(
        registry
            .runtime_executable("example.resolve")
            .expect("legacy fallback"),
        Some(runtime.executable.clone())
    );
    registry
        .set_runtime_defaults(std::collections::BTreeMap::from([(
            "python".into(),
            runtime_file.clone(),
        )]))
        .expect("same effective default");
    assert!(
        registry.packages[0].trust.is_some(),
        "no lost trust without effective mismatch"
    );
    let before = registry.clone();
    for invalid in [
        std::path::PathBuf::new(),
        "relative.exe".into(),
        source.path().join("bad\n.exe"),
        source.path().join("x".repeat(32_769)),
    ] {
        assert!(
            registry
                .set_runtime_override("example.resolve", Some(invalid.clone()))
                .is_err()
        );
        assert_eq!(registry, before);
        assert!(
            registry
                .set_runtime_defaults(std::collections::BTreeMap::from([(
                    "python".into(),
                    invalid
                )]))
                .is_err()
        );
        assert_eq!(registry, before);
    }
    for key in ["exe", "unknown", "Python"] {
        assert!(
            registry
                .set_runtime_defaults(std::collections::BTreeMap::from([(
                    key.into(),
                    runtime.executable.clone()
                )]))
                .is_err()
        );
        assert_eq!(registry, before);
    }
    assert!(registry.set_runtime_override("missing", None).is_err());
    assert_eq!(registry, before);
    let missing = source.path().join("not-installed.exe");
    registry
        .set_runtime_defaults(std::collections::BTreeMap::from([(
            "python".into(),
            missing.clone(),
        )]))
        .expect("saving configuration does not require installation");
    assert_eq!(
        registry
            .runtime_executable("example.resolve")
            .expect("explicit missing selection"),
        Some(missing.clone())
    );
    assert!(registry.packages[0].trust.is_none());
    assert!(
        RuntimeIdentity::inspect(&missing).is_err(),
        "inspection reports missing runtime"
    );
}

#[test]
fn exe_runtime_is_always_the_staged_entry_and_rejects_external_override() {
    let source = source(&SCRIPT.replace("\"program\":\"python\"", "\"program\":\"exe\""));
    let staging = tempfile::tempdir().expect("staging");
    let preview = ImportPreview::from_directory(source.path())
        .expect("preview")
        .stage_into(staging.path())
        .expect("stage");
    let hash = preview.content_hash.clone();
    let entry = preview
        .code_directory
        .as_ref()
        .expect("code directory")
        .join("resolve.py");
    let mut registry = PackageRegistry::default();
    registry
        .install(preview, "fixture".into(), false)
        .expect("install");
    assert_eq!(
        registry
            .runtime_executable("example.resolve")
            .expect("staged EXE"),
        Some(entry.clone())
    );
    let before = registry.clone();
    assert!(
        registry
            .set_runtime_override("example.resolve", Some(entry.clone()))
            .is_err()
    );
    assert_eq!(registry, before);
    registry
        .set_runtime_override("example.resolve", None)
        .expect("no override to clear");
    let external = source.path().join("external.exe");
    std::fs::write(&external, b"external").expect("external fixture");
    assert!(
        registry
            .approve(
                "example.resolve",
                &hash,
                RuntimeIdentity::inspect(&external).expect("external identity")
            )
            .is_err()
    );
    assert_eq!(registry, before);
    registry
        .approve(
            "example.resolve",
            &hash,
            RuntimeIdentity::inspect(&entry).expect("staged identity"),
        )
        .expect("approve staged EXE");
    assert!(registry.packages[0].runtime_override.is_none());
}

#[test]
fn every_interpreter_has_a_stable_default_key_and_declarative_packages_cannot_override() {
    use nxtools_extensions::ScriptProgram;
    for program in [
        ScriptProgram::Python,
        ScriptProgram::Node,
        ScriptProgram::Powershell,
        ScriptProgram::Pwsh,
    ] {
        let source = source(&SCRIPT.replace(
            "\"program\":\"python\"",
            &format!("\"program\":\"{}\"", program.key()),
        ));
        let staging = tempfile::tempdir().expect("staging");
        let preview = ImportPreview::from_directory(source.path())
            .expect("preview")
            .stage_into(staging.path())
            .expect("stage");
        let mut registry = PackageRegistry::default();
        registry
            .install(preview, "fixture".into(), false)
            .expect("install");
        let missing = source.path().join("missing-interpreter.exe");
        registry
            .set_runtime_defaults(std::collections::BTreeMap::from([(
                program.key().into(),
                missing.clone(),
            )]))
            .expect("known key");
        assert_eq!(
            registry
                .runtime_executable("example.resolve")
                .expect("selected default"),
            Some(missing)
        );
    }
    assert_eq!(ScriptProgram::Exe.key(), "exe");
    let preview = ImportPreview::from_json(br#"{"schema_version":1,"id":"example.web","name":"Web","version":"1","handlers":[{"id":"web","title":"Web","feature":"quick_launch","entry":"command","keywords":["web"],"action":{"type":"website","url_template":"https://example.test/?q={query}"}}]}"#).expect("declarative preview");
    let mut registry = PackageRegistry::default();
    registry
        .install(preview, "fixture".into(), false)
        .expect("install");
    assert_eq!(
        registry
            .runtime_executable("example.web")
            .expect("no runtime"),
        None
    );
    let before = registry.clone();
    let root = tempfile::tempdir().expect("fixture");
    assert!(
        registry
            .set_runtime_override("example.web", Some(root.path().join("external.exe")))
            .is_err()
    );
    assert_eq!(registry, before);
}
