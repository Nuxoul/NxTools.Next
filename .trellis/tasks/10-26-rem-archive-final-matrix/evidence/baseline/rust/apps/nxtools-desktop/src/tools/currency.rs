//! Bounded whole-copy money parsing and paired public catalog/reference-rate snapshots.
use chrono::Datelike as _;
use nxtools_core::CurrencyConversionSettings;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path, sync::OnceLock, time::Duration};

pub(crate) const DAY_MS: i64 = 86_400_000;
const MAX_REPLY: usize = 128 * 1024;
const MAX_CACHE: usize = 512 * 1024;
const MAX_CURRENCIES: usize = 4096;
const PROVIDER: &str = "fawazahmed0 Currency API";
const CDN: &str = "https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@latest/v1";
const FALLBACK: &str = "https://latest.currency-api.pages.dev/v1";
pub(crate) type CurrencyNames = BTreeMap<String, String>;

pub(crate) fn bundled_catalog() -> &'static CurrencyNames {
    static CATALOG: OnceLock<CurrencyNames> = OnceLock::new();
    CATALOG.get_or_init(|| {
        parse_catalog(include_bytes!("../../assets/currency-catalog.json"))
            .expect("checked bundled CC0 currency catalog")
    })
}
fn valid_code(code: &str) -> bool {
    CurrencyConversionSettings::valid_code(code)
        && code.bytes().all(|byte| !byte.is_ascii_lowercase())
}
fn valid_name(name: &str) -> bool {
    name.chars().count() <= 256 && !name.chars().any(char::is_control)
}
pub(crate) fn parse_catalog(bytes: &[u8]) -> Result<CurrencyNames, &'static str> {
    if bytes.len() > MAX_REPLY {
        return Err("reply_budget");
    }
    let raw: CurrencyNames = serde_json::from_slice(bytes).map_err(|_| "invalid_catalog")?;
    if !(2..=MAX_CURRENCIES).contains(&raw.len()) {
        return Err("catalog_count");
    }
    let mut names = CurrencyNames::new();
    for (code, name) in raw {
        let code = code.to_ascii_uppercase();
        if !valid_code(&code) || !valid_name(&name) {
            return Err("invalid_catalog");
        }
        let name = if name.trim().is_empty() {
            code.clone()
        } else {
            name
        };
        if names.insert(code, name).is_some() {
            return Err("duplicate_code");
        }
    }
    if !names.contains_key("CNY") {
        return Err("missing_base");
    }
    Ok(names)
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Amount {
    pub value: f64,
    pub currency: String,
}
// Longest matching marker wins. Explicit currency codes override one generic symbol.
const ALIASES: &[(&str, &str, bool)] = &[
    ("人民币", "CNY", false),
    ("美元", "USD", false),
    ("美刀", "USD", false),
    ("美金", "USD", false),
    ("刀", "USD", false),
    ("欧元", "EUR", false),
    ("英镑", "GBP", false),
    ("日元", "JPY", false),
    ("日圆", "JPY", false),
    ("韩元", "KRW", false),
    ("韩币", "KRW", false),
    ("港币", "HKD", false),
    ("港元", "HKD", false),
    ("新台币", "TWD", false),
    ("台币", "TWD", false),
    ("加元", "CAD", false),
    ("加币", "CAD", false),
    ("澳元", "AUD", false),
    ("澳币", "AUD", false),
    ("纽元", "NZD", false),
    ("新加坡元", "SGD", false),
    ("新币", "SGD", false),
    ("瑞士法郎", "CHF", false),
    ("卢布", "RUB", false),
    ("卢比", "INR", false),
    ("泰铢", "THB", false),
    ("越南盾", "VND", false),
    ("元", "CNY", false),
    ("块", "CNY", false),
    ("RMB", "CNY", true),
    ("CN¥", "CNY", true),
    ("JP¥", "JPY", true),
    ("US$", "USD", true),
    ("CA$", "CAD", true),
    ("C$", "CAD", true),
    ("AU$", "AUD", true),
    ("A$", "AUD", true),
    ("NZ$", "NZD", true),
    ("HK$", "HKD", true),
    ("NT$", "TWD", true),
    ("SG$", "SGD", true),
    ("S$", "SGD", true),
    ("R$", "BRL", true),
    ("$", "USD", false),
    ("＄", "USD", false),
    ("€", "EUR", false),
    ("£", "GBP", false),
    ("¥", "CNY", false),
    ("￥", "CNY", false),
    ("₩", "KRW", false),
    ("￦", "KRW", false),
    ("₹", "INR", false),
    ("₽", "RUB", false),
    ("฿", "THB", false),
    ("₫", "VND", false),
    ("₱", "PHP", false),
    ("₺", "TRY", false),
    ("₴", "UAH", false),
    ("₪", "ILS", false),
    ("₦", "NGN", false),
    ("₿", "BTC", false),
];
#[derive(Clone, Copy)]
struct Marker<'a> {
    code: &'a str,
    len: usize,
    explicit: bool,
}
fn marker<'a>(text: &str, catalog: &'a CurrencyNames) -> Option<Marker<'a>> {
    let mut best = None;
    for (token, code, explicit) in ALIASES {
        if catalog.contains_key(*code)
            && text
                .get(..token.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(token))
            && best.is_none_or(|prior: Marker<'_>| token.len() > prior.len)
        {
            best = Some(Marker {
                code,
                len: token.len(),
                explicit: *explicit,
            });
        }
    }
    for code in catalog.keys() {
        if text
            .get(..code.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(code))
            && best.is_none_or(|prior| code.len() >= prior.len)
        {
            best = Some(Marker {
                code,
                len: code.len(),
                explicit: true,
            });
        }
    }
    best
}
fn starts_number(text: &str) -> bool {
    text.trim_start()
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_digit() || matches!(ch, '+' | '-' | '.'))
}
fn add_marker<'a>(
    value: Marker<'a>,
    explicit: &mut Option<&'a str>,
    inferred: &mut Option<&'a str>,
) -> Option<()> {
    let slot = if value.explicit { explicit } else { inferred };
    if slot.is_some_and(|prior| prior != value.code) {
        return None;
    }
    *slot = Some(value.code);
    Some(())
}
#[cfg(test)]
pub(crate) fn parse_amounts(text: &str) -> Option<Vec<Amount>> {
    parse_amounts_in(text, bundled_catalog())
}
pub(crate) fn parse_amounts_in(text: &str, catalog: &CurrencyNames) -> Option<Vec<Amount>> {
    if text.len() > 256 || text.trim().is_empty() {
        return None;
    }
    let mut rest = text.trim();
    let mut amounts = Vec::new();
    while !rest.is_empty() {
        if amounts.len() == 8 {
            return None;
        }
        let (mut explicit, mut inferred) = (None, None);
        for _ in 0..2 {
            let Some(value) = marker(rest, catalog) else {
                break;
            };
            add_marker(value, &mut explicit, &mut inferred)?;
            rest = rest[value.len..].trim_start();
        }
        let length = rest
            .bytes()
            .take_while(|byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'+' | b'-'))
            .count();
        let number = rest.get(..length)?;
        if number.is_empty() || number.len() > 20 {
            return None;
        }
        let unsigned = number.strip_prefix(['+', '-']).unwrap_or(number);
        if unsigned.is_empty()
            || !unsigned.bytes().any(|byte| byte.is_ascii_digit())
            || unsigned.matches('.').count() > 1
            || unsigned
                .split('.')
                .next()
                .is_some_and(|whole| whole.len() > 12)
            || unsigned
                .split_once('.')
                .is_some_and(|(_, fraction)| fraction.is_empty() || fraction.len() > 4)
        {
            return None;
        }
        let value = number.parse::<f64>().ok()?;
        if !value.is_finite() || value.abs() > 1e12 {
            return None;
        }
        rest = &rest[length..];
        for _ in 0..2 {
            let trimmed = rest.trim_start();
            let Some(token) = marker(trimmed, catalog) else {
                break;
            };
            // Whitespace plus a new prefix marker starts the next amount, not a conflicting suffix.
            let after = trimmed[token.len..].trim_start();
            let next_money = starts_number(after)
                || marker(after, catalog).is_some_and(|next| starts_number(&after[next.len..]));
            if rest.len() != trimmed.len()
                && (explicit.is_some() || inferred.is_some())
                && next_money
            {
                break;
            }
            add_marker(token, &mut explicit, &mut inferred)?;
            rest = &trimmed[token.len..];
        }
        let currency = explicit.or(inferred)?.to_owned();
        amounts.push(Amount { value, currency });
        if rest.is_empty() {
            break;
        }
        let separators = rest
            .chars()
            .take_while(|ch| ch.is_whitespace() || matches!(ch, ',' | ';' | '，' | '；'))
            .map(char::len_utf8)
            .sum::<usize>();
        if separators == 0 {
            return None;
        }
        rest = rest[separators..].trim();
    }
    Some(amounts)
}
pub(crate) fn choice_label(code: &str, name: &str) -> String {
    let localized = CurrencyConversionSettings::COMMON_CURRENCIES
        .iter()
        .find_map(|(key, name)| (*key == code).then_some(*name));
    let mut label = format!("{code} · {name}");
    if let Some(localized) = localized {
        label.push_str(&format!(" · {localized}"));
    }
    if code == "USD" {
        label.push_str(" · 美刀/美金/刀");
    }
    label
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RateSnapshot {
    pub version: u32,
    pub provider: String,
    pub date: String,
    pub fetched_at: i64,
    pub rates: BTreeMap<String, f64>,
    #[serde(default)]
    pub names: CurrencyNames,
}
impl RateSnapshot {
    pub(crate) fn validate(&self, now: i64) -> Result<(), &'static str> {
        let date = chrono::NaiveDate::parse_from_str(&self.date, "%Y-%m-%d")
            .map_err(|_| "invalid_date")?;
        let today = chrono::DateTime::from_timestamp_millis(now)
            .ok_or("invalid_clock")?
            .date_naive();
        if self.date.len() != 10 || date > today || date.year() < 1999 {
            return Err("invalid_date");
        }
        let legacy = self.version == 1
            && matches!(self.provider.as_str(), "ExchangeRate-API" | "Frankfurter");
        if !(legacy || self.version == 2 && self.provider == PROVIDER)
            || self.fetched_at <= 0
            || self.fetched_at > now.saturating_add(300_000)
            || !(2..=MAX_CURRENCIES).contains(&self.rates.len())
            || self
                .rates
                .get("CNY")
                .is_none_or(|rate| (*rate - 1.0).abs() > 1e-9)
            || self.rates.iter().any(|(code, rate)| {
                !valid_code(code) || !rate.is_finite() || *rate <= 0.0 || *rate > 1e12
            })
        {
            return Err("invalid_rates");
        }
        if self.version == 2
            && (self.names.len() != self.rates.len()
                || self.names.iter().any(|(code, name)| {
                    !self.rates.contains_key(code) || name.is_empty() || !valid_name(name)
                }))
        {
            return Err("catalog_rates_mismatch");
        }
        Ok(())
    }
    pub(crate) fn catalog(&self) -> &CurrencyNames {
        if self.version == 2 {
            &self.names
        } else {
            bundled_catalog()
        }
    }
    pub(crate) fn refresh_due(&self, now: i64) -> bool {
        self.version != 2 || now.saturating_sub(self.fetched_at) >= DAY_MS
    }
    pub(crate) fn stale(&self, now: i64) -> bool {
        now.saturating_sub(self.fetched_at) >= DAY_MS
            || chrono::NaiveDate::parse_from_str(&self.date, "%Y-%m-%d")
                .ok()
                .and_then(|date| date.and_hms_opt(0, 0, 0))
                .is_none_or(|date| {
                    now.saturating_sub(date.and_utc().timestamp_millis()) >= 4 * DAY_MS
                })
    }
    pub(crate) fn convert(&self, amount: &Amount, target: &str) -> Option<f64> {
        let result = amount.value / self.rates.get(&amount.currency)? * self.rates.get(target)?;
        (result.is_finite() && result.abs() <= 1e15).then_some(result)
    }
    pub(crate) fn message(&self, amounts: &[Amount], target: &str, now: i64) -> Option<String> {
        let mut lines = Vec::new();
        for amount in amounts {
            if amount.currency == target {
                continue;
            }
            let converted = self.convert(amount, target)?;
            lines.push(format!(
                "{} {} ≈ {converted:.2} {target}",
                format_amount(amount.value),
                amount.currency
            ));
        }
        if lines.is_empty() {
            return None;
        }
        Some(format!(
            "{}\n参考汇率 · {} · {}{}",
            lines.join("\n"),
            self.date,
            self.provider,
            if self.stale(now) {
                " · 缓存已过期"
            } else {
                ""
            }
        ))
    }
}
fn format_amount(value: f64) -> String {
    let text = format!("{value:.4}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CnyReply {
    date: String,
    cny: BTreeMap<String, f64>,
}
pub(crate) fn parse_pair(
    catalog: &[u8],
    rates: &[u8],
    now: i64,
) -> Result<RateSnapshot, &'static str> {
    let names = parse_catalog(catalog)?;
    if rates.len() > MAX_REPLY {
        return Err("reply_budget");
    }
    let reply: CnyReply = serde_json::from_slice(rates).map_err(|_| "invalid_rates_reply")?;
    let mut rates = BTreeMap::new();
    for (code, rate) in reply.cny {
        if rates.insert(code.to_ascii_uppercase(), rate).is_some() {
            return Err("duplicate_code");
        }
    }
    let snapshot = RateSnapshot {
        version: 2,
        provider: PROVIDER.into(),
        date: reply.date,
        fetched_at: now,
        rates,
        names,
    };
    snapshot.validate(now)?;
    Ok(snapshot)
}
pub(crate) fn load_cache(path: &Path, now: i64) -> Option<RateSnapshot> {
    use std::io::Read as _;
    let file = std::fs::File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_CACHE as u64 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > MAX_CACHE {
        return None;
    }
    let snapshot: RateSnapshot = serde_json::from_slice(&bytes).ok()?;
    snapshot.validate(now).ok()?;
    Some(snapshot)
}
pub(crate) fn save_cache(path: &Path, snapshot: &RateSnapshot) -> Result<(), &'static str> {
    use std::io::Write as _;
    snapshot.validate(snapshot.fetched_at)?;
    let bytes = serde_json::to_vec(snapshot).map_err(|_| "cache_encode")?;
    if bytes.len() > MAX_CACHE {
        return Err("cache_budget");
    }
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| "cache_create")?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "cache_write")?;
        drop(file);
        std::fs::rename(&temporary, path).map_err(|_| "cache_commit")
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}
pub(crate) async fn fetch_rates(
    route: nxtools_network::NetworkOperationSnapshot,
    now: i64,
    id: uuid::Uuid,
) -> Result<RateSnapshot, &'static str> {
    let client = route
        .build_client(
            reqwest::Client::builder()
                .timeout(Duration::from_secs(8))
                .connect_timeout(Duration::from_secs(3))
                .redirect(reqwest::redirect::Policy::none()),
        )
        .map_err(|_| "client")?;
    tokio::time::timeout(
        Duration::from_secs(16),
        fetch_sources(
            &client,
            [("jsdelivr", CDN), ("cloudflare", FALLBACK)],
            now,
            id,
        ),
    )
    .await
    .map_err(|_| "deadline")?
}
async fn fetch_sources(
    client: &reqwest::Client,
    sources: [(&str, &str); 2],
    now: i64,
    id: uuid::Uuid,
) -> Result<RateSnapshot, &'static str> {
    for (source, root) in sources {
        let result = tokio::time::timeout(Duration::from_secs(7), fetch_pair(client, root, now))
            .await
            .map_err(|_| "source_deadline")
            .and_then(|result| result);
        tracing::info!(event="tools.currency.provider_completed",correlation_id=%id,source,success=result.is_ok(),failure_kind=result.as_ref().err().copied().unwrap_or("none"),"fixed catalog/rates source completed");
        if let Ok(snapshot) = result {
            return Ok(snapshot);
        }
    }
    Err("sources_unavailable")
}
async fn fetch_pair(
    client: &reqwest::Client,
    root: &str,
    now: i64,
) -> Result<RateSnapshot, &'static str> {
    let catalog_url = format!("{root}/currencies.min.json");
    let rates_url = format!("{root}/currencies/cny.min.json");
    let (catalog, rates) = tokio::try_join!(
        fetch_bytes(client, &catalog_url),
        fetch_bytes(client, &rates_url)
    )?;
    parse_pair(&catalog, &rates, now)
}
async fn fetch_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, &'static str> {
    let mut reply = client.get(url).send().await.map_err(|_| "request")?;
    if !reply.status().is_success() {
        return Err("http_status");
    }
    if reply
        .content_length()
        .is_some_and(|length| length > MAX_REPLY as u64)
    {
        return Err("reply_budget");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = reply.chunk().await.map_err(|_| "body")? {
        if bytes.len() + chunk.len() > MAX_REPLY {
            return Err("reply_budget");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
#[path = "currency_tests.rs"]
mod tests;
