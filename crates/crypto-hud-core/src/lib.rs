use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DEFAULT_MARKET_SYMBOLS: &[&str] = &[
    "binance:spot:BTC/USDT",
    "binance:spot:ETH/USDT",
    "binance:spot:SOL/USDT",
];
pub const MIN_REFRESH_INTERVAL_SECONDS: i32 = 5;
pub const MAX_REFRESH_INTERVAL_SECONDS: i32 = 60;
pub const DEFAULT_REFRESH_INTERVAL_SECONDS: i32 = 5;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketDataSource {
    #[default]
    Binance,
    Coinbase,
    Okx,
    Hyperliquid,
}

impl MarketDataSource {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Binance => "binance",
            Self::Coinbase => "coinbase",
            Self::Okx => "okx",
            Self::Hyperliquid => "hyperliquid",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Binance => "Binance",
            Self::Coinbase => "Coinbase",
            Self::Okx => "OKX",
            Self::Hyperliquid => "Hyperliquid",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        match normalized_identifier(value).as_str() {
            "binance" | "bin" => Some(Self::Binance),
            "coinbase" | "coin" | "cb" => Some(Self::Coinbase),
            "okx" | "ok" => Some(Self::Okx),
            "hyperliquid" | "hl" | "hyper" => Some(Self::Hyperliquid),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketType {
    #[default]
    Spot,
    Perp,
}

impl MarketType {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Spot => "spot",
            Self::Perp => "perp",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Spot => "Spot",
            Self::Perp => "Perp",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        match normalized_identifier(value).as_str() {
            "spot" => Some(Self::Spot),
            "perp" | "perpetual" | "perpetuals" | "futures" | "swap" => Some(Self::Perp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MarketPair {
    pub source: MarketDataSource,
    pub market_type: MarketType,
    pub base: String,
    pub quote: String,
}

impl MarketPair {
    pub fn new(
        source: MarketDataSource,
        market_type: MarketType,
        base: impl Into<String>,
        quote: impl Into<String>,
    ) -> Option<Self> {
        let base = normalize_asset_code(&base.into())?;
        let quote = normalize_asset_code(&quote.into())?;
        Some(Self {
            source,
            market_type,
            base,
            quote,
        })
    }

    pub fn key(&self) -> String {
        format!(
            "{}:{}:{}/{}",
            self.source.slug(),
            self.market_type.slug(),
            self.base,
            self.quote
        )
    }

    pub fn pair_symbol(&self) -> String {
        format!("{}/{}", self.base, self.quote)
    }
}

pub fn default_enabled_market_sources() -> Vec<MarketDataSource> {
    vec![
        MarketDataSource::Binance,
        MarketDataSource::Coinbase,
        MarketDataSource::Okx,
        MarketDataSource::Hyperliquid,
    ]
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketProviderPreference {
    #[default]
    Auto,
    Binance,
    Okx,
}

impl MarketProviderPreference {
    pub const fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Binance,
            2 => Self::Okx,
            _ => Self::Auto,
        }
    }

    pub const fn index(self) -> i32 {
        match self {
            Self::Auto => 0,
            Self::Binance => 1,
            Self::Okx => 2,
        }
    }
}

pub fn default_market_symbols() -> Vec<String> {
    DEFAULT_MARKET_SYMBOLS
        .iter()
        .map(|symbol| (*symbol).to_string())
        .collect()
}

pub fn clamp_refresh_interval(value: i32) -> i32 {
    value.clamp(MIN_REFRESH_INTERVAL_SECONDS, MAX_REFRESH_INTERVAL_SECONDS)
}

pub fn normalize_market_symbols(symbols: Vec<String>) -> Vec<String> {
    normalize_symbols_with_limit(
        symbols,
        DEFAULT_MARKET_SYMBOLS.len(),
        default_market_symbols(),
    )
}

pub fn normalize_symbols_with_limit(
    symbols: Vec<String>,
    limit: usize,
    fallback: Vec<String>,
) -> Vec<String> {
    let normalized = symbols
        .iter()
        .filter_map(|symbol| normalize_market_pair_key(symbol))
        .fold(Vec::new(), |mut symbols, symbol| {
            if symbols.len() < limit && !symbols.contains(&symbol) {
                symbols.push(symbol);
            }
            symbols
        });

    if normalized.is_empty() {
        fallback
    } else {
        normalized
    }
}

pub fn normalize_symbol_token(raw: &str) -> Option<String> {
    parse_market_pair(raw)
        .map(|pair| pair.base)
        .or_else(|| normalize_legacy_symbol(raw))
}

pub fn normalize_market_pair_key(raw: &str) -> Option<String> {
    parse_market_pair(raw).map(|pair| pair.key())
}

pub fn parse_market_pair(raw: &str) -> Option<MarketPair> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    let parts = raw
        .split('·')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let value = parts.first().copied().unwrap_or(raw);
    let source_hint = parts
        .iter()
        .rev()
        .find_map(|part| market_source_from_display_part(part));
    let type_hint = parts
        .iter()
        .rev()
        .find_map(|part| market_type_from_display_part(part));

    parse_keyed_market_pair(value, source_hint, type_hint)
        .or_else(|| parse_unkeyed_market_pair(value, source_hint, type_hint))
}

pub fn format_market_pair_symbol(raw: &str) -> String {
    parse_market_pair(raw)
        .map(|pair| pair.pair_symbol())
        .unwrap_or_else(|| normalize_symbol_token(raw).unwrap_or_else(|| raw.trim().to_string()))
}

pub fn format_market_pair_source(raw: &str) -> String {
    parse_market_pair(raw)
        .map(|pair| {
            if pair.source == MarketDataSource::Hyperliquid && pair.market_type == MarketType::Perp
            {
                format!("{} {}", pair.source.label(), pair.market_type.label())
            } else {
                pair.source.label().to_string()
            }
        })
        .unwrap_or_default()
}

pub fn format_market_pair_display(raw: &str) -> String {
    parse_market_pair(raw)
        .map(|pair| {
            let source = if pair.source == MarketDataSource::Hyperliquid
                && pair.market_type == MarketType::Perp
            {
                format!("{} {}", pair.source.label(), pair.market_type.label())
            } else {
                pair.source.label().to_string()
            };
            format!("{} · {source}", pair.pair_symbol())
        })
        .unwrap_or_else(|| raw.trim().to_string())
}

pub fn market_pair_source(raw: &str) -> Option<MarketDataSource> {
    parse_market_pair(raw).map(|pair| pair.source)
}

fn parse_keyed_market_pair(
    value: &str,
    source_hint: Option<MarketDataSource>,
    type_hint: Option<MarketType>,
) -> Option<MarketPair> {
    let keyed_parts = value.split(':').map(str::trim).collect::<Vec<_>>();
    match keyed_parts.as_slice() {
        [source, market_type, pair] => {
            let source = MarketDataSource::from_slug(source)?;
            let market_type = MarketType::from_slug(market_type)?;
            let (base, quote) = parse_pair_assets(pair)?;
            MarketPair::new(source, market_type, base, quote)
        }
        [source, pair] => {
            let source = MarketDataSource::from_slug(source)?;
            let market_type = type_hint.unwrap_or(default_market_type_for_source(source));
            let (base, quote) = parse_pair_assets(pair)?;
            MarketPair::new(source, market_type, base, quote)
        }
        [pair] => {
            let source = source_hint?;
            let market_type = type_hint.unwrap_or(default_market_type_for_source(source));
            let (base, quote) = parse_pair_assets(pair)?;
            MarketPair::new(source, market_type, base, quote)
        }
        _ => None,
    }
}

fn parse_unkeyed_market_pair(
    value: &str,
    source_hint: Option<MarketDataSource>,
    type_hint: Option<MarketType>,
) -> Option<MarketPair> {
    let source = source_hint.unwrap_or(MarketDataSource::Binance);
    let market_type = type_hint.unwrap_or(default_market_type_for_source(source));
    let (base, quote) = parse_pair_assets(value).or_else(|| {
        normalize_legacy_symbol(value)
            .map(|base| (base, default_quote_for_source(source).to_string()))
    })?;
    MarketPair::new(source, market_type, base, quote)
}

fn parse_pair_assets(value: &str) -> Option<(String, String)> {
    let value = value.trim();
    if value.contains('/') || value.contains('-') {
        let mut parts = value.split(['/', '-']).map(str::trim);
        let base = normalize_asset_code(parts.next()?)?;
        let quote = normalize_asset_code(parts.next()?)?;
        return Some((base, quote));
    }

    let compact = normalize_asset_code(value)?;
    for quote in ["USDT", "USDC", "USD"] {
        if compact.len() > quote.len() && compact.ends_with(quote) {
            return Some((
                compact.trim_end_matches(quote).to_string(),
                quote.to_string(),
            ));
        }
    }
    None
}

fn normalize_legacy_symbol(raw: &str) -> Option<String> {
    let token = raw.trim().to_ascii_uppercase();
    let head = token
        .split(['/', '-', ':'])
        .next()
        .unwrap_or(token.as_str());
    let base = head
        .strip_suffix("USDT")
        .or_else(|| head.strip_suffix("USDC"))
        .unwrap_or(head);
    normalize_asset_code(base)
}

fn normalize_asset_code(raw: &str) -> Option<String> {
    let value = raw
        .trim()
        .to_ascii_uppercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn normalized_identifier(raw: &str) -> String {
    raw.trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
}

fn market_source_from_display_part(value: &str) -> Option<MarketDataSource> {
    let normalized = normalized_identifier(value);
    if normalized.contains("hyperliquid") {
        Some(MarketDataSource::Hyperliquid)
    } else if normalized.contains("coinbase") {
        Some(MarketDataSource::Coinbase)
    } else if normalized.contains("binance") || normalized == "bin" {
        Some(MarketDataSource::Binance)
    } else if normalized.contains("okx") || normalized == "ok" {
        Some(MarketDataSource::Okx)
    } else {
        None
    }
}

fn market_type_from_display_part(value: &str) -> Option<MarketType> {
    let normalized = normalized_identifier(value);
    if normalized.contains("perp") || normalized.contains("future") || normalized.contains("swap") {
        Some(MarketType::Perp)
    } else if normalized.contains("spot") {
        Some(MarketType::Spot)
    } else {
        None
    }
}

fn default_market_type_for_source(source: MarketDataSource) -> MarketType {
    match source {
        MarketDataSource::Hyperliquid => MarketType::Perp,
        MarketDataSource::Binance | MarketDataSource::Coinbase | MarketDataSource::Okx => {
            MarketType::Spot
        }
    }
}

fn default_quote_for_source(source: MarketDataSource) -> &'static str {
    match source {
        MarketDataSource::Hyperliquid => "USDC",
        MarketDataSource::Binance | MarketDataSource::Coinbase | MarketDataSource::Okx => "USDT",
    }
}

pub fn format_price(price: f64) -> String {
    if price >= 1_000.0 {
        format!("{price:.0}")
    } else if price >= 10.0 {
        format!("{price:.2}")
    } else {
        format!("{price:.4}")
    }
}

pub fn format_pair_change(change: f64) -> String {
    let sign = if change >= 0.0 { "+" } else { "" };
    format!("{sign}{change:.2}%")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertCondition {
    PriceAbove,
    PriceBelow,
    ChangePercentAbove,
    ChangePercentBelow,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub symbol: String,
    pub condition: AlertCondition,
    pub threshold: f64,
    #[serde(default = "default_alert_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlertEvaluation {
    pub rule_id: String,
    pub symbol: String,
    pub condition: AlertCondition,
    pub threshold: f64,
    pub current_value: f64,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlertQuote {
    pub price: f64,
    pub change_percent_24h: f64,
}

pub fn evaluate_alerts(
    rules: &[AlertRule],
    quotes: &HashMap<String, AlertQuote>,
) -> Vec<AlertEvaluation> {
    rules
        .iter()
        .filter(|rule| rule.enabled)
        .filter_map(|rule| evaluate_alert(rule, quotes))
        .collect()
}

fn evaluate_alert(
    rule: &AlertRule,
    quotes: &HashMap<String, AlertQuote>,
) -> Option<AlertEvaluation> {
    let symbol = normalize_market_pair_key(&rule.symbol)?;
    let quote = quotes.get(&symbol)?;
    let current_value = match rule.condition {
        AlertCondition::PriceAbove | AlertCondition::PriceBelow => quote.price,
        AlertCondition::ChangePercentAbove | AlertCondition::ChangePercentBelow => {
            quote.change_percent_24h
        }
    };
    let triggered = match rule.condition {
        AlertCondition::PriceAbove | AlertCondition::ChangePercentAbove => {
            current_value >= rule.threshold
        }
        AlertCondition::PriceBelow | AlertCondition::ChangePercentBelow => {
            current_value <= rule.threshold
        }
    };
    if !triggered {
        return None;
    }

    let metric = match rule.condition {
        AlertCondition::PriceAbove | AlertCondition::PriceBelow => "price",
        AlertCondition::ChangePercentAbove | AlertCondition::ChangePercentBelow => "24h change",
    };
    let direction = match rule.condition {
        AlertCondition::PriceAbove | AlertCondition::ChangePercentAbove => "above",
        AlertCondition::PriceBelow | AlertCondition::ChangePercentBelow => "below",
    };

    Some(AlertEvaluation {
        rule_id: rule.id.clone(),
        symbol: symbol.clone(),
        condition: rule.condition,
        threshold: rule.threshold,
        current_value,
        title: format!("{} alert", format_market_pair_symbol(&symbol)),
        body: format!(
            "{} {metric} is {direction} {}: {}",
            format_market_pair_symbol(&symbol),
            format_alert_value(rule.condition, rule.threshold),
            format_alert_value(rule.condition, current_value)
        ),
    })
}

fn format_alert_value(condition: AlertCondition, value: f64) -> String {
    match condition {
        AlertCondition::PriceAbove | AlertCondition::PriceBelow => format_price(value),
        AlertCondition::ChangePercentAbove | AlertCondition::ChangePercentBelow => {
            format_pair_change(value)
        }
    }
}

fn default_alert_enabled() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_indices_are_stable() {
        assert_eq!(
            MarketProviderPreference::from_index(0),
            MarketProviderPreference::Auto
        );
        assert_eq!(
            MarketProviderPreference::from_index(1),
            MarketProviderPreference::Binance
        );
        assert_eq!(
            MarketProviderPreference::from_index(2),
            MarketProviderPreference::Okx
        );
        assert_eq!(MarketProviderPreference::Okx.index(), 2);
    }

    #[test]
    fn normalizes_symbols_to_unique_pair_keys() {
        let normalized = normalize_market_symbols(vec![
            "btc".to_string(),
            "ETHUSDT".to_string(),
            "sol/usdt".to_string(),
            "SOL-USDT".to_string(),
            "bnb".to_string(),
            "xrp".to_string(),
            "doge".to_string(),
        ]);

        assert_eq!(
            normalized,
            vec![
                "binance:spot:BTC/USDT",
                "binance:spot:ETH/USDT",
                "binance:spot:SOL/USDT",
            ]
        );
    }

    #[test]
    fn parses_and_formats_market_pair_keys() {
        assert_eq!(
            normalize_market_pair_key("okx:spot:btc/usdt").as_deref(),
            Some("okx:spot:BTC/USDT")
        );
        assert_eq!(
            normalize_market_pair_key("BTC/USDT · Coinbase").as_deref(),
            Some("coinbase:spot:BTC/USDT")
        );
        assert_eq!(
            normalize_market_pair_key("BTC/USDC · Hyperliquid Perp").as_deref(),
            Some("hyperliquid:perp:BTC/USDC")
        );
        assert_eq!(
            format_market_pair_display("binance:spot:ETH/USDT"),
            "ETH/USDT · Binance"
        );
    }

    #[test]
    fn formats_prices_for_widget_display() {
        assert_eq!(format_price(106800.12), "106800");
        assert_eq!(format_price(3420.5), "3420");
        assert_eq!(format_price(42.125), "42.12");
        assert_eq!(format_price(0.123456), "0.1235");
    }

    #[test]
    fn evaluates_price_and_change_alert_rules() {
        let mut quotes = HashMap::new();
        quotes.insert(
            "binance:spot:BTC/USDT".to_string(),
            AlertQuote {
                price: 106800.12,
                change_percent_24h: 1.234,
            },
        );
        let rules = vec![
            AlertRule {
                id: "btc-price".to_string(),
                symbol: "btcusdt".to_string(),
                condition: AlertCondition::PriceAbove,
                threshold: 100000.0,
                enabled: true,
            },
            AlertRule {
                id: "btc-change".to_string(),
                symbol: "BTC".to_string(),
                condition: AlertCondition::ChangePercentBelow,
                threshold: -2.0,
                enabled: true,
            },
        ];

        let alerts = evaluate_alerts(&rules, &quotes);

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule_id, "btc-price");
        assert_eq!(alerts[0].symbol, "binance:spot:BTC/USDT");
        assert!(alerts[0].body.contains("100000"));
    }
}
