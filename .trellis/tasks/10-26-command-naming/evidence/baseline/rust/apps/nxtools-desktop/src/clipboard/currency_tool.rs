//! Currency runtime on the existing clipboard owner; effects cross owners on a later turn.
use super::*;
use crate::tools::currency::{self, Amount, CurrencyNames, RateSnapshot};

#[derive(Default)]
pub(super) struct CurrencyState {
    pub rates: Option<RateSnapshot>,
    pub pending: Option<uuid::Uuid>,
    pub error: Option<&'static str>,
    initialized: bool,
    generation: u64,
    capture_serial: u64,
    last_attempt: i64,
    network: Option<nxtools_network::NetworkPolicy>,
    queued: Option<(CurrencyNoticeToken, Vec<Amount>)>,
    daily_task: Option<gpui_kit::Task<()>>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct CurrencyNoticeToken {
    generation: u64,
    capture_serial: u64,
    pause_generation: u64,
}
#[derive(Clone, Default)]
pub(crate) struct CurrencyStatus {
    pub enabled: bool,
    pub target: String,
    pub busy: bool,
    pub date: Option<String>,
    pub updated_at: Option<String>,
    pub provider: Option<String>,
    pub error: Option<&'static str>,
    pub stale: bool,
}
impl ClipboardApplication {
    #[cfg(test)]
    pub(crate) fn currency_fixture_refresh_state(
        &mut self,
        busy: bool,
        error: Option<&'static str>,
        cx: &mut Context<Self>,
    ) {
        self.currency.pending = busy.then(uuid::Uuid::new_v4);
        self.currency.error = error;
        cx.notify();
    }
    #[cfg(test)]
    pub(crate) fn currency_fixture_apply_rates(
        &mut self,
        rates: RateSnapshot,
        cx: &mut Context<Self>,
    ) {
        rates
            .validate(nxtools_ipc::unix_time_ms())
            .expect("valid fixture rates");
        let id = uuid::Uuid::new_v4();
        self.currency.pending = Some(id);
        self.finish_currency_refresh(id, Ok((rates, true)), Instant::now(), cx);
    }
    pub(crate) fn currency_catalog(&self) -> &CurrencyNames {
        match &self.currency.rates {
            Some(rates) => rates.catalog(),
            None => currency::bundled_catalog(),
        }
    }
    pub(crate) fn currency_catalog_revision(&self) -> (u32, i64) {
        self.currency
            .rates
            .as_ref()
            .map_or((0, 0), |rates| (rates.version, rates.fetched_at))
    }
    pub(crate) fn currency_status(&self) -> CurrencyStatus {
        let settings = &self.config.clipboard.currency_conversion;
        CurrencyStatus {
            enabled: settings.enabled,
            target: settings.target.clone(),
            busy: self.currency.pending.is_some(),
            date: self.currency.rates.as_ref().map(|rates| rates.date.clone()),
            updated_at: self
                .currency
                .rates
                .as_ref()
                .and_then(|rates| chrono::DateTime::from_timestamp_millis(rates.fetched_at))
                .map(|date| {
                    date.with_timezone(&chrono::Local)
                        .format("%m-%d %H:%M")
                        .to_string()
                }),
            provider: self
                .currency
                .rates
                .as_ref()
                .map(|rates| rates.provider.clone()),
            error: self.currency.error,
            stale: self
                .currency
                .rates
                .as_ref()
                .is_some_and(|rates| rates.stale(nxtools_ipc::unix_time_ms())),
        }
    }
    pub(crate) fn bind_currency_network(
        &mut self,
        policy: nxtools_network::NetworkPolicy,
        cx: &mut Context<Self>,
    ) {
        self.currency.network = Some(policy);
        self.initialize_currency_cache(cx);
    }
    fn currency_cache_path(&self) -> PathBuf {
        self.config_path.with_file_name("currency-rates-v2.json")
    }
    fn initialize_currency_cache(&mut self, cx: &mut Context<Self>) {
        if self.currency.initialized {
            return;
        }
        self.currency.initialized = true;
        let path = self.currency_cache_path();
        let legacy_path = self.config_path.with_file_name("currency-rates-v1.json");
        cx.spawn(async move |owner, cx| {
            let rates = cx
                .background_executor()
                .spawn(async move {
                    let now = nxtools_ipc::unix_time_ms();
                    currency::load_cache(&path, now)
                        .or_else(|| currency::load_cache(&legacy_path, now))
                })
                .await;
            let _ = owner.update(cx, |owner, cx| {
                if owner.currency.rates.is_none() {
                    owner.currency.rates = rates;
                }
                if owner.config.clipboard.currency_conversion.enabled {
                    owner.refresh_currency_rates(false, cx);
                    owner.schedule_currency_daily(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }
    pub(crate) fn set_currency_settings(
        &mut self,
        enabled: bool,
        target: String,
        cx: &mut Context<Self>,
    ) {
        let target = target.trim().to_ascii_uppercase();
        if !self.currency_catalog().contains_key(&target)
            && target != self.config.clipboard.currency_conversion.target
        {
            return;
        }
        match AppConfig::update_section(&self.config_path, |config| {
            config.clipboard.currency_conversion =
                nxtools_core::CurrencyConversionSettings { enabled, target };
        }) {
            Ok(merged) => {
                self.config.clipboard.currency_conversion = merged.clipboard.currency_conversion;
                self.currency.generation = self.currency.generation.wrapping_add(1);
                self.invalidate_currency_capture();
                self.currency.error = None;
                self.currency.daily_task = None;
                if enabled {
                    self.refresh_currency_rates(false, cx);
                    self.schedule_currency_daily(cx);
                }
            }
            Err(_) => self.currency.error = Some("设置保存失败，请重试"),
        }
        cx.notify();
    }
    fn schedule_currency_daily(&mut self, cx: &mut Context<Self>) {
        self.currency.daily_task = Some(cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(Duration::from_secs(24 * 60 * 60))
                .await;
            let _ = owner.update(cx, |owner, cx| {
                if owner.config.clipboard.currency_conversion.enabled {
                    owner.refresh_currency_rates(false, cx);
                    owner.schedule_currency_daily(cx);
                }
            });
        }));
    }
    pub(crate) fn refresh_currency_rates(&mut self, manual: bool, cx: &mut Context<Self>) {
        if self.currency.pending.is_some() {
            return;
        }
        let now = nxtools_ipc::unix_time_ms();
        if !manual
            && (self
                .currency
                .rates
                .as_ref()
                .is_some_and(|rates| !rates.refresh_due(now))
                || now.saturating_sub(self.currency.last_attempt) < 60_000)
        {
            return;
        }
        let Some(policy) = self
            .currency
            .network
            .clone()
            .or_else(|| nxtools_network::NetworkPolicy::new(self.config.network.clone()).ok())
        else {
            self.currency.error = Some("网络设置不可用");
            cx.notify();
            return;
        };
        let Ok(route) = policy.snapshot() else {
            self.currency.error = Some("网络设置不可用");
            cx.notify();
            return;
        };
        let id = uuid::Uuid::new_v4();
        self.currency.pending = Some(id);
        self.currency.last_attempt = now;
        self.currency.error = None;
        let path = self.currency_cache_path();
        let started = Instant::now();
        info!(event="tools.currency.refresh_started",correlation_id=%id,manual,"currency rates refresh accepted");
        cx.spawn(async move |owner, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|_| "runtime")?;
                    let rates = runtime.block_on(currency::fetch_rates(route, now, id))?;
                    let saved = currency::save_cache(&path, &rates).is_ok();
                    Ok::<_, &'static str>((rates, saved))
                })
                .await;
            let _ = owner.update(cx, |owner, cx| {
                owner.finish_currency_refresh(id, result, started, cx)
            });
        })
        .detach();
        cx.notify();
    }
    fn finish_currency_refresh(
        &mut self,
        id: uuid::Uuid,
        result: Result<(RateSnapshot, bool), &'static str>,
        started: Instant,
        cx: &mut Context<Self>,
    ) {
        if self.currency.pending != Some(id) {
            return;
        }
        self.currency.pending = None;
        let failure = result.as_ref().err().copied();
        match result {
            Ok((rates, saved)) => {
                self.currency.rates = Some(rates);
                self.currency.error =
                    (!saved).then_some("汇率已更新，但缓存保存失败；重启后需重试");
            }
            Err(_) => {
                self.currency.error = Some(if self.currency.rates.is_some() {
                    "汇率更新失败，已保留上次有效数据，请重试"
                } else {
                    "汇率更新失败，暂无可用数据，请重试"
                })
            }
        }
        info!(event="tools.currency.refresh_completed",correlation_id=%id,success=failure.is_none(),failure_kind=failure.unwrap_or("none"),rate_count=self.currency.rates.as_ref().map_or(0,|rates|rates.rates.len()),elapsed_ms=started.elapsed().as_millis(),"currency rates terminal result");
        if let Some((token, amounts)) = self.currency.queued.take()
            && self.currency_notice_is_current(token)
        {
            self.notify_currency_amounts(amounts, cx);
        }
        cx.notify();
    }
    pub(super) fn invalidate_currency_capture(&mut self) {
        self.currency.capture_serial = self.currency.capture_serial.wrapping_add(1);
        self.currency.queued = None;
    }
    fn currency_notice_token(&self) -> CurrencyNoticeToken {
        CurrencyNoticeToken {
            generation: self.currency.generation,
            capture_serial: self.currency.capture_serial,
            pause_generation: self.capture_pause.generation,
        }
    }
    fn currency_notice_is_current(&self, token: CurrencyNoticeToken) -> bool {
        token == self.currency_notice_token()
            && self.config.clipboard.enabled
            && self.config.clipboard.currency_conversion.enabled
            && !self.capture_pause.blocks(nxtools_ipc::unix_time_ms())
    }
    pub(super) fn handle_currency_capture(&mut self, text: &str, cx: &mut Context<Self>) {
        if !self.currency_notice_is_current(self.currency_notice_token()) {
            return;
        }
        let Some(amounts) = currency::parse_amounts_in(text, self.currency_catalog()) else {
            return;
        };
        let target = &self.config.clipboard.currency_conversion.target;
        if amounts.iter().all(|amount| &amount.currency == target) {
            return;
        }
        if self.currency.rates.is_some() {
            self.notify_currency_amounts(amounts, cx);
            self.refresh_currency_rates(false, cx);
        } else {
            self.currency.queued = Some((self.currency_notice_token(), amounts));
            self.refresh_currency_rates(false, cx);
            if self.currency.pending.is_none() {
                self.currency.queued = None;
                self.notify_currency_message("暂无可用汇率，请在汇率换算设置中更新数据".into(), cx);
            }
        }
    }
    fn notify_currency_amounts(&self, amounts: Vec<Amount>, cx: &mut Context<Self>) {
        let target = &self.config.clipboard.currency_conversion.target;
        let message = match &self.currency.rates {
            Some(rates) => rates
                .message(&amounts, target, nxtools_ipc::unix_time_ms())
                .unwrap_or_else(|| "当前汇率数据不含所需币种，请更新数据后重试".into()),
            None => "暂无可用汇率，请在汇率换算设置中更新数据".into(),
        };
        self.notify_currency_message(message, cx);
    }
    fn notify_currency_message(&self, message: String, cx: &mut Context<Self>) {
        let owner = cx.entity().downgrade();
        let token = self.currency_notice_token();
        let notifications = self.notifications.clone();
        let id = format!("tools.currency.{}", uuid::Uuid::new_v4());
        cx.defer(move |cx| {
            if owner
                .upgrade()
                .is_some_and(|owner| owner.read(cx).currency_notice_is_current(token))
            {
                notifications.update(cx, |center, cx| {
                    center.show_info(id, "汇率换算", message, cx)
                });
            }
        });
    }
}

#[cfg(test)]
#[path = "currency_tests.rs"]
mod tests;
