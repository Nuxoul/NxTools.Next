use nxtools_extensions::{ImportPreview, PackageRegistry};
use serde_json::{Value, json};
fn preview(settings: Value) -> Result<ImportPreview, nxtools_extensions::PackageError> {
    let manifest = json!({"schema_version":1,"id":"fixture.validation","name":"Validation","version":"1","settings":settings,
        "handlers":[{"id":"web","title":"Web","feature":"tools","entry":"action","action":{"type":"website","url_template":"https://fixture.test"}}]});
    ImportPreview::from_json(&serde_json::to_vec(&manifest).expect("manifest"))
}
#[test]
fn expansion_settings_formal_import_preserves_legacy_choices_and_validates_unicode_whole_pattern() {
    let old=preview(json!([{"key":"choice","title":"Choice","type":"choice","default":"a","options":["a","a","b"]}])).expect("legacy duplicates remain valid");
    let raw = serde_json::to_value(&old.manifest.settings[0]).expect("serialize");
    assert_eq!(
        raw,
        json!({"key":"choice","title":"Choice","description":"","type":"choice","default":"a","options":["a","a","b"]})
    );
    let modern=preview(json!([
        {"key":"name","title":"Name","type":"text","validation":{"required":true,"min_length":2,"max_length":3,"pattern":"[\\p{L}]+"}},
        {"key":"choice","title":"Choice","type":"choice","default":"raw","options":[{"value":"raw","label":"显示标签"},{"value":"other","label":"其他"}]}
    ])).expect("new settings declarations");
    let field = &modern.manifest.settings[0];
    assert!(
        field.accepts(&json!("汉𠀀")),
        "two Unicode scalars, not UTF8 bytes or UTF16 units"
    );
    for invalid in ["", "  ", "汉", "汉字甲乙", "汉1", "汉字!"] {
        assert!(!field.accepts(&json!(invalid)));
    }
    assert!(modern.manifest.settings[1].accepts(&json!("raw")));
    assert!(
        !modern.manifest.settings[1].accepts(&json!("显示标签")),
        "label is never stored value"
    );
    let mut registry = PackageRegistry::default();
    registry
        .install(modern, "owned".into(), false)
        .expect("required unconfigured import");
    registry.validate().expect("unconfigured state can load");
}

