use nxtools_finder::{ApplicationSource, FinderCatalog, SearchResultAction, calculator_answer};
use tempfile::tempdir;

#[test]
fn calculator_evaluates_a_parenthesized_expression() {
    let answer = calculator_answer("=12 * (4 + 3)").expect("valid expression");

    assert_eq!(answer.expression, "12 * (4 + 3)");
    assert_eq!(answer.value, "84");
    assert_eq!(answer.title(), "12 * (4 + 3) = 84");
}

#[test]
fn calculator_supports_bounded_scientific_functions() {
    let answer = calculator_answer("=sqrt(81) + round(pi, 2)").expect("valid function expression");

    assert_eq!(answer.value, "12.14");
}

#[test]
fn calculator_supports_explicit_base_conversion() {
    let answer = calculator_answer("=255 to hex").expect("valid base conversion");

    assert_eq!(answer.value, "0xFF");
    assert_eq!(answer.title(), "255 to hex = 0xFF");
    let exact_large =
        calculator_answer("=0x20000000000001 to dec").expect("large exact integer conversion");
    assert_eq!(exact_large.value, "9007199254740993");
    assert_eq!(
        exact_large
            .outputs
            .iter()
            .find(|output| output.representation.label() == "中文金额")
            .map(|output| output.value.as_str()),
        Some("玖仟零柒兆壹仟玖佰玖拾贰亿伍仟肆佰柒拾肆万零玖佰玖拾叁元")
    );
    let octal = calculator_answer("=255 to oct").expect("valid octal conversion");
    assert_eq!(octal.value, "0o377");
    assert_eq!(
        octal
            .outputs
            .iter()
            .map(|output| (output.value.as_str(), output.representation.label()))
            .collect::<Vec<_>>(),
        vec![
            ("0o377", "八进制"),
            ("255", "计算结果"),
            ("0xFF", "十六进制"),
            ("11111111", "二进制"),
            ("二百五十五", "中文数字"),
            ("贰佰伍拾伍元", "中文金额"),
        ]
    );
    assert_eq!(
        calculator_answer("=0o377 to dec")
            .expect("valid reverse octal conversion")
            .value,
        "255"
    );
}

#[test]
fn calculator_supports_prefixed_and_scientific_numbers() {
    let answer = calculator_answer("=0x10 + 0b10 + 0o10 + 1e1").expect("valid numeric forms");

    assert_eq!(answer.value, "36");
}

#[test]
fn calculator_formats_floating_point_results_for_people() {
    assert_eq!(
        calculator_answer("=0.1 + 0.2")
            .expect("decimal expression")
            .value,
        "0.3"
    );
    assert_eq!(
        calculator_answer("=1 / 1000000000000")
            .expect("small result")
            .value,
        "1e-12"
    );
    assert_eq!(
        calculator_answer("=120.00000000004 + 0")
            .expect("rounded integer")
            .value,
        "120"
    );
}

#[test]
fn calculator_is_a_typed_instant_search_result() {
    let directory = tempdir().expect("temporary catalog");
    let catalog = FinderCatalog::open(directory.path().join("history.sqlite3"), &[])
        .expect("open empty catalog");

    let results = catalog.search("=132 + 23", 10).expect("search calculator");

    assert_eq!(results.len(), 5);
    assert_eq!(
        results
            .iter()
            .map(|result| (
                result.entry.name.as_str(),
                result.subtitle.as_deref(),
                result.copy_text()
            ))
            .collect::<Vec<_>>(),
        vec![
            ("155", Some("计算结果"), Some("155")),
            ("0x9B", Some("十六进制"), Some("0x9B")),
            ("10011011", Some("二进制"), Some("10011011")),
            ("一百五十五", Some("中文数字"), Some("一百五十五")),
            ("壹佰伍拾伍元", Some("中文金额"), Some("壹佰伍拾伍元")),
        ]
    );
    assert!(
        results
            .iter()
            .all(|result| result.entry.source == ApplicationSource::Calculator)
    );
    assert!(results.iter().all(|result| result.entry.target.is_empty()));
    assert_eq!(
        results[0].action,
        SearchResultAction::CopyText { text: "155".into() }
    );
}

