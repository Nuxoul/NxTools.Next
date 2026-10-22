use nxtools_finder::{ApplicationSource, ExternalSearchEntry, FileQuery, FinderCatalog};

#[test]
fn w1_full_mixed_and_polyphonic_names_match_through_file_query() {
    for (name, terms) in [
        (
            "年度报告.docx",
            vec!["baogao", "baog", "bgao", "bg", "niandubaogao"],
        ),
        ("银行卡.txt", vec!["yinhang", "yhk", "yinxing"]),
        ("重庆", vec!["chongqing", "zhongqing", "cq"]),
        ("V2报告", vec!["v2baogao", "v2bg"]),
        ("绿色", vec!["lvse", "luse"]),
    ] {
        for term in terms {
            assert!(
                FileQuery::parse(term).score_name(name).is_some(),
                "{term}: {name}"
            );
        }
    }
}

#[test]
fn w1_external_merge_uses_shared_pinyin_without_changing_literal_rank() {
    let directory = tempfile::tempdir().expect("temporary catalog");
    let catalog = FinderCatalog::open(directory.path().join("usage.db"), &[]).expect("catalog");
    let entries =
        ["报告.txt", "年度报告.docx", "baogao.txt", "其他.txt"].map(|name| ExternalSearchEntry {
            id: name.into(),
            name: name.into(),
            target: format!(r"D:\Fixture\{name}"),
            source: ApplicationSource::IndexedFile,
        });
    let results = catalog
        .search_with_external("baogao", &entries, 20)
        .expect("search");
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].entry.name, "baogao.txt");
    assert!(results.iter().any(|item| item.entry.name == "报告.txt"));
}

#[test]
fn w1_explicit_operators_exclusions_and_scopes_remain_literal() {
    for term in ["'baogao", "\"baogao\"", "^baogao", "baogao$", "'baogao'"] {
        assert!(
            FileQuery::parse(term).score_name("报告").is_none(),
            "{term}"
        );
    }
    assert!(
        FileQuery::parse("baogao !bg")
            .score_name("报告.txt")
            .is_some()
    );
    assert!(
        FileQuery::parse("baogao !报告")
            .score_name("报告.txt")
            .is_none()
    );
    let query = FileQuery::parse("d: baogao ::Work");
    assert!(query.score("报告.txt", r"D:\Work\报告.txt").is_some());
    assert!(query.score("报告.txt", r"C:\Work\报告.txt").is_none());
    assert!(query.score("报告.txt", r"D:\工作\报告.txt").is_none());
    assert!(
        FileQuery::parse(r"D:\baogao")
            .score("报告", r"D:\报告")
            .is_none()
    );
}

#[test]
fn w1_highlights_and_name_masks_share_the_admitting_match() {
    for (name, term, range) in [
        ("年度报告.docx", "baogao", 6..12),
        ("年度报告.docx", "bgao", 6..12),
        ("🙂银行卡.txt", "yinhang", 4..10),
        ("V2报告", "v2bg", 0..8),
    ] {
        let query = FileQuery::parse(term);
        assert!(query.score_name(name).is_some());
        assert!(query.name_mask_may_match(nxtools_finder::search_char_mask(name)));
        assert_eq!(query.match_ranges(name), vec![range]);
    }
    assert_eq!(
        FileQuery::parse("agm").match_ranges("AyuGram.exe"),
        vec![0..1, 3..4, 6..7]
    );
    assert_eq!(
        FileQuery::parse("GRAM").match_ranges("AyuGram.exe"),
        vec![3..7]
    );
    assert!(FileQuery::parse("!bg").match_ranges("报告").is_empty());
}

#[test]
fn w1_noneligible_terms_and_character_boundaries_do_not_guess() {
    for (term, name) in [
        ("baoo", "报告"),
        ("bx", "报告"),
        ("bgd", "报告.docx"),
        ("bg", "报 告"),
        ("bg", "报-告"),
        ("g", "告"),
        ("22", "报告"),
        ("b_g", "报告"),
        ("bg", "〇告"),
    ] {
        assert!(
            FileQuery::parse(term).score_name(name).is_none(),
            "{term}: {name}"
        );
    }
    assert!(
        FileQuery::parse(&"b".repeat(64))
            .score_name(&"报".repeat(64))
            .is_none()
    );
    assert!(
        FileQuery::parse("baogao | notes")
            .score_name("报告")
            .is_some()
    );
    assert!(
        FileQuery::parse("baogao txt")
            .score_name("报告.txt")
            .is_some()
    );
}

#[test]
fn w1_synthetic_query_cost_and_literal_scores_are_observable() {
    let names = (0..2000)
        .map(|index| match index % 4 {
            0 => format!("file-{index:05}.txt"),
            1 => format!("年度报告{index:05}.docx"),
            2 => format!("银行卡{index:05}.txt"),
            _ => format!("重庆资料{index:05}.txt"),
        })
        .collect::<Vec<_>>();
    for (raw, expected_hits) in [
        ("file", 500),
        ("baogao", 500),
        ("bgao", 500),
        ("yinhang", 500),
        ("chongqing", 500),
    ] {
        let query = FileQuery::parse(raw);
        let mut samples = Vec::new();
        for _ in 0..9 {
            let started = std::time::Instant::now();
            let count = names
                .iter()
                .filter(|name| {
                    std::hint::black_box(&query)
                        .score_name(std::hint::black_box(name))
                        .is_some()
                })
                .count();
            samples.push(started.elapsed().as_micros());
            assert_eq!(count, expected_hits);
        }
        samples.sort_unstable();
        println!(
            "W1 synthetic Debug: query={raw}, names={}, p50_us={}, pmax_us={}",
            names.len(),
            samples[4],
            samples[8]
        );
    }
    assert_eq!(
        FileQuery::parse("file").score_name("file-123.txt"),
        Some(5_992)
    );
    assert_eq!(
        FileQuery::parse("agm").score_name("AyuGram.exe"),
        Some(1_994)
    );
    assert_eq!(
        FileQuery::parse("^report").score_name("report.txt"),
        Some(6_500)
    );
    assert_eq!(
        FileQuery::parse("'report").score_name("my-report.txt"),
        Some(4_997)
    );
}
