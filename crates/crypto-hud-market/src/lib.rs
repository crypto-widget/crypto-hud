use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use crypto_hud_core::{
    clamp_refresh_interval, default_enabled_market_sources, market_pair_source,
    normalize_market_pair_key, parse_market_pair, MarketDataSource, MarketPair,
    MarketProviderPreference, MarketType, DEFAULT_REFRESH_INTERVAL_SECONDS,
};
use serde::Deserialize;

const BINANCE_BASE_URL: &str = "https://api.binance.com";
const COINBASE_EXCHANGE_BASE_URL: &str = "https://api.exchange.coinbase.com";
const OKX_BASE_URL: &str = "https://www.okx.com";
const HYPERLIQUID_BASE_URL: &str = "https://api.hyperliquid.xyz";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_CONCURRENT_PAIR_FETCHES: usize = 4;
const CANDLE_REFRESH_INTERVAL: Duration = Duration::from_secs(5 * 60);
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
struct CandleCacheEntry {
    candles: Vec<MarketCandle>,
    fetched_at: Instant,
}

#[derive(Debug, Clone)]
struct CandleFetchOutcome {
    candles: Vec<MarketCandle>,
    fetched_at: Option<Instant>,
    error: Option<String>,
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

pub fn spawn_market_feed(config: Arc<Mutex<MarketFeedConfig>>) -> Receiver<MarketEvent> {
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let mut agent = build_agent(None).expect("direct market agent should be valid");
        let mut agent_proxy_url: Option<String> = None;
        let mut candle_cache = CandleCache::new();

        loop {
            let config = config
                .lock()
                .map(|config| config.clone())
                .unwrap_or_else(|_| MarketFeedConfig {
                    symbols: Vec::new(),
                    provider: MarketProviderPreference::default(),
                    refresh_interval_seconds: DEFAULT_REFRESH_INTERVAL_SECONDS,
                    enabled_sources: default_enabled_market_sources(),
                    proxy_url: None,
                });
            let symbols = unique_market_pair_keys(config.symbols)
                .into_iter()
                .filter(|symbol| {
                    market_pair_source(symbol).is_some_and(|source| {
                        config.enabled_sources.is_empty()
                            || config.enabled_sources.contains(&source)
                    })
                })
                .collect::<Vec<_>>();
            let refresh_interval =
                Duration::from_secs(clamp_refresh_interval(config.refresh_interval_seconds) as u64);
            let proxy_url = normalized_proxy_url(config.proxy_url);

            if agent_proxy_url != proxy_url {
                match build_agent(proxy_url.as_deref()) {
                    Ok(next_agent) => {
                        agent = next_agent;
                        agent_proxy_url = proxy_url;
                        candle_cache.clear();
                    }
                    Err(error) => {
                        if sender.send(MarketEvent::Error(error.to_string())).is_err() {
                            return;
                        }
                        thread::sleep(refresh_interval);
                        continue;
                    }
                }
            }

            match fetch_symbols(&agent, &symbols, &mut candle_cache) {
                Ok(MarketBatch { snapshots, errors }) => {
                    for snapshot in snapshots {
                        if sender.send(MarketEvent::Snapshot(snapshot)).is_err() {
                            return;
                        }
                    }
                    if !errors.is_empty()
                        && sender.send(MarketEvent::Error(errors.join("; "))).is_err()
                    {
                        return;
                    }
                }
                Err(error) => {
                    if sender.send(MarketEvent::Error(error.to_string())).is_err() {
                        return;
                    }
                }
            }

            thread::sleep(refresh_interval);
        }
    });

    receiver
}

pub fn fetch_symbol_catalog(proxy_url: Option<&str>) -> Result<SymbolCatalog> {
    let agent = build_agent(proxy_url)?;
    fetch_symbol_catalog_with_agent(&agent)
}

