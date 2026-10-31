use nxtools_extensions::ImportPreview;
use serde_json::{Value, json};

const CODE: &[u8] = b"synthetic entry; never executed\n";
const LEGACY_CANONICAL: &str = r#"{"schema_version":1,"id":"fixture.transport","name":"Transport","version":"1","description":"","author":"","permissions":[],"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","keywords":[],"action":{"type":"script","entry_file":"entry.py"}}],"runtime":{"program":"python","arguments":[]},"files":["entry.py"]}"#;

fn manifest() -> Value {
    json!({
        "schema_version":1,"id":"fixture.transport","name":"Transport","version":"1",
        "runtime":{"program":"python"},"files":["entry.py"],
        "handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action",
            "action":{"type":"script","entry_file":"entry.py"}}]
    })
}
fn preview(manifest: &Value) -> Result<ImportPreview, nxtools_extensions::PackageError> {
    let root = tempfile::tempdir().expect("owned transport declaration fixture");
    std::fs::write(
        root.path().join("extension.json"),
        serde_json::to_vec(manifest).expect("fixture JSON"),
    )
    .expect("owned manifest");
    std::fs::write(root.path().join("entry.py"), CODE).expect("owned unexecuted code");
    ImportPreview::from_directory(root.path())
}

#[test]
fn expansion_transport_declaration_preserves_legacy_bytes_and_explicit_defaults() {
    let legacy = preview(&manifest()).expect("legacy import");
    assert_eq!(
        serde_json::to_string(&legacy.manifest).expect("canonical legacy"),
        LEGACY_CANONICAL
    );
    assert_eq!(
        legacy.content_hash, "23116cf22f41756531b1d48c90c9827ead6197b5776d6d1a2eb087bc9216456e",
        "literal canonical corpus independently hashed before the transport change"
    );
    let mut explicit = manifest();
    explicit["runtime"]["protocol_version"] = json!(1);
    explicit["runtime"]["mode"] = json!("once");
    let imported = preview(&explicit).expect("explicit protocol1 once is the legacy default");
    assert_eq!(
        serde_json::to_string(&imported.manifest).expect("explicit canonical"),
        LEGACY_CANONICAL
    );
    assert_eq!(legacy.content_hash, imported.content_hash);
    let mut v2 = manifest();
    v2["runtime"]["protocol_version"] = json!(2);
    let imported = preview(&v2).expect("explicit v2 declaration");
    assert_eq!(
        serde_json::to_value(imported.manifest).expect("v2 declaration")["runtime"]["protocol_version"],
        2
    );
    assert_ne!(
        legacy.content_hash, imported.content_hash,
        "transport change needs new approval"
    );
}

#[test]
fn expansion_transport_declaration_rejects_unknown_versions_and_invalid_resident_shapes() {
    for runtime in [
        json!({"program":"python","protocol_version":0}),
        json!({"program":"python","protocol_version":3}),
        json!({"program":"python","protocol_version":"2"}),
        json!({"program":"python","protocol_version":null}),
        json!({"program":"python","mode":"eager"}),
        json!({"program":"python","mode":"resident","resident_entry_file":"entry.py"}),
        json!({"program":"python","protocol_version":2,"mode":"resident"}),
        json!({"program":"python","protocol_version":2,"mode":"resident","resident_entry_file":"../entry.py"}),
        json!({"program":"python","protocol_version":2,"mode":"resident","resident_entry_file":"other.py"}),
        json!({"program":"python","protocol_version":2,"resident_entry_file":"entry.py"}),
        json!({"program":"python","protocol_version":2,"idle_exit_seconds":120}),
        json!({"program":"python","protocol_version":2,"mode":"resident","resident_entry_file":"entry.py","idle_exit_seconds":0}),
        json!({"program":"python","protocol_version":2,"mode":"resident","resident_entry_file":"entry.py","idle_exit_seconds":601}),
    ] {
        let mut value = manifest();
        value["runtime"] = runtime;
        assert!(
            preview(&value).is_err(),
            "invalid transport declaration must reject"
        );
    }
    let mut resident = manifest();
    resident["runtime"] = json!({"program":"python","protocol_version":2,"mode":"resident","resident_entry_file":"entry.py","idle_exit_seconds":120});
    assert!(
        preview(&resident).is_ok(),
        "explicit bounded resident declaration is parseable, not executable approval"
    );
}

#[tokio::test]
async fn expansion_resident_declaration_never_falls_back_to_one_shot_broker() {
    use nxtools_extensions::{
        Causality, ExtensionRuntime, FeatureAdapter, HostContext, HostFuture, InvokeRequest,
        PackageRegistry, RuntimeError, RuntimeIdentity,
    };
    struct NoEffects;
    impl FeatureAdapter for NoEffects {
        fn describe(&self) -> Value {
            json!({})
        }
        fn call<'a>(&'a self, _: &'a HostContext, _: &'a str, _: Value) -> HostFuture<'a> {
            panic!("unsupported transport must not dispatch any feature effect");
        }
    }
    for resident in [true] {
        let root = tempfile::tempdir().expect("owned unsupported execution");
        let mut value = manifest();
        value["runtime"]["protocol_version"] = json!(2);
        if resident {
            value["runtime"]["mode"] = json!("resident");
            value["runtime"]["resident_entry_file"] = json!("entry.py");
        }
        let staged = preview(&value)
            .expect("explicit import")
            .stage_into(&root.path().join("packages"))
            .expect("immutable stage");
        let hash = staged.content_hash.clone();
        let interpreter = root.path().join("unexecuted.exe");
        std::fs::write(&interpreter, b"not executable; must never launch")
            .expect("identity fixture");
        let mut registry = PackageRegistry::default();
        registry
            .install(staged, "owned".into(), false)
            .expect("install");
        registry
            .approve(
                "fixture.transport",
                &hash,
                RuntimeIdentity::inspect(&interpreter).expect("synthetic identity"),
            )
            .expect("explicit approval");
        registry
            .set_enabled("fixture.transport", true, &[])
            .expect("enable");
        let runtime = ExtensionRuntime::new(
            registry,
            root.path().join("absent-broker.exe"),
            std::sync::Arc::new(NoEffects),
        );
        let (_cancel, cancellation) = tokio::sync::watch::channel(false);
        let outcome = runtime
            .invoke(InvokeRequest {
                package_id: "fixture.transport".into(),
                handler_id: "run".into(),
                correlation_id: uuid::Uuid::new_v4(),
                generation: 1,
                context: json!({}),
                causality: Causality::root(uuid::Uuid::new_v4()),
                cancellation,
                batches: None,
            })
            .await;
        assert!(matches!(outcome, Err(RuntimeError::UnsupportedTransport)));
        runtime.shutdown();
    }
}

#[test]
fn expansion_blob_required_methods_need_explicit_transport_two_at_formal_import() {
    let mut value = manifest();
    value["requires"] = json!([{"method":"blobs.begin","version":1}]);
    assert!(preview(&value).is_err(), "v1 cannot require v2 blob method");
    value["runtime"]["protocol_version"] = json!(2);
    assert!(
        preview(&value).is_ok(),
        "implemented v2 blob method declaration"
    );
}
