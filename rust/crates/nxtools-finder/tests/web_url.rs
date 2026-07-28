use nxtools_finder::{ApplicationSource, ExternalSearchEntry, FinderCatalog, SearchResultAction};
use tempfile::{TempDir, tempdir};

fn empty_catalog() -> (TempDir, FinderCatalog) {
    let directory = tempdir().expect("temporary catalog");
    let catalog = FinderCatalog::open(directory.path().join("history.sqlite3"), &[])
        .expect("open empty catalog");
    (directory, catalog)
}

#[test]
fn explicit_https_url_is_one_typed_browser_result() {
    let (_directory, catalog) = empty_catalog();

    let results = catalog
        .search("  https://example.com/docs?q=rust#start  ", 10)
        .expect("search URL");

    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(result.entry.source, ApplicationSource::WebUrl);
    assert_eq!(result.entry.name, "https://example.com/docs?q=rust#start");
    assert_eq!(result.entry.target, result.entry.name);
    assert_eq!(result.subtitle.as_deref(), Some("用浏览器打开"));
    assert_eq!(result.action, SearchResultAction::OpenTarget);
}

#[test]
fn bare_domain_is_normalized_to_https() {
    let (_directory, catalog) = empty_catalog();

    let results = catalog
        .search("example.com/docs?q=rust", 10)
        .expect("search bare domain");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entry.source, ApplicationSource::WebUrl);
    assert_eq!(results[0].entry.target, "https://example.com/docs?q=rust");
}

#[test]
fn local_and_ip_http_targets_are_supported() {
    let (_directory, catalog) = empty_catalog();

    let localhost = catalog
        .search("http://localhost:8080/status", 10)
        .expect("search localhost URL");
    let ipv4 = catalog
        .search("192.168.1.1:8080/status", 10)
        .expect("search bare IPv4 URL");
    let ipv6 = catalog
        .search("[::1]:8080/status", 10)
        .expect("search bare IPv6 URL");

    assert_eq!(localhost[0].entry.target, "http://localhost:8080/status");
    assert_eq!(ipv4[0].entry.target, "https://192.168.1.1:8080/status");
    assert_eq!(ipv6[0].entry.target, "https://[::1]:8080/status");
}

#[test]
fn url_lookalikes_do_not_create_browser_results() {
    let (_directory, catalog) = empty_catalog();

    for query in [
        "example",
        "a.b",
        "bad_.com",
        "-bad.com",
        "https://bad_.com",
        "https://-bad.com",
        "foo bar.com",
        "user@example.com",
        "https://user:secret@example.com",
        "https://@example.com",
        "ftp://example.com",
        "1.2",
        "192.168.1",
        r"C:\Work\example.com",
    ] {
        assert!(
            catalog
                .search(query, 10)
                .expect("search lookalike")
                .is_empty(),
            "unexpected URL result for {query}"
        );
    }
    let label = "a".repeat(63);
    let oversized_domain = format!("{label}.{label}.{label}.{label}.com");
    assert!(
        catalog
            .search(&oversized_domain, 10)
            .expect("search oversized domain")
            .is_empty()
    );
}

#[test]
fn browser_result_ranks_before_ordinary_file_matches() {
    let (_directory, catalog) = empty_catalog();
    let external = [ExternalSearchEntry {
        id: "file:example".into(),
        name: "example.com".into(),
        target: r"C:\Archive\example.com.txt".into(),
        source: ApplicationSource::IndexedFile,
    }];

    let results = catalog
        .search_with_external("example.com", &external, 10)
        .expect("search URL with file candidate");

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].entry.source, ApplicationSource::WebUrl);
    assert_eq!(results[1].entry.source, ApplicationSource::IndexedFile);
}
