use nxtools_finder::{
    ApplicationSource, ExternalSearchEntry, FileQuery, FinderCatalog, query_requests_file_index,
};

#[test]
fn browser_urls_are_not_quick_launch_results() {
    let directory = tempfile::tempdir().expect("temporary finder catalog");
    let catalog = FinderCatalog::open(directory.path().join("usage.db"), &[])
        .expect("open empty application catalog");

    for query in ["https://example.com/docs", "example.com/docs"] {
        assert!(
            catalog
                .search(query, 20)
                .expect("search URL-shaped query")
                .is_empty(),
            "URL-shaped queries must not create Quick Launch results"
        );
        assert!(
            query_requests_file_index(query),
            "URL-shaped queries must continue through the ordinary file-search path"
        );
    }
}
#[test]
fn multiple_fuzzy_terms_all_match_regardless_of_term_order() {
    let query = FileQuery::parse("report final");

    assert!(
        query
            .score("final-Q3-report.docx", r"D:\Documents\final-Q3-report.docx")
            .is_some()
    );
    assert!(
        query
            .score("report-draft.docx", r"D:\Documents\report-draft.docx")
            .is_none()
    );
}

#[test]
fn excluded_terms_remove_an_otherwise_matching_result() {
    let query = FileQuery::parse("report !temp");

    assert!(
        query
            .score("report-final.docx", r"D:\Documents\report-final.docx")
            .is_some()
    );
    assert!(
        query
            .score("report-temp.docx", r"D:\Documents\report-temp.docx")
            .is_none()
    );
}

#[test]
fn apostrophe_uses_exact_substring_matching_instead_of_fuzzy_matching() {
    let query = FileQuery::parse("'report");

    assert!(
        query
            .score("Q3-report-final.docx", r"D:\Q3-report-final.docx")
            .is_some()
    );
    assert!(
        query
            .score("r-e-p-o-r-t.docx", r"D:\r-e-p-o-r-t.docx")
            .is_none()
    );
}

#[test]
fn caret_requires_the_file_name_to_start_with_the_term() {
    let query = FileQuery::parse("^IMG");

    assert!(query.score("IMG_2024.jpg", r"D:\IMG_2024.jpg").is_some());
    assert!(
        query
            .score("backup_IMG_2024.jpg", r"D:\backup_IMG_2024.jpg")
            .is_none()
    );
}

#[test]
fn dollar_requires_the_file_name_to_end_with_the_term() {
    let query = FileQuery::parse(".pdf$");

    assert!(query.score("report.pdf", r"D:\report.pdf").is_some());
    assert!(
        query
            .score("report.pdf.bak", r"D:\report.pdf.bak")
            .is_none()
    );
}

#[test]
fn pipe_matches_either_query_branch() {
    let query = FileQuery::parse("report | summary");

    assert!(query.score("report.docx", r"D:\report.docx").is_some());
    assert!(query.score("summary.docx", r"D:\summary.docx").is_some());
    assert!(query.score("budget.docx", r"D:\budget.docx").is_none());
}

#[test]
fn leading_drive_selector_limits_results_to_that_volume() {
    let query = FileQuery::parse("d: report");

    assert!(query.score("report.docx", r"D:\Work\report.docx").is_some());
    assert!(query.score("report.docx", r"C:\Work\report.docx").is_none());
}

#[test]
fn a_query_with_separators_matches_the_normalized_full_path() {
    let query = FileQuery::parse(r"D:/Projects/SwiftList");

    assert!(query.score("SwiftList", r"D:\Projects\SwiftList").is_some());
    assert!(query.score("SwiftList", r"D:\Archive\SwiftList").is_none());
}

#[test]
fn a_trailing_separator_lists_only_the_exact_folders_direct_children() {
    let query = FileQuery::parse(r"D:\Projects\");

    assert!(query.score("SwiftList", r"D:\Projects\SwiftList").is_some());
    assert!(
        query
            .score("README.md", r"D:\Projects\SwiftList\README.md")
            .is_none()
    );
}

#[test]
fn unified_catalog_applies_the_same_query_to_external_file_results() {
    let directory = tempfile::tempdir().expect("temporary finder catalog");
    let catalog = FinderCatalog::open(directory.path().join("usage.db"), &[])
        .expect("open empty application catalog");
    let external = [
        ExternalSearchEntry {
            id: "2026".into(),
            name: "Budget 2026.xlsx".into(),
            target: r"D:\Work\Budget 2026.xlsx".into(),
            source: ApplicationSource::IndexedFile,
        },
        ExternalSearchEntry {
            id: "2025".into(),
            name: "Budget 2025.xlsx".into(),
            target: r"D:\Work\Budget 2025.xlsx".into(),
            source: ApplicationSource::IndexedFile,
        },
    ];

    let results = catalog
        .search_with_external("d: ^Budget !2025", &external, 20)
        .expect("unified file search");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entry.id, "2026");
}

#[test]
fn quoted_text_is_one_exact_phrase() {
    let query = FileQuery::parse("\"final report\"");

    assert!(
        query
            .score("Q3 final report.docx", r"D:\\Q3 final report.docx")
            .is_some()
    );
    assert!(
        query
            .score("final Q3 report.docx", r"D:\\final Q3 report.docx")
            .is_none()
    );
}

#[test]
fn single_quoted_phrase_requires_word_boundaries() {
    let query = FileQuery::parse("'final report'");

    assert!(
        query
            .score("Q3 final report.docx", r"D:\Q3 final report.docx")
            .is_some()
    );
    assert!(
        query
            .score("Q3 final reporter.docx", r"D:\Q3 final reporter.docx")
            .is_none()
    );
}

#[test]
fn parent_selector_limits_a_name_match_to_the_requested_parent_path() {
    let query = FileQuery::parse("report ::Projects");

    assert!(
        query
            .score("report.docx", r"D:\\Projects\\report.docx")
            .is_some()
    );
    assert!(
        query
            .score("report.docx", r"D:\\Archive\\report.docx")
            .is_none()
    );
}

#[test]
fn parent_selectors_are_fuzzy_and_support_multiple_comma_filters() {
    let query = FileQuery::parse("report ::2024,:fnl");

    assert!(
        query
            .score("report.docx", r"D:\2024\final\report.docx")
            .is_some()
    );
    assert!(
        query
            .score("report.docx", r"D:\2023\final\report.docx")
            .is_none()
    );
}

#[test]
fn leading_star_marks_the_query_as_bypassing_user_exclusions() {
    let query = FileQuery::parse("*report");

    assert!(query.bypasses_exclusions());
    assert!(
        query
            .score("report.docx", r"D:\\Build\\report.docx")
            .is_some()
    );
}