#[tokio::test]
async fn expansion_settings_required_missing_save_rejects_before_any_secret_io_or_commit() {
    use nxtools_extensions::{
        SecretFuture, SecretStore, SettingsDraft, SettingsSaveError, save_settings,
    };
    use std::{
        collections::BTreeMap,
        sync::atomic::{AtomicUsize, Ordering},
    };
    struct Store(AtomicUsize);
    impl SecretStore for Store {
        fn get<'a>(&'a self, _: &'a str, _: &'a str) -> SecretFuture<'a, Option<String>> {
            Box::pin(async move {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(None)
            })
        }
        fn set<'a>(&'a self, _: &'a str, _: &'a str, _: Option<&'a str>) -> SecretFuture<'a, ()> {
            Box::pin(async move {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        }
    }
    let mut registry = PackageRegistry::default();
    registry
        .install(
            preview(json!([
                {"key":"name","title":"Name","type":"text","validation":{"required":true}},
                {"key":"token","title":"Token","type":"password"}
            ]))
            .expect("unconfigured import"),
            "owned".into(),
            false,
        )
        .expect("install");
    let before = registry.clone();
    assert!(
        registry
            .set_settings("fixture.validation", BTreeMap::new())
            .is_err(),
        "explicit Save requires readiness"
    );
    assert_eq!(registry, before);
    let store = Store(AtomicUsize::new(0));
    let commits = AtomicUsize::new(0);
    let result = save_settings(
        &registry.packages[0],
        SettingsDraft {
            values: BTreeMap::new(),
            secrets: BTreeMap::from([("token".into(), Some("synthetic".into()))]),
        },
        &store,
        |_| async {
            commits.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
    )
    .await;
    assert_eq!(result, Err(SettingsSaveError::Invalid));
    assert_eq!(
        store.0.load(Ordering::SeqCst),
        0,
        "no secret reads or writes"
    );
    assert_eq!(commits.load(Ordering::SeqCst), 0);
    registry
        .set_settings(
            "fixture.validation",
            BTreeMap::from([("name".into(), json!("configured"))]),
        )
        .expect("valid explicit save");
}

#[test]
fn expansion_settings_invalid_declarations_and_nonempty_invalid_defaults_fail_before_import() {
    for validation in [
        json!({"min_length":4,"max_length":3}),
        json!({"min_length":32769}),
        json!({"pattern":"(?=x)"}),
        json!({"pattern":"x".repeat(513)}),
        json!({"pattern":"(x){1000000}"}),
        json!({"required":true,"unknown":true}),
        json!({"min_length":-1}),
    ] {
        assert!(
            preview(json!([{"key":"name","title":"Name","type":"text","validation":validation}]))
                .is_err()
        );
    }
    for kind in ["bool", "number", "choice", "password"] {
        assert!(
            preview(
                json!([{"key":"name","title":"Name","type":kind,"validation":{"required":true}}])
            )
            .is_err()
        );
    }
    for options in [
        json!(["raw",{"value":"raw","label":"标签"}]),
        json!([{"value":"raw","label":"A"},{"value":"raw","label":"B"}]),
        json!([{"value":"raw","label":"A","unknown":true}]),
        json!([{"value":"raw","label":"bad\nlabel"}]),
    ] {
        assert!(preview(json!([{"key":"name","title":"Name","type":"choice","default":"raw","options":options}])).is_err());
    }
    for value in ["x", "  "] {
        assert!(preview(json!([{"key":"name","title":"Name","type":"text","default":value,"validation":{"required":true,"min_length":2,"pattern":"[a-z]+"}}])).is_err());
    }
    for kind in ["file", "folder"] {
        let imported=preview(json!([{"key":"name","title":"Name","type":kind,"default":"\\\\offline.invalid\\missing","validation":{"required":true,"min_length":2}}])).expect("path shape only; no existence or share probe");
        assert!(
            imported
                .manifest
                .settings_ready(&std::collections::BTreeMap::new()),
            "valid declared default supplies readiness"
        );
    }
}

#[tokio::test]
async fn expansion_settings_unconfigured_invocation_rejects_before_native_or_host_dispatch() {
    use nxtools_extensions::{
        Causality, ExtensionRuntime, FeatureAdapter, HostContext, HostFuture, HostReply,
        InvokeRequest, RuntimeError, RuntimeIdentity,
    };
    use std::{
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };
    struct Adapter(Arc<AtomicUsize>);
    impl FeatureAdapter for Adapter {
        fn describe(&self) -> Value {
            json!({})
        }
        fn call<'a>(&'a self, _: &'a HostContext, _: &'a str, _: Value) -> HostFuture<'a> {
            Box::pin(async move {
                self.0.fetch_add(1, Ordering::SeqCst);
                HostReply::error("unsupported")
            })
        }
    }
    let root = tempfile::tempdir().expect("owned invocation fixture");
    let source = root.path().join("source");
    std::fs::create_dir(&source).expect("source");
    let manifest = json!({"schema_version":1,"id":"fixture.ready","name":"Ready","version":"1","runtime":{"program":"python"},"files":["entry.py"],
        "settings":[{"key":"name","title":"Name","type":"text","validation":{"required":true}}],
        "handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":"entry.py"}}]});
    std::fs::write(
        source.join("extension.json"),
        serde_json::to_vec(&manifest).expect("manifest"),
    )
    .expect("write");
    std::fs::write(
        source.join("entry.py"),
        "raise RuntimeError('must never run')\n",
    )
    .expect("unexecuted script");
    let preview = ImportPreview::from_directory(&source)
        .expect("preview")
        .stage_into(&root.path().join("packages"))
        .expect("stage");
    let hash = preview.content_hash.clone();
    let mut registry = PackageRegistry::default();
    registry
        .install(preview, "owned".into(), false)
        .expect("install");
    let python = std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|path| path.join("python.exe"))
        .find(|path| path.is_file())
        .expect("fixture Python");
    registry
        .approve(
            "fixture.ready",
            &hash,
            RuntimeIdentity::inspect(&python).expect("runtime identity"),
        )
        .expect("approve");
    registry
        .set_enabled("fixture.ready", true, &[])
        .expect("enable");
    let calls = Arc::new(AtomicUsize::new(0));
    let runtime = ExtensionRuntime::new(registry, PathBuf::new(), Arc::new(Adapter(calls.clone())));
    let (_cancel, cancellation) = tokio::sync::watch::channel(false);
    let result = runtime
        .invoke(InvokeRequest {
            package_id: "fixture.ready".into(),
            handler_id: "run".into(),
            correlation_id: uuid::Uuid::new_v4(),
            generation: 1,
            context: json!({}),
            causality: Causality::root(uuid::Uuid::new_v4()),
            cancellation,
            batches: None,
        })
        .await;
    assert!(
        matches!(result, Err(RuntimeError::InvalidSettings)),
        "no broker/native-unavailable fallback"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    runtime.shutdown();
}

#[test]
fn expansion_settings_unicode_byte_cap_and_no_normalization_or_legacy_revalidation() {
    let imported=preview(json!([{"key":"name","title":"Name","type":"text","default":"","validation":{"max_length":32768}}])).expect("optional empty default");
    assert!(imported.manifest.settings[0].accepts(&json!("x".repeat(32768))));
    assert!(
        !imported.manifest.settings[0].accepts(&json!("汉".repeat(10923))),
        "scalar budget never overrides UTF8 byte cap"
    );
    let legacy =
        preview(json!([{"key":"name","title":"Name","type":"text","default":""}])).expect("legacy");
    assert!(
        legacy.manifest.settings[0].accepts(&json!("  ")),
        "no retroactive required rule"
    );
    let mut registry = PackageRegistry::default();
    registry
        .install(imported, "owned".into(), false)
        .expect("install");
    registry
        .set_settings(
            "fixture.validation",
            std::collections::BTreeMap::from([("name".into(), json!("  x  "))]),
        )
        .expect("accepted shape");
    assert_eq!(
        registry.packages[0].settings["name"], "  x  ",
        "accepted values not trimmed or normalized"
    );
}
