use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        mpsc::{self, RecvTimeoutError, TryRecvError},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use crypto_hud_core::{
    clamp_refresh_interval, market_pair_source, normalize_market_pair_key, parse_market_pair,
    MarketDataSource, MarketPair, MarketProviderPreference, MarketType,
};
use serde::Deserialize;

const BINANCE_BASE_URL: &str = "https://api.binance.com";
const COINBASE_EXCHANGE_BASE_URL: &str = "https://api.exchange.coinbase.com";
const OKX_BASE_URL: &str = "https://www.okx.com";
const HYPERLIQUID_BASE_URL: &str = "https://api.hyperliquid.xyz";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_CONCURRENT_PAIR_FETCHES: usize = 4;
const CONFIG_POLL_INTERVAL: Duration = Duration::from_millis(250);
const MAX_FAILURE_BACKOFF: Duration = Duration::from_secs(60);
const CANDLE_REFRESH_INTERVAL: Duration = Duration::from_secs(5 * 60);
const MAX_CANDLE_FAILURE_BACKOFF: Duration = Duration::from_secs(60 * 60);
const CANDLE_LIMIT_24H_5M: usize = 24 * 60 / 5;
const HYPERLIQUID_CANDLE_INTERVAL: &str = "5m";
const HYPERLIQUID_24H_MILLIS: u64 = 24 * 60 * 60 * 1000;
const USER_AGENT: &str = concat!("crypto-hud/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone)]
pub struct MarketSnapshot {
    pub symbol: String,
    pub price: f64,
    pub change_percent_24h: f64,
    pub chart_closes_24h: Vec<f64>,
    pub chart_candles_24h: Vec<MarketCandle>,
    pub chart_updated_at: Option<Instant>,
    pub chart_error: Option<String>,
    pub source: MarketDataSource,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarketCandle {
    pub open_time_millis: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

#[derive(Debug, Clone)]
pub enum MarketEvent {
    Snapshot(MarketSnapshot),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketFeedConfig {
    pub symbols: Vec<String>,
    pub provider: MarketProviderPreference,
    pub refresh_interval_seconds: i32,
    pub enabled_sources: Vec<MarketDataSource>,
    pub proxy_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolCatalogEntry {
    pub key: String,
    pub base: String,
    pub quote: String,
    pub source: MarketDataSource,
    pub market_type: MarketType,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SymbolCatalog {
    pub entries: Vec<SymbolCatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolCatalogWarning {
    pub source: MarketDataSource,
    pub message: String,
}

impl std::fmt::Display for SymbolCatalogWarning {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.source.label(), self.message)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SymbolCatalogFetch {
    pub catalog: SymbolCatalog,
    pub warnings: Vec<SymbolCatalogWarning>,
}

#[derive(Debug, Clone)]
struct CandleCacheEntry {
    candles: Vec<MarketCandle>,
    fetched_at: Option<Instant>,
    retry_at: Option<Instant>,
    consecutive_failures: u32,
    last_error: Option<String>,
}

#[derive(Debug, Clone)]
struct CandleFetchOutcome {
    candles: Vec<MarketCandle>,
    fetched_at: Option<Instant>,
    error: Option<String>,
    new_failure: bool,
}

type CandleCache = HashMap<String, CandleCacheEntry>;

struct MarketFetchOutcome {
    snapshot: MarketSnapshot,
    warning: Option<String>,
}

struct MarketBatch {
    snapshots: Vec<MarketSnapshot>,
    errors: Vec<String>,
}

enum MarketFeedControl {
    Stop,
}

enum MarketFeedWait {
    Elapsed,
    ConfigChanged,
    Stop,
}

pub struct MarketFeed {
    events: mpsc::Receiver<MarketEvent>,
    control: mpsc::Sender<MarketFeedControl>,
}

impl MarketFeed {
    pub fn from_events(events: Vec<MarketEvent>) -> Self {
        let (event_sender, event_receiver) = mpsc::channel();
        for event in events {
            if event_sender.send(event).is_err() {
                break;
            }
        }
        drop(event_sender);
        let (control, control_receiver) = mpsc::channel();
        drop(control_receiver);
        Self {
            events: event_receiver,
            control,
        }
    }

    pub fn try_recv(&self) -> std::result::Result<MarketEvent, TryRecvError> {
        self.events.try_recv()
    }

    pub fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> std::result::Result<MarketEvent, RecvTimeoutError> {
        self.events.recv_timeout(timeout)
    }
}

impl Drop for MarketFeed {
    fn drop(&mut self) {
        let _ = self.control.send(MarketFeedControl::Stop);
    }
}

pub fn spawn_market_feed(config: Arc<Mutex<MarketFeedConfig>>) -> MarketFeed {
    let (event_sender, events) = mpsc::channel();
    let (control, control_receiver) = mpsc::channel();
    let worker_sender = event_sender.clone();
    if let Err(error) = thread::Builder::new()
        .name("crypto-hud-market-feed".to_string())
        .spawn(move || {
            run_market_feed(config, worker_sender, control_receiver);
        })
    {
        let _ = event_sender.send(MarketEvent::Error(format!(
            "failed to start market feed worker: {error}"
        )));
    }
    drop(event_sender);

    MarketFeed { events, control }
}

fn run_market_feed(
    shared_config: Arc<Mutex<MarketFeedConfig>>,
    sender: mpsc::Sender<MarketEvent>,
    control: mpsc::Receiver<MarketFeedControl>,
) {
    let Ok(mut agent) = build_agent(None) else {
        let _ = sender.send(MarketEvent::Error(
            "failed to create direct market agent".to_string(),
        ));
        return;
    };
    let mut agent_proxy_url: Option<String> = None;
    let mut candle_cache = CandleCache::new();
    let mut consecutive_failures = 0_u32;

    'feed: loop {
        if market_feed_stop_requested(&control) {
            return;
        }
        let config = market_feed_config_snapshot(&shared_config);
        let refresh_interval =
            Duration::from_secs(clamp_refresh_interval(config.refresh_interval_seconds) as u64);
        let proxy_url = normalized_proxy_url(config.proxy_url.clone());
        let symbols = unique_market_pair_keys(config.symbols.clone())
            .into_iter()
            .filter(|symbol| {
                market_pair_source(symbol).is_some_and(|source| {
                    config.enabled_sources.is_empty() || config.enabled_sources.contains(&source)
                })
            })
            .collect::<Vec<_>>();

        let agent_error = if agent_proxy_url != proxy_url {
            match build_agent(proxy_url.as_deref()) {
                Ok(next_agent) => {
                    agent = next_agent;
                    agent_proxy_url = proxy_url;
                    candle_cache.clear();
                    None
                }
                Err(error) => Some(error),
            }
        } else {
            None
        };

        let cycle_succeeded = if let Some(error) = agent_error {
            send_market_error(&sender, error.to_string())
        } else {
            let mut interrupted_by_stop = false;
            let mut interrupted_by_config = false;
            let fetch = fetch_symbols_interruptible(&agent, &symbols, &mut candle_cache, || {
                interrupted_by_stop = market_feed_stop_requested(&control);
                interrupted_by_config =
                    !interrupted_by_stop && market_feed_config_snapshot(&shared_config) != config;
                interrupted_by_stop || interrupted_by_config
            });
            if interrupted_by_stop {
                return;
            }
            if interrupted_by_config {
                consecutive_failures = 0;
                continue 'feed;
            }
            let Some(fetch) = fetch else {
                continue 'feed;
            };
            send_market_batch(&sender, fetch)
        };

        let Some(cycle_succeeded) = cycle_succeeded else {
            return;
        };
        if cycle_succeeded {
            consecutive_failures = 0;
        } else {
            consecutive_failures = consecutive_failures.saturating_add(1);
        }
        let wait = if cycle_succeeded {
            refresh_interval
        } else {
            failure_backoff(refresh_interval, consecutive_failures)
        };
        match wait_for_market_feed(&control, &shared_config, &config, wait) {
            MarketFeedWait::Elapsed => {}
            MarketFeedWait::ConfigChanged => {
                consecutive_failures = 0;
            }
            MarketFeedWait::Stop => return,
        }
    }
}

fn send_market_batch(
    sender: &mpsc::Sender<MarketEvent>,
    result: Result<MarketBatch>,
) -> Option<bool> {
    match result {
        Ok(MarketBatch { snapshots, errors }) => {
            for snapshot in snapshots {
                if sender.send(MarketEvent::Snapshot(snapshot)).is_err() {
                    return None;
                }
            }
            if !errors.is_empty() && sender.send(MarketEvent::Error(errors.join("; "))).is_err() {
                return None;
            }
            Some(true)
        }
        Err(error) => send_market_error(sender, error.to_string()),
    }
}

fn send_market_error(sender: &mpsc::Sender<MarketEvent>, error: String) -> Option<bool> {
    sender.send(MarketEvent::Error(error)).ok().map(|()| false)
}

fn market_feed_config_snapshot(config: &Arc<Mutex<MarketFeedConfig>>) -> MarketFeedConfig {
    match config.lock() {
        Ok(config) => config.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

fn market_feed_stop_requested(control: &mpsc::Receiver<MarketFeedControl>) -> bool {
    matches!(
        control.try_recv(),
        Ok(MarketFeedControl::Stop) | Err(TryRecvError::Disconnected)
    )
}

fn wait_for_market_feed(
    control: &mpsc::Receiver<MarketFeedControl>,
    config: &Arc<Mutex<MarketFeedConfig>>,
    observed_config: &MarketFeedConfig,
    wait: Duration,
) -> MarketFeedWait {
    let deadline = Instant::now() + wait;
    loop {
        if market_feed_stop_requested(control) {
            return MarketFeedWait::Stop;
        }
        if market_feed_config_snapshot(config) != *observed_config {
            return MarketFeedWait::ConfigChanged;
        }
        let now = Instant::now();
        if now >= deadline {
            return MarketFeedWait::Elapsed;
        }
        let timeout = deadline
            .saturating_duration_since(now)
            .min(CONFIG_POLL_INTERVAL);
        match control.recv_timeout(timeout) {
            Ok(MarketFeedControl::Stop) | Err(RecvTimeoutError::Disconnected) => {
                return MarketFeedWait::Stop;
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

fn failure_backoff(refresh_interval: Duration, consecutive_failures: u32) -> Duration {
    let multiplier = 1_u32
        .checked_shl(consecutive_failures.min(6))
        .unwrap_or(u32::MAX);
    refresh_interval
        .saturating_mul(multiplier)
        .min(MAX_FAILURE_BACKOFF)
}

pub fn fetch_symbol_catalog(proxy_url: Option<&str>) -> Result<SymbolCatalog> {
    Ok(fetch_symbol_catalog_with_warnings(proxy_url)?.catalog)
}

pub fn fetch_symbol_catalog_with_warnings(proxy_url: Option<&str>) -> Result<SymbolCatalogFetch> {
    let agent = build_agent(proxy_url)?;
    fetch_symbol_catalog_with_agent(&agent)
}

fn fetch_symbol_catalog_with_agent(agent: &ureq::Agent) -> Result<SymbolCatalogFetch> {
    let mut warnings = Vec::new();
    let mut catalogs = Vec::new();

    collect_symbol_catalog_result(
        MarketDataSource::Binance,
        fetch_binance_symbol_catalog(agent),
        &mut catalogs,
        &mut warnings,
    );
    collect_symbol_catalog_result(
        MarketDataSource::Coinbase,
        fetch_coinbase_symbol_catalog(agent),
        &mut catalogs,
        &mut warnings,
    );
    collect_symbol_catalog_result(
        MarketDataSource::Okx,
        fetch_okx_symbol_catalog(agent),
        &mut catalogs,
        &mut warnings,
    );
    collect_symbol_catalog_result(
        MarketDataSource::Hyperliquid,
        fetch_hyperliquid_symbol_catalog(agent),
        &mut catalogs,
        &mut warnings,
    );

    finish_symbol_catalog_fetch(catalogs, warnings)
}

fn collect_symbol_catalog_result(
    source: MarketDataSource,
    result: Result<Vec<SymbolCatalogEntry>>,
    catalogs: &mut Vec<Vec<SymbolCatalogEntry>>,
    warnings: &mut Vec<SymbolCatalogWarning>,
) {
    match result {
        Ok(entries) if entries.is_empty() => warnings.push(SymbolCatalogWarning {
            source,
            message: "source returned no supported market pairs".to_string(),
        }),
        Ok(entries) => catalogs.push(entries),
        Err(error) => warnings.push(SymbolCatalogWarning {
            source,
            message: format!("{error:#}"),
        }),
    }
}

fn finish_symbol_catalog_fetch(
    catalogs: Vec<Vec<SymbolCatalogEntry>>,
    warnings: Vec<SymbolCatalogWarning>,
) -> Result<SymbolCatalogFetch> {
    let catalog = combine_symbol_catalogs(catalogs);
    if catalog.entries.is_empty() {
        if warnings.is_empty() {
            bail!("all market sources returned an empty symbol catalog");
        }
        bail!(
            "{}",
            warnings
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        );
    }
    Ok(SymbolCatalogFetch { catalog, warnings })
}

fn build_agent(proxy_url: Option<&str>) -> Result<ureq::Agent> {
    let mut builder = ureq::AgentBuilder::new()
        .timeout_connect(REQUEST_TIMEOUT)
        .timeout_read(REQUEST_TIMEOUT);
    if let Some(proxy_url) = proxy_url.and_then(non_empty_trimmed) {
        let proxy = ureq::Proxy::new(proxy_url).context("invalid proxy URL")?;
        builder = builder.proxy(proxy);
    }
    Ok(builder.build())
}

fn normalized_proxy_url(proxy_url: Option<String>) -> Option<String> {
    proxy_url.and_then(|proxy_url| {
        let proxy_url = proxy_url.trim();
        if proxy_url.is_empty() {
            None
        } else {
            Some(proxy_url.to_string())
        }
    })
}

fn non_empty_trimmed(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn unique_market_pair_keys(symbols: Vec<String>) -> Vec<String> {
    symbols
        .iter()
        .filter_map(|symbol| normalize_market_pair_key(symbol))
        .fold(Vec::new(), |mut unique, symbol| {
            if !unique.contains(&symbol) {
                unique.push(symbol);
            }
            unique
        })
}

fn fetch_symbols_interruptible(
    agent: &ureq::Agent,
    symbols: &[String],
    candle_cache: &mut CandleCache,
    mut interrupted: impl FnMut() -> bool,
) -> Option<Result<MarketBatch>> {
    if interrupted() {
        return None;
    }
    if symbols.is_empty() {
        return Some(Err(anyhow!("no market pairs configured")));
    }

    let mut snapshots = Vec::new();
    let mut errors = Vec::new();
    let mut pairs = Vec::new();
    for symbol in symbols {
        match parse_market_pair(symbol) {
            Some(pair) => pairs.push((symbol.clone(), pair)),
            None => errors.push(format!("{symbol}: invalid market pair")),
        }
    }

    candle_cache.retain(|key, _| symbols.contains(key));
    for chunk in pairs.chunks(MAX_CONCURRENT_PAIR_FETCHES) {
        if interrupted() {
            return None;
        }
        let results = thread::scope(|scope| {
            let handles = chunk
                .iter()
                .map(|(symbol, pair)| {
                    let agent = agent.clone();
                    let symbol = symbol.clone();
                    let pair = pair.clone();
                    let cached_entry = candle_cache.get(&pair.key()).cloned();
                    scope.spawn(move || {
                        let key = pair.key();
                        let mut local_cache = CandleCache::new();
                        if let Some(entry) = cached_entry {
                            local_cache.insert(key.clone(), entry);
                        }
                        let result = fetch_pair(&agent, &pair, &mut local_cache);
                        (symbol, result, local_cache.remove(&key))
                    })
                })
                .collect::<Vec<_>>();

            handles
                .into_iter()
                .map(|handle| handle.join())
                .collect::<Vec<_>>()
        });

        for result in results {
            let Ok((symbol, result, cached_entry)) = result else {
                errors.push("market pair worker panicked".to_string());
                continue;
            };
            if let Some(entry) = cached_entry {
                candle_cache.insert(symbol.clone(), entry);
            }
            match result {
                Ok(outcome) => {
                    snapshots.push(outcome.snapshot);
                    if let Some(warning) = outcome.warning {
                        errors.push(format!("{symbol}: {warning}"));
                    }
                }
                Err(error) => errors.push(format!("{symbol}: {error}")),
            }
        }
        if interrupted() {
            return None;
        }
    }

    Some(market_batch(snapshots, errors))
}

fn market_batch(snapshots: Vec<MarketSnapshot>, errors: Vec<String>) -> Result<MarketBatch> {
    if snapshots.is_empty() {
        bail!("{}", errors.join("; "));
    }
    Ok(MarketBatch { snapshots, errors })
}

fn fetch_pair(
    agent: &ureq::Agent,
    pair: &MarketPair,
    candle_cache: &mut CandleCache,
) -> Result<MarketFetchOutcome> {
    match pair.source {
        MarketDataSource::Binance => fetch_binance(agent, pair, candle_cache),
        MarketDataSource::Coinbase => fetch_coinbase(agent, pair, candle_cache),
        MarketDataSource::Okx => fetch_okx(agent, pair, candle_cache),
        MarketDataSource::Hyperliquid => fetch_hyperliquid(agent, pair, candle_cache),
    }
}

fn fetch_binance(
    agent: &ureq::Agent,
    pair: &MarketPair,
    candle_cache: &mut CandleCache,
) -> Result<MarketFetchOutcome> {
    ensure_spot_pair(pair)?;
    let ticker_symbol = binance_ticker_symbol(pair);
    let url = format!("{BINANCE_BASE_URL}/api/v3/ticker/24hr?symbol={ticker_symbol}");
    let body = get_json(agent, &url)?;
    let snapshot = parse_binance_ticker(pair, &body)?;
    Ok(snapshot_with_candles(
        snapshot,
        cached_or_fetch_candles(agent, pair, candle_cache, fetch_binance_candles),
    ))
}

fn fetch_coinbase(
    agent: &ureq::Agent,
    pair: &MarketPair,
    candle_cache: &mut CandleCache,
) -> Result<MarketFetchOutcome> {
    ensure_spot_pair(pair)?;
    let product_id = coinbase_product_id(pair);
    let url = format!("{COINBASE_EXCHANGE_BASE_URL}/products/{product_id}/stats");
    let body = get_json(agent, &url)?;
    let snapshot = parse_coinbase_stats(pair, &body)?;
    Ok(snapshot_with_candles(
        snapshot,
        cached_or_fetch_candles(agent, pair, candle_cache, fetch_coinbase_candles),
    ))
}

fn fetch_okx(
    agent: &ureq::Agent,
    pair: &MarketPair,
    candle_cache: &mut CandleCache,
) -> Result<MarketFetchOutcome> {
    ensure_spot_pair(pair)?;
    let instrument_id = okx_instrument_id(pair);
    let url = format!("{OKX_BASE_URL}/api/v5/market/ticker?instId={instrument_id}");
    let body = get_json(agent, &url)?;
    let snapshot = parse_okx_ticker(pair, &body)?;
    Ok(snapshot_with_candles(
        snapshot,
        cached_or_fetch_candles(agent, pair, candle_cache, fetch_okx_candles),
    ))
}

fn fetch_hyperliquid(
    agent: &ureq::Agent,
    pair: &MarketPair,
    candle_cache: &mut CandleCache,
) -> Result<MarketFetchOutcome> {
    if pair.market_type != MarketType::Perp {
        bail!("Hyperliquid spot pairs are not supported yet");
    }
    if pair.quote != "USDC" {
        bail!("Hyperliquid perpetual pairs must be quoted in USDC");
    }

    let url = format!("{HYPERLIQUID_BASE_URL}/info");
    let body = post_json(
        agent,
        &url,
        serde_json::json!({ "type": "metaAndAssetCtxs" }),
    )?;
    let snapshot = parse_hyperliquid_ticker(pair, &body)?;
    Ok(snapshot_with_candles(
        snapshot,
        cached_or_fetch_candles(agent, pair, candle_cache, fetch_hyperliquid_candles),
    ))
}

fn snapshot_with_candles(
    mut snapshot: MarketSnapshot,
    candles: CandleFetchOutcome,
) -> MarketFetchOutcome {
    let CandleFetchOutcome {
        candles,
        fetched_at,
        error,
        new_failure,
    } = candles;
    snapshot.chart_closes_24h = candles.iter().map(|candle| candle.close).collect();
    snapshot.chart_candles_24h = candles;
    snapshot.chart_updated_at = fetched_at;
    snapshot.chart_error = error.clone();
    MarketFetchOutcome {
        snapshot,
        warning: if new_failure {
            error.map(|error| format!("chart update failed: {error}"))
        } else {
            None
        },
    }
}

fn ensure_spot_pair(pair: &MarketPair) -> Result<()> {
    if pair.market_type != MarketType::Spot {
        bail!("{} only supports spot pairs", pair.source.label());
    }
    Ok(())
}

fn get_json(agent: &ureq::Agent, url: &str) -> Result<String> {
    agent
        .get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .with_context(|| format!("request failed: {url}"))?
        .into_string()
        .context("failed to read response body")
}

fn post_json(agent: &ureq::Agent, url: &str, body: serde_json::Value) -> Result<String> {
    agent
        .post(url)
        .set("User-Agent", USER_AGENT)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .with_context(|| format!("request failed: {url}"))?
        .into_string()
        .context("failed to read response body")
}

fn cached_or_fetch_candles(
    agent: &ureq::Agent,
    pair: &MarketPair,
    candle_cache: &mut CandleCache,
    fetch: fn(&ureq::Agent, &MarketPair) -> Result<Vec<MarketCandle>>,
) -> CandleFetchOutcome {
    cached_or_fetch_candles_at(agent, pair, candle_cache, fetch, Instant::now())
}

fn cached_or_fetch_candles_at(
    agent: &ureq::Agent,
    pair: &MarketPair,
    candle_cache: &mut CandleCache,
    fetch: fn(&ureq::Agent, &MarketPair) -> Result<Vec<MarketCandle>>,
    now: Instant,
) -> CandleFetchOutcome {
    let key = pair.key();
    if let Some(entry) = candle_cache.get(&key) {
        if entry.fetched_at.is_some_and(|fetched_at| {
            now.saturating_duration_since(fetched_at) < CANDLE_REFRESH_INTERVAL
        }) {
            return CandleFetchOutcome {
                candles: entry.candles.clone(),
                fetched_at: entry.fetched_at,
                error: None,
                new_failure: false,
            };
        }
        if entry.retry_at.is_some_and(|retry_at| now < retry_at) {
            return CandleFetchOutcome {
                candles: entry.candles.clone(),
                fetched_at: entry.fetched_at,
                error: entry.last_error.clone(),
                new_failure: false,
            };
        }
    }

    let fetched = fetch(agent, pair).and_then(|candles| {
        ensure_usable_candles(&candles)?;
        Ok(candles)
    });
    match fetched {
        Ok(candles) => {
            candle_cache.insert(
                key,
                CandleCacheEntry {
                    candles: candles.clone(),
                    fetched_at: Some(now),
                    retry_at: None,
                    consecutive_failures: 0,
                    last_error: None,
                },
            );
            CandleFetchOutcome {
                candles,
                fetched_at: Some(now),
                error: None,
                new_failure: false,
            }
        }
        Err(error) => {
            let error = error.to_string();
            let entry = candle_cache.entry(key).or_insert_with(|| CandleCacheEntry {
                candles: Vec::new(),
                fetched_at: None,
                retry_at: None,
                consecutive_failures: 0,
                last_error: None,
            });
            entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
            entry.retry_at = Some(now + candle_failure_backoff(entry.consecutive_failures));
            entry.last_error = Some(error.clone());
            CandleFetchOutcome {
                candles: entry.candles.clone(),
                fetched_at: entry.fetched_at,
                error: Some(error),
                new_failure: true,
            }
        }
    }
}

fn candle_failure_backoff(consecutive_failures: u32) -> Duration {
    let multiplier = 1_u32
        .checked_shl(consecutive_failures.saturating_sub(1).min(6))
        .unwrap_or(u32::MAX);
    CANDLE_REFRESH_INTERVAL
        .saturating_mul(multiplier)
        .min(MAX_CANDLE_FAILURE_BACKOFF)
}

fn ensure_usable_candles(candles: &[MarketCandle]) -> Result<()> {
    if candles.len() < 2 {
        bail!("candle response did not include enough 24h OHLC data");
    }
    for candle in candles {
        let prices = [candle.open, candle.high, candle.low, candle.close];
        if candle.open_time_millis == 0
            || prices
                .iter()
                .any(|value| !value.is_finite() || *value <= 0.0)
            || candle.low > candle.open.min(candle.close)
            || candle.high < candle.open.max(candle.close)
            || candle.low > candle.high
        {
            bail!("candle response included invalid OHLC data");
        }
    }
    if candles
        .windows(2)
        .any(|pair| pair[0].open_time_millis >= pair[1].open_time_millis)
    {
        bail!("candle response timestamps were not strictly increasing");
    }
    Ok(())
}

fn fetch_binance_candles(agent: &ureq::Agent, pair: &MarketPair) -> Result<Vec<MarketCandle>> {
    let ticker_symbol = binance_ticker_symbol(pair);
    let url = format!(
        "{BINANCE_BASE_URL}/api/v3/klines?symbol={ticker_symbol}&interval=5m&limit={CANDLE_LIMIT_24H_5M}"
    );
    let body = get_json(agent, &url)?;
    parse_binance_klines(&body)
}

fn fetch_coinbase_candles(agent: &ureq::Agent, pair: &MarketPair) -> Result<Vec<MarketCandle>> {
    let product_id = coinbase_product_id(pair);
    let url = format!("{COINBASE_EXCHANGE_BASE_URL}/products/{product_id}/candles?granularity=300");
    let body = get_json(agent, &url)?;
    parse_coinbase_candles(&body)
}

fn fetch_okx_candles(agent: &ureq::Agent, pair: &MarketPair) -> Result<Vec<MarketCandle>> {
    let instrument_id = okx_instrument_id(pair);
    let url = format!(
        "{OKX_BASE_URL}/api/v5/market/candles?instId={instrument_id}&bar=5m&limit={CANDLE_LIMIT_24H_5M}"
    );
    let body = get_json(agent, &url)?;
    parse_okx_candles(&body)
}

fn fetch_hyperliquid_candles(agent: &ureq::Agent, pair: &MarketPair) -> Result<Vec<MarketCandle>> {
    let end_time = unix_epoch_millis()?;
    let start_time = end_time.saturating_sub(HYPERLIQUID_24H_MILLIS);
    let url = format!("{HYPERLIQUID_BASE_URL}/info");
    let body = post_json(
        agent,
        &url,
        serde_json::json!({
            "type": "candleSnapshot",
            "req": {
                "coin": pair.base,
                "interval": HYPERLIQUID_CANDLE_INTERVAL,
                "startTime": start_time,
                "endTime": end_time
            }
        }),
    )?;
    parse_hyperliquid_candles(&body)
}

fn unix_epoch_millis() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?
        .as_millis() as u64)
}

fn fetch_binance_symbol_catalog(agent: &ureq::Agent) -> Result<Vec<SymbolCatalogEntry>> {
    let url =
        format!("{BINANCE_BASE_URL}/api/v3/exchangeInfo?permissions=SPOT&symbolStatus=TRADING");
    let body = get_json(agent, &url)?;
    parse_binance_symbol_catalog(&body)
}

fn fetch_coinbase_symbol_catalog(agent: &ureq::Agent) -> Result<Vec<SymbolCatalogEntry>> {
    let url = format!("{COINBASE_EXCHANGE_BASE_URL}/products");
    let body = get_json(agent, &url)?;
    parse_coinbase_symbol_catalog(&body)
}

fn fetch_okx_symbol_catalog(agent: &ureq::Agent) -> Result<Vec<SymbolCatalogEntry>> {
    let url = format!("{OKX_BASE_URL}/api/v5/public/instruments?instType=SPOT");
    let body = get_json(agent, &url)?;
    parse_okx_symbol_catalog(&body)
}

fn fetch_hyperliquid_symbol_catalog(agent: &ureq::Agent) -> Result<Vec<SymbolCatalogEntry>> {
    let url = format!("{HYPERLIQUID_BASE_URL}/info");
    let body = post_json(
        agent,
        &url,
        serde_json::json!({ "type": "metaAndAssetCtxs" }),
    )?;
    parse_hyperliquid_symbol_catalog(&body)
}

fn combine_symbol_catalogs(catalogs: Vec<Vec<SymbolCatalogEntry>>) -> SymbolCatalog {
    let mut entries = BTreeMap::<String, SymbolCatalogEntry>::new();
    for entry in catalogs.into_iter().flatten() {
        entries.entry(entry.key.clone()).or_insert(entry);
    }
    SymbolCatalog {
        entries: entries.into_values().collect(),
    }
}

fn binance_ticker_symbol(pair: &MarketPair) -> String {
    format!("{}{}", pair.base, pair.quote)
}

fn coinbase_product_id(pair: &MarketPair) -> String {
    format!("{}-{}", pair.base, pair.quote)
}

fn okx_instrument_id(pair: &MarketPair) -> String {
    format!("{}-{}", pair.base, pair.quote)
}

fn catalog_entry(
    source: MarketDataSource,
    market_type: MarketType,
    base: impl Into<String>,
    quote: impl Into<String>,
) -> Option<SymbolCatalogEntry> {
    let pair = MarketPair::new(source, market_type, base, quote)?;
    Some(SymbolCatalogEntry {
        key: pair.key(),
        base: pair.base,
        quote: pair.quote,
        source: pair.source,
        market_type: pair.market_type,
    })
}

#[derive(Debug, Deserialize)]
struct BinanceTicker {
    #[serde(rename = "lastPrice")]
    last_price: String,
    #[serde(rename = "priceChangePercent")]
    price_change_percent: String,
}

#[derive(Debug, Deserialize)]
struct BinanceExchangeInfo {
    symbols: Vec<BinanceExchangeSymbol>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceExchangeSymbol {
    status: String,
    base_asset: String,
    quote_asset: String,
    is_spot_trading_allowed: bool,
}

fn parse_binance_symbol_catalog(body: &str) -> Result<Vec<SymbolCatalogEntry>> {
    let response = serde_json::from_str::<BinanceExchangeInfo>(body)
        .context("failed to parse Binance exchangeInfo")?;
    Ok(unique_catalog_entries(
        response.symbols.into_iter().filter_map(|symbol| {
            (symbol.status == "TRADING"
                && stable_quote(&symbol.quote_asset)
                && symbol.is_spot_trading_allowed)
                .then(|| {
                    catalog_entry(
                        MarketDataSource::Binance,
                        MarketType::Spot,
                        symbol.base_asset,
                        symbol.quote_asset,
                    )
                })
                .flatten()
        }),
    ))
}

fn parse_binance_ticker(pair: &MarketPair, body: &str) -> Result<MarketSnapshot> {
    let ticker =
        serde_json::from_str::<BinanceTicker>(body).context("failed to parse Binance ticker")?;
    market_snapshot(
        pair,
        MarketDataSource::Binance,
        parse_positive_decimal(&ticker.last_price, "Binance lastPrice")?,
        parse_decimal(&ticker.price_change_percent, "Binance priceChangePercent")?,
    )
}

fn parse_binance_klines(body: &str) -> Result<Vec<MarketCandle>> {
    let rows = serde_json::from_str::<Vec<Vec<serde_json::Value>>>(body)
        .context("failed to parse Binance klines")?;
    let mut candles = rows
        .iter()
        .map(|row| {
            Ok(MarketCandle {
                open_time_millis: parse_json_u64(row.first(), "Binance kline open time")?,
                open: parse_json_decimal(row.get(1), "Binance kline open")?,
                high: parse_json_decimal(row.get(2), "Binance kline high")?,
                low: parse_json_decimal(row.get(3), "Binance kline low")?,
                close: parse_json_decimal(row.get(4), "Binance kline close")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    candles.sort_by_key(|candle| candle.open_time_millis);
    Ok(candles)
}

#[derive(Debug, Deserialize)]
struct CoinbaseStats {
    open: String,
    last: String,
}

#[derive(Debug, Deserialize)]
struct CoinbaseProduct {
    base_currency: String,
    quote_currency: String,
    status: String,
    #[serde(default)]
    trading_disabled: bool,
}

fn parse_coinbase_symbol_catalog(body: &str) -> Result<Vec<SymbolCatalogEntry>> {
    let products = serde_json::from_str::<Vec<CoinbaseProduct>>(body)
        .context("failed to parse Coinbase products")?;
    Ok(unique_catalog_entries(products.into_iter().filter_map(
        |product| {
            (product.status == "online"
                && !product.trading_disabled
                && coinbase_quote(&product.quote_currency))
            .then(|| {
                catalog_entry(
                    MarketDataSource::Coinbase,
                    MarketType::Spot,
                    product.base_currency,
                    product.quote_currency,
                )
            })
            .flatten()
        },
    )))
}

fn parse_coinbase_stats(pair: &MarketPair, body: &str) -> Result<MarketSnapshot> {
    let stats =
        serde_json::from_str::<CoinbaseStats>(body).context("failed to parse Coinbase stats")?;
    let price = parse_positive_decimal(&stats.last, "Coinbase last")?;
    let open_24h = parse_positive_decimal(&stats.open, "Coinbase open")?;

    market_snapshot(
        pair,
        MarketDataSource::Coinbase,
        price,
        percent_change(price, open_24h, "Coinbase 24h change")?,
    )
}

fn parse_coinbase_candles(body: &str) -> Result<Vec<MarketCandle>> {
    let rows = serde_json::from_str::<Vec<Vec<serde_json::Value>>>(body)
        .context("failed to parse Coinbase candles")?;
    let mut candles = rows
        .iter()
        .map(|row| {
            Ok(MarketCandle {
                open_time_millis: parse_json_u64(row.first(), "Coinbase candle time")?
                    .checked_mul(1_000)
                    .ok_or_else(|| anyhow!("Coinbase candle time overflow"))?,
                low: parse_json_decimal(row.get(1), "Coinbase candle low")?,
                high: parse_json_decimal(row.get(2), "Coinbase candle high")?,
                open: parse_json_decimal(row.get(3), "Coinbase candle open")?,
                close: parse_json_decimal(row.get(4), "Coinbase candle close")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    candles.sort_by_key(|candle| candle.open_time_millis);
    Ok(candles)
}

#[derive(Debug, Deserialize)]
struct OkxTickerResponse {
    code: String,
    msg: String,
    data: Vec<OkxTicker>,
}

#[derive(Debug, Deserialize)]
struct OkxTicker {
    last: String,
    #[serde(rename = "open24h")]
    open_24h: String,
}

#[derive(Debug, Deserialize)]
struct OkxCandlesResponse {
    code: String,
    msg: String,
    data: Vec<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct OkxInstrumentsResponse {
    code: String,
    msg: String,
    data: Vec<OkxInstrument>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxInstrument {
    base_ccy: String,
    quote_ccy: String,
    state: String,
}

fn parse_okx_symbol_catalog(body: &str) -> Result<Vec<SymbolCatalogEntry>> {
    let response = serde_json::from_str::<OkxInstrumentsResponse>(body)
        .context("failed to parse OKX instruments")?;
    if response.code != "0" {
        bail!("OKX returned {}: {}", response.code, response.msg);
    }

    Ok(unique_catalog_entries(
        response.data.into_iter().filter_map(|instrument| {
            (stable_quote(&instrument.quote_ccy) && instrument.state == "live")
                .then(|| {
                    catalog_entry(
                        MarketDataSource::Okx,
                        MarketType::Spot,
                        instrument.base_ccy,
                        instrument.quote_ccy,
                    )
                })
                .flatten()
        }),
    ))
}

fn parse_okx_candles(body: &str) -> Result<Vec<MarketCandle>> {
    let response =
        serde_json::from_str::<OkxCandlesResponse>(body).context("failed to parse OKX candles")?;
    if response.code != "0" {
        bail!("OKX returned {}: {}", response.code, response.msg);
    }

    let mut candles = response
        .data
        .iter()
        .map(|row| {
            Ok(MarketCandle {
                open_time_millis: parse_u64(
                    row.first().map(String::as_str).unwrap_or(""),
                    "OKX candle time",
                )?,
                open: parse_decimal(
                    row.get(1).map(String::as_str).unwrap_or(""),
                    "OKX candle open",
                )?,
                high: parse_decimal(
                    row.get(2).map(String::as_str).unwrap_or(""),
                    "OKX candle high",
                )?,
                low: parse_decimal(
                    row.get(3).map(String::as_str).unwrap_or(""),
                    "OKX candle low",
                )?,
                close: parse_decimal(
                    row.get(4).map(String::as_str).unwrap_or(""),
                    "OKX candle close",
                )?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    candles.sort_by_key(|candle| candle.open_time_millis);
    Ok(candles)
}

fn parse_okx_ticker(pair: &MarketPair, body: &str) -> Result<MarketSnapshot> {
    let response =
        serde_json::from_str::<OkxTickerResponse>(body).context("failed to parse OKX ticker")?;
    if response.code != "0" {
        bail!("OKX returned {}: {}", response.code, response.msg);
    }
    let ticker = response
        .data
        .first()
        .ok_or_else(|| anyhow!("OKX response did not include ticker data"))?;
    let price = parse_positive_decimal(&ticker.last, "OKX last")?;
    let open_24h = parse_positive_decimal(&ticker.open_24h, "OKX open24h")?;

    market_snapshot(
        pair,
        MarketDataSource::Okx,
        price,
        percent_change(price, open_24h, "OKX 24h change")?,
    )
}

#[derive(Debug, Deserialize)]
struct HyperliquidPerpMeta {
    universe: Vec<HyperliquidPerpAsset>,
}

#[derive(Debug, Deserialize)]
struct HyperliquidPerpAsset {
    name: String,
}

#[derive(Debug, Deserialize)]
struct HyperliquidAssetCtx {
    #[serde(rename = "markPx")]
    mark_px: Option<String>,
    #[serde(rename = "midPx")]
    mid_px: Option<String>,
    #[serde(rename = "prevDayPx")]
    prev_day_px: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HyperliquidCandle {
    t: u64,
    o: String,
    h: String,
    l: String,
    c: String,
}

fn parse_hyperliquid_symbol_catalog(body: &str) -> Result<Vec<SymbolCatalogEntry>> {
    let (meta, _contexts) =
        serde_json::from_str::<(HyperliquidPerpMeta, Vec<HyperliquidAssetCtx>)>(body)
            .context("failed to parse Hyperliquid metaAndAssetCtxs")?;
    Ok(unique_catalog_entries(
        meta.universe.into_iter().filter_map(|asset| {
            catalog_entry(
                MarketDataSource::Hyperliquid,
                MarketType::Perp,
                asset.name,
                "USDC",
            )
        }),
    ))
}

fn parse_hyperliquid_ticker(pair: &MarketPair, body: &str) -> Result<MarketSnapshot> {
    let (meta, contexts) =
        serde_json::from_str::<(HyperliquidPerpMeta, Vec<HyperliquidAssetCtx>)>(body)
            .context("failed to parse Hyperliquid metaAndAssetCtxs")?;
    let index = meta
        .universe
        .iter()
        .position(|asset| asset.name.eq_ignore_ascii_case(&pair.base))
        .ok_or_else(|| anyhow!("Hyperliquid did not include {}", pair.base))?;
    let context = contexts
        .get(index)
        .ok_or_else(|| anyhow!("Hyperliquid did not include context for {}", pair.base))?;
    let price = context
        .mark_px
        .as_deref()
        .or(context.mid_px.as_deref())
        .ok_or_else(|| anyhow!("Hyperliquid context did not include a mark price"))
        .and_then(|price| parse_positive_decimal(price, "Hyperliquid markPx"))?;
    let prev_day = context
        .prev_day_px
        .as_deref()
        .ok_or_else(|| anyhow!("Hyperliquid context did not include prevDayPx"))
        .and_then(|price| parse_positive_decimal(price, "Hyperliquid prevDayPx"))?;

    market_snapshot(
        pair,
        MarketDataSource::Hyperliquid,
        price,
        percent_change(price, prev_day, "Hyperliquid 24h change")?,
    )
}

fn parse_hyperliquid_candles(body: &str) -> Result<Vec<MarketCandle>> {
    let rows = serde_json::from_str::<Vec<HyperliquidCandle>>(body)
        .context("failed to parse Hyperliquid candles")?;
    let mut candles = rows
        .iter()
        .map(|row| {
            Ok(MarketCandle {
                open_time_millis: row.t,
                open: parse_decimal(&row.o, "Hyperliquid candle open")?,
                high: parse_decimal(&row.h, "Hyperliquid candle high")?,
                low: parse_decimal(&row.l, "Hyperliquid candle low")?,
                close: parse_decimal(&row.c, "Hyperliquid candle close")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    candles.sort_by_key(|candle| candle.open_time_millis);
    Ok(candles)
}

fn stable_quote(quote: &str) -> bool {
    matches!(quote.trim().to_ascii_uppercase().as_str(), "USDT" | "USDC")
}

fn coinbase_quote(quote: &str) -> bool {
    matches!(
        quote.trim().to_ascii_uppercase().as_str(),
        "USD" | "USDT" | "USDC"
    )
}

fn unique_catalog_entries(
    entries: impl IntoIterator<Item = SymbolCatalogEntry>,
) -> Vec<SymbolCatalogEntry> {
    entries.into_iter().fold(Vec::new(), |mut unique, entry| {
        if !unique.iter().any(|candidate| candidate.key == entry.key) {
            unique.push(entry);
        }
        unique
    })
}

fn parse_json_decimal(value: Option<&serde_json::Value>, field: &str) -> Result<f64> {
    match value {
        Some(serde_json::Value::String(value)) => parse_decimal(value, field),
        Some(serde_json::Value::Number(value)) => value
            .as_f64()
            .ok_or_else(|| anyhow!("failed to parse {field}: {value}"))
            .and_then(|value| ensure_finite_decimal(value, field)),
        Some(value) => bail!("failed to parse {field}: {value}"),
        None => bail!("missing {field}"),
    }
}

fn parse_json_u64(value: Option<&serde_json::Value>, field: &str) -> Result<u64> {
    match value {
        Some(serde_json::Value::String(value)) => parse_u64(value, field),
        Some(serde_json::Value::Number(value)) => value
            .as_u64()
            .ok_or_else(|| anyhow!("failed to parse {field}: {value}")),
        Some(value) => bail!("failed to parse {field}: {value}"),
        None => bail!("missing {field}"),
    }
}

fn parse_decimal(value: &str, field: &str) -> Result<f64> {
    value
        .parse::<f64>()
        .with_context(|| format!("failed to parse {field}: {value}"))
        .and_then(|value| ensure_finite_decimal(value, field))
}

fn ensure_finite_decimal(value: f64, field: &str) -> Result<f64> {
    if !value.is_finite() {
        bail!("{field} must be finite");
    }
    Ok(value)
}

fn parse_positive_decimal(value: &str, field: &str) -> Result<f64> {
    let value = parse_decimal(value, field)?;
    if value <= 0.0 {
        bail!("{field} must be greater than zero");
    }
    Ok(value)
}

fn percent_change(current: f64, baseline: f64, field: &str) -> Result<f64> {
    ensure_finite_decimal(((current - baseline) / baseline) * 100.0, field)
}

fn market_snapshot(
    pair: &MarketPair,
    source: MarketDataSource,
    price: f64,
    change_percent_24h: f64,
) -> Result<MarketSnapshot> {
    let price = ensure_finite_decimal(price, "ticker price")?;
    if price <= 0.0 {
        bail!("ticker price must be greater than zero");
    }
    let change_percent_24h =
        ensure_finite_decimal(change_percent_24h, "ticker 24h change percent")?;
    Ok(MarketSnapshot {
        symbol: pair.key(),
        price,
        change_percent_24h,
        chart_closes_24h: Vec::new(),
        chart_candles_24h: Vec::new(),
        chart_updated_at: None,
        chart_error: None,
        source,
    })
}

fn parse_u64(value: &str, field: &str) -> Result<u64> {
    value
        .parse::<u64>()
        .with_context(|| format!("failed to parse {field}: {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_hud_core::{default_enabled_market_sources, DEFAULT_REFRESH_INTERVAL_SECONDS};

    fn pair(raw: &str) -> MarketPair {
        parse_market_pair(raw).unwrap()
    }

    fn snapshot(symbol: &str) -> MarketSnapshot {
        MarketSnapshot {
            symbol: symbol.to_string(),
            price: 100.0,
            change_percent_24h: 1.0,
            chart_closes_24h: Vec::new(),
            chart_candles_24h: Vec::new(),
            chart_updated_at: None,
            chart_error: None,
            source: MarketDataSource::Binance,
        }
    }

    fn candle(open_time_millis: u64, open: f64, high: f64, low: f64, close: f64) -> MarketCandle {
        MarketCandle {
            open_time_millis,
            open,
            high,
            low,
            close,
        }
    }

    fn unavailable_candles(_: &ureq::Agent, _: &MarketPair) -> Result<Vec<MarketCandle>> {
        Err(anyhow!("candles unavailable"))
    }

    fn unexpected_candle_fetch(_: &ureq::Agent, _: &MarketPair) -> Result<Vec<MarketCandle>> {
        panic!("candle fetch should remain suppressed during retry backoff")
    }

    fn feed_config() -> MarketFeedConfig {
        MarketFeedConfig {
            symbols: vec!["binance:spot:BTC/USDT".to_string()],
            provider: MarketProviderPreference::Auto,
            refresh_interval_seconds: DEFAULT_REFRESH_INTERVAL_SECONDS,
            enabled_sources: default_enabled_market_sources(),
            proxy_url: None,
        }
    }

    #[test]
    fn fixed_event_feed_never_starts_network_work() {
        let feed = MarketFeed::from_events(vec![
            MarketEvent::Snapshot(snapshot("binance:spot:BTC/USDT")),
            MarketEvent::Error("offline fixture".to_string()),
        ]);

        assert!(matches!(feed.try_recv(), Ok(MarketEvent::Snapshot(_))));
        assert!(
            matches!(feed.try_recv(), Ok(MarketEvent::Error(error)) if error == "offline fixture")
        );
        assert!(matches!(feed.try_recv(), Err(TryRecvError::Disconnected)));
    }

    #[test]
    fn market_feed_wait_is_interrupted_by_stop_control() {
        let config = Arc::new(Mutex::new(feed_config()));
        let observed = market_feed_config_snapshot(&config);
        let (control_sender, control_receiver) = mpsc::channel();
        control_sender.send(MarketFeedControl::Stop).unwrap();

        assert!(matches!(
            wait_for_market_feed(
                &control_receiver,
                &config,
                &observed,
                Duration::from_secs(60)
            ),
            MarketFeedWait::Stop
        ));
    }

    #[test]
    fn market_feed_wait_detects_config_changes_without_waiting_for_refresh() {
        let config = Arc::new(Mutex::new(feed_config()));
        let observed = market_feed_config_snapshot(&config);
        config.lock().unwrap().symbols = vec!["okx:spot:ETH/USDT".to_string()];
        let (_control_sender, control_receiver) = mpsc::channel();

        assert!(matches!(
            wait_for_market_feed(
                &control_receiver,
                &config,
                &observed,
                Duration::from_secs(60)
            ),
            MarketFeedWait::ConfigChanged
        ));
    }

    #[test]
    fn market_feed_failure_backoff_is_deterministic_and_bounded() {
        let base = Duration::from_secs(5);
        for (failures, expected_seconds) in [(1, 10), (2, 20), (3, 40), (4, 60), (20, 60)] {
            assert_eq!(
                failure_backoff(base, failures),
                Duration::from_secs(expected_seconds),
                "unexpected delay after {failures} failures"
            );
        }
    }

    #[test]
    fn candle_failure_backoff_starts_at_refresh_interval_and_is_bounded() {
        for (failures, expected_minutes) in [(1, 5), (2, 10), (3, 20), (4, 40), (5, 60), (20, 60)] {
            assert_eq!(
                candle_failure_backoff(failures),
                Duration::from_secs(expected_minutes * 60),
                "unexpected candle retry delay after {failures} failures"
            );
        }
    }

    #[test]
    fn partial_market_batch_keeps_snapshots_and_reports_errors() {
        let batch = market_batch(
            vec![snapshot("binance:spot:BTC/USDT")],
            vec!["okx:spot:ETH/USDT: timeout".to_string()],
        )
        .unwrap();

        assert_eq!(batch.snapshots.len(), 1);
        assert_eq!(batch.errors, vec!["okx:spot:ETH/USDT: timeout"]);
    }

    #[test]
    fn candle_failure_does_not_discard_ticker_snapshot() {
        let outcome = snapshot_with_candles(
            snapshot("binance:spot:BTC/USDT"),
            CandleFetchOutcome {
                candles: Vec::new(),
                fetched_at: None,
                error: Some("candles unavailable".to_string()),
                new_failure: true,
            },
        );

        assert_eq!(outcome.snapshot.price, 100.0);
        assert!(outcome.snapshot.chart_closes_24h.is_empty());
        assert!(outcome.snapshot.chart_candles_24h.is_empty());
        assert_eq!(outcome.snapshot.chart_updated_at, None);
        assert_eq!(
            outcome.snapshot.chart_error.as_deref(),
            Some("candles unavailable")
        );
        assert_eq!(
            outcome.warning.as_deref(),
            Some("chart update failed: candles unavailable")
        );
    }

    #[test]
    fn failed_stale_candle_refresh_preserves_original_freshness_and_reports_error() {
        let pair = pair("binance:spot:BTC/USDT");
        let fetched_at = Instant::now();
        let now = fetched_at + CANDLE_REFRESH_INTERVAL + Duration::from_secs(1);
        let cached_candles = vec![
            candle(1, 100.0, 105.0, 95.0, 101.5),
            candle(2, 101.5, 106.0, 99.0, 104.25),
        ];
        let mut cache = CandleCache::from([(
            pair.key(),
            CandleCacheEntry {
                candles: cached_candles.clone(),
                fetched_at: Some(fetched_at),
                retry_at: None,
                consecutive_failures: 0,
                last_error: None,
            },
        )]);

        let outcome = cached_or_fetch_candles_at(
            &build_agent(None).unwrap(),
            &pair,
            &mut cache,
            unavailable_candles,
            now,
        );

        assert_eq!(outcome.candles, cached_candles);
        assert_eq!(outcome.fetched_at, Some(fetched_at));
        assert_eq!(outcome.error.as_deref(), Some("candles unavailable"));
        assert_eq!(cache[&pair.key()].fetched_at, Some(fetched_at));
        assert_eq!(
            cache[&pair.key()].retry_at,
            Some(now + CANDLE_REFRESH_INTERVAL)
        );

        let suppressed = cached_or_fetch_candles_at(
            &build_agent(None).unwrap(),
            &pair,
            &mut cache,
            unexpected_candle_fetch,
            now + Duration::from_secs(30),
        );
        assert_eq!(suppressed.candles, cached_candles);
        assert_eq!(suppressed.fetched_at, Some(fetched_at));
        assert_eq!(suppressed.error.as_deref(), Some("candles unavailable"));
        assert!(!suppressed.new_failure);

        let suppressed_outcome =
            snapshot_with_candles(snapshot("binance:spot:BTC/USDT"), suppressed);
        assert_eq!(
            suppressed_outcome.snapshot.chart_error.as_deref(),
            Some("candles unavailable")
        );
        assert_eq!(suppressed_outcome.warning, None);
    }

    #[test]
    fn parses_binance_24hr_ticker() {
        let snapshot = parse_binance_ticker(
            &pair("binance:spot:BTC/USDT"),
            r#"{"lastPrice":"106800.12","priceChangePercent":"1.234"}"#,
        )
        .unwrap();

        assert_eq!(snapshot.symbol, "binance:spot:BTC/USDT");
        assert_eq!(snapshot.source, MarketDataSource::Binance);
        assert_eq!(snapshot.price, 106800.12);
        assert_eq!(snapshot.change_percent_24h, 1.234);
        assert!(snapshot.chart_closes_24h.is_empty());
    }

    #[test]
    fn ticker_parsers_reject_non_finite_and_non_positive_values() {
        let invalid = [
            (
                "Binance non-finite price",
                parse_binance_ticker(
                    &pair("binance:spot:BTC/USDT"),
                    r#"{"lastPrice":"NaN","priceChangePercent":"1.0"}"#,
                ),
            ),
            (
                "Binance non-finite change",
                parse_binance_ticker(
                    &pair("binance:spot:BTC/USDT"),
                    r#"{"lastPrice":"100","priceChangePercent":"inf"}"#,
                ),
            ),
            (
                "Coinbase zero price",
                parse_coinbase_stats(
                    &pair("coinbase:spot:BTC/USD"),
                    r#"{"open":"100","last":"0"}"#,
                ),
            ),
            (
                "Coinbase zero baseline",
                parse_coinbase_stats(
                    &pair("coinbase:spot:BTC/USD"),
                    r#"{"open":"0","last":"100"}"#,
                ),
            ),
            (
                "OKX negative price",
                parse_okx_ticker(
                    &pair("okx:spot:ETH/USDT"),
                    r#"{"code":"0","msg":"","data":[{"last":"-1","open24h":"100"}]}"#,
                ),
            ),
            (
                "OKX overflowing change",
                parse_okx_ticker(
                    &pair("okx:spot:ETH/USDT"),
                    r#"{"code":"0","msg":"","data":[{"last":"1e308","open24h":"1e-308"}]}"#,
                ),
            ),
            (
                "Hyperliquid non-finite baseline",
                parse_hyperliquid_ticker(
                    &pair("hyperliquid:perp:BTC/USDC"),
                    r#"[{"universe":[{"name":"BTC"}]},[{"markPx":"100","prevDayPx":"NaN"}]]"#,
                ),
            ),
        ];

        for (label, result) in invalid {
            assert!(result.is_err(), "{label} should be rejected");
        }
    }

    #[test]
    fn parses_okx_ticker_and_derives_24hr_change() {
        let snapshot = parse_okx_ticker(
            &pair("okx:spot:ETH/USDT"),
            r#"{"code":"0","msg":"","data":[{"last":"3420","open24h":"3400"}]}"#,
        )
        .unwrap();

        assert_eq!(snapshot.symbol, "okx:spot:ETH/USDT");
        assert_eq!(snapshot.source, MarketDataSource::Okx);
        assert_eq!(snapshot.price, 3420.0);
        assert!((snapshot.change_percent_24h - 0.588235294).abs() < 0.000001);
        assert!(snapshot.chart_closes_24h.is_empty());
    }

    #[test]
    fn parses_hyperliquid_ticker_and_derives_24hr_change() {
        let snapshot = parse_hyperliquid_ticker(
            &pair("hyperliquid:perp:BTC/USDC"),
            r#"[
                {"universe":[{"name":"ETH"},{"name":"BTC"}]},
                [
                    {"markPx":"3420","midPx":"3421","prevDayPx":"3400"},
                    {"markPx":"106800","midPx":"106801","prevDayPx":"106000"}
                ]
            ]"#,
        )
        .unwrap();

        assert_eq!(snapshot.symbol, "hyperliquid:perp:BTC/USDC");
        assert_eq!(snapshot.source, MarketDataSource::Hyperliquid);
        assert_eq!(snapshot.price, 106800.0);
        assert!((snapshot.change_percent_24h - 0.754716981).abs() < 0.000001);
    }

    #[test]
    fn parses_binance_5m_kline_ohlc() {
        let candles = parse_binance_klines(
            r#"[
                [1499040000000,"100.0","105.0","95.0","101.5","12.0",1499040299999,"0",0,"0","0","0"],
                [1499040300000,"101.5","106.0","99.0","104.25","10.0",1499040599999,"0",0,"0","0","0"]
            ]"#,
        )
        .unwrap();

        assert_eq!(
            candles,
            vec![
                candle(1_499_040_000_000, 100.0, 105.0, 95.0, 101.5),
                candle(1_499_040_300_000, 101.5, 106.0, 99.0, 104.25),
            ]
        );
    }

    #[test]
    fn candle_validation_rejects_invalid_ohlc_tables() {
        let valid = vec![
            candle(1, 100.0, 105.0, 95.0, 101.5),
            candle(2, 101.5, 106.0, 99.0, 104.25),
        ];
        assert!(ensure_usable_candles(&valid).is_ok());

        let invalid = vec![
            ("too few rows", vec![valid[0]]),
            (
                "zero timestamp",
                vec![candle(0, 100.0, 105.0, 95.0, 101.5), valid[1]],
            ),
            (
                "NaN open",
                vec![candle(1, f64::NAN, 105.0, 95.0, 101.5), valid[1]],
            ),
            (
                "infinite high",
                vec![candle(1, 100.0, f64::INFINITY, 95.0, 101.5), valid[1]],
            ),
            (
                "zero low",
                vec![candle(1, 100.0, 105.0, 0.0, 101.5), valid[1]],
            ),
            (
                "negative close",
                vec![candle(1, 100.0, 105.0, 95.0, -1.0), valid[1]],
            ),
            (
                "low above body",
                vec![candle(1, 100.0, 105.0, 100.5, 101.5), valid[1]],
            ),
            (
                "high below body",
                vec![candle(1, 100.0, 101.0, 95.0, 101.5), valid[1]],
            ),
            (
                "low above high",
                vec![candle(1, 100.0, 99.0, 101.0, 100.0), valid[1]],
            ),
            (
                "duplicate timestamps",
                vec![valid[0], candle(1, 101.5, 106.0, 99.0, 104.25)],
            ),
            ("descending timestamps", vec![valid[1], valid[0]]),
        ];

        for (label, candles) in invalid {
            assert!(
                ensure_usable_candles(&candles).is_err(),
                "{label} should be rejected"
            );
        }
    }

    #[test]
    fn parses_binance_symbol_catalog_for_trading_stable_spot_pairs() {
        let entries = parse_binance_symbol_catalog(
            r#"{
                "symbols": [
                    {"symbol":"BTCUSDT","status":"TRADING","baseAsset":"BTC","quoteAsset":"USDT","isSpotTradingAllowed":true},
                    {"symbol":"ETHUSDC","status":"TRADING","baseAsset":"ETH","quoteAsset":"USDC","isSpotTradingAllowed":true},
                    {"symbol":"ETHUSDT","status":"BREAK","baseAsset":"ETH","quoteAsset":"USDT","isSpotTradingAllowed":true},
                    {"symbol":"BNBBTC","status":"TRADING","baseAsset":"BNB","quoteAsset":"BTC","isSpotTradingAllowed":true},
                    {"symbol":"SOLUSDT","status":"TRADING","baseAsset":"SOL","quoteAsset":"USDT","isSpotTradingAllowed":false},
                    {"symbol":"btcusdt","status":"TRADING","baseAsset":"btc","quoteAsset":"USDT","isSpotTradingAllowed":true}
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            vec!["binance:spot:BTC/USDT", "binance:spot:ETH/USDC"]
        );
    }

    #[test]
    fn parses_coinbase_stats_and_derives_24hr_change() {
        let snapshot = parse_coinbase_stats(
            &pair("coinbase:spot:BTC/USD"),
            r#"{"open":"100000.00","high":"108000.00","low":"99000.00","last":"106800.00","volume":"12.0"}"#,
        )
        .unwrap();

        assert_eq!(snapshot.symbol, "coinbase:spot:BTC/USD");
        assert_eq!(snapshot.source, MarketDataSource::Coinbase);
        assert_eq!(snapshot.price, 106800.0);
        assert!((snapshot.change_percent_24h - 6.8).abs() < 0.000001);
        assert!(snapshot.chart_closes_24h.is_empty());
    }

    #[test]
    fn parses_coinbase_5m_candles_oldest_first() {
        let candles = parse_coinbase_candles(
            r#"[
                [1499040300,99.0,106.0,101.5,104.25,10.0],
                [1499040000,95.0,105.0,100.0,101.5,12.0]
            ]"#,
        )
        .unwrap();

        assert_eq!(
            candles,
            vec![
                candle(1_499_040_000_000, 100.0, 105.0, 95.0, 101.5),
                candle(1_499_040_300_000, 101.5, 106.0, 99.0, 104.25),
            ]
        );
    }

    #[test]
    fn parses_coinbase_symbol_catalog_for_online_stable_spot_pairs() {
        let entries = parse_coinbase_symbol_catalog(
            r#"[
                {"id":"BTC-USD","base_currency":"BTC","quote_currency":"USD","status":"online","trading_disabled":false},
                {"id":"ETH-USDT","base_currency":"ETH","quote_currency":"USDT","status":"online","trading_disabled":false},
                {"id":"SOL-USDC","base_currency":"SOL","quote_currency":"USDC","status":"online","trading_disabled":false},
                {"id":"BTC-USDC","base_currency":"BTC","quote_currency":"USDC","status":"delisted","trading_disabled":true},
                {"id":"ETH-BTC","base_currency":"ETH","quote_currency":"BTC","status":"online","trading_disabled":false},
                {"id":"DOGE-USD","base_currency":"DOGE","quote_currency":"USD","status":"online","trading_disabled":true},
                {"id":"btc-usd","base_currency":"btc","quote_currency":"USD","status":"online","trading_disabled":false}
            ]"#,
        )
        .unwrap();

        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            vec![
                "coinbase:spot:BTC/USD",
                "coinbase:spot:ETH/USDT",
                "coinbase:spot:SOL/USDC"
            ]
        );
    }

    #[test]
    fn parses_okx_5m_candles_oldest_first() {
        let candles = parse_okx_candles(
            r#"{
                "code":"0",
                "msg":"",
                "data":[
                    ["1597026600000","101","110","100","108","1","2","3","1"],
                    ["1597026300000","100","105","95","101","1","2","3","1"]
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(
            candles,
            vec![
                candle(1_597_026_300_000, 100.0, 105.0, 95.0, 101.0),
                candle(1_597_026_600_000, 101.0, 110.0, 100.0, 108.0),
            ]
        );
    }

    #[test]
    fn parses_okx_symbol_catalog_for_live_stable_spot_pairs() {
        let entries = parse_okx_symbol_catalog(
            r#"{
                "code":"0",
                "msg":"",
                "data":[
                    {"instId":"BTC-USDT","baseCcy":"BTC","quoteCcy":"USDT","state":"live"},
                    {"instId":"ETH-USDT","baseCcy":"ETH","quoteCcy":"USDT","state":"suspend"},
                    {"instId":"SOL-USDC","baseCcy":"SOL","quoteCcy":"USDC","state":"live"},
                    {"instId":"btc-USDT","baseCcy":"btc","quoteCcy":"USDT","state":"live"}
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            vec!["okx:spot:BTC/USDT", "okx:spot:SOL/USDC"]
        );
    }

    #[test]
    fn parses_hyperliquid_catalog_as_usdc_perps() {
        let entries = parse_hyperliquid_symbol_catalog(
            r#"[
                {"universe":[{"name":"BTC"},{"name":"ETH"},{"name":"BTC"}]},
                [
                    {"markPx":"106800","prevDayPx":"106000"},
                    {"markPx":"3420","prevDayPx":"3400"},
                    {"markPx":"106800","prevDayPx":"106000"}
                ]
            ]"#,
        )
        .unwrap();

        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            vec!["hyperliquid:perp:BTC/USDC", "hyperliquid:perp:ETH/USDC"]
        );
    }

    #[test]
    fn parses_hyperliquid_candles() {
        let candles = parse_hyperliquid_candles(
            r#"[
                {"t":1,"T":2,"s":"BTC","i":"5m","o":"100","c":"101.5","h":"102","l":"99","v":"10","n":1},
                {"t":2,"T":3,"s":"BTC","i":"5m","o":"101.5","c":"104.25","h":"105","l":"100","v":"12","n":1}
            ]"#,
        )
        .unwrap();

        assert_eq!(
            candles,
            vec![
                candle(1, 100.0, 102.0, 99.0, 101.5),
                candle(2, 101.5, 105.0, 100.0, 104.25),
            ]
        );
    }

    #[test]
    fn combines_symbol_catalog_entries_by_key() {
        let catalog = combine_symbol_catalogs(vec![
            vec![
                catalog_entry(MarketDataSource::Binance, MarketType::Spot, "BTC", "USDT").unwrap(),
                catalog_entry(MarketDataSource::Okx, MarketType::Spot, "BTC", "USDT").unwrap(),
            ],
            vec![
                catalog_entry(MarketDataSource::Binance, MarketType::Spot, "BTC", "USDT").unwrap(),
                catalog_entry(
                    MarketDataSource::Hyperliquid,
                    MarketType::Perp,
                    "BTC",
                    "USDC",
                )
                .unwrap(),
            ],
        ]);

        assert_eq!(
            catalog
                .entries
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            vec![
                "binance:spot:BTC/USDT",
                "hyperliquid:perp:BTC/USDC",
                "okx:spot:BTC/USDT"
            ]
        );
    }

    #[test]
    fn partial_symbol_catalog_keeps_source_warnings() {
        let warning = SymbolCatalogWarning {
            source: MarketDataSource::Okx,
            message: "timeout".to_string(),
        };
        let fetched = finish_symbol_catalog_fetch(
            vec![vec![catalog_entry(
                MarketDataSource::Binance,
                MarketType::Spot,
                "BTC",
                "USDT",
            )
            .unwrap()]],
            vec![warning.clone()],
        )
        .unwrap();

        assert_eq!(fetched.catalog.entries.len(), 1);
        assert_eq!(fetched.warnings, vec![warning]);
    }

    #[test]
    fn empty_symbol_catalog_reports_every_source_warning() {
        let error = finish_symbol_catalog_fetch(
            Vec::new(),
            vec![
                SymbolCatalogWarning {
                    source: MarketDataSource::Binance,
                    message: "blocked".to_string(),
                },
                SymbolCatalogWarning {
                    source: MarketDataSource::Okx,
                    message: "timeout".to_string(),
                },
            ],
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("Binance: blocked"));
        assert!(error.contains("OKX: timeout"));
    }

    #[test]
    fn empty_symbol_catalog_without_source_warnings_has_a_useful_error() {
        let error = finish_symbol_catalog_fetch(Vec::new(), Vec::new())
            .unwrap_err()
            .to_string();

        assert_eq!(error, "all market sources returned an empty symbol catalog");
    }

    #[test]
    fn empty_source_catalog_is_reported_as_a_partial_warning() {
        let mut catalogs = Vec::new();
        let mut warnings = Vec::new();
        collect_symbol_catalog_result(
            MarketDataSource::Coinbase,
            Ok(Vec::new()),
            &mut catalogs,
            &mut warnings,
        );

        assert!(catalogs.is_empty());
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].source, MarketDataSource::Coinbase);
        assert!(warnings[0].message.contains("no supported market pairs"));
    }

    #[test]
    fn market_agent_accepts_http_and_socks_proxy_urls() {
        assert!(build_agent(None).is_ok());
        assert!(build_agent(Some("http://127.0.0.1:7890")).is_ok());
        assert!(build_agent(Some("socks5://127.0.0.1:1080")).is_ok());
        assert!(build_agent(Some("ftp://127.0.0.1:21")).is_err());
    }

    #[test]
    fn normalizes_optional_proxy_url() {
        assert_eq!(normalized_proxy_url(None), None);
        assert_eq!(normalized_proxy_url(Some("   ".to_string())), None);
        assert_eq!(
            normalized_proxy_url(Some("  socks://127.0.0.1:1080  ".to_string())),
            Some("socks://127.0.0.1:1080".to_string())
        );
    }
}
