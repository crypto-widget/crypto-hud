use anyhow::{bail, Context, Result};
pub use crypto_hud_core::{evaluate_alerts, AlertCondition, AlertEvaluation, AlertRule};
use crypto_hud_core::{
    format_market_pair_symbol, format_pair_change, format_price, market_pair_source,
    normalize_market_pair_key, parse_market_pair, AlertQuote, MarketDataSource,
};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Write,
    path::Path,
    time::{Duration, Instant},
};

pub const HOST_PLUGIN_API_VERSION: &str = "0.1.0";
pub const MIN_SYMBOL_LIMIT: usize = 1;
pub const MAX_SYMBOL_LIMIT: usize = 5;

const MIN_PLUGIN_WIDTH: i32 = 120;
const MIN_PLUGIN_HEIGHT: i32 = 80;
const MAX_PLUGIN_WIDTH: i32 = 1200;
const MAX_PLUGIN_HEIGHT: i32 = 900;
const SUPPORTED_CAPABILITIES: &[&str] = &["market.price", "market.candles"];
const CHART_VIEWBOX_WIDTH: f64 = 100.0;
const CHART_VIEWBOX_HEIGHT: f64 = 40.0;
const CHART_VERTICAL_PADDING: f64 = 6.0;
const CHART_MAX_RENDER_POINTS: usize = 96;
const STALE_DATA_SECONDS: u64 = 180;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub host_api_version: String,
    pub renderer: PluginRendererManifest,
    pub permissions: PluginPermissionsManifest,
    pub default_size: PluginSize,
    #[serde(default)]
    pub size_policy: PluginSizePolicy,
    #[serde(default = "default_min_symbol_limit")]
    pub min_symbol_limit: usize,
    pub symbol_limit: usize,
    #[serde(default)]
    pub default_symbols: Vec<String>,
    #[serde(default)]
    pub data_requirements: Vec<PluginDataRequirement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PluginRendererManifest {
    pub kind: String,
    pub entry: String,
    pub component: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct PluginPermissionsManifest {
    pub network: bool,
    pub filesystem: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct PluginSize {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PluginSizePolicy {
    Fixed,
    SymbolBlocks {
        #[serde(rename = "blockSize")]
        block_size: PluginSize,
        padding: PluginSize,
    },
    SymbolGrid {
        #[serde(rename = "cellSize")]
        cell_size: PluginSize,
        #[serde(rename = "contentPadding")]
        content_padding: PluginSize,
        #[serde(default)]
        columns: Option<usize>,
        #[serde(default)]
        rows: Option<usize>,
    },
}

impl Default for PluginSizePolicy {
    fn default() -> Self {
        Self::Fixed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PluginDataRequirement {
    pub capability: String,
}

fn default_min_symbol_limit() -> usize {
    MIN_SYMBOL_LIMIT
}

pub fn parse_manifest(contents: &str) -> Result<PluginManifest> {
    let manifest = serde_json::from_str::<PluginManifest>(contents)
        .context("failed to parse plugin manifest")?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub fn validate_manifest(manifest: &PluginManifest) -> Result<()> {
    if manifest.schema_version != 3 {
        bail!("schemaVersion must be 3");
    }
    validate_plugin_id(&manifest.id)?;
    if manifest.name.trim().is_empty() {
        bail!("name must be non-empty");
    }
    Version::parse(&manifest.version).context("version must be valid SemVer")?;
    validate_host_api_version(&manifest.host_api_version)?;
    if manifest.renderer.kind != "slint" {
        bail!("renderer.kind must be slint");
    }
    validate_relative_path(&manifest.renderer.entry, "renderer.entry")?;
    if manifest.renderer.component.trim().is_empty() {
        bail!("renderer.component must be non-empty");
    }
    if manifest.permissions.network || manifest.permissions.filesystem {
        bail!("permissions.network and permissions.filesystem must both be false");
    }
    validate_size(manifest.default_size)?;
    if !(MIN_SYMBOL_LIMIT..=MAX_SYMBOL_LIMIT).contains(&manifest.min_symbol_limit) {
        bail!("minSymbolLimit must be between {MIN_SYMBOL_LIMIT} and {MAX_SYMBOL_LIMIT}");
    }
    if !(MIN_SYMBOL_LIMIT..=MAX_SYMBOL_LIMIT).contains(&manifest.symbol_limit) {
        bail!("symbolLimit must be between {MIN_SYMBOL_LIMIT} and {MAX_SYMBOL_LIMIT}");
    }
    if manifest.min_symbol_limit > manifest.symbol_limit {
        bail!("minSymbolLimit must be less than or equal to symbolLimit");
    }
    validate_default_symbols(
        &manifest.default_symbols,
        manifest.min_symbol_limit,
        manifest.symbol_limit,
    )?;
    validate_size_policy(
        manifest.size_policy,
        manifest.default_size,
        manifest.symbol_limit,
    )?;
    for requirement in &manifest.data_requirements {
        if !SUPPORTED_CAPABILITIES.contains(&requirement.capability.as_str()) {
            bail!(
                "unsupported data requirement capability {}",
                requirement.capability
            );
        }
    }
    Ok(())
}

fn validate_default_symbols(symbols: &[String], min: usize, limit: usize) -> Result<()> {
    if symbols.is_empty() {
        return Ok(());
    }
    if symbols.len() < min {
        bail!("defaultSymbols must contain at least minSymbolLimit entries");
    }
    if symbols.len() > limit {
        bail!("defaultSymbols must not exceed symbolLimit");
    }

    let mut normalized_symbols = Vec::new();
    for symbol in symbols {
        let normalized = normalize_market_pair_key(symbol)
            .with_context(|| format!("defaultSymbols contains invalid market pair {symbol}"))?;
        if normalized_symbols.contains(&normalized) {
            bail!("defaultSymbols contains duplicate market pair {normalized}");
        }
        normalized_symbols.push(normalized);
    }

    Ok(())
}

pub fn validate_relative_path(raw: &str, label: &str) -> Result<()> {
    let path = Path::new(raw);
    if raw.trim().is_empty() {
        bail!("{label} must be non-empty");
    }
    if path.is_absolute() {
        bail!("{label} must be relative");
    }
    if raw.contains("..")
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        bail!("{label} must not contain ..");
    }
    Ok(())
}

fn validate_plugin_id(id: &str) -> Result<()> {
    let id = id.trim();
    if id.is_empty() {
        bail!("id must be non-empty");
    }
    if !id.contains('.') && !id.contains('-') {
        bail!("id should contain dots or hyphens");
    }
    if id.chars().any(|character| {
        !(character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '-'))
    }) {
        bail!("id must use lowercase ASCII letters, digits, dots, or hyphens");
    }
    Ok(())
}

fn validate_host_api_version(requirement: &str) -> Result<()> {
    let requirement = VersionReq::parse(requirement)
        .context("hostApiVersion must be a valid SemVer requirement")?;
    let host = Version::parse(HOST_PLUGIN_API_VERSION).expect("host plugin API version is valid");
    if !requirement.matches(&host) {
        bail!("hostApiVersion does not match host {HOST_PLUGIN_API_VERSION}");
    }
    Ok(())
}

fn validate_size(size: PluginSize) -> Result<()> {
    if !(MIN_PLUGIN_WIDTH..=MAX_PLUGIN_WIDTH).contains(&size.width)
        || !(MIN_PLUGIN_HEIGHT..=MAX_PLUGIN_HEIGHT).contains(&size.height)
    {
        bail!(
            "defaultSize must be between {MIN_PLUGIN_WIDTH}x{MIN_PLUGIN_HEIGHT} and {MAX_PLUGIN_WIDTH}x{MAX_PLUGIN_HEIGHT}"
        );
    }
    Ok(())
}

fn validate_size_policy(
    policy: PluginSizePolicy,
    default_size: PluginSize,
    symbol_limit: usize,
) -> Result<()> {
    match policy {
        PluginSizePolicy::Fixed => Ok(()),
        PluginSizePolicy::SymbolBlocks {
            block_size,
            padding,
        } => {
            if block_size.width <= 0 || block_size.height <= 0 {
                bail!("sizePolicy.blockSize must be positive");
            }
            if padding.width < 0 || padding.height < 0 {
                bail!("sizePolicy.padding must not be negative");
            }
            let width = block_size
                .width
                .checked_mul(symbol_limit as i32)
                .and_then(|width| width.checked_add(padding.width))
                .context("sizePolicy symbol block width overflowed")?;
            let height = block_size
                .height
                .checked_add(padding.height)
                .context("sizePolicy symbol block height overflowed")?;
            if width != default_size.width || height != default_size.height {
                bail!("defaultSize must match sizePolicy at symbolLimit");
            }
            validate_size(PluginSize { width, height })?;
            Ok(())
        }
        PluginSizePolicy::SymbolGrid {
            cell_size,
            content_padding,
            columns,
            rows,
        } => {
            if cell_size.width <= 0 || cell_size.height <= 0 {
                bail!("sizePolicy.cellSize must be positive");
            }
            if content_padding.width < 0 || content_padding.height < 0 {
                bail!("sizePolicy.contentPadding must not be negative");
            }
            if columns.is_none() && rows.is_none() {
                bail!("sizePolicy.columns or sizePolicy.rows must be set");
            }
            if let Some(columns) = columns {
                if !(1..=MAX_SYMBOL_LIMIT).contains(&columns) {
                    bail!("sizePolicy.columns must be between 1 and {MAX_SYMBOL_LIMIT}");
                }
            }
            if let Some(rows) = rows {
                if !(1..=MAX_SYMBOL_LIMIT).contains(&rows) {
                    bail!("sizePolicy.rows must be between 1 and {MAX_SYMBOL_LIMIT}");
                }
            }
            if let (Some(columns), Some(rows)) = (columns, rows) {
                if columns.saturating_mul(rows) < symbol_limit {
                    bail!("sizePolicy.columns * sizePolicy.rows must cover symbolLimit");
                }
            }

            let (track_columns, track_rows) = symbol_grid_tracks(symbol_limit, columns, rows)
                .context("sizePolicy symbol grid tracks overflowed")?;
            let width = cell_size
                .width
                .checked_mul(track_columns as i32)
                .and_then(|width| width.checked_add(content_padding.width))
                .context("sizePolicy symbol grid width overflowed")?;
            let height = cell_size
                .height
                .checked_mul(track_rows as i32)
                .and_then(|height| height.checked_add(content_padding.height))
                .context("sizePolicy symbol grid height overflowed")?;
            if width != default_size.width || height != default_size.height {
                bail!("defaultSize must match sizePolicy at symbolLimit");
            }
            validate_size(PluginSize { width, height })?;
            Ok(())
        }
    }
}

fn symbol_grid_tracks(
    symbol_count: usize,
    columns: Option<usize>,
    rows: Option<usize>,
) -> Option<(usize, usize)> {
    let count = symbol_count.max(1);
    match (columns, rows) {
        (Some(max_columns), Some(max_rows)) => {
            let columns = count.min(max_columns).max(1);
            let rows = div_ceil_usize(count, max_columns).min(max_rows).max(1);
            Some((columns, rows))
        }
        (Some(max_columns), None) => {
            let columns = count.min(max_columns).max(1);
            let rows = div_ceil_usize(count, max_columns).max(1);
            Some((columns, rows))
        }
        (None, Some(max_rows)) => {
            let rows = count.min(max_rows).max(1);
            let columns = div_ceil_usize(count, max_rows).max(1);
            Some((columns, rows))
        }
        (None, None) => None,
    }
}

fn div_ceil_usize(value: usize, divisor: usize) -> usize {
    if divisor == 0 {
        return 0;
    }
    value.div_ceil(divisor)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketQuoteSnapshot {
    pub symbol: String,
    pub price: f64,
    pub change_percent_24h: f64,
    pub chart_closes_24h: Vec<f64>,
    pub source: MarketDataSource,
}

pub type QuoteCache = HashMap<String, QuoteState>;

#[derive(Debug, Clone, PartialEq)]
pub struct QuoteState {
    pub price: f64,
    pub change_percent_24h: f64,
    pub chart_closes_24h: Vec<f64>,
    pub source: MarketDataSource,
    pub updated_at: Instant,
}

impl QuoteState {
    pub fn new(
        price: f64,
        change_percent_24h: f64,
        chart_closes_24h: Vec<f64>,
        source: MarketDataSource,
        updated_at: Instant,
    ) -> Self {
        Self {
            price,
            change_percent_24h,
            chart_closes_24h,
            source,
            updated_at,
        }
    }

    pub fn from_snapshot(snapshot: &MarketQuoteSnapshot, updated_at: Instant) -> Self {
        Self::new(
            snapshot.price,
            snapshot.change_percent_24h,
            snapshot.chart_closes_24h.clone(),
            snapshot.source,
            updated_at,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteRowView {
    pub symbol: String,
    pub price: String,
    pub change: String,
    pub positive: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WidgetDisplayOptions {
    pub hide_quote_asset: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WidgetRuntimeView {
    pub widget_id: String,
    pub quote_rows: Vec<QuoteRowView>,
    #[serde(default)]
    pub source_name_text: String,
    pub source_text: String,
    pub updated_text: String,
    pub chart_line_path: String,
    pub chart_fill_path: String,
    pub chart_end_y_ratio: i32,
    pub chart_ready: bool,
    pub chart_positive: bool,
}

pub struct WidgetRuntimeViewParams<'a> {
    pub widget_id: &'a str,
    pub symbols: &'a [String],
    pub quote_cache: &'a QuoteCache,
    pub source_prefix: &'a str,
    pub provider_labels: ProviderLabels<'a>,
    pub labels: RuntimeTextLabels<'a>,
    pub has_market_error: bool,
    pub now: Instant,
    pub display_options: WidgetDisplayOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeTextLabels<'a> {
    pub no_pairs: &'a str,
    pub connecting: &'a str,
    pub connection_error: &'a str,
    pub updated: &'a str,
    pub stale: &'a str,
    pub fallback: &'a str,
    pub source_error: &'a str,
    pub live_count_prefix: &'a str,
    pub live_count_suffix: &'a str,
}

impl Default for RuntimeTextLabels<'static> {
    fn default() -> Self {
        Self {
            no_pairs: "No pairs",
            connecting: "Connecting",
            connection_error: "Connection failed",
            updated: "Updated",
            stale: "Stale",
            fallback: "fallback",
            source_error: "source issue",
            live_count_prefix: "",
            live_count_suffix: " live",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderLabels<'a> {
    pub binance: &'a str,
    pub okx: &'a str,
    pub hyperliquid: &'a str,
    pub mixed: &'a str,
}

impl Default for ProviderLabels<'static> {
    fn default() -> Self {
        Self {
            binance: "Binance",
            okx: "OKX",
            hyperliquid: "Hyperliquid",
            mixed: "Mixed",
        }
    }
}

impl QuoteRowView {
    pub fn from_snapshot(snapshot: &MarketQuoteSnapshot) -> Self {
        Self::from_snapshot_with_options(snapshot, WidgetDisplayOptions::default())
    }

    pub fn from_snapshot_with_options(
        snapshot: &MarketQuoteSnapshot,
        display_options: WidgetDisplayOptions,
    ) -> Self {
        Self {
            symbol: format_pair_symbol_with_options(&snapshot.symbol, display_options),
            price: format_price(snapshot.price),
            change: format_pair_change(snapshot.change_percent_24h),
            positive: snapshot.change_percent_24h >= 0.0,
        }
    }

    pub fn from_state(symbol: &str, state: &QuoteState) -> Self {
        Self::from_state_with_options(symbol, state, WidgetDisplayOptions::default())
    }

    pub fn from_state_with_options(
        symbol: &str,
        state: &QuoteState,
        display_options: WidgetDisplayOptions,
    ) -> Self {
        Self {
            symbol: format_pair_symbol_with_options(symbol, display_options),
            price: format_price(state.price),
            change: format_pair_change(state.change_percent_24h),
            positive: state.change_percent_24h >= 0.0,
        }
    }

    pub fn loading(symbol: &str, labels: RuntimeTextLabels<'_>) -> Self {
        Self::loading_with_options(symbol, labels, WidgetDisplayOptions::default())
    }

    pub fn loading_with_options(
        symbol: &str,
        labels: RuntimeTextLabels<'_>,
        display_options: WidgetDisplayOptions,
    ) -> Self {
        Self {
            symbol: format_pair_symbol_with_options(symbol, display_options),
            price: labels.connecting.to_string(),
            change: "--".to_string(),
            positive: true,
        }
    }
}

pub fn build_widget_runtime_view(params: WidgetRuntimeViewParams<'_>) -> WidgetRuntimeView {
    let chart = chart_path_view_for_symbols(params.symbols, params.quote_cache);
    WidgetRuntimeView {
        widget_id: params.widget_id.to_string(),
        quote_rows: quote_rows_for_symbols_with_options(
            params.symbols,
            params.quote_cache,
            params.labels,
            params.display_options,
        ),
        source_name_text: source_name_text_for_symbols(
            params.symbols,
            params.quote_cache,
            params.provider_labels,
        ),
        source_text: source_text_for_symbols(
            params.source_prefix,
            params.symbols,
            params.quote_cache,
            params.provider_labels,
            params.labels,
            data_health_for_symbols(params.symbols, params.quote_cache, params.now),
            params.has_market_error,
        ),
        updated_text: updated_text_for_symbols(
            params.symbols,
            params.quote_cache,
            params.labels,
            params.has_market_error,
            params.now,
        ),
        chart_line_path: chart.line_path,
        chart_fill_path: chart.fill_path,
        chart_end_y_ratio: chart.end_y_ratio,
        chart_ready: chart.ready,
        chart_positive: chart.positive,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartPathView {
    pub line_path: String,
    pub fill_path: String,
    pub end_y_ratio: i32,
    pub ready: bool,
    pub positive: bool,
}

impl ChartPathView {
    fn empty() -> Self {
        Self {
            line_path: format!(
                "M 0 {} L {} {}",
                CHART_VIEWBOX_HEIGHT / 2.0,
                CHART_VIEWBOX_WIDTH,
                CHART_VIEWBOX_HEIGHT / 2.0
            ),
            fill_path: format!(
                "M 0 {0} L 0 {1} L {2} {1} L {2} {0} Z",
                CHART_VIEWBOX_HEIGHT,
                CHART_VIEWBOX_HEIGHT / 2.0,
                CHART_VIEWBOX_WIDTH
            ),
            end_y_ratio: 500,
            ready: false,
            positive: true,
        }
    }
}

pub fn chart_path_view_for_symbols(symbols: &[String], quote_cache: &QuoteCache) -> ChartPathView {
    let Some(state) = symbols.first().and_then(|symbol| quote_cache.get(symbol)) else {
        return ChartPathView::empty();
    };
    chart_path_view_from_closes(&state.chart_closes_24h)
}

pub fn chart_path_view_from_closes(closes: &[f64]) -> ChartPathView {
    let values = closes
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    if values.len() < 2 {
        return ChartPathView::empty();
    }

    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    let render_values = chart_render_values(&values);
    let last_index = render_values.len().saturating_sub(1).max(1) as f64;
    let points = render_values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let x = index as f64 * CHART_VIEWBOX_WIDTH / last_index;
            let y = if range.abs() <= f64::EPSILON {
                CHART_VIEWBOX_HEIGHT / 2.0
            } else {
                let drawable_height = CHART_VIEWBOX_HEIGHT - CHART_VERTICAL_PADDING * 2.0;
                CHART_VIEWBOX_HEIGHT
                    - CHART_VERTICAL_PADDING
                    - ((*value - min) / range).clamp(0.0, 1.0) * drawable_height
            };
            (x, y)
        })
        .collect::<Vec<_>>();

    let mut line_path = String::new();
    let mut fill_path = String::new();
    let _ = write!(fill_path, "M 0 {:.1}", CHART_VIEWBOX_HEIGHT);
    for (index, (x, y)) in points.iter().enumerate() {
        let command = if index == 0 { 'M' } else { 'L' };
        let _ = write!(line_path, "{command} {:.1} {:.1} ", x, y);
        let _ = write!(fill_path, " L {:.1} {:.1}", x, y);
    }
    let _ = write!(
        fill_path,
        " L {:.1} {:.1} Z",
        CHART_VIEWBOX_WIDTH, CHART_VIEWBOX_HEIGHT
    );

    ChartPathView {
        line_path: line_path.trim_end().to_string(),
        fill_path,
        end_y_ratio: points
            .last()
            .map(|(_, y)| ((*y / CHART_VIEWBOX_HEIGHT) * 1000.0).round() as i32)
            .unwrap_or(500)
            .clamp(0, 1000),
        ready: true,
        positive: values.last().unwrap_or(&0.0) >= values.first().unwrap_or(&0.0),
    }
}

fn chart_render_values(values: &[f64]) -> Vec<f64> {
    if values.len() <= CHART_MAX_RENDER_POINTS {
        return values.to_vec();
    }

    let last_source_index = values.len() - 1;
    let last_output_index = CHART_MAX_RENDER_POINTS - 1;
    (0..CHART_MAX_RENDER_POINTS)
        .map(|index| {
            let source_index = ((index as f64 / last_output_index as f64)
                * last_source_index as f64)
                .round() as usize;
            values[source_index.min(last_source_index)]
        })
        .collect()
}

pub fn quote_rows_for_symbols(
    symbols: &[String],
    quote_cache: &QuoteCache,
    labels: RuntimeTextLabels<'_>,
) -> Vec<QuoteRowView> {
    quote_rows_for_symbols_with_options(
        symbols,
        quote_cache,
        labels,
        WidgetDisplayOptions::default(),
    )
}

pub fn quote_rows_for_symbols_with_options(
    symbols: &[String],
    quote_cache: &QuoteCache,
    labels: RuntimeTextLabels<'_>,
    display_options: WidgetDisplayOptions,
) -> Vec<QuoteRowView> {
    symbols
        .iter()
        .map(|symbol| {
            quote_cache
                .get(symbol)
                .map(|state| QuoteRowView::from_state_with_options(symbol, state, display_options))
                .unwrap_or_else(|| {
                    QuoteRowView::loading_with_options(symbol, labels, display_options)
                })
        })
        .collect()
}

pub fn newest_update_for_symbols(symbols: &[String], quote_cache: &QuoteCache) -> Option<Instant> {
    symbols
        .iter()
        .filter_map(|symbol| quote_cache.get(symbol).map(|state| state.updated_at))
        .max()
}

pub fn source_for_symbols(
    symbols: &[String],
    quote_cache: &QuoteCache,
) -> Option<MarketDataSource> {
    symbols
        .iter()
        .filter_map(|symbol| quote_cache.get(symbol))
        .max_by_key(|state| state.updated_at)
        .map(|state| state.source)
}

pub fn updated_text_for_symbols(
    symbols: &[String],
    quote_cache: &QuoteCache,
    labels: RuntimeTextLabels<'_>,
    has_market_error: bool,
    now: Instant,
) -> String {
    if symbols.is_empty() {
        return labels.no_pairs.to_string();
    }

    let health = data_health_for_symbols(symbols, quote_cache, now);
    let Some(updated_at) = newest_update_for_symbols(symbols, quote_cache) else {
        return if has_market_error {
            labels.connection_error.to_string()
        } else {
            labels.connecting.to_string()
        };
    };

    let mut text = data_freshness_text(now.saturating_duration_since(updated_at), labels);
    if health.connected < health.total {
        text.push_str(" · ");
        text.push_str(&live_count_text(health.connected, health.total, labels));
    }
    text
}

fn data_freshness_text(elapsed: Duration, labels: RuntimeTextLabels<'_>) -> String {
    if elapsed.as_secs() > STALE_DATA_SECONDS {
        format!("{} {}", labels.stale, format_elapsed(elapsed))
    } else {
        format!("{} {}", labels.updated, format_elapsed(elapsed))
    }
}

pub fn source_text_for_symbols(
    source_prefix: &str,
    symbols: &[String],
    quote_cache: &QuoteCache,
    labels: ProviderLabels<'_>,
    text_labels: RuntimeTextLabels<'_>,
    health: DataHealth,
    has_market_error: bool,
) -> String {
    let mut parts = vec![
        source_prefix.to_string(),
        source_display_text(symbols, quote_cache, labels),
    ];
    if has_market_error {
        parts.push(text_labels.source_error.to_string());
    }
    if health.connected < health.total {
        parts.push(live_count_text(health.connected, health.total, text_labels));
    }
    parts.join(" · ")
}

pub fn source_name_text_for_symbols(
    symbols: &[String],
    quote_cache: &QuoteCache,
    labels: ProviderLabels<'_>,
) -> String {
    source_display_text(symbols, quote_cache, labels)
}

fn source_display_text(
    symbols: &[String],
    quote_cache: &QuoteCache,
    labels: ProviderLabels<'_>,
) -> String {
    let mut sources = symbols
        .iter()
        .filter_map(|symbol| {
            quote_cache
                .get(symbol)
                .map(|state| state.source)
                .or_else(|| market_pair_source(symbol))
        })
        .fold(Vec::new(), |mut sources, source| {
            if !sources.contains(&source) {
                sources.push(source);
            }
            sources
        });
    sources.sort_by_key(|source| match source {
        MarketDataSource::Binance => 0,
        MarketDataSource::Okx => 1,
        MarketDataSource::Hyperliquid => 2,
    });

    match sources.as_slice() {
        [] => labels.mixed.to_string(),
        [source] => provider_display_label(*source, labels).to_string(),
        _ => labels.mixed.to_string(),
    }
}

pub fn provider_display_label(provider: MarketDataSource, labels: ProviderLabels<'_>) -> &'_ str {
    match provider {
        MarketDataSource::Binance => labels.binance,
        MarketDataSource::Okx => labels.okx,
        MarketDataSource::Hyperliquid => labels.hyperliquid,
    }
}

pub fn format_pair_symbol(symbol: &str) -> String {
    format_market_pair_symbol(symbol)
}

pub fn format_pair_symbol_with_options(
    symbol: &str,
    display_options: WidgetDisplayOptions,
) -> String {
    if display_options.hide_quote_asset {
        parse_market_pair(symbol)
            .map(|pair| pair.base)
            .unwrap_or_else(|| format_market_pair_symbol(symbol))
    } else {
        format_market_pair_symbol(symbol)
    }
}

pub fn format_elapsed(elapsed: Duration) -> String {
    if elapsed.as_secs() < 60 {
        format!("{}s", elapsed.as_secs())
    } else {
        format!("{}m", elapsed.as_secs() / 60)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataHealth {
    pub total: usize,
    pub connected: usize,
    pub fresh: usize,
    pub stale: usize,
}

pub fn data_health_for_symbols(
    symbols: &[String],
    quote_cache: &QuoteCache,
    now: Instant,
) -> DataHealth {
    let mut connected = 0;
    let mut fresh = 0;
    let mut stale = 0;

    for symbol in symbols {
        if let Some(state) = quote_cache.get(symbol) {
            connected += 1;
            if now.saturating_duration_since(state.updated_at).as_secs() > STALE_DATA_SECONDS {
                stale += 1;
            } else {
                fresh += 1;
            }
        }
    }

    DataHealth {
        total: symbols.len(),
        connected,
        fresh,
        stale,
    }
}

fn live_count_text(connected: usize, total: usize, labels: RuntimeTextLabels<'_>) -> String {
    format!(
        "{}{connected}/{total}{}",
        labels.live_count_prefix, labels.live_count_suffix
    )
}

pub fn evaluate_alerts_from_cache(
    rules: &[AlertRule],
    quote_cache: &QuoteCache,
) -> Vec<AlertEvaluation> {
    let quotes = quote_cache
        .iter()
        .map(|(symbol, state)| {
            (
                symbol.clone(),
                AlertQuote {
                    price: state.price,
                    change_percent_24h: state.change_percent_24h,
                },
            )
        })
        .collect();
    evaluate_alerts(rules, &quotes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest_json() -> String {
        r#"{
            "schemaVersion": 3,
            "id": "com.example.price-card",
            "name": "Example Price Card",
            "version": "1.0.0",
            "hostApiVersion": ">=0.1.0, <1.0.0",
            "renderer": {
                "kind": "slint",
                "entry": "ui/main.slint",
                "component": "ExamplePriceCard"
            },
            "permissions": {
                "network": false,
                "filesystem": false
            },
            "defaultSize": {
                "width": 260,
                "height": 170
            },
            "symbolLimit": 5,
            "dataRequirements": [
                {
                    "capability": "market.price"
                }
            ]
        }"#
        .to_string()
    }

    #[test]
    fn parses_valid_plugin_manifest_contract() {
        let manifest = parse_manifest(&valid_manifest_json()).unwrap();

        assert_eq!(manifest.id, "com.example.price-card");
        assert_eq!(manifest.renderer.entry, "ui/main.slint");
        assert_eq!(manifest.default_size.width, 260);
        assert_eq!(manifest.size_policy, PluginSizePolicy::Fixed);
        assert_eq!(manifest.min_symbol_limit, 1);
        assert_eq!(manifest.symbol_limit, 5);
        assert!(manifest.default_symbols.is_empty());
    }

    #[test]
    fn parses_symbol_block_size_policy() {
        let json = valid_manifest_json().replace(
            r#""defaultSize": {
                "width": 260,
                "height": 170
            }"#,
            r#""defaultSize": {
                "width": 688,
                "height": 92
            },
            "sizePolicy": {
                "kind": "symbolBlocks",
                "blockSize": {
                    "width": 136,
                    "height": 84
                },
                "padding": {
                    "width": 8,
                    "height": 8
                }
            }"#,
        );

        let manifest = parse_manifest(&json).unwrap();

        assert_eq!(
            manifest.size_policy,
            PluginSizePolicy::SymbolBlocks {
                block_size: PluginSize {
                    width: 136,
                    height: 84
                },
                padding: PluginSize {
                    width: 8,
                    height: 8
                }
            }
        );
    }

    #[test]
    fn parses_symbol_grid_size_policy() {
        let json = valid_manifest_json().replace(
            r#""defaultSize": {
                "width": 260,
                "height": 170
            }"#,
            r#""defaultSize": {
                "width": 688,
                "height": 92
            },
            "sizePolicy": {
                "kind": "symbolGrid",
                "cellSize": {
                    "width": 136,
                    "height": 84
                },
                "contentPadding": {
                    "width": 8,
                    "height": 8
                },
                "columns": 5
            }"#,
        );

        let manifest = parse_manifest(&json).unwrap();

        assert_eq!(
            manifest.size_policy,
            PluginSizePolicy::SymbolGrid {
                cell_size: PluginSize {
                    width: 136,
                    height: 84
                },
                content_padding: PluginSize {
                    width: 8,
                    height: 8
                },
                columns: Some(5),
                rows: None
            }
        );
    }

    #[test]
    fn rejects_symbol_block_policy_that_disagrees_with_default_size() {
        let json = valid_manifest_json().replace(
            r#""defaultSize": {
                "width": 260,
                "height": 170
            }"#,
            r#""defaultSize": {
                "width": 260,
                "height": 170
            },
            "sizePolicy": {
                "kind": "symbolBlocks",
                "blockSize": {
                    "width": 136,
                    "height": 84
                },
                "padding": {
                    "width": 8,
                    "height": 8
                }
            }"#,
        );

        assert!(parse_manifest(&json)
            .unwrap_err()
            .to_string()
            .contains("defaultSize"));
    }

    #[test]
    fn parses_explicit_symbol_bounds() {
        let json = valid_manifest_json().replace(
            r#""symbolLimit": 5"#,
            r#""minSymbolLimit": 2,
            "symbolLimit": 3"#,
        );

        let manifest = parse_manifest(&json).unwrap();

        assert_eq!(manifest.min_symbol_limit, 2);
        assert_eq!(manifest.symbol_limit, 3);
    }

    #[test]
    fn parses_default_symbols() {
        let json = valid_manifest_json().replace(
            r#""symbolLimit": 5"#,
            r#""symbolLimit": 5,
            "defaultSymbols": [
                "binance:spot:BTC/USDT",
                "eth/usdt",
                "sol"
            ]"#,
        );

        let manifest = parse_manifest(&json).unwrap();

        assert_eq!(
            manifest.default_symbols,
            vec!["binance:spot:BTC/USDT", "eth/usdt", "sol"]
        );
    }

    #[test]
    fn rejects_default_symbols_above_symbol_limit() {
        let json = valid_manifest_json().replace(
            r#""symbolLimit": 5"#,
            r#""symbolLimit": 2,
            "defaultSymbols": [
                "BTC",
                "ETH",
                "SOL"
            ]"#,
        );

        assert!(parse_manifest(&json)
            .unwrap_err()
            .to_string()
            .contains("defaultSymbols"));
    }

    #[test]
    fn rejects_min_symbol_limit_above_symbol_limit() {
        let json = valid_manifest_json().replace(
            r#""symbolLimit": 5"#,
            r#""minSymbolLimit": 4,
            "symbolLimit": 3"#,
        );

        assert!(parse_manifest(&json)
            .unwrap_err()
            .to_string()
            .contains("minSymbolLimit"));
    }

    #[test]
    fn rejects_plugin_manifest_path_traversal() {
        let json = valid_manifest_json().replace("ui/main.slint", "../main.slint");

        assert!(parse_manifest(&json)
            .unwrap_err()
            .to_string()
            .contains("must not contain .."));
    }

    #[test]
    fn rejects_plugin_manifest_permissions() {
        let json = valid_manifest_json().replace(
            r#""network": false,
                "filesystem": false"#,
            r#""network": true,
                "filesystem": false"#,
        );

        assert!(parse_manifest(&json)
            .unwrap_err()
            .to_string()
            .contains("permissions.network"));
    }

    #[test]
    fn snapshot_maps_to_quote_row_view() {
        let snapshot = MarketQuoteSnapshot {
            symbol: "binance:spot:BTC/USDT".to_string(),
            price: 106800.12,
            change_percent_24h: 1.234,
            chart_closes_24h: vec![100.0, 101.0, 102.0],
            source: MarketDataSource::Binance,
        };

        let row = QuoteRowView::from_snapshot(&snapshot);

        assert_eq!(row.symbol, "BTC/USDT");
        assert_eq!(row.price, "106800");
        assert_eq!(row.change, "+1.23%");
        assert!(row.positive);
    }

    #[test]
    fn quote_row_display_can_hide_quote_asset() {
        let state = QuoteState::new(
            1.0,
            -0.2,
            vec![1.0, 0.9],
            MarketDataSource::Binance,
            Instant::now(),
        );
        let display_options = WidgetDisplayOptions {
            hide_quote_asset: true,
        };

        let row =
            QuoteRowView::from_state_with_options("binance:spot:BTC/USDC", &state, display_options);
        let loading = QuoteRowView::loading_with_options(
            "binance:spot:ETH/USDT",
            RuntimeTextLabels::default(),
            display_options,
        );

        assert_eq!(row.symbol, "BTC");
        assert_eq!(loading.symbol, "ETH");
    }

    #[test]
    fn builds_widget_runtime_view_from_quote_cache() {
        let now = Instant::now();
        let mut cache = QuoteCache::new();
        cache.insert(
            "binance:spot:BTC/USDT".to_string(),
            QuoteState::new(
                106800.12,
                1.234,
                vec![105000.0, 106000.0, 106800.12],
                MarketDataSource::Binance,
                now - Duration::from_secs(42),
            ),
        );

        let view = build_widget_runtime_view(WidgetRuntimeViewParams {
            widget_id: "quote-board-1",
            symbols: &[
                "binance:spot:BTC/USDT".to_string(),
                "binance:spot:ETH/USDT".to_string(),
            ],
            quote_cache: &cache,
            source_prefix: "live feed",
            provider_labels: ProviderLabels::default(),
            labels: RuntimeTextLabels::default(),
            has_market_error: false,
            now,
            display_options: WidgetDisplayOptions::default(),
        });

        assert_eq!(view.widget_id, "quote-board-1");
        assert_eq!(view.source_name_text, "Binance");
        assert_eq!(view.source_text, "live feed · Binance · 1/2 live");
        assert_eq!(view.updated_text, "Updated 42s · 1/2 live");
        assert_eq!(view.quote_rows[0].symbol, "BTC/USDT");
        assert_eq!(view.quote_rows[0].price, "106800");
        assert_eq!(view.quote_rows[1].symbol, "ETH/USDT");
        assert_eq!(view.quote_rows[1].price, "Connecting");
        assert!(view.chart_ready);
        assert!(view.chart_positive);
        assert!(view.chart_line_path.starts_with("M 0.0"));
        assert!(view.chart_fill_path.ends_with(" Z"));
        assert_eq!(view.chart_end_y_ratio, 150);
    }

    #[test]
    fn runtime_view_applies_symbol_display_options() {
        let view = build_widget_runtime_view(WidgetRuntimeViewParams {
            widget_id: "quote-board-1",
            symbols: &["binance:spot:BTC/USDC".to_string()],
            quote_cache: &QuoteCache::new(),
            source_prefix: "live feed",
            provider_labels: ProviderLabels::default(),
            labels: RuntimeTextLabels::default(),
            has_market_error: false,
            now: Instant::now(),
            display_options: WidgetDisplayOptions {
                hide_quote_asset: true,
            },
        });

        assert_eq!(view.quote_rows[0].symbol, "BTC");
    }

    #[test]
    fn runtime_view_uses_pair_source_until_quotes_arrive() {
        let view = build_widget_runtime_view(WidgetRuntimeViewParams {
            widget_id: "mini-ticker-1",
            symbols: &["okx:spot:ETH/USDT".to_string()],
            quote_cache: &QuoteCache::new(),
            source_prefix: "实时行情",
            provider_labels: ProviderLabels {
                binance: "Binance",
                okx: "OKX",
                hyperliquid: "Hyperliquid",
                mixed: "多个源",
            },
            labels: RuntimeTextLabels {
                no_pairs: "未配置交易对",
                connecting: "连接中",
                connection_error: "连接失败",
                updated: "已更新",
                stale: "已过期",
                fallback: "备用源",
                source_error: "数据源异常",
                live_count_prefix: "已连 ",
                live_count_suffix: "",
            },
            has_market_error: false,
            now: Instant::now(),
            display_options: WidgetDisplayOptions::default(),
        });

        assert_eq!(view.source_name_text, "OKX");
        assert_eq!(view.source_text, "实时行情 · OKX · 已连 0/1");
        assert_eq!(view.updated_text, "连接中");
        assert_eq!(view.quote_rows[0].price, "连接中");
        assert!(!view.chart_ready);
    }

    #[test]
    fn runtime_view_marks_old_data_as_stale() {
        let now = Instant::now();
        let mut cache = QuoteCache::new();
        cache.insert(
            "binance:spot:BTC/USDT".to_string(),
            QuoteState::new(
                106800.12,
                1.234,
                vec![105000.0, 106000.0, 106800.12],
                MarketDataSource::Binance,
                now - Duration::from_secs(240),
            ),
        );

        let view = build_widget_runtime_view(WidgetRuntimeViewParams {
            widget_id: "quote-board-1",
            symbols: &["binance:spot:BTC/USDT".to_string()],
            quote_cache: &cache,
            source_prefix: "live feed",
            provider_labels: ProviderLabels::default(),
            labels: RuntimeTextLabels::default(),
            has_market_error: false,
            now,
            display_options: WidgetDisplayOptions::default(),
        });

        assert_eq!(view.updated_text, "Stale 4m");
    }

    #[test]
    fn runtime_view_marks_mixed_sources_and_source_errors() {
        let now = Instant::now();
        let mut cache = QuoteCache::new();
        cache.insert(
            "okx:spot:BTC/USDT".to_string(),
            QuoteState::new(
                106800.12,
                1.234,
                vec![105000.0, 106000.0, 106800.12],
                MarketDataSource::Okx,
                now - Duration::from_secs(12),
            ),
        );

        let view = build_widget_runtime_view(WidgetRuntimeViewParams {
            widget_id: "quote-board-1",
            symbols: &[
                "okx:spot:BTC/USDT".to_string(),
                "hyperliquid:perp:ETH/USDC".to_string(),
            ],
            quote_cache: &cache,
            source_prefix: "live feed",
            provider_labels: ProviderLabels::default(),
            labels: RuntimeTextLabels::default(),
            has_market_error: true,
            now,
            display_options: WidgetDisplayOptions::default(),
        });

        assert_eq!(view.source_name_text, "Mixed");
        assert_eq!(
            view.source_text,
            "live feed · Mixed · source issue · 1/2 live"
        );
        assert_eq!(view.updated_text, "Updated 12s · 1/2 live");
    }

    #[test]
    fn builds_chart_paths_from_24h_closes() {
        let chart = chart_path_view_from_closes(&[100.0, 90.0, 110.0]);

        assert!(chart.ready);
        assert!(chart.positive);
        assert!(chart.line_path.contains("L 50.0 34.0"));
        assert!(chart.fill_path.starts_with("M 0 40.0"));
        assert!(chart.fill_path.ends_with(" Z"));
        assert_eq!(chart.end_y_ratio, 150);
    }

    #[test]
    fn evaluates_price_and_change_alert_rules() {
        let mut cache = QuoteCache::new();
        cache.insert(
            "binance:spot:BTC/USDT".to_string(),
            QuoteState::new(
                106800.12,
                1.234,
                vec![105000.0, 106000.0, 106800.12],
                MarketDataSource::Binance,
                Instant::now(),
            ),
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

        let alerts = evaluate_alerts_from_cache(&rules, &cache);

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule_id, "btc-price");
        assert_eq!(alerts[0].symbol, "binance:spot:BTC/USDT");
        assert!(alerts[0].body.contains("100000"));
    }
}
