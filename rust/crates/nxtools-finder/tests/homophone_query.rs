use nxtools_finder::{ApplicationSource, ExternalSearchEntry, FinderCatalog};

fn entries(names: &[&str]) -> Vec<ExternalSearchEntry> {
    names
        .iter()
        .map(|name| ExternalSearchEntry {
            id: (*name).into(),
            name: (*name).into(),
            target: format!(r"D:\Fixture\{name}"),
            source: ApplicationSource::IndexedFile,
        })
        .collect()
}
#[test]
fn w2_no_result_homophones_are_labeled_and_never_mix_with_normal_results() {
    let root = tempfile::tempdir().expect("synthetic catalog");
    let catalog = FinderCatalog::open(root.path().join("usage.db"), &[]).expect("catalog");
    let results = catalog
        .search_with_external("年度报高", &entries(&["年度报告.docx", "其他.txt"]), 10)
        .expect("search");
    assert_eq!(
        results.len(),
        1,
        "no-result fallback must find the homophone"
    );
    assert_eq!(results[0].entry.name, "年度报告.docx");
    assert!(
        results[0]
            .subtitle
            .as_deref()
            .is_some_and(|label| label.contains("同音近似"))
    );
    let normal = catalog
        .search_with_external("年度报高", &entries(&["年度报高.txt", "年度报告.docx"]), 10)
        .expect("normal");
    assert_eq!(normal.len(), 1);
    assert_eq!(normal[0].entry.name, "年度报高.txt");
    assert!(normal[0].subtitle.is_none());
}
#[test]
fn w2_explicit_and_negative_terms_are_not_rewritten() {
    let root = tempfile::tempdir().expect("synthetic catalog");
    let catalog = FinderCatalog::open(root.path().join("usage.db"), &[]).expect("catalog");
    for query in [
        "'年度报高",
        "\"年度报高\"",
        "^年度报高",
        "年度报高$",
        "年度报高 !报告",
    ] {
        assert!(
            catalog
                .search_with_external(query, &entries(&["年度报告.docx"]), 10)
                .expect("search")
                .is_empty(),
            "{query}"
        );
    }
}

#[test]
fn w2_variants_are_bounded_and_preserve_scopes_and_highlights() {
    use nxtools_finder::FileQuery;
    let variants = FileQuery::parse("行行行行行行").homophone_variants();
    assert!(!variants.is_empty() && variants.len() <= 8);
    assert!(
        FileQuery::parse(&"行".repeat(40))
            .homophone_variants()
            .is_empty()
    );
    assert!(FileQuery::parse("report").homophone_variants().is_empty());
    assert!(
        FileQuery::parse(r"D:\年度报高")
            .homophone_variants()
            .is_empty()
    );
    let scoped = FileQuery::parse("d: 年度报高 !草稿 ::Work").homophone_variants();
    assert!(scoped.iter().any(|query| {
        query
            .score("年度报告.docx", r"D:\Work\年度报告.docx")
            .is_some()
    }));
    assert!(scoped.iter().all(|query| {
        query
            .score("年度报告草稿.docx", r"D:\Work\年度报告草稿.docx")
            .is_none()
    }));
    assert!(scoped.iter().all(|query| {
        query
            .score("年度报告.docx", r"C:\Work\年度报告.docx")
            .is_none()
    }));
    assert_eq!(
        FileQuery::parse("年度报高").homophone_match_ranges("年度报告.docx"),
        vec![0..12]
    );
}

#[test]
fn w3_retained_directory_uses_original_rank_without_directory_bonus() {
    let root = tempfile::tempdir().expect("synthetic catalog");
    let catalog = FinderCatalog::open(root.path().join("usage.db"), &[]).expect("catalog");
    let mut external = entries(&["report"]);
    external[0].target = r"D:\obj\report".into();
    external.push(ExternalSearchEntry {
        id: "folder".into(),
        name: "report".into(),
        target: r"D:\Work\report".into(),
        source: ApplicationSource::IndexedDirectory,
    });
    let results = catalog
        .search_with_external("report", &external, 1)
        .expect("rank");
    assert_eq!(
        results[0].entry.id, "folder",
        "existing generated-file penalty ranks retained useful folder first"
    );
}