fn fetch_symbol_catalog_with_agent(agent: &ureq::Agent) -> Result<SymbolCatalog> {
    let mut errors = Vec::new();
    let mut catalogs = Vec::new();

    match fetch_binance_symbol_catalog(agent) {
        Ok(entries) => catalogs.push(entries),
        Err(error) => errors.push(format!("Binance: {error:#}")),
    }
    match fetch_coinbase_symbol_catalog(agent) {
        Ok(entries) => catalogs.push(entries),
        Err(error) => errors.push(format!("Coinbase: {error:#}")),
    }
    match fetch_okx_symbol_catalog(agent) {
        Ok(entries) => catalogs.push(entries),
        Err(error) => errors.push(format!("OKX: {error:#}")),
    }
    match fetch_hyperliquid_symbol_catalog(agent) {
        Ok(entries) => catalogs.push(entries),
        Err(error) => errors.push(format!("Hyperliquid: {error:#}")),
    }

    let catalog = combine_symbol_catalogs(catalogs);
    if catalog.entries.is_empty() {
        bail!("{}", errors.join("; "));
    }
    Ok(catalog)
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

fn fetch_symbols(
    agent: &ureq::Agent,
    symbols: &[String],
    candle_cache: &mut CandleCache,
) -> Result<MarketBatch> {
    if symbols.is_empty() {
        bail!("no market pairs configured");
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
    }

    market_batch(snapshots, errors)
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
    snapshot.chart_closes_24h = candles.candles.iter().map(|candle| candle.close).collect();
    snapshot.chart_candles_24h = candles.candles;
    snapshot.chart_updated_at = candles.fetched_at;
    snapshot.chart_error = candles.error.clone();
    MarketFetchOutcome {
        snapshot,
        warning: candles
            .error
            .map(|error| format!("chart update failed: {error}")),
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
        if now.saturating_duration_since(entry.fetched_at) < CANDLE_REFRESH_INTERVAL {
            return CandleFetchOutcome {
                candles: entry.candles.clone(),
                fetched_at: Some(entry.fetched_at),
                error: None,
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
                    fetched_at: now,
                },
            );
            CandleFetchOutcome {
                candles,
                fetched_at: Some(now),
                error: None,
            }
        }
        Err(error) => {
            if let Some(entry) = candle_cache.get(&key) {
                CandleFetchOutcome {
                    candles: entry.candles.clone(),
                    fetched_at: Some(entry.fetched_at),
                    error: Some(error.to_string()),
                }
            } else {
                CandleFetchOutcome {
                    candles: Vec::new(),
                    fetched_at: None,
                    error: Some(error.to_string()),
                }
            }
        }
    }
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
    Ok(MarketSnapshot {
        symbol: pair.key(),
        price: parse_decimal(&ticker.last_price, "Binance lastPrice")?,
        change_percent_24h: parse_decimal(
            &ticker.price_change_percent,
            "Binance priceChangePercent",
        )?,
        chart_closes_24h: Vec::new(),
        chart_candles_24h: Vec::new(),
        chart_updated_at: None,
        chart_error: None,
        source: MarketDataSource::Binance,
    })
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
    let price = parse_decimal(&stats.last, "Coinbase last")?;
    let open_24h = parse_decimal(&stats.open, "Coinbase open")?;
    if open_24h <= 0.0 {
        bail!("Coinbase open must be greater than zero");
    }

    Ok(MarketSnapshot {
        symbol: pair.key(),
        price,
        change_percent_24h: ((price - open_24h) / open_24h) * 100.0,
        chart_closes_24h: Vec::new(),
        chart_candles_24h: Vec::new(),
        chart_updated_at: None,
        chart_error: None,
        source: MarketDataSource::Coinbase,
    })
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
    let price = parse_decimal(&ticker.last, "OKX last")?;
    let open_24h = parse_decimal(&ticker.open_24h, "OKX open24h")?;
    if open_24h <= 0.0 {
        bail!("OKX open24h must be greater than zero");
    }

    Ok(MarketSnapshot {
        symbol: pair.key(),
        price,
        change_percent_24h: ((price - open_24h) / open_24h) * 100.0,
        chart_closes_24h: Vec::new(),
        chart_candles_24h: Vec::new(),
        chart_updated_at: None,
        chart_error: None,
        source: MarketDataSource::Okx,
    })
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
        .and_then(|price| parse_decimal(price, "Hyperliquid markPx"))?;
    let prev_day = context
        .prev_day_px
        .as_deref()
        .ok_or_else(|| anyhow!("Hyperliquid context did not include prevDayPx"))
        .and_then(|price| parse_decimal(price, "Hyperliquid prevDayPx"))?;
    if prev_day <= 0.0 {
        bail!("Hyperliquid prevDayPx must be greater than zero");
    }

    Ok(MarketSnapshot {
        symbol: pair.key(),
        price,
        change_percent_24h: ((price - prev_day) / prev_day) * 100.0,
        chart_closes_24h: Vec::new(),
        chart_candles_24h: Vec::new(),
        chart_updated_at: None,
        chart_error: None,
        source: MarketDataSource::Hyperliquid,
    })
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
            .ok_or_else(|| anyhow!("failed to parse {field}: {value}")),
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
}

fn parse_u64(value: &str, field: &str) -> Result<u64> {
    value
        .parse::<u64>()
        .with_context(|| format!("failed to parse {field}: {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
                fetched_at,
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
        assert_eq!(cache[&pair.key()].fetched_at, fetched_at);
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
