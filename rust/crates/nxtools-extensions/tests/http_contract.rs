use nxtools_extensions::http::{HttpError, HttpRequest, HttpRun, MAX_RUN_IO_BYTES};
use nxtools_extensions::{InvocationIdentity, NetworkDeclaration};
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;
fn identity() -> InvocationIdentity {
    InvocationIdentity {
        run_id: Uuid::new_v4(),
        correlation_id: Uuid::new_v4(),
        generation: 1,
        package_id: "fixture.http".into(),
        handler_id: "run".into(),
        content_hash: "a".repeat(64),
    }
}
#[test]
fn expansion_http_params_strict_objects_methods_headers_bodies_and_limits_before_effect() {
    for value in [
        json!(null),
        json!([]),
        json!(["https://example.test", "GET"]),
        json!({}),
        json!({"url":"https://example.test","method":"CONNECT"}),
        json!({"url":"https://u:p@example.test","method":"GET"}),
        json!({"url":"https://@example.test","method":"GET"}),
        json!({"url":"ftp://example.test","method":"GET"}),
        json!({"url":" https://example.test","method":"GET"}),
        json!({"url":"https://example.test/#fragment","method":"GET"}),
        json!({"url":"http://127.1","method":"GET"}),
        json!({"url":"https://example.test","method":"GET","extra":true}),
    ] {
        assert!(
            HttpRequest::parse(value).is_err(),
            "synthetic invalid request shape"
        );
    }
    for method in ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE"] {
        let request =
            HttpRequest::parse(json!({"url":"https://EXAMPLE.test:443/a?q=1","method":method}))
                .expect("allowed method");
        assert_eq!(request.url.as_str(), "https://example.test/a?q=1");
        assert_eq!(request.timeout, Duration::from_secs(10));
        assert_eq!(request.response_max_bytes, 2 * 1024 * 1024);
        assert_eq!(request.max_redirects, 0);
    }
    for name in [
        "Host",
        "Connection",
        "Transfer-Encoding",
        "Content-Length",
        "Proxy-Authorization",
        "Proxy-Connection",
        "Keep-Alive",
        "TE",
        "Trailer",
        "Upgrade",
        "bad name",
        "bad\r\nname",
    ] {
        assert!(HttpRequest::parse(json!({"url":"https://example.test","method":"GET","headers":[{"name":name,"value":"synthetic"}]})).is_err());
    }
    for headers in [
        json!([{"name":"X-Test","value":"one\r\ntwo"}]),
        json!([{"name":"X-Test","value":"a","unknown":true}]),
        json!([["X-Test", "a"]]),
        json!(
            (0..65)
                .map(|_| json!({"name":"X-Test","value":"a"}))
                .collect::<Vec<_>>()
        ),
        json!([{"name":"X-Test","value":"a".repeat(16384)}]),
    ] {
        assert!(
            HttpRequest::parse(
                json!({"url":"https://example.test","method":"GET","headers":headers})
            )
            .is_err()
        );
    }
    for (field, value) in [
        ("timeout_ms", json!(0)),
        ("timeout_ms", json!(30001)),
        ("timeout_ms", json!(null)),
        ("response_max_bytes", json!(0)),
        ("response_max_bytes", json!(8388609)),
        ("max_redirects", json!(4)),
        ("body_text", json!("a".repeat(2 * 1024 * 1024 + 1))),
    ] {
        let mut request = json!({"url":"https://example.test","method":"POST"});
        request[field] = value;
        assert!(
            HttpRequest::parse(request).is_err(),
            "synthetic budget {field}"
        );
    }
    assert!(HttpRequest::parse(json!({"url":"https://example.test","method":"POST","body_text":"a","body_blob_id":Uuid::new_v4()})).is_err());
    let request=HttpRequest::parse(json!({"url":"https://example.test","method":"POST","headers":[{"name":"Authorization","value":"synthetic-token"},{"name":"Cookie","value":"synthetic=explicit"}],"body_text":"synthetic","timeout_ms":30000,"response_max_bytes":8388608,"max_redirects":3})).expect("explicit bounded author parameters");
    assert_eq!(request.headers.len(), 2);
    assert_eq!(request.body_text.as_deref(), Some("synthetic"));
}
#[test]
fn expansion_http_params_public_json_rejects_duplicates_and_null_optional_fields() {
    for bytes in [
        r#"{"url":"https://example.test","url":"https://other.test","method":"GET"}"#,
        r#"{"url":"https://example.test","method":"GET","headers":null}"#,
        r#"{"url":"https://example.test","method":"POST","body_text":null}"#,
    ] {
        assert!(serde_json::from_str::<HttpRequest>(bytes).is_err());
    }
    let request: HttpRequest =
        serde_json::from_str(r#"{"url":"https://example.test","method":"GET"}"#)
            .expect("strict DTO");
    assert_eq!(request.method, "GET");
}
#[test]
fn expansion_http_run_exact_identity_origin_two_concurrent_admissions_budget_and_original_deadline()
{
    let id = identity();
    let origins: NetworkDeclaration =
        serde_json::from_value(json!({"origins":["https://example.test"]})).expect("exact origin");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let run = HttpRun::new(id.clone(), origins, deadline);
    let request = HttpRequest::parse(json!({"url":"https://example.test/path","method":"GET"}))
        .expect("request");
    assert_eq!(
        run.admit(&identity(), &request).err(),
        Some(HttpError::Stale)
    );
    let foreign = HttpRequest::parse(json!({"url":"https://other.test/path","method":"GET"}))
        .expect("other URL");
    assert_eq!(
        run.admit(&id, &foreign).err(),
        Some(HttpError::OriginDenied)
    );
    let first = run.admit(&id, &request).expect("first concurrent request");
    let second = run.admit(&id, &request).expect("second concurrent request");
    assert_eq!(first.deadline(), deadline);
    assert_eq!(second.deadline(), deadline);
    assert_eq!(
        run.admit(&id, &request).err(),
        Some(HttpError::ResourceLimit)
    );
    drop(first);
    let later = run
        .admit(&id, &request)
        .expect("finished request releases concurrency, no lifetime request cap");
    assert_eq!(later.deadline(), deadline);
    run.charge(MAX_RUN_IO_BYTES).expect("actual decoded budget");
    assert_eq!(run.charge(1), Err(HttpError::ResourceLimit));
    let expired = HttpRun::new(
        id.clone(),
        NetworkDeclaration::default(),
        tokio::time::Instant::now() - Duration::from_millis(1),
    );
    assert_eq!(expired.admit(&id, &request).err(), Some(HttpError::Timeout));
}
