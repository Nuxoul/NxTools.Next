use super::*;
const NOW: i64 = 1_800_000_000_000;
const NAMES:&[u8]=br#"{"cny":"Chinese Yuan","usd":"US Dollar","cad":"Canadian Dollar","eur":"Euro","krw":"South Korean Won","1inch":"1inch","xau":"Gold"}"#;
const RATES:&[u8]=br#"{"date":"2027-01-15","cny":{"cny":1,"usd":0.14,"cad":0.2,"eur":0.125,"krw":200,"1inch":1.5,"xau":0.00005}}"#;
fn snapshot() -> RateSnapshot {
    parse_pair(NAMES, RATES, NOW).expect("paired snapshot")
}
#[test]
fn currency_catalog_aliases_user_examples_are_recognized_without_prose_extraction() {
    for (text, expected) in [
        ("₩1000", "KRW"),
        ("1000₩", "KRW"),
        ("￦1000", "KRW"),
        ("32刀", "USD"),
        ("32美刀", "USD"),
        ("美刀32", "USD"),
        ("32美元", "USD"),
        ("32美金", "USD"),
        ("100韩元", "KRW"),
        ("₹5", "INR"),
        ("₽5", "RUB"),
        ("HK$5", "HKD"),
        ("CA$5", "CAD"),
    ] {
        let values = parse_amounts(text).unwrap_or_else(|| panic!("currency alias {text}"));
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].currency, expected, "{text}");
    }
    for text in [
        "这把刀32元",
        "The price is 5CAD today",
        "123",
        "5BAD",
        "5CAD blah",
        "NaN USD",
        "1e9 USD",
        "12.34.56USD",
        "5 USD EUR",
        "--5USD",
        "5.USD",
    ] {
        assert!(parse_amounts(text).is_none(), "reject wholecopy {text}");
    }
    let values = parse_amounts("5CAD 2.5cad 32$ €43").expect("original list");
    assert_eq!(
        values
            .iter()
            .map(|amount| (amount.value, amount.currency.as_str()))
            .collect::<Vec<_>>(),
        vec![(5.0, "CAD"), (2.5, "CAD"), (32.0, "USD"), (43.0, "EUR")]
    );
    for text in [
        "$32", "32 USD", "USD 32", "€43", "43eur", "CAD $5", "$5CAD", "JPY ¥5",
    ] {
        assert_eq!(parse_amounts(text).expect(text).len(), 1);
    }
    assert_eq!(
        parse_amounts("JPY ¥5").expect("explicitpriority")[0].currency,
        "JPY"
    );
    for text in [
        "$5 €43",
        "USD 5 CAD 2",
        "CAD5 USD2",
        "5 USD 2CAD",
        "₩1000 32美刀",
    ] {
        assert_eq!(parse_amounts(text).expect(text).len(), 2);
    }
    assert!(parse_amounts(&"5CAD ".repeat(9)).is_none());
    assert!(parse_amounts(&"a".repeat(257)).is_none());
}
#[test]
fn currency_catalog_all_bundled_codes_and_new_runtime_codes_parse_locally() {
    let catalog = bundled_catalog();
    assert!(catalog.len() > 300);
    assert_eq!(catalog["KRW"], "South Korean Won");
    for code in catalog.keys() {
        assert_eq!(
            parse_amounts(&format!("{code} 2")).unwrap_or_else(|| panic!("allcode {code}"))[0]
                .currency,
            *code
        );
    }
    for text in ["1inch 2", "2 1INCH", "USDT2", "2usdt", "XAU 5"] {
        assert!(parse_amounts(text).is_some(), "nonISO/metal {text}");
    }
    let mut updated = catalog.clone();
    updated.insert("NEWCOIN".into(), "New Coin".into());
    assert_eq!(
        parse_amounts_in("5newcoin", &updated).expect("runtimecatalog")[0].currency,
        "NEWCOIN"
    );
    assert!(parse_amounts("5newcoin").is_none());
    assert!(choice_label("USD", "US Dollar").contains("美刀"));
    assert!(choice_label("KRW", "South Korean Won").contains("韩元"));
}
#[test]
fn currency_catalog_pair_validation_cross_conversion_and_names_fallback() {
    let rates = snapshot();
    let amounts = parse_amounts("5CAD 32$").expect("amounts");
    assert_eq!(rates.convert(&amounts[0], "CNY"), Some(25.0));
    assert!((rates.convert(&amounts[1], "EUR").expect("cross") - 28.571428).abs() < 0.001);
    assert_eq!(
        rates.convert(&parse_amounts("₩1000").expect("won")[0], "CNY"),
        Some(5.0)
    );
    assert!(
        rates
            .message(&amounts, "CNY", NOW)
            .expect("notice")
            .contains("25.00 CNY")
    );
    assert!(
        rates
            .message(&amounts, "CNY", NOW + DAY_MS)
            .expect("stale")
            .contains("过期")
    );
    assert!(rates.message(&amounts, "AED", NOW).is_none());
    assert!(!rates.refresh_due(NOW + DAY_MS - 1));
    assert!(rates.refresh_due(NOW + DAY_MS));
    let names = parse_catalog(br#"{"cny":"Chinese Yuan","eurc":""}"#).expect("empty name");
    assert_eq!(names["EURC"], "EURC");
    for invalid in [
        br#"{"cny":"ok","bad code":"bad"}"#.as_slice(),
        br#"{"cny":"ok","usd":"bad\nname"}"#.as_slice(),
        br#"{"usd":"Dollar","eur":"Euro"}"#.as_slice(),
        br#"{"cny":"ok","USD":"Dollar","usd":"duplicate"}"#.as_slice(),
    ] {
        assert!(parse_catalog(invalid).is_err());
    }
    let mut invalid: serde_json::Value = serde_json::from_slice(RATES).expect("rates");
    for patch in [
        serde_json::json!({"date":"2099-01-01"}),
        serde_json::json!({"date":"bad"}),
        serde_json::json!({"usd":{"cny":7}}),
        serde_json::json!({"cny":{"cny":2,"usd":0.14}}),
        serde_json::json!({"cny":{"cny":1,"usd":0}}),
        serde_json::json!({"cny":{"cny":1,"usd":-1}}),
    ] {
        let mut value: serde_json::Value = serde_json::from_slice(RATES).expect("fixture");
        for (key, value2) in patch.as_object().expect("patch") {
            value[key] = value2.clone();
        }
        assert!(parse_pair(NAMES, &serde_json::to_vec(&value).expect("encode"), NOW).is_err());
    }
    invalid["cny"].as_object_mut().expect("map").remove("xau");
    assert_eq!(
        parse_pair(NAMES, &serde_json::to_vec(&invalid).expect("encode"), NOW)
            .expect_err("mismatch"),
        "catalog_rates_mismatch"
    );
    assert!(parse_pair(&vec![b' '; MAX_REPLY + 1], RATES, NOW).is_err());
    assert!(parse_pair(NAMES, &vec![b' '; MAX_REPLY + 1], NOW).is_err());
}
#[test]
fn currency_catalog_atomic_cache_commit_failure_legacy_restart_and_corruption() {
    let dir = tempfile::tempdir().expect("cache");
    let path = dir.path().join("rates.json");
    let snapshot = snapshot();
    save_cache(&path, &snapshot).expect("cache");
    let loaded = load_cache(&path, NOW).expect("restart");
    assert_eq!(loaded.names, snapshot.names);
    assert_eq!(loaded.rates, snapshot.rates);
    let before = std::fs::read(&path).expect("lastgood");
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .expect("denydelete");
        assert_eq!(save_cache(&path, &snapshot), Err("cache_commit"));
        drop(lock);
        assert_eq!(std::fs::read(&path).expect("preserve"), before);
        assert_eq!(
            std::fs::read_dir(dir.path()).expect("tempsclean").count(),
            1
        );
    }
    let legacy = serde_json::json!({"version":1,"provider":"ExchangeRate-API","date":"2027-01-15","fetched_at":NOW,"rates":{"CNY":1,"USD":0.14}});
    std::fs::write(&path, serde_json::to_vec(&legacy).expect("legacy")).expect("oldcache");
    let old = load_cache(&path, NOW).expect("legacyload");
    assert_eq!(old.version, 1);
    assert!(old.refresh_due(NOW));
    assert!(old.catalog().len() > 300);
    assert!(
        old.message(&parse_amounts("$5").expect("amount"), "CNY", NOW)
            .is_some()
    );
    std::fs::write(&path, b"corrupt").expect("badcache");
    assert!(load_cache(&path, NOW).is_none());
    std::fs::write(&path, vec![0; MAX_CACHE + 1]).expect("largecache");
    assert!(load_cache(&path, NOW).is_none());
}
#[test]
fn currency_catalog_real_local_http_pair_fallback_budget_redirect_timeout_and_privacy() {
    use std::io::{Read as _, Write as _};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("localAPI");
    listener.set_nonblocking(true).expect("boundedaccept");
    let address = listener.local_addr().expect("address");
    let server = std::thread::spawn(move || {
        let mut requests = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while requests.len() < 7 && std::time::Instant::now() < deadline {
            let (mut stream, _) = match listener.accept() {
                Ok(value) => value,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(error) => panic!("accept {error}"),
            };
            stream.set_nonblocking(false).expect("blocking");
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .expect("readbudget");
            stream
                .set_write_timeout(Some(Duration::from_secs(1)))
                .expect("writebudget");
            let mut buffer = [0; 4096];
            let size = stream.read(&mut buffer).expect("request");
            let request = String::from_utf8_lossy(&buffer[..size]).to_string();
            let target = request
                .split_whitespace()
                .nth(1)
                .expect("target")
                .to_owned();
            requests.push(request);
            let response = if target.starts_with("/primary/") {
                "HTTP/1.1 503 Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_owned()
            } else if target == "/oversize" {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    MAX_REPLY + 1
                )
            } else if target == "/redirect" {
                "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/unexpected\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()
            } else if target == "/slow" {
                std::thread::sleep(Duration::from_millis(200));
                String::new()
            } else {
                let body = if target.ends_with("/currencies.min.json") {
                    NAMES
                } else {
                    RATES
                };
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    String::from_utf8_lossy(body)
                )
            };
            if !response.is_empty() {
                let _ = stream.write_all(response.as_bytes());
            }
        }
        requests
    });
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(async {
            let client = reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_millis(100))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("client");
            let primary = format!("http://{address}/primary");
            let fallback = format!("http://{address}/fallback");
            let rates = fetch_sources(
                &client,
                [("primary", &primary), ("fallback", &fallback)],
                NOW,
                uuid::Uuid::new_v4(),
            )
            .await
            .expect("completefallbackpair");
            assert_eq!(rates.names.len(), 7);
            assert_eq!(
                fetch_bytes(&client, &format!("http://{address}/oversize"))
                    .await
                    .expect_err("oversize"),
                "reply_budget"
            );
            assert_eq!(
                fetch_bytes(&client, &format!("http://{address}/redirect"))
                    .await
                    .expect_err("redirect"),
                "http_status"
            );
            assert_eq!(
                fetch_bytes(&client, &format!("http://{address}/slow"))
                    .await
                    .expect_err("timeout"),
                "request"
            );
        });
    let requests = server.join().expect("server");
    assert!(requests.len() >= 6);
    assert!(
        requests
            .iter()
            .any(|value| value.starts_with("GET /fallback/currencies.min.json "))
    );
    assert!(
        requests
            .iter()
            .any(|value| value.starts_with("GET /fallback/currencies/cny.min.json "))
    );
    assert!(requests.iter().all(|value| !value.contains("美刀")
        && !value.contains("CAD")
        && !value.contains("amount")
        && !value.to_ascii_lowercase().contains("cookie:")));
}

#[test]
fn currency_catalog_captured_provider_pair_all_entries_validate_without_network() {
    let catalog = include_bytes!("../../assets/currency-catalog.json");
    let rates = include_bytes!("currency-fixtures/cny-2026-10-19.json");
    let snapshot = parse_pair(catalog, rates, NOW)
        .expect("actual captured provider schema without network or scratchfiles");
    assert_eq!(snapshot.names.len(), 339);
    assert_eq!(snapshot.names.len(), snapshot.rates.len());
    assert!((snapshot.rates["KRW"] - 199.29046727).abs() < 1e-8);
}