#[test]
fn calculator_search_result_preserves_the_typed_expression() {
    let directory = tempdir().expect("temporary catalog");
    let catalog = FinderCatalog::open(directory.path().join("history.sqlite3"), &[])
        .expect("open empty catalog");

    let results = catalog
        .search("=0xFF to dec", 10)
        .expect("search calculator");

    assert!(results[0].entry.id.contains("0xff to dec"));
    assert_eq!(results[0].entry.name, "255");
}

#[test]
fn calculator_outputs_only_representations_that_are_safe_for_the_value() {
    let directory = tempdir().expect("temporary catalog");
    let catalog = FinderCatalog::open(directory.path().join("history.sqlite3"), &[])
        .expect("open empty catalog");

    let fractional = catalog
        .search("=1 / 2", 10)
        .expect("search fractional calculator");
    assert_eq!(
        fractional
            .iter()
            .map(|result| result.subtitle.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("计算结果"), Some("中文金额")]
    );
    assert_eq!(fractional[1].entry.name, "伍角");

    let negative = catalog
        .search("=0 - 42", 10)
        .expect("search negative calculator");
    assert_eq!(
        negative
            .iter()
            .map(|result| result.entry.name.as_str())
            .collect::<Vec<_>>(),
        vec!["-42", "负四十二", "负肆拾贰元"]
    );

    let zero = catalog.search("=1 - 1", 10).expect("search zero result");
    assert_eq!(
        zero.iter()
            .map(|result| (result.entry.name.as_str(), result.subtitle.as_deref()))
            .collect::<Vec<_>>(),
        vec![
            ("0", Some("计算结果")),
            ("0x0", Some("十六进制")),
            ("0", Some("二进制")),
            ("零", Some("中文数字")),
            ("零元", Some("中文金额")),
        ]
    );
}

#[test]
fn calculator_formats_chinese_zero_bridges_and_currency_fractions() {
    let directory = tempdir().expect("temporary catalog");
    let catalog = FinderCatalog::open(directory.path().join("history.sqlite3"), &[])
        .expect("open empty catalog");

    let integer = catalog
        .search("=10001 + 0", 10)
        .expect("search integer with internal zero");
    assert_eq!(integer[3].entry.name, "一万零一");
    assert_eq!(integer[4].entry.name, "壹万零壹元");

    let amount = catalog
        .search("=1.05 + 0", 10)
        .expect("search fractional amount");
    assert_eq!(
        amount
            .iter()
            .find(|result| result.subtitle.as_deref() == Some("中文金额"))
            .map(|result| result.entry.name.as_str()),
        Some("壹元零伍分")
    );
}

#[test]
fn calculator_rejects_non_expressions_and_unbounded_input() {
    assert!(calculator_answer("calculator").is_none());
    assert!(calculator_answer("12 * (4 + 3)").is_none());
    assert!(calculator_answer("sqrt(81)").is_none());
    assert!(calculator_answer("255 to hex").is_none());
    assert!(calculator_answer("=").is_none());
    assert!(calculator_answer("42").is_none());
    assert!(calculator_answer("=42").is_none());
    assert!(calculator_answer("=0x10").is_none());
    assert!(calculator_answer("=0b10").is_none());
    assert!(calculator_answer("=0o10").is_none());
    assert!(calculator_answer("=-0x10").is_none());
    assert!(calculator_answer("=+0b10").is_none());
    assert!(calculator_answer("=1 +").is_none());
    assert!(calculator_answer("=1 / 0").is_none());
    assert!(calculator_answer("=3.9 to hex").is_none());
    assert!(calculator_answer("=9223372036854775808 to hex").is_none());
    assert!(calculator_answer(&format!("={}", "(".repeat(40))).is_none());
    assert!(calculator_answer(&format!("={}1{}", "(".repeat(40), ")".repeat(40))).is_none());
    assert!(calculator_answer(&format!("={}", "1+".repeat(130))).is_none());
}
