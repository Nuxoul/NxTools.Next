use nxtools_extensions::{ImportPreview, NetworkDeclaration, NetworkOrigin, PackageError};
use serde_json::{Value, json};

const CODE: &[u8] = b"synthetic entry; never executed\n";
const LEGACY_HASH: &str = "23116cf22f41756531b1d48c90c9827ead6197b5776d6d1a2eb087bc9216456e";
fn manifest() -> Value {
    json!({"schema_version":1,"id":"fixture.transport","name":"Transport","version":"1",
        "runtime":{"program":"python"},"files":["entry.py"],
        "handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":"entry.py"}}]})
}
fn import_bytes(bytes: &[u8]) -> Result<ImportPreview, PackageError> {
    let root = tempfile::tempdir().expect("owned origin declaration");
    std::fs::write(root.path().join("extension.json"), bytes).expect("manifest");
    std::fs::write(root.path().join("entry.py"), CODE).expect("unexecuted code");
    ImportPreview::from_directory(root.path())
}
fn import(value: &Value) -> Result<ImportPreview, PackageError> {
    import_bytes(&serde_json::to_vec(value).expect("JSON"))
}
#[test]
fn expansion_http_origins_omitted_or_empty_preserve_literal_legacy_hash() {
    let legacy = import(&manifest()).expect("legacy");
    assert_eq!(legacy.content_hash, LEGACY_HASH);
    let bytes = serde_json::to_vec(&legacy.manifest).expect("canonical");
    for network in [json!({}), json!({"origins":[]})] {
        let mut value = manifest();
        value["network"] = network;
        let empty = import(&value).expect("explicit empty declaration");
        assert_eq!(empty.content_hash, LEGACY_HASH);
        assert_eq!(
            serde_json::to_vec(&empty.manifest).expect("unchanged canonical"),
            bytes
        );
    }
}
#[test]
fn expansion_http_origins_formal_import_normalizes_exact_origin_and_binds_approval_hash() {
    let mut value = manifest();
    value["network"] = json!({"origins":["HTTPS://ExAmPlE.test:443","http://localhost:80/","http://127.0.0.1:8080","https://[::1]:8443","https://bücher.test","http://192.168.1.9:3000"]});
    let imported = import(&value).expect("exact public/local/private declarations");
    let canonical = serde_json::to_value(&imported.manifest).expect("canonical");
    assert_eq!(
        canonical["network"]["origins"],
        json!([
            "https://example.test",
            "http://localhost",
            "http://127.0.0.1:8080",
            "https://[::1]:8443",
            "https://xn--bcher-kva.test",
            "http://192.168.1.9:3000"
        ])
    );
    assert_ne!(
        imported.content_hash, LEGACY_HASH,
        "destination authority is content/trust bound"
    );
    let reparsed = import(&canonical).expect("canonical reopen");
    assert_eq!(reparsed.content_hash, imported.content_hash);
    value["network"]["origins"][0] = json!("https://other.test");
    assert_ne!(
        import(&value).expect("new declaration").content_hash,
        imported.content_hash
    );
}
#[test]
fn expansion_http_origins_reject_unknown_duplicate_ambiguous_and_overbudget_declarations() {
    for (case, network) in [
        json!(null),
        json!([]),
        json!([[]]),
        json!([["https://example.test"]]),
        json!({"origins":null}),
        json!({"origins":"https://example.test"}),
        json!({"origins":[7]}),
        json!({"origins":["https://example.test"],"wildcard":true}),
        json!({"origins":["https://EXAMPLE.test:443","https://example.test/"]}),
        json!({"origins":(0..33).map(|i|format!("https://host-{i}.test")).collect::<Vec<_>>()}),
    ]
    .into_iter()
    .enumerate()
    {
        let mut value = manifest();
        value["network"] = network;
        assert!(
            import(&value).is_err(),
            "strict bounded declaration before any effect, synthetic case {case}"
        );
    }
    for origin in [
        "",
        "https://*.example.test",
        "http://192.168.0.0/16",
        "https://example.test/path",
        "https://example.test/?",
        "https://example.test/#",
        "https://u:p@example.test",
        "https://@example.test",
        "https://example.test:0",
        "ftp://example.test",
        "https://example.test\\path",
        " https://example.test",
        "https://example.test\n",
        "http://127.1",
        "http://0x7f000001",
        "http://0177.0.0.1",
        "http://[fe80::1%25eth0]",
    ] {
        let mut value = manifest();
        value["network"] = json!({"origins":[origin]});
        assert!(
            import(&value).is_err(),
            "not an unambiguous exact origin: {origin:?}"
        );
    }
    let mut value = manifest();
    value["network"] = json!({"origins":[format!("https://{}", "a".repeat(8193))]});
    assert!(import(&value).is_err());
    let mut value = manifest();
    value["network"] =
        json!({"origins":(0..32).map(|i|format!("https://host-{i}.test")).collect::<Vec<_>>()});
    assert!(import(&value).is_ok(), "actual32 declaration boundary");
    let bytes = serde_json::to_vec(&manifest()).expect("base");
    let base = std::str::from_utf8(&bytes)
        .expect("UTF8")
        .trim_end_matches('}');
    let duplicate = format!(r#"{base},"network":{{"origins":[],"origins":[]}}}}"#);
    assert!(
        import_bytes(duplicate.as_bytes()).is_err(),
        "duplicate key rejection survives typed conversion"
    );
}

#[test]
fn expansion_http_origins_match_only_exact_normalized_scheme_host_and_effective_port() {
    let cases = [
        (
            "HTTPS://ExAmPlE.test:443/",
            "https://example.test",
            vec![
                "https://EXAMPLE.test/a?q=1#part",
                "https://example.test:443/",
            ],
            vec![
                "http://example.test/",
                "https://example.test:444/",
                "https://sub.example.test/",
                "https://example.test.evil.test/",
                "https://example.test./",
            ],
        ),
        (
            "https://bücher.test",
            "https://xn--bcher-kva.test",
            vec!["https://bücher.test/a", "https://xn--bcher-kva.test:443/"],
            vec!["https://other.test/", "http://bücher.test/"],
        ),
        (
            "http://[0:0:0:0:0:0:0:1]:80/",
            "http://[::1]",
            vec!["http://[::1]/a", "http://[0:0:0:0:0:0:0:1]:80/"],
            vec![
                "http://[::1]:8080/",
                "http://[::2]/",
                "http://localhost/",
                "http://127.0.0.1/",
            ],
        ),
        (
            "http://127.0.0.1:8080",
            "http://127.0.0.1:8080",
            vec!["http://127.0.0.1:8080/a"],
            vec![
                "http://127.0.0.1/",
                "http://127.0.0.2:8080/",
                "http://localhost:8080/",
            ],
        ),
    ];
    for (input, canonical, allowed, rejected) in cases {
        let origin = NetworkOrigin::parse(input).expect("valid exact origin");
        assert_eq!(origin.as_str(), canonical);
        let declaration = NetworkDeclaration {
            origins: vec![origin.clone()],
        };
        for candidate in allowed {
            let url = url::Url::parse(candidate).expect("synthetic URL");
            assert!(
                origin.matches_url(&url),
                "synthetic exact match: {candidate}"
            );
            assert!(declaration.allows_url(&url));
        }
        for candidate in rejected {
            let url = url::Url::parse(candidate).expect("synthetic URL");
            assert!(
                !origin.matches_url(&url),
                "synthetic distinct origin: {candidate}"
            );
            assert!(!declaration.allows_url(&url));
        }
    }
    let declaration = NetworkDeclaration::default();
    assert!(!declaration.allows_url(&url::Url::parse("https://example.test/").expect("URL")));
}

#[test]
fn expansion_http_origins_public_deserializer_requires_object_and_rejects_duplicate_keys() {
    for bytes in [
        r#"[]"#,
        r#"[[]]"#,
        r#"[["https://example.test"]]"#,
        r#"null"#,
        r#"true"#,
        r#""https://example.test""#,
        r#"{"origins":[],"origins":[]}"#,
        r#"{"origins":[],"unknown":false}"#,
    ] {
        assert!(
            serde_json::from_str::<NetworkDeclaration>(bytes).is_err(),
            "synthetic malformed declaration: {bytes}"
        );
    }
    assert!(
        serde_json::from_str::<NetworkDeclaration>(r#"{}"#)
            .expect("empty object")
            .is_empty()
    );
}

#[test]
fn expansion_http_origins_declaration_does_not_enable_an_unimplemented_http_method() {
    assert!(!nxtools_extensions::supports_method("http.request", 1));
    let mut value = manifest();
    value["network"] = json!({"origins":["https://example.test"]});
    value["requires"] = json!([{"method":"http.request","version":1}]);
    assert!(
        import(&value).is_err(),
        "origin declaration cannot advertise unavailable HTTP"
    );
}
